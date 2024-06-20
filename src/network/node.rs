use std::collections::{HashMap, HashSet};

use crate::dht::Peer;
use getset::{Getters, MutGetters};
use ractor::concurrency::{Instant, Interval};
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator};
use uuid::Uuid;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Getters, Hash)]
struct Ballot {
    candidate: Peer,
    result: [u8; 32]
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Getters, Hash)]
struct Vote {
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

#[derive(Debug)]
pub enum NodeState {
    Follower,
    Leader
}

#[derive(Getters, MutGetters)]
struct Node {
    peer_info: Peer,
    current_leader: Option<Peer>,
    state: NodeState,
    current_term: u64,
    voted_for: Option<Uuid>,
    last_election_time: Instant,
    last_election_block: Option<u64>,
    last_election_block_hash: Option<[u8; 32]>,
    election_interval: Interval,
    election_block_interval: u64,
    votes_collected: HashMap<Ballot, HashMap<Peer, Vote>>,
    last_election_votes: HashMap<Ballot, HashMap<Peer, Vote>>,
    trigger_block: u64,
    last_block_checked: u64,
    block_check_interval: u64
}

impl Node { 
    pub async fn start_election(&mut self, peers: HashSet<Peer>, uri: String) -> std::io::Result<()> {
        self.current_term += 1;

        let block_hash = Self::get_election_block_hash(self.trigger_block).await?;

        let ballots: Vec<Ballot> = peers.par_iter()
            .map(|candidate| {
                Self::calculate_xor_metric(block_hash, candidate)
            }).collect::<Vec<Ballot>>();

        Self::share_vote(
            Vote {
                peer: self.peer_info.clone(),
                ballots, 
                block_height: self.trigger_block, 
                block_hash
            },
            &uri
        ).await?;
        Ok(())
    }

    pub fn handle_vote_recieved(
        &mut self,
        vote: Vote,
        quorum_size: usize
    ) -> std::io::Result<()> {

        // If vote is from the previous election (election already completed)
        // add to the last election votes map
        if Some(vote.block_height) == self.last_election_block && 
            Some(vote.block_hash) == self.last_election_block_hash {
                //If the peer has not casted a vote in that election already
                if !self.last_election_votes.par_iter().any(|(_, v)| v.contains_key(&vote.peer)) {
                    self.last_election_votes.entry(
                        Self::get_winning_ballot(vote.ballots.clone())?
                    ).or_insert_with(HashMap::new).insert(vote.peer.clone(), vote);
                    return Ok(())
                }

                return Ok(())
        }

        // If not from previous election, check if part of current/next election
        if vote.block_height != self.trigger_block {
            return Ok(())
        }

        self.votes_collected.entry(
            Self::get_winning_ballot(vote.ballots.clone())?
        ).or_insert_with(HashMap::new).insert(vote.peer.clone(), vote);

        self.check_majority_reached(quorum_size)?;


        return Ok(())
    }

    fn check_majority_reached(&mut self, quorum_size: usize) -> std::io::Result<()> {
        if let Some(_) = self.votes_collected.clone().par_iter().find_any(|(_, v)| {
            v.len() > (quorum_size / 2)
        }) {
            // Naive majority reached, check that the votes "match"
            let vote_subsets = self.votes_collected.clone().iter().flat_map(|(ballot, value)| {
                value.values().map(move |value| {
                   ((ballot.clone(), value.block_height, value.block_hash), value.clone()) 
                })
            }).fold(HashMap::new(), |mut subsets, value| {
                subsets.entry(value.0).or_insert_with(Vec::new).push(value.1.clone());
                subsets
            });

            // If votes match handle_majority reached
            if let Some(((ballot, block_height, _), _)) = vote_subsets.clone().par_iter().find_any(|(_, v)| {
                v.len() > (quorum_size / 2)
            }) {
                self.handle_majority_reached(&ballot, *block_height)?;
            }
        }

        return Ok(())
    }

    fn get_winning_ballot(ballots: Vec<Ballot>) -> std::io::Result<Ballot> {
        ballots.par_iter()
            .min().ok_or(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "Unable to extract winning ballot"
            )
        ).cloned()
    }

    fn handle_majority_reached(
        &mut self,
        winning_ballot: &Ballot,
        block_height: u64
    ) -> std::io::Result<()> {
        if winning_ballot.candidate == self.peer_info {
            self.state = NodeState::Leader;
        } else {
            self.state = NodeState::Follower;
        }
        self.current_leader = Some(winning_ballot.candidate.clone());
        self.last_election_block = Some(block_height);
        self.trigger_block = {
            block_height + self.election_block_interval
        };
        self.last_election_votes = self.votes_collected.clone();
        //TODO(asmith) replace hardcoded number with configurable static
        //MAX_QUORUM_SIZE
        self.votes_collected = HashMap::with_capacity(50);

        Ok(())

    }

    async fn share_vote(
        vote: Vote,
        uri: &str,
    ) -> std::io::Result<()> {
        todo!();
    }

    async fn get_election_block_hash(
        block_height: u64
    ) -> std::io::Result<[u8; 32]> {
        todo!()
    }

    fn calculate_xor_metric(block_hash: [u8; 32], candidate: &Peer) -> Ballot {
        let mut result = [0u8; 32];
        block_hash.par_iter()
            .zip(
                candidate.id_hash().par_iter()
            ).zip(
                result.par_iter_mut()
            ).for_each(|((bh, can), res)| *res = bh ^ can);

        Ballot { candidate: candidate.clone(), result }
    }
}
