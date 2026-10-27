#!/usr/bin/env python3
"""Retain bounded original time dependency diagnostics; never infer admission."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import shutil
import signal
import sys
import subprocess


def require(condition, message):
    if not condition:
        raise ValueError(message)


def verify_sources(directory, selection, root):
    spec = importlib.util.spec_from_file_location("time_error_oracle", directory / "oracle.py")
    oracle = importlib.util.module_from_spec(spec)
    previous = sys.dont_write_bytecode
    try:
        sys.dont_write_bytecode = True
        spec.loader.exec_module(oracle)
    finally:
        sys.dont_write_bytecode = previous
    regions = selection["source_contexts"] + selection["functions"] + list(selection["diagnostic_regions"].values())
    for region in regions:
        oracle.checked_source(root, region)


def source_bytes(value, snapshots):
    if "bytes" in value:
        return value["bytes"].encode()
    require(set(value) == {"snapshot", "range", "digest"}, "unexpected shared source reference")
    span = value["range"]
    original = snapshots[value["snapshot"]].encode()
    require(0 <= span["start"] <= span["end"] <= len(original), "shared source span outside snapshot")
    return original[span["start"]:span["end"]]


def known(build, key, expected):
    require(build[key]["status"] == "known" and build[key]["value"] == expected,
            f"captured {key} differs from frozen build selection")


def check_build(facts, selection, context, target, root):
    build = facts["build"]
    expected = selection["build_selection"]
    for key, value in {
        "rustc_commit": expected["rustc_commit"], "edition": expected["edition"],
        "pointer_width": expected["pointer_width"], "overflow_checks": expected["overflow_checks"],
        "debug_assertions": expected["debug_assertions"], "panic_strategy": expected["panic_strategy"],
        "opt_level": expected["optimization"],
        "target_triple": {"aarch64-apple-darwin": "arm64-apple-macosx"}.get(target, target),
    }.items():
        known(build, key, value)
    require(build["features"] == context["expected_feature_closure"], "captured feature closure differs")
    options = build["frontend_options"]
    require(options["target"] == target and options["extraction_profile"] == "hir-data-v3",
            "captured frontend target/profile differs")
    require("--test" not in options["invocation"], "selected original must be library/test=false")
    unit = options["rstyle.nominal_selected_unit"]
    package = unit["package"]
    require(unit["crate_name"] == "time" and Path(unit["root_source"]).resolve() == root / "src/lib.rs",
            "captured original crate root differs")
    require((package["name"], package["version"]) == ("time", "0.3.47")
            and Path(package["manifest_path"]).resolve() == root / "Cargo.toml"
            and package["manifest_bytes"].encode() == (root / "Cargo.toml").read_bytes(),
            "captured original package/manifest differs")


def observation(value):
    require(value["status"] in {"Captured", "Unsupported", "Unknown", "BudgetExceeded"},
            "unknown concrete observation status")
    return {key: value[key] for key in ["status", "reason"] if key in value}


def definition(value, crate, suffix, local):
    require(value["crate"] == crate and value["local"] is local
            and value["path"].split("::")[-len(suffix):] == suffix
            and value["definition_identity"] and value["stable_crate_id"],
            "conversion definition lacks exact crate/member identity: " + "::".join(suffix))


def nominal_type(values, path):
    require(len(values) == 1 and values[0]["kind"] == "type"
            and values[0]["identity_status"] == "known", "conversion requires one known concrete type")
    identity = values[0]["identity"]
    require(identity["kind"] == "adt" and identity["arguments"] == [],
            "conversion type must be exact nongeneric nominal")
    definition(identity["definition"], "time", path.split("::"), True)
    return identity


def original_member_source(value, expected, root, original):
    span = {"start": expected["start"], "end": expected["end"]}
    require(Path(value["path"]).resolve() == root / expected["path"] and value["span"] == span,
            "conversion member source span/path differs from frozen selection")
    require(original[span["start"]:span["end"]] == expected["source"].encode(),
            "conversion member source bytes differ from frozen selection")


def conversion_member(facts, expected, selection, root, required):
    if expected not in selection["functions"][2:]:
        return {"status": "not_applicable"}
    value = facts["build"]["frontend_options"].get("rstyle.rust.concrete-conversion-member")
    if value is None:
        require(not required, "required concrete conversion member observation is missing")
        return {"status": "missing"}
    if value["status"] != "observed":
        require(not required, "required concrete conversion member was not observed")
        return {key: value[key] for key in ["status", "reason"] if key in value}
    observed = value["observation"]
    require(observed["identity"] == "rstyle.rust.concrete-conversion-member-observation/1"
            and observed["admission_authority"] is False, "conversion observation must grant no admission authority")
    inverse = expected == selection["functions"][3]
    trait, method = ("TryFrom", "try_from") if inverse else ("From", "from")
    definition(observed["trait_definition"], "core", ["convert", trait], False)
    definition(observed["trait_method"], "core", ["convert", trait, method], False)
    require(observed["selected"] == observed["method"] and observed["selected_arguments"] == [],
            "selected concrete conversion differs from owning method")
    definition(observed["method"], "time", [method], True)
    implementation = observed["implementation"]
    require(implementation["crate"] == "time" and implementation["local"] is True
            and implementation["definition_identity"] and implementation["stable_crate_id"]
            and observed["method"]["path"] == implementation["path"] + "::" + method,
            "conversion method does not belong to original local implementation")
    require(observed["method_header"] == {"is_async": False, "is_const": False},
            "conversion method header differs from original")
    region = selection["diagnostic_regions"]["try_from_impl" if inverse else "from_impl"]
    original = (root / expected["path"]).read_bytes()
    original_member_source(observed["implementation_source"], region, root, original)
    original_member_source(observed["method_source"], expected, root, original)
    members = observed["members"]
    require(len(members) == (2 if inverse else 1), "conversion complete member count differs")
    methods = [member for member in members if member["definition"] == observed["method"]]
    require(len(methods) == 1 and methods[0]["trait_member"] == observed["trait_method"]
            and methods[0]["source"] == observed["method_source"], "conversion method mapping differs")
    arguments = observed["trait_arguments"]
    require(len(arguments) == 2, "conversion must retain Self and input type arguments")
    component = "error::component_range::ComponentRange"
    parsed = "error::try_from_parsed::TryFromParsed"
    own, incoming = (component, parsed) if inverse else (parsed, component)
    nominal_type([arguments[0]], own)
    nominal_type([arguments[1]], incoming)
    signature = observed["selected_signature"]
    require(signature == observed["requested_signature"] and signature["bound_variables"] == 0
            and signature["safety"] == "Safe" and signature["variadic"] is False
            and signature["abi"] == '\"Rust\"' and len(signature["inputs"]) == 1,
            "conversion concrete requested/selected signature differs")
    nominal_type(signature["inputs"][0], incoming)
    error = observed["associated_error"]
    if inverse:
        require(error is not None and error["declared_type"] == error["normalized_projection"],
                "TryFrom Error declaration and normalized projection differ")
        nominal_type(error["declared_type"], "error::different_variant::DifferentVariant")
        definition(error["trait_member"], "core", ["convert", "TryFrom", "Error"], False)
        definition(error["implementation_member"], "time", ["Error"], True)
        require(error["implementation_member"]["path"] == implementation["path"] + "::Error"
                and error["implementation_member"]["definition_identity"] != observed["method"]["definition_identity"],
                "TryFrom associated Error is not its distinct owning member")
        associated = [member for member in members if member["definition"] == error["implementation_member"]]
        require(len(associated) == 1 and associated[0]["trait_member"] == error["trait_member"]
                and associated[0]["source"] == error["source"], "TryFrom Error member mapping differs")
        spelling = "type Error = error::DifferentVariant;"
        offset = region["source"].encode().index(spelling.encode())
        start = region["start"] + offset
        error_region = {"path": region["path"], "start": start, "end": start + len(spelling), "source": spelling}
        original_member_source(error["source"], error_region, root, original)
        result = signature["output"][0]["identity"]
        require(len(signature["output"]) == 1 and signature["output"][0]["identity_status"] == "known"
                and result["kind"] == "adt" and len(result["arguments"]) == 2,
                "TryFrom output must be concrete Result")
        definition(result["definition"], "core", ["result", "Result"], False)
        require(result["arguments"] == [arguments[0]["identity"], error["declared_type"][0]["identity"]],
                "TryFrom output success/error identities differ")
    else:
        require(error is None, "From must not invent associated Error")
        nominal_type(signature["output"], own)
    return {"status": "observed", "trait": "core::convert::" + trait,
            "complete_members": len(members), "selected_is_owning_method": True,
            "associated_error": "error::different_variant::DifferentVariant" if inverse else None,
            "original_impl_and_method_source": True, "admission_authority": False}


def summarize(output, selection, context, target, root, exit_code, require_members=False):
    require(exit_code == 2, "capture-only invocation must report exit two")
    report = json.loads((output / "stdout.json").read_text())
    require(report["capture_complete"] is False and report["functions"] == [],
            "capture-only unexpectedly published ordinary functions")
    require((output / "Cargo.lock").is_file(), "missing retained dependency workspace lock")
    files = sorted((output / "snapshots").glob("*-pre-helper-certification.json"))
    matches = []
    for path in files:
        raw = json.loads(path.read_text())
        require(raw["schema"] == "rstyle.diagnostic.raw-facts/2"
                and raw["phase"] == "pre-helper-certification", "unexpected raw snapshot schema/phase")
        selected = []
        for expected in selection["functions"]:
            candidates = [facts for facts in raw["facts"]
                          if facts["fn_span"] == {"start": expected["start"], "end": expected["end"]}
                          and Path(facts["build"]["frontend_options"]["source_file"]).resolve() == root / expected["path"]]
            if not candidates:
                break
            require(len(candidates) == 1, "selected original is ambiguous within owning unit")
            facts = candidates[0]
            check_build(facts, selection, context, target, root)
            require(source_bytes(facts["source"], raw["sources"]) == expected["source"].encode(),
                    "captured selected source differs from frozen original")
            concrete = facts["build"]["frontend_options"]["concrete_type_observations"]
            selected.append({
                "function": expected["name"], "captured_name": facts["name"],
                "source_scan_complete": facts["source_scan_complete"],
                "source_diagnostics": facts["source_diagnostics"],
                "parameter_observations": [observation(value) for value in concrete["parameters"]],
                "result_observation": observation(concrete["result"]),
                "conversion_member": conversion_member(facts, expected, selection, root, require_members),
            })
        if len(selected) == len(selection["functions"]):
            matches.append((path, raw, selected))
    require(len(matches) == 1, "expected one complete selected original owning snapshot")
    path, raw, selected = matches[0]
    # The retained snapshot keeps complete original source context even though ordinary
    # source closure/certification is deliberately not performed in diagnostic mode.
    sources = {value.encode() for value in raw["sources"]}
    for expected in selection["source_contexts"]:
        if expected["path"].startswith("src/"):
            require(expected["source"].encode() in sources,
                    "selected complete source context absent from retained snapshots: " + expected["path"])
    return {
        "context": context["name"], "target": target, "exit_code": exit_code,
        "raw_snapshot": str(path), "raw_definition_count": len(raw["facts"]),
        "selected_functions": selected, "unchanged_selected_source": True,
        "dependency_lock": str(output / "Cargo.lock"),
        "capture_failure_count": len(report["capture_failures"]),
        "handoff_failure_count": len(report["handoff_failures"]),
        "capture_complete": False, "admission_evaluated": False,
        "source_closure_certified": False, "matching_evaluated": False,
    }


def environment_for(output, driver, selection):
    environment = dict(os.environ)
    for key in list(environment):
        if key.startswith(("RSTYLE_", "CARGO_PROFILE_", "CARGO_TARGET_")) or key in {
            "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC", "RUSTDOC", "RUSTC_WRAPPER",
            "RUSTC_WORKSPACE_WRAPPER", "CARGO_BUILD_TARGET", "CARGO_BUILD_RUSTFLAGS",
        }:
            environment.pop(key, None)
    environment.update({
        "RSTYLE_DRIVER": str(driver.resolve()), "RSTYLE_DIAGNOSTIC_CAPTURE_ONLY": "1",
        "RSTYLE_DIAGNOSTIC_SHARED_SOURCES": "1",
        "RSTYLE_DIAGNOSTIC_FACTS_DIR": str(output / "snapshots"),
        "CARGO_TARGET_DIR": str(output / "target"),
        "RUSTUP_TOOLCHAIN": selection["build_selection"]["toolchain"],
    })
    return environment


def capture(args, output, selection, context, root, target):
    output.mkdir(parents=True, exist_ok=False)
    (output / "src").mkdir()
    (output / "src/lib.rs").write_text("// Diagnostic dependency workspace; original time source is unchanged.\n")
    default = str(context["default_features"]).lower()
    (output / "Cargo.toml").write_text(
        '[package]\nname="time_error_capture"\nversion="0.0.0"\nedition="2024"\n'
        '[workspace]\n[dependencies]\n'
        f'time={{path={json.dumps(str(root))},default-features={default},features=["parsing"]}}\n'
    )
    command = [str(args.cli.resolve()), "clones", "--manifest-path", str(output / "Cargo.toml"),
               "--capture-dependency", "time", "--offline", "--format", "json",
               "--report-detail", "compact", "--extraction-profile", "hir-data-v3",
               "--max-pairs", "0", "--target", target]
    if args.lockfile:
        shutil.copyfile(args.lockfile, output / "Cargo.lock")
        command.append("--locked")
    (output / "command.json").write_text(json.dumps({"command": command, "timeout_seconds": args.timeout,
        "mode": "capture-only/shared-source raw diagnostics; no certification or admission"}, indent=2) + "\n")
    with (output / "stdout.json").open("w") as stdout, (output / "stderr.log").open("w") as stderr:
        process = subprocess.Popen(command, env=environment_for(output, args.driver, selection),
                                   stdout=stdout, stderr=stderr, start_new_session=True)
        try:
            exit_code = process.wait(timeout=args.timeout)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGTERM)
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait(timeout=10)
            (output / "exit-code.json").write_text(json.dumps({"timed_out": True}) + "\n")
            raise
    (output / "exit-code.json").write_text(json.dumps({"exit_code": exit_code}) + "\n")
    return exit_code


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--context", choices=["default-parsing", "no-default-parsing"], default="default-parsing")
    parser.add_argument("--source-root", type=Path)
    parser.add_argument("--target", default="aarch64-unknown-linux-gnu")
    parser.add_argument("--summarize", action="store_true", help="inspect retained diagnostics without running Cargo")
    parser.add_argument("--require-conversion-members", action="store_true",
                        help="require observed exact original From/TryFrom member mappings")
    parser.add_argument("--cli", type=Path, default=Path("target/debug/cargo-rstyle"))
    parser.add_argument("--driver", type=Path, default=Path("target/debug/rstyle-driver"))
    parser.add_argument("--lockfile", type=Path, help="reuse retained workspace lock, adding --locked")
    parser.add_argument("--timeout", type=int, default=240)
    args = parser.parse_args()
    require(1 <= args.timeout <= 600, "capture timeout must be between one and 600 seconds")
    directory = Path(__file__).resolve().parent
    selection = json.loads((directory / "selection.json").read_text())
    require(args.target in [selection["build_selection"]["initial_target"]]
            + selection["build_selection"]["later_capture_targets"], "target outside frozen selection")
    context = next(value for value in selection["build_selection"]["contexts"] if value["name"] == args.context)
    root = (args.source_root or Path(selection["package"]["source_root"])).resolve()
    verify_sources(directory, selection, root)
    output = args.output.resolve()
    if args.summarize:
        status = json.loads((output / "exit-code.json").read_text())
        require("exit_code" in status, "retained capture timed out or lacks terminal exit status")
        exit_code = status["exit_code"]
    else:
        exit_code = capture(args, output, selection, context, root, args.target)
    verify_sources(directory, selection, root)
    summary = summarize(output, selection, context, args.target, root, exit_code, args.require_conversion_members)
    (output / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary))


if __name__ == "__main__":
    main()
