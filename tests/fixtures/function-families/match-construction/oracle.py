#!/usr/bin/env python3
"""Check frozen Match sources; --run-native executes the unchanged public API."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile

RUNNER = r'''
use regex_automata::{Match, PatternID, Span};
use std::{cell::RefCell, panic::{catch_unwind, AssertUnwindSafe}, rc::Rc};

fn endpoints() -> Vec<usize> {
    let mut values: Vec<usize> = (0..=32).collect();
    for boundary in [64,128,256,65536,1usize << 31,1usize << 32,usize::MAX] {
        values.push(boundary - 1);
        values.push(boundary);
        if boundary != usize::MAX { values.push(boundary + 1); }
    }
    values.sort_unstable();
    values.dedup();
    values
}

fn check(result: std::thread::Result<Match>, pattern: usize, start: usize, end: usize, counts: &mut [u64;2]) {
    if start > end {
        assertion_panic(result);
        counts[1] += 1;
    } else {
        let value = result.expect("valid match must succeed");
        assert_eq!(value.pattern().as_usize(), pattern);
        assert_eq!(value.start(), start);
        assert_eq!(value.end(), end);
        assert_eq!(value.span(), Span { start, end });
        counts[0] += 1;
    }
}

fn assertion_panic(result: std::thread::Result<Match>) {
    let payload = result.expect_err("inverted match must panic");
    let message = payload.downcast_ref::<&str>().copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str));
    assert_eq!(message, Some("invalid match span"));
}

struct Custom {
    span: Span,
    events: Rc<RefCell<Vec<&'static str>>>,
    mode: u8,
}
impl Into<Span> for Custom {
    fn into(self) -> Span {
        self.events.borrow_mut().push("convert");
        if self.mode == 1 { std::panic::panic_any("custom conversion panic"); }
        if self.mode == 2 { Span { start: 7, end: 11 } } else { self.span }
    }
}
impl Drop for Custom {
    fn drop(&mut self) { self.events.borrow_mut().push("drop"); }
}
fn pattern(events: &Rc<RefCell<Vec<&'static str>>>) -> PatternID {
    events.borrow_mut().push("pattern");
    PatternID::must(2)
}
fn argument(events: &Rc<RefCell<Vec<&'static str>>>, mode: u8) -> Custom {
    events.borrow_mut().push("argument");
    Custom { span: Span { start: 99, end: 0 }, events: events.clone(), mode }
}
fn custom_cases() -> u64 {
    for mode in 0..3 {
        let events = Rc::new(RefCell::new(Vec::new()));
        let result = catch_unwind(AssertUnwindSafe(|| Match::new(pattern(&events), argument(&events, mode))));
        assert_eq!(*events.borrow(), ["pattern", "argument", "convert", "drop"]);
        match mode {
            0 => assertion_panic(result),
            1 => assert_eq!(result.unwrap_err().downcast_ref::<&str>(), Some(&"custom conversion panic")),
            _ => {
                let value = result.expect("conversion repairs original endpoints before assertion");
                assert_eq!((value.pattern().as_usize(), value.start(), value.end()), (2,7,11));
            }
        }
    }
    3
}
fn main() {
    std::panic::set_hook(Box::new(|_| {}));
    let mut counts = [0u64;2];
    for pattern in [0,1,2,2147483646] {
        let id = PatternID::must(pattern);
        for start in endpoints() {
            for end in endpoints() {
                check(catch_unwind(|| Match::new(id, Span { start,end })), pattern,start,end,&mut counts);
                // Range is consumed by its conversion; this does not imply Copy.
                let range = start..end;
                check(catch_unwind(|| Match::new(id, range)), pattern,start,end,&mut counts);
            }
        }
    }
    let custom = custom_cases();
    println!("{{\"pointer_width\":{},\"successes\":{},\"panics\":{},\"custom_cases\":{}}}", usize::BITS,counts[0],counts[1],custom);
}
'''
NONCOPY = 'fn require_copy<T: Copy>() {}\nfn main() { require_copy::<core::ops::Range<usize>>(); }\n'


def verify_sources(selection, package):
    for item in selection["functions"] + selection["owners"] + selection["source_obligations"]:
        raw = (package / item["file"]).read_bytes()
        span = item["span"]
        if raw[span["start"]:span["end"]] != item["source"].encode():
            raise RuntimeError(f"Original source/range changed: {item['name']}")
    identity = selection["package"]["historical_source_identity"]
    historic = json.loads((Path(__file__).parent / identity["selection"]).read_text())["package"]
    if historic["file_sha256"] != identity["files"]:
        raise RuntimeError("Historical identity references changed")
    if any(historic[key] != selection["package"][key] for key in ["name", "version"]):
        raise RuntimeError("Historical package identity changed")


def manifest(package, context):
    return ('[package]\nname="original_match_construction_witness"\nversion="0.0.0"\n'
            'edition="2021"\n[workspace]\n[dependencies.regex-automata]\n'
            f'path={json.dumps(str(package))}\n'
            f'default-features={str(context["default_features"]).lower()}\n'
            f'features={json.dumps(context["features"])}\n'
            '[profile.dev]\nopt-level=0\noverflow-checks=true\n'
            'debug-assertions=true\npanic="unwind"\n')


def expected_counts():
    values = set(range(33))
    maximum = (1 << 64) - 1
    for boundary in [64, 128, 256, 65536, 1 << 31, 1 << 32, maximum]:
        values.update([boundary - 1, boundary])
        if boundary != maximum:
            values.add(boundary + 1)
    size = len(values)
    return {"pointer_width": 64, "successes": 8 * size * (size + 1) // 2,
            "panics": 8 * size * (size - 1) // 2, "custom_cases": 3}


def native(args, selection, package):
    environment = dict(os.environ)
    environment["RUSTUP_TOOLCHAIN"] = selection["build_selection"]["toolchain"]
    for key in ["RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC_WRAPPER",
                "RUSTC_WORKSPACE_WRAPPER", "RUSTC", "CARGO_BUILD_TARGET"]:
        environment.pop(key, None)
    for key in list(environment):
        if key.startswith("CARGO_PROFILE_DEV_"):
            environment.pop(key)
    compiler = subprocess.run(["rustc", "-vV"], env=environment, text=True,
                              capture_output=True, check=True, timeout=30).stdout
    metadata = dict(line.split(": ", 1) for line in compiler.splitlines() if ": " in line)
    if metadata.get("commit-hash") != selection["build_selection"]["rustc_commit"]:
        raise RuntimeError("Compiler differs from frozen selection")
    if metadata.get("host") not in selection["build_selection"]["targets"]:
        raise RuntimeError("Native host outside selected target contexts")
    records = []
    with tempfile.TemporaryDirectory(prefix="rstyle-match-construction-") as temporary:
        workspace = Path(temporary)
        (workspace / "src").mkdir()
        for context in selection["build_selection"]["contexts"]:
            (workspace / "src/main.rs").write_text(RUNNER)
            (workspace / "Cargo.toml").write_text(manifest(package, context))
            command = [args.cargo, "run", "--offline", "--quiet", "--manifest-path",
                       str(workspace / "Cargo.toml"), "--target", metadata["host"]]
            if args.target_dir:
                command += ["--target-dir", str(args.target_dir.resolve())]
            completed = subprocess.run(command, env=environment, cwd=workspace, text=True,
                                       capture_output=True, timeout=args.timeout)
            if completed.returncode:
                raise RuntimeError(completed.stderr)
            result = json.loads(completed.stdout)
            if result != expected_counts():
                raise RuntimeError(f"Unexpected or incomplete oracle counts: {result}")
            records.append({"context": context["name"], **result, "mismatches": 0})
        (workspace / "src/main.rs").write_text(NONCOPY)
        command = [args.cargo, "check", "--offline", "--quiet", "--manifest-path",
                   str(workspace / "Cargo.toml"), "--target", metadata["host"]]
        if args.target_dir:
            command += ["--target-dir", str(args.target_dir.resolve())]
        rejected = subprocess.run(command, env=environment, cwd=workspace, text=True,
                                  capture_output=True, timeout=args.timeout)
        if rejected.returncode == 0 or "E0277" not in rejected.stderr or "Copy" not in rejected.stderr:
            raise RuntimeError("Range<usize> unexpectedly satisfies Copy or compile-fail evidence is incomplete")
    return {"native_host": metadata["host"], "rustc": metadata, "contexts": records,
            "range_copy_compile_fail": "expected E0277"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package-root", type=Path)
    parser.add_argument("--run-native", action="store_true",
                        help="build/run only when coordinator schedules")
    parser.add_argument("--cargo", default="cargo")
    parser.add_argument("--target-dir", type=Path)
    parser.add_argument("--timeout", type=int, default=180)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    selection = json.loads(Path(__file__).with_name("selection.json").read_text())
    package = (args.package_root or Path(selection["package"]["source_root"])).resolve()
    verify_sources(selection, package)
    result = {"identity": "rstyle.original-match-construction-witness/1",
              "scope": "unchanged original public API; runtime results are separate from extraction support",
              "selected_source_ranges_checked": True, "historical_identity_reference_checked": True,
              "whole_package_source_identity_recomputed": False,
              "execution": "native witness" if args.run_native else "not performed"}
    if args.run_native:
        result.update(native(args, selection, package))
    rendered = json.dumps(result, indent=2) + "\n"
    if args.output:
        args.output.write_text(rendered)
    print(rendered, end="")


if __name__ == "__main__":
    main()
