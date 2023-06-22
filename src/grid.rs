use bevy::prelude::*;
use bevy_mod_picking::prelude::{Drag, DragEnd, OnPointer};
use bevy_prototype_lyon::{
    prelude::{Fill, GeometryBuilder, ShapeBundle},
    shapes,
};

use crate::{color, events::AddComponentEvent, game_state::GameState, layer};

const GRID_SIZE: f32 = 50.0;
const GRID_VERTEX_RADIUS: f32 = GRID_SIZE / 20.0;
const SYSTEM_COMPONENT_SCALE: f32 = ((GRID_SIZE + GRID_VERTEX_RADIUS) * 2.0) / 100.0;

pub struct GridPlugin;

impl Plugin for GridPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(spawn_grid.in_schedule(OnEnter(GameState::Playing)))
            .add_system(add_system_component);
    }
}

fn spawn_grid(mut commands: Commands) {
    for x in ((-25 * (GRID_SIZE as i32))..=25 * (GRID_SIZE as i32)).step_by(GRID_SIZE as usize) {
        for y in (-15 * (GRID_SIZE as i32)..=15 * (GRID_SIZE as i32)).step_by(GRID_SIZE as usize) {
            commands.spawn((
                ShapeBundle {
                    path: GeometryBuilder::build_as(&shapes::Circle {
                        radius: GRID_VERTEX_RADIUS,
                        ..Default::default()
                    }),
                    transform: Transform::from_xyz(x as f32, y as f32, layer::GRID),
                    ..default()
                },
                Fill::color(color::GRID),
            ));
        }
    }
}

fn add_system_component(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut add_component_events: EventReader<AddComponentEvent>,
) {
    for _ in add_component_events.iter() {
        commands.spawn((
            SpriteBundle {
                texture: asset_server.load("textures/system_components/server.png"),
                transform: Transform::from_xyz(0.0, 0.0, layer::SYSTEM_COMPONENTS)
                    .with_scale(Vec3::splat(SYSTEM_COMPONENT_SCALE)),
                ..default()
            },
            OnPointer::<Drag>::target_component_mut::<Transform>(|drag, transform| {
                transform.translation += Vec3::from((drag.delta, 0.0));
            }),
            OnPointer::<DragEnd>::target_component_mut::<Transform>(|_, transform| {
                transform.translation = snap_to_grid(
                    Vec2::new(transform.translation.x, transform.translation.y),
                    GRID_SIZE,
                )
                .extend(layer::SYSTEM_COMPONENTS);
            }),
        ));
    }
}

fn snap_to_grid(position: Vec2, grid_size: f32) -> Vec2 {
    (position / grid_size).round() * grid_size
}
