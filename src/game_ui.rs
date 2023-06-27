use bevy::prelude::{
    in_state, App, EventWriter, IntoSystemConfig, IntoSystemSetConfig, Plugin, SystemSet,
};
use bevy_egui::{egui, EguiContexts};

use crate::{events::AddComponentEvent, game_state::GameState, node::NodeType};

use bevy::{input::common_conditions::input_toggle_active, prelude::*};

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
struct GameUiSystemSet;

pub struct GameUiPlugin;

impl Plugin for GameUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameUiState>();
        app.configure_set(GameUiSystemSet.run_if(in_state(GameState::Playing)));

        app.add_system(inspector_ui.run_if(input_toggle_active(true, KeyCode::Escape)));
    }
}

#[derive(Default, Resource)]
pub struct GameUiState {
    pub selected_node: Option<Entity>,
}

fn inspector_ui(
    mut contexts: EguiContexts,
    mut add_component_events: EventWriter<AddComponentEvent>,
    game_ui_state: Res<GameUiState>,
    nodes: Query<&NodeType>,
) {
    let ctx = contexts.ctx_mut();

    egui::SidePanel::left("tools")
        .default_width(200.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Create Components");

                if ui.button("Add Client").clicked() {
                    add_component_events.send(AddComponentEvent(crate::node::NodeType::Client));
                }

                if ui.button("Add Server").clicked() {
                    add_component_events.send(AddComponentEvent(crate::node::NodeType::Server));
                }

                ui.label("Press escape to toggle UI");
                ui.allocate_space(ui.available_size());
            });
        });

    egui::SidePanel::right("inspector")
        .default_width(200.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Inspector");

                if let Some(e) = game_ui_state.selected_node {
                    ui.label(format!("{:?}", nodes.get(e)));
                }

                ui.allocate_space(ui.available_size());
            });
        });
}
