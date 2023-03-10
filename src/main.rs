//! This binary is responsible for implementing an ICMP echo client.
//!
//! The ICMP echo protocol is specified in [RFC 792](https://www.rfc-editor.org/rfc/rfc792)

use parser::parse_input;
pub mod parser;
pub mod icmp;
fn main() {
    println!("{:?}", parse_input())
}
