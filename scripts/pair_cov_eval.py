#!/usr/bin/env python3
"""Candidate-level test of selecting pairs by alignment coverage (patterns --pair-coverage).

For each (WL threshold, pair coverage) config: pooled TP/FN/FP over the 4 labeled examples and
candidate counts on tarpc/mundane/argh. thr=1001 means WL never selects (coverage only).
Writes eval/align/pair_cov.csv. Results are cached in /tmp/rx_pc.
"""
import csv, json, os, subprocess, sys
from concurrent.futures import ThreadPoolExecutor

sys.path.insert(0, os.path.dirname(__file__))
import loo_eval as L

CACHE = "/tmp/rx_pc"
CONFIGS = [(t, pc) for t in (800, 600, 1001) for pc in (None, 700, 650, 600)]
MIN_COV = 600


def run(project, thr, pc, refresh):
    cache = f"{CACHE}/{project}_{thr}_{pc}.json"
    if not refresh and os.path.exists(cache):
        return json.load(open(cache))
    extra = L.NOISE.get(project)
    cwd = f"{L.NOISE_DIR}/{project}" if extra is not None else f"{L.EX}/{project}"
    env = dict(os.environ, **(extra or {}), CARGO_TARGET_DIR=f"/tmp/rx_t_{project}",
               RSTYLE_DRIVER=f"{L.W}/target/debug/rstyle-driver")
    cmd = [f"{L.W}/target/debug/cargo-rstyle", "rstyle", "--format", "json", "patterns", "--all-targets",
           "--threshold", str(thr), "--min-coverage", str(MIN_COV)]
    if pc is not None:
        cmd += ["--pair-coverage", str(pc)]
    p = subprocess.run(cmd, cwd=cwd, env=env, capture_output=True, text=True)
    if not p.stdout.strip():
        sys.exit(f"miner failed {project} {thr}/{pc}: {p.stderr[-400:]}")
    os.makedirs(CACHE, exist_ok=True)
    open(cache, "w").write(p.stdout)
    return json.loads(p.stdout)


def main():
    refresh = "--refresh" in sys.argv
    labels = L.load_labels()
    subprocess.run(["cargo", "build", "-q", "--workspace"], cwd=L.W, check=True,
                   env={k: v for k, v in os.environ.items() if k != "CARGO_TARGET_DIR"})
    projects = L.PROJECTS + list(L.NOISE)
    jobs = [(p, t, pc) for p in projects for (t, pc) in CONFIGS]
    with ThreadPoolExecutor(4) as ex:
        reports = dict(zip(jobs, ex.map(lambda j: run(*j, refresh), jobs)))
    rows = []
    for t, pc in CONFIGS:
        sc = [L.score(p, reports[(p, t, pc)], labels) for p in L.PROJECTS]
        tp, fn = sum(s["tp"] for s in sc), sum(s["fn"] for s in sc)
        fpd, fpu = sum(s["fp_decoy"] for s in sc), sum(s["fp_unlabeled"] for s in sc)
        noise = [len(reports[(n, t, pc)].get("candidates", [])) for n in L.NOISE]
        rows.append([t, pc if pc is not None else "off", tp, fn, fpd, fpu, *noise, sum(noise)])
    hdr = ["wl_threshold", "pair_coverage", "tp", "fn", "fp_decoy", "fp_unlabeled", *L.NOISE, "noise_total"]
    os.makedirs(L.OUT.replace("loo", "align"), exist_ok=True)
    with open(f"{L.OUT.replace('loo', 'align')}/pair_cov.csv", "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(hdr)
        w.writerows(rows)
    print(" ".join(f"{h:>14}" for h in hdr))
    for r in rows:
        print(" ".join(f"{x!s:>14}" for x in r))


if __name__ == "__main__":
    main()
