use bevy::prelude::{Component, Entity, EventWriter, Query};
use strum::{Display, EnumIter};

use crate::message::{Message, MessageComponent, Request, SendMessageEvent};

use super::{Hostname, NodeConnections, SystemNodeTrait};

#[derive(Component, Clone, Debug, Default)]
pub struct Client {
    pub request_configs: Vec<RequestConfig>,
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, EnumIter, Display)]
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
    Waiting,
}

pub fn client_system(
    mut client_query: Query<(Entity, &mut Client, &NodeConnections)>,
    hostnames: Query<(Entity, &Hostname)>,
    mut events: EventWriter<SendMessageEvent>,
) {
    for (client_entity, mut client, client_connections) in client_query.iter_mut() {
        match client.state {
            ClientState::Start => {
                // Send first request
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

                    client.state = ClientState::Waiting;
                }
            }
            ClientState::Waiting => {}
            _ => {}
        };
    }
}
