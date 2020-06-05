use std::borrow::BorrowMut;
use std::path::Path;
use std::str;

use rocksdb::{ColumnFamily, DB, DBRawIterator, Options, ReadOptions};

use crate::waitmate::api::Event;

pub struct Cursor<'a, 'b> {
    position: Option<Vec<u8>>,
    iter: &'a mut DBRawIterator<'b>,
    db: &'a DB,
    off_cf: Option<&'a ColumnFamily>,
    off_key: Option<&'a [u8]>,
    seek_needed: bool
}
impl Iterator for Cursor<'_, '_> {
    type Item = (String, Event);
    fn next(&mut self) -> Option<(String, Event)> {
        let mut ret = None;
        if self.iter.valid() {
            if self.seek_needed {
                self.iter.next();
            }
            self.seek_needed = true;

            if self.iter.valid() {
                let key = self.iter.key().unwrap();
                let value = self.iter.value().unwrap();
                self.position = Some(key.to_vec());

                ret = Some((
                    String::from(str::from_utf8(key).unwrap()),
                    serde_json::from_slice(value).unwrap()
                ));

                match self.off_cf {
                    Some(off_cf) => {
                        match self.off_key {
                            Some(off_key) => {
                                self.db.put_cf(off_cf, off_key, key).unwrap();
                            },
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }
        return ret;
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
    pub fn add(&self, event: &Event) {
        let key = format!("{}|{}", event.time, event.id);
        let val = serde_json::to_vec(event).unwrap();
        let cf = self.db.cf_handle("log").unwrap();
        self.db.put_cf(cf, key.as_bytes(), val).unwrap();
    }
    pub fn dump<DumpCallback>(&self, dump_func: DumpCallback)
        where
            DumpCallback: Fn(&mut Cursor) {
        self.tail("", |c| {
            dump_func(c);
            return false;
        });
    }
    pub fn tail<TailCallback>(&self, name: &str,
                              tail_func: TailCallback)
        where
            TailCallback: Fn(&mut Cursor) -> bool {


        let mut off_cf_opt: Option<&ColumnFamily> = None;
        let mut start_key: Option<Vec<u8>> = None;
        let off_key = name.as_bytes();
        let log_cf = self.db.cf_handle("log").unwrap();

        if !name.is_empty() {
            let off_cf = self.db.cf_handle("offsets").unwrap();
            start_key = self.db.get_pinned_cf(off_cf, off_key)
                .unwrap()
                .map_or(None, |k| Some(k.to_vec()));
            off_cf_opt = Some(off_cf);
        }

        let mut opts = ReadOptions::default();
        opts.set_tailing(true);
        let mut iter = self.db.raw_iterator_cf_opt(log_cf, opts);

        loop {
            match start_key.as_ref() {
                Some(k) => {
                    iter.seek(k);
                    if iter.valid() {
                        iter.next();
                    }
                }
                _ => {
                    iter.seek_to_first();
                }
            }

            let mut cursor = Cursor {
                db: &self.db,
                position: None,
                iter: iter.borrow_mut(),
                off_key: Some(&off_key),
                off_cf: off_cf_opt,
                seek_needed: false
            };
            if tail_func(cursor.borrow_mut()) {
                match cursor.position {
                    Some(p) => {
                        start_key = Some(p)
                    }
                    _ => {}
                }
            } else {
                break;
            }
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