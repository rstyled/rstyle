//! Read an inherited descriptor through an owned, close-on-exec duplicate.
use std::fs::File;

#[cfg(unix)]
pub(super) fn open(number: i32) -> Result<File, String> {
    duplicate(number).map_err(|error| format!("Cannot duplicate input fd {number}: {error}"))
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

#[cfg(not(unix))]
pub(super) fn open(number: i32) -> Result<File, String> {
    Err(format!(
        "--input-fd {number} requires Unix; use --stdin on this platform"
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
        let mut copy = open(original.as_raw_fd()).unwrap();
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
    fn a_closed_number_returns_an_error_without_constructing_an_invalid_borrow() {
        assert!(
            open(i32::MAX)
                .unwrap_err()
                .contains("Cannot duplicate input fd")
        );
    }
}
