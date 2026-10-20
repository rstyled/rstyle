use super::*;

#[test]
fn outside_loop_merges_compose_with_nested_choices_and_short_circuits() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), fixture()).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 11);
    for function in functions {
        assert_eq!(
            function["outcome"],
            if function["facts"]["name"] == "inside" {
                "excluded"
            } else {
                "admitted"
            },
            "{function}"
        );
    }
    check_aggregate_documents(&report);
    check_equivalent_matching(&report, ["after", "same"]);
    let after = functions
        .iter()
        .find(|function| function["facts"]["name"] == "after")
        .unwrap();
    reject_cyclic_site_mutations(after);
}

fn reject_cyclic_site_mutations(function: &Value) {
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    for change in ["frontier", "guard", "snapshot", "controller", "successor"] {
        let mut pair = rstyle_pdg::producer::produce(&facts).unwrap();
        let sites = pair
            .pdg
            .extensions
            .get_mut("compat.rust.hir-r0:value-sites")
            .unwrap();
        corrupt_site(sites, change, &pair.pdg.nodes);
        synchronize_alternative(&mut pair.pdg.edges, sites, change);
        pair.cfg
            .extensions
            .insert("compat.rust.hir-r0:value-sites".into(), sites.clone());
        assert!(
            rstyle_pdg::validate_value_sites(&pair.pdg, &pair.cfg).is_err(),
            "{change}"
        );
        assert!(
            rstyle_ccgraph::admit_produced(&pair, &facts).is_err(),
            "{change}"
        );
    }
}

fn synchronize_alternative(edges: &mut [rstyle_pdg::PdgEdge], sites: &Value, change: &str) {
    if matches!(change, "guard" | "snapshot") {
        let first = &sites[0]["alternatives"][0];
        let edge = edges
            .iter_mut()
            .find(|edge| {
                edge.target == sites[0]["pdg_node_id"].as_str().unwrap()
                    && edge.subkind == "value_alternative"
                    && edge.operand_position == Some(0)
            })
            .unwrap();
        edge.source = first["value_pdg_node_id"].as_str().unwrap().into();
        edge.annotations.insert(
            "compat.rust.hir-r0:guards".into(),
            first["annotations"]["compat.rust.hir-r0:guards"].clone(),
        );
    }
}

fn fixture() -> &'static str {
    r#"
        pub fn after(mut n:u32, b:bool, x:u32, y:u32) -> u32 {
            while n>0 { n-=1; } if b {x} else {y}
        }
        pub fn same(mut n:u32, b:bool, x:u32, y:u32) -> u32 {
            while n>0 { n-=1; } if b {x} else {y}
        }
        pub fn before(mut n:u32, b:bool, x:u32, y:u32) -> u32 {
            let saved=if b {x} else {y}; while n>0 {n-=1;} saved
        }
        pub fn nested(mut n:u32,b:bool,c:bool,x:u32,y:u32) -> u32 {
            while n>0 {n-=1;} if b {if c{x}else{y}} else {0}
        }
        pub fn short(mut n:u32, b:bool,c:bool) -> bool {
            let saved=b&&c; while n>0{n-=1;} saved
        }
        pub fn arm_loop(mut n:u32,b:bool) -> u32 {
            if b {while n>0{n-=1;} 1} else {2}
        }
        pub fn changing_guard(mut n:u32,b:bool) -> bool {
            while n>0{n-=1;} b || n>0
        }
        pub fn guard_break(mut n:u32,stop:bool) -> u32 {
            while n>0{if stop{break;} n-=1;} if n==0{1}else{2}
        }
        pub fn matching(mut n:u32,v:u8) -> u32 { while n>0{n-=1;} match v {0=>1,_=>2} }
        pub fn divergent() -> u32 { loop {} if true {1} else {2} }
        pub fn inside(mut n:u32,b:bool) -> u32 {
            while n>0{n=if b{0}else{n-1};} n
        }
    "#
}

fn corrupt_site(sites: &mut Value, change: &str, nodes: &[rstyle_pdg::PdgNode]) {
    match change {
        "frontier" => {
            sites[0]["alternatives"][0]["completion_cfg_node_ids"] =
                sites[0]["alternatives"][1]["completion_cfg_node_ids"].clone()
        }
        "successor" => {
            sites[0]["cfg_successor_node_id"] =
                sites[0]["alternatives"][0]["completion_cfg_node_ids"][0].clone()
        }
        "guard" => {
            sites[0]["alternatives"][0]["annotations"]["compat.rust.hir-r0:guards"] =
                serde_json::json!([[]])
        }
        "snapshot" => {
            sites[0]["alternatives"][0]["value_pdg_node_id"] =
                sites[0]["alternatives"][1]["value_pdg_node_id"].clone()
        }
        "controller" => {
            sites[0]["controller_pdg_node_id"] = nodes
                .iter()
                .find(|node| node.attributes.construct.as_deref() == Some("while"))
                .unwrap()
                .id
                .clone()
                .into()
        }
        _ => unreachable!(),
    }
}
