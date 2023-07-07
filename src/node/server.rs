use std::collections::{HashMap, VecDeque};

use bevy::prelude::{warn, Bundle, Component, Entity, EventWriter, Query};
use boa_engine::{property::Attribute, Context, JsValue};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::message::{
    DatabaseCall, Message, MessageComponent, Request, Response, SendMessageEvent,
};

use super::{client::HttpMethod, Hostname, NodeConnections, SystemNodeTrait};

#[derive(Component, Clone, Debug)]
pub struct Server {
    pub endpoint_handlers: Vec<Endpoint>,
    pub message_queue: VecDeque<MessageComponent>,
    pub state: ServerState,
    active_executions: HashMap<Uuid, ServerExecution>,
}

#[derive(Clone, Debug)]
pub struct Endpoint {
    pub path: String,
    pub method: HttpMethod,
    pub handler: String,
}

const EXAMPLE_REQUEST_HANDLER: &str = r#"const requestHandler = function* () {
  const result = yield db.save("db1", request.body);
  return response(200, result);
}
"#;

impl Default for Server {
    fn default() -> Self {
        Self {
            message_queue: Default::default(),
            state: Default::default(),
            active_executions: Default::default(),
            endpoint_handlers: vec![Endpoint {
                path: "/".to_string(),
                method: HttpMethod::Get,
                handler: EXAMPLE_REQUEST_HANDLER.to_string(),
            }],
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
                if server.message_queue.is_empty() {
                    continue;
                }

                let message_queue = server.message_queue.drain(..).collect::<Vec<_>>();

                let endpoints_by_method: HashMap<HttpMethod, HashMap<String, String>> =
                    server.endpoint_handlers.clone().into_iter().fold(
                        HashMap::new(),
                        |mut acc,
                         Endpoint {
                             path,
                             method,
                             handler,
                         }| {
                            acc.entry(method).or_default().insert(path, handler);
                            acc
                        },
                    );

                for message in message_queue {
                    let execution = match message.message {
                        Message::Request(mut request) => endpoints_by_method
                            .get(&request.method)
                            .and_then(|endpoints_by_path| {
                                map_url_to_path_with_params(
                                    &request.path,
                                    endpoints_by_path.keys().collect(),
                                )
                            })
                            .map(|EndpointMatch { path, params }| {
                                request.params = params;
                                ServerExecution::new(
                                    endpoints_by_method
                                        .get(&request.method)
                                        .unwrap()
                                        .get(path)
                                        .unwrap()
                                        .to_string(),
                                    request,
                                    message.sender,
                                    message.trace_id,
                                )
                            }),
                        Message::Response(response) => {
                            let mut execution =
                                server.active_executions.remove(&message.trace_id).unwrap();

                            execution
                                .yield_values
                                .push(serde_json::to_value(response).unwrap());

                            Some(execution)
                        }
                        Message::DatabaseAnswer(answer) => {
                            let mut execution =
                                server.active_executions.remove(&message.trace_id).unwrap();

                            execution
                                .yield_values
                                .push(serde_json::to_value(answer).unwrap());

                            Some(execution)
                        }
                        _ => None,
                    };

                    if let Some(mut execution) = execution {
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
                            (false, YieldValue::DatabaseCall(database_call)) => {
                                let new_trace_id = Uuid::new_v4();

                                let recipient = hostnames
                                    .iter()
                                    .find(|(_, node_name)| node_name.0 == database_call.name);

                                if let Some((recipient, _)) = recipient {
                                    if connections.is_connected_to(recipient) {
                                        events.send(SendMessageEvent {
                                            sender: server_entity,
                                            recipients: vec![recipient],
                                            message: Message::DatabaseCall(database_call),
                                            trace_id: new_trace_id,
                                        });

                                        server.active_executions.insert(new_trace_id, execution);
                                    }
                                }
                            }
                            _ => warn!("Unexpected yield value"),
                        };
                    } else {
                        events.send(SendMessageEvent {
                            sender: server_entity,
                            recipients: vec![message.sender],
                            message: Message::Response(Response::not_found()),
                            trace_id: message.trace_id,
                        });
                    }
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
    yield_values: Vec<Value>,
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

        let db_script = r#"
const db = {
  save: function(name, value) { return { DatabaseCall: { name, call_type: { Save: value } } } },
  findOne: function(name, id) { return { DatabaseCall: { name, call_type: { FindOne: id } } } },
  findAll: function(name) { return { DatabaseCall: { name, call_type: "FindAll" } } },
  contains: function(name, id) { return { DatabaseCall: { name, call_type: { Contains: id } } } }
};
          "#;

        context.eval(db_script).unwrap();

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
            let prev_js_yield_value = JsValue::from_json(prev_yield_value, &mut context).unwrap();

            context.register_global_property(
                "lastGenResult",
                prev_js_yield_value,
                Attribute::all(),
            );

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

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct GeneratorResultValue {
    pub done: bool,
    pub value: YieldValue,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum YieldValue {
    Response(Response),
    Request(Request),
    DatabaseCall(DatabaseCall),
    DatabaseAnswer(Value),
}

#[derive(PartialEq, Eq, Debug)]
struct EndpointMatch<'a> {
    path: &'a String,
    params: HashMap<String, String>,
}

fn map_url_to_path_with_params<'a>(
    url: &str,
    endpoints_paths: Vec<&'a String>,
) -> Option<EndpointMatch<'a>> {
    let url_segments: Vec<_> = url.split('/').collect();
    for endpoint_path in endpoints_paths.iter() {
        let endpoint_segments: Vec<_> = endpoint_path.split('/').collect();

        if url_segments.len() != endpoint_segments.len() {
            continue;
        }

        let mut params: HashMap<String, String> = HashMap::new();

        for (idx, (url_segment, endpoint_segment)) in url_segments
            .iter()
            .zip(endpoint_segments.iter())
            .enumerate()
        {
            if endpoint_segment.starts_with(':') {
                params.insert(
                    endpoint_segment.strip_prefix(':').unwrap().to_string(),
                    url_segment.to_string(),
                );
            } else if url_segment != endpoint_segment {
                params.clear();
                break;
            }

            if idx == url_segments.len() - 1 {
                return Some(EndpointMatch {
                    path: endpoint_path,
                    params,
                });
            }
        }
    }
    None
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn should_return_matching_path_and_empty_params_for_root() {
        let url: &str = "/";
        let endpoints_paths = ["/".to_string()];

        let endpoints_paths: Vec<&String> = endpoints_paths.iter().collect();

        assert_eq!(
            Some(EndpointMatch {
                path: &"/".to_string(),
                params: HashMap::new()
            }),
            map_url_to_path_with_params(url, endpoints_paths)
        )
    }

    #[test]
    fn should_return_matching_path_with_single_param_after_root() {
        let url: &str = "/1";
        let endpoints_paths = ["/:id".to_string()];

        let endpoints_paths: Vec<&String> = endpoints_paths.iter().collect();

        assert_eq!(
            Some(EndpointMatch {
                path: &"/:id".to_string(),
                params: HashMap::from_iter([("id".to_string(), "1".to_string())])
            }),
            map_url_to_path_with_params(url, endpoints_paths)
        )
    }

    #[test]
    fn should_return_matching_path_with_single_param() {
        let url: &str = "/orders/1";
        let endpoints_paths = [
            "/orders".to_string(),
            "/orders/:id".to_string(),
            "/orders/:id/items".to_string(),
            "/users".to_string(),
            "/users/:id".to_string(),
            "/users/:id/messages".to_string(),
        ];

        let endpoints_paths: Vec<&String> = endpoints_paths.iter().collect();

        assert_eq!(
            Some(EndpointMatch {
                path: &"/orders/:id".to_string(),
                params: HashMap::from_iter([("id".to_string(), "1".to_string())])
            }),
            map_url_to_path_with_params(url, endpoints_paths)
        )
    }

    #[test]
    fn should_return_matching_path_with_multiple_params() {
        let url: &str = "/orders/123/items/456";
        let endpoints_paths = [
            "/orders".to_string(),
            "/orders/:orderId".to_string(),
            "/orders/:orderId/items".to_string(),
            "/orders/:orderId/items/:itemId".to_string(),
            "/orders/:orderId/items/:itemId/id".to_string(),
        ];

        let endpoints_paths: Vec<&String> = endpoints_paths.iter().collect();

        assert_eq!(
            Some(EndpointMatch {
                path: &"/orders/:orderId/items/:itemId".to_string(),
                params: HashMap::from_iter([
                    ("orderId".to_string(), "123".to_string()),
                    ("itemId".to_string(), "456".to_string())
                ])
            }),
            map_url_to_path_with_params(url, endpoints_paths)
        )
    }

    #[test]
    fn should_return_none_for_no_matching_path() {
        let url: &str = "/users/123";
        let endpoints_paths = [
            "/users".to_string(),
            "/users/:userId/messages".to_string(),
            "/orders/:orderId/items".to_string(),
            "/orders/:orderId/items/:itemId".to_string(),
        ];

        let endpoints_paths: Vec<&String> = endpoints_paths.iter().collect();

        assert_eq!(None, map_url_to_path_with_params(url, endpoints_paths))
    }
}
