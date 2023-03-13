//! Contains the implementation for the Ping program.

use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap},
    io,
    net::{IpAddr, Ipv4Addr},
    process::id,
    time::{Duration, Instant},
};

use pnet::{
    packet::{
        icmp::echo_reply::EchoReplyPacket,
        ip::IpNextHeaderProtocol,
        ipv4::{Ipv4, MutableIpv4Packet},
        Packet,
    },
    transport::{Ipv4TransportChannelIterator, TransportSender},
};

use crate::icmp::IcmpEcho;

/// The parameters for the pinging program
#[derive(Debug, Clone)]
pub struct PingParams {
    /// address to ping
    pub ip: Ipv4Addr,
    /// number of requests to send
    pub requests: u16,
    /// interval between send requests
    pub interval: u32,
}

#[derive(Debug)]
pub enum PingError {
    IoError(io::Error),
}

impl From<io::Error> for PingError {
    fn from(value: io::Error) -> Self {
        PingError::IoError(value)
    }
}

pub type PingResult<T> = Result<T, PingError>;

fn construct_icmp_echo_request(buf: &mut [u8], seq: u16, id: u16) {
    let echo = IcmpEcho::new(id, seq);
    echo.construct_buf(buf);
}

/// Represents a set of inputs to run a ping program on
pub struct Pinger<'a> {
    data: &'a PingParams,
    timeout: Duration,
}

impl<'a> Pinger<'a> {
    pub fn new(data: &'a PingParams, timeout: Duration) -> Self {
        Pinger { data, timeout }
    }

    /// Generates ICMP Echo Request/Reply packets based on the parameters within
    /// the struct.
    ///
    /// ping output is printed to stdout
    pub fn ping(
        &self,
        tx: &mut TransportSender,
        rx: &mut Ipv4TransportChannelIterator,
    ) -> PingResult<()> {
        // stores set of instants where a timeout should be recorded and entry removed
        // from the active request map
        let mut timeouts = BinaryHeap::new();
        // active request map to store non-timed out ICMP echoes and the request time
        let mut reqs = HashMap::new();
        // Instant when the next ICMP echo should be sent
        let mut next_pkt = Instant::now();
        let mut seq = 0;

        // main event loop
        loop {
            // this could be executed with timeouts at finer granularity, but ping
            // the requirements for ICMP echo generally don't require higher levels
            // of precision.
            match rx.next_with_timeout(Duration::from_millis(1)) {
                Ok(Some((packet, addr))) => {
                    let recv = Instant::now();
                    let ttl = packet.get_ttl();
                    let payload = packet.payload();
                    if let Some(reply) = EchoReplyPacket::new(payload) {
                        let seq = reply.get_sequence_number();
                        let mut remove = false;
                        if let Some((time, _)) = reqs.get(&seq) {
                            let lat: Duration = recv - *time;
                            if let IpAddr::V4(ip) = addr {
                                remove = true;
                                println!("{},{},{},{}", ip, ttl, seq, lat.as_micros())
                            }
                        };
                        if remove {
                            let _ = reqs.remove(&seq);
                        }
                    }
                }
                Ok(None) => (),
                Err(e) => eprintln!("Error occurred while reading packets: {:?}", e),
            }

            // check if we need to break the event loop
            // finish condition is that we've sent `seq` # of requests and that
            // all outstanding requests have been printed
            if seq >= self.data.requests && reqs.is_empty() {
                break;
            }

            // check if a new request should be sent
            if Instant::now() > next_pkt && seq < self.data.requests {
                let mut buf =
                    vec![0u8; MutableIpv4Packet::minimum_packet_size() + IcmpEcho::size()];
                let mut icmp_buf = vec![0u8; IcmpEcho::size()];
                construct_icmp_echo_request(&mut icmp_buf, seq, id() as u16);
                let mut pkt = MutableIpv4Packet::new(&mut buf)
                    .unwrap_or_else(|| panic!("Couldn't create ipv4 packet"));
                pkt.populate(&Ipv4 {
                    version: 4,
                    header_length: 5,
                    dscp: 0,
                    ecn: 0,
                    total_length: icmp_buf.len() as u16
                        + MutableIpv4Packet::minimum_packet_size() as u16,
                    identification: 0,
                    flags: 2,
                    fragment_offset: 0,
                    ttl: 64, // unsure if required when tx channel has TTL set
                    next_level_protocol: IpNextHeaderProtocol(1),
                    checksum: 0,
                    source: Ipv4Addr::new(0, 0, 0, 0),
                    destination: self.data.ip,
                    options: vec![],
                    payload: icmp_buf,
                });

                let time = Instant::now();

                if let Err(e) = tx.send_to(pkt, IpAddr::V4(self.data.ip)) {
                    eprintln!(
                        "Failed to send echo request {} to {}: {:?}",
                        seq, self.data.ip, e
                    );
                }

                // schedule the next request
                next_pkt = time + Duration::from_millis(self.data.interval as u64);
                // schedule the timeout
                timeouts.push(Reverse((time + self.timeout, seq)));
                reqs.insert(seq, (time, &self.data.ip));
                seq += 1;
            }

            // time out any previous requests
            loop {
                let mut pop = false;
                if let Some(Reverse((t, _seq))) = timeouts.peek() {
                    if Instant::now() > *t {
                        // timeout
                        println!("{},-1,{},timeout exceeded", self.data.ip, _seq);
                        pop = true; // needed due to lifetime constraints on peek
                    }
                } else {
                    break;
                }
                if pop {
                    if let Some(Reverse((_, seq))) = timeouts.pop() {
                        let _ = reqs.remove(&seq);
                    }
                } else {
                    break;
                }
            }
        }

        Ok(())
    }
}
