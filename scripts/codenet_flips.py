#!/usr/bin/env python3
"""Paired flag flips between two miner configs on the same function pairs (no deps).

For every labeled fn pair, flagged by A only / B only / both / neither. The discordant pairs (A only, B only)
carry all the information about B vs A: an exact two-sided sign test says whether B flags more, or fewer, than
chance. Pairs in one shard share functions, so the p-value is optimistic; read it with the counts.

Usage: scripts/codenet_flips.py --a none --b all+pc650+h4 [--split dev] [--seeds 1,2,3]
"""
import argparse, csv, os, sys
from collections import defaultdict
from math import comb

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import codenet_eval as ev
import codenet_score as cs


def sign_test(only_a, only_b):
    """Exact two-sided binomial p for only_b ~ Bin(only_a + only_b, 1/2)."""
    n, k = only_a + only_b, min(only_a, only_b)
    return min(1.0, 2 * sum(comb(n, i) for i in range(k + 1)) / 2**n) if n else 1.0


def flips(root, seeds, split, config_a, config_b):
    """tier -> [both, only_a, only_b, neither]."""
    cells = defaultdict(lambda: [0, 0, 0, 0])
    for seed in seeds:
        sdir = f"{root}/s{seed}"
        rows = list(csv.DictReader(open(f"{sdir}/pairs.csv", newline="")))
        for shard in sorted({r["shard"] for r in rows if r["split"] == split}):
            tag = f"s{seed}_{shard}"
            flagged = []
            for config in (config_a, config_b):
                report = ev.run_miner(f"{sdir}/shards/{shard}", tag, ev.miner_args(config))
                table = cs.fn_table(ev.load_facts(tag))
                flagged.append(cs.split_candidates(report, table)[1])
            tiers = {frozenset([r["a"], r["b"]]): r["tier"] for r in rows if r["shard"] == shard}
            for x, y, tier in cs.fn_pairs(table, tiers):
                key = frozenset((x, y))
                cells[tier][3 - 2 * (key in flagged[0]) - (key in flagged[1])] += 1
    return cells


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", default=f"{ev.W}/eval/codenet")
    ap.add_argument("--seeds", default="1,2,3")
    ap.add_argument("--split", default="dev", choices=["dev", "test"])
    ap.add_argument("--a", default="none")
    ap.add_argument("--b", required=True)
    a = ap.parse_args()
    print(f"{a.split}: A={a.a}  B={a.b}")
    print(f"{'tier':22} {'both':>6} {'A only':>7} {'B only':>7} {'neither':>8}  sign-test p")
    for tier, (both, oa, ob, nei) in sorted(flips(a.root, a.seeds.split(","), a.split, a.a, a.b).items()):
        print(f"{tier:22} {both:6} {oa:7} {ob:7} {nei:8}  {sign_test(oa, ob):.2g}")


if __name__ == "__main__":
    main()
