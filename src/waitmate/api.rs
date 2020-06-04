use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
pub enum Level {
    TRACE,
    DEBUG,
    INFO,
    WARN,
    ERROR
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Event {
    pub id: Uuid,
    pub time: u128,
    pub name: String,
    pub description: String,
    pub category: String,
    pub level: Level,
    pub source: String,
}
impl Event {
    pub fn new(source: &dyn Named, name: String, description: String, category: String, level: Level) -> Self {
        Event {
            id: Uuid::new_v4(),
            time: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_micros(),
            name,
            description,
            category,
            level,
            source: String::from(source.name())
        }
    }
}

pub trait Named {
    fn name(&self) -> &str;
}

pub trait EventBus: Send {
    fn publish(&self, event: Event);
}

pub trait Notifier: Send + Named {
    fn notify(&self, event: Event, event_bus: &dyn EventBus);
}

pub trait Waiter: Send + Named {
    fn wait(&self, bus: &dyn EventBus);
}