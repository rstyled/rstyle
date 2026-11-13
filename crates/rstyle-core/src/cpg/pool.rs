//! Derive a run's documents on several threads while one thread receives them. Neither
//! arrival nor completion order can show in the result: any failure rejects the run, and
//! records leave in canonical order.
use super::{
    layout,
    model::Document,
    records::{self, Demand, Record},
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
    /// Shown each bundled document, on a worker, for what a run's records don't hold.
    pub on_document: &'a (dyn Fn(&Document) + Sync),
    /// Given each document's records on the worker that derived them, before the pool keeps
    /// what it returns, to take what needn't wait for the rest of the run.
    pub on_records: OnRecords<'a>,
}

/// What a pool hands each document's records to. Its error fails the run.
pub type OnRecords<'a> = &'a (dyn Fn(Vec<Record>) -> Result<Vec<Record>, String> + Sync);

/// Keeps every record whole.
pub fn keep_records(records: Vec<Record>) -> Result<Vec<Record>, String> {
    Ok(records)
}

/// Derives every job's document. `load` turns a job, numbered in arrival order, into its
/// document on a worker thread. At most `workers` jobs wait in the queue, so a producer
/// faster than the workers holds a bounded number of documents in memory. Split documents
/// are retained until every job has arrived, since they can arrive in any order.
pub fn pooled_records<J: Send>(
    jobs: impl IntoIterator<Item = Result<J, String>>,
    load: impl Fn(usize, J) -> Result<Value, String> + Sync,
    pool: &Pool<'_>,
) -> Result<Vec<Record>, String> {
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
    state.finish()
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

fn work<J>(
    queue: Arc<Mutex<Receiver<(usize, J)>>>,
    state: &State<'_>,
    load: &impl Fn(usize, J) -> Result<Value, String>,
) {
    while let Some((index, job)) = next(&queue) {
        // After a failure, keep draining so the producer never waits on a full queue.
        if state.failed() {
            continue;
        }
        if let Err(error) = load(index, job).and_then(|document| state.derive(index, document)) {
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
    failed: AtomicBool,
    failure: Mutex<Option<String>>,
    functions: Mutex<layout::FunctionIds>,
    split: Mutex<Vec<(usize, Value)>>,
    records: Mutex<Vec<Record>>,
}

impl<'a> State<'a> {
    fn new(pool: &Pool<'a>) -> Self {
        Self {
            demand: pool.demand,
            on_failure: pool.on_failure,
            on_document: pool.on_document,
            on_records: pool.on_records,
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

    fn derive(&self, index: usize, document: Value) -> Result<(), String> {
        if document["layout"].as_str() == Some("bundled") {
            return self.bundled(&layout::bundle(vec![document])?);
        }
        self.split
            .lock()
            .expect("no panic while retaining")
            .push((index, document));
        Ok(())
    }

    fn bundled(&self, document: &Document) -> Result<(), String> {
        (self.on_document)(document);

        self.functions
            .lock()
            .expect("no panic while including")
            .include(document)?;
        let batch = (self.on_records)(records::records(document, self.demand)?)?;
        self.records
            .lock()
            .expect("no panic while extending")
            .extend(batch);
        Ok(())
    }

    fn finish(self) -> Result<Vec<Record>, String> {
        if let Some(failure) = self.failure.lock().expect("no panic").take() {
            return Err(failure);
        }
        let mut split = std::mem::take(&mut *self.split.lock().expect("no panic"));
        split.sort_by_key(|(index, _)| *index);
        let split = split.into_iter().map(|(_, document)| document).collect();

        for group in layout::split_runs(split) {
            self.bundled(&layout::bundle(group)?)?;
        }
        let mut records = self.records.into_inner().expect("no panic");
        records::canonical_order(&mut records);
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
            |_, document| Ok(document),
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
    fn it_derives_what_the_serial_reader_derives() {
        let demand = Demand::default();
        let serial = read_records([Ok(example())], demand).unwrap();
        let mut sorted = serial.clone();
        records::canonical_order(&mut sorted);
        let parallel = pooled_records([Ok(example())], |_, d| Ok(d), &pool(4, demand));
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
        };
        let jobs = (0..100).map(|index| match index {
            3 => Err("broken pipe".to_string()),
            _ => Ok(index),
        });
        let error = pooled_records(jobs, |_, _| Ok(example()), &pool).unwrap_err();
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
            |_, _| Err("worker failed".to_string()),
            &pool(2, Demand::default()),
        )
        .unwrap_err();
        assert_eq!(error, "worker failed");
        assert!(produced.load(Ordering::SeqCst) < 10_000);
    }
}
