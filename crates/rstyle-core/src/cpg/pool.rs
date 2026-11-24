//! Derive a run's documents on several threads while one thread receives them. Completion
//! order can't show in the result: any failure rejects the run, and records leave in
//! canonical order or in the order their documents arrived.
use super::{
    layout::{self, Parsed},
    model::Document,
    records::{self, Demand, Record},
    store::facts::{FunctionFacts, KeptDocument},
};
use serde_json::Value;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc::{Receiver, SyncSender, sync_channel},
};

/// How a pool works. `on_failure` runs once, on the first failure, so a caller can stop
/// whatever the receiving thread is blocked on, such as a frontend writing to a pipe.
pub struct Pool<'a> {
    pub workers: usize,
    pub demand: Demand,
    pub on_failure: &'a (dyn Fn() + Sync),
    /// Shown each bundled document, on a worker, for what a run's records don't hold. A
    /// document `facts` keeps whole is never parsed, so isn't shown.
    pub on_document: &'a (dyn Fn(&Document) + Sync),
    /// Given each document's records on the worker that derived them, before the pool keeps
    /// what it returns, to take what needn't wait for the rest of the run.
    pub on_records: OnRecords<'a>,
    pub order: Order,
    /// Where each function's records are kept between runs, if anywhere: a function the store
    /// keeps isn't derived again. Its demand must be the pool's.
    pub facts: Option<&'a FunctionFacts>,
}

/// The order records leave a pool in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Order {
    /// By unit and function, whatever order the documents came in.
    Canonical,
    /// As one thread reading the documents in turn would leave them: each document's records
    /// in their place, and split units', merged at the end, after them.
    Arrival,
}

/// A job's document as `load` reads it.
pub enum Loaded {
    Parsed(Parsed),
    /// Not parsed yet: the sha256 its bytes were stored under, and how to parse them. A document
    /// the pool's store keeps whole under that digest is never parsed.
    Stored {
        digest: String,
        parse: Box<dyn FnOnce() -> Result<Parsed, String>>,
    },
}

impl From<Parsed> for Loaded {
    fn from(parsed: Parsed) -> Self {
        Loaded::Parsed(parsed)
    }
}

/// What a pool hands each document's records to. Its error fails the run.
pub type OnRecords<'a> = &'a (dyn Fn(Vec<Record>) -> Result<Vec<Record>, String> + Sync);

/// Keeps every record whole.
pub fn keep_records(records: Vec<Record>) -> Result<Vec<Record>, String> {
    Ok(records)
}

/// Derives every job's document. `load` reads a job, numbered in arrival order, into its
/// document on a worker thread. At most `workers` jobs wait in the queue, so a producer
/// faster than the workers holds a bounded number of documents in memory. Split documents
/// are retained until every job has arrived, since they can arrive in any order.
pub fn pooled_records<J: Send, L: Into<Loaded>>(
    jobs: impl IntoIterator<Item = Result<J, String>>,
    load: impl Fn(usize, J) -> Result<L, String> + Sync,
    pool: &Pool<'_>,
) -> Result<Vec<Record>, String> {
    if let Some(facts) = pool.facts.filter(|facts| facts.demand() != pool.demand) {
        return Err(format!(
            "the store's facts are for {:?}, not the run's {:?}",
            facts.demand(),
            pool.demand
        ));
    }
    let workers = pool.workers.max(1);
    let state = State::new(pool);
    let (sender, receiver) = sync_channel(workers);
    let queue = Arc::new(Mutex::new(receiver));

    std::thread::scope(|scope| {
        for _ in 0..workers {
            let queue = Arc::clone(&queue);
            let (state, load) = (&state, &load);
            scope.spawn(move || work(queue, state, load));
        }
        // With every worker gone, even by a panic, the send below fails instead of blocking.
        drop(queue);
        produce(jobs, sender, &state);
    });
    state.finish(pool.order)
}

fn produce<J>(
    jobs: impl IntoIterator<Item = Result<J, String>>,
    sender: SyncSender<(usize, J)>,
    state: &State<'_>,
) {
    for (index, job) in jobs.into_iter().enumerate() {
        if state.failed() {
            return;
        }
        let job = match job {
            Ok(job) => job,
            Err(error) => return state.fail(error),
        };
        if sender.send((index, job)).is_err() {
            return;
        }
    }
}

fn work<J, L: Into<Loaded>>(
    queue: Arc<Mutex<Receiver<(usize, J)>>>,
    state: &State<'_>,
    load: &impl Fn(usize, J) -> Result<L, String>,
) {
    while let Some((index, job)) = next(&queue) {
        // After a failure, keep draining so the producer never waits on a full queue.
        if state.failed() {
            continue;
        }
        let loaded = load(index, job);
        if let Err(error) = loaded.and_then(|loaded| state.derive(index, loaded.into())) {
            state.fail(error);
        }
    }
}

fn next<J>(queue: &Mutex<Receiver<J>>) -> Option<J> {
    queue.lock().ok()?.recv().ok()
}

struct State<'a> {
    demand: Demand,
    on_failure: &'a (dyn Fn() + Sync),
    on_document: &'a (dyn Fn(&Document) + Sync),
    on_records: OnRecords<'a>,
    facts: Option<&'a FunctionFacts>,
    failed: AtomicBool,
    failure: Mutex<Option<String>>,
    functions: Mutex<layout::FunctionIds>,
    split: Mutex<Vec<(usize, Value)>>,
    /// Each document's records, by its place in arrival order.
    records: Mutex<Vec<(usize, Vec<Record>)>>,
}

impl<'a> State<'a> {
    fn new(pool: &Pool<'a>) -> Self {
        Self {
            demand: pool.demand,
            on_failure: pool.on_failure,
            on_document: pool.on_document,
            on_records: pool.on_records,
            facts: pool.facts,
            failed: AtomicBool::new(false),
            failure: Mutex::default(),
            functions: Mutex::default(),
            split: Mutex::default(),
            records: Mutex::default(),
        }
    }

    fn failed(&self) -> bool {
        self.failed.load(Ordering::Acquire)
    }

    /// The first failure is the one reported. Stopping the input can cause others.
    fn fail(&self, error: String) {
        let mut failure = self.failure.lock().expect("no panic while recording");
        if failure.is_none() {
            *failure = Some(error);
            self.failed.store(true, Ordering::Release);
            drop(failure);
            (self.on_failure)();
        }
    }

    fn derive(&self, index: usize, loaded: Loaded) -> Result<(), String> {
        let (document, digest) = match loaded {
            Loaded::Parsed(document) => (document, None),
            Loaded::Stored { digest, parse } => {
                if let Some(kept) = self.facts.and_then(|facts| facts.kept_document(&digest)) {
                    return self.kept(index, kept);
                }
                (parse()?, Some(digest))
            }
        };
        match document {
            Parsed::Bundled(document) => self.bundled(index, &document, digest.as_deref()),
            Parsed::Split(document) => {
                self.split
                    .lock()
                    .expect("no panic while retaining")
                    .push((index, document));
                Ok(())
            }
        }
    }

    // rstyle-allow(side_effect_density): IO boundary: a store's facts are read and kept here
    /// Derives a bundled document's records. With the `digest` its bytes were stored under, the
    /// store also keeps them whole, for a later run to read without parsing the document.
    fn bundled(
        &self,
        index: usize,
        document: &Document,
        digest: Option<&str>,
    ) -> Result<(), String> {
        (self.on_document)(document);

        self.functions
            .lock()
            .expect("no panic while including")
            .include(document)?;
        let records = match self.facts {
            Some(facts) => facts.records(document, digest)?,
            None => records::records(document, self.demand)?,
        };
        self.extend(index, records)
    }

    /// The records of a document the store kept whole, as deriving it again would give them.
    fn kept(&self, index: usize, kept: KeptDocument) -> Result<(), String> {
        let functions = kept.records.iter().map(|record| record.function.as_str());
        self.functions
            .lock()
            .expect("no panic while including")
            .include_ids(&kept.unit, functions)?;
        self.extend(index, kept.records)
    }

    fn extend(&self, index: usize, records: Vec<Record>) -> Result<(), String> {
        let batch = (self.on_records)(records)?;
        self.records
            .lock()
            .expect("no panic while extending")
            .push((index, batch));
        Ok(())
    }

    fn finish(self, order: Order) -> Result<Vec<Record>, String> {
        if let Some(failure) = self.failure.lock().expect("no panic").take() {
            return Err(failure);
        }
        let mut split = std::mem::take(&mut *self.split.lock().expect("no panic"));
        split.sort_by_key(|(index, _)| *index);
        let split = split.into_iter().map(|(_, document)| document).collect();

        // After every document that arrived, in the order their groups are merged.
        for group in layout::split_runs(split) {
            self.bundled(usize::MAX, &layout::bundle(group)?, None)?;
        }
        let mut batches = self.records.into_inner().expect("no panic");
        batches.sort_by_key(|(index, _)| *index);
        let mut records: Vec<Record> = batches.into_iter().flat_map(|(_, batch)| batch).collect();
        if order == Order::Canonical {
            records::canonical_order(&mut records);
        }
        Ok(records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpg::read_records;
    use std::sync::atomic::AtomicUsize;

    fn example() -> Value {
        serde_json::from_str(include_str!("testdata/example.cpg.json")).unwrap()
    }

    fn no_failure() {}

    fn pool(workers: usize, demand: Demand) -> Pool<'static> {
        Pool {
            workers,
            demand,
            on_failure: &no_failure,
            on_document: &|_| {},
            on_records: &keep_records,
            order: Order::Canonical,
            facts: None,
        }
    }

    fn pooled(documents: Vec<Value>, workers: usize) -> Result<Vec<Record>, String> {
        let demand = Demand {
            graphs: true,
            bounds: true,
            flow: true,
            defects: false,
        };
        pooled_records(
            documents.into_iter().map(Ok),
            |_, document| Parsed::from_json(&document),
            &pool(workers, demand),
        )
    }

    fn functions(records: &[Record]) -> Vec<String> {
        records
            .iter()
            .map(|record| {
                format!(
                    "{:?}",
                    (&record.compilation, &record.identity, &record.function)
                )
            })
            .collect()
    }

    fn other_unit() -> Value {
        let mut document = example();
        document["unit"]["id"] = "unit:other".into();
        document
    }

    #[test]
    fn any_worker_count_or_order_gives_the_same_records() {
        let expected = functions(&pooled(vec![example(), other_unit()], 1).unwrap());
        assert!(!expected.is_empty());
        for workers in [1, 2, 8] {
            let records = pooled(vec![other_unit(), example()], workers).unwrap();
            assert_eq!(functions(&records), expected);
        }
    }

    #[test]
    fn arrival_order_is_the_order_one_thread_reads_them_in() {
        let mut third = example();
        third["unit"]["id"] = "unit:third".into();
        let mut documents = vec![other_unit()];
        documents.extend(layout::split(&example()));
        documents.push(third);
        let serial = read_records(documents.iter().map(Parsed::from_json), Demand::default());
        let serial = functions(&serial.unwrap());
        for workers in [1, 4] {
            let pool = Pool {
                order: Order::Arrival,
                ..pool(workers, Demand::default())
            };
            let jobs = documents.iter().map(Ok);
            let arrived = pooled_records(jobs, |_, d| Parsed::from_json(d), &pool).unwrap();
            assert_eq!(functions(&arrived), serial);
        }
        let jobs = documents.iter().map(Ok);
        let pool = pool(4, Demand::default());
        let canonical = pooled_records(jobs, |_, d| Parsed::from_json(d), &pool).unwrap();
        assert_ne!(functions(&canonical), serial);
    }

    #[test]
    fn it_derives_what_the_serial_reader_derives() {
        let demand = Demand::default();
        let serial = read_records([Parsed::from_json(&example())], demand).unwrap();
        let mut sorted = serial.clone();
        records::canonical_order(&mut sorted);
        let parallel = pooled_records(
            [Ok(example())],
            |_, d| Parsed::from_json(&d),
            &pool(4, demand),
        );
        assert_eq!(functions(&parallel.unwrap()), functions(&sorted));
    }

    #[test]
    fn split_documents_wait_for_the_end_of_the_run() {
        let split = layout::split(&example());
        let mut reversed = split.clone();
        reversed.reverse();
        let expected = functions(&pooled(vec![example()], 1).unwrap());
        assert_eq!(functions(&pooled(split, 4).unwrap()), expected);
        assert_eq!(functions(&pooled(reversed, 4).unwrap()), expected);
    }

    #[test]
    fn a_function_seen_twice_rejects_the_run_across_workers() {
        let error = pooled(vec![example(), example()], 4).unwrap_err();
        assert!(error.contains("appears more than once"), "{error}");
    }

    #[test]
    fn a_failed_job_stops_the_input_once_and_is_the_reported_error() {
        let stops = AtomicUsize::new(0);
        let stop = || {
            stops.fetch_add(1, Ordering::SeqCst);
        };
        let pool = Pool {
            workers: 4,
            demand: Demand::default(),
            on_failure: &stop,
            on_document: &|_| {},
            on_records: &keep_records,
            order: Order::Canonical,
            facts: None,
        };
        let jobs = (0..100).map(|index| match index {
            3 => Err("broken pipe".to_string()),
            _ => Ok(index),
        });
        let error = pooled_records(jobs, |_, _| Parsed::from_json(&example()), &pool).unwrap_err();
        assert!(
            error == "broken pipe" || error.contains("appears more than once"),
            "{error}"
        );
        assert_eq!(stops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn a_worker_failure_ends_a_producer_with_more_to_send() {
        let produced = AtomicUsize::new(0);
        let jobs = (0..10_000).map(|index| {
            produced.fetch_add(1, Ordering::SeqCst);
            Ok(index)
        });
        let error = pooled_records(
            jobs,
            |_, _| Err::<Parsed, _>("worker failed".to_string()),
            &pool(2, Demand::default()),
        )
        .unwrap_err();
        assert_eq!(error, "worker failed");
        assert!(produced.load(Ordering::SeqCst) < 10_000);
    }
}
