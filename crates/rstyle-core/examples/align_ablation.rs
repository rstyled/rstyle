//! `align_ablation <project> <facts.json>...` prints one JSON line per comparable fn pair:
//! WL similarity plus alignment coverage and hole count for every `align::Variant`.
//! Pairs are all comparable pairs of the project's fns (dedup by id), so keep corpora modest.

use rstyle_core::align::{Ctx, Variant, align_variant};
use rstyle_core::facts::{FnFacts, Pdg};
use rstyle_core::wl::{Wl, comparable, similarity_milli};
use std::{collections::BTreeMap, env, fs, process::exit};

const VARIANTS: [(&str, Variant); 6] = [
    ("baseline", Variant::Baseline),
    ("labels_only", Variant::LabelsOnly),
    ("no_final_prune", Variant::NoFinalPrune),
    ("complete_checked", Variant::CompleteChecked),
    ("coarse_to_fine", Variant::CoarseToFine),
    ("combined", Variant::Combined),
];

fn load(paths: &[String]) -> Result<Vec<FnFacts>, String> {
    let mut fns = BTreeMap::new();
    for path in paths {
        let text = fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
        let list: Vec<FnFacts> = serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))?;
        for f in list.into_iter().filter(|f| f.pdg.is_some()) {
            fns.entry(f.id.clone()).or_insert(f);
        }
    }
    Ok(fns.into_values().collect())
}

fn line(project: &str, a: (&FnFacts, &Wl), b: (&FnFacts, &Wl)) -> String {
    let (pa, pb): (&Pdg, &Pdg) = (a.0.pdg.as_ref().unwrap(), b.0.pdg.as_ref().unwrap());
    let ctx = Ctx::new(pa, a.1, pb, b.1);
    let runs: Vec<_> = VARIANTS
        .iter()
        .map(|(name, v)| {
            let al = align_variant(&ctx, *v).0;
            format!("\"{name}\":[{},{}]", al.coverage_milli, al.holes.len())
        })
        .collect();
    format!(
        "{{\"project\":\"{project}\",\"a\":\"{}\",\"b\":\"{}\",\"wl\":{},\"nodes\":[{},{}],{}}}",
        a.0.id,
        b.0.id,
        similarity_milli(a.1, b.1),
        a.1.nodes(),
        b.1.nodes(),
        runs.join(",")
    )
}

fn run(args: &[String]) -> Result<(), String> {
    let [project, files @ ..] = args else {
        return Err("usage: align_ablation <project> <facts.json>...".into());
    };
    let fns = load(files)?;
    let wls: Vec<Wl> = fns
        .iter()
        .map(|f| Wl::new(f.pdg.as_ref().unwrap()))
        .collect();
    for i in 0..fns.len() {
        for j in i + 1..fns.len() {
            if comparable(&wls[i], &wls[j]) {
                println!("{}", line(project, (&fns[i], &wls[i]), (&fns[j], &wls[j])));
            }
        }
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if let Err(e) = run(&args) {
        eprintln!("align_ablation: {e}");
        exit(2);
    }
}
