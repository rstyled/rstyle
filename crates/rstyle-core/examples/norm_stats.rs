//! `norm_stats <facts.json>...` prints, per rule alone and for `all`, how many functions with a
//! graph change under `canonicalize`, and the mean node count before/after. Tells whether a
//! corpus exercises a rule at all (a rule that never fires cannot show a gain or a loss).

use rstyle_core::facts::FnFacts;
use rstyle_core::normalize::{Rules, canonicalize};
use rstyle_core::wl::{Wl, mineable};
use std::{fs, process::exit};

fn load(paths: &[String]) -> Result<Vec<FnFacts>, String> {
    let mut all = Vec::new();
    for path in paths {
        let text = fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
        let list: Vec<FnFacts> = serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))?;
        all.extend(list.into_iter().filter(|f| f.pdg.is_some()));
    }
    Ok(all)
}

fn main() {
    let files: Vec<String> = std::env::args().skip(1).collect();
    let fns = load(&files).unwrap_or_else(|e| {
        eprintln!("norm_stats: {e}");
        exit(2)
    });
    let mineable_raw = fns
        .iter()
        .filter(|f| mineable(&Wl::new(f.pdg.as_ref().expect("pdg"))))
        .count();
    println!("functions\t{}\tmineable_raw\t{mineable_raw}", fns.len());
    println!("rule\tchanged\tmineable_after\tnodes_before\tnodes_after");
    for name in [
        "ctor", "pred", "case", "exit", "counter", "cmp", "try", "all",
    ] {
        let rules = Rules::parse(name).expect("known rule");
        let (mut changed, mut mine, mut before, mut after) = (0, 0, 0, 0);
        for f in &fns {
            let raw = f.pdg.as_ref().expect("pdg");
            let canon = canonicalize(raw, &rules);
            changed += usize::from(&canon != raw);
            mine += usize::from(mineable(&Wl::new(&canon)));
            (before, after) = (before + raw.nodes.len(), after + canon.nodes.len());
        }
        println!("{name}\t{changed}\t{mine}\t{before}\t{after}");
    }
}
