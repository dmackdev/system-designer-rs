use std::{
    collections::{hash_map::Iter, HashMap},
    fmt::Display,
};

use bevy::prelude::{Bundle, Component, Entity};

use crate::message::MessageComponent;

pub mod client;
pub mod server;

#[derive(Bundle)]
pub struct SystemNodeBundle {
    node: SystemNode,
    node_type: NodeType,
    node_name: NodeName,
    node_connections: NodeConnections,
}

impl SystemNodeBundle {
    pub fn new(node_type: NodeType) -> Self {
        Self {
            node: SystemNode,
            node_type,
            node_name: NodeName::new(),
            node_connections: NodeConnections::new(),
        }
    }
}

pub trait SystemNodeTrait {
    fn start_simulation(&mut self);
    fn handle_message(&mut self, message: MessageComponent);
}

#[derive(Component)]
pub struct SystemNode;

#[derive(Component, Clone, Debug)]
pub enum NodeType {
    Client,
    Server,
}

impl Display for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let s = match self {
            NodeType::Client => "Client",
            NodeType::Server => "Server",
        };
        write!(f, "{}", s)
    }
}

impl NodeType {
    pub fn get_texture_path(&self) -> String {
        let t = match self {
            NodeType::Client => "client",
            NodeType::Server => "server",
        };

        format!("textures/system_components/{t}.png")
    }
}

#[derive(Default, Component)]
pub struct NodeName(pub String);

impl NodeName {
    fn new() -> Self {
        Default::default()
    }
}

#[derive(Default, Component)]
pub struct Hostname(pub String);

#[derive(Component)]
pub struct NodeConnections {
    // other node entity -> connection line entity
    connections: HashMap<Entity, Entity>,
}

impl NodeConnections {
    fn new() -> Self {
        Self {
            connections: HashMap::new(),
        }
    }

    pub fn is_connected_to(&self, other_node: Entity) -> bool {
        self.connections.contains_key(&other_node)
    }

    pub fn add_connection(&mut self, other_node: Entity, line: Entity) {
        self.connections.insert(other_node, line);
    }

    pub fn remove_connection(&mut self, other_node: Entity) {
        self.connections.remove(&other_node);
    }

    pub fn iter(&self) -> Iter<Entity, Entity> {
        self.connections.iter()
    }
}
