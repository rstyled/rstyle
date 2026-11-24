//! Work over a slice split across the machine's threads, with results in the slice's order, so
//! nothing that reads them can tell how many threads there were.

use std::sync::atomic::{AtomicUsize, Ordering};

/// How many items a thread takes at a time: enough that claiming them costs nothing, few enough
/// that threads finish together when some items cost far more than others.
const CHUNK: usize = 64;

/// `f` of each item, in order, worked out on up to as many threads as the machine offers.
pub(crate) fn mapped<T: Sync, U: Send>(items: &[T], f: impl Fn(&T) -> U + Sync) -> Vec<U> {
    let available = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
    mapped_on(available, items, f)
}

/// As [`mapped`], on at most `threads` threads.
fn mapped_on<T: Sync, U: Send>(threads: usize, items: &[T], f: impl Fn(&T) -> U + Sync) -> Vec<U> {
    let threads = threads.min(items.len().div_ceil(CHUNK));
    if threads <= 1 {
        return items.iter().map(f).collect();
    }
    let next = AtomicUsize::new(0);
    let claim = || {
        let mut done = Vec::new();
        loop {
            let start = next.fetch_add(CHUNK, Ordering::Relaxed);
            let Some(part) = items.get(start..items.len().min(start + CHUNK)) else {
                return done;
            };
            done.push((start, part.iter().map(&f).collect::<Vec<_>>()));
        }
    };
    let mut parts: Vec<(usize, Vec<U>)> = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..threads).map(|_| scope.spawn(claim)).collect();
        let joined = workers.into_iter().map(|worker| {
            worker
                .join()
                .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
        });
        joined.flatten().collect()
    });
    parts.sort_unstable_by_key(|(start, _)| *start);
    parts.into_iter().flat_map(|(_, part)| part).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn results_keep_the_items_order_however_many_threads_work() {
        let items: Vec<usize> = (0..1000).collect();
        let squared: Vec<usize> = items.iter().map(|n| n * n).collect();
        for threads in [1, 2, 3, 8, 64] {
            assert_eq!(mapped_on(threads, &items, |n| n * n), squared, "{threads}");
        }
        assert_eq!(mapped(&items, |n| n * n), squared);
        assert!(mapped(&[] as &[usize], |n| *n).is_empty());
    }

    #[test]
    fn a_panic_on_a_worker_carries_on_to_the_caller() {
        let items: Vec<usize> = (0..1000).collect();
        let caught = std::panic::catch_unwind(|| {
            mapped_on(4, &items, |n| assert!(*n != 999, "the last item"))
        });
        let panic = caught.expect_err("the panic carried on");
        assert_eq!(panic.downcast_ref::<&str>(), Some(&"the last item"));
    }
}
