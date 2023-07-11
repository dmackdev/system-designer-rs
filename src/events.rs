use crate::node::{client::Client, database::Database, server::Server, NodeType};

pub struct AddComponentEvent(pub AddComponentPayload);

impl AddComponentEvent {
    pub fn new_client() -> Self {
        AddComponentEvent(AddComponentPayload::Client(Client::new()))
    }

    pub fn new_server() -> Self {
        AddComponentEvent(AddComponentPayload::Server(Server::default()))
    }

    pub fn new_database() -> Self {
        AddComponentEvent(AddComponentPayload::Database(Database::default()))
    }
}

#[derive(Clone)]
pub enum AddComponentPayload {
    Client(Client),
    Server(Server),
    Database(Database),
}

impl AddComponentPayload {
    pub fn get_node_type(&self) -> NodeType {
        match self {
            AddComponentPayload::Client(_) => NodeType::Client,
            AddComponentPayload::Server(_) => NodeType::Server,
            AddComponentPayload::Database(_) => NodeType::Database,
        }
    }
}
