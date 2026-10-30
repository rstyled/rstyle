//! Diagnostic subprocess adapter: tee stderr live while preserving Output.
use std::io::{self, Read, Write};
use std::process::{Command, Output, Stdio};

pub fn output(command: &mut Command) -> io::Result<Output> {
    if !rstyle_pdg::diagnostics::switch("RSTYLE_PERF") {
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
