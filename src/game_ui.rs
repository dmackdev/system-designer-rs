use bevy::prelude::{in_state, App, Commands, EventWriter, IntoSystemConfig, Plugin};
use bevy_egui::{egui, EguiContexts};

use crate::{events::AddComponentEvent, game_state::GameState};

pub struct GameUiPlugin;

impl Plugin for GameUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup)
            .add_system(ui_example_system.run_if(in_state(GameState::Playing)));
    }
}

fn setup(mut commands: Commands) {}

fn ui_example_system(
    mut contexts: EguiContexts,
    mut add_component_events: EventWriter<AddComponentEvent>,
) {
    let ctx = contexts.ctx_mut();

    egui::SidePanel::left("side_panel")
        .resizable(false)
        .show(ctx, |ui| {
            if ui.button("Add Server").clicked() {
                add_component_events.send(AddComponentEvent::Server);
            }
        });
}
