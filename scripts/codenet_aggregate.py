#!/usr/bin/env python3
"""Mean and spread over seeds of the codenet_eval.py results (no deps).

Reads eval/codenet/s<seed>/results_<split>.csv, prints per (config, level, tier, scope) the mean
rate, the sample sd over seeds, min..max, and the pooled flagged/pairs. With --baseline it also
prints `delta` (mean rate minus the baseline's) and `beyond`: |delta| > the larger of the two
spreads (max-min over seeds), the plan's "more than the seed spread" test.

Usage: scripts/codenet_aggregate.py --seeds 1,2,3 [--split dev] [--baseline none] [--level fn] [--csv out.csv]
"""
import argparse, csv, os
from statistics import mean, stdev

W = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))


def load(root, seeds, split):
    """{(config, level, tier, scope): [(pairs, flagged) per seed]}."""
    cells = {}
    for seed in seeds:
        with open(f"{root}/s{seed}/results_{split}.csv", newline="") as f:
            for r in csv.DictReader(f):
                key = (r["config"], r["level"], r["tier"], r["scope"])
                cells.setdefault(key, []).append((int(r["pairs"]), int(r["flagged"])))
    return cells


def summarize(per_seed):
    rates = [h / n if n else 0.0 for n, h in per_seed]
    return {"mean": mean(rates), "sd": stdev(rates) if len(rates) > 1 else 0.0, "lo": min(rates), "hi": max(rates),
            "pairs": sum(n for n, _ in per_seed), "flagged": sum(h for _, h in per_seed)}


def rows(cells, baseline):
    out = []
    for key, per_seed in sorted(cells.items()):
        s = summarize(per_seed)
        base = cells.get((baseline, *key[1:])) if baseline else None
        if base and key[2] != "shared_helper":
            b = summarize(base)
            s["delta"] = s["mean"] - b["mean"]
            s["beyond"] = abs(s["delta"]) > max(s["hi"] - s["lo"], b["hi"] - b["lo"])
        out.append((key, s))
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", default=f"{W}/eval/codenet")
    ap.add_argument("--seeds", default="1,2,3")
    ap.add_argument("--split", default="dev")
    ap.add_argument("--baseline", default="")
    ap.add_argument("--level", default="", help="module | fn (default both)")
    ap.add_argument("--csv", default="")
    a = ap.parse_args()
    table = [(k, s) for k, s in rows(load(a.root, a.seeds.split(","), a.split), a.baseline)
             if a.level in ("", k[1]) and k[2] != "shared_helper"]  # counts, not rates: see results_<split>.csv
    print(f"{'config':16} {'level':6} {'tier':20} {'scope':10} {'mean':>7} {'sd':>7} {'min':>7} {'max':>7} {'flagged/pairs':>14} {'delta':>8} beyond")
    for (c, lv, t, sc), s in table:
        d = f"{s['delta']:+.4f}" if "delta" in s else ""
        print(f"{c:16} {lv:6} {t:20} {sc:10} {s['mean']:7.4f} {s['sd']:7.4f} {s['lo']:7.4f} {s['hi']:7.4f} "
              f"{s['flagged']:>6}/{s['pairs']:<7} {d:>8} {s.get('beyond', '')}")
    if a.csv:
        with open(a.csv, "w", newline="") as f:
            w = csv.writer(f)
            w.writerow(["config", "level", "tier", "scope", "mean", "sd", "min", "max", "flagged", "pairs", "delta", "beyond"])
            w.writerows([[*k, *(round(s[x], 5) for x in ("mean", "sd", "lo", "hi")), s["flagged"], s["pairs"],
                          round(s.get("delta", 0), 5), s.get("beyond", "")] for k, s in table])


if __name__ == "__main__":
    main()
