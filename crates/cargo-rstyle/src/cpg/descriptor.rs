//! Read an inherited descriptor through an owned duplicate that no child inherits. On Windows
//! the number is an inherited handle's value, which stays the same in the child.
use std::fs::File;

#[cfg(unix)]
pub(super) fn open(number: i64) -> Result<File, String> {
    i32::try_from(number)
        .map_err(std::io::Error::other)
        .and_then(duplicate)
        .map_err(|error| format!("Cannot duplicate input fd {number}: {error}"))
}

#[cfg(unix)]
fn duplicate(number: i32) -> std::io::Result<File> {
    use std::os::fd::FromRawFd;
    // SAFETY: fcntl accepts arbitrary descriptor numbers and reports EBADF for
    // closed ones. No BorrowedFd/OwnedFd is constructed from the caller's number.
    let copy = unsafe { libc::fcntl(number, libc::F_DUPFD_CLOEXEC, 3) };
    match copy {
        -1 => Err(std::io::Error::last_os_error()),
        // SAFETY: successful fcntl returned a new descriptor owned only by us.
        _ => Ok(unsafe { File::from_raw_fd(copy) }),
    }
}

#[cfg(windows)]
pub(super) fn open(number: i64) -> Result<File, String> {
    duplicate(number).map_err(|error| format!("Cannot duplicate input handle {number}: {error}"))
}

#[cfg(windows)]
fn duplicate(number: i64) -> std::io::Result<File> {
    use std::ffi::c_void;
    use std::os::windows::io::FromRawHandle;
    const DUPLICATE_SAME_ACCESS: u32 = 2;
    unsafe extern "system" {
        fn GetCurrentProcess() -> *mut c_void;
        fn DuplicateHandle(
            source_process: *mut c_void,
            source: *mut c_void,
            target_process: *mut c_void,
            target: *mut *mut c_void,
            access: u32,
            inherit: i32,
            options: u32,
        ) -> i32;
    }
    let mut copy = std::ptr::null_mut();
    // SAFETY: DuplicateHandle accepts arbitrary handle values and reports invalid ones. No
    // borrowed or owned handle is constructed from the caller's number.
    let duplicated = unsafe {
        let process = GetCurrentProcess();
        DuplicateHandle(
            process,
            number as usize as *mut c_void,
            process,
            &mut copy,
            0,
            0,
            DUPLICATE_SAME_ACCESS,
        )
    };
    match duplicated {
        0 => Err(std::io::Error::last_os_error()),
        // SAFETY: a successful DuplicateHandle returned a new handle owned only by us.
        _ => Ok(unsafe { File::from_raw_handle(copy) }),
    }
}

#[cfg(not(any(unix, windows)))]
pub(super) fn open(number: i64) -> Result<File, String> {
    Err(format!(
        "--input-fd {number} needs Unix or Windows; use --stdin on this platform"
    ))
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::{
        io::{Read, Seek, Write},
        os::fd::AsRawFd,
    };

    #[test]
    fn dropping_the_duplicate_keeps_the_original_open_and_shares_its_position() {
        let mut original = tempfile::tempfile().unwrap();
        original.write_all(b"abcdef").unwrap();
        original.rewind().unwrap();
        let mut copy = open(i64::from(original.as_raw_fd())).unwrap();
        // SAFETY: copy owns a live descriptor throughout this flag query.
        let flags = unsafe { libc::fcntl(copy.as_raw_fd(), libc::F_GETFD) };
        assert_ne!(flags & libc::FD_CLOEXEC, 0);
        let mut bytes = [0; 3];
        copy.read_exact(&mut bytes).unwrap();
        assert_eq!(&bytes, b"abc");
        drop(copy);
        original.read_exact(&mut bytes).unwrap();
        assert_eq!(&bytes, b"def");
    }

    #[test]
    fn a_number_too_wide_for_a_descriptor_is_an_error() {
        assert!(
            open(i64::from(i32::MAX) + 1)
                .unwrap_err()
                .contains("Cannot duplicate input fd")
        );
    }

    #[test]
    fn a_closed_number_returns_an_error_without_constructing_an_invalid_borrow() {
        assert!(
            open(i64::from(i32::MAX))
                .unwrap_err()
                .contains("Cannot duplicate input fd")
        );
    }
}
