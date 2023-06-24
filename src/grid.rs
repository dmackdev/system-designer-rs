use bevy::{math::Vec3Swizzles, prelude::*};
use bevy_mod_picking::prelude::{
    Drag, DragEnd, DragStart, ListenedEvent, OnPointer, PointerButton, Up,
};
use bevy_prototype_lyon::{
    prelude::{Fill, GeometryBuilder, Path, ShapeBundle, ShapePath, Stroke},
    shapes,
};

use crate::{color, events::AddComponentEvent, game_state::GameState, layer};

const GRID_SIZE: f32 = 50.0;
const GRID_VERTEX_RADIUS: f32 = GRID_SIZE / 20.0;
const SYSTEM_COMPONENT_SCALE: f32 = ((GRID_SIZE + GRID_VERTEX_RADIUS) * 2.0) / 100.0;

pub struct GridPlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
struct DragEventSet;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
struct DragEndEventSet;

impl Plugin for GridPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NodeConnectState>();
        app.add_event::<ListenedEvent<Drag>>();
        app.add_event::<ListenedEvent<DragStart>>();
        app.add_event::<ListenedEvent<DragEnd>>();
        app.add_event::<ListenedEvent<Up>>();

        app.configure_set(DragEventSet.run_if(on_event::<ListenedEvent<Drag>>()));
        app.configure_set(DragEndEventSet.run_if(on_event::<ListenedEvent<DragEnd>>()));

        app.add_system(spawn_grid.in_schedule(OnEnter(GameState::Playing)))
            .add_system(add_system_component.run_if(on_event::<AddComponentEvent>()))
            .add_system(drag_start_node.run_if(on_event::<ListenedEvent<DragStart>>()));

        app.add_systems(
            (drag_node, update_connection_paths::<ListenedEvent<Drag>>)
                .chain()
                .in_set(DragEventSet),
        );

        app.add_systems(
            (
                drag_end_node,
                update_connection_paths::<ListenedEvent<DragEnd>>,
            )
                .chain()
                .in_set(DragEndEventSet),
        );

        app.add_system(
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
    active_path: Option<Entity>,
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
            Node {
                connections: vec![],
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

#[derive(Component)]
struct Node {
    connections: Vec<(Entity, Entity)>,
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

trait PointerEventOnNode {
    fn target(&self) -> Entity;
    fn pointer_position(&self) -> Vec2;
}

impl PointerEventOnNode for ListenedEvent<Drag> {
    fn target(&self) -> Entity {
        self.target
    }

    fn pointer_position(&self) -> Vec2 {
        self.pointer_location.position
    }
}

impl PointerEventOnNode for ListenedEvent<DragEnd> {
    fn target(&self) -> Entity {
        self.target
    }

    fn pointer_position(&self) -> Vec2 {
        self.pointer_location.position
    }
}

fn update_connection_paths<E: PointerEventOnNode + Send + Sync + 'static>(
    mut drag_event: EventReader<E>,
    nodes_query: Query<(Ref<Transform>, &Node)>,
    mut path_query: Query<&mut Path>,
    node_connect_state: Res<NodeConnectState>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera>>,
) {
    for drag_event in drag_event.iter() {
        let (transform, node) = nodes_query.get(drag_event.target()).unwrap();

        if let Some(active_path) = node_connect_state.active_path {
            let (camera, camera_transform) = camera_query.single();
            let mouse_pos = camera
                .viewport_to_world_2d(camera_transform, drag_event.pointer_position())
                .unwrap();

            if let Ok(mut path) = path_query.get_mut(active_path) {
                let polygon = shapes::Line(transform.translation.xy(), mouse_pos);

                *path = ShapePath::build_as(&polygon);
            }
        } else {
            for (other_node, path) in node.connections.iter() {
                let mut path = path_query.get_mut(*path).unwrap();

                let polygon = shapes::Line(
                    transform.translation.xy(),
                    nodes_query.get(*other_node).unwrap().0.translation.xy(),
                );

                *path = ShapePath::build_as(&polygon);
            }
        }
    }
}

#[derive(Component)]
struct ActivePath;

fn drag_start_node(
    mut commands: Commands,
    nodes_query: Query<&Transform, With<Node>>,
    mut drag_event: EventReader<ListenedEvent<DragStart>>,
    mut node_connect_state: ResMut<NodeConnectState>,
) {
    for drag_event in drag_event.iter() {
        if matches!(drag_event.button, PointerButton::Secondary) {
            node_connect_state.start = Some(drag_event.target);

            let connection_path_entity = commands
                .spawn((
                    ShapeBundle {
                        path: GeometryBuilder::build_as(&shapes::Line(
                            nodes_query.get(drag_event.target).unwrap().translation.xy(),
                            nodes_query.get(drag_event.target).unwrap().translation.xy(),
                        )),
                        transform: Transform::from_xyz(0.0, 0.0, layer::CONNECTIONS),
                        ..default()
                    },
                    Stroke::new(Color::YELLOW, 2.0),
                    ActivePath,
                ))
                .id();

            node_connect_state.active_path = Some(connection_path_entity);
        }
    }
}

fn drag_end_node(
    mut commands: Commands,
    mut drag_event: EventReader<ListenedEvent<DragEnd>>,
    mut nodes_query: Query<&mut Transform>,
    mut node_connect_state: ResMut<NodeConnectState>,
) {
    for drag_event in drag_event.iter() {
        match drag_event.button {
            PointerButton::Primary => {
                let mut transform = nodes_query.get_mut(drag_event.target).unwrap();
                transform.translation = snap_to_grid(transform.translation.xy(), GRID_SIZE)
                    .extend(layer::SYSTEM_COMPONENTS);
            }
            PointerButton::Secondary => {
                if let Some(e) = node_connect_state.active_path.take() {
                    println!("REMOVING ACTIVE PATH");
                    commands.entity(e).despawn_recursive();
                }
                node_connect_state.start = None;
                println!("CLEARED NODE CONNECT STATE");
            }
            _ => {}
        }
    }
}

fn pointer_up_node(
    mut commands: Commands,
    mut events: EventReader<ListenedEvent<Up>>,
    mut nodes_query: Query<(&Transform, &mut Node)>,
    mut node_connect_state: ResMut<NodeConnectState>,
) {
    for pointer_up_event in events.iter() {
        if matches!(pointer_up_event.button, PointerButton::Secondary) {
            if let Some(start_node) = node_connect_state.start {
                let end_node = pointer_up_event.target;

                if start_node != end_node {
                    println!("FOUND CONNECTION");
                    let mut nodes = nodes_query.get_many_mut([start_node, end_node]).unwrap();

                    nodes[0]
                        .1
                        .connections
                        .push((end_node, node_connect_state.active_path.unwrap()));
                    nodes[1]
                        .1
                        .connections
                        .push((start_node, node_connect_state.active_path.unwrap()));

                    commands
                        .entity(node_connect_state.active_path.unwrap())
                        .remove::<ActivePath>();

                    node_connect_state.active_path = None;
                }
            }
        }
    }
}
