use bevy::{prelude::Resource, reflect::TypeUuid};
use serde::Deserialize;

use crate::node::client::RequestConfig;

#[derive(Deserialize, Debug, TypeUuid)]
#[uuid = "F542117A-81DB-43E1-BB4C-4B4130B440C5"]
pub struct Level {
    pub clients: Vec<ClientConfig>,
}

#[derive(Deserialize, Debug)]
pub struct ClientConfig {
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub request_configs: Vec<RequestConfig>,
}

#[derive(Resource, Default)]
pub struct LevelState {
    pub current_level: Option<usize>,
}
