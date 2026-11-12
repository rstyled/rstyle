//! Diagnostic subprocess adapter: tee stderr live while preserving Output.
use std::io::{self, Read, Write};
use std::process::{Command, Output, Stdio};

pub fn output(command: &mut Command) -> io::Result<Output> {
    if !rstyle_capture::diagnostics::switch("RSTYLE_PERF") {
        return command.output();
    }
    streamed_output(command)
}

// rstyle-allow(side_effect_density): Subprocess IO boundary drains stdout and stderr concurrently before collecting child status.
fn streamed_output(command: &mut Command) -> io::Result<Output> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn()?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("missing stdout pipe"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| io::Error::other("missing stderr pipe"))?;
    let captured = std::thread::scope(|scope| {
        let stdout = scope.spawn(|| collect(stdout));
        let stderr = tee(stderr);
        let stdout = stdout
            .join()
            .map_err(|_| io::Error::other("stdout reader panicked"))?;
        Ok::<_, io::Error>((stdout?, stderr?))
    });
    let status = child.wait()?;
    let (stdout, stderr) = captured?;

    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

/// Retain one source diagnostic, at most 512 UTF-8 bytes; stream the rest to the log.
/// Continue draining after selection so a verbose frontend cannot block capture.
pub(crate) fn stderr_diagnostic(mut reader: impl Read) -> io::Result<String> {
    let mut diagnostic = Diagnostic::default();
    let mut buffer = [0; 512];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            diagnostic.keep_line();
            return Ok(diagnostic.message);
        }
        emit(&buffer[..count]);
        for part in buffer[..count].split_inclusive(|byte| *byte == b'\n') {
            diagnostic.push(part);
        }
    }
}

#[derive(Default)]
struct Diagnostic {
    line: Vec<u8>,
    message: String,
    located: bool,
}

impl Diagnostic {
    fn push(&mut self, part: &[u8]) {
        if self.located {
            return;
        }
        // One extra byte detects an overlong line without retaining its remainder.
        self.line
            .extend_from_slice(&part[..part.len().min(513 - self.line.len())]);
        if part.ends_with(b"\n") {
            self.keep_line();
            self.line.clear();
        }
    }

    fn keep_line(&mut self) {
        if self.located || self.line.is_empty() {
            return;
        }
        let text = String::from_utf8_lossy(&self.line);
        let text = text.trim();
        if text.is_empty() {
            return;
        }
        const MARKER: &str = " [truncated]";
        let mut end = text.len().min(512 - MARKER.len());
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        self.message = match self.line.len().max(text.len()) > 512 {
            true => format!("{}{MARKER}", &text[..end]),
            false => text.to_owned(),
        };
        // Frontends use path:line: cause (or path:line:column: cause).
        self.located = text
            .split(':')
            .skip(1)
            .any(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()));
    }
}

fn collect(mut reader: impl Read) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;

    Ok(bytes)
}

fn tee(reader: impl Read) -> io::Result<Vec<u8>> {
    collect(StderrTee(reader))
}

struct StderrTee<R>(R);

impl<R: Read> Read for StderrTee<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let count = self.0.read(buffer)?;
        emit(&buffer[..count]);

        Ok(count)
    }
}

fn emit(bytes: &[u8]) {
    let mut stderr = io::stderr().lock();
    let _ = stderr.write_all(bytes);
    let _ = stderr.flush();
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn stderr_diagnostic_preserves_parser_details() {
        let details = b"invalid.py:1: invalid decimal literal";
        assert_eq!(
            stderr_diagnostic(details.as_slice()).unwrap().as_bytes(),
            details
        );
    }

    #[test]
    fn diagnostic_keeps_one_source_error_and_drains_progress() {
        let input = format!(
            "{}a.py:2: invalid syntax\nb.py:3: another error\n{}",
            "progress\n".repeat(8192),
            "done\n".repeat(8192)
        );
        let mut reader = input.as_bytes();
        assert_eq!(
            stderr_diagnostic(&mut reader).unwrap(),
            "a.py:2: invalid syntax"
        );
        assert!(reader.is_empty());
    }

    #[test]
    fn diagnostic_bounds_long_utf8_and_marks_truncation() {
        let input = format!("a.py:1: {}", "é".repeat(8192));
        let result = stderr_diagnostic(input.as_bytes()).unwrap();
        assert!(result.len() <= 512);
        assert!(result.starts_with("a.py:1: "));
        assert!(result.ends_with(" [truncated]"));
    }

    #[test]
    fn diagnostic_bounds_invalid_utf8() {
        let result = stderr_diagnostic([0xff; 8192].as_slice()).unwrap();
        assert!(result.len() <= 512);
        assert!(result.ends_with(" [truncated]"));
    }

    #[test]
    fn diagnostic_falls_back_to_last_nonempty_line() {
        assert_eq!(
            stderr_diagnostic(b"progress\nfatal error\n\n".as_slice()).unwrap(),
            "fatal error"
        );
    }

    #[test]
    fn diagnostic_marks_a_long_line_even_when_its_tail_is_whitespace() {
        let input = format!("a.py:1: invalid syntax{}", " ".repeat(8192));
        assert!(
            stderr_diagnostic(input.as_bytes())
                .unwrap()
                .ends_with(" [truncated]")
        );
    }

    #[test]
    fn diagnostic_is_empty_without_stderr() {
        assert!(stderr_diagnostic(b"".as_slice()).unwrap().is_empty());
    }

    #[test]
    fn streamed_child_output_preserves_status_and_exact_bytes() {
        let output = streamed_output(
            Command::new("sh").args(["-c", "printf 'stdout\n'; printf 'stderr\n' >&2; exit 7"]),
        )
        .unwrap();
        assert_eq!(output.status.code(), Some(7));
        assert_eq!(output.stdout, b"stdout\n");
        assert_eq!(output.stderr, b"stderr\n");
    }
}
