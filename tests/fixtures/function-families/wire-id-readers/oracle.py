#!/usr/bin/env python3
"""Source-selected ID reader/error family; opt-in append-only whole-crate oracle."""
import argparse
import importlib.util
import json
from pathlib import Path
import re
import shutil
import subprocess
import tempfile


# Observe private error variants inside their original module. Production bodies
# are never copied, substituted or changed by these appended entry points.
WIRE = r'''
#[doc(hidden)]
pub(crate) fn __rstyle_wire_id_readers_probe(
    api: u8, slice: &[u8], what: &'static str,
) -> (u8, u32, usize, u64, &'static str) {
    let actual = match api {
        0 => read_state_id(slice, what).map(|(id, nr)| (id.as_u32(), nr)),
        1 => try_read_state_id(slice, what).map(|(id, nr)| (id.as_u32(), nr)),
        2 => read_pattern_id(slice, what).map(|(id, nr)| (id.as_u32(), nr)),
        _ => panic!("oracle API outside selected family"),
    };
    match actual {
        Ok((id, nr)) => (0, id, nr, 0, ""),
        Err(error) => match error.0 {
            DeserializeErrorKind::BufferTooSmall { what } => (1, 0, 0, 0, what),
            DeserializeErrorKind::StateID { err, what } => (2, 0, 0, err.attempted(), what),
            DeserializeErrorKind::PatternID { err, what } => (3, 0, 0, err.attempted(), what),
            _ => panic!("wrong original error variant"),
        },
    }
}
#[doc(hidden)]
pub(crate) fn __rstyle_wire_id_readers_constructors(
    bytes: [u8; 4], what: &'static str,
) -> (u64, &'static str, u64, &'static str) {
    let state = StateID::from_ne_bytes(bytes).unwrap_err();
    let pattern = PatternID::from_ne_bytes(bytes).unwrap_err();
    let before = (state.attempted(), pattern.attempted());
    let DeserializeErrorKind::StateID { err: state, what: state_what } =
        DeserializeError::state_id_error(state, what).0 else { panic!("wrong state constructor") };
    let DeserializeErrorKind::PatternID { err: pattern, what: pattern_what } =
        DeserializeError::pattern_id_error(pattern, what).0 else { panic!("wrong pattern constructor") };
    assert_eq!((state.attempted(), pattern.attempted()), before);
    (state.attempted(), state_what, pattern.attempted(), pattern_what)
}
'''

BRIDGE = r'''
#[doc(hidden)]
pub fn __rstyle_wire_id_readers_probe(
    api: u8, slice: &[u8], what: &'static str,
) -> (u8, u32, usize, u64, &'static str) {
    util::wire::__rstyle_wire_id_readers_probe(api, slice, what)
}
#[doc(hidden)]
pub fn __rstyle_wire_id_readers_constructors(
    bytes: [u8; 4], what: &'static str,
) -> (u64, &'static str, u64, &'static str) {
    util::wire::__rstyle_wire_id_readers_constructors(bytes, what)
}
'''

RUNNER = r'''
const LIMIT: u32 = 2_147_483_646;
fn bytes(mut word: u32) -> [u8; 4] {
    let mut result = [0; 4];
    for position in 0..4 {
        let slot = if cfg!(target_endian = "little") { position } else { 3 - position };
        result[slot] = (word % 256) as u8;
        word /= 256;
    }
    result
}
fn message(actual: &'static str, expected: &'static str) {
    assert_eq!(actual.len(), expected.len());
    assert_eq!(actual.as_ptr(), expected.as_ptr());
}
fn words() -> Vec<u32> {
    let mut values = vec![0, 1, LIMIT - 1, LIMIT, LIMIT + 1, LIMIT + 2, u32::MAX - 1, u32::MAX];
    values.extend((0..32).map(|bit| 1u32 << bit));
    let mut sample = 0x9E37_79B9u32;
    for _ in 0..256 {
        sample = sample.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        values.push(sample);
    }
    assert_eq!(values.len(), 296);
    values
}
fn full(slice: &[u8], word: u32, what: &'static str, api: u8) {
    let (tag, id, consumed, attempted, actual_what) =
        regex_automata::__rstyle_wire_id_readers_probe(api, slice, what);
    if word <= LIMIT {
        assert_eq!((tag, id, consumed), (0, word, 4));
    } else {
        assert_eq!(tag, if api == 2 { 3 } else { 2 });
        assert_eq!(attempted, u64::from(word));
        message(actual_what, what);
    }
}
fn full_inputs(messages: &[&'static str]) -> u64 {
    let mut count = 0;
    for word in words() {
        for offset in 0..8 {
            for len in 4..=8 {
                for tail in [0, 0xA5, 0xFF] {
                    let mut backing = [tail; 24];
                    backing[offset..offset + 4].copy_from_slice(&bytes(word));
                    let original = backing;
                    for &what in messages {
                        for api in 0..3 {
                            full(&backing[offset..offset + len], word, what, api);
                            assert_eq!(backing, original);
                            count += 1;
                        }
                    }
                }
            }
        }
    }
    count
}
fn short_inputs(messages: &[&'static str]) -> [u64; 2] {
    let mut counts = [0; 2];
    // Only expected short direct-reader panics unwind. Avoid repeated irrelevant
    // word/panic combinations and suppress their normal diagnostic printing.
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    for pattern in [0, 127, 128, 255] {
        for offset in 0..8 {
            for len in 0..4 {
                for tail in [0, 0xA5, 0xFF] {
                    let mut backing = [tail; 24];
                    backing[offset..offset + 4].fill(pattern);
                    let original = backing;
                    for &what in messages {
                        let actual = regex_automata::__rstyle_wire_id_readers_probe(
                            1, &backing[offset..offset + len], what);
                        assert_eq!(actual.0, 1, "checked short error precedence");
                        message(actual.4, what);
                        counts[0] += 1;
                        for api in [0, 2] {
                            let outcome = std::panic::catch_unwind(|| {
                                regex_automata::__rstyle_wire_id_readers_probe(
                                    api, &backing[offset..offset + len], what)
                            });
                            assert!(outcome.is_err(), "direct short read must panic");
                            counts[1] += 1;
                        }
                        assert_eq!(backing, original);
                    }
                }
            }
        }
    }
    std::panic::set_hook(previous);
    counts
}
fn constructors(messages: &[&'static str]) -> u64 {
    let mut count = 0;
    for word in [LIMIT + 1, LIMIT + 2, u32::MAX] {
        for &what in messages {
            let (state, state_what, pattern, pattern_what) =
                regex_automata::__rstyle_wire_id_readers_constructors(bytes(word), what);
            assert_eq!(state, u64::from(word));
            assert_eq!(pattern, u64::from(word));
            message(state_what, what);
            message(pattern_what, what);
            count += 2;
        }
    }
    count
}
fn main() {
    assert_eq!(usize::BITS, 64);
    static LEFT: [u8; 5] = *b"field";
    static RIGHT: [u8; 5] = *b"field";
    let messages = ["", core::str::from_utf8(&LEFT).unwrap(), core::str::from_utf8(&RIGHT).unwrap()];
    assert_ne!(messages[1].as_ptr(), messages[2].as_ptr());
    assert_eq!(messages[1], messages[2]);
    let full = full_inputs(&messages);
    let [short, panics] = short_inputs(&messages);
    let constructors = constructors(&messages);
    assert_eq!([full, short, panics, constructors], [319_680, 1_152, 2_304, 18]);
    println!("{{\"full_inputs\":{},\"checked_short_errors\":{},\"direct_short_panics\":{},\"constructor_controls\":{},\"total\":{},\"debug_assertions\":{},\"mismatches\":0}}",
        full, short, panics, constructors, full + short + panics + constructors, cfg!(debug_assertions));
}
'''


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


def verify(selection, root, common):
    common.verify_sources(selection, root)
    names = {"util::wire::" + name for name in ["read_state_id", "try_read_state_id", "read_pattern_id"]}
    names.update("util::wire::DeserializeError::" + name for name in ["state_id_error", "pattern_id_error"])
    if {item["name"] for item in selection["functions"] if item.get("role") == "target"} != names:
        raise ValueError("Selected original target family differs")
    declaration = next(item["source"] for item in selection["source_obligations"]
                       if item["name"] == "complete DeserializeErrorKind declaration")
    if re.findall(r"^    ([A-Za-z]+) \{", declaration, re.MULTILINE) != selection["error_variant_roster"]:
        raise ValueError("Complete error variant roster differs")
    corpus = selection["corpus"]
    expected = dict(full_inputs=296 * 8 * 5 * 3 * 3 * 3,
                    checked_short_errors=4 * 8 * 4 * 3 * 3,
                    direct_short_panics=4 * 8 * 4 * 3 * 3 * 2, constructor_controls=3 * 3 * 2)
    expected["total"] = sum(expected.values())
    if selection["expected_cases_per_context"] != expected or selection["limit"] != 2147483646:
        raise ValueError("Independent case inventory/limit differs")
    if (corpus["boundary_words"] != [0, 1, 2147483645, 2147483646, 2147483647, 2147483648, 4294967294, 4294967295]
            or corpus["one_hot_bits"] != list(range(32))
            or corpus["samples"] != dict(count=256, seed=2654435769, multiplier=1664525, increment=1013904223)
            or corpus["offsets"] != list(range(8)) or corpus["full_lengths"] != list(range(4, 9))
            or corpus["short_lengths"] != list(range(4)) or corpus["short_patterns"] != [0, 127, 128, 255]
            or corpus["tail_bytes"] != [0, 165, 255]
            or corpus["messages"] != ["", "field from distinct static LEFT", "field from distinct static RIGHT"]
            or corpus["constructor_words"] != [2147483647, 2147483648, 4294967295]):
        raise ValueError("Independent source-selected corpus differs")
    if [(context["name"], context["default_features"], context["features"])
            for context in selection["build_selection"]["contexts"]] != [
                ("default", True, []), ("no-default", False, [])]:
        raise ValueError("Original default/no-default selections differ")


def native(selection, root, args, here, common):
    # Reuse native compiler/environment checks and whole-original copy auditing.
    controls = module("native_controls", here.parent / "wire-flag-parser/oracle.py")
    env, host = controls.environment(selection, args)
    copies = module("original_copy_checks", here.parent / "time-digit-prefixes/oracle.py")
    records = []
    with tempfile.TemporaryDirectory(prefix="rstyle-wire-id-readers-") as temporary:
        copied = Path(temporary) / "original"
        shutil.copytree(root, copied, ignore=shutil.ignore_patterns("target", ".git"))
        for path in copied.rglob("*"):
            if path.is_file():
                path.chmod(path.stat().st_mode | 0o200)
        additions = {Path("src/util/wire.rs"): b"\n\n" + WIRE.encode(),
                     Path("src/lib.rs"): b"\n\n" + BRIDGE.encode()}
        for path, extra in additions.items():
            (copied / path).write_bytes((root / path).read_bytes() + extra)
        copies.verify_copy(root, copied, additions)
        runner = Path(temporary) / "runner"
        (runner / "src").mkdir(parents=True)
        (runner / "src/main.rs").write_text(RUNNER)
        modes = [True, False] if args.debug_mode == "both" else [args.debug_mode == "debug"]
        for context in selection["build_selection"]["contexts"]:
            if args.context not in ("both", context["name"]):
                continue
            for debug in modes:
                (runner / "Cargo.toml").write_text(
                    '[package]\nname="original_wire_id_readers_oracle"\nversion="0.0.0"\nedition="2021"\n[workspace]\n'
                    '[dependencies]\nregex-automata={path="../original",default-features='
                    + str(context["default_features"]).lower() + '}\n'
                    '[profile.dev]\nopt-level=0\noverflow-checks=true\npanic="unwind"\ndebug-assertions='
                    + str(debug).lower() + '\n')
                command = [args.cargo, "run", "--offline", "--quiet", "--manifest-path",
                           str(runner / "Cargo.toml"), "--target-dir", str(args.target_dir.resolve())]
                completed = subprocess.run(command, env=env, text=True, capture_output=True, timeout=args.timeout)
                if completed.returncode:
                    raise RuntimeError(f"Original ID oracle failed context={context['name']} debug={debug}:\n{completed.stderr}")
                record = json.loads(completed.stdout)
                if (any(record.get(key) != count for key, count in selection["expected_cases_per_context"].items())
                        or record.get("mismatches") != 0 or record.get("debug_assertions") != debug):
                    raise ValueError(f"Independent oracle inventory/context differs: {record}")
                records.append(dict(record, context=context["name"], host=host, admission_evaluated=False))
        copies.verify_copy(root, copied, additions)
    verify(selection, root, common)
    return records


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path)
    parser.add_argument("--run-native", action="store_true", help="coordinator schedules Cargo runs")
    parser.add_argument("--context", choices=["default", "no-default", "both"], default="both")
    parser.add_argument("--debug-mode", choices=["debug", "release-branch", "both"], default="both")
    parser.add_argument("--target-dir", type=Path, default=Path("target/wire-id-readers-oracle"))
    parser.add_argument("--cargo", default=str(Path.home() / ".cargo/bin/cargo"))
    parser.add_argument("--rustc", default=str(Path.home() / ".cargo/bin/rustc"))
    parser.add_argument("--timeout", type=int, default=300)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    here = Path(__file__).resolve().parent
    selection = json.loads((here / "selection.json").read_text())
    root = (args.source_root or Path(selection["package"]["source_root"])).resolve()
    common = module("source_checks", here.parent / "checked-index-decode/oracle.py")
    verify(selection, root, common)
    result = dict(identity="rstyle.original-wire-id-readers-oracle/1", source_identities_checked=True,
                  checksum="XXH3-64", admission_evaluated=False, contexts=[])
    if args.run_native:
        result["contexts"] = native(selection, root, args, here, common)
    rendered = json.dumps(result, indent=2) + "\n"
    if args.output:
        args.output.write_text(rendered)
    print(rendered, end="")


if __name__ == "__main__":
    main()
