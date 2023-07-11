use bevy::{math::Vec3Swizzles, prelude::*, sprite::MaterialMesh2dBundle};
use bevy_mod_picking::{
    prelude::{
        Click, Drag, DragEnd, DragStart, IsPointerEvent, ListenedEvent, OnPointer, PointerButton,
        RaycastPickTarget, Up,
    },
    PickableBundle,
};
use bevy_prototype_lyon::{
    prelude::{Fill, GeometryBuilder, Path, ShapeBundle, ShapePath, Stroke},
    shapes,
};

use crate::{
    color,
    events::AddComponentEvent,
    game_state::AppState,
    layer,
    level::{Level, LevelState},
    node::{
        client::Client, database::DatabaseBundle, server::ServerBundle, NodeConnections, NodeType,
        SystemNodeBundle,
    },
    EditSet, Handles,
};

const GRID_SIZE: f32 = 50.0;
const GRID_VERTEX_RADIUS: f32 = GRID_SIZE / 20.0;
const SYSTEM_COMPONENT_NODE_MESH_SCALE: f32 = GRID_SIZE * 2.0;
const SYSTEM_COMPONENT_SPRITE_SIZE: f32 = 100.0;
const SYSTEM_COMPONENT_SPRITE_SCALE: f32 = ((GRID_SIZE + GRID_VERTEX_RADIUS) * 2.0)
    / (SYSTEM_COMPONENT_SPRITE_SIZE * SYSTEM_COMPONENT_NODE_MESH_SCALE);

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
        app.add_event::<ListenedEvent<Click>>();

        app.configure_set(
            DragEventSet
                .run_if(on_event::<ListenedEvent<Drag>>())
                .in_set(EditSet),
        );
        app.configure_set(
            DragEndEventSet
                .run_if(on_event::<ListenedEvent<DragEnd>>())
                .in_set(EditSet),
        );

        app.add_system(spawn_grid.in_schedule(OnEnter(AppState::Edit)));

        app.add_systems(
            (
                add_system_component.run_if(on_event::<AddComponentEvent>()),
                drag_start_node.run_if(on_event::<ListenedEvent<DragStart>>()),
            )
                .in_set(EditSet),
        );

        app.add_systems(
            (drag_node, update_connection_paths::<Drag>)
                .chain()
                .in_set(DragEventSet),
        );

        app.add_systems(
            (drag_end_node, update_connection_paths::<DragEnd>)
                .chain()
                .in_set(DragEndEventSet),
        );

        app.add_system(
            pointer_up_node
                .run_if(on_event::<ListenedEvent<Up>>())
                .before(drag_end_node)
                .in_set(EditSet),
        );

        app.add_system(
            remove_connection
                .run_if(on_event::<ListenedEvent<Click>>())
                .in_set(EditSet),
        );
    }
}

fn spawn_grid(
    mut commands: Commands,
    levels: Res<Assets<Level>>,
    handles: Res<Handles>,
    level_state: Res<LevelState>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
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

    if let Some(selected_level) = level_state.current_level {
        let level = levels.get(&handles.levels[selected_level]).unwrap();

        println!("{:?}", level);

        for (x, y, request_configs) in level.clients.iter() {
            create_component(
                &mut commands,
                &asset_server,
                &mut meshes,
                &mut materials,
                NodeType::Client,
                *x,
                *y,
            );
        }
    }
}

#[derive(Default, Resource)]
struct NodeConnectState {
    start_node_entity: Option<Entity>,
    line_in_progress_entity: Option<Entity>,
}

fn add_system_component(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut add_component_events: EventReader<AddComponentEvent>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for event in add_component_events.into_iter() {
        let node_type = event.0;

        create_component(
            &mut commands,
            &asset_server,
            &mut meshes,
            &mut materials,
            node_type,
            0.0,
            0.0,
        );
    }
}

fn create_component(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<ColorMaterial>>,
    node_type: NodeType,
    x: f32,
    y: f32,
) {
    let texture_path = node_type.get_texture_path();

    let mut node_entity = commands.spawn((
        MaterialMesh2dBundle {
            mesh: meshes.add(Mesh::from(shape::Quad::default())).into(),
            transform: Transform::from_xyz(x, y, layer::SYSTEM_COMPONENTS)
                .with_scale(Vec3::splat(SYSTEM_COMPONENT_NODE_MESH_SCALE)),
            material: materials.add(ColorMaterial::from(Color::NONE)),
            ..default()
        },
        SystemNodeBundle::new(node_type),
        OnPointer::<DragStart>::send_event::<ListenedEvent<DragStart>>(),
        OnPointer::<Drag>::send_event::<ListenedEvent<Drag>>(),
        OnPointer::<DragEnd>::send_event::<ListenedEvent<DragEnd>>(),
        OnPointer::<Up>::send_event::<ListenedEvent<Up>>(),
        PickableBundle::default(),
        RaycastPickTarget::default(),
    ));

    node_entity.with_children(|builder| {
        builder.spawn(SpriteBundle {
            texture: asset_server.load(texture_path),
            transform: Transform::default().with_scale(Vec3::splat(SYSTEM_COMPONENT_SPRITE_SCALE)),
            ..default()
        });
    });

    match node_type {
        NodeType::Client => node_entity.insert(Client::new()),
        NodeType::Server => node_entity.insert(ServerBundle::default()),
        NodeType::Database => node_entity.insert(DatabaseBundle::default()),
    };
}

fn snap_to_grid(position: Vec2, grid_size: f32) -> Vec2 {
    (position / grid_size).round() * grid_size
}

fn drag_node(
    mut drag_event: EventReader<ListenedEvent<Drag>>,
    mut nodes_query: Query<&mut Transform>,
) {
    for drag_event in drag_event.iter() {
        if matches!(drag_event.button, PointerButton::Primary) && drag_event.delta != Vec2::ZERO {
            let mut transform = nodes_query.get_mut(drag_event.target).unwrap();
            transform.translation += Vec3::from((drag_event.delta, 0.0));
        }
    }
}

fn update_connection_paths<E: IsPointerEvent>(
    mut drag_event: EventReader<ListenedEvent<E>>,
    nodes_query: Query<(&Transform, &NodeConnections)>,
    mut path_query: Query<&mut Path, With<NodeConnectionLine>>,
    node_connect_state: Res<NodeConnectState>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera>>,
) {
    for drag_event in drag_event.iter() {
        let (transform, node) = nodes_query.get(drag_event.target).unwrap();

        if let Some(line_in_progress_entity) = node_connect_state.line_in_progress_entity {
            let (camera, camera_transform) = camera_query.single();
            let mouse_pos = camera
                .viewport_to_world_2d(camera_transform, drag_event.pointer_location.position)
                .unwrap();

            if let Ok(mut path) = path_query.get_mut(line_in_progress_entity) {
                let polygon = shapes::Line(transform.translation.xy(), mouse_pos);

                *path = ShapePath::build_as(&polygon);
            }
        } else {
            for (other_node, path) in node.iter() {
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
struct NodeConnectionLine;

fn drag_start_node(
    mut commands: Commands,
    nodes_query: Query<&Transform, With<NodeConnections>>,
    mut drag_event: EventReader<ListenedEvent<DragStart>>,
    mut node_connect_state: ResMut<NodeConnectState>,
) {
    for drag_event in drag_event.iter() {
        if matches!(drag_event.button, PointerButton::Secondary) {
            node_connect_state.start_node_entity = Some(drag_event.target);

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
                    Stroke::new(Color::YELLOW, 5.0),
                    NodeConnectionLine,
                ))
                .id();

            node_connect_state.line_in_progress_entity = Some(connection_path_entity);
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
                if let Some(e) = node_connect_state.line_in_progress_entity.take() {
                    println!("REMOVING ACTIVE PATH");
                    commands.entity(e).despawn_recursive();
                }
                node_connect_state.start_node_entity = None;
                println!("CLEARED NODE CONNECT STATE");
            }
            _ => {}
        }
    }
}

#[derive(Component)]
struct ConnectedNodeConnectionLine(Entity, Entity);

fn pointer_up_node(
    mut commands: Commands,
    mut events: EventReader<ListenedEvent<Up>>,
    mut nodes_query: Query<&mut NodeConnections>,
    mut node_connect_state: ResMut<NodeConnectState>,
    node_types: Query<&NodeType>,
) {
    for pointer_up_event in events.iter() {
        if !matches!(pointer_up_event.button, PointerButton::Secondary) {
            continue;
        }

        if let Some(start_node_entity) = node_connect_state.start_node_entity {
            let end_node_entity = pointer_up_event.target;

            if start_node_entity == end_node_entity {
                continue;
            }

            let start_node_type = node_types.get(start_node_entity).unwrap();
            let end_node_type = node_types.get(end_node_entity).unwrap();

            if !start_node_type.is_valid_connection(end_node_type) {
                println!("INVALID CONNECTION");
                continue;
            }

            let [mut start_node, mut end_node] = nodes_query
                .get_many_mut([start_node_entity, end_node_entity])
                .unwrap();

            if start_node.is_connected_to(end_node_entity) {
                println!("ALREADY MADE CONNETION");
                continue;
            }

            println!("FOUND CONNECTION");

            let line_in_progress_entity = node_connect_state.line_in_progress_entity.unwrap();

            start_node.add_connection(end_node_entity, line_in_progress_entity);
            end_node.add_connection(start_node_entity, line_in_progress_entity);

            commands
                .entity(node_connect_state.line_in_progress_entity.unwrap())
                .insert(ConnectedNodeConnectionLine(
                    start_node_entity,
                    end_node_entity,
                ))
                .insert(OnPointer::<Click>::send_event::<ListenedEvent<Click>>())
                .insert(PickableBundle::default())
                .insert(RaycastPickTarget::default());

            node_connect_state.line_in_progress_entity = None;
        }
    }
}

fn remove_connection(
    mut commands: Commands,
    mut events: EventReader<ListenedEvent<Click>>,
    connections: Query<&ConnectedNodeConnectionLine>,
    mut nodes: Query<&mut NodeConnections>,
) {
    for event in events.iter() {
        if matches!(event.button, PointerButton::Secondary) {
            let connection = connections.get(event.target).unwrap();

            nodes
                .get_mut(connection.0)
                .unwrap()
                .remove_connection(connection.1);

            nodes
                .get_mut(connection.1)
                .unwrap()
                .remove_connection(connection.0);

            commands.entity(event.target).despawn_recursive();
        }
    }
}
