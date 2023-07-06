use bevy::prelude::*;
use serde_json::Value;

use super::Hostname;

#[derive(Component, Clone, Debug, Default)]
pub struct Database {
    pub documents: Vec<Value>,
}

#[derive(Bundle, Default)]
pub struct DatabaseBundle {
    database: Database,
    hostname: Hostname,
}
