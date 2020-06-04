use std::{io, str};
use std::io::BufRead;
use std::thread::sleep;
use std::time::Duration;

use regex::Regex;

use crate::waitmate::api::{Event, EventBus, Level, Named, Notifier, Waiter};

pub struct StdinWaiter {
    matcher: Regex
}
impl StdinWaiter {
    const NAME: &'static str = "StdoutNotifier";
    pub fn new() -> StdinWaiter {
        StdinWaiter {
            matcher: Regex::new(r"^(.*)bash(.*)$").unwrap()
        }
    }
}
impl Named for StdinWaiter {
    fn name(&self) -> &str {return StdinWaiter::NAME;}
}
impl Waiter for StdinWaiter {
    fn wait(&self, bus: &dyn EventBus) {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            let l = line.unwrap();
            if self.matcher.is_match(l.as_str()) {
                let e = Event::new(
                    self,
                    "A name".to_string(),
                    l,
                    "Cat".to_string(),
                    Level::WARN
                );
                bus.publish(e);
            }
        }
    }
}
pub struct SleepyWaiter {
}
impl SleepyWaiter {
    const NAME: &'static str = "SleepyWaiter";
    pub fn new() -> SleepyWaiter {
        return SleepyWaiter {}
    }
}
impl Named for SleepyWaiter {
    fn name(&self) -> &str {
        return SleepyWaiter::NAME;
    }
}
impl Waiter for SleepyWaiter {
    fn wait(&self, bus: &dyn EventBus) {
        for i in 0..10 {
            bus.publish(Event::new(
                self,
                "A name".to_string(),
                format!("EVENT {}", i),
                "Doggo".to_string(),
                Level::WARN
            ));
            sleep(Duration::from_millis(1));
        }
    }
}

pub struct StdoutNotifier {
}
impl StdoutNotifier {
    const NAME: &'static str = "StdoutNotifier";
    pub fn new() -> StdoutNotifier {
        return StdoutNotifier {};
    }
}
impl Named for StdoutNotifier {
    fn name(&self) -> &str {return StdoutNotifier::NAME;}
}
impl Notifier for StdoutNotifier {
    fn notify(&self, event: Event, _: &dyn EventBus) {
        println!("{:?}", event);
    }
}