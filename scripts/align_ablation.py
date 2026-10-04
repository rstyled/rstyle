#!/usr/bin/env python3
"""Summarize eval/align/pairs.jsonl (from examples/align_ablation) against labels.csv. No deps.

Writes eval/align/{labeled,variants,selectors}.csv and prints them. Regenerate pairs with
scripts/align_ablation.sh. 'Flagged pairs' are single pairs, not clusters: compare selectors
to each other (WL>=600 is the miner's current pair filter), not to candidate counts.
"""
import csv, json, os, collections

W = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
OUT = f"{W}/eval/align"
EX = ["telemetry_workspace", "backup_sync", "queue_worker", "feature_gate"]
REAL = ["tarpc", "mundane", "argh"]
VARIANTS = ["baseline", "labels_only", "no_final_prune", "complete_checked", "coarse_to_fine", "combined"]


def tail(site):
    return site.partition("::")[2].split(" (arm")[0]


def labeled():
    out = {}
    for r in csv.DictReader(open(f"{W}/rust_refactoring_examples/labels.csv")):
        if r["site_b"] and not r["label_id"].startswith("X"):
            out[(r["project"], tail(r["site_a"]), tail(r["site_b"]))] = (r["label_id"], r["should_refactor"])
    return out


def resolve(pairs, labels):
    hits = {}
    for p in pairs:
        if p["project"] not in EX:
            continue
        for (proj, ta, tb), (lid, want) in labels.items():
            if proj != p["project"]:
                continue
            ok = lambda x, t: x.endswith("::" + t)
            if (ok(p["a"], ta) and ok(p["b"], tb)) or (ok(p["a"], tb) and ok(p["b"], ta)):
                hits[lid] = (p, want)
    return hits


def main():
    pairs = [json.loads(l) for l in open(f"{OUT}/pairs.jsonl")]
    hits = resolve(pairs, labeled())
    with open(f"{OUT}/labeled.csv", "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["label", "want", "wl", *[f"cov_{v}" for v in VARIANTS], *[f"holes_{v}" for v in VARIANTS]])
        for lid, (p, want) in sorted(hits.items()):
            w.writerow([lid, want, p["wl"], *[p[v][0] for v in VARIANTS], *[p[v][1] for v in VARIANTS]])
    print(open(f"{OUT}/labeled.csv").read())

    base = [p["baseline"][0] for p in pairs]
    with open(f"{OUT}/variants.csv", "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["variant", "pairs", "differs_from_baseline", "mean_cov", "cov_ge_600", "cov_ge_800"])
        for v in VARIANTS:
            cv = [p[v][0] for p in pairs]
            w.writerow([v, len(pairs), sum(a != b for a, b in zip(cv, base)), round(sum(cv) / len(cv)),
                        sum(c >= 600 for c in cv), sum(c >= 800 for c in cv)])
    print(open(f"{OUT}/variants.csv").read())

    pos = {l for l, (p, want) in hits.items() if want in ("yes", "maybe")}
    neg = {l for l, (p, want) in hits.items() if want == "no"}
    sel = [("wl>=800", lambda p: p["wl"] >= 800), ("wl>=600", lambda p: p["wl"] >= 600), ("wl>=500", lambda p: p["wl"] >= 500)]
    for v in ("baseline", "coarse_to_fine", "combined"):
        for x in (600, 700, 800, 900):
            sel.append((f"cov[{v}]>={x}", lambda p, v=v, x=x: p[v][0] >= x))
            sel.append((f"wl>=600 or cov[{v}]>={x}", lambda p, v=v, x=x: p["wl"] >= 600 or p[v][0] >= x))
    with open(f"{OUT}/selectors.csv", "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["selector", "tp_labeled", "pos_labeled", "decoys_flagged", "examples_flagged", *[f"{r}_flagged" for r in REAL]])
        for name, fn in sel:
            tp = sum(fn(hits[l][0]) for l in pos)
            dec = sum(fn(hits[l][0]) for l in neg)
            by = collections.Counter(p["project"] for p in pairs if fn(p))
            w.writerow([name, tp, len(pos), dec, sum(by[e] for e in EX), *[by[r] for r in REAL]])
    print(open(f"{OUT}/selectors.csv").read())


if __name__ == "__main__":
    main()
