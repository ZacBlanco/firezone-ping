//! This module is responsible for providing functionality to parsing stdin and

use std::{
    collections::HashMap,
    io::{stdin, Error, ErrorKind},
    net::Ipv4Addr,
};

use crate::pinger::PingParams;

/// parses the input and prints any invalid input lines to stdout.
///
/// Returns a vector of the valid inputs as [ICMPClientData]
pub fn parse_input() -> Vec<PingParams> {
    let inputs: Vec<PingParams> = stdin()
        .lines()
        .enumerate()
        .map(|(idx, l)| {
            if idx > 499 {
                // over 500 lines -- panic and exit
                // panicking isn't the best option here, but we're doing it here
                // for simplicity
                // Another option would be to simply just take() 500 from the
                // iterator and log that we're not counting anything after the
                // 500th line
                panic!("Too many input lines!")
            }
            match l {
                Ok(l) => {
                    let inputs = l.split(',').collect::<Vec<&str>>();
                    if inputs.len() != 3 {
                        return Err((idx, Error::from(ErrorKind::InvalidInput)));
                    }

                    // masking the actual errors here...ok for now
                    let ip = inputs[0]
                        .parse::<Ipv4Addr>()
                        .map_err(|_| (idx, Error::from(ErrorKind::InvalidInput)))?;
                    let requests = inputs[1]
                        .parse::<u16>()
                        .map_err(|_| (idx, Error::from(ErrorKind::InvalidInput)))?;
                    let interval = inputs[2]
                        .parse::<u32>()
                        .map_err(|_| (idx, Error::from(ErrorKind::InvalidInput)))?;

                    if !(1..=10).contains(&requests) {
                        return Err((idx, Error::from(ErrorKind::InvalidInput)));
                    }
                    if !(1..=1000).contains(&interval) {
                        return Err((idx, Error::from(ErrorKind::InvalidInput)));
                    }

                    Ok(PingParams {
                        ip,
                        requests,
                        interval,
                    })
                }
                Err(e) => Err((idx, e)),
            }
        })
        .map(|inp| {
            if let Err((idx, e)) = &inp {
                println!("Failed to parse input on line {} with {}", idx, e);
            }
            inp
        })
        .filter_map(|x| x.ok())
        .collect::<_>();
    check_duplicate_ips(&inputs);
    inputs
}

/// Checks if there's any duplicate IPs in the parsed data. Panics if there are.
fn check_duplicate_ips(inp: &Vec<PingParams>) {
    let map = inp
        .iter()
        .map(|x| (x.ip, x))
        .collect::<HashMap<_, _>>();
    if map.len() != inp.len() {
        panic!("Duplicate IPs in input");
    }
}
