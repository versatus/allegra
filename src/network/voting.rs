use getset::Getters;
use serde::{Serialize, Deserialize};
use crate::network::peer::Peer;
use derive_new::new as New;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Getters, Hash, New)]
#[getset(get = "pub")]
pub struct Ballot {
    candidate: Peer,
    result: [u8; 32]
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Getters, Hash, New)]
#[getset(get = "pub")]
pub struct Vote {
    peer: Peer,
    ballots: Vec<Ballot>,
    block_height: u64,
    block_hash: [u8; 32],
    //TODO(asmith): add ecdsa signature
    //sig: String,
}

impl PartialOrd for Ballot {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.result.partial_cmp(&other.result)
    }
}

impl Ord for Ballot {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.result.cmp(&other.result)
    }
}

