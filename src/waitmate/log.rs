use std::path::Path;
use std::str;

use rocksdb::{DB, Options, ReadOptions};

use crate::waitmate::api::Event;

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
    pub fn tail<TailCallback, ContinueCallback>(&self, name: &str,
                                            continue_func: ContinueCallback,
                                            tail_func: TailCallback)
        where
            ContinueCallback: Fn() -> bool,
            TailCallback: Fn(&str, Event) {

        let mut run = true;
        let off_key = name.as_bytes();
        let log_cf = self.db.cf_handle("log").unwrap();
        let off_cf = self.db.cf_handle("offsets").unwrap();

        let mut start_key: Option<Vec<u8>> = self.db.get_pinned_cf(off_cf, off_key)
            .unwrap()
            .map_or(None, |k| Some(k.to_vec()));

        let mut opts = ReadOptions::default();
        opts.set_tailing(true);
        let mut iter = self.db.raw_iterator_cf_opt(log_cf, opts);

        while run {
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

            while iter.valid() {
                let key = iter.key().unwrap();
                let value = iter.value().unwrap();

                tail_func(str::from_utf8(key).unwrap(),
                          serde_json::from_slice(value).unwrap());

                self.db.put_cf(off_cf, off_key, key).unwrap();

                start_key = Some(key.to_vec());
                iter.next();
            }
            run = continue_func();
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