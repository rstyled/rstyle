//! The smallest frontend that keeps the runner's contract, so the pipe a runner hands over can
//! be tested for real on every platform: `describe`, then `cpg --source DIR --output-fd N`.

use std::fs::File;
use std::io::Write;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        ["describe"] => println!(
            r#"{{"schema":"rstyle.frontend-description/1","language":"go","analyses":[]}}"#
        ),
        ["cpg", "--source", _, "--output-fd", number] => stream(number.parse().unwrap()),
        _ => panic!("unexpected arguments: {args:?}"),
    }
}

fn stream(number: u64) {
    let document = std::fs::read(std::env::var_os("FAKE_DOCUMENT").unwrap()).unwrap();
    let mut output = inherited(number);
    output.write_all(b"\x1e").unwrap();
    output.write_all(&document).unwrap();
    output.write_all(b"\n\x1enull\n").unwrap();
}

#[cfg(unix)]
fn inherited(number: u64) -> File {
    use std::os::fd::FromRawFd;
    // SAFETY: the runner handed this process the descriptor to own for the rest of its life.
    unsafe { File::from_raw_fd(number as i32) }
}

#[cfg(windows)]
fn inherited(number: u64) -> File {
    use std::os::windows::io::FromRawHandle;
    // SAFETY: the runner handed this process the handle to own for the rest of its life.
    unsafe { File::from_raw_handle(number as usize as _) }
}
