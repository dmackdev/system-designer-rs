use bevy::prelude::{Entity, EventWriter, IntoSystemAppConfig, OnEnter, Plugin, Query};

use crate::{
    game_state::GameState,
    message::{Message, Request, SendMessageEvent},
    node::{client::Client, Hostname, NodeConnections, NodeName},
};

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(start_simulation.in_schedule(OnEnter(GameState::Simulate)));
    }
}

fn start_simulation(
    mut client_query: Query<(Entity, &mut Client, &NodeConnections)>,
    hostnames: Query<(Entity, &Hostname)>,
    mut events: EventWriter<SendMessageEvent>,
) {
    // Test:

    let (client_entity, mut client, client_connections) = client_query.single_mut();

    let request_config = client.request_configs.remove(0);

    let recipient = hostnames
        .iter()
        .find(|(_, node_name)| node_name.0 == request_config.url);

    if let Some((recipient, _)) = recipient {
        if !client_connections.is_connected_to(recipient) {
            return;
        }

        let request = Request::try_from(request_config).unwrap();

        events.send(SendMessageEvent {
            sender: client_entity,
            recipients: vec![recipient],
            message: Message::Request(request),
        });
    }
}
