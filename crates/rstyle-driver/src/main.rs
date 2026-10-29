#![feature(rustc_private)]

//! `RUSTC_WORKSPACE_WRAPPER` driver: runs rustc normally, and after analysis
//! dumps per-function facts as JSON into `$RSTYLE_OUT_DIR`.

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

mod args;
mod driver_complexity;
mod driver_options;
mod effects;
mod extract;
mod hir_extraction;
mod ids;
mod mutations;
mod pdg;
mod sig;

use rstyle_core::cli_args::flag_value;
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

/// `None` for anything that is not a real crate compilation (probes, build scripts).
fn target_of(args: &[String]) -> Option<Target> {
    let is_probe = args.iter().any(|arg| arg.starts_with("--print"));
    let crate_name = flag_value(args, "--crate-name")?.to_owned();

    if is_probe || crate_name == "build_script_build" {
        return None;
    }
    let kind = if args.iter().any(|arg| arg == "--test") {
        "test"
    } else if flag_value(args, "--crate-type") == Some("bin") {
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
    complexity: driver_complexity::Functions,
}

impl Callbacks for Extract {
    fn after_expansion<'tcx>(&mut self, _: &Compiler, tcx: TyCtxt<'tcx>) -> Compilation {
        self.complexity = driver_complexity::collect(tcx);
        Compilation::Continue
    }

    // rstyle-allow(side_effect_density): IO boundary: writes the facts file
    fn after_analysis<'tcx>(&mut self, _: &Compiler, tcx: TyCtxt<'tcx>) -> Compilation {
        let facts = extract::crate_facts(tcx, &self.root, self.pdg, &self.complexity);
        let json = serde_json::to_string_pretty(&facts).expect("facts serialize");
        std::fs::write(&self.out_file, json).expect("write facts file");
        Compilation::Continue
    }
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
    let args = invocation.arguments;
    let out_dir = std::env::var_os("RSTYLE_OUT_DIR").map(PathBuf::from);
    let args = with_sysroot(args);
    let extract = out_dir.zip(target_of(&args)).map(|(dir, target)| Extract {
        out_file: dir.join(format!("{}-{}.json", target.crate_name, target.kind)),
        root: std::env::var_os("RSTYLE_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().expect("cwd")),
        pdg: std::env::var_os("RSTYLE_PDG").is_some(),
        complexity: Default::default(),
    });

    let document_dir = std::env::var_os("RSTYLE_HIR_DOCUMENT_DIR").map(PathBuf::from);
    let facts_path = std::env::var_os("RSTYLE_HIR_FACTS_FILE")
        .map(PathBuf::from)
        .or_else(|| document_dir.as_ref().map(|dir| dir.join("facts.json")));
    let r0 = facts_path.map(|path| HirExtract {
        output: hir_extraction::output::Destination {
            facts_file: path,
            document_directory: document_dir,
        },
        scanned: Vec::new(),
        invocation: args.clone(),
        profile: invocation.profile,
        unit_context: invocation.unit_context,
        source_root: std::env::var_os(rstyle_core::advisory::SOURCE_ROOT_ENV).map(PathBuf::from),
    });

    rustc_driver::catch_with_exit_code(|| match (r0, extract) {
        (Some(mut callbacks), _) => rustc_driver::run_compiler(&args, &mut callbacks),
        (None, Some(mut callbacks)) => rustc_driver::run_compiler(&args, &mut callbacks),
        (None, None) => rustc_driver::run_compiler(&args, &mut NoOp),
    })
}

struct NoOp;
impl Callbacks for NoOp {}

struct HirExtract {
    output: hir_extraction::output::Destination,
    scanned: Vec<hir_extraction::source::Function>,
    invocation: Vec<String>,
    profile: Option<rstyle_pdg::extraction_profile::ExtractionProfile>,
    unit_context: Option<rstyle_pdg::nominal_boundary::UnitContext>,
    source_root: Option<PathBuf>,
}

impl Callbacks for HirExtract {
    fn after_crate_root_parsing(
        &mut self,
        _: &Compiler,
        krate: &mut rustc_ast::Crate,
    ) -> Compilation {
        self.scanned = hir_extraction::source::scan(krate, self.profile.unwrap_or_default());
        Compilation::Continue
    }

    fn after_analysis<'tcx>(&mut self, _: &Compiler, tcx: TyCtxt<'tcx>) -> Compilation {
        let options = hir_extraction::Options {
            source_root: self.source_root.as_deref(),
            sources: Default::default(),
            profile: self.profile,
            invocation: &self.invocation,
            unit_context: self.unit_context.as_ref(),
        };
        self.capture(tcx, &options)
    }
}

impl HirExtract {
    fn capture(&self, tcx: TyCtxt<'_>, options: &hir_extraction::Options<'_>) -> Compilation {
        if rstyle_pdg::diagnostics::switch("RSTYLE_DIAGNOSTIC_CAPTURE_ONLY") {
            self.capture_diagnostic(tcx, options);
            Compilation::Stop
        } else {
            self.capture_complete(tcx, options);
            Compilation::Continue
        }
    }

    fn capture_diagnostic(&self, tcx: TyCtxt<'_>, options: &hir_extraction::Options<'_>) {
        let facts = hir_extraction::raw_crate_facts(tcx, &self.scanned, options)
            .expect("capture diagnostic raw facts");
        hir_extraction::capture_instance_diagnostics(tcx, &self.scanned, options)
            .expect("capture requested instance diagnostics");
        Self::retain_diagnostic(&facts);
    }

    fn retain_diagnostic(facts: &[rstyle_pdg::HirFnFacts]) {
        hir_extraction::diagnostics::snapshot("pre-helper-certification", facts)
            .expect("retain diagnostic raw facts");
        eprintln!(
            "rstyle diagnostic scope=extraction-only; helper certification and successful publication not performed"
        );
    }

    fn capture_complete(&self, tcx: TyCtxt<'_>, options: &hir_extraction::Options<'_>) {
        rstyle_pdg::verification_policy::generated(|| self.capture_generated(tcx, options));
    }

    fn capture_generated(&self, tcx: TyCtxt<'_>, options: &hir_extraction::Options<'_>) {
        let facts = hir_extraction::crate_facts(tcx, &self.scanned, options)
            .expect("capture complete source-facing R0 evidence");

        self.write_capture(tcx, options, &facts);
    }

    fn write_capture(
        &self,
        tcx: TyCtxt<'_>,
        options: &hir_extraction::Options<'_>,
        store: &rstyle_pdg::unit_store::UnitStore,
    ) {
        let inventory = self.inventory(tcx, options, store.functions());
        self.output
            .write_capture(&hir_extraction::output::CapturedOutput {
                store,
                inventory: inventory.as_ref(),
            })
            .expect("write captured evidence");
    }

    fn inventory(
        &self,
        tcx: TyCtxt<'_>,
        options: &hir_extraction::Options<'_>,
        facts: &[rstyle_pdg::HirFnFacts],
    ) -> Option<serde_json::Value> {
        options
            .selected()
            .supports("rust.conditional-source-correspondence")
            .then(|| {
                hir_extraction::source_inventory(tcx, &self.scanned, facts, options)
                    .expect("capture owned source inventory")
            })
    }
}
