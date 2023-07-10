use bevy::prelude::{
    Component, IntoSystemAppConfigs, IntoSystemConfigs, OnEnter, OnUpdate, Plugin, Query,
};

use crate::{
    game_state::AppState,
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
                .in_schedule(OnEnter(AppState::Simulate)),
        );

        app.add_systems(
            (client_system, server_system, database_system).in_set(OnUpdate(AppState::Simulate)),
        );
    }
}

fn start<T: Component + SystemNodeTrait>(mut query: Query<&mut T>) {
    for mut node in query.iter_mut() {
        node.start_simulation();
    }
}
