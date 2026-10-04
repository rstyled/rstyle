#!/usr/bin/env python3
"""Run the patterns miner on the CodeNet shards from codenet_sample.py and score it per label tier (no deps).

Two levels (column `level` of results_<split>.csv):
  module  a submission pair is flagged when some candidate has sites in both (sanity column; shared
          stdin helpers credit it, so do not read it as recall)
  fn      function pairs (scripts/codenet_score.py): same (name, arity) across submissions of one problem
          (`fn_resubmit`, `fn_same_problem`) or across identical problems (`fn_identical_problem`) are
          positives; different names across unrelated problems (`fn_cross_problem`) are negatives. IO
          helpers are left out; a candidate with < 2 non-IO sites from different submissions goes to
          `shared_helper` (pairs = such candidates, flagged = module pairs only they credit, no rate).
Column `scope`: `all` pairs, or only `comparable` ones (both graphs pass wl::comparable under the config).
`rate` is flagged / pairs: recall on positives, false-positive rate on `*cross_problem`.

Splits: dev (default); test and holdout need an explicit flag. test: once per final config (see scripts/README.md).
holdout: the untouched reserve (written by `codenet_sample.py --emit-holdout`), needs --confirm-holdout.

Usage: scripts/codenet_eval.py [--out eval/codenet/s1] [--configs none,all,all+pc650]
Config names: `none`, `all`, `all-<rule>` (ablation), a rule list (`case,counter`), modifiers `+pc<N>`
(pair coverage), `+h<N>` (max holes), `+guard`.
"""
import argparse, csv, glob, json, os, subprocess, sys
from collections import defaultdict
from itertools import combinations

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import codenet_score as cs

W = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
DEFAULT = "none,all,all+pc650"
RULES = ["ctor", "pred", "case", "exit", "counter", "cmp", "try"]
NO_CARGO_TARGET = {k: v for k, v in os.environ.items() if k != "CARGO_TARGET_DIR"}
HEADER = ["config", "split", "level", "tier", "scope", "pairs", "flagged", "rate"]


def rules_of(base):
    """Config base -> rule list for `--normalize` / norm_comparable ('' = none): `all-case` = all but case."""
    if base == "none":
        return ""
    if base.startswith("all-"):
        out = base[4:].split("-")
        unknown = [r for r in out if r not in RULES]
        if unknown:
            sys.exit(f"unknown rule {unknown} in `{base}`")
        return ",".join(r for r in RULES if r not in out)
    return base


def miner_args(config):
    """'all+pc650+guard' -> ['--normalize', 'all', '--pair-coverage', '650', '--single-call-guard']."""
    base, *mods = config.split("+")
    args = [] if base == "none" else ["--normalize", rules_of(base)]
    for m in mods:
        if m == "guard":
            args.append("--single-call-guard")
        elif m.startswith("pc"):
            args += ["--pair-coverage", m[2:]]
        elif m.startswith("h") and m[1:].isdigit():
            args += ["--max-holes", m[1:]]
        else:
            sys.exit(f"unknown config modifier `{m}`")
    return args


def run_miner(shard_dir, name, args):
    env = dict(os.environ, CARGO_TARGET_DIR=f"/tmp/cn_m_{name}", RSTYLE_DRIVER=f"{W}/target/debug/rstyle-driver")
    cmd = [f"{W}/target/debug/cargo-rstyle", "rstyle", "--format", "json", "patterns", "--all-targets", *args]
    p = subprocess.run(cmd, cwd=shard_dir, env=env, capture_output=True, text=True)
    if p.returncode not in (0, 1) or not p.stdout.strip():
        sys.exit(f"miner failed on {shard_dir}: {p.stderr[-400:]}")
    return json.loads(p.stdout)


def facts_files(shard):
    return sorted(glob.glob(f"/tmp/cn_m_{shard}/rstyle/facts-sem/*.json"))


def load_facts(shard):
    return [f for path in facts_files(shard) for f in json.load(open(path))]


def comparable_flags(pairs, rules, files):
    """One 0/1 per (a, b, tier) pair: do both graphs pass wl::comparable after `rules`?"""
    if not pairs:
        return []
    stdin = "".join(f"{a}\t{b}\n" for a, b, _ in pairs)
    cmd = [f"{W}/target/debug/examples/norm_comparable", rules, *files]
    p = subprocess.run(cmd, input=stdin, capture_output=True, text=True)
    if p.returncode != 0:
        sys.exit(f"norm_comparable failed: {p.stderr[-300:]}")
    return [line == "1" for line in p.stdout.split()]


def load_pairs(out):
    by_shard = defaultdict(list)
    with open(f"{out}/pairs.csv", newline="") as f:
        for r in csv.DictReader(f):
            by_shard[r["shard"]].append(r)
    return by_shard


def module_score(rows, flagged):
    """{(tier, 'all'): [pairs, flagged]} over module pairs."""
    table = defaultdict(lambda: [0, 0])
    for r in rows:
        cell = table[(r["tier"], "all")]
        cell[0] += 1
        cell[1] += frozenset([r["a"], r["b"]]) in flagged
    return table


def add(total, key, cell):
    total[key][0] += cell[0]
    total[key][1] += cell[1]


def score_shard(shard_dir, rows, config):
    """(module cells, fn cells, shared_helper [candidates, module pairs]) of one config on one shard.

    The scratch dir /tmp/cn_m_<seed dir>_<shard> keeps seeds apart (shard names repeat across seeds)."""
    shard = f"{os.path.basename(os.path.dirname(os.path.dirname(shard_dir)))}_{os.path.basename(shard_dir)}"
    report = run_miner(shard_dir, shard, miner_args(config))
    table = cs.fn_table(load_facts(shard))
    module_tier = {frozenset([r["a"], r["b"]]): r["tier"] for r in rows}
    mod_flagged, fn_flagged, (shared_n, shared_pairs) = cs.split_candidates(report, table)
    pairs = cs.fn_pairs(table, module_tier)
    ok = comparable_flags(pairs, rules_of(config.split("+")[0]), facts_files(shard))
    return module_score(rows, mod_flagged), cs.tally(pairs, fn_flagged, ok), [shared_n, len(shared_pairs)]


def evaluate(out, split, configs):
    by_shard = {k: v for k, v in load_pairs(out).items() if k.startswith(split + "_")}
    rows = []
    for config in configs:
        mod, fn, shared = defaultdict(lambda: [0, 0]), defaultdict(lambda: [0, 0]), [0, 0]
        for shard, pairs in by_shard.items():
            m, f, sh = score_shard(f"{out}/shards/{shard}", pairs, config)
            for k, v in m.items():
                add(mod, k, v)
            for k, v in f.items():
                add(fn, k, v)
            shared = [shared[0] + sh[0], shared[1] + sh[1]]
        rows += [[config, split, "module", t, sc, n, h, rate(h, n)] for (t, sc), (n, h) in sorted(mod.items())]
        rows += [[config, split, "fn", t, sc, n, h, rate(h, n)] for (t, sc), (n, h) in sorted(fn.items())]
        rows.append([config, split, "fn", "shared_helper", "all", shared[0], shared[1], ""])
    return rows


def rate(hit, n):
    return round(hit / n, 4) if n else 0.0


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default=f"{W}/eval/codenet/sample")
    ap.add_argument("--configs", default=DEFAULT)
    ap.add_argument("--split", default="dev", choices=["dev", "test", "holdout"],
                    help="test: once per final config (see scripts/README.md); holdout needs --confirm-holdout; default dev")
    ap.add_argument("--confirm-holdout", action="store_true", help="I know the holdout is a one-shot reserve")
    args = ap.parse_args()
    if args.split == "holdout" and not args.confirm_holdout:
        sys.exit("holdout is the untouched reserve: pass --confirm-holdout to run it")
    subprocess.run(["cargo", "build", "-q", "--workspace", "--examples"], cwd=W, check=True, env=NO_CARGO_TARGET)
    rows = evaluate(args.out, args.split, args.configs.split(","))
    with open(f"{args.out}/results_{args.split}.csv", "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(HEADER)
        w.writerows(rows)
    for r in rows:
        print(f"{r[0]:14} {r[1]:5} {r[2]:6} {r[3]:20} {r[4]:10} {r[6]:6}/{r[5]:<7} {r[7]}")


if __name__ == "__main__":
    main()
