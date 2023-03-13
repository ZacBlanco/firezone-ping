//! This binary is responsible for implementing an ICMP echo client.
//!
//! The ICMP echo protocol is specified in [RFC 792](https://www.rfc-editor.org/rfc/rfc792)

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::Duration,
};

use parser::parse_input;
use pinger::{PingParams, PingResult, Pinger};
use pnet::{
    packet::ip::IpNextHeaderProtocol,
    transport::{ipv4_packet_iter, TransportChannelType},
};

mod icmp;
pub mod parser;
mod pinger;
fn main() -> PingResult<()> {
    let inputs = parse_input();
    ping(inputs)
}

fn ping(hosts: Vec<PingParams>) -> PingResult<()> {
    let map = Arc::new(RwLock::new(HashMap::new()));
    let buf_size = 4096 * 10; // 40KiB, probably overkill
    let proto = IpNextHeaderProtocol::new(1); // 1 for ICMP
    let (mut tx, mut rx) =
        pnet::transport::transport_channel(buf_size, TransportChannelType::Layer3(proto))?;
    tx.set_ttl(64)?;
    let mut iter = ipv4_packet_iter(&mut rx);

    for host in hosts {
        if let Err(e) =
            Pinger::new(&host, Duration::from_secs(5), map.clone()).ping(&mut tx, &mut iter)
        {
            eprintln!("Failed to ping {}: {:?}", host.ip, e);
        }
    }

    Ok(())
}
