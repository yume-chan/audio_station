use std::{
    net::Ipv4Addr,
    sync::{
        Arc, Mutex,
        mpsc::{Sender, channel},
    },
    thread,
    time::Duration,
};

use clap::ValueEnum;
use cpal::{
    Host,
    traits::{HostTrait, StreamTrait},
};
use cpal::{StreamConfig, traits::DeviceTrait};
use if_addrs::{IfAddr, Ifv4Addr, Interface};

pub const BROADCAST_PORT: u16 = 5000;
pub const MAGIC_HEADER: &[u8] = b"AUDIO_STATION_V1";
pub const CLIENT_BUFFER_SIZE: usize = 48000;
pub const FIXED_SAMPLE_RATE: u32 = 48000;
pub const OPUS_FRAME_SIZE: usize = 960;
pub const OPUS_MAX_PACKET_SIZE: usize = 1200;
pub const ENCODED_PACKET_SIZE: usize = MAGIC_HEADER.len() + OPUS_MAX_PACKET_SIZE;
pub const WARMUP_THRESHOLD: usize = FIXED_SAMPLE_RATE as usize / 2;

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DeviceType {
    Input,
    Output,
}

pub enum StreamSignal {
    Retry,
    Stop,
}

pub struct DefaultDeviceStream {
    sender: Arc<Sender<StreamSignal>>,
}

impl DefaultDeviceStream {
    pub fn input<D>(host: Host, r#type: DeviceType, config: StreamConfig, callback: D) -> Self
    where
        D: FnMut(&[i16]) -> () + Send + 'static,
    {
        let callback = Arc::new(Mutex::new(callback));
        let callback_clone = callback.clone();

        let (sender, receiver) = channel::<StreamSignal>();
        let sender = Arc::new(sender);
        let sender_clone = sender.clone();

        thread::spawn(move || {
            loop {
                let device = match r#type {
                    DeviceType::Input => host.default_input_device(),
                    DeviceType::Output => host.default_output_device(),
                }
                .unwrap();

                println!(
                    "Using input device: {}",
                    device.name().unwrap_or_else(|_| "Unknown".to_string())
                );

                let callback_clone = callback_clone.clone();
                let sender_clone = sender_clone.clone();

                let stream = device
                    .build_input_stream(
                        &config,
                        move |data: &[i16], _| {
                            callback_clone.lock().unwrap()(data);
                        },
                        move |err| {
                            eprintln!("Audio stream error: {}", err);
                            thread::sleep(Duration::from_secs(1));
                            sender_clone.send(StreamSignal::Retry).unwrap();
                        },
                        None,
                    )
                    .unwrap();

                stream.play().unwrap();

                if let StreamSignal::Stop = receiver.recv().unwrap() {
                    break;
                }
            }
        });

        Self { sender }
    }

    pub fn output<D>(host: Host, config: StreamConfig, callback: D) -> Self
    where
        D: FnMut(&mut [i16]) -> () + Send + 'static,
    {
        let callback = Arc::new(Mutex::new(callback));
        let callback_clone = callback.clone();

        let (sender, receiver) = channel::<StreamSignal>();
        let sender = Arc::new(sender);
        let sender_clone = sender.clone();

        thread::spawn(move || {
            loop {
                let device = host.default_output_device().unwrap();

                println!(
                    "Using output device: {}",
                    device.name().unwrap_or_else(|_| "Unknown".to_string())
                );

                let callback_clone = callback_clone.clone();
                let sender_clone = sender_clone.clone();

                let stream = device
                    .build_output_stream(
                        &config,
                        move |data: &mut [i16], _| {
                            callback_clone.lock().unwrap()(data);
                        },
                        move |err| {
                            eprintln!("Audio stream error: {}", err);
                            thread::sleep(Duration::from_secs(1));
                            sender_clone.send(StreamSignal::Retry).unwrap();
                        },
                        None,
                    )
                    .unwrap();

                stream.play().unwrap();

                if let StreamSignal::Stop = receiver.recv().unwrap() {
                    break;
                }
            }
        });

        Self { sender }
    }
}

impl Drop for DefaultDeviceStream {
    fn drop(&mut self) {
        self.sender.send(StreamSignal::Stop).unwrap();
    }
}

pub fn get_interfaces() -> Vec<(Interface, Ifv4Addr)> {
    if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|iface| {
            if iface.is_loopback() {
                None
            } else if let IfAddr::V4(addr) = iface.addr.clone() {
                Some((iface, addr))
            } else {
                None
            }
        })
        .collect()
}

pub fn get_broadcast_addr(addr: Ifv4Addr) -> Ipv4Addr {
    let ip = u32::from(addr.ip);
    let mask = u32::from(addr.netmask);
    Ipv4Addr::from(ip | !mask)
}
