use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::fmt::Display;
use serde::export::Formatter;
use core::fmt;

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
impl Display for Event {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
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