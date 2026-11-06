#![feature(rustc_private)]

//! `RUSTC_WORKSPACE_WRAPPER` driver: runs rustc normally, and after analysis publishes the
//! crate's documents into the explicitly selected unit directory.

extern crate rustc_abi;
extern crate rustc_ast;
extern crate rustc_attr_ir;
extern crate rustc_driver;
extern crate rustc_hir;
extern crate rustc_hir_typeck;
extern crate rustc_interface;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_span;

mod cpg;
mod driver_options;
mod ids;
mod mutations;
mod paths;
mod spans;

use rstyle_core::cli_args::flag_value;
use rustc_driver::{Callbacks, Compilation};
use rustc_interface::interface::Compiler;
use rustc_middle::ty::TyCtxt;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

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

fn main() -> ExitCode {
    match driver_options::parse(std::env::args().skip(1).collect()) {
        Ok(invocation) => run(invocation),
        Err(error) => {
            eprintln!("rstyle-driver: {error}");
            ExitCode::from(2)
        }
    }
}

fn run(invocation: driver_options::Invocation) -> ExitCode {
    // The private driver selection has been consumed; rustc receives its path
    // followed by only its own compiler arguments.
    let args = with_sysroot(invocation.arguments);
    let capture = std::env::var_os("RSTYLE_DOCUMENT_DIR").map(|directory| CaptureDocuments {
        directory: PathBuf::from(directory),
        invocation: args.clone(),
        unit_context: invocation.unit_context,
        source_root: std::env::var_os(rstyle_core::advisory::SOURCE_ROOT_ENV).map(PathBuf::from),
    });

    if let (Some(dir), Some(root)) = (
        std::env::var_os(cpg::DIR_ENV),
        std::env::var_os(cpg::ROOT_ENV),
    ) {
        let mut callbacks = CpgEmit {
            dir: PathBuf::from(dir),
            root: PathBuf::from(root),
        };
        return rustc_driver::catch_with_exit_code(|| {
            rustc_driver::run_compiler(&args, &mut callbacks)
        });
    }

    rustc_driver::catch_with_exit_code(|| match capture {
        Some(mut callbacks) => rustc_driver::run_compiler(&args, &mut callbacks),
        None => rustc_driver::run_compiler(&args, &mut NoOp),
    })
}

struct NoOp;
impl Callbacks for NoOp {}

/// Writes the crate's `compat.cpg` documents.
struct CpgEmit {
    dir: PathBuf,
    root: PathBuf,
}

impl Callbacks for CpgEmit {
    fn after_analysis<'tcx>(&mut self, _: &Compiler, tcx: TyCtxt<'tcx>) -> Compilation {
        if let Err(error) = cpg::emit(tcx, &self.root, &self.dir) {
            tcx.dcx().fatal(format!(
                "rstyle-driver: compat.cpg emission failed: {error}"
            ));
        }
        Compilation::Continue
    }
}

/// Publishes the crate's documents; everything the run derives comes from them.
struct CaptureDocuments {
    directory: PathBuf,
    invocation: Vec<String>,
    unit_context: Option<rstyle_capture::UnitContext>,
    source_root: Option<PathBuf>,
}

impl Callbacks for CaptureDocuments {
    fn after_analysis<'tcx>(&mut self, _: &Compiler, tcx: TyCtxt<'tcx>) -> Compilation {
        let options = cpg::publish::Options {
            source_root: self.source_root.as_deref(),
            invocation: &self.invocation,
            unit_context: self.unit_context.as_ref(),
        };
        cpg::publish::publish(tcx, &options, &self.directory)
            .expect("publish compat.cpg documents");
        Compilation::Continue
    }
}
