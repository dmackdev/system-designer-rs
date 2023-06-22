use bevy::prelude::*;
use bevy_mod_picking::prelude::{
    Drag, DragEnd, DragStart, ListenedEvent, OnPointer, PointerButton, Up,
};
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
        app.init_resource::<NodeConnectState>();
        app.add_event::<ListenedEvent<Drag>>();
        app.add_event::<ListenedEvent<DragStart>>();
        app.add_event::<ListenedEvent<DragEnd>>();
        app.add_event::<ListenedEvent<Up>>();

        app.add_system(spawn_grid.in_schedule(OnEnter(GameState::Playing)))
            .add_system(add_system_component.run_if(on_event::<AddComponentEvent>()))
            .add_system(drag_start_node.run_if(on_event::<ListenedEvent<DragStart>>()))
            .add_system(drag_node.run_if(on_event::<ListenedEvent<Drag>>()))
            .add_system(drag_end_node.run_if(on_event::<ListenedEvent<DragEnd>>()))
            .add_system(
                pointer_up_node
                    .run_if(on_event::<ListenedEvent<Up>>())
                    .before(drag_end_node),
            );
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

#[derive(Default, Resource)]
struct NodeConnectState {
    start: Option<Entity>,
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
            OnPointer::<DragStart>::send_event::<ListenedEvent<DragStart>>(),
            OnPointer::<Drag>::send_event::<ListenedEvent<Drag>>(),
            OnPointer::<DragEnd>::send_event::<ListenedEvent<DragEnd>>(),
            OnPointer::<Up>::send_event::<ListenedEvent<Up>>(),
        ));
    }
}

fn snap_to_grid(position: Vec2, grid_size: f32) -> Vec2 {
    (position / grid_size).round() * grid_size
}

fn drag_node(
    mut drag_event: EventReader<ListenedEvent<Drag>>,
    mut nodes_query: Query<&mut Transform>,
) {
    for drag_event in drag_event.iter() {
        if matches!(drag_event.button, PointerButton::Primary) {
            let mut transform = nodes_query.get_mut(drag_event.target).unwrap();
            transform.translation += Vec3::from((drag_event.delta, 0.0));
        }
    }
}

fn drag_start_node(
    mut drag_event: EventReader<ListenedEvent<DragStart>>,
    mut node_connect_state: ResMut<NodeConnectState>,
) {
    for drag_event in drag_event.iter() {
        if matches!(drag_event.button, PointerButton::Secondary) {
            node_connect_state.start = Some(drag_event.target);
        }
    }
}

fn drag_end_node(
    mut drag_event: EventReader<ListenedEvent<DragEnd>>,
    mut nodes_query: Query<&mut Transform>,
    mut node_connect_state: ResMut<NodeConnectState>,
) {
    for drag_event in drag_event.iter() {
        match drag_event.button {
            PointerButton::Primary => {
                let mut transform = nodes_query.get_mut(drag_event.target).unwrap();
                transform.translation = snap_to_grid(
                    Vec2::new(transform.translation.x, transform.translation.y),
                    GRID_SIZE,
                )
                .extend(layer::SYSTEM_COMPONENTS);
            }
            PointerButton::Secondary => {
                node_connect_state.start = None;
                println!("CLEARED NODE CONNECT STATE");
            }
            _ => {}
        }
    }
}

fn pointer_up_node(
    mut events: EventReader<ListenedEvent<Up>>,
    node_connect_state: Res<NodeConnectState>,
) {
    for pointer_up_event in events.iter() {
        if matches!(pointer_up_event.button, PointerButton::Secondary) {
            if let Some(start_node) = node_connect_state.start {
                let end_node = pointer_up_event.target;

                if start_node != end_node {
                    println!("FOUND CONNECTION");
                }
            }
        }
    }
}
