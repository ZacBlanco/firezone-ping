//! A ping program identical to the one found in `main.rs` which utilizes tokio
//! to provide a threadpool for asynchronous ping sweeps
//!
//! The design has a number of shortfalls:
//!     1. The underlying IO isn't async.
//!         Essentially this implementation is using tokio like a threading
//!         library and launches all IO synchronously within the tokio thread.
//!         If large numbers of clients are provided as input, the later clients
//!         will be blocked by the earlier ones until they finish. This is
//!         becauses none of the IO calls `.await` inside the ping function.
//!     2. IO socket is not shared between threads.
//!         not sharing the IO socket means that this program can potentially
//!         request a large number of resources from the system (up to 500
//!         sockets based on the limit provided in the problem description). A
//!         more optimized program could
//!     3. Locking using non-tokio locks.
//!         Locking in this implementation doesn't use tokio locks so when an
//!         acquisition is blocked, then the tokio runtime doesn't get the
//!         opportunity to re-schedule another task.
//!

use firezone_ping::{
    parser,
    pinger::{PingParams, PingResult, Pinger},
};
use pnet::{
    packet::ip::IpNextHeaderProtocol,
    transport::{ipv4_packet_iter, TransportChannelType},
};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::Duration,
};
use tokio::task::JoinHandle;

#[tokio::main]
async fn main() -> PingResult<()> {
    let inputs = parser::parse_input();
    ping(inputs).await
}

async fn ping(hosts: Vec<PingParams>) -> PingResult<()> {
    let map = Arc::new(RwLock::new(HashMap::new()));
    let buf_size = 4096;
    let proto = IpNextHeaderProtocol::new(1); // 1 for ICMP
    let mut tasks: Vec<JoinHandle<PingResult<()>>> = vec![];
    for host in hosts {
        let m = map.clone();
        tasks.push(tokio::spawn(async move {
            let (mut tx, mut rx) =
                pnet::transport::transport_channel(buf_size, TransportChannelType::Layer3(proto))?;
            tx.set_ttl(64)?;
            let mut iter = ipv4_packet_iter(&mut rx);

            if let Err(e) = Pinger::new(&host, Duration::from_secs(5), m).ping(&mut tx, &mut iter) {
                eprintln!("Failed to ping {}: {:?}", host.ip, e);
            }
            Ok(())
        }));
    }

    for task in tasks {
        if let Err(e) = task.await {
            eprintln!("ping failed: {:?}", e);
        }
    }

    Ok(())
}
