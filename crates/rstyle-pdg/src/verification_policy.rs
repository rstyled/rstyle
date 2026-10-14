//! Validation policy for explicitly trusted, current-run compiler production.
//! Independent library calls and imported replay default to full verification.
use std::cell::Cell;

thread_local! {
    static GENERATED: Cell<bool> = const { Cell::new(false) };
}

struct Restore(bool);

impl Drop for Restore {
    fn drop(&mut self) {
        GENERATED.with(|active| active.set(self.0));
    }
}

/// Trust the producer's source correspondence and certified helper summaries.
/// Structural graph validation still runs. Use only around owned current-run
/// production, or explicitly selected diagnostic replay of one's own facts.
/// The policy is thread-local, nested, and restored on early return or unwind.
pub fn generated<T>(operation: impl FnOnce() -> T) -> T {
    with_policy(true, operation)
}

/// Enforce independent imported verification even inside a generated scope.
pub fn strict<T>(operation: impl FnOnce() -> T) -> T {
    with_policy(false, operation)
}

fn with_policy<T>(generated: bool, operation: impl FnOnce() -> T) -> T {
    let previous = GENERATED.with(|active| active.replace(generated));
    let _restore = Restore(previous);

    operation()
}

pub(crate) fn is_generated() -> bool {
    GENERATED.with(Cell::get)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_policy_is_scoped_and_restored_after_unwind() {
        assert!(!is_generated());
        generated(|| {
            assert!(is_generated());
            generated(|| assert!(is_generated()));
            assert!(is_generated());
            assert!(!std::thread::spawn(is_generated).join().unwrap());
        });
        assert!(!is_generated());
        let result = std::panic::catch_unwind(|| generated(|| panic!("diagnostic failure")));
        assert!(result.is_err());
        assert!(!is_generated());
    }
}
