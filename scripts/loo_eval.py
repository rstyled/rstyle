#!/usr/bin/env python3
"""Leave-one-project-out evaluation of the patterns miner (no deps).

For each (threshold, min_coverage) grid point the miner runs once per project; candidates are
matched to rust_refactoring_examples/labels.csv. For each held-out project, the grid point that
maximizes F1 on the OTHER projects is picked (ties: higher threshold, higher coverage), then
scored on the held-out one. Writes eval/loo/{grid,loo}.csv.

Usage: scripts/loo_eval.py [--refresh] [--normalize RULES]   (miner results are cached in /tmp/rx_loo)
With --normalize the grid/noise/loo CSVs go to eval/loo_norm_<rules>/ so the baseline stays pinned.
"""
import csv, json, os, subprocess, sys
from compiler_patterns import evaluation
from concurrent.futures import ThreadPoolExecutor

W = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
EX = f"{W}/rust_refactoring_examples"
EXTRA = []  # extra miner flags (set by scripts/normalize_ablation.py)
NORM = sys.argv[sys.argv.index("--normalize") + 1] if "--normalize" in sys.argv else ""
OUT = f"{W}/eval/loo" if not NORM else f"{W}/eval/loo_norm_{NORM.replace(',', '+')}"
CACHE = "/tmp/rx_loo"
PROJECTS = ["telemetry_workspace", "backup_sync", "queue_worker", "feature_gate"]
SYNTH = ["spelling_variants"]  # same behaviour, different spellings; reported apart, not in LOO
NOISE = {"tarpc": {}, "mundane": {"RUSTFLAGS": "--cap-lints warn"}, "argh": {}}
NOISE_DIR = "/tmp/real"  # clones; noise = candidate count (no labels), reported per grid point
THRESHOLDS = [200, 300, 400, 500, 600, 700, 800, 900]
COVERAGES = [0, 400, 600, 800]


def run_miner(project, thr, cov, refresh):
    extra = NOISE.get(project)
    cwd = f"{NOISE_DIR}/{project}" if extra is not None else f"{EX}/{project}"
    stamp = int(os.path.getmtime(f"{W}/target/debug/cargo-rstyle"))  # a rebuilt miner never reads stale results
    cache = f"{CACHE}/{project}_{thr}_{cov}{'_' + NORM if NORM else ''}{'_' + '_'.join(EXTRA) if EXTRA else ''}_{stamp}.json"
    if not refresh and os.path.exists(cache):
        return evaluation(json.load(open(cache)))
    env = dict(os.environ, **(extra or {}), CARGO_TARGET_DIR=f"/tmp/rx_t_{project}",
               RSTYLE_DRIVER=f"{W}/target/debug/rstyle-driver")
    cmd = [f"{W}/target/debug/cargo-rstyle", "rstyle", "--format", "json", "patterns",
           "--all-targets", "--threshold", str(thr), "--min-coverage", str(cov)]
    cmd += ["--normalize", NORM] if NORM else []
    cmd += EXTRA
    p = subprocess.run(cmd, cwd=cwd, env=env, capture_output=True, text=True)
    if p.returncode not in (0, 1) or not p.stdout.strip():
        sys.exit(f"miner failed on {project} {thr}/{cov}: {p.stderr[-500:]}")
    os.makedirs(CACHE, exist_ok=True)
    open(cache, "w").write(p.stdout)
    return evaluation(json.loads(p.stdout))


def site_key(s):
    path, _, tail = s.partition("::")
    return path, tail.split(" (arm")[0]


RANK = {"no": 0, "maybe": 1, "yes": 2}


def stronger(old, new):
    """Keep the stronger label: a pair that is both decoy and positive (Q3 vs Q4) counts as positive."""
    return new if old is None or RANK[new] > RANK[old] else old


def load_labels():
    """pair -> 'yes' | 'maybe' | 'no' per project. 'maybe' counts as positive for F1 (yes+maybe)."""
    pairs = {p: {} for p in PROJECTS + SYNTH}
    for r in csv.DictReader(open(f"{EX}/labels.csv")):
        if not r["site_b"] or r["label_id"].startswith("X"):
            continue
        a, b = site_key(r["site_a"]), site_key(r["site_b"])
        k = frozenset([a, b])
        d = pairs[r["project"]]
        d[k] = stronger(d.get(k), r["should_refactor"])
        d.setdefault("ids", {}).setdefault(k, []).append(r["label_id"])
    return pairs


def cand_hits(cand_sites, pair):
    def has(site):
        path, tail = site
        return any(path.endswith(c["path"]) and c["id"].endswith("::" + tail) for c in cand_sites)
    return all(has(s) for s in pair)


def score(project, report, labels):
    flagged = [c["sites"] for c in report.get("candidates", [])]
    flagged += [g.get("sites", []) for g in report.get("signature_groups", [])]
    pairs = {k: v for k, v in labels[project].items() if k != "ids"}
    tp_pairs, fp_decoy, unlabeled = set(), 0, 0
    for sites in flagged:
        hit = [(k, v) for k, v in pairs.items() if cand_hits(sites, k)]
        if any(v != "no" for _, v in hit):
            tp_pairs |= {k for k, v in hit if v != "no"}
        elif hit:
            fp_decoy += 1
        else:
            unlabeled += 1
    npos = sum(1 for v in pairs.values() if v != "no")
    nyes = sum(1 for v in pairs.values() if v == "yes")
    tp_yes = sum(1 for k in tp_pairs if pairs[k] == "yes")
    return dict(tp=len(tp_pairs), fn=npos - len(tp_pairs), fp_decoy=fp_decoy, fp_unlabeled=unlabeled,
                tp_yes=tp_yes, n_yes=nyes)


def prf(tp, fp, fn):
    p = tp / (tp + fp) if tp + fp else 0.0
    r = tp / (tp + fn) if tp + fn else 0.0
    return p, r, (2 * p * r / (p + r) if p + r else 0.0)


def main():
    refresh = "--refresh" in sys.argv
    labels = load_labels()
    subprocess.run(["cargo", "build", "-q", "--workspace"], cwd=W, check=True,
                   env={k: v for k, v in os.environ.items() if k != "CARGO_TARGET_DIR"})
    grid = [(t, c) for t in THRESHOLDS for c in COVERAGES]
    jobs = [(p, t, c) for p in PROJECTS + SYNTH + list(NOISE) for (t, c) in grid]
    with ThreadPoolExecutor(4) as ex:
        reports = dict(zip(jobs, ex.map(lambda j: run_miner(*j, refresh), jobs)))
    res = {j: score(j[0], reports[j], labels) for j in jobs if j[0] in PROJECTS + SYNTH}
    noise = {g: {n: len(reports[(n, *g)].get("candidates", [])) for n in NOISE} for g in grid}

    os.makedirs(OUT, exist_ok=True)
    with open(f"{OUT}/grid.csv", "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["project", "threshold", "min_coverage", "tp", "fn", "fp_decoy", "fp_unlabeled"])
        for (p, t, c), s in res.items():
            w.writerow([p, t, c, s["tp"], s["fn"], s["fp_decoy"], s["fp_unlabeled"]])

    with open(f"{OUT}/noise.csv", "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["threshold", "min_coverage", *NOISE, "total"])
        for g, d in noise.items():
            w.writerow([*g, *d.values(), sum(d.values())])

    def agg(ps, t, c):
        tp = sum(res[(p, t, c)]["tp"] for p in ps)
        fn = sum(res[(p, t, c)]["fn"] for p in ps)
        fp = sum(res[(p, t, c)]["fp_decoy"] + res[(p, t, c)]["fp_unlabeled"] for p in ps)
        return tp, fp, fn

    rows = []
    for held in PROJECTS:
        train = [p for p in PROJECTS if p != held]
        best = max(grid, key=lambda g: (prf(*agg(train, *g))[2], g[0], g[1]))
        ttp, tfp, tfn = agg(train, *best)
        htp, hfp, hfn = agg([held], *best)
        orc = max(grid, key=lambda g: (prf(*agg([held], *g))[2], g[0], g[1]))
        otp, ofp, ofn = agg([held], *orc)
        rows.append([held, best[0], best[1], *map(lambda x: round(x, 3), prf(ttp, tfp, tfn)),
                     htp, hfp, hfn, *map(lambda x: round(x, 3), prf(htp, hfp, hfn)),
                     orc[0], orc[1], round(prf(otp, ofp, ofn)[2], 3)])
    hdr = ["held_out", "thr", "cov", "train_p", "train_r", "train_f1", "test_tp", "test_fp",
           "test_fn", "test_p", "test_r", "test_f1", "oracle_thr", "oracle_cov", "oracle_f1"]
    with open(f"{OUT}/loo.csv", "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(hdr)
        w.writerows(rows)
    base = (600, 600)
    print(" ".join(f"{h:>14}" for h in hdr))
    for r in rows:
        print(" ".join(f"{x!s:>14}" for x in r))
    print("\ndefault (600/600) per project: tp fp fn")
    for p in PROJECTS:
        print(f"  {p:22}", agg([p], *base))
    tot = [sum(r[i] for r in rows) for i in (6, 7, 8)]
    print("LOO pooled test tp/fp/fn:", tot, "p/r/f1:", [round(x, 3) for x in prf(*tot)])
    print("\nnoise (candidates per real crate), thr cov:", list(NOISE))
    for g, d in noise.items():
        print(f"  {g[0]:4} {g[1]:4}", list(d.values()), sum(d.values()))
    for p in SYNTH:
        print(f"synthetic {p} at default (tp fp fn):", agg([p], *base))
    ty = sum(res[(p, *base)]["tp_yes"] for p in PROJECTS)
    ny = sum(res[(p, *base)]["n_yes"] for p in PROJECTS)
    tp, _, fn = agg(PROJECTS, *base)
    print(f"default recall yes-only : {ty}/{ny}   yes+maybe: {tp}/{tp + fn}")
    print("default pooled          :", agg(PROJECTS, *base),
          [round(x, 3) for x in prf(*agg(PROJECTS, *base))])


if __name__ == "__main__":
    main()
