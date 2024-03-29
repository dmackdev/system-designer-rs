use std::collections::{HashMap, HashSet, VecDeque};

use bevy::prelude::{warn, Component, Entity, EventWriter, Query};
use boa_engine::{property::Attribute, Context, JsResult, JsValue};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::message::{
    DatabaseCall, Message, MessageComponent, Request, Response, SendMessageEvent,
};

use super::{client::HttpMethod, HostnameConnections, SystemNodeTrait};

#[derive(Component, Clone, Debug)]
pub struct Server {
    pub endpoint_handlers: Vec<Endpoint>,
    pub message_queue: VecDeque<MessageComponent>,
    pub state: ServerState,
    active_executions: HashMap<Uuid, ServerExecution>,
    can_be_edited: bool,
    max_concurrent_connections: Option<u8>,
}

#[derive(Clone, Debug)]
pub struct Endpoint {
    pub path: String,
    pub method: HttpMethod,
    pub handler: String,
}

impl Endpoint {
    fn is_path_valid(&self) -> bool {
        self.path.starts_with('/') && (self.path.len() == 1 || !self.path.ends_with('/'))
    }

    pub fn is_handler_valid(&self) -> bool {
        if self.handler.is_empty() {
            return false;
        }

        let mut context = Context::default();

        context.parse(&self.handler).is_ok()
    }

    fn is_valid(&self) -> bool {
        self.is_path_valid() && self.is_handler_valid()
    }
}

const EXAMPLE_REQUEST_HANDLER: &str = r#"const requestHandler = function* () {
  return response(200, "Ok");
}"#;

impl Default for Endpoint {
    fn default() -> Self {
        Self {
            path: "/".to_string(),
            method: HttpMethod::Post,
            handler: EXAMPLE_REQUEST_HANDLER.to_string(),
        }
    }
}

impl Default for Server {
    fn default() -> Self {
        Self {
            message_queue: Default::default(),
            state: Default::default(),
            active_executions: Default::default(),
            endpoint_handlers: vec![Endpoint::default()],
            can_be_edited: true,
            max_concurrent_connections: None,
        }
    }
}

impl Server {
    fn can_handle_new_request(&self) -> bool {
        if let Some(max_concurrent_connections) = self.max_concurrent_connections {
            let num_current_connections = self.active_executions.len();
            return num_current_connections < max_concurrent_connections.into();
        };
        true
    }

    pub fn with_max_concurrent_connections(
        mut self,
        max_concurrent_connections: Option<u8>,
    ) -> Self {
        self.max_concurrent_connections = max_concurrent_connections;
        self
    }

    pub fn max_concurrent_connections(&self) -> Option<u8> {
        self.max_concurrent_connections
    }

    fn create_execution_for_request(
        &mut self,
        mut request: Request,
        original_sender: Entity,
        original_trace_id: Uuid,
    ) -> Option<ServerExecution> {
        let endpoints_by_method: HashMap<HttpMethod, HashMap<String, String>> =
            self.endpoint_handlers.clone().into_iter().fold(
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

        endpoints_by_method
            .get(&request.method)
            .and_then(|endpoints_by_path| {
                map_url_to_path_with_params(&request.path, endpoints_by_path.keys().collect())
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
                    original_sender,
                    original_trace_id,
                )
            })
    }

    fn reset(&mut self) {
        self.state = ServerState::SimulationNotStarted;
        self.message_queue.drain(..);
        self.active_executions.clear();
    }

    pub fn is_path_valid(&self, idx: usize) -> bool {
        if !self.endpoint_handlers[idx].is_path_valid() {
            return false;
        }

        let mut endpoints = self.endpoint_handlers.clone();
        let endpoint_to_check = endpoints.swap_remove(idx);

        let other_endpoints: HashSet<(String, HttpMethod)> =
            HashSet::from_iter(endpoints.iter().map(|e| (strip_params(&e.path), e.method)));

        let endpoint = (
            strip_params(&endpoint_to_check.path),
            endpoint_to_check.method,
        );

        !other_endpoints.contains(&endpoint)
    }

    pub fn is_valid(&self) -> bool {
        let unique_endpoints: HashSet<(String, HttpMethod)> = HashSet::from_iter(
            self.endpoint_handlers
                .iter()
                .map(|e| (strip_params(&e.path), e.method)),
        );

        if unique_endpoints.len() != self.endpoint_handlers.len() {
            return false;
        }

        self.endpoint_handlers.iter().all(|e| e.is_valid())
    }
}

fn strip_params(path: &str) -> String {
    let segments: Vec<_> = path
        .split('/')
        .map(|seg| if seg.starts_with(':') { "param" } else { seg })
        .collect();

    segments.join("/")
}

impl SystemNodeTrait for Server {
    fn start_simulation(&mut self) {
        self.state = ServerState::Active;
    }

    fn handle_message(&mut self, message: MessageComponent) {
        if self.state == ServerState::Active {
            println!("HANDLING MESSAGE FOR SERVER:");
            println!("{:?}", message);

            self.message_queue.push_back(message);
        }
    }

    fn can_be_edited(&self) -> bool {
        self.can_be_edited
    }

    fn reset(&mut self) {
        self.reset();
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ServerState {
    #[default]
    SimulationNotStarted,
    Active,
}

#[allow(clippy::single_match)]
pub fn server_system(
    mut server_query: Query<(Entity, &mut Server)>,
    mut events: EventWriter<SendMessageEvent>,
    hostname_connections: HostnameConnections,
) {
    for (server_entity, mut server) in server_query.iter_mut() {
        match server.state {
            ServerState::Active => {
                if server.message_queue.is_empty() {
                    continue;
                }

                let message_queue = server.message_queue.drain(..).collect::<Vec<_>>();

                for message in message_queue {
                    let handle_message_result = match message.message {
                        Message::Request(request) => {
                            if server.can_handle_new_request() {
                                server
                                    .create_execution_for_request(
                                        request,
                                        message.sender,
                                        message.trace_id,
                                    )
                                    .ok_or(ExecutionError::NotFound)
                            } else {
                                Err(ExecutionError::ServiceUnavailable)
                            }
                        }
                        Message::Response(response) => {
                            let mut execution =
                                server.active_executions.remove(&message.trace_id).unwrap();

                            execution
                                .yield_values
                                .push(serde_json::to_value(response).unwrap());

                            Ok(execution)
                        }
                        Message::DatabaseAnswer(answer) => {
                            let mut execution =
                                server.active_executions.remove(&message.trace_id).unwrap();

                            execution
                                .yield_values
                                .push(serde_json::to_value(answer).unwrap());

                            Ok(execution)
                        }
                        _ => Err(ExecutionError::BadRequest),
                    };

                    match handle_message_result {
                        Ok(execution) => {
                            let res = execution.execute();

                            println!("{:?}", res);

                            match res {
                                Ok(res) => {
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

                                            let recipient = hostname_connections
                                                .get_connected_entity_by_hostname(
                                                    server_entity,
                                                    &new_request.url,
                                                );

                                            if let Some(recipient) = recipient {
                                                events.send(SendMessageEvent {
                                                    sender: server_entity,
                                                    recipients: vec![recipient],
                                                    message: Message::Request(new_request),
                                                    trace_id: new_trace_id,
                                                });

                                                server
                                                    .active_executions
                                                    .insert(new_trace_id, execution);
                                            } else {
                                                events.send(SendMessageEvent {
                                                    sender: server_entity,
                                                    recipients: vec![execution.original_sender],
                                                    message: Message::Response(
                                                        Response::internal_server_error(
                                                            "Upstream request refused.".into(),
                                                        ),
                                                    ),
                                                    trace_id: execution.original_trace_id,
                                                });
                                            }
                                        }
                                        (false, YieldValue::DatabaseCall(database_call)) => {
                                            let recipient = hostname_connections
                                                .get_connected_entity_by_hostname(
                                                    server_entity,
                                                    &database_call.name,
                                                );

                                            match recipient {
                                                Some(recipient) => {
                                                    let new_trace_id = Uuid::new_v4();

                                                    events.send(SendMessageEvent {
                                                        sender: server_entity,
                                                        recipients: vec![recipient],
                                                        message: Message::DatabaseCall(
                                                            database_call,
                                                        ),
                                                        trace_id: new_trace_id,
                                                    });

                                                    server
                                                        .active_executions
                                                        .insert(new_trace_id, execution);
                                                }
                                                None => {
                                                    events.send(SendMessageEvent {
                                                        sender: server_entity,
                                                        recipients: vec![execution.original_sender],
                                                        message: Message::Response(
                                                            Response::internal_server_error(
                                                                "Upstream request refused.".into(),
                                                            ),
                                                        ),
                                                        trace_id: execution.original_trace_id,
                                                    });
                                                }
                                            };
                                        }
                                        _ => warn!("Unexpected yield value"),
                                    };
                                }
                                Err(execution_error) => {
                                    events.send(SendMessageEvent {
                                        sender: server_entity,
                                        recipients: vec![message.sender],
                                        message: Message::Response(execution_error.into()),
                                        trace_id: message.trace_id,
                                    });
                                }
                            }
                        }
                        Err(execution_error) => {
                            events.send(SendMessageEvent {
                                sender: server_entity,
                                recipients: vec![message.sender],
                                message: Message::Response(execution_error.into()),
                                trace_id: message.trace_id,
                            });
                        }
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
    yield_values: Vec<Value>,
    original_sender: Entity,
    original_trace_id: Uuid,
}

#[derive(Deserialize, Debug)]
enum ExecutionError {
    NotFound,
    BadRequest,
    InternalServerError(Value),
    ServiceUnavailable,
}

impl From<JsResult<Value>> for ExecutionError {
    fn from(value: JsResult<Value>) -> Self {
        match value {
            Ok(value) => Self::InternalServerError(value),
            Err(_) => Self::InternalServerError(Value::Null),
        }
    }
}

impl From<ExecutionError> for Response {
    fn from(value: ExecutionError) -> Self {
        match value {
            ExecutionError::NotFound => Response::not_found(),
            ExecutionError::BadRequest => Response::bad_request(),
            ExecutionError::InternalServerError(value) => Response::internal_server_error(value),
            ExecutionError::ServiceUnavailable => Response::service_unavailable(),
        }
    }
}

impl From<serde_json::Error> for ExecutionError {
    fn from(value: serde_json::Error) -> Self {
        println!("{:?}", value);
        println!("{:?}", value.to_string());
        // TODO: better message for client
        Self::InternalServerError(Value::from("Incorrect yield value or response."))
    }
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
    fn execute(&self) -> Result<GeneratorResultValue, ExecutionError> {
        let mut context = Context::default();

        let request = serde_json::to_value(&self.request).unwrap();
        let request = JsValue::from_json(&request, &mut context).unwrap();

        context.register_global_property("request", request, Attribute::all());

        let http_script = r#"
const http = {
  get: function(url, path) { return { Request: { url, path, method: "Get", body: null, params: {} }}; },
}
        "#;

        context.eval(http_script).unwrap();

        let db_script = r#"
const db = {
  save: function(name, value) { return { DatabaseCall: { name, call_type: { Save: value } } } },
  findOne: function(name, id) { return { DatabaseCall: { name, call_type: { FindOne: id } } } },
  findAll: function(name) { return { DatabaseCall: { name, call_type: "FindAll" } } },
  contains: function(name, id) { return { DatabaseCall: { name, call_type: { Contains: id } } } },
  delete: function(name, id) { return { DatabaseCall: { name, call_type: { Delete: id } } } }
};
          "#;

        context.eval(db_script).unwrap();

        let response_script = r#"
function response(status, data) {
  return { Response: { status, data } };
}
          "#;

        context.eval(response_script).unwrap();

        context
            .eval(&self.request_handler)
            .map_err(|error| error.to_json(&mut context))?;

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
            .map_err(|error| error.to_json(&mut context))?;

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
                .map_err(|error| error.to_json(&mut context))?;
        }

        let latest_value = value
            .to_json(&mut context)
            .map_err(|error| error.to_json(&mut context))?;

        println!("LATEST YIELD JS VALUE:");
        println!("{:?}", latest_value);

        // TODO: determine if the generator is done
        // If it is, then create an ExecutionError indicating a wrong return type from the endpoint
        // Otherwise malformed yield value

        match serde_json::from_value::<GeneratorResultValue>(latest_value) {
            Ok(v) => Ok(v),
            Err(e) => Err(e.into()),
        }
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
