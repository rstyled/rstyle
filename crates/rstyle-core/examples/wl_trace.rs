//! `wl_trace <fn-substring-a> <fn-substring-b> <facts.json>...` prints a WL trace as JSON.
//! Facts files come from a `patterns` run (`<target-dir>/rstyle/facts-sem/*.json`).
//! A substring must match exactly one fn id (the same fn in several targets counts once).

use rstyle_core::facts::FnFacts;
use rstyle_core::wl_trace::{Side, trace};
use std::{collections::BTreeMap, env, fs, process::exit};

const THRESHOLD_MILLI: u32 = 600;

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

fn find<'a>(fns: &'a BTreeMap<String, FnFacts>, needle: &str) -> Result<&'a FnFacts, String> {
    let hits: Vec<_> = fns.values().filter(|f| f.id.ends_with(needle)).collect();
    match hits.as_slice() {
        [one] => Ok(one),
        [] => Err(format!("no fn id ends with `{needle}`")),
        many => Err(format!("`{needle}` is ambiguous: {} matches", many.len())),
    }
}

fn run(args: &[String]) -> Result<String, String> {
    let [a, b, files @ ..] = args else {
        return Err("usage: wl_trace <fn-a> <fn-b> <facts.json>...".into());
    };
    let fns = load(files)?;
    let (fa, fb) = (find(&fns, a)?, find(&fns, b)?);
    let (sa, sb) = (Side::from_facts(fa), Side::from_facts(fb));
    let (Some(sa), Some(sb)) = (sa, sb) else {
        return Err("fn has no PDG".into());
    };
    serde_json::to_string(&trace(&sa, &sb, THRESHOLD_MILLI)).map_err(|e| e.to_string())
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    match run(&args) {
        Ok(json) => println!("{json}"),
        Err(e) => {
            eprintln!("wl_trace: {e}");
            exit(2);
        }
    }
}
