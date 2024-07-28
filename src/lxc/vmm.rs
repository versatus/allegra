
async fn attempt_sync_instance(&mut self, original_event_id: String, namespace: Namespace, event: QuorumSyncEvent) -> std::io::Result<()> {
    log::info!("Attempting to sync instance {} with quorum", namespace.to_string());
    let instance_quorum = self.get_instance_quorum_membership(&namespace).ok_or(
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Unable to acquire quuorum membership for instance {}", namespace.to_string())
        )
    )?;

    let local_quorum = self.get_local_quorum_membership()?;

    if local_quorum == instance_quorum {
        log::info!("local quorum is responsible for instance {}", namespace.to_string());
        let node_state = self.node().state().clone();
        match node_state {
            NodeState::Leader => {
                log::info!("local node is the leader, start sync...");

                let local_peers = self.get_local_peers().clone().ok_or(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "unable to acquire quorum peers..."
                    )
                )?.clone();

                let local_peer = self.node().peer_info().clone();
                let publisher_uri = self.publisher.peer_addr()?;
                let inner_namespace = namespace.clone();
                let future = tokio::spawn(async move {
                    for peer in local_peers {
                        Self::attempt_stop_instance(peer.wallet_address_hex(), inner_namespace.to_string()).await?;
                        Self::attempt_copy_instance(peer.wallet_address_hex(), inner_namespace.to_string()).await?;
                        Self::attempt_start_instance(peer.wallet_address_hex(), inner_namespace.to_string()).await?;
                        Self::attempt_share_server_config(peer, local_peer.clone(), &publisher_uri).await?; 
                    }

                    Ok::<_, std::io::Error>(QuorumResult::Unit(()))
                });

                self.futures.push(future);
                log::info!("Created future to sync instance with all remotes... Awaiting completion without blocking...");
            }
            NodeState::Follower => {
                log::info!("local node is a follower, not leader...");
                match event {
                    QuorumSyncEvent::LibrettoEvent(e) => {
                        log::info!("Libretto event {e:?}... Inform leader...");
                        let leader = self.node().current_leader().clone().ok_or(
                            std::io::Error::new(
                                std::io::ErrorKind::Other,
                                "quorum currently has no leader"
                            )
                        )?;

                        log::info!("Discovered leader: {}, inform leader of sync event...", leader.wallet_address_hex());

                        let event_id = Uuid::new_v4().to_string();
                        let task_id = TaskId::new(Uuid::new_v4().to_string());
                        let requestor = self.node().peer_info().clone();
                        self.publisher_mut().publish(
                            Box::new(NetworkTopic),
                            Box::new(NetworkEvent::SyncInstanceToLeader {
                                event_id,
                                original_event_id,
                                requestor,
                                task_id,
                                namespace: namespace.clone(),
                                event: QuorumSyncEvent::LibrettoEvent(e),
                                dst: leader
                            })
                        ).await?;
                    }
                    QuorumSyncEvent::IntervalEvent(i) => {
                        log::info!("Only leader triggers syncs for IntervalEvents... Ignore...");
                    }
                }
            }
        }

        return Ok(())
    }

    return Ok(())
}
