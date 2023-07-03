use std::collections::{HashMap, VecDeque};

use bevy::prelude::{warn, Bundle, Component, Entity, EventWriter, Query};
use boa_engine::{property::Attribute, Context, JsValue};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::message::{Message, MessageComponent, Request, Response, SendMessageEvent};

use super::{Hostname, NodeConnections, SystemNodeTrait};

#[derive(Component, Clone, Debug)]
pub struct Server {
    pub request_handler: String,
    pub message_queue: VecDeque<MessageComponent>,
    pub state: ServerState,
    active_executions: HashMap<Uuid, ServerExecution>,
}

const EXAMPLE_REQUEST_HANDLER: &str = r#"
const requestHandler = function* () {
  const fetchResult = yield http.get("server1", "", {});

  return response(200, fetchResult);
}
"#;

impl Default for Server {
    fn default() -> Self {
        Self {
            request_handler: EXAMPLE_REQUEST_HANDLER.to_string(),
            message_queue: Default::default(),
            state: Default::default(),
            active_executions: Default::default(),
        }
    }
}

impl SystemNodeTrait for Server {
    fn start_simulation(&mut self) {
        self.state = ServerState::Active;
    }

    fn handle_message(&mut self, message: MessageComponent) {
        println!("HANDLING MESSAGE FOR SERVER:");
        println!("{:?}", message);

        self.message_queue.push_back(message);
    }
}

#[derive(Bundle, Default)]
pub struct ServerBundle {
    server: Server,
    hostname: Hostname,
}

#[derive(Clone, Debug, Default)]
pub enum ServerState {
    #[default]
    SimulationNotStarted,
    Active,
}

#[allow(clippy::single_match)]
pub fn server_system(
    mut server_query: Query<(Entity, &mut Server, &NodeConnections)>,
    mut events: EventWriter<SendMessageEvent>,
    hostnames: Query<(Entity, &Hostname)>,
) {
    for (server_entity, mut server, connections) in server_query.iter_mut() {
        match server.state {
            ServerState::Active => {
                let message_queue = server.message_queue.drain(..).collect::<Vec<_>>();

                for message in message_queue {
                    let mut execution = match message.message {
                        Message::Request(request) => ServerExecution::new(
                            server.request_handler.clone(),
                            request,
                            message.sender,
                            message.trace_id,
                        ),
                        Message::Response(response) => {
                            let mut execution =
                                server.active_executions.remove(&message.trace_id).unwrap();

                            execution.yield_values.push(YieldValue::Response(response));

                            execution
                        }
                    };

                    let res = execution.execute();

                    println!("{:?}", res);

                    match (res.done, res.value) {
                        (true, YieldValue::Response(response)) => {
                            events.send(SendMessageEvent {
                                sender: server_entity,
                                recipients: vec![execution.original_sender],
                                message: Message::Response(response),
                                trace_id: execution.original_trace_id,
                            });
                        }
                        (false, YieldValue::Request(new_request)) => {
                            let new_trace_id = Uuid::new_v4();

                            let recipient = hostnames
                                .iter()
                                .find(|(_, node_name)| node_name.0 == new_request.url);

                            if let Some((recipient, _)) = recipient {
                                if connections.is_connected_to(recipient) {
                                    events.send(SendMessageEvent {
                                        sender: server_entity,
                                        recipients: vec![recipient],
                                        message: Message::Request(new_request),
                                        trace_id: new_trace_id,
                                    });

                                    server.active_executions.insert(new_trace_id, execution);
                                }
                            }
                        }
                        _ => warn!("Unexpected yield value"),
                    };
                }
            }
            _ => {}
        };
    }
}

#[derive(Clone, Debug)]
struct ServerExecution {
    request_handler: String,
    request: Request,
    yield_values: Vec<YieldValue>,
    original_sender: Entity,
    original_trace_id: Uuid,
}

impl ServerExecution {
    fn new(
        request_handler: String,
        request: Request,
        original_sender: Entity,
        original_trace_id: Uuid,
    ) -> Self {
        Self {
            request_handler,
            request,
            yield_values: vec![],
            original_sender,
            original_trace_id,
        }
    }

    // Because we cannot store Context in a Bevy Component (it is not Send + Sync), we instead create
    // a fresh Context and apply all the previous yield values to the generator in turn,
    // in order to get the latest yield value.
    fn execute(&mut self) -> GeneratorResultValue {
        let mut context = Context::default();

        let request = serde_json::to_value(&self.request).unwrap();
        let request = JsValue::from_json(&request, &mut context).unwrap();

        context.register_global_property("request", request, Attribute::all());

        let http_script = r#"
const http = {
  get: function(url, path, params) { return { Request: { url, path, method: "Get", body: null, params }}; },
}
        "#;

        context.eval(http_script).unwrap();

        let insert_db_script = r#"
function insertDb(db, values) {
  return { Db: { Insert: { db_name: db, values }}};
}
          "#;

        context.eval(insert_db_script).unwrap();

        let response_script = r#"
function response(status, data) {
  return { Response: { status, data } };
}
          "#;

        context.eval(response_script).unwrap();

        context.eval(&self.request_handler).unwrap();

        let generator_setup = r#"
const gen = requestHandler(request);
"#;

        context.eval(generator_setup).unwrap();

        let mut value = context
            .eval(
                r#"
gen.next();
"#,
            )
            .unwrap();

        for prev_yield_value in self.yield_values.iter() {
            let prev = serde_json::to_value(prev_yield_value).unwrap();
            let prev = JsValue::from_json(&prev, &mut context).unwrap();

            context.register_global_property("lastGenResult", prev, Attribute::all());

            value = context
                .eval(
                    r#"
gen.next(lastGenResult);
"#,
                )
                .unwrap();
        }

        let latest_value = value.to_json(&mut context).unwrap();

        serde_json::from_value(latest_value).unwrap()
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct GeneratorResultValue {
    pub done: bool,
    pub value: YieldValue,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum YieldValue {
    Response(Response),
    Request(Request),
}
