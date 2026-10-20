use super::*;

#[test]
fn scalar_tuple_assignments_compose_with_loops_and_operand_effects() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        r#"
        pub fn step(mut a: u128, mut b: u128) -> (u128,u128) { (a,b)=(b,a%b); (a,b) }
        pub fn same(mut a: u128, mut b: u128) -> (u128,u128) { (a,b)=(b,a%b); (a,b) }
        pub fn ordered(mut a: u32, mut b: u32) -> (u32,u32) { (a,b)=({ b=3; a },b); (a,b) }
        pub fn loop_step(mut a: u128, mut b: u128) -> u128 { while b!=0 { (a,b)=(b,a%b); } a }
        pub fn single(mut a: bool, b: bool) -> bool { (a,)=(b,); a }
        pub fn repeated(mut a: u32) -> u32 { (a,a)=(1,2); a }
        pub fn nested(mut a:u32, mut b:u32) -> u32 { ((a,),b)=((1,),2); a+b }
        pub fn indirect(a:&mut u32, mut b:u32) -> u32 { (*a,b)=(b,1); b }
        pub fn nonliteral(mut a:u32, mut b:u32, x:(u32,u32)) -> u32 { (a,b)=x; a+b }
    "#,
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 9);
    for function in functions {
        let positive = matches!(
            function["facts"]["name"].as_str().unwrap(),
            "step" | "same" | "ordered" | "loop_step" | "single"
        );
        assert_eq!(
            function["outcome"],
            if positive { "admitted" } else { "excluded" },
            "{function}"
        );
    }
    check_aggregate_documents(&report);
    check_equivalent_matching(&report, ["step", "same"]);
    let step = functions
        .iter()
        .find(|function| function["facts"]["name"] == "step")
        .unwrap();
    reject_target_mutations(step);
}

fn reject_target_mutations(function: &Value) {
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    for change in ["repeat", "immutable", "rhs-order"] {
        let mut changed = facts.clone();
        let assignment = &mut changed.body.children[0].children[0];
        match change {
            "repeat" => {
                assignment.children[0].children[1].res =
                    assignment.children[0].children[0].res.clone()
            }
            "immutable" => changed.bindings[0].is_mut = false,
            "rhs-order" => assignment.children[1].children.swap(0, 1),
            _ => unreachable!(),
        }
        assert!(rstyle_pdg::producer::produce(&changed).is_err(), "{change}");
        shared_evidence::reject_facts(function, &changed);
    }
}
