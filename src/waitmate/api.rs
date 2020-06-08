use core::fmt;
use std::fmt::Display;
use std::time::SystemTime;

use config::Config;
use serde::{Deserialize, Serialize};
use serde::export::Formatter;
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum Level {
    TRACE,
    DEBUG,
    INFO,
    WARN,
    ERROR
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
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
    pub fn new(source: &dyn Named, name: &str, description: &str, category: &str, level: Level) -> Self {
        Event {
            id: Uuid::new_v4(),
            time: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_micros(),
            name: String::from(name),
            description: String::from(description),
            category: String::from(category),
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

pub struct EmptyNamed {
}
impl Named for EmptyNamed {
    fn name(&self) -> &str {
        return "NAMED";
    }
}

pub struct EmptyEventBus {
}
impl EventBus for EmptyEventBus {
    fn publish(&self, _event: Event) {
    }
}

#[cfg(test)]
mod tests {
    use std::thread::sleep;
    use std::time::{Duration, SystemTime};

    use crate::waitmate::api::{EmptyNamed, Event, Level};

    #[test]
    fn event_io() {
        let source = EmptyNamed {};
        let start = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_micros();
        let e = Event::new(&source, "a", "b", "c", Level::WARN);
        assert!(e.time >= start);
        assert_eq!("a", e.name);
        assert_eq!("b", e.description);
        assert_eq!("c", e.category);
        assert_eq!(Level::WARN, e.level);
        sleep(Duration::from_millis(10));
        let e2 = Event::new(&source, "a", "b", "c", Level::WARN);
        assert!(e2.time > e.time);
    }
}