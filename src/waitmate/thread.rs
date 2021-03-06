use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;
use log::info;

use crossbeam::channel::{bounded, Receiver, Sender, unbounded};

use crate::waitmate::api::{Event, EventBus, Notifier, Waiter};
use crate::waitmate::log::EventLog;

pub trait Producer {
    fn channel(&self) -> &Receiver<Option<Event>>;
}

pub struct EventChannel {
    sender: Sender<Option<Event>>,
    done: bool
}
impl EventChannel {
    pub fn new() -> (EventChannel, Receiver<Option<Event>>) {
        let (sender, receiver): (Sender<Option<Event>>, Receiver<Option<Event>>) = unbounded();
        return (EventChannel {
            sender,
            done: false
        }, receiver);
    }
    pub fn done(&mut self) {
        if !self.done {
            self.done = true;
            self.sender.send(None).unwrap_or(());
        }
    }
}
impl EventBus for EventChannel {
    fn publish(&self, event: Event) {
        self.sender.send(Some(event)).unwrap();
    }
}
impl Drop for EventChannel {
    fn drop(&mut self) {
        self.done();
    }
}

pub struct NotifierThread {
    handle: Option<JoinHandle<()>>,
    receiver: Receiver<Option<Event>>,
    tickler: Sender<bool>
}
impl NotifierThread {
    pub fn new(notifier: Box<dyn Notifier>, event_log: Arc<EventLog>) -> NotifierThread {
        let (tickler, ticklee): (Sender<bool>, Receiver<bool>) = bounded(1);
        let (event_bus, receiver) = EventChannel::new();
        let handle = thread::Builder::new()
            .name(String::from(notifier.name()))
            .spawn(move || {
                let cursor = event_log.build_cursor()
                    .named(notifier.name())
                    .tailing(Some(ticklee))
                    .build();
                for (_, event) in cursor {
                    notifier.notify(event, &event_bus);
                }
            }).unwrap();

        return NotifierThread {
            handle: Some(handle),
            receiver,
            tickler
        };
    }
    pub fn tickle(&self) {
        if self.tickler.is_empty() {
            self.tickler.send(true).unwrap();
        }
    }
}
impl Drop for NotifierThread {
    fn drop(&mut self) {
        self.tickler.send(false).unwrap();
        self.handle
            .take().unwrap()
            .join().unwrap();
    }
}
impl Producer for NotifierThread {
    fn channel(&self) -> &Receiver<Option<Event>> {
        return &self.receiver;
    }
}

pub struct WaiterThread {
    handle: JoinHandle<()>,
    receiver: Receiver<Option<Event>>
}
impl WaiterThread {
    pub fn new(waiter: Box<dyn Waiter>) -> WaiterThread {
        let (event_bus, receiver) = EventChannel::new();

        let handle = thread::Builder::new()
            .name(String::from(waiter.name()))
            .spawn(move || {
                info!("{} starting", waiter.name());
                waiter.wait(&event_bus);
                info!("{} finished", waiter.name());
            }).unwrap();

        return WaiterThread {
            handle,
            receiver
        };
    }
}
impl Producer for WaiterThread {
    fn channel(&self) -> &Receiver<Option<Event>> {
        return &self.receiver;
    }
}