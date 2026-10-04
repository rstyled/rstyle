//! `norm_comparable <rules> <facts.json>...` reads `id-a<TAB>id-b` (full fn ids) from stdin and
//! prints `1` or `0` per pair: whether the two graphs, after `canonicalize(rules)` (`""` = raw),
//! pass `wl::comparable` (size floor, scale ratio, calls). A fn without a graph is `0`.
//! The CodeNet scorer uses it to report recall over comparable pairs and over all pairs separately.

use rstyle_core::facts::FnFacts;
use rstyle_core::normalize::{Rules, canonicalize};
use rstyle_core::wl::{Wl, comparable};
use std::{collections::HashMap, env, fs, io::Read, process::exit};

fn load(paths: &[String], rules: &Rules) -> Result<HashMap<String, Wl>, String> {
    let mut graphs = HashMap::new();
    for path in paths {
        let text = fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
        let list: Vec<FnFacts> = serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))?;
        for f in list {
            if let Some(pdg) = &f.pdg {
                graphs
                    .entry(f.id.clone())
                    .or_insert_with(|| Wl::new(&canonicalize(pdg, rules)));
            }
        }
    }
    Ok(graphs)
}

fn verdict(graphs: &HashMap<String, Wl>, line: &str) -> Result<u8, String> {
    let (a, b) = line.split_once('\t').ok_or("pair lines are `a<TAB>b`")?;
    Ok(match (graphs.get(a), graphs.get(b.trim())) {
        (Some(wa), Some(wb)) => u8::from(comparable(wa, wb)),
        _ => 0,
    })
}

fn run(args: &[String]) -> Result<(), String> {
    let [rules, files @ ..] = args else {
        return Err("usage: norm_comparable <rules> <facts.json>... < pairs".into());
    };
    let graphs = load(files, &Rules::parse(rules)?)?;
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .map_err(|e| e.to_string())?;
    let out: Result<Vec<String>, String> = input
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| verdict(&graphs, l).map(|v| v.to_string()))
        .collect();
    println!("{}", out?.join("\n"));
    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if let Err(e) = run(&args) {
        eprintln!("norm_comparable: {e}");
        exit(2);
    }
}
