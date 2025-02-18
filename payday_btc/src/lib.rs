pub mod on_chain_aggregate;
pub mod on_chain_processor;

use std::str::FromStr;

use bitcoin::{Address, Network};
use payday_core::Result;

/// Given a Bitcoin address string and a network, parses and validates the address.
/// Returns a checked address result.
pub fn to_address(addr: &str, network: Network) -> Result<Address> {
    Ok(Address::from_str(addr)?.require_network(network)?)
}
