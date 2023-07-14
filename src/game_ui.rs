use bevy::prelude::{App, EventWriter, Plugin, SystemSet};
use bevy_egui::{
    egui::{self, Context},
    EguiContexts,
};
use bevy_mod_picking::selection::PickSelection;
use strum::IntoEnumIterator;

use crate::{
    events::AddComponentEvent,
    game_state::{AppState, GameMode},
    grid::DeleteNodeEvent,
    level::{CurrentLevel, Level, LevelState},
    node::{
        client::{Client, HttpMethod, RequestConfig},
        database::Database,
        server::{Endpoint, Server},
        Hostname, NodeName, NodeType, SystemNodeTrait,
    },
    EditSet, MainMenuSet, SimulateSet,
};

use bevy::prelude::*;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
struct GameUiSystemSet;

pub struct GameUiPlugin;

impl Plugin for GameUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(main_menu_ui.in_set(MainMenuSet));
        app.add_system(level_select_ui.in_set(OnUpdate(AppState::LevelSelect)));

        let ui_systems = (
            tools_ui,
            node_inspector_ui::<Client>,
            node_inspector_ui::<Server>,
            node_inspector_ui::<Database>,
        );

        app.add_systems(ui_systems.in_set(EditSet));
        app.add_systems(ui_systems.in_set(SimulateSet));
    }
}

fn main_menu_ui(
    mut contexts: EguiContexts,
    mut app_state: ResMut<NextState<AppState>>,
    mut game_mode: ResMut<NextState<GameMode>>,
) {
    let ctx = contexts.ctx_mut();

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
            ui.heading("System Architect");

            if ui.button("Level Select").clicked() {
                app_state.set(AppState::LevelSelect);
            }

            if ui.button("Sandbox Mode").clicked() {
                app_state.set(AppState::Edit);
                game_mode.set(GameMode::Sandbox);
            }
        });
    });
}

fn level_select_ui(
    mut contexts: EguiContexts,
    mut app_state: ResMut<NextState<AppState>>,
    levels: Res<Assets<Level>>,
    mut level_state: ResMut<LevelState>,
) {
    let ctx = contexts.ctx_mut();

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
            ui.heading("Levels");

            for (idx, (_, level)) in levels.iter().enumerate() {
                let level_button_label = format!("Level {}: {}", idx + 1, level.name);

                if ui.button(level_button_label).clicked() {
                    level_state.current_level = Some(idx);

                    app_state.set(AppState::Edit);
                }
            }
        });
    });
}

fn tools_ui(
    mut contexts: EguiContexts,
    mut add_component_events: EventWriter<AddComponentEvent>,
    curr_app_state: Res<State<AppState>>,
    mut app_state: ResMut<NextState<AppState>>,
    current_level: CurrentLevel,
) {
    let ctx = contexts.ctx_mut();

    egui::SidePanel::left("tools")
        .default_width(200.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                if let Some((level_idx, level)) = current_level.get() {
                    ui.heading(format!("Level {}: {}", level_idx + 1, level.name));
                    ui.separator();
                    ui.label(&level.description);
                    ui.separator();
                }

                ui.add_enabled_ui(curr_app_state.0 == AppState::Edit, |ui| {
                    ui.heading("Components");

                    if ui.button("Add Client").clicked() {
                        add_component_events.send(AddComponentEvent::new_client());
                    }

                    if ui.button("Add Server").clicked() {
                        add_component_events.send(AddComponentEvent::new_server());
                    }

                    if ui.button("Add Database").clicked() {
                        add_component_events.send(AddComponentEvent::new_database());
                    }

                    ui.heading("Simulation");

                    if ui.button("Execute").clicked() {
                        app_state.set(AppState::Simulate);
                    }
                });
            });

            if curr_app_state.0 == AppState::Simulate && ui.button("Stop").clicked() {
                app_state.set(AppState::Edit);
            }

            if curr_app_state.0 == AppState::SimulateFinish && ui.button("Try Again").clicked() {
                app_state.set(AppState::Edit);
            }

            ui.allocate_space(ui.available_size());
        });
}

#[allow(clippy::complexity)]
fn node_inspector_ui<T: View + Component + SystemNodeTrait>(
    mut contexts: EguiContexts,
    mut nodes: Query<(
        &PickSelection,
        Entity,
        &mut NodeName,
        &mut NodeType,
        Option<&mut Hostname>,
        &mut T,
    )>,
    app_state: Res<State<AppState>>,
    delete_node_event: EventWriter<DeleteNodeEvent>,
) {
    if let Some((_, entity, mut node_name, mut node_type, hostname, mut node)) =
        nodes.iter_mut().find(|query| query.0.is_selected)
    {
        let ctx = contexts.ctx_mut();
        let can_be_edited = node.can_be_edited();

        show_inspector(
            ctx,
            &mut node_name,
            &mut node_type,
            hostname,
            node.as_mut(),
            can_be_edited && app_state.0 == AppState::Edit,
            delete_node_event,
            entity,
        );
    }
}

#[allow(clippy::complexity)]
fn show_inspector<T: View>(
    ctx: &mut Context,
    node_name: &mut NodeName,
    node_type: &mut NodeType,
    hostname: Option<Mut<'_, Hostname>>,
    node: &mut T,
    enabled: bool,
    mut delete_node_event: EventWriter<DeleteNodeEvent>,
    entity: Entity,
) {
    egui::SidePanel::right("inspector")
        .default_width(200.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Inspector");
                node_type.ui(ui, enabled);
                node_name.ui(ui, enabled);

                if let Some(mut hostname) = hostname {
                    hostname.ui(ui, enabled);
                }

                node.ui(ui, enabled);

                if enabled && ui.button("Delete node").clicked() {
                    delete_node_event.send(DeleteNodeEvent(entity));
                }

                ui.allocate_space(ui.available_size());
            });
        });
}

trait ToggleEditableUi {
    fn text_edit_label_toggle(&mut self, editable: bool, text: &mut String);
}

impl ToggleEditableUi for egui::Ui {
    fn text_edit_label_toggle(&mut self, editable: bool, text: &mut String) {
        if editable {
            self.text_edit_singleline(text);
        } else {
            self.label(&*text);
        }
    }
}

trait View {
    fn ui(&mut self, ui: &mut egui::Ui, editable: bool);
}

impl View for NodeName {
    fn ui(&mut self, ui: &mut egui::Ui, editable: bool) {
        ui.horizontal(|ui| {
            ui.label("Name:");

            ui.text_edit_label_toggle(editable, &mut self.0);
        });
    }
}

impl View for Hostname {
    fn ui(&mut self, ui: &mut egui::Ui, editable: bool) {
        ui.horizontal(|ui| {
            ui.label("Hostname:");
            ui.text_edit_label_toggle(editable, &mut self.0);
        });
    }
}

impl View for NodeType {
    fn ui(&mut self, ui: &mut egui::Ui, _: bool) {
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
    fn ui(&mut self, ui: &mut egui::Ui, editable: bool) {
        ui.separator();

        ui.heading("Requests");
        ui.separator();

        let mut request_idx_to_delete = None;

        for (idx, config) in self.request_configs.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.label("URL:");
                ui.text_edit_label_toggle(editable, &mut config.url);
            });

            ui.horizontal(|ui| {
                ui.label("Path:");
                ui.text_edit_label_toggle(editable, &mut config.path);
            });

            ui.horizontal(|ui| {
                ui.label("Method:");

                if editable {
                    egui::ComboBox::from_id_source(idx)
                        .selected_text(format_method(&config.method))
                        .show_ui(ui, |ui| {
                            for method in HttpMethod::iter() {
                                ui.selectable_value(
                                    &mut config.method,
                                    method,
                                    format_method(&method),
                                );
                            }
                        });
                } else {
                    ui.label(format_method(&config.method));
                }
            });

            if config.method == HttpMethod::Post || config.method == HttpMethod::Put {
                ui.label("Body:");
                ui.add(
                    egui::TextEdit::multiline(&mut config.body)
                        .interactive(editable)
                        .font(egui::TextStyle::Monospace)
                        .code_editor()
                        .desired_rows(1)
                        .lock_focus(true)
                        .desired_width(f32::INFINITY),
                );
            }

            if editable && ui.button("Delete Request").clicked() {
                request_idx_to_delete = Some(idx);
            }

            if let Some(response) = &config.response {
                ui.label("Response:");
                let mut pretty_string = serde_json::to_string_pretty(&response).unwrap();

                ui.add(
                    egui::TextEdit::multiline(&mut pretty_string)
                        .interactive(false)
                        .font(egui::TextStyle::Monospace)
                        .code_editor()
                        .desired_width(f32::INFINITY),
                );

                for (_passed, message) in config.expectations_results.iter() {
                    ui.label(message);
                }
            }

            ui.separator();
        }

        if let Some(i) = request_idx_to_delete {
            self.request_configs.remove(i);
        }

        if editable && ui.button("Add Request").clicked() {
            self.request_configs.push(RequestConfig::default());
        }
    }
}

impl View for Server {
    fn ui(&mut self, ui: &mut egui::Ui, editable: bool) {
        ui.separator();
        ui.heading("Endpoints");
        ui.separator();

        let mut endpoint_idx_to_delete = None;

        for (idx, endpoint) in self.endpoint_handlers.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.label("Path:");
                ui.text_edit_label_toggle(editable, &mut endpoint.path);
            });

            if editable {
                egui::ComboBox::from_id_source(idx)
                    .selected_text(format_method(&endpoint.method))
                    .show_ui(ui, |ui| {
                        for method in HttpMethod::iter() {
                            ui.selectable_value(
                                &mut endpoint.method,
                                method,
                                format_method(&method),
                            );
                        }
                    });
            } else {
                ui.label(format_method(&endpoint.method));
            }

            egui::CollapsingHeader::new("Request handler")
                .id_source(idx)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut endpoint.handler)
                            .interactive(editable)
                            .font(egui::TextStyle::Monospace) // for cursor height
                            .code_editor()
                            .desired_rows(1)
                            .lock_focus(true)
                            .desired_width(f32::INFINITY),
                    );
                });

            if editable && ui.button("Delete endpoint").clicked() {
                endpoint_idx_to_delete = Some(idx);
            }

            ui.separator();
        }

        if let Some(idx) = endpoint_idx_to_delete {
            self.endpoint_handlers.remove(idx);
        }

        if editable && ui.button("Add endpoint").clicked() {
            self.endpoint_handlers.push(Endpoint::default());
        }
    }
}

impl View for Database {
    fn ui(&mut self, ui: &mut egui::Ui, _editable: bool) {
        ui.separator();
        ui.heading("Documents");

        let mut document_entries: Vec<_> = self.documents.iter().collect();
        document_entries.sort_by_key(|e| e.0);

        let documents: Vec<_> = document_entries.iter().map(|e| e.1).collect();
        let mut pretty_string = serde_json::to_string_pretty(&documents).unwrap();
        ui.add(
            egui::TextEdit::multiline(&mut pretty_string)
                .interactive(false)
                .font(egui::TextStyle::Monospace)
                .code_editor()
                .desired_rows(1)
                .lock_focus(true)
                .desired_width(f32::INFINITY),
        );
    }
}
