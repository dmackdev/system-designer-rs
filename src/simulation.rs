use bevy::prelude::{Component, IntoSystemAppConfigs, OnEnter, Plugin, Query};

use crate::{
    game_state::GameState,
    node::{
        client::{client_system, Client},
        database::{database_system, Database},
        server::{server_system, Server},
        SystemNodeTrait,
    },
};

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_systems(
            (start::<Client>, start::<Server>, start::<Database>)
                .in_schedule(OnEnter(GameState::Simulate)),
        );

        app.add_systems((client_system, server_system, database_system));
    }
}

fn start<T: Component + SystemNodeTrait>(mut query: Query<&mut T>) {
    for mut node in query.iter_mut() {
        node.start_simulation();
    }
}
