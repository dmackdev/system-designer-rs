use bevy::prelude::{
    Commands, Component, DespawnRecursiveExt, Entity, IntoSystemAppConfigs, IntoSystemConfig,
    IntoSystemConfigs, NextState, OnEnter, OnUpdate, Plugin, Query, ResMut, With,
};

use crate::{
    game_state::AppState,
    message::MessageComponent,
    node::{
        client::{client_system, Client, ClientState},
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

        app.add_system(verify_solution.in_set(OnUpdate(AppState::Simulate)));

        app.add_systems(
            (
                reset::<Client>,
                reset::<Server>,
                reset::<Database>,
                destroy_in_flight_messages,
            )
                .in_schedule(OnEnter(AppState::Edit)),
        );
    }
}

fn start<T: Component + SystemNodeTrait>(mut query: Query<&mut T>) {
    for mut node in query.iter_mut() {
        node.start_simulation();
    }
}

fn reset<T: Component + SystemNodeTrait>(mut query: Query<&mut T>) {
    for mut node in query.iter_mut() {
        node.reset();
    }
}

fn destroy_in_flight_messages(
    mut commands: Commands,
    message_query: Query<Entity, With<MessageComponent>>,
) {
    for message_entity in message_query.iter() {
        commands.entity(message_entity).despawn_recursive();
    }
}

fn verify_solution(
    mut clients: Query<&mut Client>,
    message_query: Query<Entity, With<MessageComponent>>,
    mut app_state: ResMut<NextState<AppState>>,
) {
    if !clients
        .iter()
        .all(|client| client.state == ClientState::Finished)
    {
        return;
    }

    if message_query.iter().count() > 0 {
        return;
    }

    for mut client in clients.iter_mut() {
        client.verify();
    }

    app_state.set(AppState::SimulateFinish);
}
