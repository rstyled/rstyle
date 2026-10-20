# Rust feature coverage and roadmap

A practical master checklist for processing real-world Rust through the **strict
HIR source-to-pair (`cargo rstyle clones`) pipeline**. This is not a checklist for
legacy `patterns`, a Rust language specification, or a claim of complete Rust
semantics. Real-world coverage is a ladder, not one final syntax switch.

Audited against `4a0e346` (2026-10-15), after the performance work. The latest
implemented selection is **`hir-data-v3`**, extraction
`rstyle.rust.hir-data/3.32.0`, matching `rstyle.ccgraph.data-calls/1.0.0`.
The default remains `r0`; `hir-expanded-v1` does **not** select the Data features.
The continuation log below records implementation boundaries and frozen inventory results.

For a hands-on example of taking a Missing row through representation, registration,
verification and tests, see [adding float support by hand](docs/adding-float-support.md).

## How to read the checklist

| Status | Meaning |
| --- | --- |
| **Covered** | A connected implementation exists for the deliberately narrow feature named in the row, with recorded correctness tests. It is not exhaustive certification of every composition, target, or semantic setting. |
| **Partial** | A useful subset exists, but common forms or required proof are missing. The last column states the boundary. |
| **Missing** | No admitted implementation for the feature is established in the current strict pipeline. Compiler observation or a draft contract is not support. |

All statuses refer to the latest Data-v3 selection unless a row says otherwise.
Examples are fragments, not independently runnable fixtures. Names such as `S`,
`helper`, and `read_field` stand for appropriately declared items. An example
illustrates a capability, **not a promise that its surrounding function or crate
will admit**. Every other operation, interface, source owner, and dependency must
also be supported. `Covered` rows remain subject to the cross-cutting limitations
below, particularly the conservative nominal-unit proof.

Two kinds of remaining work are distinguished:

- **Read-only milestone:** unfinished work from the existing compositional-data
  goal: construction, local value/reference correspondence, indexing/subslicing,
  concrete Option, assertions, richer scalar patterns/constants, and cross-unit
  calls. The existing draft is not an admission contract for these missing pieces.
- **Broader Rust tiers:** mutation/ownership/drop, general traits/generics/enums,
  iterators/closures, async, concurrency, unsafe and external effects. These belong
  on the master roadmap, but are not implied commitments of the read-only milestone.

The IDs below are documentation tracking keys, not extraction capability tokens.

## 1. Primitive values, bindings, and expressions

| ID | Feature | Status | Small example | Current boundary / remaining work |
| --- | --- | --- | --- | --- |
| V01 | Integer, bool, and unit interfaces | Covered | `fn f(x: i32, b: bool) -> i32` | Resolved scalar types; includes unused inputs and unit results. |
| V02 | Fixed-width and native-width integers | Covered | `u128`, `isize`, `usize` | Native-width compatibility requires known equal pointer widths where relevant; exhaustive operator/type/target testing remains outstanding. |
| V03 | Scalar literals and built-in operators | Covered | `x + 1`, `x < y`, `!flag` | Admitted integer/bool dispatch only; not overloaded operators. Arithmetic hazards can retain partial-flow evidence. |
| V04 | Initialized scalar locals, shadowing, and reads | Covered | `let x = n; let x = x + 1;` | Exact binding identity, saved values, and reaching definitions. |
| V05 | Local scalar assignment and compound update | Covered | `x = y; x += 1;` | Lexical scalar bindings, not field/index/dereference memory writes. |
| V06 | Scalar blocks and expression tails | Covered | `{ let y = x; y }` | Normal-flow evaluation and saved operands are retained. |
| V07 | Integer-to-integer casts | Covered | `x as u64` | Resolved built-in integer casts only. |
| V08 | Ordinary const-function bodies | Covered | `const fn f(x: u32) -> u32 { x }` | Function qualification is supported; this is not general compile-time evaluation. |
| V09 | Floats and floating-point operations | Missing | `x: f64`, `x / 2.0` | Type, literal, operation, and comparison semantics; NaNs, signed zero, and casts must not be treated as integer semantics. |
| V10 | Character values and operations | Missing | `c: char`, `'a'` | Unicode scalar types/literals and relevant calls/conversions. |
| V11 | Additional built-in casts/coercions | Missing | `flag as u8`, `x as f32` | Bool/char/float conversions are outside the current integer-cast boundary; pointer casts belong to unsafe work. |
| V12 | Named constants and associated constants | Partial | `x + LIMIT`, `S::LIMIT` | Data-v3 3.3.0 supports same-unit primitive scalar constants and nongeneric inherent constants, evaluated by rustc; see [constant contract](docs/extraction-profiles/scalar-constants-v1.md). Nonlocal/generic/trait constants and aggregate values remain missing. |
| V13 | General const expressions and const blocks | Missing | `[u8; N + 1]`, `const { 1 + 2 }` | Current array types require literal lengths; const fn support does not authorize this. |
| V14 | Uninitialized locals / definite assignment | Missing | `let x; x = 1; x` | Declaration without initializer is currently rejected. |
| V15 | Destructuring and wildcard bindings | Missing | `let (a, b) = pair;`, `let _ = x;` | Named single-binding patterns are the supported declaration/parameter grammar; unit value discard is not general wildcard-pattern support. |
| V16 | Type aliases | Missing | `type Count = u32;` | Alias declaration/use correspondence, not merely an import renamed with `use ... as ...`. |
| V17 | General never-type interfaces | Missing | `fn stop() -> !` | Diverging loop control exists, but a general `!` interface/coercion boundary is not established. |

Evidence: [R0 ledger](HIR_EXTRACTION_COVERAGE.md),
[expanded contracts](docs/extraction-profiles/coverage-v1.md),
[eligibility](crates/rstyle-pdg/src/eligibility.rs).

## 2. Branches, patterns, loops, and error propagation

| ID | Feature | Status | Small example | Current boundary / remaining work |
| --- | --- | --- | --- | --- |
| F01 | Boolean `if`/`else` and explicit return | Covered | `if b { return x; } y` | Includes returning arms, control dependence, and unreachable supported suffixes. |
| F02 | Acyclic conditional expression values | Covered | `let n = if b { x } else { y };` | Branch-value availability and merges; see F10 for the loop restriction. |
| F03 | Short-circuit boolean evaluation | Covered | `a && b`, `a \|\| b` | Saved left values and conditional right evaluation; restricted in loop-containing functions. |
| F04 | Bounded scalar literal/wildcard match | Partial | `match n { -1 => x, _ => y }` | Integer/bool scrutinee and 1–16 arms; Data-v3 3.9.0 independently proves coverage of supported unguarded patterns without requiring a wildcard. See [exhaustive scalar contract](docs/extraction-profiles/exhaustive-scalar-matches-v1.md); broader pattern domains remain missing. |
| F05 | Scalar match guards | Partial | `match n { 0 if b => x, _ => y }` | Ordered pure scalar guard subset; arbitrary effectful guards and patterns are not supported. |
| F06 | OR/range/constant patterns | Partial | `1 \| 2 => x`, `1..=9 => y`, `LIMIT => z` | Data-v3 3.10.0 supports integer/bool literal/constant/range/wildcard leaves in nested grouped OR patterns, guards and independently exhaustive matches; see [nested contract](docs/extraction-profiles/nested-scalar-or-patterns-v1.md). Character ranges and nonlocal/generic/trait constants remain missing. |
| F07 | Destructuring, reference, slice, and enum patterns | Missing | `Some(x) => x`, `[a, ..] => a` | Payload/field/element dependencies, binding modes, and ownership/borrow effects. |
| F08 | `if let`, `while let`, let chains, `let else` | Missing | `if let Some(x) = v { ... }` | Pattern decision/binding flow and divergence requirements; not ordinary boolean branching. |
| F09 | Scalar `while`/`loop` and innermost exits | Covered | `while n > 0 { n -= 1; }` | Scalar state, nesting, early return, unlabelled valueless `break`/`continue`; divergence-sensitive flow. |
| F10 | Value merges in loop-containing functions | Missing | `while b { n = if c { x } else { y }; }` | Current blanket exclusion includes nonunit conditional and short-circuit merges anywhere in a function containing a loop. Separate safe outside-cycle merges from epoch-sensitive cyclic ones. |
| F11 | Labelled and value-carrying exits | Missing | `'outer: loop { break 'outer; }`, `break x` | Exact target resolution and value/frontier dependencies; also labelled block expressions. |
| F12 | Iterator `for` loops | Missing | `for x in xs { ... }` | Iterator protocol, yielded values, effects, drop, and source/desugaring correspondence; not covered by scalar `while`. |
| F13 | `Option`/`Result` branching and `?` | Missing | `let x = parse()?;` | Concrete discriminants/payloads, early return, conversions, and cleanup; `?` is not merely a scalar branch. |
| F14 | Assertion macros | Partial | `assert!(b);`, `debug_assert!(b);` | Data-v3 3.6.0 supports unformatted standard assertions, original invocation/condition correspondence, one condition evaluation, debug settings and disclosed omitted panic flow; see [assertion contract](docs/extraction-profiles/scalar-assertions-v1.md). Formatting/comparison macros and general exceptional flow remain missing. |
| F15 | General panic/unwind/cleanup flow | Missing | `panic!("bad")`, `catch_unwind(f)` | Current extraction is bounded normal flow; omitted applicable exceptional flow is disclosed, not represented as complete. |

Evidence: [cycle/match contracts](docs/extraction-profiles/coverage-v1.md),
[planned scalar composition](docs/extraction-profiles/compositional-data-v1.md),
[eligibility](crates/rstyle-pdg/src/eligibility.rs).

## 3. Concrete data, places, references, and memory

| ID | Feature | Status | Small example | Current boundary / remaining work |
| --- | --- | --- | --- | --- |
| D01 | Tuple and literal-length array interfaces | Covered | `p: (u32, bool)`, `a: [u8; 4]` | Interface support is distinct from construction, indexing, and reference-bearing element origins. |
| D02 | Concrete local struct interfaces | Partial | `s: S`, `s: &S` | Finite supported field trees and independent closed-unit/drop-free proof; real crates often fail that unit-wide boundary. |
| D03 | Shared scalar/tuple/array/slice/str interfaces | Covered | `xs: &[u8]`, `text: &str` | Supported concrete immutable referents; shared syntax alone does not establish effects. |
| D04 | Shared inherent receivers / getter bodies | Partial | `fn value(&self) -> u32 { self.value }` | Exact source impl ownership, supported concrete fields, and nominal-unit proof; not general receiver forms or callable methods on every impl. |
| D05 | Formal tuple/struct field reads | Covered | `p.0`, `s.inner.value` | Exact formal roots and ordered projections, including actual stored-reference loads where required. |
| D06 | Shared borrows of formal field places | Covered | `&s.inner.value` | Address projections do not invent value loads; grouping is supported in the registered receiver forms. |
| D07 | Borrowed outputs and branching formal origins | Covered | `if b { left } else { right }` | Shared outputs retain every supported formal-origin alternative, including verified field origins. |
| D08 | Stored shared-reference fields | Covered | `s.bytes`, where the field is `&[u8]` | Distinguishes loading a reference from borrowing its storage and reading its referent. |
| D09 | By-value reference-bearing struct/tuple inputs | Partial | `s: Borrowed<'_>` | Verified formal component origins; Data-v3 3.22.0 also supports [construction and whole immutable tuple/struct locals](docs/extraction-profiles/reference-bearing-values-v1.md). [Component projections from constructed/local values](docs/extraction-profiles/reference-value-fields-v1.md) compose in Data-v3 3.23.0; mutable rebindings and general by-value memory-reading helper substitution remain missing. |
| D10 | Lifetime-only nominal declarations | Partial | `struct View<'a> { bytes: &'a [u8] }` | Unbounded lifetime-only grammar and exact region/arity proof; no type/const parameters, bounds, or where clauses. Callable inherent impls have a narrower nongeneric boundary. |
| D11 | Aggregate construction and struct update | Partial | `S { value: x }`, `(x, y)`, `[x; 4]`, `S { value: x, ..old }` | Data-v3 3.22.0 additionally supports [reference-bearing tuple/named/tuple-struct values](docs/extraction-profiles/reference-bearing-values-v1.md). Ordered reference-free tuple/explicit array literals, repeat arrays with literal counts, named/tuple/unit structs and same-typed struct updates remain supported; see [literal construction](docs/extraction-profiles/aggregate-literal-values-v1.md), [repeats](docs/extraction-profiles/repeat-array-values-v1.md), [named structs](docs/extraction-profiles/named-struct-values-v1.md), [updates](docs/extraction-profiles/struct-update-values-v1.md), [tuple structs](docs/extraction-profiles/tuple-struct-values-v1.md) and [unit structs](docs/extraction-profiles/unit-struct-values-v1.md). Data-v3 3.24.0 also supports [reference-bearing struct updates](docs/extraction-profiles/reference-struct-updates-v1.md) with component-specific origin replacement. Broader const counts and reference-bearing arrays remain unfinished. Data-v3 3.27.0 supports [whole reference-bearing rebindings](docs/extraction-profiles/reference-rebindings-v1.md) with snapshot and component origins. |
| D12 | General non-scalar local values/reference rebindings | Partial | `let r = s.bytes; r.len()`, `let t = s; t.value` | Data-v3 3.22.0 supports [whole immutable reference-bearing tuple/struct values](docs/extraction-profiles/reference-bearing-values-v1.md), [single-origin shared-local field reads](docs/extraction-profiles/shared-local-fields-v1.md), [immutable shared-reference locals](docs/extraction-profiles/shared-reference-locals-v1.md), [whole reference-free local values](docs/extraction-profiles/local-aggregate-values-v1.md), assignments, shadowing, existing branch/loop flow and [value field projections](docs/extraction-profiles/aggregate-value-fields-v1.md). Data-v3 3.23.0 additionally supports [component projections from reference-bearing aggregate values](docs/extraction-profiles/reference-value-fields-v1.md). Data-v3 3.27.0 adds [whole reference-bearing rebindings](docs/extraction-profiles/reference-rebindings-v1.md), snapshot copies and branch/loop origin flow. Data-v3 3.28.0 adds [indirect shared-reference field reads](docs/extraction-profiles/indirect-field-reads-v1.md) through multiple origins, mutable bindings, snapshots and shared helper results. Aggregate helper-result field bases remain unfinished. |
| D13 | Borrows of locals and temporaries | Missing | `let r = &local;`, `&S { value: x }` | Storage identity/lifetime, temporary scope, escape/origin proof, and local value dependence. |
| D14 | Explicit and broader implicit dereference | Partial | `*r`, `nested_ref.field` | Data-v3 3.29.0 supports [built-in shared dereference reads](docs/extraction-profiles/shared-dereference-reads-v1.md), including nested pointer loads, physical field places, snapshot/branch/loop flow and existing helper composition. Unsized loads, overloaded dereference and general repeated implicit dereferences remain unfinished. |
| D15 | Array/slice element reads | Partial | `xs[i]` | Data-v3 3.30.0 supports [scalar index reads](docs/extraction-profiles/scalar-index-reads-v1.md) over owned arrays and shared arrays/slices, evaluated place/value snapshots, access-relative element selectors and explicit normal-completion bounds requirements. Aggregate/reference elements and complete exceptional flow remain unfinished. |
| D16 | Slice ranges and borrowed subslices | Partial | `&xs[start..end]` | Data-v3 3.31.0 supports [bounded shared subslice views](docs/extraction-profiles/shared-subslice-views-v1.md) with scalar elements, exact base/start/end snapshots and normal-completion bounds. Arrays, open/inclusive ranges, mutable views, str/UTF-8 rules and transport through helper summaries remain unfinished. |
| D17 | Checked access and concrete Option | Missing | `xs.get(i)`, `Some(x)`, `None` | Registered access semantics plus construction/discriminants/bounded payload extraction; part of the read-only goal, not general enum support. |
| D18 | Reference-bearing array element origins | Missing | `refs: [&S; 2]` | Type observation alone lacks the per-element origin evidence needed for by-value reference-bearing arrays. |
| D19 | General enums and variants | Missing | `enum E { A(u32), B }` | Variant identity, construction, discriminants, payloads, pattern bindings, moves and drop; includes Result and enum-based APIs beyond concrete Option. |
| D20 | Mutable references and reborrowing | Missing | `x: &mut S`, `&mut *x` | Mutable locations, alias/reborrow relationships and read/write effects; beyond the current read-only milestone. |
| D21 | Field/index/dereference memory writes | Missing | `s.value = x;`, `xs[i] += 1;`, `*r = x;` | Memory reaching definitions, alias-sensitive dependencies, address/value evaluation order and invalidation. Local scalar writes already work. |
| D22 | Heap-backed containers and allocation | Missing | `Vec<u8>`, `String`, `Box<S>`, `HashMap<K, V>` | Types, allocation/reallocation, ownership, library effects and drop; no general container summaries exist. |
| D23 | Moves, partial moves, and nontrivial ownership | Missing | `let field = owned.field;` | Current finite drop-free values are not a general ownership/move model; track storage validity and subsequent uses. |
| D24 | Destructors and scope cleanup | Missing | `impl Drop for S { ... }`, `drop(value)` | Implicit/explicit drop on normal/early/panic paths; current supported nominal boundary excludes these effects. |
| D25 | Interior mutability and shared ownership | Missing | `Cell<T>`, `RefCell<T>`, `Rc<T>`, `Arc<T>` | Shared references can mutate; borrowing checks, reference counts, allocation, effects and synchronization need explicit semantics. |
| D26 | Globals, statics, and thread-local storage | Missing | `static COUNT: ...`, `thread_local! { ... }` | Persistent location identity, initialization, reads/writes, synchronization and cross-call effects. |

Evidence: [Data contract](docs/extraction-profiles/data-v1.md),
[field context](docs/extraction-profiles/field-value-context-v1.md),
[nominal proof](docs/extraction-profiles/nominal-data-v1.md),
[lifetime nominals](docs/extraction-profiles/lifetime-nominals-v1.md),
[data-path verification](crates/rstyle-pdg/src/data_frontend.rs).

## 4. Calls, methods, closures, and library operations

| ID | Feature | Status | Small example | Current boundary / remaining work |
| --- | --- | --- | --- | --- |
| C01 | Same-unit direct scalar helpers | Covered | `helper(x)` | Resolved definition and verified summary, source-order arguments, no inlining. Callee must be supported, acyclic, loop-free, nonpanicking and returning. Zero arguments and unit results work; unit-valued argument positions remain unsupported across the helper boundary, including later helper rows. |
| C02 | Qualified paths and named import aliases | Covered | `module::helper(x)`, `use module::helper as h; h(x)` | Supported same-unit resolution; this is not general type aliases, glob imports, or external linking. |
| C03 | Memory-free helpers with shared inputs | Covered | `choose(left, right, flag)` | Supported canonical shared types and actual arguments; shared input alone does not authorize memory reads. |
| C04 | Whole shared-formal helper results | Covered | `identity_ref(xs).len()` | Verified selected-formal return origins and actual substitution; not a field-reading borrowed-result summary. |
| C05 | Direct memory-reading shared-input helpers | Covered | `read_field(s)` | Data-v2/v3: scalar/unit results, exact per-occurrence reads and actual-origin substitution, preserving alias uncertainty. |
| C06 | Concrete inherent method calls and UFCS | Partial | `s.value()`, `S::value(s)` | Data-v3: same-unit nongeneric inherent impl/method, shared receiver or [static associated calls](docs/extraction-profiles/static-inherent-helper-calls-v1.md), exact owner/source proof and supported summary. Shared receivers require adjustment proof. No trait dispatch. |
| C07 | Receiver adjustments and coercions | Partial | `s.value()` | Data-v3 inherent helper method syntax requires an exact shared type and one builtin shared dereference followed by one shared borrow. Registered integer methods have scalar by-value receivers; metadata methods also accept an empty adjustment chain. General autoref, repeated/overloaded deref, unsizing and mutable/by-value inherent receivers remain unavailable. |
| C08 | Registered integer methods | Partial | `x.unsigned_abs()`, `x.wrapping_mul(y)` | Four instantiations only: i32/i64 unsigned_abs and u32/u64 wrapping_mul. Not general primitive-method support. |
| C09 | Slice/str length and emptiness | Covered | `xs.len()`, `text.is_empty()` | Four registered nonmutating operations; exact supported compiler/core/target resolution and receiver evidence. Slice elements must be supported non-reference-bearing types; a shared-slice interface alone does not authorize these calls on reference-bearing elements. No buffer-content read is invented. |
| C10 | General core/std/third-party APIs | Partial | `u32::from_ne_bytes(bytes)`, `hint::unlikely(b)` | Data-v3 3.32.0 connects [copied four-byte slice conversion, concrete Result/unwrap and native u32 conversion](docs/extraction-profiles/copied-byte-windows-v1.md) under pinned aarch64 core authority. Other APIs need resolved summaries/bodies and effect/result proof; familiar names are insufficient. |
| C11 | By-value memory-reading helper arguments/results | Missing | `inspect_value(s)`, `make_value(x)` | Pure reference-free construction results compose in the D11 fixture; by-value memory-reading substitution, reference-bearing helper results and ownership remain unavailable. Supported interface types do not establish that call boundary. |
| C12 | Borrowed results from memory-reading helpers | Missing | `get_field_ref(s)` | Exact callee read effects plus result origin/location substitution, not just whole-formal forwarding. |
| C13 | Cross-unit/package helper linking | Missing | `library_crate::helper(x)` | Capture/resolve selected units before summary derivation, preserve build contexts/transitive dependencies; modules inside one unit already work. |
| C14 | Looping, recursive, panicking or nonreturning callees | Missing | `recursive(x)`, `helper_with_loop(x)` | Existing caller scalar loops do not authorize these summaries; need interprocedural fixed points/flow/effect contracts. |
| C15 | Function items as values, pointers and indirect calls | Missing | `let f: fn(u32) -> u32 = helper; f(x)` | Callable identity/possible targets, concrete signatures, effects and indirect-call dependencies. |
| C16 | Closures and captured environments | Missing | `move \|x\| x + offset` | Separate body/environment owners, capture modes, invocation, effects, escape/lifetime and drop. Legacy patterns behavior is not strict-pipeline support. |
| C17 | Iterator adapters and higher-order functions | Missing | `xs.iter().map(f).collect()` | Protocol/call composition, closures, state, short-circuiting, yielded values and cleanup; separate from plain scalar loops. |
| C18 | Side-effecting calls | Missing | `std::fs::write(path, bytes)`, `println!("x")` | IO/global effects, ordering and failure behavior; distinguish macro provenance from the underlying call effects. |

Evidence: [helper contract](docs/extraction-profiles/helpers-v1.md),
[shared/helper origin boundary](docs/extraction-profiles/read-only-helper-calls-v1.md),
[read-only calls](docs/extraction-profiles/data-calls-v1.md),
[inherent calls](docs/extraction-profiles/inherent-read-only-calls-v1.md),
[current library authority](docs/extraction-profiles/read-only-library-resolution-v2.md).

## 5. Traits, generics, and advanced type resolution

These are broader Rust tiers, except the already implemented lifetime-only subset.
A compiler-resolved call is not automatically a verified call summary.

| ID | Feature | Status | Small example | Current boundary / remaining work |
| --- | --- | --- | --- | --- |
| T01 | Concrete trait implementations and default methods | Missing | `impl Trait for S`, `s.trait_method()` | Exact trait/impl/default-body ownership, selected implementation, instantiated interface and effects. Inherent support does not cover this. |
| T02 | Type-generic functions, structs and impls | Missing | `fn f<T>(x: T)`, `struct S<T>(T)` | Type parameter/substitution evidence and a declared choice of generic-body versus concrete-instantiation analysis. |
| T03 | Bounds, where clauses, blanket and supertrait resolution | Missing | `fn f<T: Read>(x: T)`, `where T: Trait` | Obligation/selection evidence, body behavior and dependency identity; not just signature strings. |
| T04 | Const generics | Missing | `fn f<const N: usize>(x: [u8; N])` | Constant arguments, array/type instantiation and source/value proof. |
| T05 | General lifetime parameters and bounds | Partial | `fn id<'a>(x: &'a [u8]) -> &'a [u8]` | Narrow late-bound shared-formal helpers and lifetime-only nominals exist. Bounded/early-bound impl substitutions and general lifetime-bearing APIs remain unsupported. |
| T06 | Associated types/constants, qualified projections and GATs | Missing | `<T as Trait>::Item`, `type Item<'a>` | Owner/trait substitution, normalization, source identity and concrete value/type evidence. |
| T07 | Trait objects and dynamic dispatch | Missing | `x: &dyn Read`, `x.read(buf)` | Vtable/possible-target abstraction, object types, effects and conservative dispatch dependencies. |
| T08 | Opaque types and `impl Trait` | Missing | `fn values() -> impl Iterator<Item = u8>` | Hidden concrete type/body evidence, capture/identity and trait behavior. |
| T09 | Overloaded operators, indexing and deref | Missing | `a + b` on a struct, custom `Index`, `Deref` | Resolve trait calls and effects; must not reuse built-in scalar or field-location semantics without proof. |
| T10 | Higher-ranked bounds and callable traits | Missing | `for<'a> Fn(&'a S)`, `FnMut`, `FnOnce` | Binder/substitution, environment ownership, receiver/capture effects and invocation semantics. |

Evidence: [type/source boundary](docs/extraction-profiles/data-source-types-v1.md),
[lifetime subset](docs/extraction-profiles/lifetime-nominals-v1.md),
[inherent exclusions](docs/extraction-profiles/inherent-read-only-calls-v1.md).

## 6. Macros, modules, conditional compilation, and source ownership

These are admission prerequisites, not optional polish: a supported expression
inside an unsupported source/effect context can still be unavailable.

| ID | Feature | Status | Small example | Current boundary / remaining work |
| --- | --- | --- | --- | --- |
| S01 | Exact original source and function identity | Covered | UTF-8 names, CRLF files, same-named functions | Source buffers/ranges, original byte offsets, distinct owners/build contexts and navigation; generated-owner identity is not general expansion support. |
| S02 | Ordinary modules and named imports | Partial | `mod child;`, `use child::S as Local;` | Closed inline/external module census and supported lexical aliases; nominal census rejects module attributes and unresolved/glob/external paths. |
| S03 | Active conditional-source correspondence | Partial | A cfg-controlled statement present in resolved HIR | Original-source/HIR correspondence in the actual build, with Data-v3 3.7.0 resolution of body-level cfg removal; general conditional signatures/attributes remain unfinished. |
| S04 | Cfg-removed source and inactive definitions | Partial | `#[cfg(feature = "x")] statement` absent from HIR | Data-v3 3.8.0 resolves standard cfg/cfg_attr guards for removed body operations and definitions in captured original files, including enclosing scopes; see [removal contract](docs/extraction-profiles/cfg-source-removal-v1.md). Unloaded external files and macro-token definitions remain unfinished; missing capture is not proof. |
| S05 | Conditional/path-selected modules | Missing | `#[path = "other.rs"] mod m;` | Resolved module presence/path/ownership; the current nominal census rejects all module attributes. |
| S06 | Declarative macros | Missing | `macro_rules! m { ... }`, `m!()` | Original invocation, expansion ownership, argument/operation correspondence and hidden item/effect evidence. Assertions are a proposed narrow first slice. |
| S07 | Derives and procedural/attribute macros | Missing | `#[derive(Clone)]`, `#[some_attribute]` | Generated definitions/impls, expansion provenance, source correspondence and effects; source spelling alone is insufficient. |
| S08 | Included/build-generated code | Missing | `include!(...)`, generated `OUT_DIR` files | Expansion/input/build ownership and original-source identity beyond the current admitted subset. Build scripts compiling is not graph support for generated source. |
| S09 | Nominal proof in ordinary mixed-content crates | Partial | A getter beside an enum, const, trait impl or body macro | Current unit whitelist accepts ordinary functions, structs, modules, uses and restricted inherent impls. Data-v3 3.11.0 additionally accepts constant declarations and inherent associated constants with recursive initializer checks; see [constant-context contract](docs/extraction-profiles/nominal-constant-context-v1.md). Data-v3 3.25.0 additionally supports [nominal types used only in bodies](docs/extraction-profiles/body-nominal-types-v1.md), including scalar-returning construction/projection without a signature witness. Other items, macros/derives and unsafe forms can still block simple APIs; finer ownership/drop/effect proofs remain. |
| S10 | Nested item declarations | Missing | `fn outer() { fn inner() {} ... }` | Owners can be distinguished, but the outer function's nested item is still unsupported; no certified supported remainder is emitted. |
| S11 | Broader attributes, item forms and import resolution | Partial | `use m::*;`, type aliases, associated items | Basic supported paths and benign item attributes exist; independent namespace/presence/expansion checks do not yet cover general Rust items and imports. |

Evidence: [conditional boundary](docs/extraction-profiles/coverage-v1.md),
[nominal proof](docs/extraction-profiles/nominal-data-v1.md),
[current census whitelist](crates/rstyle-pdg/src/nominal_boundary/census.rs).

## 7. Async, concurrency, unsafe Rust, and external boundaries

These are later tiers. They require explicit analysis contracts; accepting
lowered/desugared syntax alone would not establish correct representation.

| ID | Feature | Status | Small example | Current boundary / remaining work |
| --- | --- | --- | --- | --- |
| A01 | Async functions, async blocks and await | Missing | `async fn f()`, `future.await` | Future state/captures, poll/suspension/resumption dependencies and source correspondence. |
| A02 | Async cancellation, pinning and state cleanup | Missing | `Pin<&mut Future>`, dropping a pending future | Pin/address guarantees, cancellation/drop paths and effects across suspension. |
| A03 | Coroutines and yield | Missing | `yield value` | Suspension, yielded/resumed values, state ownership and completion. |
| A04 | Threads, channels, locks and atomics | Missing | `thread::spawn(f)`, `mutex.lock()`, `atomic.load(...)` | Shared state, synchronization, memory ordering and interleaving abstraction; ordinary sequential edges are not a concurrency model. |
| U01 | Unsafe functions/blocks and raw pointers | Missing | `unsafe { *ptr }` | Pointer provenance/aliasing, validity, unsafe preconditions and effects; currently rejected explicitly. |
| U02 | Layout-sensitive operations and unions | Missing | `transmute`, union field access, pointer arithmetic | Layout/initialization/alignment, representation, validity and target-specific semantics. |
| U03 | Foreign calls and non-Rust ABIs | Missing | `extern "C" fn`, calling a C function | ABI/interface representation and external effect contracts; non-Rust ABI is currently rejected. |
| U04 | Intrinsics, inline assembly and platform operations | Missing | `asm!(...)`, volatile accesses, SIMD intrinsics | Target-specific inputs/outputs, memory/clobber effects and unsupported-operation evidence. |

Evidence: [signature/expression exclusions](crates/rstyle-pdg/src/eligibility.rs).
Later-tier descriptions are roadmap requirements, not frozen implementation designs.

## 8. Cross-cutting graph, verification, and practical-coverage obligations

These are not all Rust syntax features, but without them a feature checklist can
look complete while real functions still cannot be processed reliably.

| ID | Capability | Status | Example / check | Current boundary / remaining work |
| --- | --- | --- | --- | --- |
| X01 | Scalar CFG, reaching definitions, control/execution dependencies | Covered | Branch writes reach the correct later reads | Exact supported normal-flow facts, fixed points for supported cycles and independent serialized checks; no general Rust semantic oracle. |
| X02 | Memory locations, origins and alias uncertainty | Partial | Two shared inputs may refer to the same storage | Formal read-only field/origin model exists; general local/heap/mutable/global locations and alias-sensitive writes remain missing. |
| X03 | Complete owned-source assessment | Partial | Unsupported code after `return` still blocks certification | No supported-remainder emission; source discovery/expansion/cfg gaps remain. This is intentionally stricter than accepting only reachable operations. |
| X04 | Compiler-independent graph handoff and strict import | Covered | Malformed source/manifest/graph evidence is rejected | Connected production path and independent import boundaries exist; local compiler trust and arbitrary imports have different documented assumptions. |
| X05 | Semantic build/target compatibility | Partial | Different overflow settings or pointer widths | Known required settings are gated; all cross-target/operator/settings combinations are not independently verified. Registered library operations support specific compiler/target identities. |
| X06 | Interprocedural dependencies and invalidation | Partial | Editing a getter invalidates its callers | Supported same-unit acyclic dependencies work; cross-unit linking, recursion and broader effects remain missing. |
| X07 | Resource bounds and explicit incomplete execution | Covered | Evidence or pair-budget exhaustion is not success | Checked budgets, no silently truncated successful result; broader stress/resource coverage and practical scalability remain ongoing obligations. |
| X08 | Pair projection and exhaustive pair decisions | Covered | 21 admitted functions yield 210 unordered decisions | Existing gates/compatibility can reject a pair. Graph admission is not a positive clone finding; budgets can leave execution explicitly incomplete. |
| X09 | Current natural-crate coverage inventory | Covered | Rerun frozen samples with Data-v3 | Frozen Data-v3 3.2 inventory: 9 admitted, 44 excluded, 7 unknown out of 60; see [refresh report](eval/hir-coverage/REPORT-data-v3.md). |
| X10 | Independent broader applicability targets | Missing | Freeze in-scope tiers and expected admissions before running | Broader data draft calls for a source-only inventory and declared acceptance targets; no completion claim follows from constructed fixtures. |
| X11 | Held-out detection quality/calibration | Missing | Untouched project/family holdouts after expansion | Historical studies are not post-Data-v3 quality evidence. Keep separate from feature correctness and coverage. |
| X12 | General equivalence, purity or rewrite safety | Missing | “These functions can safely be replaced by one abstraction” | Not promised by graph extraction or a clone score; requires a separate contract and analysis, not just more syntax coverage. |

Evidence: [source-to-pair workflow](docs/source-to-pair.md),
[coverage ledger](HIR_EXTRACTION_COVERAGE.md),
[ownership-corrected inventory](eval/hir-coverage/REPORT-v2.md),
[performance acceptance audit](docs/performance/hash-and-matching-acceptance.md).

## What this means for the existing sample functions

These are source-derived blocker combinations, **not a new Data-v3 run or predicted
admission gains**. Other interface/unit/source restrictions can independently block
the same function. Frozen source is in
[`selection.json`](eval/hir-coverage/selection.json).

| Sample family / functions | Relevant remaining rows | Why one extra node kind will not be enough |
| --- | --- | --- |
| `regex-syntax`: `Span::new`, `Position::new`, `with_start`, `with_end`, `Span::splat` | D11–D12, C11, S09 | Constructors/updates, aggregate propagation and constructor-call results must compose; mixed crate contents can block nominal proof first. |
| `time-core`: cfg-selected `is_leap_year`, `days_in_month_leap` | F14, C10, S03–S04 | Assertions, additional resolved calls and conditional-source evidence occur together. |
| `time-core`: `weeks_in_year` | F06 | Large negative/positive OR-patterns go beyond the current literal/wildcard match boundary because of the alternatives; individual negative integer literal patterns already work. |
| `lz4_flex`: safe `get_batch`, `get_batch_arch` | D12, D14–D16, C10, V12–V13 | Subslice origins, local values/references, conversions/unwrap/byte methods, and in some variants explicit dereference or named array lengths. |
| `lz4_flex`: `count_same_bytes` and its tail helper | D12, D16, D20–D21, F12, C10, C16–C17, S10 | Buffer slices, iterator/closure composition, mutation, conversions and nested ownership; reaches beyond the read-only milestone. |
| `lz4_flex`: `backtrack_match` | D15, D20–D21 | Indexing plus mutable-reference reads/writes. |
| `regex-syntax`: `auxiliary_span`, `partial_cmp` | D17, D19, T01, T09, S09 | Option construction is only part of the problem: enum payloads and trait comparison/dispatch are broader tiers. |

## Delivery priorities (updated 2026-10-20)

Optimize for useful, correctly admitted complete function families. Capability,
fixture and commit counts are not success measures. Finish the current scalar
indexing slice before changing direction.

1. **Declare the next end-to-end target from source.** Inspect all known blocker
   combinations and unknowns for a small tractable natural function family.
   Record functions, exact build contexts, bounded support and independently
   specified positive/negative expectations before evaluating admissions. Keep
   the existing frozen inventory and identities/denominators unchanged; use a
   separate small development corpus where needed.
2. **Investigate shared source boundaries early.** Audit S09 and module/import
   restrictions for constructors, getters and their callers. Replace coarse
   unit rejection only with justified ownership/drop/effect evidence. Neighboring
   items, derives, macros and hidden effects remain part of that proof.
3. **Complete the selected family's read-only chain.** Prioritize required
   indexing/subslicing, bounds/evaluation order, reference/helper-result flow,
   concrete checked access/Option and call summaries. Require complete composition
   through Cargo capture, independent verification, strict admission and matching.
4. **Close high-impact composition gaps.** Address F10, patterns, aliases,
   destructuring and scalar gaps when they block that family. Treat concrete
   Option/Result payload binding, match/if-let and eventually `?` as connected
   work. Defer broad async/concurrency/unsafe/FFI and general macro support unless
   required by the declared target.
5. **Measure each milestone.** Report end-to-end support, new/lost admissions on
   unchanged selections, remaining blocker combinations and actual validation.
   A foundational increment may have no gain but must advance a concrete blocker
   chain. Reassess after repeated milestones without practical gains.

Preserve strict evidence and historical results: no silent omission, prefix
certification, invented purity/alias independence, missing-data-as-zero or relaxed
thresholds. Keep unsupported, unknown, inactive and incomplete outcomes distinct.
Use independent semantic expectations, adversarial/composition cases and existing
regressions. Abstraction mining activation, rewrite safety and held-out detector
quality remain separate from extraction coverage.

## Measurement baseline and maintenance

- Historical corrected natural inventory: **60 selected contexts; 53 captured and
  assessed; R0 admits 0, expanded admits 5; 7 remain unknown**. These are not
  Data-v3 numbers. See [REPORT-v2](eval/hir-coverage/REPORT-v2.md).
- Latest inherent-call connected fixture: **21 admitted functions / 210 pair
  decisions**, including warm/edit checks. This is correctness evidence only.
- Do not total the rows into a “percent of Rust supported”: rows have deliberately
  different granularity and real functions require combinations of them.
- When implementation changes, update the affected row, its evidence link and
  the audited revision. Preserve historical reports. A roadmap example alone
  must never authorize an extraction capability.

## Continuation log (2026-10-19)

Work proceeds on `feature/rust-feature-coverage` in semantic increments. Start with
X09, then follow the delivery ladder above; do not mark syntax rows covered from
compiler observation alone. Each implementation increment requires its bounded
contract, independent positive/negative evidence, connected Cargo fixture, and
updated inventory before claiming admission gains.

### X09: refresh the inventory runner — still incomplete

Baseline: `e1729e7`, with the runner repair in the working tree. The runner now
selects Data-v3 alongside R0/expanded and verifies current sealed unit-bundle
checksums, namespaces, graph roots and CFG/PDG schemas. The frozen selection,
source archives, lockfiles, compiler, default build settings and denominator are
unchanged. Historical reports remain unchanged.

Fresh evidence: `target/hir-coverage/run-97isyyox/`. R0 reproduces 0 admitted,
53 excluded and 7 unknown; expanded reproduces 5 admitted, 48 excluded and
7 unknown. Twelve expanded CFG/PDG documents pass schema checks. Data-v3 fails
capture on the first package, `lz4_flex`, with `Concrete array exceeds length
budget`. This is incomplete execution, not zero admissions or exclusion. There
is no completed Data-v3 measurement, summary, or measured gain. X09 stays Missing.
The earlier `run-llbig8z4` exposed the obsolete per-function artifact lookup;
`run-66ex6uk7` failed because rustup was absent from PATH.

Next increment: investigate bounded concrete-type observation on ordinary large
arrays, keeping resource exhaustion explicit; then rerun all three packages and
record every selected outcome and blocker combination. Do not raise limits or
reinterpret missing capture as inactivity merely to complete the inventory.

Validation for this increment: 12 Python bookkeeping/verification tests pass,
including tampered sealed evidence and missing graph roots. `cargo rstyle --format
json`: 2,127 functions, 0 violations. `cargo rstyle patterns --format json
--fail-on-candidates`: 0 remaining candidates, with the existing 15 ignored
entries unchanged. No extraction semantics changed in this increment.

### X09 capture repairs

The natural rerun exposed four ordinary capture failures. Array lengths now remain
compact u64 values instead of failing above 4096; depth and actual tree-size
budgets remain enforced. Method signatures are normalized before observing
concrete array results. Macro-argument methods retain receiver records without
claiming a standalone source method outside the macro; the enclosing unsupported
macro still excludes the function. Fully expanded/generated methods do not force
parsing their expansion anchor as an original function. Struct declaration ranges
start at visibility/`struct`, matching rustc and excluding leading attributes.
These repairs add no new provenance fields or per-element array expansion.

Regression evidence: the complete `rstyle-pdg` test suites pass; 34 Data Cargo
checks pass, including large-array admission, recursive-budget failure,
macro-argument exclusion, normalized byte-array method exclusion, and derived
method exclusion. Style checks report zero violations and zero remaining pattern
candidates with the existing ignores unchanged. The natural rerun now captures
`lz4_flex` under Data-v3; the full inventory result is recorded separately below
when all packages finish. No new syntax row is claimed covered by these repairs.

### X09 completed baseline

Fresh Data-v3 reproduction `target/hir-coverage/run-25mgywk_/` completes all three
packages: **60 selected contexts, 53 assessed, 8 admitted, 45 excluded, 7 unknown,
0 failed, 0 proven inactive**. Eighteen CFG/PDG documents pass verification.
The full generated assessment retains every selected function, all reported
reasons, source blocker combinations and unknowns. Relative to expanded's five
admissions, `time-core::{likely, unlikely, days_in_year}` are newly admitted.
See [the refresh report](eval/hir-coverage/REPORT-data-v3.md). X09 is Covered;
X10/X11 remain separate, uncompleted coverage-target/quality work.

### F06 literal OR-pattern slice

Data-v3 now selects extraction 3.1.0 and `rust.scalar-or-patterns`. Flat integer
and boolean literal alternatives (including negative integers) compose with
ordered pure guards, saved scrutinees, branch joins and the existing final
wildcard. A compact `match.or` selector retains the ordered literals without
copying the arm or re-evaluating its guard. No provenance fields were added.
Unsupported character/payload patterns are not expanded into unnecessary capture
detail. Ranges and named constants remain missing; F06 is Partial.

Fresh full inventory `target/hir-coverage/run-wjnhsac7/` measures **9 admitted,
44 excluded, 7 unknown out of 60**, with R0/expanded unchanged. The gain over the
3.0 baseline is `time-core::weeks_in_year`; there are no lost selected admissions.
The Cargo fixture checks integer/bool alternatives, guard evaluation once,
unsupported alternatives, legacy exclusion, strict selector verification and
stable fresh/warm pair results. The workspace suite passes 714 tests across
27 suites (5 existing ignored tests). See the [contract](docs/extraction-profiles/scalar-or-patterns-v1.md).

Final focused inventory `target/hir-coverage/run-4v0dzgj6/` confirms identical
outcomes for all 60 selected identities after narrowing unsupported-pattern
capture. Repository-built style checks: 2,131 functions / 0 violations; patterns:
0 remaining candidates, existing ignores unchanged. Feature commit: `1f0ef93`.
Next scalar slices remain range patterns, resolved constants (V12), and assertions
(F14), followed by the connected read-only data ladder. The full roadmap remains
in progress; no broader Rust or detection-quality completion is claimed.

### F06 integer range-pattern slice

Data-v3 extraction 3.2.0 adds `rust.scalar-range-patterns`: integer literal ranges
with inclusive/exclusive/open bounds, negative endpoints and mixed literal/range
OR arms. The compiler capture retains bounds once, and independent source checks
preserve their exact spellings and upper-bound inclusivity. One selector uses the
saved scrutinee; guards/bodies and individual range values are never expanded.
No provenance fields or caches were added. F06 remains Partial because resolved
constants and other pattern tiers remain unfinished.

The [range contract](docs/extraction-profiles/scalar-range-patterns-v1.md) records
capture, source verification, lowering, handoff and the admitted boundary. The
workspace suite passes 717 tests in 28 suites (5 existing ignores); final targeted
Cargo checks also pass after checking source-bound consistency. Clippy passes
with warnings denied. Repository-built rstyle: 2,143 functions, 0 violations;
patterns: 0 remaining candidates, existing ignore entries unchanged. The frozen
natural refresh is recorded in the inventory report when it completes.

The 3.2.0 frozen refresh `target/hir-coverage/run-6s9q_4up/` completes with the
same 60 selected identities/outcomes as 3.1.0: 9 admitted, 44 excluded, 7 unknown.
Range support adds no measured admission in this frozen selection. Feature
commit: `849a910`; the [inventory report](eval/hir-coverage/REPORT-data-v3.md)
retains the validation scope and the distinction from detector-quality evidence.

### V12 resolved scalar constant-expression slice

Data-v3 extraction 3.3.0 adds `rust.resolved-scalar-constants`: same-unit primitive
integer/bool/unit constants, imported names and nongeneric inherent associated
constants. Rustc evaluates referenced initializers; independent admission checks
declaration/name/type/value consistency without reimplementing const execution.
The existing resolution description carries the small record; lowering emits a
typed computation without runtime memory reads. Edits update dependent function
evidence and preserve existing helper validation. See the
[constant contract](docs/extraction-profiles/scalar-constants-v1.md).

V12 remains Partial for nonlocal, generic, trait and aggregate constants. F06
still excludes named constant patterns and constant range bounds. Those are the
next scalar-composition increment, followed by F14 assertions and S03–S04;
the connected read-only data and later delivery stages remain in scope.

The workspace suite passes 720 tests (5 existing ignored tests). Clippy passes
with warnings denied. Repository-built rstyle reports 0 violations across 2,155
functions; patterns reports 0 remaining candidates with the 15 existing ignores
unchanged. The unchanged frozen inventory is assessed separately; no formal
performance benchmark or detector-quality result is claimed.

Frozen refresh `target/hir-coverage/run-qobq35og/` completes all three packages:
9 admitted, 44 excluded, 7 unknown, with all 60 selected identities/outcomes
unchanged from 3.2.0. The [inventory report](eval/hir-coverage/REPORT-data-v3.md)
records the measured result and its limits; this slice adds no frozen admission.

### F06 resolved constant-pattern composition

Data-v3 extraction 3.4.0 adds `rust.scalar-constant-patterns`, reusing V12's
compiler-resolved scalar records for integer/bool match arms. Imported constants
and nongeneric inherent constants compose with literal/range OR alternatives and
guards. Independent admission checks selector/value/type consistency and original
source correspondence; binding patterns remain excluded after name resolution.
One saved scrutinee feeds each selector, without runtime constant reads or copied
guards/bodies. The [contract](docs/extraction-profiles/scalar-constant-patterns-v1.md)
records the boundary and handoff. No provenance fields or caches were introduced.

The workspace suite passes 722 tests (5 existing ignores), and the final mixed
constant/literal/range fixture passes. Clippy with warnings denied passes.
Repository-built rstyle reports 0 violations across 2,158 functions; patterns
reports 0 remaining candidates, with all 15 existing ignores unchanged.
F06 remains Partial: constant range bounds are next, followed by assertions and
the remaining scalar-composition and connected read-only data roadmap. Natural
coverage is measured with the unchanged frozen inventory, separately from tests.

Frozen refresh `target/hir-coverage/run-nhkwv78q/` completes all three packages
with 9 admitted, 44 excluded and 7 unknown. All 60 selected identities/outcomes
match 3.3.0: no frozen admission gain is inferred from syntax support. See the
[inventory report](eval/hir-coverage/REPORT-data-v3.md) for evidence and limits.

### F06 constant range-bound composition

Data-v3 extraction 3.5.0 adds `rust.scalar-constant-range-bounds`. Same-unit
integer constants and nongeneric inherent constants now supply either endpoint
of inclusive/exclusive/open ranges, including imported names and guarded mixed
OR arms. Each named endpoint reuses a scalar constant record; source checks bind
it to the exact original endpoint span and verify the canonical descriptor.
Lowering retains one compact selector over the saved scrutinee, without endpoint
reads, per-integer expansion, copied guards/bodies, provenance fields or caches.
See the [bound contract](docs/extraction-profiles/scalar-constant-range-bounds-v1.md).

The workspace passes 724 tests (5 existing ignores), including real Cargo ranges
and independent source-position/type/value checks. Clippy passes with warnings
denied. Repository-built rstyle reports 0 violations across 2,166 functions;
patterns reports 0 remaining candidates, with all 15 existing ignores unchanged.
F06 remains Partial for character ranges, nonlocal/generic constants, nested
alternatives and general exhaustive matches. The next scalar-composition step is
F14 assertions with S03–S04 build/source correspondence, followed by the connected
read-only data ladder; later roadmap stages remain unfinished. Frozen inventory
results are recorded separately, without a performance or detector-quality claim.

Frozen refresh `target/hir-coverage/run-56rfkhh2/` completes all three packages:
9 admitted, 44 excluded, 7 unknown, with all 60 selected identities/outcomes
unchanged from 3.4.0. The [inventory report](eval/hir-coverage/REPORT-data-v3.md)
records the measured result; syntax composition adds no frozen admission here.

### F14 standard assertion slice

Data-v3 extraction 3.6.0 adds `rust.scalar-assertions`: unformatted standard
`assert!`/`debug_assert!`, including qualified invocations and trailing commas.
Compiler macro resolution and the original condition/callsite are retained in
existing frontend fields. Independent admission checks source, types and build
settings. Enabled conditions evaluate once in order; disabled debug conditions
emit no runtime operations while remaining in original-source evidence.
The [assertion contract](docs/extraction-profiles/scalar-assertions-v1.md) records
the expansion, verification, lowering and matching boundary.

Successful normal continuation is modeled. Unproved assertion success retains
Partial evidence with an explicit panic/unwind/cleanup limitation; directly known
true bool operands discharge that limitation. The existing nonpanicking helper
boundary still applies. F14 remains Partial for formatting/comparison assertions
and general exceptional flow. This does not complete S03–S04: arbitrary cfg-removed
source and inactive definitions still lack an independent resolver. Those source
boundaries remain next in scalar composition before the connected data ladder.

The workspace passes 727 tests (5 existing ignores), including debug/release,
single condition mutation, method receivers, helper boundaries and source/build
consistency checks. Clippy with warnings denied passes. Repository-built rstyle
reports 0 violations across 2,182 functions; patterns reports 0 remaining
candidates with all 15 existing ignores unchanged. No provenance table, snapshot
or cache was added; natural coverage and performance claims require separate evidence.

Frozen refresh `target/hir-coverage/run-i3z4hul6/` completes all three packages
with 10 admitted, 42 excluded and 8 unknown across the same 60 source identities.
`time-core::days_in_month_leap` is newly admitted with partial panic-flow evidence;
no admission is lost. The cfg-selected `is_leap_year` context moves from excluded
to unknown because source correspondence is still incomplete after assertion
rejection is removed. Seven uncaptured contexts remain unknown; no inactivity is
inferred. See the [inventory report](eval/hir-coverage/REPORT-data-v3.md).

### S03–S04 body-level conditional source removal

Data-v3 extraction 3.7.0 adds `rust.cfg-source-removal`: standard cfg flags,
string settings, all/any/not and nested cfg_attr are evaluated against the
captured compiler cfg set. Proven inactive roots reuse an entry in the existing
source correspondence inventory. Missing HIR alone remains insufficient, and
contradictory captured HIR is rejected even after regenerating the inventory.
The [removal contract](docs/extraction-profiles/cfg-source-removal-v1.md) records
representation, verification and the earlier-profile boundary.

Removed blocks, locals, macro statements, nested items and match arms no longer
need active-operation correspondence when an independent false guard is proved.
This also removes diagnostics confined to those inactive roots. The independent
field-origin inventory uses the same cfg semantics for active field reads,
bindings and mutations. Graphs continue to come from active HIR exclusively.
Whole-function and enclosing-scope inactivity remain unfinished; uncaptured
selected contexts remain unknown. S03–S04 remain Partial, and the full connected
data and broader Rust roadmap is still in scope.

The workspace passes 730 tests (5 existing ignores), with feature-on/off and
contradictory-source/HIR checks. Clippy with warnings denied passes. Repository-built
rstyle reports 0 violations across 2,201 functions; patterns reports 0 remaining
candidates with all 15 existing ignores unchanged. Capture now uses one inventory
for correspondence and diagnostics, avoiding its earlier duplicate source parse.
No new snapshot, provenance table or cache was added; no formal performance or
detector-quality claim follows from this implementation.

Final frozen refresh `target/hir-coverage/run-6iuv_l96/` completes all three
packages with 11 admitted, 42 excluded and 7 unknown across the same 60 identities.
The cfg-selected `time-core::is_leap_year` at `src/util.rs:728` moves from unknown
to admitted, with its existing partial panic-flow boundary; no admission is lost.
Seven uncaptured contexts remain unknown. The [inventory report](eval/hir-coverage/REPORT-data-v3.md)
records the final run and its limits; whole-definition inactivity is not inferred.

### S04 independently inactive definitions

Data-v3 extraction 3.8.0 adds `rust.cfg-inactive-definitions`. The original-file
inventory reuses standard cfg evaluation against actual compiler settings for
function attributes and traversed enclosing scopes. It changes the existing
presence value rather than adding cfg copies or serialized provenance. Contradictory
active HIR fails capture; unsupported predicates without an independent false guard
remain unknown. The [contract](docs/extraction-profiles/cfg-source-removal-v1.md)
records loaded-file and macro-token limits.

Frozen assessment version 3 consumes exact original-source/context correspondence
and reports inactive contexts separately, retaining all selected identities. Every
active context must be admitted for a source-level admission; an all-inactive
selection is never an admission. The [protocol amendment](eval/hir-coverage/AMENDMENTS.md)
records this extension without relabeling historical measurements.

The initial run `target/hir-coverage/run-9ru8zhlb/` measures 11 admitted,
42 excluded and 7 independently inactive out of 60, with no unknowns, failures,
new admissions or lost admissions. These seven `lz4_flex` definitions were formerly
unknown; resolving their inactivity is accounting evidence, not a coverage gain.
Final focused run `target/hir-coverage/run-i7ya5m_4/` verifies assessment version 3
and the final implementation with identical selected identities/outcomes; 26 graph
documents pass schema checks. S04 remains Partial because unloaded external files
and macro-token definitions are not covered. The broader roadmap remains unfinished.

Validation passes 735 workspace tests (5 existing ignores), 14 independent
inventory tests, Clippy with warnings denied, repository-built rstyle with 0
violations across 2,212 functions, and patterns with 0 remaining candidates and
the same 15 existing ignores. Existing parsing/traversal is reused; empty attributes
and inherited inactivity bypass predicate evaluation. No new serialized cfg copies,
source snapshots, provenance tables or caches are added.

### F04–F06 exhaustive scalar matches

Data-v3 extraction 3.9.0 adds `rust.exhaustive-scalar-match`: supported integer/bool
matches can omit the wildcard when their unguarded patterns cover the entire
resolved domain. The [contract](docs/extraction-profiles/exhaustive-scalar-matches-v1.md)
records compact interval coverage, actual pointer-width limits, full 128-bit signed
and unsigned endpoints, source-bound literals and the existing constant/range
checks. Guarded arms do not prove unconditional coverage. Existing final-wildcard
matches bypass interval parsing and retain their prior operation/control structure.

Lowering keeps one scrutinee evaluation, ordered pattern/guard/body operations and
value alternatives. An independently closing selector has no impossible false
continuation. Strict reconstruction and independent serialized value-site checks
reject corrupted failure edges; all source arms remain assessed, including
unreachable suffixes. Helpers, early returns and cfg-removed arms compose in real
Cargo tests. No new frontend field, serialized witness, source snapshot, provenance
table, cache or matching vocabulary is introduced. Earlier Cargo selections keep
their wildcard boundary. F04/F06 remain Partial for broader pattern domains and
nested forms; the full roadmap remains active.

Validation passes 739 workspace tests (5 existing ignores), the final composed
Cargo regression, 14 independent inventory tests, and repository-built rstyle with
0 violations across 2,220 functions. Patterns reports 0 remaining candidates with
the same 15 existing ignores. The independent small-domain oracle checks compact
coverage against enumerated values; the production proof never enumerates interval
members. Natural coverage is measured separately below.

Focused run `target/hir-coverage/run-peldp9o8/` completes all three packages under
assessment version 3: 11 admitted, 42 excluded and 7 independently inactive out of
the unchanged 60 selections, with no unknowns or failures. All identities/outcomes
match 3.8.0; this adds no measured frozen admission and loses none. Twenty-six
admitted graph documents pass schema checks. Clippy with warnings denied passes.
This is bounded feature support and coverage accounting, not a detector-quality
or formal performance result. The broader roadmap remains unfinished.


### F06 nested and grouped scalar-pattern composition

Data-v3 extraction 3.10.0 adds `rust.nested-scalar-or-patterns`. The
[contract](docs/extraction-profiles/nested-scalar-or-patterns-v1.md) connects
recursive scalar OR capture, transparent grouping, source correspondence,
eligibility, exhaustive coverage and ordered selector lowering. Literal,
same-unit constant, integer-range and wildcard alternatives compose with guards;
leading pipes and duplicate alternatives retain their compiler structure and
ordered payloads. Payload/reference/`@` patterns and broader scalar domains remain
outside this slice, so F06 remains Partial and F07 remains Missing.

Nested and flat forms produce the same selector payload and an accepted matching
pair with score 1. A wildcard alternative removes only the impossible selector
failure continuation; guard failure still advances. Flat selectors keep their
existing direct path. Nested lowering and interval coverage each use one shared
vector rather than copying subtrees or expanding ranges. Source traversal stops
at opaque unsupported OR parents without inventing child correspondence. No new
frontend fields, provenance tables, source snapshots, caches or matching vocabulary
are introduced.

Validation passes 741 workspace tests (5 existing ignores), Clippy with warnings
denied and 14 inventory tests. Repository-built rstyle reports 0 violations across
2,225 functions; patterns reports 0 candidates with the same 15 existing ignores.
The composed Cargo fixture verifies retained nested HIR, comments/grouped literals
and bounds, exhaustive bool alternatives, mixed leaves, duplicate ordering,
wildcard/guard flow, legacy exclusions, selector corruption rejection and actual
nested/flat pair evaluation. Independent source-correspondence tests reject
non-group wrappers. The full roadmap remains active.


Fresh focused run `target/hir-coverage/run-bzeoyikc/` completes all three packages:
11 admitted, 42 excluded and 7 independently inactive out of the unchanged 60
selections, with no unknowns or failures. Every frozen identity/outcome and all
reported denominators match 3.9.0. Twenty-six admitted graph documents pass schema
checks (10 lz4, 0 regex, 16 time). This adds bounded synthetic feature coverage but
no measured frozen admission; it makes no performance or detector-quality claim.


### S09 constant declarations beside nominal APIs

Data-v3 extraction 3.11.0 adds `rust.nominal-constant-context`; matching remains
`rstyle.ccgraph.data-calls/1.0.0`. The
[contract](docs/extraction-profiles/nominal-constant-context-v1.md) admits ordinary
constant items and constants in supported inherent impls to the existing nominal
unit census. Initializers are still traversed for unavailable nested items and
macros, and attributes retain the existing presence/expansion boundary. Constant
value uses keep their separate resolved primitive contract. This does not provide
aggregate construction or general constant evaluation. S09 remains Partial.

The existing shared source-environment lookup distinguishes the new capability
from earlier selections while retaining owner/policy isolation and reuse. No new
serialized records, provenance tables, snapshots or caches are added. Struct field,
namespace, source ownership and drop proofs are unchanged. A real Cargo family
composes module/import aliases, associated constants, constant range patterns,
field reads and inherent helper calls through serialized unit verification and
all six pairs. Data-v2 excludes the same family. A second Cargo fixture preserves
nominal exclusion for a hidden Drop impl inside an otherwise unused initializer;
its independent scalar function still admits. Census tests retain macro, unsafe
and conditional-attribute exclusions, including associated constants.

Validation passes 744 workspace tests (5 existing ignores), Clippy with warnings
denied, and 14 inventory tests. Repository-built rstyle reports 0 violations across
2,227 functions; patterns reports 0 candidates with the same 15 existing ignores.
The full read-only and broader Rust roadmap remains active.


Focused run `target/hir-coverage/run-3s_vx4i9/` completes all three packages with
11 admitted, 42 excluded and 7 independently inactive out of 60, with no unknowns
or failures. All frozen identities/outcomes and reported denominators match
3.10.0. Twenty-six admitted graph documents pass schema checks (10 lz4, 0 regex,
16 time). The composed family gains admission, but the unchanged natural sample
still has other blockers; no frozen gain or performance improvement is claimed.


### D11 tuple and explicit array literal values

Data-v3 extraction 3.12.0 adds `rust.aggregate-literal-values`; matching remains
`rstyle.ccgraph.data-calls/1.0.0`. The
[contract](docs/extraction-profiles/aggregate-literal-values-v1.md) connects ordered
compiler children, existing concrete type tokens, independent source/component
checks, constructor computations, serialized operand verification and matching.
Nested reference-free tuple/array literals, empty arrays, conditional component
values and scalar effects compose. Empty unit tuples retain their earlier literal
semantics. D11 advances to Partial; struct/update/repeat construction and reference
origins remain unfinished, alongside D12–D13 local values and borrows.

Each component evaluates once, with a saved value and its exact operand position.
Unit-valued blocks without a value operation receive a unit computation after
normal completion; a slot cannot disappear and shift later components. The
shared operand collector now appends into one vector instead of repeatedly
rebuilding it. Array type checks compare the recorded length without expanding a
component-type vector. Independent serialized checks build temporary lookup maps
once and compare source values, types and unique ordered slots. Their effect
check is shared with address computations. No frontend records, provenance tables,
source snapshots or retained caches are added.

Validation passes 745 workspace tests (5 existing ignores), Clippy with warnings
denied and 14 inventory tests. Repository-built rstyle reports 0 violations across
2,241 functions; patterns reports 0 candidates with the same 15 existing ignores.
The composed Cargo fixture includes nested values, units, an earlier component
read followed by scalar reassignment, actual equivalent tuple matching with score
1, unsupported local/reference/repeat forms and legacy exclusions. Same-typed
source-child swaps and serialized operand-position swaps fail independent checks
and strict reconstruction. The full roadmap remains active.


Final focused run `target/hir-coverage/run-q590o4no/` uses the completed typed unit
slot behavior and finishes all three packages: 11 admitted, 42 excluded and 7
independently inactive out of 60, with no unknowns or failures. All frozen
identities/outcomes and reported denominators match 3.11.0. Twenty-six admitted
graph documents pass schema checks (10 lz4, 0 regex, 16 time). No natural admission
gain, detector-quality result or measured performance improvement is claimed.


### D11 repeat array values without element expansion

Data-v3 extraction 3.13.0 adds `rust.repeat-array-values`; matching remains
`rstyle.ccgraph.data-calls/1.0.0`. The
[contract](docs/extraction-profiles/repeat-array-values-v1.md) composes the existing
reference-free constructor path with `[value; count]`. One captured child and one
value operand represent the single runtime evaluation. The existing resolved array
type retains the dimension; original literal counts are independently checked,
including grouping, radix/underscore spelling and `usize` suffixes. Named/general
constant counts remain unfinished V13 work. D11 stays Partial for the remaining
struct/update and reference-bearing construction work; D12–D13 remain active.

Zero-length repeats still evaluate the operand and its supported scalar effects.
Unit repeats retain one typed slot even at length zero. Nested and conditional
values compose without generating per-element copies or expanded type lists.
Source inventory visits the runtime value rather than inventing a runtime operation
for the compile-time count. Independent serialized checks reuse the constructor
component/type/effect checks; unit-slot validation is bounded to the registered
repeat operation. No frontend fields, serialized witnesses, provenance tables,
source snapshots or retained caches are added.

Validation passes 746 workspace tests (5 existing ignores), Clippy with warnings
denied and 14 inventory tests. Repository-built rstyle reports 0 violations across
2,244 functions; patterns reports 0 candidates with the same 15 existing ignores.
Every admitted fixture pair passes independent serialized data-document and strict
producer verification. Four-element and 65,536-element repeats have equal graph
node/edge counts. A zero-length repeat retains exactly one assignment and one
constructor operand. Equivalent radix/count spellings produce an evaluated accepted
pair with score 1. A corrupted resolved length fails specifically at the original
repeat-count check; named/complex count, reference-bearing and legacy cases retain
their exclusions. The broader roadmap remains unfinished.


Focused run `target/hir-coverage/run-k8i6upob/` completes all three packages with
11 admitted, 42 excluded and 7 independently inactive out of 60, with no unknowns
or failures. Every frozen identity/outcome and all reported denominators match
3.12.0. Twenty-six admitted graph documents pass schema checks (10 lz4, 0 regex,
16 time). This adds bounded repeat construction without a measured frozen admission
gain; the graph-size regression is not a general performance benchmark or detector
quality result.


### D11 named struct values with nominal field slots

Data-v3 extraction 3.14.0 adds `rust.named-struct-values`; matching remains
`rstyle.ccgraph.data-calls/1.0.0`. The
[contract](docs/extraction-profiles/named-struct-values-v1.md) admits fully enumerated,
reference-free named constructors through exact source/nominal resolution. Children
and operands evaluate in source order; the existing construct/literal attributes
encode the mapping to declaration slots. Unit fields, aliases, `Self`, shorthand,
empty structs and nested aggregate values compose without new frontend records,
provenance or retained caches. Struct updates, general aggregate local reads,
reference-bearing constructors and helper substitution remain unfinished. D11 stays
Partial and the full roadmap remains active.

The real Cargo fixture verifies all admitted serialized documents independently and
through strict producer reconstruction. Altering the slot mapping between two
same-typed fields fails both checks; reordering captured children fails frontend
source correspondence. Equivalent functions reach evaluated accepted matching
with score 1. Legacy construction exclusions remain intact.

Validation: 747 workspace tests pass with 5 existing ignores; Clippy across all
targets passes with warnings denied. Repository-built rstyle reports 2,255
functions and 0 violations; patterns reports 0 candidates with the same 15 existing
ignores. All 14 inventory bookkeeping tests pass.

Frozen run `target/hir-coverage/run-806spmc0/` completes all three packages:
11 admitted, 42 excluded and 7 independently inactive out of 60, with no unknowns
or failures. All 60 frozen identity/outcome tuples and all denominators match
3.13.0 (`run-k8i6upob`). Twenty-six admitted graph documents pass schema checks
(10 lz4, 0 regex, 16 time). This adds connected named construction without a
measured frozen admission gain; it does not establish a general performance or
detector-quality result.


### D11 struct updates with one ordered base value

Data-v3 extraction 3.15.0 adds `rust.struct-update-values`; matching remains
`rstyle.ccgraph.data-calls/1.0.0`. The
[contract](docs/extraction-profiles/struct-update-values-v1.md) composes named
reference-free construction with a same-typed base evaluated once after explicit
field values. The existing slot vector adds a final null for the base supplying
unoverridden fields; no frontend records, provenance or retained caches are added.
Even fully overridden constructors retain base evaluation and its supported scalar
effects. Formal, nested and conditional bases compose; general aggregate locals
and reference-bearing values remain unfinished. D11 stays Partial.

The connected Cargo fixture independently verifies every admitted serialized pair
and strict reconstruction, checks equivalent updates reach accepted matching with
score 1, and rejects an altered same-typed override slot or missing captured base.

Validation: 748 workspace tests pass with 5 existing ignores, including exact
assignment/operand counts for effectful and fully overridden updates. Clippy across
all targets passes with warnings denied; formatting passes. Repository-built
rstyle reports 2,256 functions and 0 violations; patterns reports 0 candidates with
the same 15 existing ignores. All 14 inventory bookkeeping tests pass.

Frozen run `target/hir-coverage/run-c7dkk68a/` completes all three packages with
11 admitted, 42 excluded and 7 independently inactive out of 60, with no unknowns
or failures. All 60 frozen identities/outcomes and all denominators match 3.14.0
(`run-806spmc0`). Twenty-six admitted graph documents pass schema checks (10 lz4,
0 regex, 16 time). This adds composed update semantics without a measured frozen
admission gain; it is not a general performance benchmark or detector-quality claim.


### D11 tuple struct constructors through exact nominal declarations

Data-v3 extraction 3.16.0 adds `rust.tuple-struct-values`; matching remains
`rstyle.ccgraph.data-calls/1.0.0`. The
[contract](docs/extraction-profiles/tuple-struct-values-v1.md) normalizes resolved
struct constructor calls to existing aggregate operations. Original path resolution
and already-retained declaration source independently establish unnamed fields,
argument arity/types and source order, including empty tuple structs. Existing
operation slots carry semantics without new frontend records, witnesses, provenance
or retained caches. Reference-free aliases, `Self`, unit components, nested
aggregates and scalar effects compose. Ordinary helpers retain call semantics.

Every admitted fixture pair passes independent document checks and strict producer
reconstruction. Equivalent functions reach evaluated accepted matching with score 1.
Changed same-typed slot order, captured argument order and a helper relabeled as a
constructor fail verification. Legacy selections retain their construction boundary.
D11 stays Partial for unit-struct paths, reference-bearing values and broader repeat
counts; general local propagation remains D12 work and the full roadmap is active.

Validation: 749 workspace tests pass with 5 existing ignores. Clippy across all
targets passes with warnings denied; formatting passes. Repository-built rstyle
reports 2,258 functions and 0 violations; patterns reports 0 candidates with the
same 15 existing ignores. All 14 inventory bookkeeping tests pass.

Frozen run `target/hir-coverage/run-_j2xrpg0/` completes all three packages with
11 admitted, 42 excluded and 7 independently inactive out of 60, with no unknowns
or failures. All 60 frozen identities/outcomes and all denominators match 3.15.0
(`run-c7dkk68a`). Twenty-six admitted graph documents pass schema checks (10 lz4,
0 regex, 16 time). Tuple construction adds no measured frozen admission gain;
these checks do not establish general performance or detector quality.


### D11 unit struct paths through exact declaration kind

Data-v3 extraction 3.17.0 adds `rust.unit-struct-values`; matching remains
`rstyle.ccgraph.data-calls/1.0.0`. The
[contract](docs/extraction-profiles/unit-struct-values-v1.md) normalizes resolved
unit constructor paths to existing aggregate operations. Exact source path/type
resolution and already-retained declaration source prove unit declaration kind and
zero runtime children. Existing operation attributes carry the empty slot vector;
no new frontend records, witnesses, provenance or retained caches are added.
Aliases, `Self`, grouping, nested aggregates, zero-length repeats and conditional
values compose. Ordinary local/constant paths retain their existing boundary.

Every admitted fixture pair passes independent document verification and strict
producer reconstruction; equivalent functions reach evaluated accepted matching
with score 1. Inventing a serialized slot or relabeling a constant of an empty named
struct type as a unit constructor fails verification. Legacy selections retain
their exclusions. D11 stays Partial for reference-bearing construction and broader
repeat counts; D12–D13 and the full roadmap remain active.

Validation: 750 workspace tests pass with 5 existing ignores. Clippy across all
targets passes with warnings denied; formatting passes. Repository-built rstyle
reports 2,262 functions and 0 violations; patterns reports 0 candidates with the
same 15 existing ignores. All 14 inventory bookkeeping tests pass.

Frozen run `target/hir-coverage/run-hmdcg04l/` completes all three packages with
11 admitted, 42 excluded and 7 independently inactive out of 60, with no unknowns
or failures. All 60 frozen identities/outcomes and all denominators match 3.16.0
(`run-_j2xrpg0`). Twenty-six admitted graph documents pass schema checks (10 lz4,
0 regex, 16 time). Unit construction adds no measured frozen admission gain;
these checks do not establish general performance or detector quality.


### D12 whole reference-free local values through binding definitions

Data-v3 extraction 3.18.0 adds `rust.local-aggregate-values`; matching remains
`rstyle.ccgraph.data-calls/1.0.0`. The
[contract](docs/extraction-profiles/local-aggregate-values-v1.md) connects exact
binding/type reads and initializer/reassignment type checks to the existing whole
value operands and kill/gen definition analysis. Tuples, arrays and supported
reference-free nominal values compose through aliases, shadowing, assignments,
mutable parameters, conditional initializers, branch joins and existing loops.
There are no new serialized records, witnesses, snapshots, provenance or retained
caches. D12 advances to Partial; local field projection and reference-valued
rebindings remain unfinished, as do D13 and the full roadmap.

Independent document checks verify exact source binding identity, typed original
initializer/reassignment operands and CFG fixed-point reaching definitions. This
reuses the existing loop verifier's binding proof for acyclic flow too, keeping
formal memory reads under their separate location checks. Unreachable definitions
and constructors retain source checks without requiring runtime operands; extra
reachability work is confined to the missing-operand path.

The composed fixture covers all admitted forms above plus struct-update bases and
unreachable source. Every admitted pair passes independent serialized checks and
strict producer reconstruction; equivalent functions reach accepted matching with
score 1. Removing a reaching-definition edge, removing an initializer operand or
substituting a same-typed later read fails independent verification. Reference
rebindings and local field reads retain their exclusions. Earlier construction
fixtures now check their whole-local cases as admitted.

Validation: 751 workspace tests pass with 5 existing ignores, including explicit
two-definition expectations at aggregate branch/loop reads and a combined formal
memory-read/local-value fixture. Clippy across all targets passes with warnings
denied; formatting passes. Repository-built rstyle reports 2,270 functions and
0 violations; patterns reports 0 candidates with the same 15 existing ignores.
All 14 inventory bookkeeping tests pass.

Frozen run `target/hir-coverage/run-utecfjp9/` completes all three packages with
11 admitted, 42 excluded and 7 independently inactive out of 60, with no unknowns
or failures. All 60 frozen identities/outcomes and all denominators match 3.17.0
(`run-hmdcg04l`). Twenty-six admitted graph documents pass schema checks (10 lz4,
0 regex, 16 time). Whole local propagation adds no measured frozen admission gain;
these checks do not establish general performance or detector quality.


### D12 projections from local and computed aggregate values

Data-v3 extraction 3.19.0 adds `rust.aggregate-value-fields`; matching remains
`rstyle.ccgraph.data-calls/1.0.0`. The
[contract](docs/extraction-profiles/aggregate-value-fields-v1.md) selects components
from reference-free tuple/struct values through locals, reassigned parameters,
nested fields and directly computed/conditional bases. Exact source member/base
correspondence, existing component types, nominal owner and compiler field index
establish the projection. Existing computation/literal/operand attributes represent
one current value and one declared component slot. No copied-value memory locations,
new frontend records, witnesses, provenance or retained caches are added.

Independent source and serialized checks cover every projection, including mixed
formal-memory/value functions. Ordinary formal fields keep their existing memory
proof; formal chains exit value classification before aggregate type parsing.
The real Cargo fixture covers all admitted forms above plus unit fields and ordered
snapshots. Every admitted pair passes independent document and strict producer
checks; equivalent functions reach evaluated accepted matching with score 1.
Altered same-typed serialized slots and compiler field positions fail verification.
An effectful base retains one assignment and one projection operand. Pure value
projections have no memory locations, while the mixed fixture retains one formal
location. A merely mutable, never-reassigned formal retains its existing field
read admission in both profiles. Legacy exclusions remain intact. D12 stays Partial for reference-valued
locals/rebindings; D13 and the full roadmap remain active.

Validation: 752 workspace tests pass with the same 5 existing ignores, including
`clones_data_assertion_argument_receivers_compose_with_partial_flow`. Clippy passes
for all targets with warnings denied; formatting passes. Repository-built rstyle
reports 2,276 functions and 0 violations; patterns reports 0 candidates with the
same 15 existing ignores. All 14 inventory bookkeeping tests pass.

Frozen run `target/hir-coverage/run-87eyhnsq/` completes all three packages with
11 admitted, 42 excluded and 7 independently inactive out of 60. All 60 frozen
identities/outcomes and all denominators match 3.18.0 (`run-utecfjp9`). Twenty-six
admitted graph documents pass schema checks (10 lz4, 0 regex, 16 time). Value-field
projection adds no measured frozen admission gain; these checks do not establish
general performance or detector quality.


### D12 initialized immutable shared-reference locals

Data-v3 extraction 3.20.0 adds `rust.shared-reference-locals`; matching remains
`rstyle.ccgraph.data-calls/1.0.0`. The
[contract](docs/extraction-profiles/shared-reference-locals-v1.md) follows exact
binding identities to original same-typed initializers. Reads must follow their
complete initializer, so recursive alias resolution strictly decreases source
positions. Existing formal/field/borrow/conditional/helper source proofs still
establish the reference values. No new frontend observations, producer operations,
source snapshots, provenance or retained caches are added.

Existing declaration operands and binding definitions carry reference values.
Independent document verification follows unit-typed binding definitions to their
reference initializers, compares graph/source origins, and checks original typed
initializer correspondence and serialized CFG reaching definitions. The real Cargo
fixture covers aliases, chains, shadowing and saved values, conditional initializers,
stored references, supported formal-field borrows and registered slice/string
metadata calls. Every admitted pair passes independent document and strict producer
checks; equivalent functions reach accepted matching with score 1. Missing definition
edges and altered origins fail verification. Metadata calls retain target authority
requirements and Data-v2 keeps its local-reference exclusions.

Mutable reference locals/rebindings, reference-bearing aggregate locals, projections
through reference-local roots and local/temporary borrows remain unfinished.
D12 stays Partial; D13 and the full roadmap remain active.

Validation: 753 workspace tests pass with the same 5 existing ignores. Clippy
passes for all targets with warnings denied; formatting passes. Repository-built
rstyle reports 2,279 functions and 0 violations; patterns reports 0 candidates with
the same 15 existing ignores. All 14 inventory bookkeeping tests pass.

Frozen run `target/hir-coverage/run-yw0u7att/` completes all three packages with
11 admitted, 42 excluded and 7 independently inactive out of 60. All 60 frozen
identities/outcomes and all denominators match 3.19.0 (`run-87eyhnsq`). Twenty-six
admitted graph documents pass schema checks (10 lz4, 0 regex, 16 time). Immutable
reference locals add no measured frozen admission gain; these checks do not
establish general performance or detector quality.


### D12 field reads through single-origin shared-reference locals

Data-v3 extraction 3.21.0 adds `rust.shared-local-field-reads`; matching remains
`rstyle.ccgraph.data-calls/1.0.0`. The
[contract](docs/extraction-profiles/shared-local-fields-v1.md) resolves existing
immutable aliases, stored reference fields, formal-field borrows, blocks and
same-referent conditionals to one formal location. Original source members and
concrete declarations independently establish the field path, result type and
compiler field identity. A borrowed field already names its referent and gains no
synthetic dereference. Ordinary formal capture and source checks keep their paths.

Existing field operations and local binding reads retain one immediate base
operand and original initializer/reaching-definition dependence. No local storage
location, new frontend record, producer operation, provenance, source snapshot or
retained cache is introduced. The real Cargo fixture covers alias chains, grouping,
shadowing and saved aliases, nested fields, stored references, formal-field borrows,
same-origin conditionals, block aliases, reference-valued results, borrows through
aliases and unit fields. Every admitted pair passes independent document and strict
producer checks; equivalent functions reach accepted matching with score 1. Altered
same-typed formal roots fail captured and serialized verification. The borrowed
alias retains exactly the original referent/inner/result path. Data-v2 keeps its
local-field exclusions; the earlier reference-local fixture now admits its tuple
field read.

Multiple distinct origins need alternative read locations; mutable rebindings,
reference-bearing aggregate locals, helper-result field bases, unavailable
implicit/explicit dereferences and local/temporary storage borrows remain
unfinished. D12 stays Partial; D13 and the full roadmap remain active.

Validation: 754 workspace tests pass with the same 5 existing ignores, including
the reported assertion-receiver regression and both reference-local fixtures.
Clippy passes for all targets with warnings denied; formatting passes.
Repository-built rstyle reports 2,290 functions and 0 violations; patterns reports
0 candidates with the same 15 existing ignores. All 14 inventory bookkeeping
tests pass.

Frozen run `target/hir-coverage/run-ol5lc3xz/` completes all three packages with
11 admitted, 42 excluded and 7 independently inactive out of 60. All 60 frozen
identities/outcomes and all denominators match 3.20.0 (`run-yw0u7att`). Twenty-six
admitted graph documents pass schema checks (10 lz4, 0 regex, 16 time). Single-origin
local field reads add no measured frozen admission gain; these checks do not
establish general performance or detector quality.


### D11–D12 reference-bearing tuple and struct values

Data-v3 extraction 3.22.0 adds `rust.reference-bearing-values`; matching remains
`rstyle.ccgraph.data-calls/1.0.0`. The
[contract](docs/extraction-profiles/reference-bearing-values-v1.md) admits tuple,
named-struct and tuple-struct literals containing supported shared-reference
values, nested constructions, scalars and units. Whole values can flow through
initialized immutable locals, aliases, shadowing, conditionals and returns. Exact
original source operands, types, evaluation positions and nominal slots retain
their existing constructor proof. Ordinary reference-free construction keeps its
path. No new frontend record, operation, origin format, provenance, snapshot or
retained cache is added.

Existing reference annotations record the whole-value union of component origins;
graph verification reconstructs it from reference-bearing operands. Independent
constructor verification still checks exact slots: exchanging same-typed reference
operands preserves their origin union but fails source/slot correspondence. Whole
local reads follow original same-typed initializers at strictly earlier source
positions. Original declaration operands, binding identities and serialized CFG
reaching definitions remain independently checked, and reference graph verification
follows definitions to their values.

The real Cargo fixture covers nested literals, reference and aggregate aliases,
shadowing and saved aggregates, stored references, formal-field borrows, conditional
references, blocks, wrapped reference-bearing formals, units, shared helper
components and effectful component evaluation. Every admitted pair passes independent
document and strict producer checks; equivalent functions reach accepted matching
with score 1. The effectful struct keeps one assignment and all three ordered
operands. Earlier tuple/named/tuple-struct fixtures now admit reference construction,
and the reference-local fixture now admits its immutable tuple local. Data-v2 keeps
its construction/local-value exclusions and ordinary shared identity admission.

Reference-bearing updates need component-specific substitution to remove overwritten
origins, so they remain excluded. By-value reference-bearing arrays, constructed/local
component projections, by-value memory-reading helper argument substitution,
reference-bearing helper results, mutable reference-bearing locals/rebindings and local/temporary storage borrows remain unfinished. D11 and D12
stay Partial; D18 and the full roadmap remain active.

Validation: 755 workspace tests pass with the same 5 existing ignores. Clippy
passes for all targets with warnings denied; formatting passes. Repository-built
rstyle reports 2,294 functions and 0 violations; patterns reports 0 candidates with
the same 15 existing ignores. All 14 inventory bookkeeping tests pass.

Frozen run `target/hir-coverage/run-_2h3cxde/` completes all three packages with
11 admitted, 42 excluded and 7 independently inactive out of 60. All 60 frozen
identities/outcomes and all denominators match 3.21.0 (`run-ol5lc3xz`). Twenty-six
admitted graph documents pass schema checks (10 lz4, 0 regex, 16 time).
Reference-bearing tuple/struct values add no measured frozen admission gain;
these checks do not establish general performance or detector quality.

### D09–D12 fields of reference-bearing aggregate values

Data-v3 extraction 3.23.0 adds `rust.reference-value-fields` (35 capabilities);
matching remains `rstyle.ccgraph.data-calls/1.0.0`. The
[contract](docs/extraction-profiles/reference-value-fields-v1.md) extends existing
value projections to reference-bearing tuple, named-struct and tuple-struct bases.
Constructed temporaries, whole immutable locals, aliases, shadowing, saved values,
nested selectors and conditional aggregates compose. Scalar components also use
ordinary evaluated-value projections. Existing formal aggregate loads retain their
original locations; constructed/local value projections add none.

Source origin reconstruction follows the selected constructor slot or same-typed
immutable initializer. Conditional aggregates select the same component in each
alternative before joining its origins. Independent graph verification carries the
selection through existing operand, reaching-definition and alternative edges;
constructor slots preserve source evaluation order and nominal declaration order.
Formal and loaded aggregates retain their exact parameter/field paths. Only the
selected subtree's reference paths are enumerated. Whole-value annotations remain
unions; no frontend record, origin format, graph operation, provenance, snapshot,
retained component map or cache is added.

The 22-function composed fixture admits 15 functions and retains 7 negative cases.
It verifies distinct same-typed siblings, reordered named fields, tuple structs,
nested selection, aliases, shadowing, conditional origin unions, invariant selected
components, scalar components and shared helper components. Exact formal and
loaded-aggregate origin paths are checked. Every admitted graph passes independent
document and strict producer verification; equivalent functions reach accepted
matching. Changed origin annotations, selected slots and base operands are rejected.
Earlier value-field and literal fixtures now admit their reference projection cases;
Data-v2 keeps its exclusions and ordinary shared identity admission.

A root returning an aggregate's stored reference can now be valid without fitting
the existing whole-shared-formal helper summary. Helper eligibility checks that
result boundary before certification and passes the computed origins to summary
construction once. Unsupported helper results leave the root available and their
callers excluded, instead of aborting capture. Body-only nominal types without an
interface-registry declaration, mutable reference-bearing bindings/rebindings,
reference-bearing updates/arrays, local storage borrows, aggregate helper results
and broader memory-reading substitution remain unfinished. D09–D12 stay Partial;
S09 and the full roadmap remain active.

Validation: 756 workspace tests pass with the same 5 existing ignores, including
the original assertion-argument receiver regression. The additional exact-path
fixture assertions pass separately. Clippy passes for all targets with warnings
denied; formatting and diff checks pass. Repository-built rstyle reports 2,304
functions and 0 violations; patterns reports 0 candidates with the same 15 existing
ignores. All 14 inventory bookkeeping tests pass.

Frozen run `target/hir-coverage/run-gon1id2y/` completes all three packages with
11 admitted, 42 excluded and 7 independently inactive out of 60. All 60 identities
and outcomes, all denominators and context counts match 3.22.0 (`run-_2h3cxde`).
Twenty-six admitted graph documents pass schema checks (10 lz4, 0 regex, 16 time).
The new component projections add no measured frozen admission gain; these checks
do not establish general performance or detector quality.

### D11 reference-bearing struct updates with component replacement

Data-v3 extraction 3.24.0 adds `rust.reference-struct-updates` (36 capabilities);
matching remains `rstyle.ccgraph.data-calls/1.0.0`. The
[contract](docs/extraction-profiles/reference-struct-updates-v1.md) extends existing
same-typed named struct updates to supported reference-bearing values. Formal,
constructed, immutable local, nested/chained and conditional bases compose, as do
projections of inherited and overridden result fields.

Result origins are reconstructed per reference-bearing field. Explicit fields use
their override operand; inherited fields keep their selector while following the
base. Their union excludes references appearing solely in overwritten fields while
preserving an origin also present in an untouched field. No subtraction from the
whole-base union is used. Source reconstruction checks original slots; graph
reconstruction uses serialized slots and actual ordered operand edges. Independent
constructor verification still compares those slots with original source and exact
nominal declarations. Ordinary struct component graph traversal also uses its
serialized descriptor, avoiding repeated constructor source parsing.

The existing base remains evaluated once after explicit operands, even when every
field is overridden. Scalar effects and unit slots keep their original assignment
and operand structure. No new frontend record, graph operation, origin format,
provenance, snapshot, retained component map or cache is added. Reference-free
updates retain their existing path.

The 18-function composed fixture covers no overrides, all overrides, reordered
fields, reference origins shared between overwritten and retained fields, immutable
local bases, constructed/nested/chained updates, conditional bases, inherited and
overridden projections, effectful overrides/bases and effectful unit components.
Fifteen functions are admitted and three mutation/helper/array cases remain
excluded. Exact result parameter origins and retained formal paths are checked;
independent document and strict producer checks pass for every admitted graph.
Equivalent updates reach accepted matching. Reintroducing an overwritten origin,
changing an override slot or removing the base edge is rejected by both verifiers.
Earlier literal and projection fixtures now admit their reference update cases;
Data-v2 retains all update exclusions.

Reference-bearing arrays, mutable reference-bearing locals/rebindings, local or
temporary storage borrows, aggregate helper results and broader memory-reading
helper substitution remain unfinished. Existing nominal proof boundaries still
apply. D11 remains Partial; D12–D18, S09 and the full roadmap remain active.

Validation: the final serial workspace run passes 757 tests with the same 5
existing ignores, including the original assertion-argument receiver regression.
An earlier overlapping inventory build replaced the CLI executable during one
test launch; the serial rerun uses the prebuilt driver and passes that test too.
Clippy passes for all targets with warnings denied; formatting and diff checks
pass. Repository-built rstyle reports 2,311 functions and 0 violations; patterns
reports 0 candidates with the same 15 existing ignores. All 14 inventory
bookkeeping tests pass.

Frozen run `target/hir-coverage/run-fdc85x8c/` completes all three packages with
11 admitted, 42 excluded and 7 independently inactive out of 60. All 60 frozen
identities/outcomes, all denominators and context counts match 3.23.0
(`run-gon1id2y`). Twenty-six admitted graph documents pass schema checks
(10 lz4, 0 regex, 16 time). Reference-bearing updates add no measured frozen
admission gain; these checks do not establish general performance or detector
quality.

### S09 nominal types used only in function bodies

Data-v3 extraction 3.25.0 adds `rust.body-nominal-types` (37 capabilities);
matching remains `rstyle.ccgraph.data-calls/1.0.0`. The
[contract](docs/extraction-profiles/body-nominal-types-v1.md) removes the signature
witness requirement for supported body-only concrete structs. Scalar/shared-only
interfaces can compose named/tuple/unit construction, imported constructor aliases,
nested values, immutable locals, reference-bearing updates, conditional aggregates,
field projections and `Self` construction.

Capture already observes body tokens and original declarations. It now retains
existing declaration-origin entries needed by successfully captured body types,
rather than pruning all entries absent from signature observations. Incomplete,
unused observations still do not add declaration entries. No frontend record,
provenance format, source snapshot or retained cache is introduced. Callable
parameter/result observations remain unchanged.

A temporary declaration inventory reconstructs bounded nominal trees from canonical
body tokens and their original declaration ranges. Field names/order, declaration
spelling, recursive types and repeated observations must agree. The same inventory
feeds existing Cargo package binding and independent lexical nominal verification,
including original file/range/digest, selected unit/package, source census, imports,
field spelling and owner checks. It is not a callable signature. Functions with no
declaration origins, or with all declarations already covered by the signature,
avoid body reconstruction. Earlier profiles keep their declaration boundary.

The composed real Cargo fixture covers imported aliases in another source module,
nested/tuple/unit constructors, locals, updates, conditional values, scalar/shared
results, a scalar helper composed with body-only construction, and `Self`
construction in an inherent implementation. It checks unchanged serialized
interface arity alongside retained body declarations. Every admitted graph passes
independent document and strict producer verification, and equivalent functions
reach accepted matching. Changed declaration identity, source range or observed
field layout is rejected. An earlier reference projection fixture now admits its
body-only constructor. Data-v2 retains its exclusions.

Local storage borrows, mutable reference-bearing locals and static associated helper
calls remain negative cases. The latter still require a broader ownership contract
than current shared-receiver helper certification; admitting the associated body
as a root does not establish that call boundary. Enums, traits/generics, macros,
derives, conditional modules and general ownership/drop remain outside this nominal
slice. S09 stays Partial, and the complete roadmap remains active.

Validation: 758 workspace tests pass with the same 5 existing ignores, including
the original assertion-argument receiver regression. The expanded 13-function
fixture additionally passes helper composition and unchanged interface-arity
checks (10 admitted, 3 excluded). Clippy passes for all targets with warnings denied;
formatting and diff checks pass. Repository-built rstyle reports 2,317 functions
and 0 violations; patterns reports 0 candidates with the same 15 existing ignores.
All 14 inventory bookkeeping tests pass. Inventory capture ran after the workspace
test process finished, avoiding executable replacement during test launches.

Frozen run `target/hir-coverage/run-2sony2uw/` completes all three packages with
11 admitted, 42 excluded and 7 independently inactive out of 60. All 60 frozen
identities/outcomes, all denominators and context counts match 3.24.0
(`run-fdc85x8c`). Twenty-six admitted graph documents pass schema checks
(10 lz4, 0 regex, 16 time). Body-only nominal types add no measured frozen admission
gain; these checks do not establish general performance or detector quality.


### C06 static associated helper calls

Data-v3 extraction 3.26.0 adds `rust.static-inherent-helper-calls` (38
capabilities); matching remains `rstyle.ccgraph.data-calls/1.0.0`. The
[contract](docs/extraction-profiles/static-inherent-helper-calls-v1.md) admits
receiver-free same-unit nongeneric inherent helpers through qualified paths,
import aliases and transitive `Self` calls. Owner values and field layouts are
unnecessary when the static function never receives or constructs them.

Capture and independent verification reuse existing inherent definitions, unit
associations, nominal identities and the closed lexical source index. Exact
implementation/function ranges, package identity and original declaration digest
remain checked. Shared receiver checks are unchanged. No new serialized records,
source snapshots or retained caches are introduced.

The composed Cargo fixture covers an owner with an unused `String` field, aliases,
qualified paths, zero arguments, unit results and transitive calls. Unsupported
local storage borrows remain excluded through callers. Owner/implementation/span
and identity corruption are rejected; admitted graphs pass independent and strict
verification and equivalent callers reach accepted matching. The earlier body-only
nominal fixture now admits its static helper call (11 admitted, 2 excluded).
C06 remains Partial; trait/generic dispatch, broader receivers, callee contracts,
cross-unit linking and the full roadmap remain active.

Validation: 759 workspace tests pass with the same 5 existing ignores, including
the original assertion-argument receiver regression and the strengthened static
ownership corruption checks. Clippy passes for all targets with warnings denied;
formatting and diff checks pass. Repository-built rstyle reports 2,325 functions
and 0 violations; patterns reports 0 candidates with the same 15 existing ignores.
All 14 inventory bookkeeping tests pass. Inventory capture ran after the workspace
test process finished, avoiding executable replacement during test launches.

Frozen run `target/hir-coverage/run-ftluk_j2/` completes all three packages with
11 admitted, 42 excluded and 7 independently inactive out of 60. All 60 frozen
identities/outcomes, all denominators and context counts match 3.25.0
(`run-2sony2uw`). Twenty-six admitted graph documents pass schema checks
(10 lz4, 0 regex, 16 time). Static associated calls add no measured frozen admission
gain; these checks do not establish general performance or detector quality.


### D11–D12 reference rebindings with source and graph snapshot flow

Data-v3 extraction 3.27.0 adds `rust.reference-rebindings` (39 capabilities);
matching remains `rstyle.ccgraph.data-calls/1.0.0`. The
[contract](docs/extraction-profiles/reference-rebindings-v1.md) permits whole shared
references and reference-bearing tuple/struct values in mutable local/formal
bindings. Whole assignment kills the prior binding definition. Earlier saved
values retain their own read snapshots, including component projections and updates.

Temporary source kill/gen follows existing HIR binding identities and original RHS
operations through branches, scalar matches/pure guards, short circuits and finite
loop fixed points. Return/break/continue retain their transfer roles. Only reference-
bearing definitions enter the flow state; functions without mutable reference-bearing
bindings retain their existing origin reconstruction. Source and serialized-graph
origin queries carry component selectors and visit each operation/component pair
once per query, traversing cycles to all formal terminals without loop unrolling.
No serialized records, source snapshots or retained caches are added.

Independent document verification compares source-derived and graph-derived formal
origins. Existing serialized CFG kill/gen checks the reaching-definition edges;
strict producer verification remains required. The data verifier recognizes the
existing unit-valued break/continue control operations alongside boolean predicates.
The composed Cargo fixture asserts exact return origins for assignment kills,
snapshots, branches, loops, saved-value swaps, aggregate replacements, updates,
mutable formals, scalar matches/guards and supported shared helper results, including
a caller of a rebinding callee. A release-build fixture distinguishes active
assertion assignments from disabled debug-assertion source operands. Missing
binding edges and changed origin annotations are rejected in branch and cyclic
cases. Earlier fixtures now admit their whole-value mutable/rebound cases.

This does not add writes through references, component mutation, local storage
borrows, reference-bearing arrays or aggregate helper results. Mutable/multiple-
origin shared-local field-memory reads and conditional expression merges in
loop-containing functions remain unfinished. D11–D12 stay Partial and the full
roadmap remains active.

Validation: 761 workspace tests pass with the same 5 existing ignores, including
the original assertion-argument receiver regression. The 18-function reference-flow
fixture admits 17 functions and excludes the local storage borrow; exact return
origins, branch/cyclic graph corruption and Data-v2 exclusions pass. The release
assertion fixture also passes independent and strict verification, preserving the
pre-assertion origin when debug assertions are disabled. Clippy passes
for all targets with warnings denied; formatting and diff checks pass.
Repository-built rstyle reports 2,362 functions and 0 violations; patterns reports
0 candidates with the same 15 existing ignores. All 14 inventory bookkeeping tests
pass. Inventory capture ran after workspace tests finished, avoiding executable
replacement during test launches.

Frozen final run `target/hir-coverage/run-myi5gqyy/` completes all three packages
with 11 admitted, 42 excluded and 7 independently inactive out of 60. All 60 frozen
identities/outcomes, all denominators and context counts match 3.26.0
(`run-ftluk_j2`). Twenty-six admitted graph documents pass schema checks
(10 lz4, 0 regex, 16 time). Reference rebindings add no measured frozen admission
gain; these checks do not establish general performance or detector quality.


### D12 field memory reads through reference-value flow

Data-v3 extraction 3.28.0 adds `rust.indirect-field-reads` (40 capabilities);
matching remains `rstyle.ccgraph.data-calls/1.0.0`. The
[contract](docs/extraction-profiles/indirect-field-reads-v1.md) supports field reads
through mutable and multiple-origin shared-reference bindings, snapshots,
conditional bases, stored references and verified shared helper results. Nested
physical projections retain their complete paths. A caller may compose a verified
memory-reading helper using these reads under the existing helper-call contract.

Temporary location derivation reuses formal reference origins, resolved fields and
existing memory access records. It adds no capture records, source snapshots or
retained cache. Functions without indirect reads skip the new location installation.
The evaluated pointer retains lexical snapshot dependencies; the memory read has
no lexical symbol or reaching-definition edge. Intermediate nested places do not
invent an aggregate load. Source and graph origin traversal union the possible
loaded origins, while independent document and strict producer verification check
the exact physical accesses and evaluated base, including conditional merges.

The 17-function composed fixture admits 15 functions and excludes the local
storage borrow and pointee write. It checks reassignment kills, saved local/formal
snapshots, loop origin unions, nested physical paths, stored-reference return
origins, shared helper results and a memory-reading helper caller. Missing reads,
invented symbols, incorrect operand positions, altered physical paths, pointer
operands and phantom lexical edges are rejected. Data-v2 field-read exclusions
remain intact; its pre-existing memory-free shared helper remains supported.
The earlier shared-local fixture now admits mutable, multiple-origin and local
aggregate reference bases.

Local/temporary storage borrows, general dereference/indexing, reference-bearing
arrays and aggregate helper results remain unfinished. D12 stays Partial, and the
full roadmap remains active. These checks do not establish general performance
or detector quality.

Validation: 762 workspace tests pass with the same 5 existing ignores, including
the original assertion-argument receiver regression and the expanded corruption
checks. Clippy passes for all targets with warnings denied; formatting and diff
checks pass. Repository-built rstyle reports 2,381 functions and 0 violations;
patterns reports 0 candidates with the same 15 existing ignores. All 14 inventory
bookkeeping tests pass. Inventory capture ran after the preceding workspace tests
finished; the final workspace run also passes after the verifier span prefilter.

Frozen run `target/hir-coverage/run-fyvdv1kb/` completes all three packages with
11 admitted, 42 excluded and 7 independently inactive out of 60. All 60 identities
and outcomes, all denominators and context counts match 3.27.0 (`run-myi5gqyy`).
Twenty-six admitted graph documents pass schema checks (10 lz4, 0 regex, 16 time).
Indirect field reads add no measured frozen admission gain.


### D14 explicit built-in shared dereference reads

Data-v3 extraction 3.29.0 adds `rust.shared-dereference-reads` (41 capabilities);
matching remains `rstyle.ccgraph.data-calls/1.0.0`. The
[contract](docs/extraction-profiles/shared-dereference-reads-v1.md) connects original
unary syntax, built-in dispatch, sized referent/result type checks, formal physical
locations and evaluated pointer dependencies. Nested explicit dereferences retain
each stored-pointer load. A dereference used as a field place preserves the final
physical field path instead of inventing a whole-aggregate read.

The existing indirect-read installer and independent verifier now handle field and
dereference reads. Both use existing access/location records with no lexical read
symbol; evaluated pointer operations carry snapshot dependencies. Stored-reference
loads derive component origins from every possible physical location. Helper read
summaries accept each construct only under its own registered capability. The
shared source-operation traversal replaces parallel field/unary traversals. No
capture records, source snapshots or retained caches are added, and source checks
already performed by location derivation are not repeated at the frontend call site.

The 18-function composed Cargo fixture admits 16 functions, excluding a local
storage borrow and a pointee write. It includes scalar and tuple loads, nested
pointer reads, direct/nested physical field places, stored references, returned
shared references, conditional and mutable bases, snapshots, reads in loops,
reference flow across loops, a memory-reading helper caller and a dereference of a
verified memory-free shared helper result. Assertions compare exact formal roots,
physical path lengths, the final nested member, and returned-reference origin
paths. Missing reads, invented lexical symbols, wrong operand positions and
invented scalar operators are rejected by independent and strict verification.
Data-v2 still excludes the dereference functions; its existing memory-free shared
helper remains supported.

Local/temporary storage borrows, address-taking through dereference places,
mutable/raw pointer access, overloaded dereference, unsized whole-value loads,
general repeated implicit dereferences and reference-
bearing arrays remain unfinished. D14 stays Partial. D13 and the full roadmap
remain active; these checks do not establish general performance or detector quality.

Validation: 763 workspace tests pass with the same 5 existing ignores, including
the original assertion-argument receiver regression. Clippy passes for all targets
with warnings denied; formatting and diff checks pass. Repository-built
`cargo rstyle --format json` reports 2,390 functions and 0 violations;
`cargo rstyle patterns --format json --fail-on-candidates` reports 0 candidates with
the same 15 existing ignores. All 14 inventory bookkeeping tests pass. Executable
builds and workspace tests finish before inventory capture.

Frozen run `target/hir-coverage/run-p_9a7q3l/` completes all three packages with
11 admitted, 42 excluded and 7 independently inactive out of 60. All 60 identities
and outcomes, all denominators and context counts match 3.28.0 (`run-fyvdv1kb`).
Twenty-six admitted graph documents pass schema checks (10 lz4, 0 regex, 16 time).
Explicit shared dereference adds no measured frozen admission gain.


### D14 reference-bearing loads during rebinding flow

The 3.29.0 follow-up routes reference-valued unary operations through the source
flow resolver's terminal dispatch. The loaded-reference terminal already derived
physical origins, but its caller omitted unary expressions when mutable reference
flow was enabled. Such functions were incorrectly excluded. No new capability,
matching policy, capture record or retained state is required.

The shared-dereference fixture now has 22 functions: 20 admit, while the local
storage borrow and pointee write remain excluded. Four added cases assert exact
returned origins for a loaded reference saved before reassignment, a load after
reassignment, a load after a loop with both possible roots, and a selected
component of a saved aggregate load. Independent and
strict shared-unit verification pass alongside the legacy exclusions.

Validation: all 763 workspace tests pass with the same 5 existing ignores.
Clippy with warnings denied, formatting and diff checks pass. Repository-built
rstyle reports 2,390 functions and 0 violations; patterns reports 0 candidates with
the same 15 existing ignores.

Frozen follow-up run `target/hir-coverage/run-8dgqxypr/` retains 11 admitted,
42 excluded and 7 independently inactive out of 60. All 60 identities/outcomes,
all denominators and context counts match `run-p_9a7q3l`. Twenty-six admitted graph
documents pass schema checks (10 lz4, 0 regex, 16 time). All 14 inventory bookkeeping
tests pass. This correction adds no measured frozen admission gain.

### D15 indexing evaluation-order acceptance notes

D15 remains Missing. The next implementation must distinguish place evaluation
from reading the value of that place. A compiler MIR experiment using
`rustc --crate-type lib --emit=mir` confirms that `a[{ a = b; 0 }]` indexes the
new `b` value for both owned array locals and shared slice-reference locals.
Likewise, `holder.bytes[{ holder = other; 0 }]` reads the new holder's field.
In contrast, `({ selected })[{ selected = other; 0 }]` retains the earlier
computed reference temporary. These cases must be included in the composed Cargo
fixture; always snapshotting the base value before evaluating the index would
misrepresent the first three cases.

The element read must retain the evaluated index and the requirement that it be
less than the selected array/slice length on normal completion. Index bounds and
source correspondence are semantic obligations, not a reason to introduce source
snapshots or retained provenance caches. General exceptional flow remains a
separate roadmap item.


### Scalar indexing: Data-v3 3.30.0

Adds `rust.scalar-index-reads` (42 capabilities), with unchanged Data matching.
The [contract](docs/extraction-profiles/scalar-index-reads-v1.md) specifies scalar
owned/shared array and shared slice reads, access-relative indexed locations,
normal-completion bounds requirements, and the indexed-helper boundary. No capture
records, source snapshots or retained caches are added.

MIR experiments confirm that local/field places materialize their base after the
index expression, whereas computed bases preserve their earlier value. Source
reference flow and graph lowering follow this ordering while retaining base/index
operand positions zero/one. The composed Cargo fixture covers 25 functions:
18 admitted and 7 intentional exclusions, exact origin and value snapshots,
shared template access counts, independent/strict import, and corrupt bounds,
locations and operand rejection. D15 remains Partial; D13–D18 and the broader
roadmap remain active.

Validation for this indexing milestone: 764 workspace tests passed with 5 existing
ignores; workspace clippy and formatting passed. Repository-built rstyle checked
2412 functions with 0 violations; patterns reported 0 candidates. All 14 inventory
tests passed. Frozen rerun `target/hir-coverage/run-1sbtdgtr/` preserves all 60
identities/outcomes, denominators and context counts from `run-8dgqxypr/`:
11 admitted, 42 excluded, 7 inactive (0 gained, 0 lost). Capture validated 26 graph
documents (10 lz4_flex, 0 regex-syntax, 16 time-core). This is foundational scalar
index support, not completion of the subslice/conversion/checked-access/helper
chains needed by the natural read-only targets. No performance gain is claimed.


### Next target: mixed-content Position/Span APIs

The [development target and blocker audit](eval/coverage-development/README.md)
selects four natural regex-syntax 0.8.11 constructors/getters, with source-derived
semantic expectations, build context, negative cases and explicit unknowns.
This is separate from the unchanged frozen inventory. S09/module/import proof
comes first; splat and by-value updates are connected follow-ons. No new admission
or complete source-boundary contract is claimed by this planning checkpoint.

### Reassessment after the conditional-attribute foundation

The [development plan](eval/coverage-development/README.md#reassessment-active-read-only-target)
now prioritizes the complete safe lz4_flex get_batch/get_batch_arch family. Existing
capture demonstrates a connected range/subslice, TryInto/Result/unwrap and native
byte conversion chain, plus local size constants and array-reference loads for
the architecture variant. Position/Span remains the S09 investigation target;
its missing unit-wide expansion/impl ownership proof makes it a larger prerequisite
than the initial four bodies suggest. This reassessment follows foundational work
without natural gains; it does not change the frozen sample or claim admission.
External associated-function capture now reuses existing library Callee and typed
Dispatch records. Identity observation alone does not authorize a call summary.


The subsequent subslice evidence checkpoint retains overloaded Index and rustc
range-type identities using existing callee records; it does not admit ranges.
The [independent runtime oracle](eval/coverage-development/subslice-order.rs)
and pinned MIR establish base-before-endpoints snapshots, unlike builtin scalar
indexing. Full validation and unchanged frozen classifications are recorded in
the development plan. The next connected support work remains subslice views,
bounds/source verification and subsequent conversion/Result/byte-read composition.


### Connected bounded subslice views: Data-v3 3.31.0

Adds `rust.shared-subslice-views` (43 capabilities). The
[contract](docs/extraction-profiles/shared-subslice-views-v1.md) connects original
range syntax, pinned core identities, base-before-endpoints lowering, normal-flow
bounds, saved/nested/shared-helper-result bases, reference origins and later
scalar reads. It reuses existing capture and graph records. No source snapshots
or retained caches are added; internal read installation and bounds serialization
are shared with existing index/dereference handling.

The composed Cargo tests exercise admitted views and later reads through
independent document verification, strict replay and matching. Corruption cases
reject missing bounds, duplicated endpoint roles and invented reads. The source
reference-flow fix preserves the independent runtime/MIR snapshot oracle while
keeping builtin scalar-index rebinding behavior. Existing helper summaries cannot
claim bounds-preserving/nonpanicking view calls, so those callees remain excluded.
This advances the declared lz4_flex chain; Result/conversion/native-byte operations
remain required before natural reader admission can be claimed.

Validation: 770 workspace tests passed with 5 existing ignores; workspace clippy
and formatting passed. Repository-built rstyle checked 2441 functions with 0
violations and patterns reported 0 candidates. The 14 inventory tests passed.
Frozen rerun `target/hir-coverage/run-yy464a72/` preserves all 60 identities/outcomes,
denominators and context counts relative to `run-tw6vnfnv/`: 11 admitted, 42
excluded, 7 inactive (0 gained, 0 lost); 26 graph documents validated. Natural
get_batch no longer reports index/range-expansion/additional-type blockers but
still lacks the required connected call summaries. The next milestone targets
conversion/Result/native-byte composition together, not separate syntax counts.


### Natural copied byte-window admission: Data-v3 3.32.0

The complete safe lz4_flex 0.14.0 `block::compress::get_batch` now admits through
whole unmodified-library Cargo capture, independent verification, strict admission
and matching. This connects subslice bounds/origins, endpoint arithmetic, concrete
slice-to-array conversion, typed Result payload/failure and native byte semantics;
see the [bounded contract](docs/extraction-profiles/copied-byte-windows-v1.md).
The development corpus also exercises local Result/array values, saved views,
helper-selected shared slices and endpoint rebinding. The independent arithmetic
runtime oracle is included in workspace tests.

The conversion records its compiler-selected blanket TryInto implementation in
the existing callee vocabulary. Its precise slice/array/error signature and pinned
core source authority justify the delegated TryFrom semantics. No general trait
or enum support follows. Conversion conditionally reads the evaluated four-byte
view on Ok; unwrap keeps its Ok normal-completion requirement and disclosed panic
flow; native conversion preserves each byte's little-endian significance. Calls
remain separate typed graph operations. No source snapshots or retained caches
were added, and unrelated non-byte functions skip byte-call installation. Byte
reads reuse the existing shared location/effect installer and existing call build
verification is shared. No broader performance claim follows.

Frozen rerun `target/hir-coverage/run-57ulgkuc/` compared with `run-yy464a72/` keeps
all 60 source identities and every denominator unchanged. The sole outcome change
is the selected safe get_batch at bytes 1298..1416, excluded to admitted:
**12 admitted / 41 excluded / 7 inactive; 1 gained, 0 lost**. There are 28 validated
graph documents (12 lz4_flex, 0 regex-syntax, 16 time-core). Historical inventory
artifacts and inactive unsafe alternatives remain unchanged.

The family is not complete. `get_batch_arch` still combines a local size_of const,
named array length, borrowed-array conversion/Result origins, explicit array load
and usize native-byte conversion; these are the next connected delivery target.
S09 mixed-content Position/Span proof remains unresolved. General Option/Result
branching and payload binding, F10 loop/value merges, and the remaining roadmap
continue after the selected family; broad async/unsafe/FFI/macros remain deferred.

Final validation: 776 workspace tests passed with 5 existing ignores, including
the assertion-argument receiver regression. Workspace clippy (`-D warnings`),
formatting and diff checks passed. Repository-built rstyle checked 2468 functions
with 0 violations; patterns reported 0 candidates, with thresholds/ignores
unchanged. All 14 inventory tests passed. The final inventory rerun
`run-n1ejrbwy` reproduced `run-57ulgkuc` exactly on selected identities, outcomes,
denominators and context counts. Byte and metadata-call verification share one
source-call inventory, avoiding a second whole-body scan. Independently importing
the retained natural get_batch bundle, verifying its documents and strictly
admitting it also succeeded; matching against the declared development expression
was evaluated and accepted at score 1 under the unchanged configuration. This is
a composition check, not held-out detector-quality evidence or rewrite authority.

The [architecture-reader source audit and preselected expectations](eval/coverage-development/README.md#architecture-reader-target-established-before-support)
now specify the next complete chain, including separately owned constant proof,
named length, conditional borrowed Result origins, indirect array reads and native
usize authority. Eight separate development functions retain a tested 3.32 baseline
of 0 admitted / 8 excluded; intended positives and negatives are defined from
source independently of that baseline. Runtime checks exercise saved-view/rebinding
and offset pointer identity. This is target preparation, with no extraction change
or claimed natural gain. The next support delivery remains the entire architecture
reader chain, preserving get_batch and the unchanged frozen inventory.
