use std::{collections::HashMap, fmt::Display};
use serde::{Serialize, Deserialize};
use crate::{vm_info::VmInfo, params::ServiceType};


#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Namespace(String);

impl Namespace {
    pub fn new(namespace: String) -> Self {
        Self(namespace)
    }

    pub fn inner(&self) -> String {
        self.0.clone()
    }
}

impl Display for Namespace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct TaskId(String);

impl TaskId {
    pub fn new(taskid: String) -> Self {
        Self(taskid)
    }
    
    pub fn task_id(&self) -> String {
        self.0.clone()
    }
}

impl Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.task_id())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Success,
    Failure(String),
}

impl Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "pending"),
            TaskStatus::Success => write!(f, "success"),
            TaskStatus::Failure(err_string) => write!(f, "Failure {}", err_string)
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExposedPort {
    port: u16,
    service_description: Option<ServiceType>,
}

impl ExposedPort {
    pub fn new(port: u16, service_description: Option<ServiceType>) -> Self {
        Self { port, service_description }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Account {
    address: [u8; 20],
    namespaces: HashMap<Namespace, Option<VmInfo>>,
    exposed_ports: HashMap<Namespace, Vec<ExposedPort>>,
    tasks: HashMap<TaskId, TaskStatus>
}

impl Account {
    pub fn new(
        address: [u8; 20],
        namespaces: impl IntoIterator<Item = (Namespace, Option<VmInfo>)>, 
        exposed_ports: impl IntoIterator<Item = (Namespace, Vec<ExposedPort>)>,
        tasks: impl IntoIterator<Item = (TaskId, TaskStatus)>
    ) -> Self {
        let namespaces = namespaces.into_iter().collect();
        let tasks = tasks.into_iter().collect();
        let exposed_ports = exposed_ports.into_iter().collect();
        Self { address, namespaces, tasks, exposed_ports }
    }

    pub fn address(&self) -> [u8; 20] {
        self.address
    }

    pub fn namespaces(&self) -> HashMap<Namespace, Option<VmInfo>> {
        self.namespaces.clone()
    }

    pub fn update_namespace(&mut self, namespace: &Namespace, vminfo: Option<VmInfo>) {
        self.namespaces.insert(namespace.clone(), vminfo);
    }

    pub fn update_exposed_ports(&mut self, namespace: &Namespace, exposed_ports: Vec<ExposedPort>) {
        if let Some(entry) = self.exposed_ports.get_mut(namespace) {
            entry.extend(exposed_ports.into_iter())
        } else {
            self.exposed_ports.insert(namespace.clone(), exposed_ports);
        }
    }

    pub fn tasks(&self) -> HashMap<TaskId, TaskStatus> {
        self.tasks.clone()
    }

    pub fn tasks_mut(&mut self) -> &mut HashMap<TaskId, TaskStatus> {
        &mut self.tasks
    }

    pub fn update_task_status(&mut self, task_id: &TaskId, task_status: TaskStatus) {
        if let Some(entry) = self.tasks.get_mut(task_id) {
            *entry = task_status;
        } else {
            self.tasks.insert(task_id.clone(), task_status);
        }
    }

    pub fn get_task_status(&self, task_id: &TaskId) -> Option<&TaskStatus> {
        self.tasks.get(task_id)
    }
}
