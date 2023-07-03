use std::collections::VecDeque;

use bevy::prelude::{Component, Entity, EventWriter, Query};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter};
use uuid::Uuid;

use crate::message::{Message, MessageComponent, Request, SendMessageEvent};

use super::{Hostname, NodeConnections, SystemNodeTrait};

#[derive(Component, Clone, Debug, Default)]
pub struct Client {
    pub request_configs: VecDeque<RequestConfig>,
    pub state: ClientState,
}

impl Client {
    pub fn new() -> Self {
        Default::default()
    }
}

impl SystemNodeTrait for Client {
    fn start_simulation(&mut self) {
        self.state = ClientState::Start;
    }

    fn handle_message(&mut self, message: MessageComponent) {
        println!("HANDLING MESSAGE FOR CLIENT:");
        println!("{:?}", message);

        if let ClientState::Waiting(trace_id) = self.state {
            if trace_id == message.trace_id {
                println!("RECEIVED CORRECT RESPONSE");
                self.state = ClientState::Start;
                return;
            }
        }

        println!("RECEIVED UNEXPECTED RESPONSE");
    }
}

#[derive(Clone, Debug, Default)]
pub struct RequestConfig {
    pub url: String,
    pub path: String,
    pub method: HttpMethod,
    pub body: String,
    pub params: Vec<(String, String)>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq, EnumIter, Display)]
pub enum HttpMethod {
    #[default]
    Get,
    Post,
}

#[derive(Clone, Debug, Default)]
pub enum ClientState {
    #[default]
    SimulationNotStarted,
    Start,
    Waiting(Uuid),
}

pub fn client_system(
    mut client_query: Query<(Entity, &mut Client, &NodeConnections)>,
    hostnames: Query<(Entity, &Hostname)>,
    mut events: EventWriter<SendMessageEvent>,
) {
    for (client_entity, mut client, client_connections) in client_query.iter_mut() {
        if let ClientState::Start = client.state {
            // Send first request
            if let Some(request_config) = client.request_configs.pop_front() {
                let recipient = hostnames
                    .iter()
                    .find(|(_, node_name)| node_name.0 == request_config.url);

                if let Some((recipient, _)) = recipient {
                    if !client_connections.is_connected_to(recipient) {
                        return;
                    }

                    let request = Request::try_from(request_config).unwrap();
                    let trace_id = Uuid::new_v4();

                    events.send(SendMessageEvent {
                        sender: client_entity,
                        recipients: vec![recipient],
                        message: Message::Request(request),
                        trace_id,
                    });

                    client.state = ClientState::Waiting(trace_id);
                }
            }
        };
    }
}
