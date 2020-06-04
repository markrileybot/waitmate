use std::borrow::Borrow;
use std::sync::Arc;

use config::{Config, FileFormat};
use crossbeam::channel::{Receiver, Select};

use crate::waitmate::api::{Event, Notifier, Waiter};
use crate::waitmate::log::EventLog;
use crate::waitmate::std::{SleepyWaiter, StdinWaiter, StdoutNotifier};
use crate::waitmate::thread::{NotifierThread, Producer, WaiterThread};

pub struct App {
    config: Config,
    event_log: Arc<EventLog>
}
impl App {
    pub fn new() -> App {
        let config_base = dirs::config_dir().unwrap();
        let config_file = config_base
            .join("waitmate.yaml");
        let event_log_dir = dirs::data_local_dir()
            .unwrap()
            .join("waitmate")
            .join("event_log.rdb");
        let mut config = Config::new();
        config
            .merge(config::File::from(config_file).required(false)).unwrap()
            .merge(config::File::new("waitmate", FileFormat::Yaml).required(false)).unwrap()
            .merge(config::Environment::with_prefix("WAITMATE"))
            .unwrap();
        let event_log = Arc::new(EventLog::new(event_log_dir.as_path()));

        return App {
            config,
            event_log
        }
    }

    pub fn run(&self) {
        let notifiers: Vec<Box<dyn Notifier>> = vec![Box::new(StdoutNotifier::new())];
        let waiters: Vec<Box<dyn Waiter>> = vec![Box::new(StdinWaiter::new()), Box::new(SleepyWaiter::new())];
        let event_log: &EventLog = &self.event_log;

        let mut receivers: Vec<&Receiver<Option<Event>>> = Vec::with_capacity(waiters.len() + notifiers.len());
        let mut selector = Select::new();
        let mut waiters_pending = waiters.len();

        let notifier_threads = notifiers
            .into_iter()
            .map(|n| NotifierThread::new(n, self.event_log.clone()))
            .collect::<Vec<_>>();
        let waiter_threads = waiters
            .into_iter()
            .map(|n| WaiterThread::new(n))
            .collect::<Vec<_>>();

        notifier_threads
            .iter()
            .map(|m| m.channel())
            .for_each(|r| receivers.push(r));
        waiter_threads
            .iter()
            .map(|m| m.channel())
            .for_each(|r| receivers.push(r));
        receivers
            .iter()
            .for_each(|r| {selector.recv(*r);});

        while waiters_pending > 0 {
            let op = selector.select();
            let index = op.index();
            let rec = receivers[index];
            match op.recv(rec) {
                Ok(e) => {
                    match e {
                        Some(event) => {
                            event_log.add(&event);
                            for x in &notifier_threads {
                                x.tickle();
                            }
                        }
                        None => {}
                    }
                }
                Err(_) => {
                    selector.remove(index);
                    waiters_pending -= 1;
                }
            }
        }
    }
}