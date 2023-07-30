use crate::node::{client::Client, database::Database, server::Server, Hostname, NodeType};

pub struct AddComponentEvent(pub AddComponentPayload);

impl AddComponentEvent {
    pub fn new_client() -> Self {
        AddComponentEvent(AddComponentPayload::Client(Client::new()))
    }

    pub fn new_server() -> Self {
        AddComponentEvent(AddComponentPayload::Server(
            Hostname::default(),
            Server::default(),
        ))
    }

    pub fn new_database() -> Self {
        AddComponentEvent(AddComponentPayload::Database(
            Hostname::default(),
            Database::new(),
        ))
    }
}

#[derive(Clone)]
pub enum AddComponentPayload {
    Client(Client),
    Server(Hostname, Server),
    Database(Hostname, Database),
}

impl AddComponentPayload {
    pub fn get_node_type(&self) -> NodeType {
        match self {
            AddComponentPayload::Client(_) => NodeType::Client,
            AddComponentPayload::Server(_, _) => NodeType::Server,
            AddComponentPayload::Database(_, _) => NodeType::Database,
        }
    }
}
