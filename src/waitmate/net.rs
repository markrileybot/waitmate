use lazy_static::lazy_static;
use crate::waitmate::api::{Event, EventBus, Named, Notifier, Waiter};

lazy_static! {
    static ref CTX: zmq::Context = zmq::Context::new();
}

pub struct Server {
    skt: zmq::Socket,
    name: String
}
impl Server {
    pub fn new(address: &str) -> Self {
        let skt = CTX.socket(zmq::REP).unwrap();
        skt.bind(address).unwrap();
        return Server {
            skt,
            name: String::from(format!("Server@{}", address))
        }
    }
}
impl Waiter for Server {
    fn wait(&self, bus: &dyn EventBus) {
        let mut msg = zmq::Message::new();
        loop {
            self.skt.recv(&mut msg, 0).unwrap();
            let event: Event = serde_json::from_slice(msg.as_ref()).unwrap();
            bus.publish(event);
            self.skt.send("OK", 0).unwrap();
        }
    }
}
impl Named for Server {
    fn name(&self) -> &str {
        return self.name.as_str();
    }
}


pub struct Client {
    skt: zmq::Socket,
    name: String
}
impl Client {
    pub fn new(address: &str) -> Self {
        let skt = CTX.socket(zmq::REQ).unwrap();
        skt.connect(address).unwrap();
        return Client {
            skt,
            name: String::from(format!("Client@{}", address))
        }
    }
}
impl Notifier for Client {
    fn notify(&self, event: Event, _event_bus: &dyn EventBus) {
        let msg = serde_json::to_vec(&event).unwrap();
        self.skt.send(msg, 0).unwrap();
        let _ = self.skt.recv_string(0).unwrap();
    }
}
impl Named for Client {
    fn name(&self) -> &str {
        return self.name.as_str();
    }
}

#[cfg(test)]
mod tests {
    use crate::waitmate::api::{Named, Event, Level};
    use std::time::{SystemTime, Duration};
    use std::thread::sleep;

    struct TestNamed {
    }
    impl Named for TestNamed {
        fn name(&self) -> &str {
            return "NAMED";
        }
    }

    #[test]
    fn event_io() {
        let source = TestNamed {};
        let start = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_micros();
        let e = Event::new(&source, "a", "b", "c", Level::WARN);
        assert!(e.time >= start);
        assert_eq!("a", e.name);
        assert_eq!("b", e.description);
        assert_eq!("c", e.category);
        assert_eq!(Level::WARN, #[cfg(test)]
mod tests {
    use crate::waitmate::api::{Named, Event, Level};
    use std::time::{SystemTime, Duration};
    use std::thread::sleep;

    struct TestNamed {
    }
    impl Named for TestNamed {
        fn name(&self) -> &str {
            return "NAMED";
        }
    }

    #[test]
    fn event_io() {
        let source = TestNamed {};
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
}e.level);
        sleep(Duration::from_millis(10));
        let e2 = Event::new(&source, "a", "b", "c", Level::WARN);
        assert!(e2.time > e.time);
    }
}