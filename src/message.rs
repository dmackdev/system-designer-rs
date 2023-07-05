use std::collections::HashMap;

use bevy::{
    prelude::*,
    sprite::{ColorMaterial, MaterialMesh2dBundle},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::node::{
    client::{Client, HttpMethod, RequestConfig},
    server::Server,
    SystemNodeTrait,
};

pub struct MessagePlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
struct MessageArrivedEventSet;

impl Plugin for MessagePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(handle_send_message_event.run_if(on_event::<SendMessageEvent>()));
        app.add_system(move_messages);

        app.configure_set(
            MessageArrivedEventSet
                .run_if(on_event::<MessageArrivedEvent>())
                .after(move_messages),
        );

        app.add_systems(
            (handle_message_for::<Client>, handle_message_for::<Server>)
                .in_set(MessageArrivedEventSet),
        );
    }
}

#[derive(Component, Clone, Debug)]
pub struct MessageComponent {
    pub sender: Entity,
    pub recipient: Entity,
    pub message: Message,
    pub trace_id: Uuid,
}

#[derive(Clone, Debug)]
pub enum Message {
    Request(Request),
    Response(Response),
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq)]
pub struct Request {
    pub url: String,
    pub path: String,
    pub method: HttpMethod,
    pub body: Value,
    pub params: HashMap<String, String>,
}

impl TryFrom<RequestConfig> for Request {
    type Error = ();

    fn try_from(value: RequestConfig) -> Result<Self, Self::Error> {
        let body = match value.method {
            HttpMethod::Get => Value::Null,
            HttpMethod::Post => serde_json::from_str(&value.body).unwrap(), // TODO: handle if this fails
        };

        Ok(Self {
            url: value.url,
            path: value.path,
            method: value.method,
            body,
            params: HashMap::new(),
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    pub data: Value,
}

impl Response {
    pub fn not_found() -> Self {
        Self {
            status: 404,
            data: Value::String("Not found".to_string()),
        }
    }
}

pub struct SendMessageEvent {
    pub sender: Entity,
    pub recipients: Vec<Entity>,
    pub message: Message,
    pub trace_id: Uuid,
}

fn handle_send_message_event(
    mut commands: Commands,
    mut send_message_events: EventReader<SendMessageEvent>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    transforms: Query<&Transform>,
) {
    for event in send_message_events.iter() {
        for recipient in event.recipients.iter() {
            commands.spawn((
                MaterialMesh2dBundle {
                    mesh: meshes.add(shape::Circle::new(15.).into()).into(),
                    material: materials.add(ColorMaterial::from(Color::YELLOW)),
                    transform: Transform::from_translation(
                        transforms.get(event.sender).unwrap().translation,
                    ),
                    ..default()
                },
                MessageComponent {
                    sender: event.sender,
                    recipient: *recipient,
                    message: event.message.clone(),
                    trace_id: event.trace_id,
                },
            ));
        }
    }
}

fn move_messages(
    mut commands: Commands,
    mut messages: Query<(Entity, &MessageComponent, &mut Transform)>,
    transforms: Query<&Transform, Without<MessageComponent>>,
    time: Res<Time>,
    mut events: EventWriter<MessageArrivedEvent>,
) {
    for (message_entity, message, mut message_transform) in messages.iter_mut() {
        let current_pos = message_transform.translation;
        let destination = transforms.get(message.recipient).unwrap().translation;
        let delta_to_destination = destination - current_pos;

        if delta_to_destination.length_squared() < 0.25 {
            events.send(MessageArrivedEvent(message.clone()));
            commands.entity(message_entity).despawn_recursive();
        } else {
            let movement_delta = 100.0 * time.delta_seconds() * delta_to_destination.normalize();
            message_transform.translation += movement_delta;
        }
    }
}

pub struct MessageArrivedEvent(pub MessageComponent);

fn handle_message_for<T: SystemNodeTrait + Component>(
    mut events: EventReader<MessageArrivedEvent>,
    mut nodes: Query<&mut T>,
) {
    for event in events.into_iter() {
        let message = &event.0;
        if let Ok(mut node) = nodes.get_mut(message.recipient) {
            node.handle_message(message.clone());
        }
    }
}
