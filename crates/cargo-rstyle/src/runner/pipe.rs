//! A pipe with one end one child inherits, named by a number it can take as an argument: a
//! descriptor on Unix, a handle's value on Windows.
use std::process::Command;
use std::sync::{Mutex, MutexGuard, PoisonError};

static SPAWNING: Mutex<()> = Mutex::new(());

/// Held from making a pipe until the child that inherits one end is spawned and this process's
/// copy of that end is closed, and around every other spawn while analyses run side by side.
/// A child spawned in between could inherit the end too and hold the pipe open: on Windows any
/// child gets a handle whose flag is set, and on macOS a new pipe isn't close-on-exec at once.
pub(super) fn alone() -> MutexGuard<'static, ()> {
    SPAWNING.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Lets `command`'s child inherit this end of a pipe, and returns the number that names it there.
// rstyle-allow(side_effect_density): process boundary: arranges a child's inherited descriptor
#[cfg(unix)]
pub(super) fn inherit(
    command: &mut Command,
    end: &impl std::os::fd::AsRawFd,
) -> Result<String, String> {
    use std::os::unix::process::CommandExt;
    let number = end.as_raw_fd();

    // SAFETY: the closure runs in the forked child before exec and only calls fcntl, which is
    // async-signal-safe, on a descriptor the child holds a copy of. The parent's copy stays
    // close-on-exec, so no other child can hold the pipe open.
    unsafe {
        command.pre_exec(move || match libc::fcntl(number, libc::F_SETFD, 0) {
            -1 => Err(std::io::Error::last_os_error()),
            _ => Ok(()),
        });
    }
    Ok(number.to_string())
}

/// Windows inherits by the handle's own flag, so any child spawned while it's set also gets it
/// and holds the pipe open. The runner holds [`alone`] until it has dropped the handle.
#[cfg(windows)]
pub(super) fn inherit(
    _command: &mut Command,
    end: &impl std::os::windows::io::AsRawHandle,
) -> Result<String, String> {
    use std::ffi::c_void;
    const HANDLE_FLAG_INHERIT: u32 = 1;
    unsafe extern "system" {
        fn SetHandleInformation(handle: *mut c_void, mask: u32, flags: u32) -> i32;
    }
    let handle = end.as_raw_handle();
    // SAFETY: `end` owns this open handle for the whole call.
    match unsafe { SetHandleInformation(handle.cast(), HANDLE_FLAG_INHERIT, HANDLE_FLAG_INHERIT) } {
        0 => Err(format!(
            "Cannot let the frontend inherit its pipe: {}",
            std::io::Error::last_os_error()
        )),
        _ => Ok((handle as usize).to_string()),
    }
}

#[cfg(not(any(unix, windows)))]
pub(super) fn inherit<End>(_command: &mut Command, _end: &End) -> Result<String, String> {
    Err("A frontend's pipe needs Unix or Windows".into())
}
