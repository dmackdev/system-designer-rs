use std::collections::{hash_map::Iter, HashMap};

use bevy::{
    ecs::system::SystemParam,
    prelude::{Bundle, Component, Entity, Query},
};

use crate::message::MessageComponent;

pub mod client;
pub mod database;
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

    pub fn node_name(mut self, name: String) -> Self {
        self.node_name.0 = name;
        self
    }
}

pub trait SystemNodeTrait {
    fn start_simulation(&mut self);
    fn reset(&mut self);
    fn handle_message(&mut self, message: MessageComponent);
    fn can_be_edited(&self) -> bool;
}

#[derive(Component)]
pub struct SystemNode;

#[derive(Component, Clone, Copy, Debug, strum::Display, PartialEq, Eq)]
pub enum NodeType {
    Client,
    Server,
    Database,
}

impl NodeType {
    pub fn get_texture_path(&self) -> String {
        let t = self.to_string().to_ascii_lowercase();

        format!("textures/system_components/{t}.png")
    }

    pub fn is_valid_connection(&self, other: &Self) -> bool {
        match self {
            NodeType::Client => [&NodeType::Server].contains(&other),
            NodeType::Server => {
                [&NodeType::Client, &NodeType::Server, &NodeType::Database].contains(&other)
            }
            NodeType::Database => [&NodeType::Server].contains(&other),
        }
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

#[derive(SystemParam)]
pub struct HostnameConnections<'w, 's> {
    hostnames: Query<'w, 's, (Entity, &'static Hostname)>,
    connections: Query<'w, 's, &'static NodeConnections>,
}

impl<'w, 's> HostnameConnections<'w, 's> {
    pub fn get_connected_entity_by_hostname(
        &self,
        from: Entity,
        to_name: &String,
    ) -> Option<Entity> {
        self.hostnames
            .iter()
            .find(|(_, node_name)| &node_name.0 == to_name)
            .and_then(|(recipient, _)| {
                self.connections
                    .get(from)
                    .unwrap()
                    .is_connected_to(recipient)
                    .then_some(recipient)
            })
    }
}

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

    pub fn line_entities(&self) -> Vec<Entity> {
        self.connections.values().cloned().collect()
    }
}
