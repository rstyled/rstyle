#![feature(rustc_private)]

//! `RUSTC_WORKSPACE_WRAPPER` driver: runs rustc normally, and after analysis
//! dumps per-function facts as JSON into `$RSTYLE_OUT_DIR`.

extern crate rustc_abi;
extern crate rustc_ast;
extern crate rustc_driver;
extern crate rustc_hir;
extern crate rustc_hir_typeck;
extern crate rustc_interface;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_span;

mod args;
mod effects;
mod extract;
mod ids;
mod mutations;
mod pdg;
mod sig;

use rustc_driver::{Callbacks, Compilation};
use rustc_interface::interface::Compiler;
use rustc_middle::ty::TyCtxt;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

/// What the wrapped rustc invocation is compiling.
struct Target {
    crate_name: String,
    kind: &'static str,
}

/// Value following `flag`, in either `--flag value` or `--flag=value` form.
fn flag_value(args: &[String], flag: &str) -> Option<String> {
    let prefix = format!("{flag}=");
    args.iter().enumerate().find_map(|(index, arg)| {
        arg.strip_prefix(&prefix).map(str::to_string).or_else(|| {
            (arg == flag)
                .then(|| args.get(index + 1).cloned())
                .flatten()
        })
    })
}

/// `None` for anything that is not a real crate compilation (probes, build scripts).
fn target_of(args: &[String]) -> Option<Target> {
    let is_probe = args.iter().any(|arg| arg.starts_with("--print"));
    let crate_name = flag_value(args, "--crate-name")?;

    if is_probe || crate_name == "build_script_build" {
        return None;
    }
    let kind = if args.iter().any(|arg| arg == "--test") {
        "test"
    } else if flag_value(args, "--crate-type").as_deref() == Some("bin") {
        "bin"
    } else {
        "lib"
    };
    Some(Target { crate_name, kind })
}

fn with_sysroot(mut args: Vec<String>) -> Vec<String> {
    if flag_value(&args, "--sysroot").is_none() {
        let output = Command::new(&args[0]).args(["--print", "sysroot"]).output();
        if let Ok(output) = output {
            args.push("--sysroot".into());
            args.push(String::from_utf8_lossy(&output.stdout).trim().to_string());
        }
    }
    args
}

struct Extract {
    out_file: PathBuf,
    root: PathBuf,
    /// Also record dependence graphs (`RSTYLE_PDG` set by `cargo rstyle patterns`).
    pdg: bool,
}

impl Callbacks for Extract {
    // rstyle-allow(side_effect_density): IO boundary: writes the facts file
    fn after_analysis<'tcx>(&mut self, _: &Compiler, tcx: TyCtxt<'tcx>) -> Compilation {
        let facts = extract::crate_facts(tcx, &self.root, self.pdg);
        let json = serde_json::to_string_pretty(&facts).expect("facts serialize");
        std::fs::write(&self.out_file, json).expect("write facts file");
        Compilation::Continue
    }
}

fn main() -> ExitCode {
    // argv: [driver, <rustc path>, <rustc args...>]; rustc's own main expects
    // the rustc path first, so drop only our own name.
    let args: Vec<String> = std::env::args().skip(1).collect();
    let out_dir = std::env::var_os("RSTYLE_OUT_DIR").map(PathBuf::from);
    let args = with_sysroot(args);
    let extract = out_dir.zip(target_of(&args)).map(|(dir, target)| Extract {
        out_file: dir.join(format!("{}-{}.json", target.crate_name, target.kind)),
        root: std::env::var_os("RSTYLE_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().expect("cwd")),
        pdg: std::env::var_os("RSTYLE_PDG").is_some(),
    });

    rustc_driver::catch_with_exit_code(|| match extract {
        Some(mut callbacks) => rustc_driver::run_compiler(&args, &mut callbacks),
        None => rustc_driver::run_compiler(&args, &mut NoOp),
    })
}

struct NoOp;
impl Callbacks for NoOp {}
