//! Exact generic instances compose across renamed dependencies and build scripts.
use super::*;

fn selected_workspace() -> TempDir {
    let directory = workspace();
    std::fs::write(directory.path().join("outside/build.rs"), "fn main() {}\n").unwrap();
    std::fs::write(
        directory.path().join("outside/src/lib.rs"),
        "pub fn bounded<T: PartialOrd>(x:T,lo:T)->T { if x < lo { lo } else { x } }\n\
         pub fn predicate<T: PartialOrd>(x:T,lo:T)->bool { x < lo }\n\
         pub fn tuple<T: Copy>(x:T)->T { (x,x).0 }\n",
    )
    .unwrap();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub fn signed(x:i32,lo:i32)->i32 { renamed::bounded(x,lo) }\n\
         pub fn signed_peer(x:i32,lo:i32)->i32 { renamed::bounded(x,lo) }\n\
         pub fn unsigned(x:u32,lo:u32)->u32 { renamed::bounded(x,lo) }\n\
         pub fn unsigned_peer(x:u32,lo:u32)->u32 { renamed::bounded(x,lo) }\n\
         pub fn outside(x:u8,lo:u8)->u8 { renamed::bounded(x,lo) }\n\
         pub fn unsupported_result(x:i32,lo:i32)->bool { renamed::predicate(x,lo) }\n\
         pub fn unsupported_body(x:i32)->i32 { renamed::tuple(x) }\n",
    )
    .unwrap();
    directory
}

fn expectations(report: &Value) {
    for name in ["signed", "signed_peer", "unsigned", "unsigned_peer"] {
        assert_eq!(named(report, name)["outcome"], "admitted", "{report}");
    }
    for name in [
        "bounded",
        "outside",
        "predicate",
        "unsupported_result",
        "tuple",
        "unsupported_body",
    ] {
        assert_eq!(named(report, name)["outcome"], "excluded", "{report}");
    }
    assert_eq!(report["functions"].as_array().unwrap().len(), 10);
    assert_eq!(report["pair_enumeration_complete"], true);
    assert_eq!(report["pair_evaluation_complete"], true);
    check_equivalent_matching(report, ["signed", "signed_peer"]);
    check_equivalent_matching(report, ["unsigned", "unsigned_peer"]);
}

#[test]
fn exact_scalar_instances_replay_into_complete_callers() {
    let directory = selected_workspace();
    let (output, captured) = clones(
        directory.path(),
        &[
            "--extraction-profile",
            "hir-data-v3",
            "--capture-dependency",
            "scalar_dependency",
        ],
    );
    assert!(output.status.success(), "{}", stdout(&output));
    inventory(&captured, 1, 2);
    expectations(&captured);
    let (output, replayed) = replay::replay(
        directory.path(),
        captured["run_directory"].as_str().unwrap(),
    );
    assert!(output.status.success(), "{}", stdout(&output));
    assert_eq!(replayed["verification_policy"], "strict-replay");
    expectations(&replayed);
    let (unit, index) = data_unit(named(&captured, "signed"));
    assert!(
        unit.pair(index).is_none(),
        "Bare caller bundle granted foreign authority"
    );
}
