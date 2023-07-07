use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use bevy_mod_picking::{prelude::RaycastPickCamera, DefaultPickingPlugins};
use bevy_prototype_lyon::prelude::*;
use events::{AddComponentEvent, StartSimulationEvent};
use game_state::{GameState, GameStatePlugin};
use game_ui::GameUiPlugin;
use grid::GridPlugin;
use message::{MessageArrivedEvent, MessagePlugin, SendMessageEvent};

use simulation::SimulationPlugin;

mod color;
mod events;
mod game_state;
mod game_ui;
mod grid;
mod layer;
mod message;
mod node;
mod simulation;

fn main() {
    let mut app = App::new();

    app.insert_resource(ClearColor(color::BACKGROUND))
        .insert_resource(Msaa::Sample4);

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
    app.add_event::<StartSimulationEvent>();
    app.add_event::<SendMessageEvent>();
    app.add_event::<MessageArrivedEvent>();

    app.add_state::<GameState>();

    app.add_plugins(default)
        .add_plugin(ShapePlugin)
        .add_plugin(EguiPlugin)
        .add_plugin(GameUiPlugin)
        .add_plugin(GridPlugin)
        .add_plugin(GameStatePlugin)
        .add_plugin(MessagePlugin)
        .add_plugin(SimulationPlugin)
        .add_plugins(DefaultPickingPlugins);

    app.add_startup_system(setup);

    app.run();
}

fn setup(mut commands: Commands) {
    commands.spawn((Camera2dBundle::default(), RaycastPickCamera::default()));
}
