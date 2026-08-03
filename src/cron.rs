use crate::chain::Chain;
use crate::job::Job;
use crate::logger::{DiscardLogger, Logger};
use crate::parser::{ParseError, Parser};
use crate::schedule::Schedule;
use chrono::DateTime;
use chrono_tz::Tz;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, oneshot};

pub type EntryID = usize;

#[derive(Clone)]
pub struct Entry {
    pub id: EntryID,
    pub schedule: Arc<dyn Schedule>,
    pub next: Option<DateTime<Tz>>,
    pub prev: Option<DateTime<Tz>>,
    pub wrapped_job: Arc<dyn Job>,
    pub job: Arc<dyn Job>,
}

impl Entry {
    pub fn valid(&self) -> bool {
        self.id != 0
    }
}

pub enum OptionSetter {
    Location(Tz),
    Parser(Parser),
    Chain(Chain),
    Logger(Arc<dyn Logger>),
}

pub struct Cron {
    entries: Arc<Mutex<Vec<Entry>>>,
    removed_ids: Arc<Mutex<HashSet<EntryID>>>,
    chain: Chain,
    stop_tx: Mutex<Option<mpsc::Sender<()>>>,
    add_tx: Mutex<Option<mpsc::Sender<Entry>>>,
    remove_tx: Mutex<Option<mpsc::Sender<EntryID>>>,
    snapshot_tx: Mutex<Option<mpsc::Sender<oneshot::Sender<Vec<Entry>>>>>,
    running: Arc<AtomicBool>,
    logger: Arc<dyn Logger>,
    location: Tz,
    parser: Parser,
    next_id: AtomicUsize,
    job_count: Arc<AtomicUsize>,
    stop_waiters: Arc<Mutex<Vec<oneshot::Sender<()>>>>,
}

impl Cron {
    pub fn new() -> Self {
        let local_tz_str = iana_time_zone::get_timezone().unwrap_or_else(|_| "UTC".to_string());
        let location = local_tz_str.parse().unwrap_or(chrono_tz::Tz::UTC);
        Cron {
            entries: Arc::new(Mutex::new(Vec::new())),
            removed_ids: Arc::new(Mutex::new(HashSet::new())),
            chain: Chain::new(Vec::new()),
            stop_tx: Mutex::new(None),
            add_tx: Mutex::new(None),
            remove_tx: Mutex::new(None),
            snapshot_tx: Mutex::new(None),
            running: Arc::new(AtomicBool::new(false)),
            logger: Arc::new(DiscardLogger),
            location,
            parser: crate::parser::standard_parser(),
            next_id: AtomicUsize::new(0),
            job_count: Arc::new(AtomicUsize::new(0)),
            stop_waiters: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn with_options(options: Vec<OptionSetter>) -> Self {
        let mut c = Cron::new();
        for opt in options {
            match opt {
                OptionSetter::Location(loc) => c.location = loc,
                OptionSetter::Parser(p) => c.parser = p,
                OptionSetter::Chain(ch) => c.chain = ch,
                OptionSetter::Logger(l) => c.logger = l,
            }
        }
        c
    }

    pub fn location(&self) -> Tz {
        self.location
    }

    pub fn add_func<F>(&self, spec: &str, cmd: F) -> Result<EntryID, ParseError>
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.add_job(spec, Arc::new(cmd))
    }

    pub fn add_job(&self, spec: &str, cmd: Arc<dyn Job>) -> Result<EntryID, ParseError> {
        let schedule = self.parser.parse(spec)?;
        Ok(self.schedule(schedule, cmd))
    }

    pub fn schedule(&self, schedule: Arc<dyn Schedule>, cmd: Arc<dyn Job>) -> EntryID {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst) + 1;
        let entry = Entry {
            id,
            schedule,
            next: None,
            prev: None,
            wrapped_job: self.chain.then(cmd.clone()),
            job: cmd,
        };

        if self.running.load(Ordering::SeqCst) {
            if let Some(ref tx) = *self.add_tx.lock().unwrap() {
                let _ = tx.try_send(entry);
            }
        } else {
            self.entries.lock().unwrap().push(entry);
        }
        id
    }

    pub fn entries(&self) -> Vec<Entry> {
        self.entry_snapshot()
    }

    pub fn entry(&self, id: EntryID) -> Option<Entry> {
        self.entries().into_iter().find(|e| e.id == id)
    }

    pub fn remove(&self, id: EntryID) {
        self.remove_entry(id);
        if self.running.load(Ordering::SeqCst) {
            if let Some(ref tx) = *self.remove_tx.lock().unwrap() {
                let _ = tx.try_send(id);
            }
        }
    }

    fn entry_snapshot(&self) -> Vec<Entry> {
        self.entries.lock().unwrap().clone()
    }

    fn remove_entry(&self, id: EntryID) {
        self.removed_ids.lock().unwrap().insert(id);
        let mut entries = self.entries.lock().unwrap();
        entries.retain(|e| e.id != id);
    }

    pub fn start(&self) {
        if self.running.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
            return;
        }
        let (stop_tx, stop_rx) = mpsc::channel(1);
        let (add_tx, add_rx) = mpsc::channel(100);
        let (remove_tx, remove_rx) = mpsc::channel(100);
        let (snapshot_tx, snapshot_rx) = mpsc::channel(100);

        *self.stop_tx.lock().unwrap() = Some(stop_tx);
        *self.add_tx.lock().unwrap() = Some(add_tx);
        *self.remove_tx.lock().unwrap() = Some(remove_tx);
        *self.snapshot_tx.lock().unwrap() = Some(snapshot_tx);

        let entries = self.entries.clone();
        let removed_ids = self.removed_ids.clone();
        let logger = self.logger.clone();
        let loc = self.location;
        let job_count = self.job_count.clone();
        let stop_waiters = self.stop_waiters.clone();

        tokio::spawn(async move {
            run_loop(
                entries,
                removed_ids,
                logger,
                loc,
                stop_rx,
                add_rx,
                remove_rx,
                snapshot_rx,
                job_count,
                stop_waiters,
            )
            .await;
        });
    }

    pub fn run(&self) {
        if self.running.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
            return;
        }
        let (stop_tx, stop_rx) = mpsc::channel(1);
        let (add_tx, add_rx) = mpsc::channel(100);
        let (remove_tx, remove_rx) = mpsc::channel(100);
        let (snapshot_tx, snapshot_rx) = mpsc::channel(100);

        *self.stop_tx.lock().unwrap() = Some(stop_tx);
        *self.add_tx.lock().unwrap() = Some(add_tx);
        *self.remove_tx.lock().unwrap() = Some(remove_tx);
        *self.snapshot_tx.lock().unwrap() = Some(snapshot_tx);

        let entries = self.entries.clone();
        let removed_ids = self.removed_ids.clone();
        let logger = self.logger.clone();
        let loc = self.location;
        let job_count = self.job_count.clone();
        let stop_waiters = self.stop_waiters.clone();

        let handle = tokio::runtime::Handle::current();
        handle.block_on(async move {
            run_loop(
                entries,
                removed_ids,
                logger,
                loc,
                stop_rx,
                add_rx,
                remove_rx,
                snapshot_rx,
                job_count,
                stop_waiters,
            )
            .await;
        });
    }

    pub fn stop(&self) -> oneshot::Receiver<()> {
        let (tx, rx) = oneshot::channel();
        if self.running.compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
            if let Some(stop_tx) = self.stop_tx.lock().unwrap().take() {
                let _ = stop_tx.try_send(());
            }
        }
        if self.job_count.load(Ordering::SeqCst) == 0 {
            let _ = tx.send(());
        } else {
            self.stop_waiters.lock().unwrap().push(tx);
        }
        rx
    }
}

async fn run_loop(
    entries_lock: Arc<Mutex<Vec<Entry>>>,
    removed_ids: Arc<Mutex<HashSet<EntryID>>>,
    logger: Arc<dyn Logger>,
    loc: Tz,
    mut stop_rx: mpsc::Receiver<()>,
    mut add_rx: mpsc::Receiver<Entry>,
    mut remove_rx: mpsc::Receiver<EntryID>,
    mut snapshot_rx: mpsc::Receiver<oneshot::Sender<Vec<Entry>>>,
    job_count: Arc<AtomicUsize>,
    stop_waiters: Arc<Mutex<Vec<oneshot::Sender<()>>>>,
) {
    logger.info("start", &[]);

    let now = chrono::Utc::now().with_timezone(&loc);
    {
        let mut entries = entries_lock.lock().unwrap();
        for entry in entries.iter_mut() {
            entry.next = entry.schedule.next(now);
            if let Some(next) = entry.next {
                logger.info(
                    "schedule",
                    &[
                        ("now", &now.to_rfc3339()),
                        ("entry", &entry.id),
                        ("next", &next.to_rfc3339()),
                    ],
                );
            }
        }
    }

    loop {
        {
            let mut entries = entries_lock.lock().unwrap();
            entries.sort_by(|a, b| match (a.next, b.next) {
                (None, None) => std::cmp::Ordering::Equal,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (Some(_), None) => std::cmp::Ordering::Less,
                (Some(t1), Some(t2)) => t1.cmp(&t2),
            });
        }

        let next_time = {
            let entries = entries_lock.lock().unwrap();
            entries.first().and_then(|e| e.next)
        };

        let sleep_duration = match next_time {
            Some(nt) => {
                let cur = chrono::Utc::now().with_timezone(&loc);
                if nt > cur {
                    (nt - cur).to_std().unwrap_or(std::time::Duration::from_millis(1))
                } else {
                    std::time::Duration::from_millis(0)
                }
            }
            None => std::time::Duration::from_secs(100_000 * 3600),
        };

        tokio::select! {
            _ = tokio::time::sleep(sleep_duration) => {
                let now = chrono::Utc::now().with_timezone(&loc);
                logger.info("wake", &[("now", &now.to_rfc3339())]);

                let mut entries = entries_lock.lock().unwrap();
                for e in entries.iter_mut() {
                    match e.next {
                        Some(nt) if nt <= now => {
                            let job = e.wrapped_job.clone();
                            let jc = job_count.clone();
                            let sw = stop_waiters.clone();
                            jc.fetch_add(1, Ordering::SeqCst);
                            tokio::spawn(async move {
                                job.run();
                                if jc.fetch_sub(1, Ordering::SeqCst) == 1 {
                                    let mut waiters = sw.lock().unwrap();
                                    for w in waiters.drain(..) {
                                        let _ = w.send(());
                                    }
                                }
                            });
                            e.prev = Some(nt);
                            e.next = e.schedule.next(now);
                            if let Some(next) = e.next {
                                logger.info("run", &[("now", &now.to_rfc3339()), ("entry", &e.id), ("next", &next.to_rfc3339())]);
                            }
                        }
                        _ => {}
                    }
                }
            }
            Some(new_entry) = add_rx.recv() => {
                if removed_ids.lock().unwrap().contains(&new_entry.id) {
                    continue;
                }
                let now = chrono::Utc::now().with_timezone(&loc);
                let mut entry = new_entry;
                entry.next = entry.schedule.next(now);
                if let Some(next) = entry.next {
                    logger.info("added", &[("now", &now.to_rfc3339()), ("entry", &entry.id), ("next", &next.to_rfc3339())]);
                }
                entries_lock.lock().unwrap().push(entry);
            }
            Some(id) = remove_rx.recv() => {
                removed_ids.lock().unwrap().insert(id);
                let mut entries = entries_lock.lock().unwrap();
                entries.retain(|e| e.id != id);
                logger.info("removed", &[("entry", &id)]);
            }
            Some(reply_tx) = snapshot_rx.recv() => {
                let snapshot = entries_lock.lock().unwrap().clone();
                let _ = reply_tx.send(snapshot);
            }
            _ = stop_rx.recv() => {
                logger.info("stop", &[]);
                return;
            }
        }
    }
}
