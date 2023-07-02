use std::collections::VecDeque;

use bevy::prelude::{Bundle, Component, Query};

use crate::message::{Message, MessageComponent, Request};

use super::{Hostname, SystemNodeTrait};

#[derive(Component, Clone, Debug, Default)]
pub struct Server {
    pub config: String,
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
            Message::Request(request) => self.request_queue.push_front(request),
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

pub fn server_system(server_query: Query<&mut Server>) {
    for server in server_query.iter() {
        match server.state {
            ServerState::Start => {
                // TODO: initialise a new execution context
            }
            ServerState::Idle => {
                // TODO: Handle next request in request queue
            }
            _ => {}
        };
    }
}
