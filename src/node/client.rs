use std::fmt::Display;

use bevy::prelude::{Component, Entity, EventWriter, Query};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::EnumIter;
use uuid::Uuid;

use crate::message::{Message, MessageComponent, Request, Response, SendMessageEvent};

use super::{HostnameConnections, SystemNodeTrait};

#[derive(Component, Clone, Debug, Default)]
pub struct Client {
    pub request_configs: Vec<RequestConfig>,
    pub state: ClientState,
    curr_request_idx: usize,
    can_be_edited: bool,
}

impl Client {
    pub fn new() -> Self {
        Self {
            request_configs: vec![RequestConfig::default()],
            can_be_edited: true,
            ..Default::default()
        }
    }

    pub fn editable(mut self, editable: bool) -> Self {
        self.can_be_edited = editable;
        self
    }

    pub fn request_configs(mut self, request_configs: Vec<RequestConfig>) -> Self {
        self.request_configs = request_configs;
        self
    }

    fn reset(&mut self) {
        self.state = ClientState::SimulationNotStarted;
        self.curr_request_idx = 0;

        for r in self.request_configs.iter_mut() {
            r.reset();
        }
    }

    pub fn verify(&mut self) -> bool {
        self.request_configs
            .iter_mut()
            .map(|request_config| request_config.verify())
            .all(|b| b)
    }

    pub fn is_valid(&self) -> bool {
        self.request_configs.iter().all(|config| config.is_valid())
    }
}

impl SystemNodeTrait for Client {
    fn start_simulation(&mut self) {
        self.state = ClientState::SendNextRequest;
    }

    fn handle_message(&mut self, message: MessageComponent) {
        if let ClientState::Waiting(trace_id) = self.state {
            println!("HANDLING MESSAGE FOR CLIENT:");
            println!("{:?}", message);

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

    fn can_be_edited(&self) -> bool {
        self.can_be_edited
    }

    fn reset(&mut self) {
        self.reset();
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
    expectations: Vec<ResponseExpectation>,
    pub expectations_results: Vec<(bool, String)>,
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
            expectations: vec![],
            expectations_results: vec![],
        }
    }
}

impl RequestConfig {
    fn reset(&mut self) {
        self.response = None;
        self.expectations_results = vec![];
    }

    fn verify(&mut self) -> bool {
        match &self.response {
            Some(response) => {
                let mut passed = true;

                for exp in self.expectations.iter() {
                    let result = exp.verify(response);

                    if !result.0 {
                        passed = false;
                    }

                    self.expectations_results.push(result)
                }

                passed
            }
            None => false,
        }
    }

    pub fn is_url_valid(&self) -> bool {
        !self.url.is_empty()
    }

    pub fn is_path_valid(&self) -> bool {
        !self.path.is_empty()
    }

    pub fn is_body_valid(&self) -> bool {
        let res: Result<Value, _> = serde_json::from_str(&self.body);
        res.is_ok()
    }

    fn is_valid(&self) -> bool {
        self.is_url_valid() && self.is_path_valid() && self.is_body_valid()
    }
}

#[derive(Deserialize, Clone, Debug)]
pub enum ResponseExpectation {
    Status(u16),
    ExactBody(Value),
    ExactSet(Vec<Value>),
}
impl ResponseExpectation {
    fn verify(&self, response: &Response) -> (bool, String) {
        match self {
            ResponseExpectation::Status(exp_status) => {
                get_expectation_result("status", exp_status, &response.status)
            }
            ResponseExpectation::ExactBody(expected) => {
                get_expectation_result("body", expected, &response.data)
            }
            ResponseExpectation::ExactSet(expected_elems) => match &response.data {
                Value::Array(actual_elems) => {
                    let passed = expected_elems.len() == actual_elems.len()
                        && actual_elems.iter().all(|e| expected_elems.contains(e));

                    let mut msg = format!("Expected {} {:?}", "body", expected_elems);

                    if !passed {
                        msg.push_str(&format!(", received {:?}", actual_elems));
                    }

                    println!("ExactSet expectation:");
                    println!("{:?}", msg);

                    (passed, msg)
                }
                _ => (false, String::default()),
            },
        }
    }
}

fn get_expectation_result<T: Display + PartialEq>(
    name: &str,
    expected: T,
    actual: T,
) -> (bool, String) {
    let passed = expected == actual;

    let mut msg = format!("Expected {} {}", name, expected);

    if !passed {
        msg.push_str(&format!(", received {}", actual));
    }

    (passed, msg)
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
    mut client_query: Query<(Entity, &mut Client)>,
    mut events: EventWriter<SendMessageEvent>,
    hostname_connections: HostnameConnections,
) {
    for (client_entity, mut client) in client_query.iter_mut() {
        if let ClientState::SendNextRequest = client.state {
            // Send first request
            let idx = client.curr_request_idx;
            if let Some(request_config) = client.request_configs.get_mut(idx) {
                let recipient = hostname_connections
                    .get_connected_entity_by_hostname(client_entity, &request_config.url);

                if let Some(recipient) = recipient {
                    let trace_id = request_config.trace_id;
                    let request: Request = request_config.into();

                    events.send(SendMessageEvent {
                        sender: client_entity,
                        recipients: vec![recipient],
                        message: Message::Request(request),
                        trace_id,
                    });

                    client.state = ClientState::Waiting(trace_id);
                } else {
                    request_config
                        .expectations_results
                        .push((false, "ERR_CONNECTION_REFUSED".to_string())); // TODO: make better
                    client.state = ClientState::Finished;
                }
            }
        };
    }
}
