//! Bounded reuse of successful callee proofs during one root verification.
//! Exact resolution bytes include the complete dependency and build context.
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
};

const MAX_RETAINED_BYTES: usize = 32 * 1024 * 1024;

#[derive(Default)]
struct Session {
    proofs: BTreeMap<bool, BTreeSet<String>>,
    retained_bytes: usize,
}

thread_local! {
    static ACTIVE: RefCell<Option<Session>> = const { RefCell::new(None) };
}

struct Root(bool);

impl Drop for Root {
    fn drop(&mut self) {
        if self.0 {
            ACTIVE.with_borrow_mut(|active| *active = None);
        }
    }
}

pub(crate) fn within<T>(operation: impl FnOnce() -> T) -> T {
    let root = ACTIVE.with_borrow_mut(|active| {
        if active.is_some() {
            false
        } else {
            *active = Some(Session::default());
            true
        }
    });
    let _root = Root(root);

    operation()
}

pub(crate) fn contains(resolution: &str) -> bool {
    ACTIVE.with_borrow(|active| {
        active.as_ref().is_some_and(|session| {
            session
                .proofs
                .get(&crate::verification_policy::is_generated())
                .is_some_and(|proofs| proofs.contains(resolution))
        })
    })
}

pub(crate) fn remember(resolution: &str) {
    ACTIVE.with_borrow_mut(|active| {
        let Some(session) = active else { return };
        if resolution.len() > MAX_RETAINED_BYTES - session.retained_bytes {
            return;
        }
        let proofs = session
            .proofs
            .entry(crate::verification_policy::is_generated())
            .or_default();
        if proofs.insert(resolution.to_owned()) {
            session.retained_bytes += resolution.len();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_proofs_are_root_scoped_and_policy_specific() {
        within(|| {
            remember("complete evidence");
            within(|| assert!(contains("complete evidence")));
            assert!(!contains("changed evidence"));
            crate::verification_policy::generated(|| assert!(!contains("complete evidence")));
            assert!(
                !std::thread::spawn(|| contains("complete evidence"))
                    .join()
                    .unwrap()
            );
        });
        within(|| assert!(!contains("complete evidence")));
        let _ = std::panic::catch_unwind(|| {
            within(|| {
                remember("complete evidence");
                panic!("interrupted verification");
            })
        });
        within(|| assert!(!contains("complete evidence")));
    }

    #[test]
    fn evidence_budget_limits_retention_without_rejecting_work() {
        within(|| {
            ACTIVE.with_borrow_mut(|active| {
                active.as_mut().unwrap().retained_bytes = MAX_RETAINED_BYTES - 3
            });
            remember("abc");
            remember("abcd");
            assert!(contains("abc"));
            assert!(!contains("abcd"));
        });
    }
}
