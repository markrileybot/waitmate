use std::borrow::Borrow;
use std::sync::Arc;

use config::{Config, FileFormat};
use crossbeam::channel::{Receiver, Select};

use crate::waitmate::api::{Event, Notifier, Waiter};
use crate::waitmate::log::EventLog;
use crate::waitmate::net::{Client, Server};
use crate::waitmate::std::{SleepyWaiter, StdinWaiter, StdoutNotifier};
use crate::waitmate::thread::{NotifierThread, Producer, WaiterThread};
use std::path::PathBuf;
use std::process;


pub struct App {
    config: Config,
}
impl App {
    pub fn new() -> Self {
        return App::new_config(None);
    }
    pub fn new_config(config_file: Option<PathBuf>) -> Self {
        let config_base = dirs::config_dir().unwrap();
        let local_config = config_base
            .join("waitmate.yaml");
        let mut config = Config::new();
        config
            .merge(config::File::from(local_config).required(false)).unwrap()
            .merge(config::File::new("waitmate", FileFormat::Yaml).required(false)).unwrap();
        if config_file.is_some() {
            config.merge(config::File::from(config_file.unwrap()).required(false)).unwrap();
        }
        config.merge(config::Environment::with_prefix("WAITMATE")).unwrap();

        return App {
            config,
        }
    }
    pub fn dump_config(&self) {
        println!("{:?}", self.config);
    }
    pub fn dump(&self) {
        let event_log = App::create_event_log(false);
        event_log.dump(|cursor| {
            for (key, event) in cursor {
                println!("{} {}", key, event);
            }
        })
    }

    pub fn run_client(&self, address: &str) {
        let notifiers: Vec<Box<dyn Notifier>> = vec![Box::new(Client::new(address))];
        let waiters: Vec<Box<dyn Waiter>> = vec![Box::new(StdinWaiter::new()), Box::new(SleepyWaiter::new())];
        self._run(true, notifiers, waiters)
    }

    pub fn run_server(&self, address: &str) {
        let notifiers: Vec<Box<dyn Notifier>> = vec![Box::new(StdoutNotifier::new())];
        let waiters: Vec<Box<dyn Waiter>> = vec![Box::new(Server::new(address))];
        self._run(false, notifiers, waiters)
    }

    pub fn run(&self) {
        let notifiers: Vec<Box<dyn Notifier>> = vec![Box::new(StdoutNotifier::new())];
        let waiters: Vec<Box<dyn Waiter>> = vec![Box::new(StdinWaiter::new()), Box::new(SleepyWaiter::new())];



        self._run(false, notifiers, waiters)
    }

    fn _run(&self, temp: bool, notifiers: Vec<Box<dyn Notifier>>, waiters: Vec<Box<dyn Waiter>>) {
        let event_log = Arc::new(App::create_event_log(temp));
        let local_event_log: &EventLog = event_log.borrow();

        let mut receivers: Vec<&Receiver<Option<Event>>> = Vec::with_capacity(waiters.len() + notifiers.len());
        let mut selector = Select::new();
        let mut waiters_pending = waiters.len();

        let notifier_threads = notifiers
            .into_iter()
            .map(|n| NotifierThread::new(n, event_log.clone()))
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
            .for_each(|r| { selector.recv(*r); });

        while waiters_pending > 0 {
            let op = selector.select();
            let index = op.index();
            let rec = receivers[index];
            match op.recv(rec) {
                Ok(e) => {
                    match e {
                        Some(event) => {
                            local_event_log.add(&event);
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

    fn create_event_log(temp: bool) -> EventLog {
        let (base_dir, pid) = if temp {
            (dirs::runtime_dir().unwrap(), process::id())
        } else {
            (dirs::data_local_dir().unwrap(), 0)
        };
        let event_log_dir = base_dir
            .join("waitmate")
            .join(format!("event_log.{}.rdb", pid));
        return EventLog::new(event_log_dir.as_path());
    }
}