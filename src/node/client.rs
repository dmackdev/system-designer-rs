use std::collections::VecDeque;

use bevy::prelude::{Component, Entity, EventWriter, Query};
use serde::{Deserialize, Serialize};
use strum::EnumIter;
use uuid::Uuid;

use crate::message::{Message, MessageComponent, Request, Response, SendMessageEvent};

use super::{Hostname, NodeConnections, SystemNodeTrait};

#[derive(Component, Clone, Debug, Default)]
pub struct Client {
    pub request_configs: VecDeque<RequestConfig>,
    pub state: ClientState,
    curr_request_idx: usize,
}

impl Client {
    pub fn new() -> Self {
        Self {
            request_configs: VecDeque::from_iter([RequestConfig::default()]),
            ..Default::default()
        }
    }
}

impl SystemNodeTrait for Client {
    fn start_simulation(&mut self) {
        self.state = ClientState::SendNextRequest;
    }

    fn handle_message(&mut self, message: MessageComponent) {
        println!("HANDLING MESSAGE FOR CLIENT:");
        println!("{:?}", message);

        if let ClientState::Waiting(trace_id) = self.state {
            if trace_id == message.trace_id {
                if let Message::Response(response) = message.message {
                    println!("RECEIVED CORRECT RESPONSE");

                    let request_config = self
                        .request_configs
                        .iter_mut()
                        .find(|r| r.trace_id == message.trace_id)
                        .unwrap();

                    request_config.response = Some(response);
                    self.curr_request_idx += 1;

                    if self.curr_request_idx >= self.request_configs.len() {
                        println!("CLIENT SENT ALL REQUESTS");
                        self.state = ClientState::Finished;
                    } else {
                        self.state = ClientState::SendNextRequest;
                    }
                    return;
                }
            }
        }

        println!("RECEIVED UNEXPECTED RESPONSE");
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct RequestConfig {
    pub url: String,
    pub path: String,
    pub method: HttpMethod,
    pub body: String,
    trace_id: Uuid,
    pub response: Option<Response>,
}

impl Default for RequestConfig {
    fn default() -> Self {
        Self {
            body: "{}".to_string(),
            path: "/".to_string(),
            url: "".to_string(),
            method: HttpMethod::default(),
            trace_id: Uuid::new_v4(),
            response: None,
        }
    }
}

#[derive(
    Serialize,
    Deserialize,
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    EnumIter,
    strum::Display,
    Hash,
)]
pub enum HttpMethod {
    #[default]
    Get,
    Post,
    Put,
    Delete,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ClientState {
    #[default]
    SimulationNotStarted,
    SendNextRequest,
    Waiting(Uuid),
    Finished,
}

pub fn client_system(
    mut client_query: Query<(Entity, &mut Client, &NodeConnections)>,
    hostnames: Query<(Entity, &Hostname)>,
    mut events: EventWriter<SendMessageEvent>,
) {
    for (client_entity, mut client, client_connections) in client_query.iter_mut() {
        if let ClientState::SendNextRequest = client.state {
            // Send first request
            if let Some(request_config) = client.request_configs.get(client.curr_request_idx) {
                let recipient = hostnames
                    .iter()
                    .find(|(_, node_name)| node_name.0 == request_config.url);

                if let Some((recipient, _)) = recipient {
                    if !client_connections.is_connected_to(recipient) {
                        continue;
                    }

                    let trace_id = request_config.trace_id;
                    let request = Request::try_from(request_config).unwrap();

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
