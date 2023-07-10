use bevy::{prelude::Resource, reflect::TypeUuid};
use serde::Deserialize;

use crate::node::client::RequestConfig;

#[derive(Deserialize, Debug, TypeUuid)]
#[uuid = "F542117A-81DB-43E1-BB4C-4B4130B440C5"]
pub struct Level {
    pub clients: Vec<(f32, f32, Vec<RequestConfig>)>,
}

#[derive(Resource, Default)]
pub struct LevelState {
    pub current_level: Option<usize>,
}
