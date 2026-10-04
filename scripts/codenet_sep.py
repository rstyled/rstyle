#!/usr/bin/env python3
"""Does canonicalization separate edited copies from unrelated functions better? (no deps, no hand labels)

Positives: `fn_resubmit` (same user, same fn name+arity: edited copies). Negatives: `fn_cross_problem`
(different problems, different names), sampled with a seeded hash order. `fn_same_problem` and
`fn_identical_problem` stay unlabeled. For each rule set the Rust example `norm_sim` returns WL
similarity and alignment coverage on raw and canonical graphs of the same pairs, so everything is paired.

Reported per rule set:
  AUC          P(score(positive) > score(negative)), raw vs canonical, with a paired bootstrap CI of the
               difference (resamples positives and negatives; pairs of one shard are correlated, so the CI is a
               little narrow). Three views: WL and coverage on pairs comparable under both, and coverage over all
               pairs (not comparable = 0, what the miner sees).
  shift        per tier, share of comparable pairs whose WL changed and the mean change (milli).
  excess       positive-unlabeled estimate for the unlabeled tiers: with t = the 99th percentile of raw coverage
               on negatives, excess = (f_tier - f_neg) / (1 - f_neg), f = share above t. Raw vs canonical.

Reads facts left in /tmp/cn_m_<seed dir>_<shard> by codenet_eval.py (run it for the split first).
Usage: scripts/codenet_sep.py --seeds 1,2,3 [--split dev] [--rules all,case,ctor] [--neg-cap 4000] [--boot 200]
"""
import argparse, bisect, csv, glob, hashlib, os, random, subprocess, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import codenet_score as cs

W = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
POS, NEG = "fn_resubmit", "fn_cross_problem"
UNLABELED = ("fn_same_problem", "fn_identical_problem")
COLS = ("cmp_raw", "wl_raw", "cov_raw", "n_raw", "cmp_canon", "wl_canon", "cov_canon", "n_canon")


def auc(pos, neg):
    """P(pos > neg) + 0.5 P(tie), via bisect on the sorted negatives."""
    if not pos or not neg:
        return float("nan")
    s = sorted(neg)
    return sum(bisect.bisect_left(s, p) + 0.5 * (bisect.bisect_right(s, p) - bisect.bisect_left(s, p))
               for p in pos) / (len(pos) * len(s))


def paired_ci(pos, neg, raw_of, canon_of, boot, seed):
    """(auc_raw, auc_canon, lo, hi): 95% percentile bootstrap of auc_canon - auc_raw."""
    rng = random.Random(seed)
    diffs = []
    for _ in range(boot):
        p = [pos[rng.randrange(len(pos))] for _ in pos]
        n = [neg[rng.randrange(len(neg))] for _ in neg]
        diffs.append(auc([canon_of(x) for x in p], [canon_of(x) for x in n])
                     - auc([raw_of(x) for x in p], [raw_of(x) for x in n]))
    diffs.sort()
    lo, hi = diffs[int(0.025 * len(diffs))], diffs[min(len(diffs) - 1, int(0.975 * len(diffs)))]
    return (auc([raw_of(x) for x in pos], [raw_of(x) for x in neg]),
            auc([canon_of(x) for x in pos], [canon_of(x) for x in neg]), lo, hi)


def unmatched(wl, nodes):
    """Nodes of the larger graph that find no partner: n * (1 - sim). Unlike the ratio it does not fall when a rule
    contracts structure both sides share (k common nodes gone: (i - k) / (n - k) < i / n)."""
    return nodes * (1000 - wl) / 1000


def percentile(values, q):
    s = sorted(values)
    return s[min(len(s) - 1, int(q * len(s)))] if s else 0


def excess(tier, neg, thr, key):
    """Positive-unlabeled share above the negatives' 99th percentile: (f_tier - f_neg) / (1 - f_neg)."""
    f = lambda xs: sum(key(x) > thr for x in xs) / len(xs) if xs else 0.0
    fn = f(neg)
    return (f(tier) - fn) / (1 - fn) if fn < 1 else 0.0


def sample_negatives(pairs, cap, seed):
    pos = [p for p in pairs if p[2] != NEG]
    neg = sorted((p for p in pairs if p[2] == NEG), key=lambda p: hashlib.sha1(f"{seed}:{p[0]}|{p[1]}".encode()).hexdigest())
    return pos + neg[:cap]


def norm_sim(rules, pairs, files):
    stdin = "".join(f"{a}\t{b}\n" for a, b, _ in pairs)
    p = subprocess.run([f"{W}/target/debug/examples/norm_sim", rules, *files], input=stdin, capture_output=True, text=True)
    if p.returncode != 0:
        sys.exit(f"norm_sim failed: {p.stderr[-300:]}")
    return [line.split("\t") for line in p.stdout.splitlines()]


def collect(root, seeds, split, rules, cap):
    """tier -> [dict of COLS as ints] over all shards of all seeds."""
    out = {}
    for seed in seeds:
        sdir = f"{root}/s{seed}"
        rows = list(csv.DictReader(open(f"{sdir}/pairs.csv", newline="")))
        for shard in sorted({r["shard"] for r in rows if r["split"] == split}):
            tag = f"s{seed}_{shard}"
            files = sorted(glob.glob(f"/tmp/cn_m_{tag}/rstyle/facts-sem/*.json"))
            if not files:
                sys.exit(f"no facts for {tag}: run codenet_eval.py --out {sdir} --split {split} first")
            table = cs.fn_table([f for p in files for f in __import__("json").load(open(p))])
            tiers = {frozenset([r["a"], r["b"]]): r["tier"] for r in rows if r["shard"] == shard}
            pairs = sample_negatives(cs.fn_pairs(table, tiers), cap, f"{seed}:{shard}")
            for (a, b, tier), vals in zip(pairs, norm_sim(rules, pairs, files)):
                if vals != ["NA"]:
                    out.setdefault(tier, []).append(dict(zip(COLS, map(int, vals))))
    return out


def views(data):
    """name -> (restrict to comparable under both?, raw score, canon score)."""
    both = lambda r: r["cmp_raw"] and r["cmp_canon"]
    return {"wl|cmp": (both, lambda r: r["wl_raw"], lambda r: r["wl_canon"]),
            "cov|cmp": (both, lambda r: r["cov_raw"], lambda r: r["cov_canon"]),
            "cov|all": (lambda r: True, lambda r: r["cov_raw"], lambda r: r["cov_canon"])}


def report(rules, data, boot):
    print(f"\n== rules: {rules}   pairs: " + ", ".join(f"{t.replace('fn_', '')}={len(v)}" for t, v in sorted(data.items())))
    for name, (keep, raw, canon) in views(data).items():
        pos, neg = [r for r in data[POS] if keep(r)], [r for r in data[NEG] if keep(r)]
        a0, a1, lo, hi = paired_ci(pos, neg, raw, canon, boot, 7)
        print(f"AUC {name:8} raw {a0:.4f} canon {a1:.4f} diff {a1 - a0:+.4f} [{lo:+.4f}, {hi:+.4f}]  (pos {len(pos)}, neg {len(neg)})")
    for tier in (POS, *UNLABELED):
        both = [r for r in data[tier] if r["cmp_raw"] and r["cmp_canon"]]
        changed = [r for r in both if r["wl_canon"] != r["wl_raw"]]
        d = sum(r["wl_canon"] - r["wl_raw"] for r in both) / len(both) if both else 0
        print(f"shift {tier:22} comparable {len(both):6} changed {len(changed) / max(len(both), 1):.3f} mean dWL {d:+.1f} milli")
    thr = percentile([r["cov_raw"] for r in data[NEG]], 0.99)
    for tier in (POS, *UNLABELED):
        both = [r for r in data[tier] if r["cmp_raw"] and r["cmp_canon"]]
        du = [unmatched(r["wl_canon"], r["n_canon"]) - unmatched(r["wl_raw"], r["n_raw"]) for r in both]
        less, more = sum(d < -0.5 for d in du), sum(d > 0.5 for d in du)
        print(f"unmatched nodes {tier:22} fewer {less:5} more {more:5} same {len(du) - less - more:5} mean {sum(du) / max(len(du), 1):+.2f}")
    for tier in (POS, *UNLABELED):
        e0 = excess(data[tier], data[NEG], thr, lambda r: r["cov_raw"])
        e1 = excess(data[tier], data[NEG], thr, lambda r: r["cov_canon"])
        print(f"excess {tier:22} (cov > {thr}) raw {e0:.4f} canon {e1:.4f}")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--root", default=f"{W}/eval/codenet")
    ap.add_argument("--seeds", default="1,2,3")
    ap.add_argument("--split", default="dev", choices=["dev", "test"])
    ap.add_argument("--rules", default="all,case,ctor,counter,pred,exit")
    ap.add_argument("--neg-cap", type=int, default=4000, help="negatives kept per shard (seeded)")
    ap.add_argument("--boot", type=int, default=200)
    a = ap.parse_args()
    subprocess.run(["cargo", "build", "-q", "--example", "norm_sim", "-p", "rstyle-core"], cwd=W, check=True,
                   env={k: v for k, v in os.environ.items() if k != "CARGO_TARGET_DIR"})
    for rules in a.rules.split(","):
        report(rules, collect(a.root, a.seeds.split(","), a.split, rules, a.neg_cap), a.boot)


if __name__ == "__main__":
    main()
