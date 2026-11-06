//! `rstyle`: the commands of `cargo rstyle` under their own name, so `rstyle go` needs no Cargo.

use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let mut command = match sibling() {
        Ok(program) => Command::new(program),
        Err(error) => return failed(&error),
    };
    command.args(std::env::args_os().skip(1));
    run(command)
}

// rstyle-allow(side_effect_density): IO boundary: locates the binary this one forwards to
fn sibling() -> Result<std::path::PathBuf, String> {
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    let program = exe.with_file_name(format!("cargo-rstyle{}", std::env::consts::EXE_SUFFIX));
    match program.is_file() {
        true => Ok(program),
        false => Err(format!(
            "cargo-rstyle not found at {}; install both binaries side by side",
            program.display()
        )),
    }
}

#[cfg(unix)]
fn run(mut command: Command) -> ExitCode {
    use std::os::unix::process::CommandExt;
    failed(&command.exec().to_string())
}

#[cfg(not(unix))]
fn run(mut command: Command) -> ExitCode {
    match command.status() {
        Ok(status) => ExitCode::from(status.code().unwrap_or(2).clamp(0, 255) as u8),
        Err(error) => failed(&error.to_string()),
    }
}

fn failed(error: &str) -> ExitCode {
    eprintln!("{error}");
    ExitCode::from(2)
}
