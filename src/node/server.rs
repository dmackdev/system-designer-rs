use std::collections::VecDeque;

use bevy::prelude::{Bundle, Component, Entity, EventWriter, Query};
use boa_engine::{property::Attribute, Context, JsValue};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::message::{Message, MessageComponent, Request, Response, SendMessageEvent};

use super::{Hostname, SystemNodeTrait};

#[derive(Component, Clone, Debug)]
pub struct Server {
    pub request_handler: String,
    pub request_queue: VecDeque<(Entity, Uuid, Request)>,
    pub state: ServerState,
}

const EXAMPLE_REQUEST_HANDLER: &str = r#"
const requestHandler = function* () {
  return response(200, request);
}
"#;

impl Default for Server {
    fn default() -> Self {
        Self {
            request_handler: EXAMPLE_REQUEST_HANDLER.to_string(),
            request_queue: Default::default(),
            state: Default::default(),
        }
    }
}

impl SystemNodeTrait for Server {
    fn start_simulation(&mut self) {
        self.state = ServerState::Start;
    }

    fn handle_message(&mut self, message: MessageComponent) {
        println!("HANDLING MESSAGE FOR SERVER:");
        println!("{:?}", message);

        if let Message::Request(request) = message.message {
            self.request_queue
                .push_back((message.sender, message.trace_id, request))
        }
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
    Start,
    Idle,
}

pub fn server_system(
    mut server_query: Query<(Entity, &mut Server)>,
    mut events: EventWriter<SendMessageEvent>,
) {
    for (server_entity, mut server) in server_query.iter_mut() {
        match server.state {
            ServerState::Start => {
                server.state = ServerState::Idle;
            }
            ServerState::Idle => {
                if let Some((sender, trace_id, request)) = server.request_queue.pop_front() {
                    let execution = ServerExecution::new(server.request_handler.clone(), request);

                    let res = execution.execute();

                    if res.done {
                        if let YieldValue::Response(response) = res.value {
                            events.send(SendMessageEvent {
                                sender: server_entity,
                                recipients: vec![sender],
                                message: Message::Response(response),
                                trace_id,
                            });

                            server.state = ServerState::Start;
                        }
                    }
                }
            }
            _ => {}
        };
    }
}

struct ServerExecution {
    context: Context,
}

impl ServerExecution {
    fn new(request_handler: String, request: Request) -> Self {
        let mut context = Context::default();

        let request = serde_json::to_value(request).unwrap();
        let request = JsValue::from_json(&request, &mut context).unwrap();

        context.register_global_property("request", request, Attribute::all());

        context.register_global_property("lastGenResult", JsValue::Undefined, Attribute::all());

        let http_script = r#"
const http = {
get: function(url) { return { Request: { method: "GET", url }}; },
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

        context.eval(request_handler).unwrap();

        let generator_setup = r#"
        const gen = requestHandler(request);
                  "#;

        context.eval(generator_setup).unwrap();

        Self { context }
    }

    fn execute(mut self) -> GeneratorResultValue {
        let value = self
            .context
            .eval(
                r#"
gen.next(lastGenResult);
"#,
            )
            .unwrap();

        let value = value.to_json(&mut self.context).unwrap();

        serde_json::from_value(value).unwrap()
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct GeneratorResultValue {
    pub done: bool,
    pub value: YieldValue,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum YieldValue {
    Response(Response),
    Request,
}
