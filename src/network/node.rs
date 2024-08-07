use std::collections::{HashMap, HashSet};

use crate::consts::*;
use crate::{
    account::TaskId,
    event::NetworkEvent,
    network::peer::Peer,
    publish::{GenericPublisher, NetworkTopic},
    voting::{Ballot, Vote},
    Namespace,
};
use conductor::publisher::PubStream;
use getset::{Getters, MutGetters};
use rayon::iter::{
    IndexedParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator,
};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use tokio::time::{Instant, Interval};
use uuid::Uuid;

#[derive(Getters, Serialize, Deserialize, Debug)]
#[getset(get = "pub")]
pub struct Config {
    node_config: NodeConfig,
}

impl Config {
    pub async fn from_file(path: &str) -> std::io::Result<Self> {
        let config_file_content = tokio::fs::read_to_string(path).await?;

        let config = toml::from_str(&config_file_content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        Ok(config)
    }
}

#[derive(Getters, Serialize, Deserialize, Debug)]
#[getset(get = "pub")]
pub struct NodeConfig {
    wallet_keyfile: Option<String>,
    public_ip_address: Option<String>,
    wallet_mnemonic: Option<String>,
    wallet_address: Option<String>,
    wallet_pubkey: Option<String>,
    wallet_signing_key: Option<String>,
    pd_endpoints: Option<Vec<String>>,
    subscriber_uri: Option<String>,
    publisher_uri: Option<String>,
    bootstrap_wallet_addresses: Option<Vec<String>>,
    bootstrap_ip_addresses: Option<Vec<String>>,
    is_bootstrap: Option<bool>,
    server_address: Option<String>,
    vmm_filesystem: Option<String>,
    // Publishers publish to frontend
    broker_frontend: Option<String>,
    // Subscribers subscribe to backend
    broker_backend: Option<String>,
}

#[derive(Getters, Serialize, Deserialize, Debug)]
#[getset(get = "pub")]
pub struct WalletConfig {
    secret_key: String,
    public_key: String,
    address: String,
}

impl NodeConfig {
    pub async fn from_file(path: &str) -> std::io::Result<Self> {
        let config_file_content = tokio::fs::read_to_string(path).await?;
        let config = toml::from_str(&config_file_content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        Ok(config)
    }
}

#[derive(Debug, Clone)]
pub enum NodeState {
    Follower,
    Leader,
}

#[derive(Getters, MutGetters)]
#[getset(get = "pub", get_copy = "pub", get_mut)]
pub struct Node {
    peer_info: Peer,
    current_leader: Option<Peer>,
    state: NodeState,
    current_term: u64,
    voted_for: Option<Uuid>,
    last_election_time: Option<Instant>,
    last_election_block: Option<u64>,
    last_election_block_hash: Option<[u8; 32]>,
    election_interval: Option<Interval>,
    election_block_interval: u64,
    votes_collected: HashMap<Ballot, HashMap<Peer, Vote>>,
    last_election_votes: HashMap<Ballot, HashMap<Peer, Vote>>,
    trigger_block: u64,
    last_block_checked: u64,
    block_check_interval: Option<Interval>,
}

impl Node {
    pub fn new(peer_info: Peer) -> Self {
        Self {
            peer_info,
            current_leader: None,
            state: NodeState::Follower,
            current_term: 0,
            voted_for: None,
            last_election_time: None,
            last_election_block: None,
            last_election_block_hash: None,
            election_interval: None,
            election_block_interval: ELECTION_BLOCK_INTERVAL,
            votes_collected: HashMap::new(),
            last_election_votes: HashMap::new(),
            trigger_block: ELECTION_BLOCK_INTERVAL,
            last_block_checked: 0,
            block_check_interval: None,
        }
    }

    pub async fn start_election(
        &mut self,
        peers: &HashSet<Peer>,
        uri: &String,
    ) -> std::io::Result<()> {
        self.current_term += 1;

        let block_hash = Self::get_election_block_hash(self.trigger_block).await?;

        let ballots: Vec<Ballot> = peers
            .par_iter()
            .map(|candidate| Self::calculate_xor_metric(block_hash, candidate))
            .collect::<Vec<Ballot>>();

        Self::share_vote(
            Vote::new(
                self.peer_info.clone(),
                ballots,
                self.trigger_block,
                block_hash,
            ),
            &uri,
            peers.par_iter().cloned().collect(),
        )
        .await?;
        Ok(())
    }

    pub fn handle_vote_recieved(&mut self, vote: Vote, quorum_size: usize) -> std::io::Result<()> {
        // If vote is from the previous election (election already completed)
        // add to the last election votes map
        if Some(vote.block_height()) == self.last_election_block.as_ref()
            && Some(vote.block_hash()) == self.last_election_block_hash.as_ref()
        {
            //If the peer has not casted a vote in that election already
            if !self
                .last_election_votes
                .par_iter()
                .any(|(_, v)| v.contains_key(&vote.peer()))
            {
                self.last_election_votes
                    .entry(Self::get_winning_ballot(vote.ballots().clone())?)
                    .or_insert_with(HashMap::new)
                    .insert(vote.peer().clone(), vote);
                return Ok(());
            }

            return Ok(());
        }

        // If not from previous election, check if part of current/next election
        if vote.block_height() != &self.trigger_block {
            return Ok(());
        }

        self.votes_collected
            .entry(Self::get_winning_ballot(vote.ballots().clone())?)
            .or_insert_with(HashMap::new)
            .insert(vote.peer().clone(), vote);

        self.check_majority_reached(quorum_size)?;

        return Ok(());
    }

    fn check_majority_reached(&mut self, quorum_size: usize) -> std::io::Result<()> {
        if let Some(_) = self
            .votes_collected
            .clone()
            .par_iter()
            .find_any(|(_, v)| v.len() > (quorum_size / 2))
        {
            // Naive majority reached, check that the votes "match"
            let vote_subsets = self
                .votes_collected
                .clone()
                .iter()
                .flat_map(|(ballot, value)| {
                    value.values().map(move |value| {
                        (
                            (
                                ballot.clone(),
                                value.block_height().clone(),
                                value.block_hash().clone(),
                            ),
                            value.clone(),
                        )
                    })
                })
                .fold(HashMap::new(), |mut subsets, value| {
                    subsets
                        .entry(value.0)
                        .or_insert_with(Vec::new)
                        .push(value.1.clone());
                    subsets
                });

            // If votes match majority reached
            if let Some(((ballot, block_height, _), _)) = vote_subsets
                .clone()
                .par_iter()
                .find_any(|(_, v)| v.len() > (quorum_size / 2))
            {
                self.handle_majority_reached(&ballot, *block_height)?;
            }
        }

        return Ok(());
    }

    fn get_winning_ballot(ballots: Vec<Ballot>) -> std::io::Result<Ballot> {
        ballots
            .par_iter()
            .min()
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Unable to extract winning ballot",
            ))
            .cloned()
    }

    fn handle_majority_reached(
        &mut self,
        winning_ballot: &Ballot,
        block_height: u64,
    ) -> std::io::Result<()> {
        if winning_ballot.candidate() == &self.peer_info {
            self.state = NodeState::Leader;
        } else {
            self.state = NodeState::Follower;
        }
        self.current_leader = Some(winning_ballot.candidate().clone());
        self.last_election_block = Some(block_height);
        self.trigger_block = block_height + self.election_block_interval;
        self.last_election_votes = self.votes_collected.clone();
        //TODO(asmith) replace hardcoded number with configurable static
        //MAX_QUORUM_SIZE
        self.votes_collected = HashMap::with_capacity(50);

        log::info!(
            "new leader elected: {:x}",
            winning_ballot.candidate().wallet_address()
        );
        Ok(())
    }

    pub async fn share_vote(vote: Vote, uri: &str, peers: Vec<Peer>) -> std::io::Result<()> {
        let mut publisher = GenericPublisher::new(&uri).await?;

        let event_id = uuid::Uuid::new_v4().to_string();
        let task_id = TaskId::new(uuid::Uuid::new_v4().to_string());
        let event = NetworkEvent::CastLeaderElectionVote {
            event_id,
            task_id,
            vote,
            peers,
        };

        publisher
            .publish(Box::new(NetworkTopic), Box::new(event))
            .await?;

        Ok(())
    }

    async fn get_election_block_hash(_block_height: u64) -> std::io::Result<[u8; 32]> {
        todo!()
    }

    fn calculate_xor_metric(block_hash: [u8; 32], candidate: &Peer) -> Ballot {
        let mut hasher = Sha3_256::new();
        hasher.update(candidate.wallet_address());
        let hash_bytes = hasher.finalize().to_vec();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&hash_bytes);
        let mut result = [0u8; 32];
        block_hash
            .par_iter()
            .zip(hash.par_iter())
            .zip(result.par_iter_mut())
            .for_each(|((bh, can), res)| *res = bh ^ can);

        Ballot::new(candidate.clone(), result)
    }

    pub async fn setup_instance_bricks(
        &mut self,
        instances: Vec<Namespace>,
    ) -> std::io::Result<()> {
        for namespace in instances {
            std::fs::create_dir_all(&format!(
                "/mnt/glusterfs/vms/{}/brick",
                namespace.inner().to_string()
            ))?;
        }

        Ok(())
    }
}
