#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthType {
    #[serde(rename = "tls")]
    Tls,
    #[serde(rename = "file access")]
    FileAccess,
    #[serde(rename = "candid")]
    Candid,
    #[serde(rename = "pos")]
    Pos,
    #[serde(rename = "pki")]
    Pki,
    #[serde(rename = "rbac")]
    Rbac,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    SimpleStream,
    Lxd,
    #[serde(other)]
    Other
}

#[derive(Debug, Clone, Getters, MutGetters, Serialize, Deserialize)]
#[getset(get = "pub")]
struct RemoteFields {
    #[serde(rename = "Addr")]
    ip_addr: String,
    #[serde(rename = "AuthType")]
    auth_type: Option<AuthType>,
    #[serde(rename = "Domain")]
    domain: Option<String>,
    #[serde(rename = "Project")]
    project: Option<String>,
    #[serde(rename = "Protocol")]
    protocol: Option<Protocol>,
    #[serde(rename = "Public")]
    public: bool,
    #[serde(rename = "Global")]
    global: bool,
    #[serde(rename = "Static")]
    static_: bool
}


#[derive(Debug, Clone, Getters, MutGetters, Serialize, Deserialize)]
#[getset(get = "pub")]
pub struct Remote {
    id: String,
    #[serde(rename = "Addr")]
    ip_addr: String,
    #[serde(rename = "AuthType")]
    auth_type: Option<AuthType>,
    #[serde(rename = "Domain")]
    domain: Option<String>,
    #[serde(rename = "Project")]
    project: Option<String>,
    #[serde(rename = "Protocol")]
    protocol: Option<Protocol>,
    #[serde(rename = "Public")]
    public: bool,
    #[serde(rename = "Global")]
    global: bool,
    #[serde(rename = "Static")]
    static_: bool
}

#[allow(private_interfaces)]
impl Remote {
    pub fn from_map(id: String, fields: RemoteFields) -> Self {
        Self {
            id,
            ip_addr: fields.ip_addr().clone(),
            auth_type: fields.auth_type().clone(),
            domain: fields.domain().clone(),
            project: fields.project().clone(),
            protocol: fields.protocol().clone(),
            public: fields.public().clone(),
            global: fields.global().clone(),
            static_: fields.static_().clone()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrustType {
    Client,
    Metrics
}

#[derive(Debug, Clone, Getters, MutGetters, Serialize, Deserialize)]
#[getset(get = "pub")]
pub struct TrustEntry {
    #[serde(rename = "name")]
    id: String,
    #[serde(rename = "type")]
    type_: TrustType,
    restricted: bool,
    projects: Vec<String>,
    certificate: String,
    fingerprint: String,
}

#[derive(Debug, Clone, Getters, MutGetters, Serialize, Deserialize)]
#[getset(get = "pub")]
pub struct TrustToken {
    #[serde(rename = "ClientName")]
    id: String,
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "ExpiresAt")]
    expires_at: String,
}

#[derive(Debug, Clone, Getters, MutGetters, Serialize, Deserialize)]
#[getset(get = "pub")]
pub struct TrustStore {
    remotes: HashMap<String, Remote>,
    trust_entries: HashMap<String, TrustEntry>,
    trust_tokens: HashMap<String, TrustToken>,
}

impl TrustStore {
    pub fn new() -> std::io::Result<Self> {
        let trust_entries = Self::update_trust_entries()?;
        let remotes = Self::update_remotes()?;
        let trust_tokens = Self::update_trust_tokens()?;

        Ok(Self { trust_tokens, trust_entries, remotes })
    }

    fn update_trust_entries() -> std::io::Result<HashMap<String, TrustEntry>> {
        let trust_entries_output = std::process::Command::new("lxc")
            .arg("config")
            .arg("trust")
            .arg("list")
            .arg("-f")
            .arg("json")
            .output()?;

        if trust_entries_output.status.success() {
            let trust_entries: Vec<TrustEntry> = serde_json::from_slice(
                &trust_entries_output.stdout
            ).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?;
            let trust_entries = trust_entries.par_iter().map(|t| {
                (t.id().to_string(), t.clone()) 
            }).collect();
            return Ok(trust_entries)
        } else {
            let err = std::str::from_utf8(&trust_entries_output.stderr).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?;

            return Err(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    err
                )
            )
        }
    }

    fn update_remotes() -> std::io::Result<HashMap<String, Remote>> {
        let remotes_output = std::process::Command::new("lxc")
            .arg("remote")
            .arg("list")
            .arg("-f")
            .arg("json")
            .output()?;

        if remotes_output.status.success() {
            let remotes_fields: HashMap<String, RemoteFields> = serde_json::from_slice(&remotes_output.stdout).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?;

            let remotes: HashMap<String, Remote> = remotes_fields.par_iter().map(|(id, fields)| {
                (id.clone(), Remote::from_map(id.clone(), fields.clone()))
            }).collect();

            return Ok(remotes)
        } else {
            let err = std::str::from_utf8(&remotes_output.stderr).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?;

            return Err(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    err
                )
            )
        }
    }

    fn update_trust_tokens() -> std::io::Result<HashMap<String, TrustToken>> {
        let trust_tokens_output = std::process::Command::new("lxc")
            .arg("config")
            .arg("trust")
            .arg("list-tokens")
            .arg("-f")
            .arg("json")
            .output()?;

        if trust_tokens_output.status.success() {
            let trust_tokens: Vec<TrustToken> = serde_json::from_slice(&trust_tokens_output.stdout)?; 
            let trust_tokens: HashMap<String, TrustToken> = trust_tokens.par_iter().map(|tt| {
                (tt.id().clone(), tt.clone())
            }).collect();

            return Ok(trust_tokens)
        } else {
            let err = std::str::from_utf8(&trust_tokens_output.stderr).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?;

            return Err(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    err
                )
            )

        }
    }
}

pub async fn share_cert(
    &mut self, 
    peer: &Peer, 
) -> std::io::Result<()> {
    log::info!("Attempting to share certificate with peer: {}", &peer.wallet_address_hex()); 
    let peer_id = peer.wallet_address_hex();
    let is_local_peer = if peer_id == self.node().peer_info().wallet_address_hex() {
        true
    } else {
        false
    };

    log::warn!("is_local_peer = {is_local_peer}");
    if !is_local_peer {
        let output = std::process::Command::new("lxc")
            .arg("config")
            .arg("trust")
            .arg("add")
            .arg("--name")
            .arg(&peer_id)
            .output()?;

        if output.status.success() {
            log::info!("Successfully created token for peer {}", &peer.wallet_address().to_string());
            let cert = match std::str::from_utf8(&output.stdout) {
                Ok(res) => res.to_string(),
                Err(e) => return Err(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e
                    )
                )
            };

            self.update_trust_store().await?;

            let cert = self.trust_store().trust_tokens().get(&peer_id).ok_or(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Unable to find peer_id in trust tokens despite success in call to lxc config trust add --name {}", peer_id)
                )
            )?.token().to_string();

            let quorum_id = self.get_local_quorum_membership()?.to_string();
            let task_id = TaskId::new(uuid::Uuid::new_v4().to_string()); 
            let event_id = uuid::Uuid::new_v4().to_string();
            let event = NetworkEvent::ShareCert { 
                peer: self.node().peer_info().clone(), 
                cert,
                task_id,
                event_id,
                quorum_id,
                dst: peer.clone() 
            };

            self.publisher.publish(
                Box::new(NetworkTopic),
                Box::new(event)
            ).await?;
        } else {
            let stderr = std::str::from_utf8(&output.stderr).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?;
            return Err(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Unable to add client certificate to trust store for peer {}: {}", &peer.wallet_address_hex(), &stderr)
                )
            )
        }
    }
    Ok(())
}

pub async fn check_remotes(&mut self) -> std::io::Result<()> {
    log::info!("Updating trust store...");
    self.update_trust_store().await?;
    log::info!("Checking to ensure all local quorum members are remotes...");
    let local_quorum_id = self.get_local_quorum_membership()?;
    let local_quorum = self.get_quorum_by_id(&local_quorum_id).ok_or(
        std::io::Error::new(
            std::io::ErrorKind::Other,
            "unable to find local quorum"
        )
    )?.clone(); 
    let local_quorum_members = local_quorum.peers();
    for peer in local_quorum_members {
        log::info!("checking if peer {}: {} is remote...", peer.wallet_address_hex(), peer.ip_address().to_string());
        if peer != self.node().peer_info() {
            log::info!("checking trust store for peer {}", peer.wallet_address_hex());
            if !self.trust_store.remotes().contains_key(&peer.wallet_address_hex()) {
                log::info!("peer {} not in trust store... checking trust tokens", peer.wallet_address_hex());
                match self.trust_store.trust_tokens().get(&peer.wallet_address_hex()) {
                    Some(token) => {
                        log::info!("found peer {} trust token, sharing...", peer.wallet_address_hex());
                        let event_id = Uuid::new_v4().to_string();
                        let task_id = TaskId::new(Uuid::new_v4().to_string());
                        let event = NetworkEvent::ShareCert { 
                            event_id,
                            task_id,
                            peer: self.node().peer_info().clone(),
                            cert: token.token().clone(),
                            quorum_id: local_quorum_id.to_string(),
                            dst: peer.clone() 
                        };
                        self.publisher_mut().publish(
                            Box::new(NetworkTopic),
                            Box::new(event)
                        ).await?;
                        log::info!("Successfully shared trust token with peer {}: {}...", peer.wallet_address_hex(), peer.ip_address().to_string());
                    }
                    None => {
                        log::info!("peer {} has no trust token, calling self.share_cert()...", peer.wallet_address_hex());
                        self.share_cert(peer).await?;
                        log::info!("successfully called self.share_cert() to create and share a trust token with peer {}...", peer.wallet_address_hex());
                    }
                }
            }
        }
    }

    Ok(())
}

pub async fn accept_cert(
    &mut self,
    peer: &Peer,
    cert: &str
) -> std::io::Result<()> {
    //TODO(asmith): We will want to check against their stake to verify membership

    // Check if peer is member of same quorum as local node
    //log::info!("checking if certificate from peer: {:?} is for local quorum member...", peer);
    let qid = self.get_local_quorum_membership()?;
    log::info!("Local node is member of quorum {}", qid.to_string());
    let quorum_peers = self.get_quorum_peers_by_id(&qid)?;
    log::warn!("quorum_peers.contains(peer) = {}", quorum_peers.contains(peer));
    if quorum_peers.contains(peer) {
        let output = std::process::Command::new("lxc")
            .arg("remote")
            .arg("add")
            .arg(peer.wallet_address_hex())
            .arg(cert)
            .output()?;

        if output.status.success() {
            log::info!("SUCCESS! SUCCESS! Successfully added peer {} to remote", &peer.wallet_address_hex());
            let stdout = std::str::from_utf8(&output.stdout).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?;
            return Ok(())
        } else {
            let stderr = std::str::from_utf8(&output.stderr).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?;
            return Err(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to add peer {} certificate to trust store: {}", &peer.wallet_address_hex(), stderr)
                )
            )
        }
    } else {
        log::warn!("Local quorum does not have {} as peer...", peer.wallet_address_hex());
    }

    log::info!("Completed self.accept_cert method returning...");
    Ok(())
}

async fn attempt_stop_instance(peer_id: String, namespace: String) -> std::io::Result<()> {
    let stop_output = tokio::process::Command::new("lxc")
        .arg("stop")
        .arg(format!("{}:{}", peer_id, namespace))
        .output().await?;

    if stop_output.status.success() {
        return Ok(())
    } else {
        let err = std::str::from_utf8(&stop_output.stderr).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;
        return Err(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                err
            )
        )
    }
}

async fn attempt_copy_instance(peer_id: String, namespace: String) -> std::io::Result<()> {
    let copy_output = tokio::process::Command::new("lxc")
        .arg("copy")
        .arg(namespace.clone())
        .arg(format!("{}:{}", peer_id.clone(), namespace.clone()))
        .arg("--refresh")
        .arg("--mode")
        .arg("pull")
        .output().await?;
                        
    if copy_output.status.success() {
        log::info!("Successfully synced: {} with {}", namespace, peer_id);
        return Ok(())
    } else {
        let err = std::str::from_utf8(&copy_output.stderr).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;
        log::error!("Error attempting to sync {} with {}: {err}", namespace, peer_id);
        return Err(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                err
            )
        )
    }
}

async fn attempt_start_instance(peer_id: String, namespace: String) -> std::io::Result<()> {
    let start_output = tokio::process::Command::new("lxc")
        .arg("start")
        .arg(format!("{}:{}", peer_id, namespace))
        .output().await?;

    if start_output.status.success() {
        return Ok(())
    } else {
        let err = std::str::from_utf8(&start_output.stderr).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;
        return Err(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                err
            )
        )
    }
}
