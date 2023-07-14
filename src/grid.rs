use bevy::{
    ecs::system::SystemParam, math::Vec3Swizzles, prelude::*, sprite::MaterialMesh2dBundle,
};
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
    events::{AddComponentEvent, AddComponentPayload},
    game_state::AppState,
    layer,
    level::{ClientConfig, Level, LevelState},
    node::{client::Client, Hostname, NodeConnections, NodeType, SystemNodeBundle},
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
        app.add_event::<ConnectionLineClickEvent>();
        app.add_event::<DeleteNodeEvent>();

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

        app.add_system(spawn_grid.in_schedule(OnExit(AppState::MainMenu)));
        app.add_system(spawn_grid.in_schedule(OnExit(AppState::LevelSelect)));

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
                .run_if(on_event::<ConnectionLineClickEvent>())
                .in_set(EditSet),
        );

        app.add_system(
            delete_node
                .run_if(on_event::<DeleteNodeEvent>())
                .in_set(EditSet),
        );
    }
}

#[derive(SystemParam)]
pub struct CurrentLevel<'w> {
    levels: Res<'w, Assets<Level>>,
    handles: Res<'w, Handles>,
    level_state: Res<'w, LevelState>,
}

impl<'w> CurrentLevel<'w> {
    fn get(&self) -> Option<&Level> {
        self.level_state
            .current_level
            .and_then(|idx| self.levels.get(&self.handles.levels[idx]))
    }
}

fn spawn_grid(
    mut commands: Commands,
    current_level: CurrentLevel,
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

    if let Some(level) = current_level.get() {
        println!("{:?}", level);

        for ClientConfig {
            name,
            x,
            y,
            request_configs,
        } in level.clients.iter()
        {
            let client = Client::new()
                .editable(false)
                .request_configs(request_configs.to_vec());

            let system_bundle = SystemNodeBundle::new(NodeType::Client).node_name(name.into());

            create_component(
                &mut commands,
                &asset_server,
                &mut meshes,
                &mut materials,
                system_bundle,
                AddComponentPayload::Client(client),
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
        let component = event.0.clone();

        create_component(
            &mut commands,
            &asset_server,
            &mut meshes,
            &mut materials,
            SystemNodeBundle::new(component.get_node_type()),
            component,
            0.0,
            0.0,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn create_component(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<ColorMaterial>>,
    system_node_bundle: SystemNodeBundle,
    component: AddComponentPayload,
    x: f32,
    y: f32,
) {
    let node_type = component.get_node_type();

    let mut node_entity = commands.spawn((
        MaterialMesh2dBundle {
            mesh: meshes.add(Mesh::from(shape::Quad::default())).into(),
            transform: Transform::from_xyz(x, y, layer::SYSTEM_COMPONENTS)
                .with_scale(Vec3::splat(SYSTEM_COMPONENT_NODE_MESH_SCALE)),
            material: materials.add(ColorMaterial::from(Color::NONE)),
            ..default()
        },
        system_node_bundle,
        OnPointer::<DragStart>::send_event::<ListenedEvent<DragStart>>(),
        OnPointer::<Drag>::send_event::<ListenedEvent<Drag>>(),
        OnPointer::<DragEnd>::send_event::<ListenedEvent<DragEnd>>(),
        OnPointer::<Up>::send_event::<ListenedEvent<Up>>(),
        PickableBundle::default(),
        RaycastPickTarget::default(),
    ));

    node_entity.with_children(|builder| {
        builder.spawn(SpriteBundle {
            texture: asset_server.load(node_type.get_texture_path()),
            transform: Transform::default().with_scale(Vec3::splat(SYSTEM_COMPONENT_SPRITE_SCALE)),
            ..default()
        });
    });

    match component {
        AddComponentPayload::Client(client) => node_entity.insert(client),
        AddComponentPayload::Server(server) => node_entity.insert((server, Hostname::default())),
        AddComponentPayload::Database(database) => {
            node_entity.insert((database, Hostname::default()))
        }
    };
}

fn delete_node(
    mut commands: Commands,
    mut events: EventReader<DeleteNodeEvent>,
    mut remove_connections: RemoveConnectionSystemParam,
) {
    for event in events.iter() {
        remove_connections.remove_connections_by_node(event.0);
        commands.entity(event.0).despawn_recursive();
    }
}

pub struct DeleteNodeEvent(pub Entity);

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
                .insert(OnPointer::<Click>::send_event::<ConnectionLineClickEvent>())
                .insert(PickableBundle::default())
                .insert(RaycastPickTarget::default());

            node_connect_state.line_in_progress_entity = None;
        }
    }
}

struct ConnectionLineClickEvent(ListenedEvent<Click>);

impl From<ListenedEvent<Click>> for ConnectionLineClickEvent {
    fn from(value: ListenedEvent<Click>) -> Self {
        Self(value)
    }
}

fn remove_connection(
    mut events: EventReader<ConnectionLineClickEvent>,
    mut remove_connections: RemoveConnectionSystemParam,
) {
    for event in events.iter() {
        if matches!(event.0.button, PointerButton::Secondary) {
            remove_connections.remove_connection_by_line(event.0.target);
        }
    }
}

#[derive(SystemParam)]
struct RemoveConnectionSystemParam<'w, 's> {
    commands: Commands<'w, 's>,
    lines: Query<'w, 's, &'static ConnectedNodeConnectionLine>,
    nodes: Query<'w, 's, &'static mut NodeConnections>,
}

impl<'w, 's> RemoveConnectionSystemParam<'w, 's> {
    fn remove_connection_by_line(&mut self, line_entity: Entity) {
        let connection = self.lines.get(line_entity).unwrap();

        self.nodes
            .get_mut(connection.0)
            .unwrap()
            .remove_connection(connection.1);

        self.nodes
            .get_mut(connection.1)
            .unwrap()
            .remove_connection(connection.0);

        self.commands.entity(line_entity).despawn_recursive();
    }

    fn remove_connections_by_node(&mut self, node_entity: Entity) {
        let line_entities = self.nodes.get(node_entity).unwrap().line_entities();

        for line_entity in line_entities {
            self.remove_connection_by_line(line_entity);
        }
    }
}
