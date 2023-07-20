use bevy::{
    prelude::{
        Commands, Component, DespawnRecursiveExt, Entity, IntoSystemAppConfigs, IntoSystemConfig,
        IntoSystemConfigs, NextState, OnEnter, OnUpdate, Plugin, Query, ResMut, With,
    },
    utils::HashSet,
};

use crate::{
    game_state::AppState,
    level::LevelState,
    message::MessageComponent,
    node::{
        client::{client_system, Client, ClientState},
        database::{database_system, Database},
        server::{server_system, Server},
        Hostname, SystemNodeTrait,
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

        app.add_system(validate.in_set(OnUpdate(AppState::Validate)));
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

fn validate(
    mut app_state: ResMut<NextState<AppState>>,
    hostnames: Query<&Hostname>,
    clients: Query<&Client>,
    servers: Query<&Server>,
) {
    if HashSet::from_iter(hostnames.iter().map(|h| h.0.clone())).len() != hostnames.iter().len()
        || !hostnames.iter().all(|h| h.is_valid())
        || !clients.iter().all(|c| c.is_valid())
        || !servers.iter().all(|s| s.is_valid())
    {
        app_state.set(AppState::Edit);
        return;
    }

    app_state.set(AppState::Simulate)
}

fn verify_solution(
    mut clients: Query<&mut Client>,
    message_query: Query<Entity, With<MessageComponent>>,
    mut app_state: ResMut<NextState<AppState>>,
    mut level_state: ResMut<LevelState>,
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

    let mut passed = true;
    for mut client in clients.iter_mut() {
        let client_passed = client.verify();

        if !client_passed {
            passed = false
        }
    }

    level_state.level_passed = passed;
    app_state.set(AppState::SimulateFinish);
}
