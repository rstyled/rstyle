use super::*;

pub(crate) const SOURCE: &str = r#"
#![feature(guard_patterns)]
pub fn accumulator(n: usize) -> u8 { let mut i = 0; let mut total = 0u8; while i < n { total += 1; i += 1; } total }
pub fn practical_vec(xs: &[u8]) -> Vec<u8> { let mut out = Vec::new(); for x in xs { out.push(*x); } out }
pub fn practical_map(xs: &[u8]) { let mut out = std::collections::HashMap::new(); for x in xs { out.insert(*x, 1); } }
pub fn practical_match(n: usize, flag: bool) { let mut i = 0; while i < n { match flag { true => (), false => () } i += 1; } }
pub fn practical_iflet(n: usize, x: Option<u8>) { let mut i = 0; while i < n { if let Some(_) = x {} i += 1; } }
trait Modify { fn advance(&mut self); fn reset(&mut self); }
impl Modify for u32 { fn advance(&mut self) { *self += 1; } fn reset(&mut self) { *self = 0; } }
pub fn autoref(n: u32) { let mut i = 0u32; while i < n { i.advance(); } }
pub fn autoref_reset(n: u32) { let mut i = 0u32; while i < n { i.reset(); i += 1; } }
pub fn refmut(n: u32) { let mut i = 0u32; while i < n { let ref mut alias = i; *alias = 0; i += 1; } }
pub fn ergonomic_refmut(n: usize) { let mut i = 0; while i < n { match &mut i { alias => *alias = 0 } i += 1; } }
pub fn metadata_rebind<'a>(mut xs: &'a [u8], other: &'a [u8]) { let mut i = 0; while i < xs.len() { i += 1; let alias = &mut xs; *alias = other; } }
pub fn unsafe_body(n: usize) { let mut i = 0; while i < n { unsafe { std::ptr::read_volatile(&i); } i += 1; } }
pub fn captured_closure(n: usize) { let mut i = 0; while i < n { let _f = || i; i += 1; } }
pub fn callee_continue(n: usize, flag: bool, fp: fn()) { let mut i = 0; while i < n { ({ if flag { continue; } fp })(); i += 1; } }
pub fn callee_reset(n: usize, fp: fn()) { let mut i = 0; while i < n { ({ i = 0; fp })(); i += 1; } }
pub fn short_circuit_and(n: usize, flag: bool) { let mut i = 0; while i < n { flag && { i += 1; true }; } }
pub fn short_circuit_or(n: usize, flag: bool) { let mut i = 0; while i < n { flag || { i += 1; true }; } }
pub fn guarded_or(n: usize, flag: bool) { let mut i = 0; while i < n { match Some(0u8) { Some(_) | Some(_) if { opaque(); flag } => (), _ => () } i += 1; } }
pub fn pattern_guard_reset(n: usize) { let mut i = 0; while i < n { match Some(0u8) { Some(_ if { i = 0; true }) => (), _ => () } i += 1; } }
pub fn guard_call(n: usize) { let mut i = 0; while { opaque(); i < n } { i += 1; } }
pub fn loop_local_extent(n: usize, a: &[u8], b: &[u8]) { let mut i = 0; while i < n { let xs = if i == 0 { a } else { b }; for _ in xs {} i += 1; } }
pub struct Container { pub bytes: Vec<u8> }
pub struct View<'a> { pub bytes: &'a [u8] }
fn switch<'a>(root: &mut &'a Container, other: &'a Container) { *root = other; }
pub fn containing_rebind<'a>(mut root: &'a Container, other: &'a Container) { let mut i = 0; while i < root.bytes.len() { i += 1; switch(&mut root, other); } }
pub fn shared_vec(xs: &Vec<u8>) { for _ in xs {} }
pub fn shared_field_vec(root: &Container) { for _ in &root.bytes {} }
pub fn shared_field_slice(root: &View<'_>) { for _ in root.bytes {} }
pub fn stable_slice_binding(xs: &[u8]) { let stable = xs; for _ in stable {} }
pub fn stable_len_guard(xs: &Vec<u8>) { let mut i = 0; while i < xs.len() { i += 1; } }

macro_rules! generated { () => { pub fn generated_owner() {} } }
generated!();
pub fn stale(n:u8){let mut j=0;while j<n{j+=1;}let end=j;let mut i=0;while i<end{i+=1;}}
pub fn sequential(n: usize, m: usize) { for _ in 0..n {} for _ in 0..m {} }
pub fn repeated_slice(xs: &[u8]) { for _ in xs { for _ in xs {} } }
fn touch(i: &mut usize) { *i = 0; }
pub fn alias(n: usize) { let mut i = 0; while i < n { touch(&mut i); i += 1; } }
pub fn skipped(n: usize, flag: bool) { let mut i = 0; while i < n { if flag { i += 1; } } }
pub fn unknown_predicate(flag: bool) { while flag {} }
pub fn linear(n: usize) { let mut i = 0; while i < n { i += 1; } }
pub fn persistent(n: usize, m: usize) { let mut i = 0; let mut j = 0; while i < n { while j < m { j += 1; } i += 1; } }
pub fn reset(n: usize, m: usize) { let mut i = 0; while i < n { let mut j = 0; while j < m { j += 1; } i += 1; } }
pub fn square(n: usize) { let mut i = 0; while i < n { let mut j = 0; while j < n { j += 1; } i += 1; } }
pub fn range(n: usize) { for _i in 0..n {} }
pub fn slice(xs: &[u8]) { for _x in xs {} }
pub fn unknown_continue(n: usize, flag: bool) { let mut i = 0; while i < n { if flag { continue; } i += 1; } }
fn opaque() {}
pub fn unknown_call(n: usize) { let mut i = 0; while i < n { opaque(); i += 1; } }
pub fn limitwrites(mut n: usize) { let mut i = 0; while i < n { n += 1; i += 1; } }
pub fn overflow(n: u8) { let mut i = 0u8; while i <= n { i += 1; } }
fn score(a: usize, b: usize) -> u32 { (a ^ b) as u32 }
fn double(x: usize) -> usize { x * 2 }
pub fn medoid_chain(wls: &[usize], group: &[usize]) -> usize { let total = |v: usize| -> u32 { group.iter().filter(|w| **w != v).map(|w| score(wls[v], wls[*w])).sum() }; group.iter().copied().min_by_key(|v| (u32::MAX - total(*v), *v)).expect("non-empty") }
pub fn pair_chain(xs: &[usize]) -> u32 { xs.iter().enumerate().flat_map(|(i, a)| xs.iter().skip(i + 1).map(move |b| score(*a, *b))).sum() }
pub fn item_callback(xs: &[usize]) -> usize { xs.iter().copied().map(double).sum() }
pub fn range_chain(n: usize) -> u32 { (0..n).map(|i| score(i, i)).sum() }
pub fn short_search(xs: &[usize]) -> bool { xs.iter().any(|x| score(*x, 1) > 3) }
pub fn bounded_prefix(xs: &[usize]) -> u32 { xs.iter().take_while(|x| **x > 0).map(|x| score(*x, 0)).sum() }
pub fn explicit_iter(xs: &[u8]) { for _x in xs.iter() {} }
pub fn closure_return(xs: &[usize]) -> u32 { xs.iter().map(|x| { if *x == 0 { return 0; } score(*x, 0) }).sum() }
pub fn mutable_callback_capture(xs: &[usize], n: usize) { let mut i = 0; xs.iter().for_each(|_| i += 1); while i < n { i += 1; } }
pub fn escaped_closure(xs: &[usize]) -> usize { let f = |x: &usize| *x; let g = f; xs.iter().map(g).sum() }
pub fn generic_callback<F: Fn(&usize) -> u32>(xs: &[usize], f: F) -> u32 { xs.iter().map(f).sum() }
pub fn lazy_callback(xs: &[usize]) -> impl Iterator<Item = u32> + '_ { xs.iter().map(|x| score(*x, 0)) }
pub fn mutable_source(xs: &mut Vec<usize>) -> usize { xs.iter().map(|x| *x).sum() }
pub fn varying_inner(rows: &[Vec<u8>]) -> usize { rows.iter().flat_map(|row| row.iter()).count() }
pub fn unsafe_callback(xs: &[usize]) -> usize { xs.iter().map(|x| unsafe { std::ptr::read_volatile(x) }).sum() }
pub fn moved_mutable(xs: &[usize]) -> usize { let mut k = 0usize; k += 1; xs.iter().map(move |x| *x + k).sum() }
pub struct Counter(usize);
impl Iterator for Counter { type Item = usize; fn next(&mut self) -> Option<usize> { self.0 += 1; Some(self.0) } }
pub fn user_iterator(c: Counter) -> u32 { c.take(3).map(|x| score(x, 0)).sum() }
pub struct Edges { pub edges: Vec<u8> }
pub fn walk_edges(xs: &[u8]) -> usize { let mut c = 0; for _ in xs { c += 1; } c }
pub fn edge_caller(g: &Edges) -> usize { let mut t = 0; for _ in 0..3usize { t += walk_edges(&g.edges); } t }
pub fn counted(n: usize) -> usize { let mut t = 0; for i in 0..n { t += i; } t }
pub fn varying_calls(m: usize) -> usize { let mut t = 0; for i in 0..m { t += counted(i); } t }
pub fn recursive_count(n: usize) -> usize { if n == 0 { 0 } else { recursive_count(n - 1) } }
pub trait Area { fn area(&self) -> usize; }
pub fn dyn_area(s: &dyn Area) -> usize { s.area() }
pub fn flat_return(xs: &[usize], ys: &[usize], zs: &[usize]) -> u32 { xs.iter().flat_map(|x| { if *x == 0 { return zs.iter().copied(); } ys.iter().copied() }).map(|y| score(y, 0)).sum() }
pub fn fold_init_escape(xs: &[usize]) -> u32 { let f = |a: u32| a + score(1, 1); xs.iter().fold(f, |acc, _| acc)(0) }
#[derive(Clone)] pub struct Cloned(pub u8);
pub fn user_clone(xs: &[Cloned]) -> usize { xs.iter().cloned().count() }
#[derive(PartialEq, Eq, PartialOrd, Ord)] pub struct Key(pub usize);
pub fn ordered_collect(xs: &[usize]) -> usize { xs.iter().map(|x| Key(*x)).collect::<std::collections::BTreeSet<Key>>().len() }
pub fn boxed_callback(xs: &[usize], f: Box<dyn Fn(&usize) -> u32>) -> u32 { xs.iter().map(f).sum() }
pub fn vec_collect(xs: &[Cloned]) -> usize { xs.iter().map(|c| c.0).collect::<Vec<u8>>().len() }
pub struct Bag(pub Vec<u8>);
impl std::ops::Add for &Bag { type Output = usize; fn add(self, other: &Bag) -> usize { let mut t = 0; for _ in &other.0 { t += 1; } t } }
pub fn bag_sum(a: &Bag, b: &Bag, m: usize) -> usize { let mut t = 0; for _ in 0..m { t += a + b; } t }
#[derive(PartialEq)] pub struct Eqd(pub u8);
pub fn default_ne(a: &Eqd, b: &Eqd) -> bool { a.ne(b) }
pub fn item_to_std(n: usize) -> usize { Some(n).map(counted).unwrap_or(0) }
pub struct Guard;
impl Drop for Guard { fn drop(&mut self) { counted(3); } }
pub fn drops(n: usize) { let mut i = 0; while i < n { let _g = Guard; i += 1; } }
pub fn mutable_receiver(n: usize) { let mut v = vec![1u8]; v.iter().count(); v.push(1); let mut i = 0; while i < n { i += 1; } }
pub struct Index { pub names: std::collections::BTreeMap<String, usize> }
pub fn btree_lookups(index: &Index, keys: &[String]) -> usize { let mut found = 0; for key in keys { if index.names.contains_key(key) { found += 1; } } found }
pub fn lookup_caller(index: &Index, keys: &[String]) -> usize { btree_lookups(index, keys) }
pub fn str_keys(s: &str) -> String { let _ = s.is_empty(); s.into() }
pub fn local_map(n: usize) -> bool { let mut map = std::collections::BTreeMap::new(); map.insert(n, n); map.contains_key(&n) }
"#;

fn runtime(report: &Value) -> &Value {
    &report["compiler_advisory"]["units"][0]["runtime_bounds"]
}

fn named_row<'a>(rows: &'a [Value], name: &str) -> &'a Value {
    rows.iter()
        .find(|row| row["name"].as_str().unwrap().rsplit("::").next() == Some(name))
        .unwrap_or_else(|| panic!("missing runtime-bound row {name}"))
}

#[test]
fn runtime_bounds_are_independent_replayable_owner_results() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), SOURCE).unwrap();
    let output = rstyle(
        directory.path(),
        &["patterns", "--format", "json", "--complexity"],
    );
    let original: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(output.status.success(), "{}", stdout(&output));
    assert_known(&original);
    assert_call(runtime(&original)["rows"].as_array().unwrap());
    assert_callbacks(runtime(&original)["rows"].as_array().unwrap());
    assert_composed(runtime(&original)["rows"].as_array().unwrap());
    assert_library(runtime(&original)["rows"].as_array().unwrap());
    assert_partial(runtime(&original)["rows"].as_array().unwrap());
    assert_unknown(runtime(&original)["rows"].as_array().unwrap());
    assert_replay(directory.path(), &original);
}

fn assert_known(original: &Value) {
    let report = runtime(original);
    assert_eq!(report["input_complete"], true);
    let rows = report["rows"].as_array().unwrap();
    assert!(
        !rows
            .iter()
            .any(|row| row["name"].as_str().unwrap().rsplit("::").next() == Some("generated_owner"))
    );
    for name in ["linear", "persistent", "reset", "square", "range", "slice"] {
        let row = named_row(rows, name);
        assert!(row["result"].get("Proven").is_some(), "{name}: {row}");
        assert!(row.get("observation").is_none());
    }
    assert_known_bounds(rows);
}

fn assert_known_bounds(rows: &[Value]) {
    for (name, expected) in [
        ("linear", 3),
        ("persistent", 8),
        ("reset", 18),
        ("square", 12),
        ("sequential", 8),
        ("repeated_slice", 12),
        ("accumulator", 3),
        ("practical_vec", 3),
        ("practical_map", 3),
        ("practical_match", 3),
        ("practical_iflet", 3),
        ("shared_vec", 3),
        ("shared_field_vec", 3),
        ("shared_field_slice", 3),
        ("stable_slice_binding", 3),
        ("stable_len_guard", 3),
        ("captured_closure", 3),
        ("medoid_chain", 12),
        ("pair_chain", 12),
        ("item_callback", 3),
        ("range_chain", 3),
        ("short_search", 3),
        ("bounded_prefix", 3),
        ("explicit_iter", 3),
        ("closure_return", 3),
        ("vec_collect", 3),
        ("mutable_receiver", 3),
    ] {
        let row = named_row(rows, name);
        let bound: rstyle_core::runtime_bounds::Bound =
            serde_json::from_value(row["result"]["Proven"]["loop_body_entries"].clone()).unwrap();
        assert_eq!(value(&bound), expected, "{name}: {bound:?}");
    }
}

fn assert_call(rows: &[Value]) {
    let call = named_row(rows, "unknown_call");
    let assessment = &call["result"]["Proven"];
    let bound: rstyle_core::runtime_bounds::Bound =
        serde_json::from_value(assessment["loop_body_entries"].clone()).unwrap();
    assert_eq!(value(&bound), 3);
    let calls = assessment["calls"].as_array().unwrap();
    let opaque = calls
        .iter()
        .find(|call| call["callee"].as_str().unwrap().ends_with("opaque"))
        .unwrap();
    let count: rstyle_core::runtime_bounds::Bound =
        serde_json::from_value(opaque["count"].clone()).unwrap();
    assert_eq!(value(&count), 3);
    assert!(
        assessment["whole_runtime"]
            .as_str()
            .unwrap()
            .starts_with("unknown")
    );
}

// Callback invocation counts compose across inlined closures and nested resets.
fn assert_callbacks(rows: &[Value]) {
    for (name, callee, expected) in [
        ("medoid_chain", "score", 9),
        ("pair_chain", "score", 9),
        ("item_callback", "double", 3),
        ("range_chain", "score", 3),
        ("short_search", "score", 3),
        ("bounded_prefix", "score", 3),
    ] {
        let assessment = &named_row(rows, name)["result"]["Proven"];
        let calls = assessment["calls"].as_array().unwrap();
        let call = calls
            .iter()
            .find(|call| call["callee"].as_str().unwrap().ends_with(callee))
            .unwrap_or_else(|| panic!("{name}: {assessment}"));
        let count: rstyle_core::runtime_bounds::Bound =
            serde_json::from_value(call["count"].clone()).unwrap();
        assert_eq!(value(&count), expected, "{name}: {count:?}");
        assert!(
            assessment["loops"][0]["source"]["evidence"]
                .as_str()
                .unwrap()
                .contains("iterator chain"),
            "{name}: {assessment}"
        );
    }
}

// Summaries substitute caller argument bounds; local counts stay separately reported.
fn assert_composed(rows: &[Value]) {
    for (name, entries, invocations) in [
        ("edge_caller", 12, 3),
        ("varying_calls", 30, 5),
        ("bag_sum", 20, 5),
        ("linear", 3, 0),
    ] {
        let composed = &named_row(rows, name)["composed"]["Proven"];
        let bound = |key: &str| -> u64 {
            value(
                &serde_json::from_value(composed[key].clone())
                    .unwrap_or_else(|_| panic!("{name}: {composed}")),
            )
        };
        assert_eq!(bound("loop_body_entries"), entries, "{name}: {composed}");
        assert_eq!(
            bound("workspace_invocations"),
            invocations,
            "{name}: {composed}"
        );
        assert!(composed["assumptions"].as_array().unwrap().len() >= 3);
    }
    let edge = &named_row(rows, "edge_caller")["composed"]["Proven"];
    assert!(
        edge["loop_body_entries"]
            .to_string()
            .contains("len(g.edges)"),
        "{edge}"
    );
    // Unsummarizable callees and unproven loops are explicit opaque factors.
    for (name, kind, reason) in [
        ("recursive_count", "Call", "Recursive"),
        ("dyn_area", "Call", "Unresolved"),
        ("unknown_continue", "Loop", "backedge can skip"),
        ("default_ne", "Call", "Unresolved"),
        ("item_to_std", "Call", "Unresolved"),
    ] {
        let composed = &named_row(rows, name)["composed"]["Partial"];
        let factors = composed["opaque_factors"].as_array().unwrap();
        assert!(
            factors.iter().any(|factor| factor["kind"] == kind
                && factor["reason"].as_str().unwrap().contains(reason)),
            "{name}: {composed}"
        );
    }
    // An owner with unknown local bounds has no composed bound at all.
    let drops = &named_row(rows, "drops")["composed"]["Unknown"]["reason"];
    assert!(drops.as_str().unwrap().contains("Local bounds"), "{drops}");
}

// A loop that cannot be proven becomes an opaque factor; sibling loops keep bounds.
fn composition(row: &Value) -> rstyle_core::runtime_bounds::compose::Composition {
    let composed: rstyle_core::runtime_bounds::compose::Composed =
        serde_json::from_value(row["composed"].clone()).unwrap();
    match composed {
        rstyle_core::runtime_bounds::compose::Composed::Proven(composition) => composition,
        other => panic!("{other:?}"),
    }
}

// Standard calls carry type-and-member keys; B-tree lookups cost log of a stable size.
fn assert_library(rows: &[Value]) {
    let lookups = named_row(rows, "btree_lookups");
    let calls = lookups["result"]["Proven"]["calls"].as_array().unwrap();
    assert!(
        calls
            .iter()
            .any(|call| call["operation"] == "rust:BTreeMap.contains_key"),
        "{lookups}"
    );
    let own = composition(lookups);
    assert_eq!(
        own.library_steps.to_string(),
        "len(keys)#1 * log(len(index.names))"
    );
    assert!(own.library_factors.is_empty(), "{own:?}");

    let caller = composition(named_row(rows, "lookup_caller"));
    assert_eq!(
        caller.library_steps.to_string(),
        "len(keys)#1 * log(len(index.names))"
    );
    assert_eq!(caller.library_steps.order().logarithmic, 1);

    // Primitive self types are keyed by kind; blanket impls over a parameter are not.
    let keys = named_row(rows, "str_keys");
    let calls = keys["result"]["Proven"]["calls"].as_array().unwrap();
    let operation = |suffix: &str| {
        calls
            .iter()
            .find(|call| call["callee"].as_str().unwrap().ends_with(suffix))
            .unwrap_or_else(|| panic!("{suffix}: {keys}"))
            .get("operation")
            .cloned()
    };
    assert_eq!(
        operation("is_empty"),
        Some(Value::from("rust:str.is_empty"))
    );
    assert_eq!(operation("into"), None);
    assert_eq!(
        composition(keys).library_steps.to_string(),
        "?library(std::convert::Into::into) + 1"
    );

    // A local map's size has no stable bound, so its lookups stay opaque.
    let local = composition(named_row(rows, "local_map"));
    assert!(!local.library_steps.is_complete());
    let kinds: Vec<_> = local
        .library_factors
        .iter()
        .map(|factor| factor.kind)
        .collect();
    assert!(
        kinds.contains(&rstyle_core::runtime_bounds::OpaqueKind::Input),
        "{local:?}"
    );
}

fn assert_partial(rows: &[Value]) {
    for (name, all_opaque) in [
        ("unknown_continue", true),
        ("limitwrites", true),
        ("overflow", true),
        ("stale", false),
        ("skipped", true),
        ("unknown_predicate", true),
        ("callee_continue", true),
        ("callee_reset", true),
        ("short_circuit_and", true),
        ("short_circuit_or", true),
    ] {
        let assessment = &named_row(rows, name)["result"]["Partial"];
        let loops = assessment["loops"]
            .as_array()
            .unwrap_or_else(|| panic!("{name}"));
        let opaque = loops.iter().filter(|item| item.get("opaque").is_some());
        assert!(opaque.count() >= 1, "{name}: {assessment}");
        if all_opaque {
            assert!(
                loops.iter().all(|item| item.get("opaque").is_some()),
                "{name}"
            );
        }
        let bound = assessment["loop_body_entries"].to_string();
        assert!(bound.contains("Opaque"), "{name}: {bound}");
    }
    // The saved `end = j` copies an incremented counter; the first scan keeps `n`.
    let stale = named_row(rows, "stale")["result"]["Partial"]["loops"]
        .as_array()
        .unwrap();
    let (opaque, proven): (Vec<_>, Vec<_>) =
        stale.iter().partition(|item| item.get("opaque").is_some());
    assert_eq!((opaque.len(), proven.len()), (1, 1), "{stale:?}");
    assert_eq!(proven[0]["body_entries"]["Input"], "n");
}

fn assert_unknown(rows: &[Value]) {
    for name in [
        "alias",
        "autoref",
        "autoref_reset",
        "refmut",
        "ergonomic_refmut",
        "metadata_rebind",
        "unsafe_body",
        "guarded_or",
        "pattern_guard_reset",
        "guard_call",
        "loop_local_extent",
        "containing_rebind",
        "mutable_callback_capture",
        "escaped_closure",
        "generic_callback",
        "lazy_callback",
        "mutable_source",
        "varying_inner",
        "unsafe_callback",
        "moved_mutable",
        "user_iterator",
        "flat_return",
        "fold_init_escape",
        "user_clone",
        "ordered_collect",
        "boxed_callback",
    ] {
        let row = named_row(rows, name);
        assert!(row["result"].get("Unknown").is_some(), "{name}: {row}");
    }
}

fn assert_replay(directory: &Path, original: &Value) {
    let report = runtime(original);
    let output = rstyle(
        directory,
        &[
            "patterns",
            "--format",
            "json",
            "--complexity",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
        ],
    );
    assert!(output.status.success(), "{}", stdout(&output));
    let replayed: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(runtime(&replayed), report);
    assert_source_free_replay(directory, original);
}

fn assert_source_free_replay(directory: &Path, original: &Value) {
    let report = runtime(original);
    std::fs::remove_dir_all(directory.join("src")).unwrap();
    std::fs::remove_file(directory.join("Cargo.toml")).unwrap();
    let output = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap())
        .current_dir(directory)
        .args([
            "patterns",
            "--format",
            "json",
            "--complexity",
            "--top",
            "1",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
        ])
        .env("PATH", "/runtime-replay-no-tools")
        .env("CARGO", "/runtime-replay-no-cargo")
        .env("RUSTC", "/runtime-replay-no-rustc")
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", stdout(&output));
    let listed: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(runtime(&listed)["selected"].as_array().unwrap().len(), 1);
    assert_eq!(runtime(&listed)["rows"], report["rows"]);
    assert_default_off(directory, original);
}

fn assert_default_off(directory: &Path, original: &Value) {
    let output = rstyle(
        directory,
        &[
            "patterns",
            "--format",
            "json",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
        ],
    );
    let off: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(output.status.success(), "{}", stdout(&output));
    assert!(
        off["compiler_advisory"]["units"][0]
            .get("runtime_bounds")
            .is_none()
    );
}

/// A derive macro shares its path with the trait it implements; the derived body,
/// recorded first, must not turn the trait into the macro and hide its implementations.
#[test]
fn derive_macros_keep_their_traits_implementations_selectable() {
    let directory = clone_workspace();
    let source = "#[derive(Default)] pub struct Settings { pub limit: usize }\n\
                  pub fn settings() -> Settings { Settings { ..Default::default() } }\n";
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let output = rstyle(
        directory.path(),
        &["patterns", "--format", "json", "--complexity"],
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(output.status.success(), "{}", stdout(&output));
    let rows = runtime(&report)["rows"].as_array().unwrap();
    let composed = &named_row(rows, "settings")["composed"]["Proven"];
    let callees = composed["resolved_callees"].as_array().unwrap();
    assert!(
        callees.iter().any(|callee| callee
            .as_str()
            .unwrap()
            .ends_with("clone_fixture::{impl#0}::default")),
        "{composed:#}"
    );
}

#[test]
fn old_capture_cannot_silently_supply_runtime_bounds() {
    let (directory, output, original) = capture("pub fn empty() {}", &["--compiler-advisory-only"]);
    assert!(output.status.success(), "{}", stdout(&output));
    assert!(
        original["compiler_advisory"]["units"][0]
            .get("runtime_bounds")
            .is_none()
    );
    let output = rstyle(
        directory.path(),
        &[
            "clones",
            "--format",
            "json",
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--compiler-runtime-bounds",
            "--compiler-advisory-only",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
        ],
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("missing compiler runtime-bound capture capability")
    );
}

fn value(bound: &rstyle_core::runtime_bounds::Bound) -> u64 {
    // Inputs named `...m` are 5 and all others 3.
    let input = |name: &str| Some(if name.ends_with('m') { 5 } else { 3 });
    bound
        .evaluate(&input)
        .unwrap_or_else(|| panic!("opaque factor in {bound}"))
}
