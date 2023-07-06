use bevy::prelude::{App, EventWriter, Plugin, SystemSet};
use bevy_egui::{
    egui::{self, Context},
    EguiContexts,
};
use bevy_mod_picking::selection::PickSelection;
use strum::IntoEnumIterator;

use crate::{
    events::{AddComponentEvent, StartSimulationEvent},
    game_state::GameState,
    node::{
        client::{Client, HttpMethod, RequestConfig},
        database::Database,
        server::{Endpoint, Server},
        Hostname, NodeName, NodeType,
    },
};

use bevy::prelude::*;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
struct GameUiSystemSet;

pub struct GameUiPlugin;

impl Plugin for GameUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems((
            tools_ui,
            node_inspector_ui::<Client>,
            node_inspector_ui::<Server>,
            node_inspector_ui::<Database>,
        ));
    }
}

fn tools_ui(
    mut contexts: EguiContexts,
    mut add_component_events: EventWriter<AddComponentEvent>,
    mut start_sim: EventWriter<StartSimulationEvent>,
    game_state: Res<State<GameState>>,
) {
    let ctx = contexts.ctx_mut();

    egui::SidePanel::left("tools")
        .default_width(200.0)
        .show(ctx, |ui| {
            ui.add_enabled_ui(game_state.0 == GameState::Edit, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.heading("Components");

                    if ui.button("Add Client").clicked() {
                        add_component_events.send(AddComponentEvent(NodeType::Client));
                    }

                    if ui.button("Add Server").clicked() {
                        add_component_events.send(AddComponentEvent(NodeType::Server));
                    }

                    if ui.button("Add Database").clicked() {
                        add_component_events.send(AddComponentEvent(NodeType::Database));
                    }

                    ui.heading("Simulation");

                    if ui.button("Execute").clicked() {
                        start_sim.send(StartSimulationEvent);
                    }

                    ui.allocate_space(ui.available_size());
                });
            });
        });
}

fn node_inspector_ui<T: View + Component>(
    mut contexts: EguiContexts,
    mut nodes: Query<(
        &PickSelection,
        &mut NodeName,
        &mut NodeType,
        Option<&mut Hostname>,
        &mut T,
    )>,
    game_state: Res<State<GameState>>,
) {
    if let Some((_, mut node_name, mut node_type, hostname, mut node)) =
        nodes.iter_mut().find(|query| query.0.is_selected)
    {
        let ctx = contexts.ctx_mut();
        show_inspector(
            ctx,
            &mut node_name,
            &mut node_type,
            hostname,
            node.as_mut(),
            game_state.0 == GameState::Edit,
        );
    }
}

fn show_inspector<T: View>(
    ctx: &mut Context,
    node_name: &mut NodeName,
    node_type: &mut NodeType,
    hostname: Option<Mut<'_, Hostname>>,
    node: &mut T,
    enabled: bool,
) {
    egui::SidePanel::right("inspector")
        .default_width(200.0)
        .show(ctx, |ui| {
            ui.add_enabled_ui(enabled, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.heading("Inspector");
                    node_type.ui(ui);
                    node_name.ui(ui);

                    if let Some(mut hostname) = hostname {
                        hostname.ui(ui);
                    }

                    node.ui(ui);
                    ui.allocate_space(ui.available_size());
                });
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

impl View for Hostname {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Hostname:");
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
    }
}

fn format_method(method: &HttpMethod) -> String {
    method.to_string().to_ascii_uppercase()
}

impl View for Client {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.separator();

        ui.heading("Requests");
        ui.separator();

        let mut request_idx_to_delete = None;

        for (idx, config) in self.request_configs.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.label("URL:");
                ui.text_edit_singleline(&mut config.url);
            });

            ui.horizontal(|ui| {
                ui.label("Path:");
                ui.text_edit_singleline(&mut config.path);
            });

            ui.horizontal(|ui| {
                ui.label("Method:");

                egui::ComboBox::from_id_source(idx)
                    .selected_text(format_method(&config.method))
                    .show_ui(ui, |ui| {
                        for method in HttpMethod::iter() {
                            ui.selectable_value(&mut config.method, method, format_method(&method));
                        }
                    });
            });

            if config.method == HttpMethod::Post {
                ui.label("Body:");
                ui.add(
                    egui::TextEdit::multiline(&mut config.body)
                        .font(egui::TextStyle::Monospace) // for cursor height
                        .code_editor()
                        .desired_rows(10)
                        .lock_focus(true)
                        .desired_width(f32::INFINITY),
                );
            }

            if ui.button("Delete Request").clicked() {
                request_idx_to_delete = Some(idx);
            }

            ui.separator();
        }

        if let Some(i) = request_idx_to_delete {
            self.request_configs.remove(i);
        }

        if ui.button("Add Request").clicked() {
            self.request_configs.push_back(RequestConfig::default());
        }
    }
}

impl View for Server {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        ui.heading("Endpoints");
        ui.separator();

        let mut endpoint_idx_to_delete = None;

        for (idx, endpoint) in self.endpoint_handlers.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.label("Path:");
                ui.text_edit_singleline(&mut endpoint.path);
            });

            egui::ComboBox::from_id_source(idx)
                .selected_text(format_method(&endpoint.method))
                .show_ui(ui, |ui| {
                    for method in HttpMethod::iter() {
                        ui.selectable_value(&mut endpoint.method, method, format_method(&method));
                    }
                });

            egui::CollapsingHeader::new("Request handler")
                .id_source(idx)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut endpoint.handler)
                            .font(egui::TextStyle::Monospace) // for cursor height
                            .code_editor()
                            .desired_rows(10)
                            .lock_focus(true)
                            .desired_width(f32::INFINITY),
                    );
                });

            if ui.button("Delete endpoint").clicked() {
                endpoint_idx_to_delete = Some(idx);
            }

            ui.separator();
        }

        if let Some(idx) = endpoint_idx_to_delete {
            self.endpoint_handlers.remove(idx);
        }

        if ui.button("Add endpoint").clicked() {
            self.endpoint_handlers.push(Endpoint {
                path: "".to_string(),
                method: HttpMethod::Get,
                handler: "".to_string(),
            });
        }
    }
}

impl View for Database {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        ui.heading("Documents");
    }
}
