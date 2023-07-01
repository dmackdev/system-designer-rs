use bevy::{
    prelude::*,
    sprite::{ColorMaterial, MaterialMesh2dBundle},
};

pub struct MessagePlugin;

impl Plugin for MessagePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(handle_send_message_event.run_if(on_event::<SendMessageEvent>()));
        app.add_system(update_messages);
    }
}

#[derive(Component)]
pub struct MessageComponent {
    sender: Entity,
    recipient: Entity,
    message: Message,
}

#[derive(Clone)]
pub enum Message {
    ClientRequest(String),
}

pub struct SendMessageEvent {
    pub sender: Entity,
    pub recipients: Vec<Entity>,
    pub message: Message,
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
                },
            ));
        }
    }
}

fn update_messages(
    mut commands: Commands,
    mut messages: Query<(Entity, &MessageComponent, &mut Transform)>,
    transforms: Query<&Transform, Without<MessageComponent>>,
    time: Res<Time>,
) {
    for (message_entity, message, mut message_transform) in messages.iter_mut() {
        let current_pos = message_transform.translation;
        let destination = transforms.get(message.recipient).unwrap().translation;
        let delta_to_destination = destination - current_pos;

        if delta_to_destination.length_squared() < 0.25 {
            commands.entity(message_entity).despawn_recursive();
        } else {
            let movement_delta = 100.0 * time.delta_seconds() * delta_to_destination.normalize();
            message_transform.translation += movement_delta;
        }
    }
}
