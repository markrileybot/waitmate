use lazy_static::lazy_static;

use crate::waitmate::api::{Event, EventBus, Named, Notifier, Waiter};

lazy_static! {
    static ref CTX: zmq::Context = zmq::Context::new();
}

pub struct Server {
    skt: zmq::Socket,
    name: String,
    kill_byte: bool
}
impl Server {
    pub fn new(address: &str) -> Self {
        return Self::new_test(address, false);
    }
    pub fn new_test(address: &str, kill_byte: bool) -> Self {
        let skt = CTX.socket(zmq::REP).unwrap();
        skt.bind(address).unwrap();
        return Server {
            skt,
            name: String::from(format!("Server@{}", address)),
            kill_byte
        }
    }
}
impl Waiter for Server {
    fn wait(&self, bus: &dyn EventBus) {
        let mut msg = zmq::Message::new();
        loop {
            self.skt.recv(&mut msg, 0).unwrap();
            if self.kill_byte && msg.len() == 1 && msg.as_ref()[0] == 0 {
                break;
            }
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
    use std::{process, thread};
    use std::time::{Duration, SystemTime};

    use crate::waitmate::api::{EmptyEventBus, EmptyNamed, Event, Level, Notifier, Waiter};
    use crate::waitmate::net::{Client, Server};
    use crate::waitmate::thread::EventChannel;

    #[test]
    fn test_event_reqrep() {
        let addr = format!("ipc:///tmp/wmnetrstest.{}", process::id());
        let server = Server::new_test(addr.as_str(), true);
        let client = Client::new(addr.as_str());
        let kill_bytes: [u8;1] = [0];
        let source = EmptyNamed {};

        let (test_server_bus, receiver) = EventChannel::new();
        let test_client_bus = EmptyEventBus {};

        let start = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_micros();
        let e = Event::new(&source, "a", "b", "c", Level::WARN);

        thread::spawn(move || server.wait(&test_server_bus));
        client.notify(e, &test_client_bus);
        client.skt.send(kill_bytes.as_ref(), 0).unwrap(); // kill the server
        let e = receiver.recv_timeout(Duration::from_millis(1000)).unwrap().unwrap();

        assert!(e.time >= start);
        assert_eq!("a", e.name);
        assert_eq!("b", e.description);
        assert_eq!("c", e.category);
        assert_eq!(Level::WARN, e.level);
    }
}