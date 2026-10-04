#!/usr/bin/env python3
"""Per-rule ablation of the canonicalizer at the default thresholds (600/600).

Configs: none, all, all minus each rule, and `all` with --pair-coverage 650 / 600. For each one the
miner runs on the 4 labeled projects, the synthetic project and the noise crates. Writes
eval/normalize/ablation.csv. Needs /tmp/real clones (see loo_eval.py).

Usage: scripts/normalize_ablation.py [config ...]   (no names: all configs, writes eval/normalize/ablation.csv;
with names: only those, writes eval/normalize/ablation_<first>.csv so the pinned file stays intact)
"""
import csv, os, sys
from concurrent.futures import ThreadPoolExecutor

sys.path.insert(0, os.path.dirname(__file__))
import loo_eval as L

RULES = ["ctor", "pred", "case", "exit", "counter", "cmp", "try"]
CONFIGS = [("none", "", [])]
CONFIGS += [("all", ",".join(RULES), [])]
CONFIGS += [(f"all-{r}", ",".join(x for x in RULES if x != r), []) for r in RULES]
CONFIGS += [(f"{r} only", r, []) for r in RULES]
CONFIGS += [("all+pc650", ",".join(RULES), ["--pair-coverage", "650"]),
            ("none+guard", "", ["--single-call-guard"]),
            ("all+guard", ",".join(RULES), ["--single-call-guard"]),
            ("all+pc600", ",".join(RULES), ["--pair-coverage", "600"])]
# Q1 has 4 holes (Type, Method, 2 Literal) and `parameterize` drops clusters over 3 holes.
CONFIGS += [("none+h4", "", ["--max-holes", "4"]),
            ("all+h4", ",".join(RULES), ["--max-holes", "4"]),
            ("all+pc800", ",".join(RULES), ["--pair-coverage", "800"]),
            ("all+pc800+h4", ",".join(RULES), ["--pair-coverage", "800", "--max-holes", "4"]),
            ("all+pc700+h4", ",".join(RULES), ["--pair-coverage", "700", "--max-holes", "4"]),
            ("all+pc650+h4", ",".join(RULES), ["--pair-coverage", "650", "--max-holes", "4"])]
BASE = (600, 600)


def flagged_ids(report, labels, project):
    """Label ids of positive and decoy pairs the report flags."""
    pairs = {k: v for k, v in labels[project].items() if k != "ids"}
    hit = set()
    for c in report.get("candidates", []):
        for k in pairs:
            if L.cand_hits(c["sites"], k):
                hit |= set(labels[project]["ids"][k])
    return hit


def run_config(cfg):
    name, rules, extra = cfg
    L.NORM, L.EXTRA = rules, extra
    out = {}
    for p in L.PROJECTS + L.SYNTH + list(L.NOISE):
        out[p] = L.run_miner(p, *BASE, False)
    return name, out


def main():
    labels = L.load_labels()
    rows, found = [], {}
    names = sys.argv[1:]
    chosen = [c for c in CONFIGS if not names or c[0] in names]
    out = "ablation.csv" if not names else f"ablation_{names[0].replace('+', '_')}.csv"
    for cfg in chosen:  # sequential: run_miner reads module globals
        name, reps = run_config(cfg)
        ex = [L.score(p, reps[p], labels) for p in L.PROJECTS]
        sy = L.score("spelling_variants", reps["spelling_variants"], labels)
        noise = {n: len(reps[n].get("candidates", [])) for n in L.NOISE}
        ids = set().union(*(flagged_ids(reps[p], labels, p) for p in L.PROJECTS + L.SYNTH))
        found[name] = ids
        rows.append([name, cfg[1], " ".join(cfg[2]),
                     sum(s["tp"] for s in ex), sum(s["fp_decoy"] + s["fp_unlabeled"] for s in ex),
                     sum(s["fn"] for s in ex), sum(s["tp_yes"] for s in ex),
                     sy["tp"], sy["fn"], sy["fp_decoy"], *noise.values(), sum(noise.values()),
                     " ".join(sorted(ids))])
    os.makedirs(f"{L.W}/eval/normalize", exist_ok=True)
    hdr = ["config", "rules", "extra", "ex_tp", "ex_fp", "ex_fn", "ex_tp_yes", "synth_tp", "synth_fn",
           "synth_decoy_fp", *L.NOISE, "noise_total", "flagged_labels"]
    with open(f"{L.W}/eval/normalize/{out}", "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(hdr)
        w.writerows(rows)
    for r in rows:
        print(f"{r[0]:14} ex tp/fp/fn {r[3]}/{r[4]}/{r[5]}  synth tp/fn/decoy {r[7]}/{r[8]}/{r[9]}  "
              f"noise {r[10:13]} {r[13]}  {r[14]}")


if __name__ == "__main__":
    main()
