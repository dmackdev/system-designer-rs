use bevy::prelude::*;
use bevy_common_assets::ron::RonAssetPlugin;
use bevy_egui::EguiPlugin;
use bevy_mod_picking::{prelude::RaycastPickCamera, DefaultPickingPlugins};
use bevy_prototype_lyon::prelude::*;
use events::AddComponentEvent;
use game_state::{AppState, GameMode};
use game_ui::GameUiPlugin;
use grid::GridPlugin;
use level::{Level, LevelState};
use loading::LoadingPlugin;
use message::{MessageArrivedEvent, MessagePlugin, SendMessageEvent};

use simulation::SimulationPlugin;

mod color;
mod events;
mod game_state;
mod game_ui;
mod grid;
mod layer;
mod level;
mod loading;
mod message;
mod node;
mod simulation;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub struct MainMenuSet;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub struct EditSet;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub struct GridSet;

fn main() {
    let mut app = App::new();

    app.insert_resource(ClearColor(color::BACKGROUND))
        .insert_resource(Msaa::Sample4);

    app.init_resource::<LevelState>();

    let default = DefaultPlugins
        .set(WindowPlugin {
            primary_window: Some(Window {
                title: String::from("System Architect"),
                canvas: Some("#bevy-canvas".to_string()),
                ..Default::default()
            }),
            ..default()
        })
        .build();

    app.add_event::<AddComponentEvent>();
    app.add_event::<SendMessageEvent>();
    app.add_event::<MessageArrivedEvent>();

    app.add_state::<AppState>();
    app.add_state::<GameMode>();

    app.configure_set(MainMenuSet.run_if(in_state(AppState::MainMenu)));
    app.configure_set(EditSet.run_if(in_state(AppState::Edit)));

    app.configure_set(
        GridSet.run_if(
            in_state(AppState::Simulate)
                .or_else(in_state(AppState::SimulateFinish).or_else(in_state(AppState::Edit))),
        ),
    );

    app.add_plugins(default)
        .add_plugin(RonAssetPlugin::<Level>::new(&["level.ron"]))
        .add_plugin(ShapePlugin)
        .add_plugin(EguiPlugin)
        .add_plugin(LoadingPlugin)
        .add_plugin(GameUiPlugin)
        .add_plugin(GridPlugin)
        .add_plugin(MessagePlugin)
        .add_plugin(SimulationPlugin)
        .add_plugins(DefaultPickingPlugins);

    app.add_startup_system(setup);

    app.run();
}

fn setup(mut commands: Commands) {
    commands.spawn((Camera2dBundle::default(), RaycastPickCamera::default()));
}

#[derive(Resource, Default)]
pub struct Handles {
    levels: Vec<Handle<Level>>,
}
