#!/usr/bin/env python3
"""Tests for codenet_sample.py on a tiny fake rust_package (needs cargo, no network).
Run: python3 scripts/test_codenet.py"""
import csv, json, os, subprocess, sys, tempfile, unittest

HERE = os.path.dirname(os.path.abspath(__file__))
SUM_LOOP = "use std::io;\nfn total(xs: &[i64]) -> i64 {\n    let mut t = 0;\n    for x in xs { t += x; }\n    t\n}\nfn main() {\n    let v: Vec<i64> = vec![1, 2, 3];\n    println!(\"{}\", total(&v));\n}\n"
SUM_FOLD = "fn total(xs: &[i64]) -> i64 {\n    xs.iter().fold(0, |a, b| a + b)\n}\nfn main() {\n    let v: Vec<i64> = vec![1, 2, 3];\n    println!(\"{}\", total(&v));\n}\n"
SUM_WHILE = "fn total(xs: &[i64]) -> i64 {\n    let mut i = 0;\n    let mut t = 0;\n    while i < xs.len() { t += xs[i]; i += 1; }\n    t\n}\nfn main() {\n    let v: Vec<i64> = vec![4, 5];\n    println!(\"{}\", total(&v));\n}\n"
MAXV = "fn best(xs: &[i64]) -> i64 {\n    let mut m = xs[0];\n    for x in xs.iter() { if x.gt(&m) { m = x.clone(); } }\n    println!(\"{}\", m.abs());\n    m\n}\nfn main() {\n    println!(\"{}\", best(&[3, 9, 2]));\n}\n"
MAXV2 = MAXV.replace("best", "largest")
EXT = "use itertools::Itertools;\nfn main() {\n    println!(\"{}\", 1);\n}\n"
BROKEN = "fn main() {\n    let x: i32 = \"nope\";\n    println!(\"{}\", x);\n}\n"
TINY = "fn main() {}\n"

ROWS = [  # id, problem, user, status, code
    ("s001", "p00001", "u1", "Accepted", SUM_LOOP),
    ("s002", "p00001", "u1", "Accepted", SUM_FOLD),
    ("s003", "p00001", "u2", "Accepted", SUM_WHILE),
    ("s004", "p00001", "u3", "Accepted", SUM_LOOP.replace("    ", "  ")),  # same tokens: duplicate of s001
    ("s005", "p00001", "u4", "Wrong Answer", SUM_FOLD),
    ("s006", "p00001", "u5", "Accepted", EXT),
    ("s007", "p00001", "u6", "Accepted", BROKEN),
    ("s008", "p00001", "u7", "Accepted", TINY),
    ("s009", "p00002", "u8", "Accepted", MAXV),
    ("s010", "p00002", "u9", "Accepted", MAXV2),
]


def make_package(root):
    with open(f"{root}/rust_submissions.csv", "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["submission_id", "problem_id", "user_id", "status", "language"])
        for sid, p, u, st, _ in ROWS:
            w.writerow([sid, p, u, st, "Rust"])
    with open(f"{root}/rust_training.jsonl", "w") as f:
        for sid, p, u, st, code in ROWS:
            f.write(json.dumps({"submission_id": sid, "metadata": {"problem_id": p}, "code": code}) + "\n")


class Sampler(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        pkg, cls.out = f"{cls.tmp.name}/pkg", f"{cls.tmp.name}/out"
        os.makedirs(pkg)
        make_package(pkg)
        subprocess.run([sys.executable, f"{HERE}/codenet_sample.py", "--package", pkg, "--out", cls.out,
                        "--min-lines", "4", "--test-frac", "0", "--holdout-frac", "0", "--per-problem", "6"],
                       check=True, capture_output=True)
        cls.manifest = json.load(open(f"{cls.out}/manifest.json"))
        cls.pairs = list(csv.DictReader(open(f"{cls.out}/pairs.csv")))

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def test_funnel_counts_each_filter(self):
        f = self.manifest["funnel"]
        self.assertEqual(f["total"], 10)
        self.assertEqual(f["not_accepted"], 1)
        self.assertEqual(f["foreign_crate"], 1)
        self.assertEqual(f["too_short"], 1)
        self.assertEqual(f["duplicate"], 1)
        self.assertEqual(f["compile_fail"], 1)

    def test_pairs_have_all_three_tiers_and_no_dropped_submission(self):
        tiers = {r["tier"] for r in self.pairs}
        self.assertEqual(tiers, {"resubmit", "same_problem", "cross_problem"})
        mods = {r["a"] for r in self.pairs} | {r["b"] for r in self.pairs}
        self.assertEqual(mods, {"s001", "s002", "s003", "s009", "s010"})

    def test_resubmit_is_same_problem_same_user(self):
        r = [r for r in self.pairs if r["tier"] == "resubmit"]
        self.assertEqual([(x["a"], x["b"]) for x in r], [("s001", "s002")])

    def test_main_is_renamed_and_crate_builds(self):
        shard = next(d for d in os.listdir(f"{self.out}/shards"))
        text = open(f"{self.out}/shards/{shard}/src/s001.rs").read()
        self.assertIn("fn entry_main(", text)
        self.assertNotIn("fn main(", text)
        self.assertTrue(all(s["builds"] for s in self.manifest["shards"]))

    def test_splits_are_by_problem(self):
        by_problem = {}
        for r in self.pairs:
            by_problem.setdefault(r["problem_a"], set()).add(r["split"])
            by_problem.setdefault(r["problem_b"], set()).add(r["split"])
        self.assertTrue(all(len(v) == 1 for v in by_problem.values()))


def cluster_package(root):
    """p00003 and p00004 have identical statements; p00005 stands alone."""
    code = lambda k: SUM_LOOP.replace("total", f"total{k}").replace("t += x;", f"t += x * {k};")
    rows = [(f"s1{i}{p}", f"p0000{p}", f"u{i}{p}", "Accepted", code(10 * p + i)) for p in (3, 4, 5) for i in (1, 2)]
    with open(f"{root}/rust_submissions.csv", "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["submission_id", "problem_id", "user_id", "status", "language"])
        w.writerows([list(r[:4]) + ["Rust"] for r in rows])
    with open(f"{root}/rust_training.jsonl", "w") as f:
        for sid, p, u, st, c in rows:
            f.write(json.dumps({"submission_id": sid, "code": c}) + "\n")
    os.makedirs(f"{root}/derived")
    open(f"{root}/derived/identical_problem_clusters.csv", "w").write("p00003,p00004\n")


def run_sampler(pkg, out, *extra):
    subprocess.run([sys.executable, f"{HERE}/codenet_sample.py", "--package", pkg, "--out", out, "--min-lines", "4",
                    "--test-frac", "0", "--holdout-frac", "0", "--no-check", *extra], check=True, capture_output=True)
    return json.load(open(f"{out}/manifest.json")), list(csv.DictReader(open(f"{out}/pairs.csv")))


class Clusters(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        cls.pkg = f"{cls.tmp.name}/pkg"
        os.makedirs(cls.pkg)
        cluster_package(cls.pkg)

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def test_cluster_problems_share_a_split_under_every_seed(self):
        import codenet_sample as cs
        clusters = cs.load_clusters(self.pkg)
        self.assertEqual(clusters, {"p00003": "p00003", "p00004": "p00003"})
        for split_seed in range(50):
            splits = {cs.split_of(clusters[p], split_seed, 0.3, 0.2) for p in ("p00003", "p00004")}
            self.assertEqual(len(splits), 1)

    def test_pair_across_cluster_is_identical_problem_not_cross_problem(self):
        _, pairs = run_sampler(self.pkg, f"{self.tmp.name}/o1", "--seed", "1")
        tier = {frozenset((r["problem_a"], r["problem_b"])): r["tier"] for r in pairs}
        self.assertEqual(tier[frozenset(("p00003", "p00004"))], "identical_problem")
        self.assertEqual(tier[frozenset(("p00003", "p00005"))], "cross_problem")
        self.assertEqual(tier[frozenset(("p00003",))], "same_problem")

    def test_same_seed_same_digest(self):
        m1, p1 = run_sampler(self.pkg, f"{self.tmp.name}/a", "--seed", "7")
        m2, p2 = run_sampler(self.pkg, f"{self.tmp.name}/b", "--seed", "7")
        self.assertEqual((m1["digest"], p1), (m2["digest"], p2))

    def test_holdout_is_not_written_unless_asked(self):
        kw = ["--seed", "1", "--holdout-frac", "1"]
        m, pairs = run_sampler(self.pkg, f"{self.tmp.name}/h0", *kw)
        self.assertEqual((m["shards"], pairs), ([], []))
        m, _ = run_sampler(self.pkg, f"{self.tmp.name}/h1", *kw, "--emit-holdout")
        self.assertEqual({s["split"] for s in m["shards"]}, {"holdout"})

    def test_sample_size_is_finite_population_corrected(self):
        from sample_size import sample_size
        self.assertEqual(sample_size(None, 95, 0.05), 385)
        self.assertEqual(sample_size(None, 99, 0.05), 664)
        self.assertEqual(sample_size(53822, 99, 0.05), 656)
        self.assertEqual(sample_size(10, 99, 0.05), 10)  # never more than the population


def fn(mod, name, calls=(), params=0, statements=3, source="", pdg=None):
    return {"id": f"cn_x::{mod}::{name}", "name": f"{mod}::{name}", "params": params, "statements": statements,
            "calls": [{"callee": c} for c in calls], "source": source, "pdg": pdg}


READ = ("std::io::stdin", "std::io::Stdin::read_line", "core::str::parse", "std::result::Result::unwrap")


class Scoring(unittest.TestCase):
    """codenet_score on hand-made facts: no cargo, no miner."""

    def facts(self):
        out = []
        for m in ("s001", "s002"):
            out += [fn(m, "read_line_helper", READ), fn(m, "read_vec", ["std::iter::Iterator::map"],
                    source="(0..n).map(|_| read_line_helper()).collect()"),
                    fn(m, "total", ["std::iter::Iterator::fold"], params=1),
                    fn(m, "report", ["std::io::_print"] + list(READ))]  # prints: a solution, not a helper
        return out

    def test_io_helpers_are_tagged_including_closure_readers_but_not_printers(self):
        import codenet_score as cs
        io = {k.split("::")[-1] for k, v in cs.fn_table(self.facts()).items() if v["io"]}
        self.assertEqual(io, {"read_line_helper", "read_vec"})

    def test_shared_helper_is_not_a_positive_and_total_is(self):
        import codenet_score as cs
        table = cs.fn_table(self.facts())
        tiers = {frozenset(("s001", "s002")): "same_problem"}
        pairs = cs.fn_pairs(table, tiers)
        self.assertEqual(sorted(p[0].split("::")[-1] for p in pairs), ["report", "total"])
        site = lambda m, n: {"id": f"cn_x::{m}::{n}"}
        report = {"candidates": [{"sites": [site("s001", "read_line_helper"), site("s002", "read_line_helper")]},
                                 {"sites": [site("s001", "total"), site("s002", "total")]}]}
        module_flagged, fn_flagged, (shared_n, shared_pairs) = cs.split_candidates(report, table)
        self.assertEqual((shared_n, len(shared_pairs), len(module_flagged)), (1, 1, 1))
        self.assertEqual(fn_flagged, {frozenset(("cn_x::s001::total", "cn_x::s002::total"))})
        cells = cs.tally(pairs, fn_flagged, [True] * len(pairs))
        self.assertEqual(cells[("fn_same_problem", "all")], [2, 1])

    def test_cross_problem_negatives_need_different_names(self):
        import codenet_score as cs
        table = cs.fn_table(self.facts())
        pairs = cs.fn_pairs(table, {frozenset(("s001", "s002")): "cross_problem"})
        self.assertEqual(len(pairs), 2)  # 2 non-IO fns each side (total, report): 2 x 2 minus the 2 same-name pairs
        self.assertTrue(all(t == "fn_cross_problem" for _, _, t in pairs))

    def test_base_name_drops_module_and_generics(self):
        import codenet_score as cs
        self.assertEqual(cs.base_name({"name": "s123456789::io::Scanner::<'a>::next"}), "io::Scanner::next")
        self.assertEqual(cs.base_name({"name": "<s123456789::T as std::ops::Add>::add"}), "<T as std::ops::Add>::add")


class EvalConfig(unittest.TestCase):
    def test_ablation_config_drops_one_rule_and_none_is_empty(self):
        import codenet_eval as ev
        self.assertEqual(ev.rules_of("all-try"), "ctor,pred,case,exit,counter,cmp")
        self.assertEqual(ev.rules_of("none"), "")
        self.assertEqual(ev.miner_args("all+pc800+h4"), ["--normalize", "all", "--pair-coverage", "800", "--max-holes", "4"])
        self.assertEqual(ev.miner_args("none"), [])

    def test_holdout_needs_confirmation(self):
        p = subprocess.run([sys.executable, f"{HERE}/codenet_eval.py", "--split", "holdout"], capture_output=True, text=True)
        self.assertNotEqual(p.returncode, 0)
        self.assertIn("--confirm-holdout", p.stderr)


class Separation(unittest.TestCase):
    def test_auc_handles_perfect_reversed_and_tied(self):
        import codenet_sep as sep
        self.assertEqual(sep.auc([3, 4], [1, 2]), 1.0)
        self.assertEqual(sep.auc([1, 2], [3, 4]), 0.0)
        self.assertEqual(sep.auc([1, 1], [1, 1]), 0.5)

    def test_excess_is_zero_for_negatives_and_one_for_pure_positives(self):
        import codenet_sep as sep
        neg = [0] * 98 + [5, 5]
        self.assertEqual(sep.excess(neg, neg, 5, lambda x: x), 0.0)
        self.assertEqual(sep.excess([9] * 10, neg, 5, lambda x: x), 1.0)

    def test_paired_ci_is_zero_when_canon_equals_raw(self):
        import codenet_sep as sep
        pos, neg = [{"s": i} for i in range(5, 15)], [{"s": i} for i in range(10)]
        a0, a1, lo, hi = sep.paired_ci(pos, neg, lambda r: r["s"], lambda r: r["s"], 20, 1)
        self.assertEqual((a0, a1, lo, hi), (a0, a0, 0.0, 0.0))


class Flips(unittest.TestCase):
    def test_sign_test_is_exact_and_symmetric(self):
        import codenet_flips as fl
        self.assertEqual(fl.sign_test(0, 0), 1.0)
        self.assertAlmostEqual(fl.sign_test(0, 6), 2 / 64)
        self.assertEqual(fl.sign_test(3, 3), 1.0)
        self.assertEqual(fl.sign_test(1, 9), fl.sign_test(9, 1))

    def test_unmatched_ignores_shared_contraction(self):
        import codenet_sep as sep
        # 100 nodes, 90 matched; contracting 5 shared nodes leaves 10 unmatched of 95 (ratio falls, gap does not)
        self.assertAlmostEqual(sep.unmatched(900, 100), 10)
        self.assertAlmostEqual(sep.unmatched(int(1000 * 85 / 95), 95), 10, delta=0.2)  # milli rounding


if __name__ == "__main__":
    unittest.main()
