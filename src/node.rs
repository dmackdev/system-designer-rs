use std::collections::{hash_map::Iter, HashMap};

use bevy::prelude::{Component, Entity};

#[derive(Component)]
pub struct SystemNode;

#[derive(Component, Clone, Debug)]
pub enum NodeType {
    Client,
    Server,
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

#[derive(Component)]
pub struct NodeConnections {
    // other node entity -> connection line entity
    connections: HashMap<Entity, Entity>,
}

impl NodeConnections {
    pub fn new() -> Self {
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
