use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use strum::EnumIter;

use super::Hostname;

#[derive(Component, Clone, Debug)]
pub struct Database {
    pub schema: Vec<ColumnSchema>,
}

impl Default for Database {
    fn default() -> Self {
        Self {
            schema: vec![ColumnSchema {
                name: "id".to_string(),
                data_type: DataType::Int,
            }],
        }
    }
}

#[derive(Bundle, Default)]
pub struct DatabaseBundle {
    database: Database,
    hostname: Hostname,
}

#[derive(Clone, Debug, Default)]
pub struct ColumnSchema {
    pub name: String,
    pub data_type: DataType,
}

#[derive(
    Serialize,
    Deserialize,
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    EnumIter,
    strum::Display,
    Hash,
)]
pub enum DataType {
    #[default]
    Int,
    String,
}
