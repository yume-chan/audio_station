use std::{
    cell::RefCell,
    io,
    net::{Ipv4Addr, UdpSocket},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

use cpal::{default_host, SampleRate, StreamConfig};
use dashmap::DashMap;
use multiversion::multiversion;
use opus2::{Channels, Decoder};
use ringbuf::{
    traits::{Consumer, Observer, Producer, Split},
    HeapRb,
};

use crate::shared::{
    DefaultDeviceStream, CLIENT_BUFFER_SIZE, ENCODED_PACKET_SIZE, FIXED_SAMPLE_RATE, MAGIC_HEADER,
    OPUS_FRAME_SIZE, WARMUP_THRESHOLD,
};

type ClientProducer = <HeapRb<i16> as Split>::Prod;
type ClientConsumer = <HeapRb<i16> as Split>::Cons;

struct ClientState {
    decoder: Mutex<Decoder>,
    producer: Mutex<ClientProducer>,
    consumer: Mutex<ClientConsumer>,
    warmup_complete: AtomicBool,
}

type Clients = Arc<DashMap<Ipv4Addr, ClientState>>;

#[multiversion(targets("x86_64+avx512bw", "x86_64+avx2", "x86_64+sse2"))]
#[inline]
pub fn add_saturating_i16(out: &mut [i16], v: &[i16]) {
    for (out, v) in out.iter_mut().zip(v) {
        *out = out.saturating_add(*v);
    }
}

thread_local! {
    static DECODE_BUFFER: RefCell<Vec<i16>> = RefCell::new(vec![0i16; OPUS_FRAME_SIZE * 2]);
}

pub fn run(port: u16) -> io::Result<()> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, port))?;
    socket.set_broadcast(true)?;

    println!("Listening for broadcasts on port {}...", port);
    println!("Local address: {:?}", socket.local_addr());

    let clients: Clients = Arc::new(DashMap::new());

    let clients_clone = clients.clone();
    let _stream = DefaultDeviceStream::output(
        default_host(),
        StreamConfig {
            sample_rate: SampleRate(FIXED_SAMPLE_RATE),
            buffer_size: cpal::BufferSize::Fixed(0),
            channels: 2,
        },
        move |data| {
            data.fill(0);

            let samples_needed = data.len();

            for entry in clients_clone.iter() {
                let client = entry.value();

                if !client.warmup_complete.load(Ordering::Relaxed) {
                    continue;
                }

                let mut consumer = client.consumer.lock().unwrap();
                let available = consumer.occupied_len();

                if available < samples_needed {
                    client.warmup_complete.store(false, Ordering::Relaxed);
                    eprintln!(
                        "Client {} buffer underflow: {} < {} (warmup reset)",
                        entry.key(),
                        available,
                        samples_needed
                    );
                    continue;
                }

                let (tail, head) = consumer.as_slices();
                let tail_len = tail.len();

                if tail.len() >= samples_needed {
                    add_saturating_i16(data, &tail[..samples_needed]);
                } else {
                    add_saturating_i16(&mut data[..tail_len], tail);
                    add_saturating_i16(
                        &mut data[tail.len()..],
                        &head[..samples_needed - tail.len()],
                    );
                }

                consumer.skip(samples_needed);
            }
        },
    );

    let mut buf = [0u8; ENCODED_PACKET_SIZE];

    loop {
        match socket.recv_from(&mut buf) {
            Ok((len, addr)) => {
                if len < MAGIC_HEADER.len() {
                    continue;
                }

                if &buf[..MAGIC_HEADER.len()] != MAGIC_HEADER {
                    continue;
                }

                let encoded_data = &buf[MAGIC_HEADER.len()..len];

                let client_ip = match addr.ip() {
                    std::net::IpAddr::V4(ip) => ip,
                    std::net::IpAddr::V6(_) => continue,
                };

                let client = clients.entry(client_ip).or_insert_with(|| {
                    println!("New client connected: {}", client_ip);
                    let ring = HeapRb::<i16>::new(CLIENT_BUFFER_SIZE);
                    let (producer, consumer) = ring.split();
                    ClientState {
                        decoder: Mutex::new(
                            Decoder::new(FIXED_SAMPLE_RATE, Channels::Stereo).unwrap(),
                        ),
                        producer: Mutex::new(producer),
                        consumer: Mutex::new(consumer),
                        warmup_complete: AtomicBool::new(false),
                    }
                });

                DECODE_BUFFER.with(|decode_buf| {
                    let mut decode_buf = decode_buf.borrow_mut();

                    let mut decoder = client.decoder.lock().unwrap();
                    let decoded =
                        match decoder.decode(encoded_data, decode_buf.as_mut_slice(), false) {
                            Ok(n) => n,
                            Err(e) => {
                                eprintln!("Decode error from {}: {:?}", client_ip, e);
                                return;
                            }
                        };

                    let samples_count = decoded * 2;
                    let mut producer = client.producer.lock().unwrap();
                    let _ = producer.push_slice(&decode_buf[..samples_count]);

                    if !client.warmup_complete.load(Ordering::Relaxed)
                        && producer.occupied_len() >= WARMUP_THRESHOLD
                    {
                        client.warmup_complete.store(true, Ordering::Relaxed);
                        println!("Client {} warmup complete", client_ip);
                    }
                });
            }
            Err(e) => {
                eprintln!("Receive error: {}", e);
            }
        }
    }
}
