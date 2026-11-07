//! The smallest frontend that keeps the runner's contract, so the pipes a runner hands over can
//! be tested for real on every platform: `describe`, `cpg --source DIR --output-fd N`, and
//! `analyze NAME --source DIR --out FILE --input-fd N`.

use std::fs::File;
use std::io::{Read, Write};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        ["describe"] => println!(
            r#"{{"schema":"rstyle.frontend-description/1","language":"go","analyses":[{{"name":"count","input":"documents"}}]}}"#
        ),
        ["cpg", "--source", _, "--output-fd", number] => stream(number.parse().unwrap()),
        [
            "analyze",
            "count",
            "--source",
            _,
            "--out",
            out,
            "--input-fd",
            number,
        ] => analyze(out, number.parse().unwrap()),
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

/// Says how many documents it read, once the last record says that's all of them.
fn analyze(out: &str, number: u64) {
    let mut received = Vec::new();
    inherited(number).read_to_end(&mut received).unwrap();
    assert!(received.ends_with(b"\x1enull\n"), "an incomplete stream");
    let documents = received.iter().filter(|byte| **byte == 0x1e).count() - 1;
    let log = format!(
        r#"{{"version":"2.1.0","runs":[{{"tool":{{"driver":{{"name":"fake"}}}},"results":[{{"message":{{"text":"read {documents}"}}}}]}}]}}"#
    );
    std::fs::write(out, log).unwrap();
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
