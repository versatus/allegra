use serde::{Serialize, Deserialize};
use getset::Getters;
use alloy::primitives::Address;
use std::net::SocketAddr;

#[derive(Debug, Clone, Getters, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Peer {
    wallet_address: Address,
    ip_address: SocketAddr,
}

impl Peer {
    pub fn new(wallet_address: Address, ip_address: SocketAddr) -> Self {
        Self { wallet_address, ip_address }
    }

    pub fn wallet_address(&self) -> &Address {
        &self.wallet_address
    }

    pub fn ip_address(&self) -> &SocketAddr {
        &self.ip_address
    }

    pub fn wallet_address_hex(&self) -> String {
        format!("{:x}", self.wallet_address())
    }
}

