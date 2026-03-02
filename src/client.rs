use cpal::{
    default_host,
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleRate, StreamConfig,
};
use opus2::{Application, Channels, Encoder};
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::sync::{Arc, Mutex};
use std::{cell::RefCell, io};

use crate::shared::{
    get_broadcast_addr, get_interfaces, BROADCAST_PORT, ENCODED_PACKET_SIZE, FIXED_SAMPLE_RATE,
    MAGIC_HEADER, OPUS_FRAME_SIZE, OPUS_MAX_PACKET_SIZE,
};

pub fn run() -> io::Result<()> {
    let interfaces = get_interfaces();

    if interfaces.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "No IPv4 interfaces found",
        ));
    }

    let mut sockets = Vec::new();
    for (iface, addr) in &interfaces {
        let socket = UdpSocket::bind((addr.ip, 0))?;
        socket.set_broadcast(true)?;
        let broadcast_ip = get_broadcast_addr(addr.clone());
        let broadcast_addr = SocketAddr::new(IpAddr::V4(broadcast_ip), BROADCAST_PORT);
        sockets.push((socket, broadcast_addr));
        println!(
            "  Interface {}: {} -> broadcast {}",
            iface.name, addr.ip, broadcast_addr
        );
    }

    let sockets = Arc::new(sockets);

    let mut encoder = Encoder::new(FIXED_SAMPLE_RATE, Channels::Stereo, Application::Audio)
        .map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to create Opus encoder: {}", e),
            )
        })?;

    encoder
        .set_bitrate(opus2::Bitrate::Bits(128000))
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    encoder
        .set_complexity(10)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    let encoder = Arc::new(Mutex::new(encoder));

    let host = default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No input device available"))?;

    println!(
        "Using input device: {}",
        device.name().unwrap_or_else(|_| "Unknown".to_string())
    );

    let encoder_clone = encoder.clone();
    let stream = device
        .build_input_stream(
            &StreamConfig {
                channels: 2,
                sample_rate: SampleRate(FIXED_SAMPLE_RATE),
                buffer_size: cpal::BufferSize::Fixed(0),
            },
            move |data: &[i16], _: &cpal::InputCallbackInfo| {
                if let Ok(mut enc) = encoder_clone.lock() {
                    send_encoded_audio_data(&sockets, &mut enc, data);
                }
            },
            |err| eprintln!("Audio stream error: {}", err),
            None,
        )
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    stream
        .play()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    println!("Streaming audio (press Ctrl+C to stop)");

    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

thread_local! {
  static SEND_BUFFER: RefCell<[u8; ENCODED_PACKET_SIZE]> = RefCell::new([0u8; ENCODED_PACKET_SIZE]);
  static SAMPLE_BUFFER: RefCell<Vec<i16>> = RefCell::new(Vec::with_capacity(OPUS_FRAME_SIZE * 4));
  static OUTPUT_BUFFER: RefCell<Vec<u8>> = RefCell::new(vec![0u8; OPUS_MAX_PACKET_SIZE]);
}

fn send_encoded_audio_data(
    sockets: &Arc<Vec<(UdpSocket, SocketAddr)>>,
    encoder: &mut Encoder,
    data: &[i16],
) {
    SAMPLE_BUFFER.with(|sample_buf| {
        OUTPUT_BUFFER.with(|output_buf| {
            let mut sample_buf = sample_buf.borrow_mut();
            let mut output_buf = output_buf.borrow_mut();

            sample_buf.extend_from_slice(data);

            let frame_size = OPUS_FRAME_SIZE * 2;
            let mut read_pos = 0;

            while sample_buf.len() - read_pos >= frame_size {
                let chunk = &sample_buf[read_pos..read_pos + frame_size];
                read_pos += frame_size;

                let encoded_len = match encoder.encode(chunk, &mut output_buf) {
                    Ok(len) => len,
                    Err(e) => {
                        eprintln!("Opus encode error: {:?}", e);
                        continue;
                    }
                };

                if encoded_len == 0 || encoded_len > OPUS_MAX_PACKET_SIZE {
                    eprintln!("Opus encoding error: encoded_len={}", encoded_len);
                    continue;
                }

                SEND_BUFFER.with(|packet| {
                    let mut packet = packet.borrow_mut();
                    packet[..MAGIC_HEADER.len()].copy_from_slice(MAGIC_HEADER);

                    packet[MAGIC_HEADER.len()..MAGIC_HEADER.len() + encoded_len]
                        .copy_from_slice(&output_buf[..encoded_len]);

                    let total_len = MAGIC_HEADER.len() + encoded_len;
                    for (socket, addr) in sockets.iter() {
                        if let Err(err) = socket.send_to(&packet[..total_len], addr) {
                            eprintln!("Error sending to {}: {}", addr, err);
                        }
                    }
                });
            }

            if read_pos > 0 {
                sample_buf.drain(..read_pos);
            }
        });
    });
}
