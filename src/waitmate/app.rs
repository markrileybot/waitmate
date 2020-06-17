use std::borrow::Borrow;
use std::path::PathBuf;
use std::process;
use std::sync::Arc;
use std::thread;

use config::{Config, FileFormat};
use crossbeam::channel::{Receiver, Select, unbounded};
use log::info;
use signal_hook::{iterator::Signals, SIGINT, SIGTERM, SIGQUIT, SIGHUP};

use crate::waitmate::api::{Event, Notifier, Waiter};
use crate::waitmate::http::Server as HttpServer;
use crate::waitmate::log::EventLog;
use crate::waitmate::net::{Client, Server};
use crate::waitmate::std::{SleepyWaiter, StdinWaiter, StdoutNotifier};
use crate::waitmate::thread::{NotifierThread, Producer, WaiterThread};

pub struct App {
    config: Config,
    event_log: Arc<EventLog>
}
impl App {
    pub fn new(temp: bool) -> Self {
        return App::new_config(temp, None);
    }
    pub fn new_config(temp: bool, config_file: Option<PathBuf>) -> Self {
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

        let event_log = Arc::new(App::create_event_log(temp));
        return App {
            config,
            event_log
        }
    }
    pub fn dump_config(&self) {
        println!("{:?}", self.config);
    }
    pub fn dump(&self) {
        let event_log = App::create_event_log(false);
        let cursor = event_log.build_cursor().build();
        for (key, event) in cursor {
            println!("{} {}", key, event);
        }
    }

    pub fn run_client(&self, address: &str) {
        let notifiers: Vec<Box<dyn Notifier>> = vec![Box::new(Client::new(address))];
        let waiters: Vec<Box<dyn Waiter>> = vec![Box::new(StdinWaiter::new()), Box::new(SleepyWaiter::new())];
        self._run(notifiers, waiters)
    }

    pub fn run_server(&self, address: &str) {
        let notifiers: Vec<Box<dyn Notifier>> = vec![Box::new(StdoutNotifier::new())];
        let waiters: Vec<Box<dyn Waiter>> = vec![Box::new(Server::new(address)),
                                                 Box::new(HttpServer::new("0.0.0.0:12346",
                                                                          self.event_log.clone()))];
        self._run(notifiers, waiters)
    }

    pub fn run(&self) {
        let notifiers: Vec<Box<dyn Notifier>> = vec![Box::new(StdoutNotifier::new())];
        let waiters: Vec<Box<dyn Waiter>> = vec![Box::new(StdinWaiter::new()), Box::new(SleepyWaiter::new())];
        self._run(notifiers, waiters)
    }

    fn _run(&self, notifiers: Vec<Box<dyn Notifier>>, waiters: Vec<Box<dyn Waiter>>) {
        let local_event_log: &EventLog = self.event_log.borrow();

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

        let (sig_tx, sig_rx) = unbounded();
        let sig_id = selector.recv(&sig_rx);
        std::thread::spawn(move || {
            let signals = Signals::new(&[SIGINT, SIGTERM, SIGQUIT, SIGHUP]).unwrap();
            for sig in signals.forever() {
                sig_tx.send(sig).unwrap_or(());
            }
        });

        receivers
            .iter()
            .for_each(|r| { selector.recv(*r); });

        while waiters_pending > 0 {
            info!("{} Waiters pending", waiters_pending);
            let op = selector.select();
            let index = op.index();

            if index == sig_id {
                let sig = op.recv(&sig_rx).unwrap_or(SIGHUP);
                if sig == SIGHUP {
                    continue;
                }
                break;
            } else {
                let rec = receivers[index - 1];
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

        info!("Exiting");
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