#!/usr/bin/env python3
"""Check frozen original bytes; optionally run bounded public/private semantic witnesses."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile

RUNNER = r'''
static LEFT: [u8; 4] = *b"name";
static RIGHT: [u8; 4] = *b"name";

fn public_round_trip(original: time::error::ComponentRange) {
    let name = original.name();
    let conditional = original.is_conditional();
    let wrapped = time::error::TryFromParsed::from(original);
    match wrapped {
        time::error::TryFromParsed::ComponentRange(value) => {
            assert!(std::ptr::eq(value.name(), name));
            assert_eq!(value.is_conditional(), conditional);
        }
        _ => panic!("From selected wrong variant"),
    }
    let recovered = time::error::ComponentRange::try_from(wrapped).expect("ComponentRange tag");
    assert!(std::ptr::eq(recovered.name(), name));
    assert_eq!(recovered.name().as_bytes(), name.as_bytes());
    assert_eq!(recovered.is_conditional(), conditional);
}

fn public_cases() -> usize {
    use time::{Date, Month};
    let errors = [
        (Month::try_from(0u8).unwrap_err(), "month", false),
        (Month::try_from(13u8).unwrap_err(), "month", false),
        (Date::from_calendar_date(2024, Month::February, 30).unwrap_err(), "day", true),
        (Date::from_calendar_date(2023, Month::February, 29).unwrap_err(), "day", true),
        (Date::from_ordinal_date(2024, 367).unwrap_err(), "ordinal", true),
        (Date::from_ordinal_date(i32::MAX, 1).unwrap_err(), "year", false),
    ];
    let count = errors.len();
    for (error, name, conditional) in errors {
        assert_eq!(error.name(), name);
        assert_eq!(error.is_conditional(), conditional);
        public_round_trip(error);
    }
    let rejected: Result<time::error::ComponentRange, time::error::DifferentVariant> =
        time::error::ComponentRange::try_from(time::error::TryFromParsed::InsufficientInformation);
    assert_eq!(rejected, Err(time::error::DifferentVariant));
    count + 1
}

fn private_cases() -> usize {
    let left: &'static str = std::str::from_utf8(&LEFT).unwrap();
    let right: &'static str = std::str::from_utf8(&RIGHT).unwrap();
    assert_ne!(left.as_ptr(), right.as_ptr(), "distinct declared static backings");
    let names = ["", "year", "日付", "a\0b", "🦀", left, right];
    let mut count = 0;
    for name in names {
        for conditional in [false, true] {
            let original = if conditional {
                error::ComponentRange::conditional(name)
            } else {
                error::ComponentRange::unconditional(name)
            };
            assert!(std::ptr::eq(original.name, name));
            assert_eq!(original.name.as_bytes(), name.as_bytes());
            assert_eq!(original.is_conditional, conditional);
            let wrapped = error::TryFromParsed::from(original);
            match wrapped {
                error::TryFromParsed::ComponentRange(value) => {
                    assert!(std::ptr::eq(value.name, name));
                    assert_eq!(value.is_conditional, conditional);
                }
                _ => panic!("From selected wrong variant"),
            }
            let recovered = error::ComponentRange::try_from(wrapped).expect("ComponentRange tag");
            assert!(std::ptr::eq(recovered.name, name));
            assert_eq!(recovered.name.as_bytes(), name.as_bytes());
            assert_eq!(recovered.is_conditional, conditional);
            count += 1;
        }
    }
    assert_eq!(error::ComponentRange::try_from(error::TryFromParsed::InsufficientInformation),
               Err(error::DifferentVariant));
    count + 1
}

fn main() {
    let public = public_cases();
    let private = private_cases();
    println!("{{\"public_original_cases\":{public},\"private_diagnostic_cases\":{private},\"mismatches\":0}}");
}
'''


def checked_source(root, region):
    observed = (root / region["path"]).read_bytes()[region["start"]:region["end"]]
    if observed != region["source"].encode():
        raise SystemExit(f"Original source bytes disagree: {region['path']}:{region['start']}")


def diagnostic_source(selection):
    regions = selection["diagnostic_regions"]
    declarations = "\n".join(regions[name]["source"] for name in [
        "component_range", "try_from_parsed", "different_variant"
    ])
    constructors = "\n".join(function["source"] for function in selection["functions"][:2])
    conversions = "\n".join(regions[name]["source"] for name in ["from_impl", "try_from_impl"])
    return f"mod error {{\nuse crate::error;\n{declarations}\nimpl ComponentRange {{\n{constructors}\n}}\n{conversions}\n}}\n{RUNNER}"


def run_native(selection, root, context_name):
    contexts = selection["build_selection"]["contexts"]
    for context in contexts:
        if context_name != "both" and context["name"] != context_name:
            continue
        with tempfile.TemporaryDirectory(prefix="rstyle-time-error-oracle-") as temporary:
            directory = Path(temporary)
            (directory / "src").mkdir()
            dependency = json.dumps(str(root))
            default = str(context["default_features"]).lower()
            manifest = (
                '[package]\nname="time_error_oracle"\nversion="0.0.0"\nedition="2024"\n'
                '[workspace]\n[dependencies]\n'
                f'time={{path={dependency},default-features={default},features=["parsing"]}}\n'
                '[profile.dev]\nopt-level=0\noverflow-checks=true\ndebug-assertions=true\npanic="unwind"\n'
            )
            (directory / "Cargo.toml").write_text(manifest)
            (directory / "src/main.rs").write_text(diagnostic_source(selection))
            cargo = os.environ.get("CARGO", str(Path.home() / ".cargo/bin/cargo"))
            print(json.dumps({"native_context": context["name"], "scope": selection["oracle_scope"]}), flush=True)
            subprocess.run([
                cargo, "+nightly-2026-09-30", "run", "--offline", "--quiet",
                "--manifest-path", str(directory / "Cargo.toml"),
                "--target-dir", str(directory / "target"),
            ], check=True, timeout=180)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path)
    parser.add_argument("--run-native", action="store_true", help="build/run host witnesses; coordinator schedules")
    parser.add_argument("--context", choices=["default-parsing", "no-default-parsing", "both"], default="both")
    args = parser.parse_args()
    selection = json.loads((Path(__file__).resolve().parent / "selection.json").read_text())
    root = (args.source_root or Path(selection["package"]["source_root"])).resolve()
    regions = selection["source_contexts"] + selection["functions"] + list(selection["diagnostic_regions"].values())
    for region in regions:
        checked_source(root, region)
    print(json.dumps({"source_check": "pass", "functions": len(selection["functions"]),
                      "context_files": len(selection["source_contexts"]),
                      "admission_evaluated": False, "native_run": args.run_native}), flush=True)
    if args.run_native:
        run_native(selection, root, args.context)


if __name__ == "__main__":
    main()
