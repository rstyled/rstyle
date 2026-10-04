#!/usr/bin/env python3
"""Sample Project CodeNet Rust submissions into labeled cargo crates for the patterns miner (no deps).

Input: the `rust_package/` folder (rust_submissions.csv with CodeNet metadata columns and
rust_training.jsonl with code per submission; or `--code-dir` with pXXXXX/Rust/sNNN.rs files).
Output (`--out`, default eval/codenet/sample):
  shards/<split>_<n>/   one crate per shard, one module per submission (src/s<id>.rs)
  pairs.csv             every submission pair inside a shard with its label tier
  manifest.json         funnel counts (how many files each filter dropped) and the parameters

Why it is a useful test: submissions to the same problem by different people implement the same
behaviour with different spellings (loops, match vs if, iterator chains). The spelling differences
are written by strangers, not by us, so this is the held-out check for the canonicalizer.

Label tiers (strongest first):
  resubmit      same problem, same user: edited copies of one solution (type 1-3 clones)
  same_problem  same problem, different users: same behaviour, maybe a different algorithm (weak positive)
  cross_problem different problems: negative (a flagged pair is a false positive or a shared helper)

  identical_problem  different problems whose statements are identical (derived/identical_problem_clusters.csv):
                same behaviour written by strangers, the strongest non-copy positive

Splits (dev / test / holdout) are by identical-problem cluster, hashed with `--split-seed` (NOT the
sampling seed), so a problem is in the same split under every seed and nothing leaks between them.
`holdout` is a reserve nobody tunes on or tests on: it is only written with `--emit-holdout`.

Sample size: per split, the number of submissions is the finite-population sample size for
`--confidence` (95|99) and `--margin` (scripts/sample_size.py), unless `--sample-n` is given.
Everything is a pure function of (package, seed, split-seed, parameters): no `random` module, units
and submissions are ordered by sha1(seed:key). `manifest.json` carries a `digest` of the sampled ids
so a rerun can be checked byte for byte.

Usage: scripts/codenet_sample.py --package /path/to/rust_package [--seed 1] [--confidence 99 --margin 0.05]
"""
import argparse, csv, hashlib, json, os, re, subprocess, sys
from collections import Counter, defaultdict

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from sample_size import sample_size

W = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
STD = {"std", "core", "alloc", "crate", "self", "super"}
ID_KEYS = ("submission_id", "id")
CODE_KEYS = ("code", "source", "src", "content", "text")
FN_RE = re.compile(r"\bfn\s+[A-Za-z_][A-Za-z_0-9]*")
MAIN_RE = re.compile(r"\bfn\s+main\s*\(")


def first_key(row, keys):
    """Look in the row and in a nested `metadata` dict."""
    for source in (row, row.get("metadata") or {}):
        for k in keys:
            if isinstance(source, dict) and source.get(k) not in (None, ""):
                return source[k]
    return None


def load_meta(pkg):
    with open(f"{pkg}/rust_submissions.csv", newline="", encoding="utf-8") as f:
        return {r["submission_id"]: r for r in csv.DictReader(f)}


def load_code(pkg, code_dir, wanted):
    """submission_id -> source for the ids in `wanted`."""
    out = {}
    if code_dir:
        for sid, row in wanted.items():
            path = f"{code_dir}/{row['problem_id']}/Rust/{sid}.rs"
            if os.path.exists(path):
                out[sid] = open(path, encoding="utf-8", errors="replace").read()
        return out
    with open(f"{pkg}/rust_training.jsonl", encoding="utf-8") as f:
        for line in f:
            row = json.loads(line)
            sid, code = first_key(row, ID_KEYS), first_key(row, CODE_KEYS)
            if sid in wanted and isinstance(code, str):
                out[sid] = code
    return out


def strip_comments(code):
    code = re.sub(r"/\*.*?\*/", "", code, flags=re.S)
    return re.sub(r"//[^\n]*", "", code)


def fingerprint(code):
    return hashlib.sha1(re.sub(r"\s+", " ", strip_comments(code)).strip().encode()).hexdigest()


def foreign_crates(code):
    """Crate names the file pulls in besides std/core/alloc (it would not build offline)."""
    names = re.findall(r"^\s*(?:pub\s+)?(?:extern\s+crate|use)\s+(?:::)?([A-Za-z_][A-Za-z_0-9]*)",
                       strip_comments(code), flags=re.M)
    local = set(re.findall(r"^\s*(?:pub\s+)?(?:mod|enum|struct)\s+([A-Za-z_0-9]+)", code, flags=re.M))
    return sorted({n for n in names if n not in STD and n not in local})


def reject_reason(code, args):
    lines = code.count("\n") + 1
    if lines < args.min_lines:
        return "too_short"
    if lines > args.max_lines:
        return "too_long"
    if "#![feature" in code:
        return "nightly"
    if foreign_crates(code):
        return "foreign_crate"
    if not FN_RE.search(code):
        return "no_fn"
    return None


def prepare(meta, codes, args):
    """Accepted, deduplicated, size-filtered submissions -> (kept rows, funnel counter)."""
    funnel = Counter(total=len(meta))
    seen, kept = set(), []
    for sid, row in sorted(meta.items()):
        if args.status and row.get("status") != args.status:
            funnel["not_accepted"] += 1
        elif sid not in codes:
            funnel["no_code"] += 1
        elif (why := reject_reason(codes[sid], args)) is not None:
            funnel[why] += 1
        elif (fp := fingerprint(codes[sid])) in seen:
            funnel["duplicate"] += 1
        else:
            seen.add(fp)
            kept.append({**row, "submission_id": sid, "code": codes[sid]})
    funnel["kept"] = len(kept)
    return kept, funnel


def load_clusters(pkg):
    """problem -> cluster root (smallest problem id of its union-find component)."""
    path = f"{pkg}/derived/identical_problem_clusters.csv"
    parent = {}

    def find(x):
        parent.setdefault(x, x)
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x

    if os.path.exists(path):
        for line in open(path, encoding="utf-8"):
            pair = re.findall(r"p\d+", line)
            if len(pair) == 2:
                ra, rb = find(pair[0]), find(pair[1])
                parent[max(ra, rb)] = min(ra, rb)
    return {p: find(p) for p in list(parent)}


def rank(seed, key):
    """Version-independent pseudo-random order: sort by this."""
    return hashlib.sha1(f"{seed}:{key}".encode()).hexdigest()


def split_of(root, split_seed, test_frac, holdout_frac):
    """dev | test | holdout for a cluster root; stable under every sampling seed."""
    h = int(rank(split_seed, root), 16) % 1000
    if h < holdout_frac * 1000:
        return "holdout"
    return "test" if h < (holdout_frac + test_frac) * 1000 else "dev"


def pick(subs, k, seed):
    """Up to k submissions of one problem; one same-user pair first so every tier can occur."""
    by_user = defaultdict(list)
    for s in sorted(subs, key=lambda s: rank(seed, s["submission_id"])):
        by_user[s.get("user_id", "")].append(s)
    repeat = sorted((u for u, v in by_user.items() if len(v) >= 2 and u), key=lambda u: rank(seed, u))
    chosen = by_user[repeat[0]][:2] if repeat else []
    rest = [s for s in sorted(subs, key=lambda s: rank(seed, s["submission_id"])) if s not in chosen]
    return (chosen + rest)[:k]


def fn_count(sub):
    return len(FN_RE.findall(strip_comments(sub["code"])))


def build_units(kept, clusters, args):
    """root -> [problem submissions lists]: a cluster is one unit so its problems share a shard."""
    by_problem = defaultdict(list)
    for s in kept:
        by_problem[s["problem_id"]].append(s)
    units = defaultdict(list)
    for p, v in sorted(by_problem.items()):
        if len(v) >= args.min_per_problem:
            units[clusters.get(p, p)].append(v)
    return units


def targets(units, args):
    """split -> number of submissions to sample (finite-population sample size, capped by availability)."""
    pool = Counter()
    for root, probs in units.items():
        pool[split_of(root, args.split_seed, args.test_frac, args.holdout_frac)] += sum(map(len, probs))
    if args.sample_n:
        return {sp: min(args.sample_n, n) for sp, n in pool.items()}, pool
    return {sp: sample_size(n, args.confidence, args.margin) for sp, n in pool.items()}, pool


def plan_shards(kept, clusters, args):
    """([(split, [submissions])], targets, pool): units fill shards of their split until the fn budget is spent."""
    units = build_units(kept, clusters, args)
    want, pool = targets(units, args)
    shards, taken = [], Counter()  # shards: [split, subs, fns]
    for root in sorted(units, key=lambda r: rank(args.seed, r)):
        split = split_of(root, args.split_seed, args.test_frac, args.holdout_frac)
        if split == "holdout" and not args.emit_holdout:
            continue
        room = want.get(split, 0) - taken[split]
        group = []
        for probs in sorted(units[root], key=lambda v: v[0]["problem_id"]):
            group += pick(probs, args.per_problem, args.seed)
        if room < 2:
            continue
        group = group[:room]
        cost = sum(fn_count(s) for s in group)
        open_ = [sh for sh in shards if sh[0] == split and sh[2] + cost <= args.max_fns]
        if open_:
            open_[0][1].extend(group)
            open_[0][2] += cost
        elif len([sh for sh in shards if sh[0] == split]) < args.shards:
            shards.append([split, list(group), cost])
        else:
            continue
        taken[split] += len(group)
    return [(sp, subs) for sp, subs, _ in shards], want, pool


def module_name(sub):
    return f"s{sub['submission_id'].lstrip('s')}"


def write_crate(path, name, subs):
    os.makedirs(f"{path}/src", exist_ok=True)
    with open(f"{path}/Cargo.toml", "w") as f:
        f.write(f'[package]\nname = "{name}"\nversion = "0.1.0"\nedition = "2021"\n\n[workspace]\n')
    for s in subs:
        body = MAIN_RE.sub("pub fn entry_main(", s["code"])  # `main` is skipped by the miner
        with open(f"{path}/src/{module_name(s)}.rs", "w") as f:
            f.write(body)
    write_lib(path, subs)


def write_lib(path, subs):
    mods = "".join(f"pub mod {module_name(s)};\n" for s in subs)
    with open(f"{path}/src/lib.rs", "w") as f:
        f.write("#![allow(warnings)]\n" + mods)


def failing_modules(path, name):
    env = dict(os.environ, CARGO_TARGET_DIR=f"/tmp/cn_t_{name}")
    p = subprocess.run(["cargo", "check", "--offline", "--all-targets", "--message-format=short", "-q"], cwd=path,
                       env=env, capture_output=True, text=True)
    if p.returncode == 0:
        return set(), True
    return set(re.findall(r"src/(s\d+)\.rs", p.stdout + p.stderr)), False


def check_loop(path, name, subs, rounds=8):
    """Drop modules that do not compile until the crate builds. Returns (survivors, dropped, ok)."""
    dropped = []
    for _ in range(rounds):
        bad, ok = failing_modules(path, name)
        if ok:
            return subs, dropped, True
        hit = [s for s in subs if module_name(s) in bad]
        if not hit:
            return subs, dropped, False  # error without a file we can drop
        dropped += hit
        subs = [s for s in subs if s not in hit]
        for s in hit:
            os.remove(f"{path}/src/{module_name(s)}.rs")
        write_lib(path, subs)
    return subs, dropped, False


def tier(a, b, clusters):
    if a["problem_id"] != b["problem_id"]:
        ca, cb = clusters.get(a["problem_id"]), clusters.get(b["problem_id"])
        return "identical_problem" if ca and ca == cb else "cross_problem"
    return "resubmit" if a.get("user_id") and a.get("user_id") == b.get("user_id") else "same_problem"


def pair_rows(shard, split, subs, clusters):
    rows = []
    for i, a in enumerate(subs):
        for b in subs[i + 1:]:
            rows.append([shard, split, module_name(a), module_name(b), a["problem_id"], b["problem_id"],
                         tier(a, b, clusters)])
    return rows


def shape(subs):
    """Functions per submission and the share of one-function (main only) programs: those have no fn pair."""
    counts = [fn_count(s) for s in subs]
    n = max(len(counts), 1)
    return {"submissions": len(counts), "fns_per_submission": round(sum(counts) / n, 2),
            "main_only_share": round(sum(c <= 1 for c in counts) / n, 3)}


def run(args):
    meta = load_meta(args.package)
    wanted = {k: v for k, v in meta.items() if not args.status or v.get("status") == args.status}
    codes = load_code(args.package, args.code_dir, wanted)
    kept, funnel = prepare(meta, codes, args)
    clusters = load_clusters(args.package)
    shards, want, pool = plan_shards(kept, clusters, args)
    shards = [(sp, sorted(subs, key=lambda s: s["submission_id"])) for sp, subs in shards]
    out = os.path.abspath(args.out)
    rows, report, sampled = [], [], []
    for n, (split, subs) in enumerate(shards):
        name = f"{split}_{n}"
        path = f"{out}/shards/{name}"
        write_crate(path, f"cn_{name}", subs)
        ok = True
        if not args.no_check:
            subs, dropped, ok = check_loop(path, f"cn_{name}", subs)
            funnel["compile_fail"] += len(dropped)
        rows += pair_rows(name, split, subs, clusters)
        sampled += subs
        report.append({"shard": name, "split": split, "submissions": len(subs),
                       "problems": len({s["problem_id"] for s in subs}), "builds": ok})
    os.makedirs(out, exist_ok=True)
    with open(f"{out}/pairs.csv", "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["shard", "split", "a", "b", "problem_a", "problem_b", "tier"])
        w.writerows(rows)
    digest = hashlib.sha1("\n".join(sorted(s["submission_id"] for s in sampled)).encode()).hexdigest()
    manifest = {"args": {k: v for k, v in vars(args).items()}, "funnel": dict(funnel), "shards": report,
                "tiers": dict(Counter(r[6] for r in rows)), "target_per_split": want, "eligible_per_split": dict(pool),
                "shape": shape(sampled), "digest": digest}
    json.dump(manifest, open(f"{out}/manifest.json", "w"), indent=2)
    print(json.dumps({"funnel": manifest["funnel"], "tiers": manifest["tiers"], "shards": report}, indent=1))


def parser():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--package", required=True, help="rust_package/ folder")
    p.add_argument("--code-dir", help="CodeNet data/ dir (pXXXXX/Rust/sNNN.rs) instead of rust_training.jsonl")
    p.add_argument("--out", default=f"{W}/eval/codenet/sample")
    p.add_argument("--seed", type=int, default=1)
    p.add_argument("--status", default="Accepted", help="keep this status only ('' keeps all)")
    p.add_argument("--min-lines", type=int, default=15)
    p.add_argument("--max-lines", type=int, default=250)
    p.add_argument("--min-per-problem", type=int, default=2)
    p.add_argument("--per-problem", type=int, default=12, help="submissions sampled per problem")
    p.add_argument("--shards", type=int, default=60, help="at most this many crates per split")
    p.add_argument("--max-fns", type=int, default=380, help="fn budget per crate (all-pairs limit is 400)")
    p.add_argument("--split-seed", type=int, default=0, help="seed of the dev/test/holdout hash (keep fixed)")
    p.add_argument("--test-frac", type=float, default=0.2)
    p.add_argument("--holdout-frac", type=float, default=0.2, help="reserve, never sampled without --emit-holdout")
    p.add_argument("--emit-holdout", action="store_true", help="also write holdout shards (use once, at the end)")
    p.add_argument("--confidence", type=int, default=99, choices=[95, 99], help="sizes the per-split sample")
    p.add_argument("--margin", type=float, default=0.05, help="half-width of the interval for the sample size")
    p.add_argument("--sample-n", type=int, default=0, help="override: submissions per split")
    p.add_argument("--no-check", action="store_true", help="skip the cargo check / drop loop")
    return p


if __name__ == "__main__":
    run(parser().parse_args())
