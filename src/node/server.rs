use std::collections::VecDeque;

use bevy::prelude::{Bundle, Component, Query};
use boa_engine::{property::Attribute, Context, JsValue};

use crate::message::{Message, MessageComponent, Request};

use super::{Hostname, SystemNodeTrait};

#[derive(Component, Clone, Debug, Default)]
pub struct Server {
    pub request_handler: String,
    pub request_queue: VecDeque<Request>,
    pub state: ServerState,
}

impl SystemNodeTrait for Server {
    fn start_simulation(&mut self) {
        self.state = ServerState::Start;
    }

    fn handle_message(&mut self, message: MessageComponent) {
        println!("HANDLING MESSAGE FOR SERVER:");
        println!("{:?}", message);

        match message.message {
            Message::Request(request) => self.request_queue.push_back(request),
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

pub fn server_system(mut server_query: Query<&mut Server>) {
    for mut server in server_query.iter_mut() {
        match server.state {
            ServerState::Start => {
                server.state = ServerState::Idle;
            }
            ServerState::Idle => {
                if let Some(request) = server.request_queue.pop_front() {
                    let execution = ServerExecution {
                        request_handler: server.request_handler.clone(),
                        request,
                    };

                    execution.execute();

                    server.state = ServerState::Start;
                }
            }
            _ => {}
        };
    }
}

struct ServerExecution {
    request_handler: String,
    request: Request,
}

impl ServerExecution {
    fn execute(self) {
        let mut context = Context::default();

        let request = serde_json::to_value(self.request).unwrap();
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
function response(payload) {
  return { Response: payload };
}
            "#;

        context.eval(response_script).unwrap();

        let _request_handler = r#"
const requestHandler = function* () {
  const name = request.name;
  const surname = request.surname;
  const email = request.email;

  const fetchResult = yield http.get("/cat-facts");

  if (!fetchResult.ok) {
    return response({ status: 500, body: "Cat Facts API is down!" });
  }

  const dbResult = yield insertDb("db1", [name, surname, email]);

  if (dbResult.ok) {
    return response({ status: 200, body: fetchResult.body });
  } else {
    return response({ status: 500, body: "Database insert failed!" });
  }
}
          "#;

        context.eval(self.request_handler).unwrap();

        let generator_setup = r#"
const gen = requestHandler(request);
          "#;

        context.eval(generator_setup).unwrap();

        let value = context
            .eval(
                r#"
gen.next(lastGenResult);
"#,
            )
            .unwrap();

        let value = value.to_json(&mut context).unwrap();

        println!("{:?}", value);
    }
}
