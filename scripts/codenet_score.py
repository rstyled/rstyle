"""Function-level scoring for the CodeNet eval (pure functions, no I/O, no deps).

Input is what the miner already writes: `facts-sem/*.json` (list of fn dicts with `id`, `name`,
`params`, `calls`, `pdg`) and the pattern report (`candidates[].sites[].id`).

IO helpers: plumbing that reads and parses input (`read`, `read_vec`, `Scanner::next`). A fn is one
when it calls an input primitive, every other call is std plumbing (or another IO helper) and it
is short. A candidate needs >= 2 non-IO sites from different submissions to count; the rest are
`shared_helper` (reported, never an error). Output (print/write) is never plumbing: a fn that
prints is a solution, not a helper.
"""
import re
from itertools import combinations, product

MODULE_RE = re.compile(r"::(s\d+)(?:::|$)")
PLUMBING = ("std::io::", "core::str::", "std::str::", "std::string::", "std::vec::", "std::result::",
            "std::option::", "std::iter::", "core::slice::", "std::slice::", "std::char::", "std::clone::",
            "std::convert::", "std::borrow::", "std::boxed::", "std::default::", "std::mem::", "std::marker::",
            "alloc::", "core::panicking::", "std::rt::", "std::fmt::Arguments::from_str", "core::num::")
OUTPUT = ("std::io::_print", "std::io::Write::", "std::io::stdout", "std::io::Stdout::", "std::io::BufWriter::",
          "std::io::_eprint", "std::io::stderr")
INPUT = ("std::io::stdin", "std::io::Stdin::", "std::io::Read::", "std::io::BufRead::", "std::io::BufReader::",
         "std::io::Lines", "core::str::parse", "core::str::split_whitespace", "std::str::FromStr::",
         "core::str::FromStr::", "std::io::Cursor")
OUTPUT_MACROS = ("print", "println", "eprint", "eprintln", "write", "writeln")
MAX_IO_STATEMENTS = 15
TIERS_POSITIVE = ("resubmit", "same_problem", "identical_problem")


def module_of(text):
    m = MODULE_RE.search(text)
    return m.group(1) if m else None


def base_name(fn):
    """`s123::io::Scanner::<'a>::next` -> `io::Scanner::next` (module prefix and generics dropped)."""
    name = re.sub(r"\bs\d{3,}(::)?", "", fn["name"])  # also inside `<s123::T as Trait>::f`
    prev = None
    while prev != name:  # `<'a>` / `<R, W>` go, `<T as Trait>` stays
        prev, name = name, re.sub(r"<(?:(?! as )[^<>])*>(::)?", "", name)
    return name


def user_calls(fn, ids):
    """(callee ids of the pdg that are fns of this crate, whether the fn prints)."""
    nodes = (fn.get("pdg") or {}).get("nodes", [])
    callees = [n["callee_id"] for n in nodes if n["kind"] == "call" and n.get("callee_id")]
    return {c for c in callees if c in ids}, any(
        n["kind"] == "macro" and str(n.get("macro_name") or "").startswith(OUTPUT_MACROS) for n in nodes)


def plumbing_only(fn, io, ids):
    """(all calls are plumbing or IO helpers, reads input)."""
    callees = [re.sub(r"^s\d{3,}::", "", c["callee"]) for c in fn["calls"]]  # `use core::str::..` shows as `sN::core::..`
    users, prints = user_calls(fn, ids)
    if prints or any(c.startswith(OUTPUT) for c in callees) or not users <= io:
        return False, False
    others = [c for c in callees if c != "<indirect callable>"]
    ok = all(c.startswith(PLUMBING) for c in others) and len(others) == len(callees)
    return ok, any(c.startswith(INPUT) for c in callees) or bool(users)


def impl_of(fid):
    """`crate::sN::{impl#2}::vec` -> `crate::sN::{impl#2}`; None for free fns."""
    head, _, _ = fid.rpartition("::")
    return head if "{impl#" in head else None


def calls_by_name(fn, names):
    """True when the fn's source calls (`f(` or `.f(`) a fn whose last path segment is in `names`: reads hidden in closures."""
    return any(re.search(rf"\b{re.escape(n)}\s*\(", fn.get("source", "")) for n in names)


def hides_read(fn, io):
    """A plumbing-only fn that still belongs to the readers: it calls an IO helper of its module from
    a closure, sits in an impl that has an IO method, or is a method of a type called `Scanner`."""
    names = {i.rsplit("::", 1)[1] for i in io if module_of(i) == module_of(fn["id"])}
    impls = {impl_of(i) for i in io} - {None}
    return bool(re.search(r"\bScanner::", fn["name"])) or calls_by_name(fn, names) or impl_of(fn["id"]) in impls


def tag_io(facts):
    """Ids of IO helper fns. Fixpoint, so `read_vec` (calls `read`) is tagged once `read` is.

    A plumbing-only method of an impl that already has an IO method is one too: `Scanner::vec`
    is `(0..n).map(|_| self.read()).collect()`, the read hides in a closure the fn's own calls miss."""
    by_id = {f["id"]: f for f in facts}
    io, changed = set(), True
    while changed:
        changed = False
        for fid, fn in by_id.items():
            if fid in io or fn.get("statements", 99) > MAX_IO_STATEMENTS:
                continue
            ok, reads = plumbing_only(fn, io, by_id.keys())
            if ok and (reads or hides_read(fn, io)):
                io.add(fid)
                changed = True
    return io


def fn_table(facts):
    """id -> {module, key=(base name, arity), io}; duplicates (lib + test target) collapse."""
    io = tag_io(list({f["id"]: f for f in facts}.values()))
    table = {}
    for f in facts:
        if f["id"] not in table and module_of(f["id"]):
            table[f["id"]] = {"module": module_of(f["id"]), "key": (base_name(f), f["params"]), "io": f["id"] in io}
    return table


def fn_pairs(table, module_tier):
    """[(id_a, id_b, tier)] over non-IO fns: same (name, arity) in same-behaviour modules are positives,
    different names across unrelated problems are negatives; everything else is unlabeled and skipped."""
    by_module = {}
    for fid, f in sorted(table.items()):
        if not f["io"]:
            by_module.setdefault(f["module"], []).append(fid)
    out = []
    for pair, tier in sorted(module_tier.items(), key=lambda kv: sorted(kv[0])):
        a, b = sorted(pair)
        for x, y in product(by_module.get(a, []), by_module.get(b, [])):
            same = table[x]["key"] == table[y]["key"]
            if (tier in TIERS_POSITIVE and same) or (tier == "cross_problem" and table[x]["key"][0] != table[y]["key"][0]):
                out.append((x, y, f"fn_{tier}"))
    return out


def split_candidates(report, table):
    """(module pairs of any candidate, fn pairs of counted candidates, shared_helper candidates).

    shared_helper = (candidate count, module pairs only they credit)."""
    module_flagged, fn_flagged, shared_pairs, shared_n = set(), set(), set(), 0
    for c in report.get("candidates", []):
        ids = [s["id"] for s in c["sites"]]
        mods = {module_of(i) for i in ids} - {None}
        module_flagged |= {frozenset(p) for p in combinations(sorted(mods), 2)}
        real = [i for i in ids if i in table and not table[i]["io"]]
        if len({table[i]["module"] for i in real}) >= 2:
            fn_flagged |= {frozenset(p) for p in combinations(sorted(real), 2)
                           if table[p[0]]["module"] != table[p[1]]["module"]}
        elif len(mods) >= 2:
            shared_n += 1
            shared_pairs |= {frozenset(p) for p in combinations(sorted(mods), 2)}
    return module_flagged, fn_flagged, (shared_n, shared_pairs)


def tally(pairs, flagged, comparable):
    """{(tier, scope): [pairs, flagged]} with scope `all` and `comparable`; `comparable` aligns with pairs."""
    cells = {}
    for (a, b, tier), ok in zip(pairs, comparable):
        hit = frozenset((a, b)) in flagged
        for scope in ("all", "comparable") if ok else ("all",):
            cell = cells.setdefault((tier, scope), [0, 0])
            cell[0] += 1
            cell[1] += hit
    return cells
