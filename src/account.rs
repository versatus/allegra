use std::{collections::{HashSet, HashMap}, fmt::Display};
use serde::{Serialize, Deserialize};
use crate::quorum::QuorumId;

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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Success,
    Failure(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Account {
    address: [u8; 20],
    namespaces: HashSet<Namespace>,
    tasks: HashMap<TaskId, TaskStatus>
}

impl Account {
    pub fn new(
        address: [u8; 20],
        namespaces: impl IntoIterator<Item = Namespace>, 
        tasks: impl IntoIterator<Item = (TaskId, TaskStatus)>
    ) -> Self {
        let namespaces = namespaces.into_iter().collect();
        let tasks = tasks.into_iter().collect();
        Self { address, namespaces, tasks }
    }

    pub fn address(&self) -> [u8; 20] {
        self.address
    }

    pub fn namespaces(&self) -> HashSet<Namespace> {
        self.namespaces.clone()
    }

    pub fn tasks(&self) -> HashMap<TaskId, TaskStatus> {
        self.tasks.clone()
    }

    pub fn tasks_mut(&mut self) -> &mut HashMap<TaskId, TaskStatus> {
        &mut self.tasks
    }

    pub fn get_namespace_quorum(&self, namespace: &Namespace) -> Option<&Namespace> {
        self.namespaces.get(namespace)
    }

    pub fn get_task_status(&self, task_id: &TaskId) -> Option<&TaskStatus> {
        self.tasks.get(task_id)
    }
}
