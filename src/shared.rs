use std::net::Ipv4Addr;

use if_addrs::{IfAddr, Ifv4Addr, Interface};

pub const BROADCAST_PORT: u16 = 5000;
pub const MAGIC_HEADER: &[u8] = b"AUDIO_STATION_V1";
pub const CLIENT_BUFFER_SIZE: usize = 48000;
pub const FIXED_SAMPLE_RATE: u32 = 48000;
pub const OPUS_FRAME_SIZE: usize = 960;
pub const OPUS_MAX_PACKET_SIZE: usize = 1200;
pub const ENCODED_PACKET_SIZE: usize = MAGIC_HEADER.len() + OPUS_MAX_PACKET_SIZE;
pub const WARMUP_THRESHOLD: usize = FIXED_SAMPLE_RATE as usize / 2;

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
