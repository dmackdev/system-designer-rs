use bevy::{
    ecs::system::SystemParam,
    prelude::{Assets, Res, Resource},
    reflect::TypeUuid,
};
use serde::Deserialize;

use crate::{node::client::RequestConfig, Handles};

#[derive(Deserialize, Debug, TypeUuid)]
#[uuid = "F542117A-81DB-43E1-BB4C-4B4130B440C5"]
pub struct Level {
    pub name: String,
    pub description: String,
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

#[derive(SystemParam)]
pub struct CurrentLevel<'w> {
    levels: Res<'w, Assets<Level>>,
    handles: Res<'w, Handles>,
    level_state: Res<'w, LevelState>,
}

impl<'w> CurrentLevel<'w> {
    pub fn get(&self) -> Option<(usize, &Level)> {
        self.level_state
            .current_level
            .map(|idx| (idx, self.levels.get(&self.handles.levels[idx]).unwrap()))
    }
}
