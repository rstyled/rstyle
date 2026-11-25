//! A capture asked to stop while Cargo compiles: SIGTERM, as a job past its time sends its whole process group. Cargo
//! and the compilers it runs stop with it, and the capture, rather than dying with them, keeps the owners they finished
//! for the next capture and then exits. Outside the compile, SIGTERM stops the capture as it always has.
use std::sync::atomic::{AtomicBool, Ordering};

static ASKED: AtomicBool = AtomicBool::new(false);

/// Whether the capture was asked to stop while compiling.
pub(super) fn asked() -> bool {
    ASKED.load(Ordering::SeqCst)
}

/// While this lives, SIGTERM only notes that the capture was asked to stop.
pub(super) struct Watch {
    #[cfg(unix)]
    previous: libc::sighandler_t,
}

#[cfg(unix)]
extern "C" fn note(_signal: libc::c_int) {
    ASKED.store(true, Ordering::SeqCst);
}

// rstyle-allow(side_effect_density): signal boundary: installs the SIGTERM handler for the compile
/// Notes SIGTERM rather than dying of it, until the watch is dropped.
#[cfg(unix)]
pub(super) fn watch() -> Watch {
    let handler: extern "C" fn(libc::c_int) = note;
    // SAFETY: the handler only stores to an atomic, which is async-signal-safe.
    let previous = unsafe { libc::signal(libc::SIGTERM, handler as libc::sighandler_t) };
    Watch { previous }
}

#[cfg(not(unix))]
pub(super) fn watch() -> Watch {
    Watch {}
}

impl Drop for Watch {
    fn drop(&mut self) {
        #[cfg(unix)]
        // SAFETY: this puts back the disposition `watch` replaced.
        unsafe {
            libc::signal(libc::SIGTERM, self.previous);
        }
    }
}
