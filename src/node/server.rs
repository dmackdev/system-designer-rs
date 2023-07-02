use bevy::prelude::Component;

use crate::message::MessageComponent;

use super::SystemNodeTrait;

#[derive(Component, Clone, Debug, Default)]
pub struct Server {
    pub config: String,
}

impl Server {
    pub fn new() -> Self {
        Default::default()
    }
}

impl SystemNodeTrait for Server {
    fn handle_message(&mut self, message: MessageComponent) {
        println!("HANDLING MESSAGE FOR SERVER:");
        println!("{:?}", message);
    }
}
