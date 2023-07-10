use bevy::prelude::{on_event, IntoSystemConfig, NextState, Plugin, ResMut, States};

use crate::events::StartSimulationEvent;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum AppState {
    #[default]
    MainMenu,
    LevelSelect,
    Edit,
    Simulate,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum GameMode {
    #[default]
    LevelSelect,
    Sandbox,
}

pub struct GameStatePlugin;

impl Plugin for GameStatePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(handle_start_sim_event.run_if(on_event::<StartSimulationEvent>()));
    }
}

fn handle_start_sim_event(mut next_state: ResMut<NextState<AppState>>) {
    next_state.set(AppState::Simulate);
}
