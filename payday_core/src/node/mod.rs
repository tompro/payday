use std::str::FromStr;

use bitcoin::{Address, Network};

use crate::PaydayResult;

pub mod node_api;

/// Given a Bitcoin address string and a network, parses and validates the address.
/// Returns a checked address result.
pub fn to_address(addr: &str, network: Network) -> PaydayResult<Address> {
    Ok(Address::from_str(addr)?.require_network(network)?)
}
