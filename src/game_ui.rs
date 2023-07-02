use bevy::prelude::{in_state, App, EventWriter, IntoSystemSetConfig, Plugin, SystemSet};
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
        server::Server,
        Hostname, NodeName, NodeType,
    },
};

use bevy::prelude::*;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
struct GameUiSystemSet;

pub struct GameUiPlugin;

impl Plugin for GameUiPlugin {
    fn build(&self, app: &mut App) {
        app.configure_set(GameUiSystemSet.run_if(in_state(GameState::Edit)));

        app.add_systems(
            (
                tools_ui,
                node_inspector_ui::<Client>,
                node_inspector_ui::<Server>,
            )
                .in_set(GameUiSystemSet),
        );
    }
}

fn tools_ui(
    mut contexts: EguiContexts,
    mut add_component_events: EventWriter<AddComponentEvent>,
    mut start_sim: EventWriter<StartSimulationEvent>,
) {
    let ctx = contexts.ctx_mut();

    egui::SidePanel::left("tools")
        .default_width(200.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Components");

                if ui.button("Add Client").clicked() {
                    add_component_events.send(AddComponentEvent(NodeType::Client));
                }

                if ui.button("Add Server").clicked() {
                    add_component_events.send(AddComponentEvent(NodeType::Server));
                }

                ui.heading("Simulation");

                if ui.button("Execute").clicked() {
                    start_sim.send(StartSimulationEvent);
                }

                ui.allocate_space(ui.available_size());
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
) {
    if let Some((_, mut node_name, mut node_type, hostname, mut node)) =
        nodes.iter_mut().find(|query| query.0.is_selected)
    {
        let ctx = contexts.ctx_mut();
        show_inspector(ctx, &mut node_name, &mut node_type, hostname, node.as_mut());
    }
}

fn show_inspector<T: View>(
    ctx: &mut Context,
    node_name: &mut NodeName,
    node_type: &mut NodeType,
    hostname: Option<Mut<'_, Hostname>>,
    node: &mut T,
) {
    egui::SidePanel::right("inspector")
        .default_width(200.0)
        .show(ctx, |ui| {
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

impl View for Client {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.separator();

        ui.heading("Requests");
        ui.separator();

        let format_method = |method: HttpMethod| method.to_string().to_ascii_uppercase();
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
                    .selected_text(format_method(config.method))
                    .show_ui(ui, |ui| {
                        for method in HttpMethod::iter() {
                            ui.selectable_value(&mut config.method, method, format_method(method));
                        }
                    });
            });

            match config.method {
                HttpMethod::Get => {
                    ui.label("Params:");

                    let mut params_idx_to_delete = None;

                    for (idx, (id, val)) in config.params.iter_mut().enumerate() {
                        ui.columns(2, |cols| {
                            cols[0].horizontal(|ui| {
                                if ui.button("x").clicked() {
                                    params_idx_to_delete = Some(idx);
                                }
                                ui.text_edit_singleline(id);
                            });
                            cols[1].text_edit_singleline(val);
                        });
                    }

                    if let Some(i) = params_idx_to_delete {
                        config.params.remove(i);
                    }

                    if ui.button("Add Param").clicked() {
                        config.params.push(("".to_string(), "".to_string()));
                    }
                }
                HttpMethod::Post => {
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
        ui.add(
            egui::TextEdit::multiline(&mut self.request_handler)
                .font(egui::TextStyle::Monospace) // for cursor height
                .code_editor()
                .desired_rows(10)
                .lock_focus(true)
                .desired_width(f32::INFINITY),
        );
    }
}
