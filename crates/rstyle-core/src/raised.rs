//! A switch a serializer reads on its own thread, for output that differs only in how it's
//! written, raised around one write and lowered again however that write ends.

use std::cell::Cell;
use std::thread::LocalKey;

/// What `write` returns, with `switch` raised on this thread while it runs.
pub(crate) fn raised<T>(switch: &'static LocalKey<Cell<bool>>, write: impl FnOnce() -> T) -> T {
    struct Lower(&'static LocalKey<Cell<bool>>, bool);
    impl Drop for Lower {
        fn drop(&mut self) {
            self.0.set(self.1);
        }
    }
    let _lower = Lower(switch, switch.replace(true));
    write()
}

#[cfg(test)]
mod tests {
    use super::*;

    thread_local! {
        static SWITCH: Cell<bool> = const { Cell::new(false) };
    }

    #[test]
    fn a_switch_is_lowered_again_however_the_write_ends() {
        assert!(raised(&SWITCH, || raised(&SWITCH, || SWITCH.get())));
        assert!(!SWITCH.get());
        let panicked = std::panic::catch_unwind(|| raised(&SWITCH, || panic!("while writing")));
        assert!(panicked.is_err());
        assert!(!SWITCH.get());
    }
}
