//! `norm_pairs <rules> <facts.json>...` reads `fn-a fn-b` id suffixes (one pair per line) from
//! stdin and prints one TSV row per pair: nodes, WL similarity and alignment coverage on the raw
//! graphs, then the same after `normalize::canonicalize(rules)`.
//! A suffix must match exactly one fn id.

use rstyle_core::align::{Ctx, align};
use rstyle_core::facts::{FnFacts, Pdg};
use rstyle_core::normalize::{Rules, canonicalize};
use rstyle_core::wl::{Wl, comparable, similarity_milli};
use std::{collections::BTreeMap, env, fs, io::Read, process::exit};

fn load(paths: &[String]) -> Result<BTreeMap<String, FnFacts>, String> {
    let mut fns = BTreeMap::new();
    for path in paths {
        let text = fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
        let list: Vec<FnFacts> = serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))?;
        for f in list.into_iter().filter(|f| f.pdg.is_some()) {
            fns.entry(f.id.clone()).or_insert(f);
        }
    }
    Ok(fns)
}

fn find<'a>(fns: &'a BTreeMap<String, FnFacts>, needle: &str) -> Result<&'a Pdg, String> {
    let hits: Vec<_> = fns.values().filter(|f| f.id.ends_with(needle)).collect();
    match hits.as_slice() {
        [one] => Ok(one.pdg.as_ref().expect("filtered on pdg")),
        [] => Err(format!("no fn id ends with `{needle}`")),
        many => Err(format!("`{needle}` is ambiguous: {} matches", many.len())),
    }
}

/// nodes of both, comparable (0/1), WL milli, alignment coverage milli.
fn measure(a: &Pdg, b: &Pdg) -> String {
    let (wa, wb) = (Wl::new(a), Wl::new(b));
    let cov = align(&Ctx::new(a, &wa, b, &wb)).coverage_milli;
    let cmp = u8::from(comparable(&wa, &wb));
    let nodes = format!("{}/{}", a.nodes.len(), b.nodes.len());
    format!("{nodes}\t{cmp}\t{}\t{cov}", similarity_milli(&wa, &wb))
}

fn run(args: &[String]) -> Result<(), String> {
    let [rules, files @ ..] = args else {
        return Err("usage: norm_pairs <rules> <facts.json>... < pairs".into());
    };
    let rules = Rules::parse(rules)?;
    let fns = load(files)?;
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .map_err(|e| e.to_string())?;
    println!("a\tb\tnodes_raw\tcmp\twl\tcov\tnodes_norm\tcmp\twl\tcov");
    for line in input.lines().filter(|l| !l.trim().is_empty()) {
        let (a, b) = line.split_once(' ').ok_or("pair lines are `a b`")?;
        let (pa, pb) = (find(&fns, a)?, find(&fns, b.trim())?);
        let (ca, cb) = (canonicalize(pa, &rules), canonicalize(pb, &rules));
        println!(
            "{a}\t{}\t{}\t{}",
            b.trim(),
            measure(pa, pb),
            measure(&ca, &cb)
        );
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if let Err(e) = run(&args) {
        eprintln!("norm_pairs: {e}");
        exit(2);
    }
}
