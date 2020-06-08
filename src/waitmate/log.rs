use std::path::Path;
use std::str;

use rocksdb::{ColumnFamily, DB, DBRawIterator, Options, ReadOptions};
use uuid::Uuid;

use crate::waitmate::api::Event;
use crate::waitmate::log::SeekOp::{Next, Start};
use crossbeam::channel::Receiver;

#[derive(PartialEq, Eq)]
enum SeekOp {
    Start,
    Next
}

pub struct Cursor<'a> {
    position: Option<Vec<u8>>,
    iter: DBRawIterator<'a>,
    db: &'a DB,
    off_cf: Option<&'a ColumnFamily>,
    off_key: Option<Vec<u8>>,
    seek_op: SeekOp,
    tail_block: Option<Receiver<bool>>
}
impl<'a> Cursor<'a> {
    fn start(&mut self) {
        if self.seek_op == Start {
            match self.position.as_ref() {
                Some(k) => {
                    self.iter.seek(k);
                    if self.iter.valid() {
                        self.iter.next();
                    }
                }
                _ => {
                    self.iter.seek_to_first();
                }
            }
        }
    }

    fn next(&mut self) {
        if self.seek_op == Next {
            self.iter.next();
        }
        self.seek_op = Next;
    }
}
impl<'a> Iterator for Cursor<'a> {
    type Item = (String, Event);
    fn next(&mut self) -> Option<(String, Event)> {
        let mut ret = None;
        loop {
            self.start();

            if self.iter.valid() {
                self.next();

                if self.iter.valid() {
                    let key = self.iter.key().unwrap();
                    let value = self.iter.value().unwrap();

                    ret = Some((
                        String::from(str::from_utf8(key).unwrap()),
                        serde_json::from_slice(value).unwrap()
                    ));

                    if self.tail_block.is_some() {
                        self.position = Some(key.to_vec());
                    }
                    if self.off_cf.is_some() {
                        let off = self.off_cf.unwrap();
                        let off_key = self.off_key.as_ref().unwrap();
                        self.db.put_cf(off, off_key, key).unwrap();
                    }
                }
            }

            if !ret.is_some() && self.tail_block.is_some() {
                let block = self.tail_block.as_ref().unwrap();
                if block.recv().unwrap_or(false) {
                    self.seek_op = Start;
                    continue;
                }
            }

            break;
        }
        return ret;
    }
}

pub struct CursorBuilder<'a> {
    start: Option<Vec<u8>>,
    tail_block: Option<Receiver<bool>>,
    name: String,
    start_time: Option<u128>,
    start_id: Option<Uuid>,
    tailing: bool,
    db: &'a DB,
}
impl<'a> CursorBuilder<'a> {
    pub fn tailing(mut self, tail_block: Option<Receiver<bool>>) -> Self {
        self.tail_block = tail_block;
        self.tailing = true;
        return self;
    }
    pub fn named(mut self, name: &str) -> Self {
        self.name = String::from(name);
        return self;
    }
    pub fn starting_after(mut self, time: u128, id: Option<Uuid>) -> Self {
        self.start_time = Some(time);
        self.start_id = id;
        return self;
    }
    pub fn build(mut self) -> Cursor<'a> {
        let mut off_cf_opt: Option<&ColumnFamily> = None;
        let mut start_key: Option<Vec<u8>> = None;
        let off_key = self.name.as_bytes();
        let log_cf = self.db.cf_handle("log").unwrap();

        if !self.name.is_empty() {
            let off_cf = self.db.cf_handle("offsets").unwrap();
            start_key = self.db.get_pinned_cf(off_cf, off_key)
                .unwrap()
                .map_or(None, |k| Some(k.to_vec()));
            off_cf_opt = Some(off_cf);
        } else if self.start.is_some() {
            start_key = self.start.take();
        } else if self.start_time.is_some() {
            if self.start_id.is_some() {
                start_key = Some(EventLog::create_key(&self.start_time.unwrap(),
                                                      &self.start_id.unwrap()));
            } else {
                start_key = Some(EventLog::create_key_after(&self.start_time.unwrap()));
            }
        }

        let mut opts = ReadOptions::default();
        if self.tailing {
            opts.set_tailing(true);
        }

        let iter = self.db.raw_iterator_cf_opt(log_cf, opts);

        return Cursor {
            position: start_key,
            iter,
            db: self.db,
            off_cf: off_cf_opt,
            off_key: Some(off_key.to_vec()),
            seek_op: SeekOp::Start,
            tail_block: self.tail_block
        };
    }
}


pub struct EventLog {
    db: DB,
    path: String
}
impl EventLog {
    pub fn new(path: &Path) -> EventLog {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        let mut db;

        if path.exists() {
            let cf_names = vec!["offsets", "log"];
            db = DB::open_cf(&opts, &path, cf_names).unwrap();
        } else {
            db = DB::open(&opts, &path).unwrap();
            db.create_cf("offsets", &opts).unwrap();
            db.create_cf("log", &opts).unwrap();
        }

        return EventLog {
            db,
            path: String::from(path.to_str().unwrap())
        }
    }
    fn create_key_after(time: &u128) -> Vec<u8> {
        let uuid = Uuid::from_u128(0);
        return Self::create_key(time, &uuid);
    }
    fn create_key(time: &u128, id: &Uuid) -> Vec<u8> {
        let key = format!("{}|{}", time, id);
        return key.into_bytes();
    }
    pub fn add(&self, event: &Event) {
        let key = Self::create_key(&event.time, &event.id);
        let val = serde_json::to_vec(event).unwrap();
        let cf = self.db.cf_handle("log").unwrap();
        self.db.put_cf(cf, key, val).unwrap();
    }
    pub fn get(&self, time: &u128, id: &Uuid) -> Option<Event> {
        let key = format!("{}|{}", time, id);
        let cf = self.db.cf_handle("log").unwrap();
        return self.db.get_pinned_cf(cf, key.as_bytes())
            .map_or(None, |d| {
                return d.map_or(None, |d|  {
                    let e: Event = serde_json::from_slice(d.as_ref()).unwrap();
                    return Some(e);
                });
            });
    }
    pub fn build_cursor(&self) -> CursorBuilder {
        return CursorBuilder {
            start: None,
            tail_block: None,
            name: "".to_string(),
            start_time: None,
            start_id: None,
            tailing: false,
            db: &self.db
        }
    }
    fn close(&self) {
        let _ = DB::destroy(&Options::default(), &self.path);
    }
}
impl Drop for EventLog {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use tempfile::tempdir;
    use uuid::Uuid;

    use crossbeam::channel::unbounded;
    use crate::waitmate::api::{EmptyNamed, Event, Level};
    use crate::waitmate::log::EventLog;
    use std::borrow::Borrow;

    #[test]
    fn test_log_add_get_iter() {
        let dir = tempdir().unwrap().into_path().join("t.rdb");
        let event_log = EventLog::new(dir.as_path());
        let source = EmptyNamed {};

        let e1 = Event::new(&source, "a", "b", "c", Level::WARN);
        std::thread::sleep(Duration::from_millis(10));
        let e2 = Event::new(&source, "a", "b", "c", Level::WARN);
        event_log.add(&e1);
        event_log.add(&e2);

        assert_eq!(None, event_log.get(&0, &Uuid::new_v4()));
        assert_eq!(e1, event_log.get(&e1.time, &e1.id).unwrap());
        assert_eq!(e2, event_log.get(&e2.time, &e2.id).unwrap());

        let mut count = 0;
        let cursor = event_log.build_cursor()
            .starting_after(e1.time, None)
            .build();
        for (key, event) in cursor {
            println!("{:?} {:?}", key, event);
            count+=1;
        }
        assert_eq!(1, count);

        count = 0;
        let cursor = event_log.build_cursor()
            .build();
        for (key, event) in cursor {
            println!("{:?} {:?}", key, event);
            count+=1;
        }
        assert_eq!(2, count);
    }

    #[test]
    fn test_named_iter() {
        let dir = tempdir().unwrap().into_path().join("t.rdb");
        let event_log = EventLog::new(dir.as_path());
        let source = EmptyNamed {};

        let e1 = Event::new(&source, "a", "b", "c", Level::WARN);
        std::thread::sleep(Duration::from_millis(10));
        let e2 = Event::new(&source, "a", "b", "c", Level::WARN);
        std::thread::sleep(Duration::from_millis(10));
        let e3 = Event::new(&source, "a", "b", "c", Level::WARN);
        event_log.add(&e1);
        event_log.add(&e2);

        let mut count = 0;
        let cursor = event_log.build_cursor()
            .named("markie")
            .build();
        for (key, event) in cursor {
            println!("{:?} {:?}", key, event);
            count+=1;
        }
        assert_eq!(2, count);

        count = 0;
        let cursor = event_log.build_cursor()
            .named("markie")
            .build();
        for (key, event) in cursor {
            println!("{:?} {:?}", key, event);
            count+=1;
        }
        assert_eq!(0, count);

        count = 0;
        event_log.add(&e3);
        let cursor = event_log.build_cursor()
            .named("markie")
            .build();
        for (key, event) in cursor {
            println!("{:?} {:?}", key, event);
            count+=1;
        }
        assert_eq!(1, count);
    }

    #[test]
    fn test_threaded_tail() {
        let dir = tempdir().unwrap().into_path().join("t.rdb");
        let event_log = Arc::new(EventLog::new(dir.as_path()));
        let (tx, rx) = unbounded();
        let source = EmptyNamed {};

        let t_event_log = event_log.clone();
        std::thread::spawn(move || {
            let event_log: &EventLog = t_event_log.borrow();

            std::thread::sleep(Duration::from_millis(100));
            let e1 = Event::new(&source, "a", "b", "c", Level::WARN);
            event_log.add(&e1);
            tx.send(true).unwrap();

            std::thread::sleep(Duration::from_millis(100));
            let e2 = Event::new(&source, "a", "b", "c", Level::WARN);
            event_log.add(&e2);
            tx.send(true).unwrap();

            std::thread::sleep(Duration::from_millis(100));
            let e3 = Event::new(&source, "a", "b", "c", Level::WARN);
            event_log.add(&e3);
            tx.send(true).unwrap();
            tx.send(false).unwrap();
        });

        let mut count = 0;
        let cursor = event_log.build_cursor()
            .tailing(Some(rx))
            .build();
        for (key, event) in cursor {
            println!("{:?} {:?}", key, event);
            count+=1;
        }
        assert_eq!(3, count);
    }
}
