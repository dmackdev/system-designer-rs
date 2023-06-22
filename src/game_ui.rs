use bevy::prelude::{
    in_state, App, Commands, EventWriter, IntoSystemConfig, IntoSystemSetConfig, Plugin, SystemSet,
};
use bevy_egui::{egui, EguiContexts};

use crate::{events::AddComponentEvent, game_state::GameState};

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
struct GameUiSystemSet;

pub struct GameUiPlugin;

impl Plugin for GameUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup);

        app.configure_set(GameUiSystemSet.run_if(in_state(GameState::Playing)));
        app.add_system(ui_example_system.in_set(GameUiSystemSet));
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
