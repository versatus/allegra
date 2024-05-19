use std::collections::{HashMap, HashSet};
use crate::node::NodeId;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct QuorumId(String);

impl QuorumId {
    pub fn new(id: String) -> Self {
        Self(id)
    }
}


pub struct Quorum {
    mapping: HashMap<QuorumId, HashSet<NodeId>>
}
