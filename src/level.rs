use bevy::reflect::TypeUuid;
use serde::Deserialize;

#[derive(Deserialize, Debug, TypeUuid)]
#[uuid = "F542117A-81DB-43E1-BB4C-4B4130B440C5"]
pub struct Level {}
