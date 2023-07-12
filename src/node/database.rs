use std::collections::{HashMap, VecDeque};

use bevy::prelude::*;
use serde_json::{Map, Value};

use crate::message::{DatabaseCallType, Message, MessageComponent, SendMessageEvent};

use super::SystemNodeTrait;

type Document = Map<String, Value>;

#[derive(Component, Clone, Debug)]
pub struct Database {
    pub documents: HashMap<i32, Document>,
    state: DatabaseState,
    message_queue: VecDeque<MessageComponent>,
    can_be_edited: bool,
}

impl Database {
    pub fn new() -> Self {
        Self {
            documents: Default::default(),
            state: Default::default(),
            message_queue: Default::default(),
            can_be_edited: true,
        }
    }

    fn save(&mut self, mut doc: Document) -> Document {
        let id = match doc.get("id") {
            Some(id) => id.as_u64().unwrap() as i32,
            None => {
                let id = self.documents.len() as i32;
                doc.insert("id".to_string(), Value::from(id));
                id
            }
        };
        self.documents.insert(id, doc.clone());
        doc
    }

    fn find_one(&self, id: i32) -> Option<Document> {
        self.documents.get(&id).cloned()
    }

    fn find_all(&self) -> Vec<Document> {
        self.documents.values().cloned().collect()
    }

    fn contains(&self, id: i32) -> bool {
        self.documents.contains_key(&id)
    }

    fn delete(&mut self, id: i32) {
        self.documents.remove(&id);
    }
}

impl SystemNodeTrait for Database {
    fn start_simulation(&mut self) {
        self.state = DatabaseState::Active;
    }

    fn handle_message(&mut self, message: MessageComponent) {
        println!("HANDLING MESSAGE FOR DATABASE:");
        println!("{:?}", message);

        self.message_queue.push_back(message);
    }

    fn can_be_edited(&self) -> bool {
        self.can_be_edited
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum DatabaseState {
    #[default]
    SimulationNotStarted,
    Active,
}

pub fn database_system(
    mut database_query: Query<(Entity, &mut Database)>,
    mut events: EventWriter<SendMessageEvent>,
) {
    for (database_entity, mut database) in database_query.iter_mut() {
        if database.state == DatabaseState::Active {
            if database.message_queue.is_empty() {
                continue;
            }

            let message_queue = database.message_queue.drain(..).collect::<Vec<_>>();

            for message in message_queue {
                if let Message::DatabaseCall(db_call) = message.message {
                    let db_answer_value = match db_call.call_type {
                        DatabaseCallType::Save(value) => {
                            let document = serde_json::from_value::<Document>(value);

                            match document {
                                Ok(document) => {
                                    let saved_doc = database.save(document);
                                    Value::from(saved_doc)
                                }
                                Err(_) => Value::Null, // TODO: Probably should wrap in a db result type to indicate if there was an error?
                            }
                        }
                        DatabaseCallType::FindOne(id) => {
                            let document = database.find_one(id as i32);
                            Value::from(document)
                        }
                        DatabaseCallType::FindAll => {
                            let documents = database.find_all();
                            Value::from(documents)
                        }
                        DatabaseCallType::Contains(id) => {
                            let contains = database.contains(id as i32);
                            Value::from(contains)
                        }
                        DatabaseCallType::Delete(id) => {
                            database.delete(id as i32);
                            Value::Null
                        }
                    };

                    events.send(SendMessageEvent {
                        sender: database_entity,
                        recipients: vec![message.sender],
                        message: Message::DatabaseAnswer(db_answer_value),
                        trace_id: message.trace_id,
                    });
                }
            }
        }
    }
}
