use bevy::prelude::{Entity, EventWriter, IntoSystemAppConfig, OnEnter, Plugin, Query, With};

use crate::{
    game_state::GameState,
    message::{Message, Request, SendMessageEvent},
    node::{client::HttpMethod, NodeType, SystemNode},
};

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(start_simulation.in_schedule(OnEnter(GameState::Simulate)));
    }
}

fn start_simulation(
    nodes: Query<&NodeType>,
    node_entities_query: Query<Entity, With<SystemNode>>,
    mut events: EventWriter<SendMessageEvent>,
) {
    // Test:
    let node_types: Vec<_> = nodes.iter().collect();
    let node_entities: Vec<_> = node_entities_query.iter().collect();

    let client = match node_types.first().unwrap() {
        NodeType::Client => node_entities[0],
        NodeType::Server => node_entities[1],
    };

    let server = match node_types.first().unwrap() {
        NodeType::Client => node_entities[1],
        NodeType::Server => node_entities[0],
    };

    events.send(SendMessageEvent {
        sender: client,
        recipients: vec![server],
        message: Message::Request(Request {
            method: HttpMethod::Get,
            ..Default::default()
        }),
    });
}
