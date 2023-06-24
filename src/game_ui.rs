use bevy::prelude::{
    in_state, App, EventWriter, IntoSystemConfig, IntoSystemSetConfig, Plugin, SystemSet,
};
use bevy_egui::{egui, EguiContexts};

use crate::{events::AddComponentEvent, game_state::GameState};

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
struct GameUiSystemSet;

pub struct GameUiPlugin;

impl Plugin for GameUiPlugin {
    fn build(&self, app: &mut App) {
        app.configure_set(GameUiSystemSet.run_if(in_state(GameState::Playing)));
        app.add_system(render_game_ui.in_set(GameUiSystemSet));
    }
}

fn render_game_ui(
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
