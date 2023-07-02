use bevy::prelude::{Bundle, Component};

use crate::message::MessageComponent;

use super::{Hostname, SystemNodeTrait};

#[derive(Component, Clone, Debug, Default)]
pub struct Server {
    pub config: String,
}

impl SystemNodeTrait for Server {
    fn handle_message(&mut self, message: MessageComponent) {
        println!("HANDLING MESSAGE FOR SERVER:");
        println!("{:?}", message);
    }
}

#[derive(Bundle, Default)]
pub struct ServerBundle {
    server: Server,
    host_name: Hostname,
}
