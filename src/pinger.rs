//! Contains the implementation for the Ping program.

use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap, HashSet},
    io,
    net::{IpAddr, Ipv4Addr},
    process::id,
    sync::{Arc, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard},
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
    PoisonedLockError,
}

impl From<io::Error> for PingError {
    fn from(value: io::Error) -> Self {
        PingError::IoError(value)
    }
}

impl<'a, T> From<PoisonError<RwLockReadGuard<'a, T>>> for PingError {
    fn from(_value: PoisonError<RwLockReadGuard<T>>) -> Self {
        PingError::PoisonedLockError
    }
}

impl<'a, T> From<PoisonError<RwLockWriteGuard<'a, T>>> for PingError {
    fn from(_value: PoisonError<RwLockWriteGuard<T>>) -> Self {
        PingError::PoisonedLockError
    }
}

pub type PingResult<T> = Result<T, PingError>;

fn construct_icmp_echo_request(buf: &mut [u8], seq: u16, id: u16) {
    let echo = IcmpEcho::new(id, seq);
    echo.construct_buf(buf);
}

type ActiveRequestMap = HashMap<(Ipv4Addr, u16), Instant>;
type SafeActiveRequestMap = Arc<RwLock<ActiveRequestMap>>;

/// Represents a set of inputs to run a ping program on
pub struct Pinger<'a> {
    /// params for pinging
    params: &'a PingParams,
    /// active request map to store non-timed out ICMP echoes and the request time
    active_requests: SafeActiveRequestMap,
    /// timeout for each echo reply
    timeout: Duration,
}

impl<'a> Pinger<'a> {
    pub fn new(
        params: &'a PingParams,
        timeout: Duration,
        active_requests: SafeActiveRequestMap,
    ) -> Self {
        Pinger {
            params,
            timeout,
            active_requests,
        }
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
        // Instant when the next ICMP echo should be sent
        let mut outstanding = HashSet::new();
        let mut next_pkt = Instant::now();
        let mut seq = 0;

        // main event loop
        loop {
            // this could be executed with timeouts at finer granularity, but ping
            // the requirements for ICMP echo generally don't require higher levels
            // of precision.
            match rx.next_with_timeout(Duration::from_millis(1)) {
                Ok(Some((packet, addr))) => {
                    if let IpAddr::V4(ip) = addr {
                        let recv = Instant::now();
                        let ttl = packet.get_ttl();
                        let payload = packet.payload();
                        if let Some(reply) = EchoReplyPacket::new(payload) {
                            if self.params.ip != ip {
                                eprintln!(
                                    "got reply from {} for {}:{}",
                                    ip,
                                    self.params.ip,
                                    reply.get_sequence_number()
                                );
                                // continue;
                            }
                            let seq = reply.get_sequence_number();
                            let mut remove = false;
                            let mut _guard = self.active_requests.write()?;
                            if let Some(time) = _guard.get(&(ip, seq)) {
                                let lat: Duration = recv - *time;
                                remove = true;
                                println!("{},{},{},{}", ip, ttl, seq, lat.as_micros())
                            }
                            outstanding.remove(&(ip, seq));
                            if remove {
                                _guard.remove(&(ip, seq));
                            }
                            drop(_guard);
                        };
                    }
                }
                Ok(None) => (),
                Err(e) => eprintln!("Error occurred while reading packets: {:?}", e),
            }

            // check if we need to break the event loop
            // finish condition is that we've sent `seq` # of requests and that
            // all outstanding requests have been printed
            if seq >= self.params.requests {
                // all messages sent, check if there any of the outstanding
                // which may have been handled by another socket
                let mut rms = vec![];
                for msg in outstanding.iter() {
                    if !self.active_requests.read()?.contains_key(msg) {
                        rms.push(*msg);
                    }
                }
                for msg in rms {
                    outstanding.remove(&msg);
                }
                if outstanding.is_empty() {
                    break;
                }
            }

            // check if a new request should be sent
            if Instant::now() > next_pkt && seq < self.params.requests {
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
                    destination: self.params.ip,
                    options: vec![],
                    payload: icmp_buf,
                });

                let time = Instant::now();

                if let Err(e) = tx.send_to(pkt, IpAddr::V4(self.params.ip)) {
                    eprintln!(
                        "Failed to send echo request {} to {}: {:?}",
                        seq, self.params.ip, e
                    );
                }

                // schedule the next request
                next_pkt = time + Duration::from_millis(self.params.interval as u64);
                // schedule the timeout
                timeouts.push(Reverse((time + self.timeout, seq)));
                self.active_requests
                    .write()?
                    .insert((self.params.ip, seq), time);
                outstanding.insert((self.params.ip, seq));
                seq += 1;
            }

            // time out any previous requests
            loop {
                let mut pop = false;
                if let Some(Reverse((t, _seq))) = timeouts.peek() {
                    if Instant::now() > *t {
                        // timeout
                        let _ = self.active_requests.write()?.remove(&(self.params.ip, seq));
                        pop = true; // needed due to lifetime constraints on peek
                    }
                } else {
                    break;
                }
                if pop {
                    if let Some(Reverse((_, seq))) = timeouts.pop() {
                        println!("{},-1,{},timeout exceeded", self.params.ip, seq);
                        outstanding.remove(&(self.params.ip, seq));
                    }
                } else {
                    break;
                }
            }
        }

        Ok(())
    }
}
