//! `norm_sim <rules> <facts.json>...` reads `id-a<TAB>id-b` (full fn ids) from stdin and prints one
//! TSV row per pair: `cmp wl cov nmax` for the raw pair, then the same four for the canonical pair (`nmax` = nodes of the larger graph). WL is the similarity
//! in milli, `cov` the alignment coverage in milli (0 when the pair is not comparable: the miner
//! never aligns those). Canonical = `canonicalize(rules)`. Pairs with an unknown fn print `NA`.
//! The CodeNet separation script uses it to ask whether the rules move similarity at all.

use rstyle_core::align::{Ctx, align};
use rstyle_core::facts::FnFacts;
use rstyle_core::normalize::{Rules, canonicalize};
use rstyle_core::wl::{Wl, comparable, similarity_milli};
use std::{collections::HashMap, env, fs, io::Read, process::exit};

struct Graphs {
    raw: Wl,
    canon: Wl,
    raw_pdg: rstyle_core::facts::Pdg,
    canon_pdg: rstyle_core::facts::Pdg,
}

fn load(paths: &[String], rules: &Rules) -> Result<HashMap<String, Graphs>, String> {
    let mut out = HashMap::new();
    for path in paths {
        let text = fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
        let list: Vec<FnFacts> = serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))?;
        for f in list {
            let Some(pdg) = f.pdg else { continue };
            out.entry(f.id).or_insert_with(|| {
                let canon_pdg = canonicalize(&pdg, rules);
                Graphs {
                    raw: Wl::new(&pdg),
                    canon: Wl::new(&canon_pdg),
                    raw_pdg: pdg,
                    canon_pdg,
                }
            });
        }
    }
    Ok(out)
}

/// `cmp wl cov` of one pair of (graph, wl) sides.
fn measure(a: (&rstyle_core::facts::Pdg, &Wl), b: (&rstyle_core::facts::Pdg, &Wl)) -> String {
    let cmp = comparable(a.1, b.1);
    let cov = if cmp {
        align(&Ctx::new(a.0, a.1, b.0, b.1)).coverage_milli
    } else {
        0
    };
    let nmax = a.1.nodes().max(b.1.nodes());
    format!(
        "{}\t{}\t{cov}\t{nmax}",
        u8::from(cmp),
        similarity_milli(a.1, b.1)
    )
}

fn row(graphs: &HashMap<String, Graphs>, line: &str) -> Result<String, String> {
    let (a, b) = line.split_once('\t').ok_or("pair lines are `a<TAB>b`")?;
    let (Some(x), Some(y)) = (graphs.get(a), graphs.get(b.trim())) else {
        return Ok("NA".into());
    };
    let raw = measure((&x.raw_pdg, &x.raw), (&y.raw_pdg, &y.raw));
    let canon = measure((&x.canon_pdg, &x.canon), (&y.canon_pdg, &y.canon));
    Ok(format!("{raw}\t{canon}"))
}

fn run(args: &[String]) -> Result<(), String> {
    let [rules, files @ ..] = args else {
        return Err("usage: norm_sim <rules> <facts.json>... < pairs".into());
    };
    let graphs = load(files, &Rules::parse(rules)?)?;
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .map_err(|e| e.to_string())?;
    let rows: Result<Vec<String>, String> = input
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| row(&graphs, l))
        .collect();
    println!("{}", rows?.join("\n"));
    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if let Err(e) = run(&args) {
        eprintln!("norm_sim: {e}");
        exit(2);
    }
}
