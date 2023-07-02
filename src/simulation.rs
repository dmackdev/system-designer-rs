use bevy::prelude::{Component, IntoSystemAppConfigs, OnEnter, Plugin, Query};

use crate::{
    game_state::GameState,
    node::{client::Client, server::Server, SystemNodeTrait},
};

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_systems(
            (start::<Client>, start::<Server>).in_schedule(OnEnter(GameState::Simulate)),
        );
    }
}

fn start<T: Component + SystemNodeTrait>(mut query: Query<&mut T>) {
    for mut node in query.iter_mut() {
        node.start_simulation();
    }
}
