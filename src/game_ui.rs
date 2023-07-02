use bevy::prelude::{
    in_state, App, EventWriter, IntoSystemConfig, IntoSystemSetConfig, Plugin, SystemSet,
};
use bevy_egui::{egui, EguiContexts};

use crate::{
    events::{AddComponentEvent, StartSimulationEvent},
    game_state::GameState,
    node::{client::Client, server::Server, NodeName, NodeType},
};

use bevy::{input::common_conditions::input_toggle_active, prelude::*};

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
struct GameUiSystemSet;

pub struct GameUiPlugin;

impl Plugin for GameUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameUiState>();
        app.configure_set(GameUiSystemSet.run_if(in_state(GameState::Edit)));

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
    mut nodes: Query<(&mut NodeType, &mut NodeName)>,
    mut start_sim: EventWriter<StartSimulationEvent>,
) {
    let ctx = contexts.ctx_mut();

    egui::SidePanel::left("tools")
        .default_width(200.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Components");

                if ui.button("Add Client").clicked() {
                    add_component_events.send(AddComponentEvent(NodeType::Client(Client::new())));
                }

                if ui.button("Add Server").clicked() {
                    add_component_events.send(AddComponentEvent(NodeType::Server(Server::new())));
                }

                ui.heading("Simulation");

                if ui.button("Execute").clicked() {
                    start_sim.send(StartSimulationEvent);
                }

                ui.allocate_space(ui.available_size());
            });
        });

    egui::SidePanel::right("inspector")
        .default_width(200.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Inspector");

                if let Some(e) = game_ui_state.selected_node {
                    let (mut node_type, mut node_name) = nodes.get_mut(e).unwrap();

                    node_name.ui(ui);

                    ui.separator();

                    node_type.ui(ui);
                }

                ui.allocate_space(ui.available_size());
            });
        });
}

trait View {
    fn ui(&mut self, ui: &mut egui::Ui);
}

impl View for NodeName {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Name:");
            ui.text_edit_singleline(&mut self.0);
        });
    }
}

impl View for NodeType {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Type:");
            ui.label(format!("{}", self));
        });

        match self {
            NodeType::Client(client) => client.ui(ui),
            NodeType::Server(server) => server.ui(ui),
        }
    }
}

impl View for Client {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.add(
            egui::TextEdit::multiline(&mut self.config)
                .font(egui::TextStyle::Monospace) // for cursor height
                .code_editor()
                .desired_rows(10)
                .lock_focus(true)
                .desired_width(f32::INFINITY),
        );
    }
}

impl View for Server {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.add(
            egui::TextEdit::multiline(&mut self.config)
                .font(egui::TextStyle::Monospace) // for cursor height
                .code_editor()
                .desired_rows(10)
                .lock_focus(true)
                .desired_width(f32::INFINITY),
        );
    }
}
