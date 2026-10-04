#!/usr/bin/env python3
"""List the pairs where canonicalization adds or removes the most unmatched nodes (dev, no deps).

For edited copies (`fn_resubmit`) canonicalization should only add similarity, so every pair where it ADDS unmatched
nodes (graph nodes with no partner; the WL ratio is not used because contracting shared structure lowers it) is a
rule defect to read. Per pair it prints WL raw / canonical for `all` and for each single rule, so the
guilty rule is visible, then both sources.

Usage: scripts/codenet_drops.py [--seeds 1,2,3] [--tier fn_resubmit] [--top 10] [--sign added|removed]
Reads facts left by codenet_eval.py in /tmp/cn_m_s<seed>_<shard>.
"""
import argparse, csv, glob, json, os, subprocess, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import codenet_score as cs
import codenet_sep as sep

SINGLE = ("ctor", "pred", "case", "exit", "counter", "cmp", "try")


def shard_pairs(root, seed, shard, tier):
    rows = list(csv.DictReader(open(f"{root}/s{seed}/pairs.csv", newline="")))
    files = sorted(glob.glob(f"/tmp/cn_m_s{seed}_{shard}/rstyle/facts-sem/*.json"))
    facts = {}
    for p in files:
        for f in json.load(open(p)):
            facts.setdefault(f["id"], f)
    table = cs.fn_table(list(facts.values()))
    tiers = {frozenset([r["a"], r["b"]]): r["tier"] for r in rows if r["shard"] == shard}
    return [p for p in cs.fn_pairs(table, tiers) if p[2] == tier], files, facts


def measure(pairs, files):
    """{rules: [row dict (sep.COLS) or None]} for `all` and each single rule."""
    return {r: [dict(zip(sep.COLS, map(int, v))) if v != ["NA"] else None for v in sep.norm_sim(r, pairs, files)]
            for r in ("all", *SINGLE)}


def gap(row):
    """Change in unmatched nodes (negative = the rule removed differences, positive = it added some)."""
    return sep.unmatched(row["wl_canon"], row["n_canon"]) - sep.unmatched(row["wl_raw"], row["n_raw"])


def collect(root, seeds, split, tier):
    out = []
    for seed in seeds:
        shards = sorted({r["shard"] for r in csv.DictReader(open(f"{root}/s{seed}/pairs.csv")) if r["split"] == split})
        for shard in shards:
            pairs, files, facts = shard_pairs(root, seed, shard, tier)
            if not pairs:
                continue
            by_rule = measure(pairs, files)
            for i, (a, b, _) in enumerate(pairs):
                m = by_rule["all"][i]
                if m and m["cmp_raw"] and m["cmp_canon"]:
                    out.append({"a": a, "b": b, "seed": seed, "d": gap(m), "all": m,
                                "rules": {r: gap(by_rule[r][i]) for r in SINGLE if by_rule[r][i]},
                                "src": (facts[a]["source"], facts[b]["source"])})
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--root", default=f"{sep.W}/eval/codenet")
    ap.add_argument("--seeds", default="1,2,3")
    ap.add_argument("--tier", default="fn_resubmit")
    ap.add_argument("--top", type=int, default=10)
    ap.add_argument("--sign", default="added", choices=["added", "removed"])
    a = ap.parse_args()
    found = collect(a.root, a.seeds.split(","), "dev", a.tier)
    found.sort(key=lambda r: -r["d"] if a.sign == "added" else r["d"])
    fewer = sum(r["d"] < -0.5 for r in found)
    more = sum(r["d"] > 0.5 for r in found)
    print(f"{a.tier}: {len(found)} comparable pairs, differences removed {fewer}, added {more}, same {len(found) - fewer - more}")
    by = {}
    for r in found:
        for rule, d in r["rules"].items():
            by.setdefault(rule, [0, 0])[d > 0.5] += abs(d) > 0.5
    print("by rule alone (removed, added):", {k: tuple(v) for k, v in by.items()})
    for r in found[: a.top]:
        print(f"\n=== seed {r['seed']} unmatched {r['d']:+.1f} nodes (WL {r['all']['wl_raw']} -> {r['all']['wl_canon']}, "
              f"nodes {r['all']['n_raw']} -> {r['all']['n_canon']})  {r['a']} | {r['b']}")
        print("by rule:", {k: v for k, v in r["rules"].items() if v})
        for s in r["src"]:
            print("---\n" + s)


if __name__ == "__main__":
    main()
