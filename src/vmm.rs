use std::num::NonZeroUsize;
use crate::{
    account::{
        TaskId,
        Namespace, 
        TaskStatus
    }, vm_info::VmList,
    params::{
        InstanceCreateParams,
        InstanceStartParams,
        InstanceStopParams,
        InstanceDeleteParams,
        InstanceAddPubkeyParams,
        InstanceExposePortParams, Payload
    },
    helpers::{
        update_task_status,
        recover_owner_address,
        recover_namespace,
        verify_ownership,
        update_iptables,
        update_account
    }
};
use futures::stream::{FuturesUnordered, StreamExt};
use tokio::task::JoinHandle;
use lru::LruCache;
use sha3::{Digest, Sha3_256};

pub const LOWEST_PORT: u16 = 2222;
pub const HIGHEST_PORT: u16 = 65535;
pub static DEFAULT_NETWORK: &'static str = "lxdbr0";
pub static SUCCESS: &'static str = "SUCCESS";
pub static FAILURE: &'static str = "FAILURE";
pub static PENDING: &'static str = "PENDING";

#[derive(Debug)]
pub enum VmManagerMessage {
    NewInstance {
        params: InstanceCreateParams,
        task_id: TaskId 
    },
    StopInstance {
        params: InstanceStopParams,
        sig: String,
        task_id: TaskId
    },
    DeleteInstance {
        params: InstanceDeleteParams,
        sig: String,
        task_id: TaskId 
    },
    InjectAuth {
        params: InstanceAddPubkeyParams,
        sig: String,
        task_id: TaskId 
    },
    StartInstance {
        params: InstanceStartParams,
        sig: String,
        task_id: TaskId 
    },
    ExposePorts {
        params: InstanceExposePortParams,
        sig: String,
        task_id: TaskId,
    }
}

pub struct VmManager {
    network: String,
    next_port: u16,
    handles: FuturesUnordered<JoinHandle<std::io::Result<([u8; 20], TaskId, TaskStatus)>>>,
    vmlist: VmList,
    state_client: tikv_client::RawClient,
    task_cache: LruCache<TaskId, TaskStatus> 
}

impl VmManager {
    pub async fn new<S: Into<String>>(
        pd_endpoints: Vec<S>,
        config: Option<tikv_client::Config>
    ) -> std::io::Result<Self> {
        let network = DEFAULT_NETWORK.to_string();
        let next_port = LOWEST_PORT;
        let handles = FuturesUnordered::new();
        let vmlist = match std::process::Command::new("lxc")
            .args(["list", "--format", "json"])
            .output() {
            Ok(output) => {
                if output.status.success() {
                    let vmlist_str = std::str::from_utf8(
                        &output.stdout
                    ).map_err(|e| {
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e.to_string()
                        )
                    })?;
                    let vmlist = serde_json::from_str(
                        vmlist_str
                    ).map_err(|e| { std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e.to_string()
                        )
                    })?;
                    vmlist
                } else {
                    let err = std::str::from_utf8(
                        &output.stderr
                    ).map_err(|e| {
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e.to_string()
                        )
                    })?.to_string();
                    return Err(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            err
                        )
                    )
                }
            },
            Err(e) => return Err(e)
        };


        let state_client = if let Some(conf) = config {
            tikv_client::RawClient::new_with_config(
                pd_endpoints,
                conf
            ).await.map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string()
                )
            })?
        } else {
            tikv_client::RawClient::new(
                pd_endpoints
            ).await.map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string()
                )
            })?
        };

        let task_cache = LruCache::new(
            NonZeroUsize::new(250).ok_or(
                std::io::Error::new(
                    std::io::ErrorKind::Other, 
                    "Unable to generate NonZeroUsize"
                )
            )?
        );

        Ok(Self {
            network: network.to_string(),
            next_port,
            handles,
            vmlist,
            state_client,
            task_cache,
        })
    }

    pub async fn run(
        mut self,
        mut message_rx: tokio::sync::mpsc::Receiver<VmManagerMessage>,
        mut stop_rx: tokio::sync::mpsc::Receiver<()>
    ) -> std::io::Result<()> {
        loop {
            tokio::select! {
                message = message_rx.recv() => {
                    if let Some(m) = message {
                        match self.handle_vmm_message(m).await {
                            Err(e) => {
                                log::error!("{e}");
                            }
                            _ => {}
                        }
                    }
                },
                stop = stop_rx.recv() => {
                    if let Some(_) = stop {
                        break
                    }
                },
                status = self.handles.next() => {
                    if let Some(Ok(Ok((owner, task_id, task_status)))) = status {
                        match update_task_status(
                            self.state_client.clone(),
                            owner,
                            task_id,
                            task_status,
                            &mut self.task_cache
                        ).await {
                            Err(e) => log::error!("{e}"),
                            _ => {} 
                        }
                    }
                },
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(180)) => {
                    match self.refresh_vmlist().await {
                        Err(e) => log::error!("{e}"),
                        _ => {}
                    }
                }
            }
        }

        return Ok(())
    }

    pub async fn refresh_vmlist(&mut self) -> std::io::Result<()> {
        let vmlist = match std::process::Command::new("lxc")
            .args(["list", "--format", "json"])
            .output() {
            Ok(output) => {
                if output.status.success() {
                    let vmlist_str = std::str::from_utf8(
                        &output.stdout
                    ).map_err(|e| {
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e.to_string()
                        )
                    })?;
                    let vmlist = serde_json::from_str(
                        vmlist_str
                    ).map_err(|e| { std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e.to_string()
                        )
                    })?;
                    vmlist
                } else {
                    let err = std::str::from_utf8(
                        &output.stderr
                    ).map_err(|e| {
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e.to_string()
                        )
                    })?.to_string();
                    return Err(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            err
                        )
                    )
                }
            },
            Err(e) => return Err(e)
        };

        self.vmlist = vmlist;

        Ok(())
    }


    async fn handle_vmm_message(
        &mut self,
        message: VmManagerMessage
    ) -> std::io::Result<()> {
        match message {
            VmManagerMessage::NewInstance { 
                params, 
                task_id 
            } => {
                return self.launch_instance(params, task_id).await
            }
            VmManagerMessage::StartInstance { 
                params, 
                sig, 
                task_id 
            } => {
                return self.start_instance(params, sig, task_id).await
            }
            VmManagerMessage::InjectAuth { 
                params, 
                sig, 
                task_id 
            } => {
                return self.inject_authorization(params, sig, task_id).await
            }
            VmManagerMessage::StopInstance { 
                params, 
                sig, 
                task_id 
            } => {
                return self.stop_instance(params, sig, task_id).await
            }
            VmManagerMessage::DeleteInstance { 
                params, 
                sig, 
                task_id 
            } => {
                return self.delete_instance(params, sig, task_id).await
            }
            VmManagerMessage::ExposePorts { 
                params, 
                sig, 
                task_id 
            } => {
                return self.expose_ports(params, sig, task_id).await
            }
        }
    }

    async fn handle_start_output_success(
        &mut self,
        owner: [u8; 20],
        task_id: TaskId
    ) -> std::io::Result<()> {
        update_task_status(
            self.state_client.clone(),
            owner,
            task_id,
            TaskStatus::Success,
            &mut self.task_cache
        ).await
    }

    async fn handle_start_output_failure(
        &mut self,
        output: std::process::Output,
        owner: [u8; 20],
        task_id: TaskId
    ) -> std::io::Result<()> {
        let err_string = std::str::from_utf8(
            &output.stderr
        ).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            )
        })?;
        update_task_status(
            self.state_client.clone(),
            owner,
            task_id,
            TaskStatus::Failure(
                err_string.to_string()
            ),
            &mut self.task_cache
        ).await
    }

    async fn handle_start_output(
        &mut self,
        output: std::process::Output,
        owner: [u8; 20],
        task_id: TaskId
    ) -> std::io::Result<()> {
        if output.status.success() {
            self.handle_start_output_success(
                owner,
                task_id
            ).await
        } else {
            self.handle_start_output_failure(
                output,
                owner, 
                task_id
            ).await
        }
    }

    async fn start_instance(
        &mut self,
        params: InstanceStartParams,
        sig: String,
        task_id: TaskId
    ) -> std::io::Result<()> {
        let payload = serde_json::json!({
            "command": "start",
            "name": &params.name,
            "console": params.console,
            "stateless": params.stateless
        }).to_string();

        let mut hasher = sha3::Sha3_256::new();
        hasher.update(
            payload.as_bytes()
        );
        let hash = hasher.finalize().to_vec();
        let owner = recover_owner_address(
            hash,
            sig,
            params.recovery_id
        )?;

        let namespace = recover_namespace(
            owner,
            &params.name
        );

        if let Ok(()) = verify_ownership(
            self.state_client.clone(),
            owner,
            namespace
        ).await {
            let mut command = std::process::Command::new("lxc");
            command.arg("start").arg(&params.name);

            if params.console {
                command.arg("--console");
            }

            if params.stateless {
                command.arg("--stateless");
            }

            match command.output() {
                Ok(o) => {
                    self.handle_start_output(
                        o,
                        owner,
                        task_id
                    ).await?
                },
                Err(e) => {
                    return Err(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e.to_string(),
                        )
                    )
                }
            };

            return Ok(())
        } 

        return Err(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "Unable to verify ownership of instance"
            )
        )

    }

    async fn handle_inject_authorization_output_success(
        &mut self,
        owner: [u8; 20],
        task_id: TaskId
    ) -> std::io::Result<()> {
        update_task_status(
            self.state_client.clone(),
            owner,
            task_id,
            TaskStatus::Success,
            &mut self.task_cache
        ).await
    }

    async fn handle_inject_authorization_output_failure(
        &mut self,
        output: std::process::Output,
        owner: [u8; 20],
        task_id: TaskId
    ) -> std::io::Result<()> {
        let err_str = std::str::from_utf8(
            &output.stderr
        ).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            )
        })?.to_string();
        update_task_status(
            self.state_client.clone(),
            owner,
            task_id,
            TaskStatus::Failure(
                err_str
            ),
            &mut self.task_cache
        ).await
    }

    async fn handle_inject_authorization_output(
        &mut self,
        output: std::process::Output,
        owner: [u8; 20],
        task_id: TaskId
    ) -> std::io::Result<()> {
        if output.status.success() {
            self.handle_inject_authorization_output_success(
                owner,
                task_id
            ).await
        } else {
            self.handle_inject_authorization_output_failure(
                output,
                owner,
                task_id
            ).await
        }
    }

    async fn inject_authorization(
        &mut self,
        params: InstanceAddPubkeyParams,
        sig: String, 
        task_id: TaskId
    ) -> std::io::Result<()> {
        let payload = serde_json::json!({
            "command": "injectAuth",
            "name": &params.name,
            "pubkey": &params.pubkey
        }).to_string();

        let mut hasher = Sha3_256::new();
        hasher.update(payload.as_bytes());
        let hash = hasher.finalize().to_vec();

        let owner = recover_owner_address(hash, sig, params.recovery_id)?;
        let namespace = recover_namespace(owner, &params.name);

        if let Ok(()) = verify_ownership(
            self.state_client.clone(),
            owner,
            namespace
        ).await {
            let echo = format!(
                r#""echo '{}' >> ~/.ssh/authorized_keys""#,
                params.pubkey
            );
            let output = std::process::Command::new("lxc")
                .arg("exec")
                .arg(&params.name)
                .arg("--")
                .arg("bash")
                .arg("-c")
                .arg(&echo)
                .output()?;
            

            return self.handle_inject_authorization_output(
                output,
                owner,
                task_id
            ).await
        }

        return Err(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "Unable to verify ownership"
            )
        )
    }

    async fn handle_stop_success(
        &mut self,
        owner: [u8; 20],
        task_id: TaskId
    ) -> std::io::Result<()> {
        update_task_status(
            self.state_client.clone(),
            owner,
            task_id,
            TaskStatus::Success,
            &mut self.task_cache
        ).await?;
        Ok(())
    }

    async fn handle_stop_failure(
        &mut self,  
        output: std::process::Output,
        owner: [u8; 20],
        task_id: TaskId
    ) -> std::io::Result<()> {
        let err_str = std::str::from_utf8(
            &output.stderr
        ).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("{e:#?}")
            )
        })?.to_string();

        update_task_status(
            self.state_client.clone(),
            owner,
            task_id, 
            TaskStatus::Failure(
                err_str
            ),
            &mut self.task_cache
        ).await?;
        return Ok(())
    }

    async fn handle_stop_output_and_response(
        &mut self,
        output: std::process::Output,
        owner: [u8; 20],
        task_id: TaskId
    ) -> std::io::Result<()> {
        if output.status.success() {
            println!("Successfully shutdown vm...");
            self.handle_stop_success(
                owner,
                task_id
            ).await?;
            Ok(())
        } else {
            self.handle_stop_failure(
                output,
                owner,
                task_id
            ).await?;
            return Err(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Failed to stop instance..."
                )
            )
        }
    }

    async fn stop_instance(
        &mut self,
        params: InstanceStopParams,
        sig: String,
        task_id: TaskId
    ) -> std::io::Result<()> {
        let payload = serde_json::json!({
            "command": "stop",
            "name": &params.name, 
        }).to_string();

        let mut hasher = Sha3_256::new();
        hasher.update(
            payload.as_bytes()
        );
        let hash = hasher.finalize().to_vec();

        let owner = recover_owner_address(
            hash,
            sig,
            params.recovery_id
        )?;

        let namespace = recover_namespace(
            owner,
            &params.name
        );

        if let Ok(()) = verify_ownership(
            self.state_client.clone(),
            owner,
            namespace
        ).await {
            println!("attempting to shutdown {}", &params.name);
            let output = std::process::Command::new("lxc")
                .args(["stop", &params.name])
                .output()?;

            println!("Retrieved output...");

            self.handle_stop_output_and_response(
                output,
                owner,
                task_id
            ).await?;
            return Ok(())
        }

        return Err(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "Unable to verify ownership",
            )
        )
    }


    async fn handle_delete_instance_output_success(
        &mut self,
        owner: [u8; 20],
        task_id: TaskId
    ) -> std::io::Result<()> {
        update_task_status(
            self.state_client.clone(),
            owner,
            task_id,
            TaskStatus::Success,
            &mut self.task_cache
        ).await
    }

    async fn handle_delete_instance_output_failure(
        &mut self,
        output: std::process::Output,
        owner: [u8; 20],
        task_id: TaskId
    ) -> std::io::Result<()> {
        let err_str = std::str::from_utf8(
            &output.stderr
        ).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            )
        })?.to_string();

        update_task_status(
            self.state_client.clone(),
            owner,
            task_id,
            TaskStatus::Failure(
                err_str
            ),
            &mut self.task_cache
        ).await
    }

    async fn handle_delete_instance_output(
        &mut self,
        output: std::process::Output,
        owner: [u8; 20],
        task_id: TaskId
    ) -> std::io::Result<()> {
        if output.status.success() {
            self.handle_delete_instance_output_success(
                owner,
                task_id
            ).await
        } else {
            self.handle_delete_instance_output_failure(
                output,
                owner,
                task_id
            ).await
        }
    }

    async fn delete_instance(
        &mut self,
        params: InstanceDeleteParams,
        sig: String,
        task_id: TaskId
    ) -> std::io::Result<()> {
        let payload = serde_json::json!({
            "command": "delete",
            "name": params.name
        }).to_string();

        let mut hasher = Sha3_256::new();
        hasher.update(payload.as_bytes());
        let hash = hasher.finalize().to_vec();

        let owner = recover_owner_address(hash, sig, params.recovery_id)?;
        let mut command = std::process::Command::new("lxc");
        command.arg("delete").arg(&params.name);

        if params.interactive {
            command.arg("--interactive");
        }

        if params.force {
            command.arg("--force");
        }

        let output = command.output()?;


        self.handle_delete_instance_output(
            output,
            owner,
            task_id
        ).await
    }

    async fn expose_ports(
        &mut self,
        params: InstanceExposePortParams,
        sig: String,
        task_id: TaskId
    ) -> std::io::Result<()> {
        let payload = serde_json::json!({
            "command": "exposePort",
            "name": &params.name,
            "ports": &params.port,
        }).to_string();

        let mut hasher = Sha3_256::new();
        hasher.update(
            payload.as_bytes()
        );
        let hash = hasher.finalize().to_vec();

        let owner = recover_owner_address(
            hash,
            sig,
            params.recovery_id
        )?;

        let namespace = recover_namespace(
            owner,
            &params.name
        );

        if let Ok(()) = verify_ownership(
            self.state_client.clone(),
            owner,
            namespace.clone()
        ).await {
            for port in params.port {
                let state_client = self.state_client.clone();
                self.refresh_vmlist().await?;
                let vmlist = self.vmlist.clone();
                let next_port = self.next_port;
                let inner_namespace = namespace.clone();
                let inner_task_id = task_id.clone();
                let handle: JoinHandle<std::io::Result<([u8; 20], TaskId, TaskStatus)>> = tokio::spawn(
                    async move {
                        let (owner, task_id, task_status) = update_iptables(
                            state_client.clone(),
                            vmlist.clone(),
                            owner,
                            inner_namespace.clone(),
                            next_port, 
                            inner_task_id.clone(),
                            port 
                        ).await?;
                        Ok((owner, task_id, task_status))
                    }
                );
                self.next_port += 1;
                self.handles.push(handle);
            }

        }
        Ok(())
    }

    async fn handle_create_output_success(
        &mut self,
        task_id: TaskId,
        owner: [u8; 20],
        namespace: Namespace,
    ) -> std::io::Result<()> {
        let next_port = self.next_port;
        let state_client = self.state_client.clone();
        self.refresh_vmlist().await?;
        let vm_list = self.vmlist.clone();

        let handle: JoinHandle<std::io::Result<([u8; 20], TaskId, TaskStatus)>> = tokio::spawn(
            async move {
                println!("Sleeping for 2 minutes to allow instance to fully launch...");
                tokio::time::sleep(tokio::time::Duration::from_secs(120)).await;
                println!("Updating iptables...");
                let (owner, task_id, task_status) = update_iptables(
                    state_client,
                    vm_list,
                    owner,
                    namespace,
                    next_port, 
                    task_id.clone(),
                    22
                ).await?;
                Ok((owner, task_id, task_status))
        });

        self.handles.push(handle);

        return Ok(())
    }

    async fn handle_create_output_failure_and_response(
        &mut self,
        output: &std::process::Output,
        owner: [u8; 20],
        task_id: TaskId
    ) -> std::io::Result<()> {
        let err_str = std::str::from_utf8(
            &output.stderr
        ).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            )
        })?.to_string();
        update_task_status(
            self.state_client.clone(),
            owner,
            task_id, 
            TaskStatus::Failure(err_str.clone()),
            &mut self.task_cache
        ).await?;

        return Err(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                err_str
            )
        )
    }

    async fn handle_create_output_and_response(
        &mut self,
        output: std::process::Output,
        task_id: TaskId,
        owner: [u8; 20],
        namespace: Namespace,
    ) -> std::io::Result<()> {
        if output.status.success() {
            println!("successfully created instance, handling output...");
            self.handle_create_output_success(
                task_id,
                owner,
                namespace,
            ).await
        } else {
            self.handle_create_output_failure_and_response(
                &output,
                owner,
                task_id
            ).await
        }
    }

    pub async fn launch_instance(
        &mut self,
        params: InstanceCreateParams,
        task_id: TaskId
    ) -> std::io::Result<()> {
        println!("Attempting to start instance...");

        let payload = params.into_payload(); 

        let mut hasher = Sha3_256::new();

        hasher.update(
            payload.as_bytes()
        );

        let hash = hasher.finalize().to_vec();

        let owner = recover_owner_address(
            hash,
            params.sig.clone(),
            params.recovery_id
        )?;

        println!("Recovered owner");
        let namespace = recover_namespace(
            owner,
            &params.name
        );

        println!("Recovered namespace");

        let output = std::process::Command::new("lxc")
            .arg("launch")
            .arg(
                &format!(
                    "{}:{}",
                    params.distro,
                    params.version
                )
            )
            .arg(
                &format!(
                    "{}",
                    &namespace
                )
            )
            .arg("--vm")
            .arg("-t")
            .arg(&params.vmtype.to_string())
            .arg("--network")
            .arg(&self.network.clone())
            .output()?;

        self.handle_create_output_and_response(
            output,
            task_id.clone(),
            owner, 
            namespace.clone(), 
        ).await?;

        println!("Attempting to update account");
        update_account(
            self.state_client.clone(),
            self.vmlist.clone(),
            owner,
            namespace.clone(),
            task_id.clone(),
            TaskStatus::Pending,
            vec![]
        ).await?;

        println!("Successfully updated account");

        Ok(())
    }
}
