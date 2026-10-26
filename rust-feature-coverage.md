# Rust feature coverage and roadmap

A practical master checklist for processing real-world Rust through the **strict
HIR source-to-pair (`cargo rstyle clones`) pipeline**. This is not a checklist for
legacy `patterns`, a Rust language specification, or a claim of complete Rust
semantics. Real-world coverage is a ladder, not one final syntax switch.

Audited against `4a0e346` (2026-10-15), after the performance work. The latest
implemented selection is **`hir-data-v3`**, extraction
`rstyle.rust.hir-data/3.120.0`, matching `rstyle.ccgraph.data-calls/1.0.0`.
Data-v3 3.55–3.57 connects byte classifiers, const-built bool tables and exact
named Option callbacks through verified helpers. Data-v3 3.58 adds borrowed owners
with independently checked Copy child fields. Data-v3 3.59 connects shared indexed
reads through verified helpers, retaining saved inputs and possible panic.
Data-v3 3.60 adds complete fieldless flag values and verified shared Copy
receivers through their original constructor/query helper chains. Data-v3 3.61
adds standalone Unicode scalar literals, comparisons and literal/OR matching.
Data-v3 3.62 connects literal character ranges and the pinned ASCII predicate;
3.63–3.66 connects original float kernels through exact bits/constants, ordered
arithmetic/narrowing, bounded i32-controlled float alternatives and independently
checked cfg-if module/dependency proof. Data-v3 3.67 adds complete borrowed
mixed-owner type and lifetime-only callable evidence; 3.68 adds exact passive
Copy dependency proof, connecting five original Input getters. Data-v3 3.69
connects stored-slice results to checked consumers; 3.70 connects checked usize
addition/multiplication and guarded Option if-let through original base64 lengths.
Data-v3 3.71 connects immediate shared-helper Option callbacks through original
Transition predicates, including copied borrowed byte payloads. Data-v3 3.72
connects original Span/Match lengths; 3.73 connects builtin bool-to-usize and
literal crate documentation inputs through original base64ct lengths.
Data-v3 3.74–3.75 connects pinned alloc imports and fresh Range helper results;
3.76–3.77 connects bounded scalar-loop helpers and pinned primitive constants.
Data-v3 3.78–3.79 connects source-proved no-drop unit errors, fresh Results and
exact same-error `?` through original Unicode capability checks. Data-v3 3.80
connects checked shared-reference Option propagation and Copy product results
through three original Memchr prefilter prefix methods. Data-v3 3.81 connects
checked prefix access, a no-drop non-Copy error, borrowed Result `?` and suffix
origins through the original lz4_flex block-header parser. Data-v3 3.82 connects
the original calendar wrappers through selected dependency-library scalar helpers
and independently proved explicit-u8 enum ownership.
Data-v3 3.83 connects checked scalar instances across selected packages; 3.84
adds local component-specific shared-reference helper results. Data-v3 3.85
activates the original byte Input constructor as a separately owned checked
instance; 3.86 adds exact string-to-byte origin projection. 3.87 connects both
original From callers to their checked constructor and component summaries.
3.88 connects original Line tail extraction through Copy-error scalar Result
propagation and checked suffix views, preserving saved values and backing origins.
Foreign aggregate callers remain pending. See the
[constructor contract](docs/extraction-profiles/reference-constructor-instances-v1.md).
3.89–3.98 add component-aware foreign helper results, checked Match instances,
mutable scalar loop state, exact Default/TryFrom selections and fresh no-drop
nominal errors through original checked SmallIndex construction. 3.99 connects
[generated PatternID construction](docs/extraction-profiles/constructor-result-maps-v1.md)
through exact constructor-only Result maps. General payload binding, owned Result
storage and generic generated callables remain outside these contracts.
Broader character/float APIs, conversions and structured forms remain deferred.
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
| V03 | Scalar literals and built-in operators | Covered | `x + 1`, `x < y`, `!flag` | Admitted integer/bool dispatch only; Data-v3 3.55 adds [source-checked byte literals and escapes](docs/extraction-profiles/word-byte-classifiers-v1.md) as concrete u8 values. Overloaded operators remain excluded; arithmetic hazards can retain partial-flow evidence. |
| V04 | Initialized scalar locals, shadowing, and reads | Covered | `let x = n; let x = x + 1;` | Exact binding identity, saved values, and reaching definitions. |
| V05 | Local scalar assignment and compound update | Covered | `x = y; x += 1;` | Lexical scalar bindings; Data-v3 3.34.0 adds [flat simultaneous scalar tuple assignment](docs/extraction-profiles/scalar-tuple-assignment-v1.md) to distinct mutable locals. Field/index/dereference memory writes remain excluded. |
| V06 | Scalar blocks and expression tails | Covered | `{ let y = x; y }` | Normal-flow evaluation and saved operands are retained. |
| V07 | Integer-to-integer casts | Covered | `x as u64` | Resolved built-in integer casts only. |
| V08 | Ordinary const-function bodies | Covered | `const fn f(x: u32) -> u32 { x }` | Function qualification is supported; this is not general compile-time evaluation. |
| V09 | Floats and floating-point operations | Partial | `x: f64`, `x / 2.0` | Standalone f32/f64, exact bits/constants, ordered f64 +/*/−, sign negation and f64→f32 narrowing connect the [original float kernels](docs/extraction-profiles/float-kernels-v1.md) through Data-v3 3.66, including exhaustive float alternatives under checked i32 equality; comparisons, division, f32 binary arithmetic and nested floats remain unsupported. |
| V10 | Character values and operations | Partial | `c: char`, `'a'` | Data-v3 3.61 supports standalone Unicode scalar interfaces/locals, exact literals, builtin comparisons and literal/OR/wildcard matching through verified helpers; see [character contract](docs/extraction-profiles/character-leaf-v1.md). Data-v3 3.62 adds literal ranges and the pinned ASCII predicate through the [escape classifier](docs/extraction-profiles/character-escapes-v1.md); other casts/APIs and structured character types remain deferred. |
| V11 | Additional built-in casts/coercions | Partial | `flag as u8`, `x as f32` | Builtin f64→f32 narrowing has a bounded contract; bool/char and other float conversions remain unsupported; pointer casts belong to unsafe work. |
| V12 | Named constants and associated constants | Partial | `x + LIMIT`, `S::LIMIT` | Data-v3 3.3.0 supports same-unit primitive scalar constants and nongeneric inherent constants, evaluated by rustc; see [constant contract](docs/extraction-profiles/scalar-constants-v1.md). Data-v3 3.33.0 additionally verifies the original top-level local `const usize = core::mem::size_of::<usize>()` for [native byte readers](docs/extraction-profiles/native-borrowed-byte-readers-v1.md). Data-v3 3.56 adds [owned local const-built bool arrays](docs/extraction-profiles/word-byte-classifiers-v1.md) with complete evaluated contents and lossless unsigned index bounds. Other aggregates and nonlocal/generic/trait constants remain missing. |
| V13 | General const expressions and const blocks | Missing | `[u8; N + 1]`, `const { 1 + 2 }` | General array expressions remain missing. Data-v3 3.33.0 permits one verified local native-size constant as a byte-array annotation length; arithmetic lengths and const blocks remain unsupported. Const fn support does not authorize these. |
| V14 | Uninitialized locals / definite assignment | Missing | `let x; x = 1; x` | Declaration without initializer is currently rejected. |
| V15 | Destructuring and wildcard bindings | Partial | `let (a, b) = pair;`, `let _ = x;` | Data-v3 3.47.0 supports only `let _ = local;` for an exact scalar or checked-Copy nominal place, with no value read, binding or drop. General wildcard initializers, destructuring and wildcard parameters remain missing. |
| V16 | Type aliases | Partial | `type Count = u32;` | Data-v3 3.53 proves direct nongeneric same-module scalar aliases against the closed original declaration. Imported, qualified, chained, local-item and aggregate aliases remain excluded; see [packing contract](docs/extraction-profiles/accelerator-packing-v1.md). |
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
| F06 | OR/range/constant patterns | Partial | `1 \| 2 => x`, `1..=9 => y`, `LIMIT => z` | Data-v3 3.10.0 supports integer/bool literal/constant/range/wildcard leaves in nested grouped OR patterns, guards and independently exhaustive matches; see [nested contract](docs/extraction-profiles/nested-scalar-or-patterns-v1.md). Data-v3 3.55 adds byte ranges/OR patterns and decoded u8 exhaustiveness through the [ASCII classifier family](docs/extraction-profiles/word-byte-classifiers-v1.md). Data-v3 3.62 adds literal character ranges; nonlocal/generic/trait constants remain missing. |
| F07 | Destructuring, reference, slice, and enum patterns | Partial | `Some(x) => x`, `[a, ..] => a` | Data-v3 3.36.0 supports exhaustive checked-access `Some(&scalar)`/`None` patterns with verified guarded loads and bindings. Data-v3 3.42.0 adds immutable owned scalar `Some(value)`/`None` bindings with guarded value projection. Data-v3 3.47.0 adds exhaustive immutable scalar tuple-variant bindings for checked Copy local enums through the [tagged-unit family](docs/extraction-profiles/tagged-units-v1.md). General payload/field/element patterns and ownership/borrow effects remain missing. |
| F08 | `if let`, `while let`, let chains, `let else` | Partial | `if let Some(x) = v { ... }` | Data-v3 3.70 connects immutable owned `Option<usize>` Some binding with explicit else, saved initialization and early return through [original base64 lengths](eval/coverage-development/base64-encoded-len-selection.json). Other payloads/patterns, while-let, chains and let-else remain deferred. |
| F09 | Scalar `while`/`loop` and innermost exits | Covered | `while n > 0 { n -= 1; }` | Scalar state, nesting, early return, unlabelled valueless `break`/`continue`; divergence-sensitive flow. |
| F10 | Value merges in loop-containing functions | Partial | `while b { n = if c { x } else { y }; }` | Data-v3 3.35.0 supports [outside-loop value merges](docs/extraction-profiles/outside-loop-value-merges-v1.md), completing the selected scalar gcd chain. Data-v3 3.41.0 additionally supports [bounded cyclic prefix scans](docs/extraction-profiles/cyclic-prefix-scans-v1.md): reducible loop-body/condition merges with finite iteration-local availability. General iterator/value-carrying exit and broader caller compositions remain open. |
| F11 | Labelled and value-carrying exits | Missing | `'outer: loop { break 'outer; }`, `break x` | Exact target resolution and value/frontier dependencies; also labelled block expressions. |
| F12 | Iterator `for` loops | Missing | `for x in xs { ... }` | Iterator protocol, yielded values, effects, drop, and source/desugaring correspondence; not covered by scalar `while`. |
| F13 | `Option`/`Result` branching and `?` | Partial | `let x = parse()?;` | Data-v3 3.36.0 connects slice `get` to exhaustive `Some(&scalar)`/`None` branching. Data-v3 3.42.0 adds [owned bool/integer Option values](docs/extraction-profiles/scalar-option-byte-widths-v1.md), exact Some/None construction, early returns, saved local values and supported helper transport. Data-v3 3.51–3.52 connect [pinned integer Results and immediate checked-sub Option unwrap](docs/extraction-profiles/end-of-input-v1.md). Data-v3 3.78–3.79 adds fresh `Result<(), E>` for independently proved no-drop unit-field errors and [exact same-error `?`](docs/extraction-profiles/same-error-result-propagation-v1.md), including method forwarding and nested/sequential composition. Data-v3 3.80 connects [Option reference `?` into Copy products](docs/extraction-profiles/checked-reference-option-propagation-v1.md). Data-v3 3.81 connects [borrowed Result parser flow](docs/extraction-profiles/checked-borrowed-result-flow-v1.md), reconstructing a fresh same-error exit when the success type changes. Data3.119 connects [checked arithmetic to fresh named errors](eval/coverage-development/wire-arithmetic-errors/README.md), preserving conditional static-field origins and complete no-drop proof. Callable arithmetic Results, general payload binding, saved Results, other residual conversions and cleanup remain missing. |
| F14 | Assertion macros | Partial | `assert!(b);`, `debug_assert!(b);` | Data-v3 3.6.0 supports unformatted standard assertions, original invocation/condition correspondence, one condition evaluation, debug settings and disclosed omitted panic flow; see [assertion contract](docs/extraction-profiles/scalar-assertions-v1.md). Data-v3 3.52.0 adds [literal scalar diagnostics](docs/extraction-profiles/end-of-input-v1.md); Data-v3 3.53 adds standard two-operand bool/integer assert_eq!/debug_assert_eq! with ordered snapshots. Explicit messages on comparisons, custom comparisons/formatting and general exceptional flow remain missing. |
| F15 | General panic/unwind/cleanup flow | Missing | `panic!("bad")`, `catch_unwind(f)` | Current extraction is bounded normal flow; omitted applicable exceptional flow is disclosed, not represented as complete. |

Evidence: [cycle/match contracts](docs/extraction-profiles/coverage-v1.md),
[planned scalar composition](docs/extraction-profiles/compositional-data-v1.md),
[eligibility](crates/rstyle-pdg/src/eligibility.rs).

## 3. Concrete data, places, references, and memory

| ID | Feature | Status | Small example | Current boundary / remaining work |
| --- | --- | --- | --- | --- |
| D01 | Tuple and literal-length array interfaces | Covered | `p: (u32, bool)`, `a: [u8; 4]` | Interface support is distinct from construction, indexing, and reference-bearing element origins. |
| D02 | Concrete local struct interfaces | Partial | `s: S`, `s: &S` | Finite supported field trees with independent source/ownership proof. Data-v3 3.40.0 permits borrowed-only scalar-field owners in mixed units without certifying Copy or drop-free ownership; Data-v3 3.58 additionally permits independently checked Copy struct children through the [ASCII-boundary target](docs/extraction-profiles/ascii-boundaries-v1.md). See S09. |
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
| D15 | Array/slice element reads | Partial | `xs[i]` | Data-v3 3.30.0 supports [scalar index reads](docs/extraction-profiles/scalar-index-reads-v1.md) over owned arrays and shared arrays/slices, evaluated place/value snapshots, access-relative element selectors and explicit normal-completion bounds requirements. Data-v3 3.46.0 connects [nested fixed-array byte membership](docs/extraction-profiles/byte-membership-v1.md): unsigned bounds, Copy array owners and nonpanicking value-index helpers. Data-v3 3.59 connects [shared indexed-read helpers](docs/extraction-profiles/indexed-read-helpers-v1.md), including nested/repeated calls, saved inputs and possible panic. Aggregate/reference elements and general exceptional flow remain unfinished. |
| D16 | Slice ranges and borrowed subslices | Partial | `&xs[start..end]` | Data-v3 3.31.0 supports [bounded shared subslice views](docs/extraction-profiles/shared-subslice-views-v1.md) with scalar elements, exact base/start/end snapshots and normal-completion bounds. Data-v3 3.44.0 adds leading `..end` views through the complete [native prefix-reader family](docs/extraction-profiles/native-prefix-readers-v1.md). Data-v3 3.50 adds checked shared array-field views and helper-return transport; 3.53 connects implicit array-field byte copies and read-only callers. Data-v3 3.81 adds exact exclusive suffix views with saved base/start, explicit bounds and reference-bearing tuple results through the [block-header parser](docs/extraction-profiles/checked-borrowed-result-flow-v1.md). Inclusive ranges, mutable views and str/UTF-8 rules remain unfinished. |
| D17 | Checked access and concrete Option | Partial | `xs.get(i)`, `Some(x)`, `None` | Data-v3 3.36.0 connects immutable scalar-slice get, exhaustive Some(&scalar)/None, saved Option flow and scalar helper composition through [strict graph verification](docs/extraction-profiles/checked-slice-option-v1.md). Data-v3 3.42.0 adds [owned bool/integer Option construction and consumption](docs/extraction-profiles/scalar-option-byte-widths-v1.md). Data-v3 3.48.0 adds exact core is_some on owned bool/integer Options with checked shared-autoref evidence, composing through tagged-unit helpers. Data-v3 3.80 connects checked `Option<&u8>` propagation into independently proved Copy-product Options, preserving guarded reference origins and fresh output-typed None exits. Data-v3 3.81 adds checked `get(..end)` with saved receiver/end, guarded shared-prefix origins and eager no-drop error conversion. General payload matching, mutable/array access, other checked ranges and broader Result propagation remain open. |
| D18 | Reference-bearing array element origins | Missing | `refs: [&S; 2]` | Type observation alone lacks the per-element origin evidence needed for by-value reference-bearing arrays. |
| D19 | General enums and variants | Partial | `enum E { A(u32), B }` | Data-v3 3.47.0 connects checked-Copy local enums with 1–16 single bool/integer tuple payloads, exact constructor tags, complete disjoint unguarded matches, conditional payload bindings and helper composition; see [tagged-unit contract](docs/extraction-profiles/tagged-units-v1.md). Data-v3 3.60 adds [fieldless flag enums](docs/extraction-profiles/look-flags-v1.md) with complete source-checked literal/shift discriminants, variants, casts and helper composition. Data-v3 3.81 inventories named scalar error variants and proves complete no-drop ownership for fresh unit-error Result transport. Data-v3 3.82 adds complete lexical and compiler proof for bounded explicit-u8 fieldless enums through the [calendar chain](docs/extraction-profiles/cross-unit-scalar-helpers-v1.md). General mixed constructors/matches, named/reference/generic payload flow, discriminant expressions, moves/drop and general Result remain missing. |
| D20 | Mutable references and reborrowing | Partial | `x: &mut S`, `&mut *x` | Data3.93: direct `&mut usize` formal roots with conservative aliases and complete bounded loop memory flow. Reborrows, local storage and escaping mutable results remain unsupported. |
| D21 | Field/index/dereference memory writes | Partial | `s.value = x;`, `xs[i] += 1;`, `*r = x;` | Data3.93: direct mutable-scalar compound writes, cyclic RAW/WAR/WAW and normal/checked-failure post-state. Plain, field and indexed stores, reborrows and general cleanup remain unsupported. |
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
| C01 | Same-unit direct scalar helpers | Covered | `helper(x)` | Resolved definition and verified summary, source-order arguments, no inlining. Callee must be supported, acyclic and loop-free. Data-v3 3.52.0 adds [checked-Copy read-only normal flow](docs/extraction-profiles/end-of-input-v1.md) with explicit possible-panic propagation; exceptional cleanup remains omitted. Zero arguments and unit results work; unit-valued argument positions remain unsupported across the helper boundary, including later helper rows. |
| C02 | Qualified paths and named import aliases | Covered | `module::helper(x)`, `use module::helper as h; h(x)` | Supported same-unit resolution; this is not general type aliases, glob imports, or external linking. |
| C03 | Memory-free helpers with shared inputs | Covered | `choose(left, right, flag)` | Supported canonical shared types and actual arguments; shared input alone does not authorize memory reads. |
| C04 | Whole shared-formal helper results | Covered | `identity_ref(xs).len()` | Verified selected-formal return origins and actual substitution; not a field-reading borrowed-result summary. |
| C05 | Direct memory-reading shared-input helpers | Covered | `read_field(s)` | Data-v2/v3: exact per-occurrence reads and actual-origin substitution, preserving alias uncertainty. Data-v3 3.52–3.53 connect reference-free checked-Copy results and copied-byte reads while retaining possible panic and callee read conditions. |
| C06 | Concrete inherent method calls and UFCS | Partial | `s.value()`, `S::value(s)` | Data-v3: same-unit nongeneric inherent impl/method, shared receiver or [static associated calls](docs/extraction-profiles/static-inherent-helper-calls-v1.md), exact owner/source proof and supported summary. Shared receivers require adjustment proof. Data-v3 3.38.0 adds [owned Copy receiver snapshots](docs/extraction-profiles/owned-copy-receiver-v1.md) for scalar-returning read-only helpers. Data-v3 3.39.0 adds [checked Copy value-self updates](docs/extraction-profiles/owned-copy-self-v1.md) under the existing body/summary restrictions. Data-v3 3.46.0 extends checked Copy owners to fixed scalar-array fields and connects bounded array-value reads through existing whole-value snapshots. Data-v3 3.47.0 connects checked-Copy enum owners and scalar Option helper results through the tagged-unit constructors/projections. No general trait dispatch. Data-v3 3.60 proves builtin shared dereference into a checked Copy value receiver, retaining the complete implicit load and saved origins; see [flags](docs/extraction-profiles/look-flags-v1.md). |
| C07 | Receiver adjustments and coercions | Partial | `s.value()` | Data-v3 inherent helper method syntax requires an exact shared type and one builtin shared dereference followed by one shared borrow. Registered integer methods have scalar by-value receivers; metadata methods also accept an empty adjustment chain. Data-v3 3.38.0 also supports exactly one shared borrow of a checked scalar-struct Copy value for a scalar-returning read-only helper. Data-v3 3.39.0 accepts an empty adjustment chain for checked Copy immutable untyped value-self receivers. General autoref, repeated/overloaded deref, unsizing and mutable inherent receivers remain unavailable. |
| C08 | Registered integer methods | Partial | `x.unsigned_abs()`, `x.wrapping_mul(y)` | Pinned i32/i64 unsigned_abs and u32/u64 wrapping_mul; Data-v3 3.70 adds 64-bit usize checked_add/checked_mul, and 3.72 adds usize saturating_sub. Other primitive operations and widths remain unavailable without their own contracts. |
| C09 | Slice/str length and emptiness | Covered | `xs.len()`, `text.is_empty()` | Four registered nonmutating operations; exact supported compiler/core/target resolution and receiver evidence. Slice elements must be supported non-reference-bearing types; a shared-slice interface alone does not authorize these calls on reference-bearing elements. No buffer-content read is invented. |
| C10 | General core/std/third-party APIs | Partial | `u32::from_ne_bytes(bytes)`, `hint::unlikely(b)` | Data-v3 3.32.0 connects [copied four-byte slice conversion, concrete Result/unwrap and native u32 conversion](docs/extraction-profiles/copied-byte-windows-v1.md) under pinned aarch64 core authority. Data-v3 3.33.0 also connects the [borrowed eight-byte Result, array read and native usize chain](docs/extraction-profiles/native-borrowed-byte-readers-v1.md). Data-v3 3.43.0 connects [u16-to-u32 From and checksum-state roundtrips](docs/extraction-profiles/checksum-state-roundtrips-v1.md) with pinned trait/implementation identities. Data-v3 3.44.0 connects [native prefix readers](docs/extraction-profiles/native-prefix-readers-v1.md): shared prefixes, exact scalar size calls, copied 2/4/16-byte Results/unwrap and native u16/u32/u128 conversion. Data-v3 3.46.0 additionally registers exact lossless u8-to-usize From on the three pinned 64-bit CI targets for the [byte-membership family](docs/extraction-profiles/byte-membership-v1.md). Data-v3 3.48.0 connects exact u16-to-usize From and owned scalar Option::is_some through the [tagged-unit classifiers](docs/extraction-profiles/tagged-unit-classifiers-v1.md), including the original byte-class lookup composition. Data-v3 3.51–3.52 add [population counts](docs/extraction-profiles/bitset-cardinality-v1.md), pinned widening/narrowing TryFrom and checked-sub/unwrap composition. Other APIs need resolved summaries/bodies and effect/result proof; familiar names are insufficient. |
| C11 | By-value memory-reading helper arguments/results | Missing | `inspect_value(s)`, `make_value(x)` | Pure reference-free construction results compose in the D11 fixture; by-value memory-reading substitution, reference-bearing helper results and ownership remain unavailable. Supported interface types do not establish that call boundary. |
| C12 | Borrowed results from memory-reading helpers | Missing | `get_field_ref(s)` | Exact callee read effects plus result origin/location substitution, not just whole-formal forwarding. |
| C13 | Cross-unit/package helper linking | Partial | `library_crate::helper(x)` | Data-v3 3.82 connects [selected dependency-library scalar helpers](docs/extraction-profiles/cross-unit-scalar-helpers-v1.md) through exact Cargo/loaded/emitted artifact and definition associations, separate owning stores and strict closure replay. Generic instances, foreign nominal/reference/memory summaries and binary/test roots remain unsupported. |
| C14 | Looping, recursive, panicking or nonreturning callees | Missing | `recursive(x)`, `helper_with_loop(x)` | Existing caller scalar loops do not authorize these summaries; need interprocedural fixed points/flow/effect contracts. |
| C15 | Function items as values, pointers and indirect calls | Partial | `value.map_or(false, classify)` | Data-v3 3.57 connects [immediate local fn(u8) -> bool items](docs/extraction-profiles/word-byte-classifiers-v1.md) to exact verified helpers, eager arguments, guarded payload invocation and result selection. Data3.99 additionally supports exact unary tuple constructors as immediate fresh Result map/map_err callbacks under the [generated constructor contract](docs/extraction-profiles/constructor-result-maps-v1.md). Stored items, function pointers, general callbacks and indirect calls remain missing. |
| C16 | Closures and captured environments | Partial | `move \|x\| x + offset` | Data-v3 3.49 supports [immediate Option<u8>::map_or equality callbacks](docs/extraction-profiles/tagged-byte-comparisons-v1.md): one by-value u8 payload and one shared u8 capture, eager default, Some-only body and independently checked result merge. Data-v3 3.54 also verifies the [immediate shared-byte position callback](docs/extraction-profiles/iterator-byte-position-v1.md), including `\|&b\|` and stable shared capture storage. General bodies, captures, escape/lifetime and drop remain open. |
| C17 | Iterator adapters and higher-order functions | Partial | `xs.iter().position(\|&b\| b == byte)` | Data-v3 3.54 connects one [fresh shared byte iterator](docs/extraction-profiles/iterator-byte-position-v1.md), exact equality callback, first-match prefix reads and Option/helper composition. Stored iterators, arbitrary predicates, adapters, other element types and general protocols/cleanup remain excluded. |
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
| T01 | Concrete trait implementations and default methods | Partial | `impl Trait for S`, `s.trait_method()` | Data3.94 connects original local nongeneric core Default implementations to zero-argument direct-prelude calls returning verified reference-free Copy products, through exact trait/impl/member selection and ordinary body/effect summaries. [Bounded family](eval/coverage-development/adler-default/README.md). Existing local From exceptions remain bounded; general trait dispatch, provided default bodies, generics and drop-sensitive results remain unavailable. |
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
| S02 | Ordinary modules and named imports | Partial | `mod child;`, `use child::S as Local;` | Closed inline/external module census and supported lexical aliases; Data-v3 3.64 independently proves exact local module-only cfg-if expansion chains. Unresolved/glob/external dependencies and general expansion remain restricted. |
| S03 | Active conditional-source correspondence | Partial | A cfg-controlled statement present in resolved HIR | Original-source/HIR correspondence in the actual build, with Data-v3 3.7.0 resolution of body-level cfg removal; general conditional signatures/attributes remain unfinished. |
| S04 | Cfg-removed source and inactive definitions | Partial | `#[cfg(feature = "x")] statement` absent from HIR | Data-v3 3.8.0 resolves standard cfg/cfg_attr guards for removed body operations and definitions in captured original files, including enclosing scopes; see [removal contract](docs/extraction-profiles/cfg-source-removal-v1.md). Unloaded external files and macro-token definitions remain unfinished; missing capture is not proof. |
| S05 | Conditional/path-selected modules | Partial | `#[path = "other.rs"] mod m;` | Cfg-selected ordinary modules and Data-v3 3.64 exact local cfg-if module ownership have source/build proof. Rewritten paths and general generated modules remain unsupported. |
| S06 | Declarative macros | Partial | `macro_rules! m { ... }`, `m!()` | Data3.99 joins exact local all-ident template substitution, complete selected impl/member/HIR/hygiene and independent source/cleanup proof for the [generated checked constructor](docs/extraction-profiles/constructor-result-maps-v1.md). Standard assertions retain their dedicated contract (F14). Sequential unknown expansions, general matcher forms, generated callables and hidden item/effect ownership remain unsupported outside these bounds. |
| S07 | Derives and procedural/attribute macros | Missing | `#[derive(Clone)]`, `#[some_attribute]` | Generated definitions/impls, expansion provenance, source correspondence and effects; source spelling alone is insufficient. |
| S08 | Included/build-generated code | Missing | `include!(...)`, generated `OUT_DIR` files | Expansion/input/build ownership and original-source identity beyond the current admitted subset. Build scripts compiling is not graph support for generated source. |
| S09 | Nominal proof in ordinary mixed-content crates | Partial | A getter beside an enum, const, trait impl or body macro | The legacy unit whitelist accepts ordinary functions, structs, modules, uses and restricted inherent impls. Data-v3 3.11.0 additionally accepts constant declarations and inherent associated constants with recursive initializer checks; see [constant-context contract](docs/extraction-profiles/nominal-constant-context-v1.md). Data-v3 3.25.0 additionally supports [nominal types used only in bodies](docs/extraction-profiles/body-nominal-types-v1.md), including scalar-returning construction/projection without a signature witness. Data-v3 3.37.0 adds [checked Copy declarations in mixed source](docs/extraction-profiles/copy-nominal-boundary-v1.md), retaining precise cfg/module/import and lexical expansion gates. Data-v3 3.38.0 connects owned Copy local autoref to exact helper-read snapshots. Data-v3 3.40.0 adds [borrowed scalar owners](docs/extraction-profiles/borrowed-scalar-owner-v1.md) without owned Copy permission. Data-v3 3.43.0 accepts exact builtin inline function attributes, admitting the original adler2 constructor/getter in its complete std/no_std mixed units. Data-v3 3.45.0 adds [closed local import scopes](docs/extraction-profiles/scoped-import-flag-owners-v1.md) and checked Copy value helper reads, connecting original Epsilons constructor/getter methods in the complete regex-automata library. Data-v3 3.47.0 reuses those source/Copy proofs for complete finite scalar enum declarations, admitting original Unit constructors/projections beside their unchanged derives and custom Debug impl. Data-v3 3.58 adds borrowed-only owners with independently checked Copy child structs; see [ASCII boundaries](docs/extraction-profiles/ascii-boundaries-v1.md). Data-v3 3.64–3.66 narrows ancestor namespace checks only for exhaustively proved call-free float leaves, including bounded i32 control dependencies; general nominal import restrictions remain unchanged. Data-v3 3.68 permits selected Copy dependencies only inside a distinct shared owner, with complete source/field/Copy proof and no direct dependency operations; original Input getters admit beside unchanged index-type macros. Data-v3 3.80 proves exact local trait-member source ownership and pinned core Debug trait/derive namespace bindings; closed unrelated globs retain type/value shadow checks. This does not enable trait-call dispatch. Data-v3 3.81 recovers exact no-drop error certificates across original module globs and one-hop aliases, with complete export/shadow checks. General non-Copy ownership, unresolved/custom expansions, rewritten module paths and general local storage borrowing remain open. |
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
| X06 | Interprocedural dependencies and invalidation | Partial | Editing a getter invalidates its callers | Supported same-unit acyclic dependencies work. Data-v3 3.82 adds selected scalar dependency closures with strict owning-bundle and actual artifact reconstruction; stale/missing associations reject. Recursion, broader foreign effects and general instance linking remain missing. |
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

### Complete architecture-reader chain and CI portability (Data-v3 3.33.0)

The selected safe lz4_flex `get_batch_arch` now passes complete Cargo capture,
independent document verification, strict admission and matching in its original
library context. The connected contract covers the separately owned local
`size_of::<usize>()` const, declaration-bound array length, borrowed slice-to-array
Result, conditional Ok origins, unwrap failure, actual array load and native usize
decoding. Each call remains a separate typed operation. The preselected development
corpus yields 5 admitted / 3 excluded; saved Results/views, helper-selected roots,
endpoint rebinding and repeated reads also compose. The reader caller remains
excluded because the existing helper contract cannot transport its possible panic.
See [the bounded contract](docs/extraction-profiles/native-borrowed-byte-readers-v1.md).

The reported macOS subslice regressions exposed an ARM Linux-only authority check.
Subslice, byte-call and native-size verification now use separate pinned registries
for Linux ARM64, Linux x86_64 and macOS ARM64. Exact compiler, core crate,
definition fingerprints, source bindings and concrete signatures remain mandatory;
unknown or mismatched authorities remain excluded. macOS capture uses LLVM triple
`arm64-apple-macosx`, distinct from Cargo's `aarch64-apple-darwin` target name.
Real Cargo cross-target regression checks exercise all three installed targets,
including both reported subslice forms, reader composition and metadata calls.
They also reject substituting another registered target or an unknown compiler.
This establishes cross-target capture/admission, not local native macOS execution.

The frozen selection remains unchanged: the architecture reader advances the safe
two-reader family; no broad Option/Result, iterator or mutation support follows.
The next family audit should prioritize F10 loop/value merges and concrete checked
access/Option composition. S09 mixed-content nominal ownership remains unresolved;
neighboring derives, impls and drop behavior still require exact evidence.

Validation at this boundary: 782 workspace tests passed, with 5 existing ignores;
all 14 inventory tests passed. Clippy across all targets with warnings denied,
formatting and diff checks passed. Repository-built rstyle checked 2489 functions
with 0 violations; patterns reported 0 candidates. No thresholds, scoring or
ignore settings changed. The retained natural architecture-reader bundle was
independently imported, verified and strictly admitted, then matched against the
preselected development reader: Accepted, score 1, under unchanged matching rules.

Final frozen rerun `target/hir-coverage/run-akr9djbd/` retains all 60 exact source
identities and every denominator from `run-n1ejrbwy`: 60 selected contexts,
53 active/captured/assessed and 7 independently inactive. The sole transition is
safe lz4_flex `get_batch_arch`, bytes 1784..2020, excluded to admitted:
**13 admitted / 40 excluded / 7 inactive; 1 gained, 0 lost**. All outcomes match
the earlier architecture run `run-12i29cvg`; the portability fix loses no selected
admissions. There are 30 validated documents: 14 lz4_flex, 0 regex-syntax and
16 time-core. Historical results and frozen identities remain untouched.

### Next connected target: scalar Euclidean reduction / F10

The [source-selected gcd target audit](eval/coverage-development/README.md#scalar-euclidean-reduction-target-specified-before-support)
now records the complete natural context, both blocker chains and independent
positive/negative expectations. Actual full-library Cargo capture confirms that
`frac::gcd` is blocked by tuple-assignment desugaring/binding/source correspondence
and by its post-loop value merge. S09 does not block this primitive leaf in its
mixed-content crate. Cyclic-value verification independently rejects merges, so
removing eligibility rejection alone cannot deliver support.

The separate seven-function development selection retains all definitions:
1 admitted / 6 excluded. The admitted sequential loop is independently verified;
the natural routine and outside-cycle merges remain excluded. Runtime divisor
enumeration checks all 1024 small pairs, boundary u128 inputs and simultaneous
assignment using the exact production function source. This checkpoint establishes
a concrete blocker chain without changing extraction or the frozen selection.
The bounded next delivery is simultaneous scalar-local tuple assignment plus
finite proof for pre/post-loop merges; merges inside cycles and memory-backed
assignments remain explicit negatives. F10 remains Missing until delivery and
will remain Partial until epoch-sensitive composition is covered.

Validation for this preparation: all 168 CLI tests passed, including the portable
reader regressions and both new baseline/runtime tests. Workspace all-targets
clippy with warnings denied, formatting and diff checks passed. Repository-built
rstyle checked 2489 functions with 0 violations; patterns found 0 candidates.
The frozen result remains 13 admitted / 40 excluded / 7 inactive; this
source/test/documentation-only checkpoint changes no extractor behavior and did
not rerun the frozen inventory. No thresholds, ignores or scoring changed.

### Scalar tuple assignment advances the selected gcd chain (Data-v3 3.34.0)

Data-v3 3.34.0 adds `rust.scalar-tuple-assignment`, under the
[bounded contract](docs/extraction-profiles/scalar-tuple-assignment-v1.md).
Capture checks the compiler's complete flat destructuring assignment, then
retains original targets and RHS source with existing identities/types. Lowering
constructs the RHS tuple once and snapshots all operands before ordered writes to
distinct scalar locals. Independent aggregate verification treats the LHS as
places and verifies the RHS constructor. No persisted provenance/caches, subtree
copies per target or whole-function scans are introduced.

Real Cargo composition covers tuple updates inside a scalar loop, RHS mutation
order, one-element bool assignment, and repeated/nested/memory/nonliteral
negatives. Independent imported graph checks prove original formal definitions
feed RHS reads; swapped write operands and corrupted capture evidence are
rejected. Duplicate step functions are strictly admitted and matched without
changing scoring. The source-selected runtime divisor oracle still passes.

The unchanged seven-function development selection gains tuple_step only:
**2 admitted / 5 excluded; 1 gained, 0 lost**. The complete production-library gcd
recapture still excludes the natural routine, now solely for F10; missing generated
bindings and source-correspondence gaps are gone. This is one connected foundation,
not completed family support. The next delivery remains finite proof for pre/post
loop merges in eligibility and the independent serialized verifier, preserving
rejections for epoch-sensitive merges inside loops. No natural admission gain is
claimed for this foundation.

Frozen rerun `target/hir-coverage/run-kb15su8j/` exactly preserves the prior
`run-akr9djbd` selection: all 60 identities and outcomes, every denominator,
13 admitted / 40 excluded / 7 inactive, and 30 verified graph documents
(14 lz4_flex, 0 regex-syntax, 16 time-core). Frozen new/lost admissions: 0/0.
All 14 inventory tests passed. Final workspace validation passed 785 tests with
5 existing ignores, including the assertion-receiver and three-target portable
reader regressions. All-targets clippy with warnings denied, formatting and diff
checks passed. Repository-built rstyle checked 2498 functions with 0 violations;
patterns reported 0 candidates. Thresholds, scoring and ignores are unchanged.

### Complete selected gcd chain with outside-loop merges (Data-v3 3.35.0)

Data-v3 3.35.0 adds `rust.outside-loop-value-merges`; see the
[finite value-availability contract](docs/extraction-profiles/outside-loop-value-merges-v1.md).
It replaces the function-wide exclusion with a bounded source restriction on
merges inside loop bodies/conditions. Independent serialized proof uses finite
relevant-controller states and a monotone must-evaluation worklist, with exact
alternative frontiers, guard partitions, snapshot availability and consumer
successor checks. Repeating merge controllers/snapshots remain excluded. No
bounded loop unwinding, virtual exits or missing-data-as-zero assumptions appear.
The existing no-merge cyclic path and historical policy registrations remain.

The exact source-selected production `frac::gcd`, SHA
cab1f7e45cca54d452ddb6c50ab82781229cf81bd320a413868fbb4f647c0648, now passes its
complete original rstyle-ccgraph library Cargo capture. Its one-function natural
development selection gains one admission with none lost. The whole context
retains all 413 functions; only gcd admits. Independently importing the retained
natural bundle, verifying its documents and strictly admitting it succeeds.
Matching it to the preselected development gcd is evaluated and Accepted at
score 1, with no diagnostics and unchanged thresholds/scoring.

The unchanged seven-function development selection advances 2/5 to **5 admitted /
2 excluded; 3 gained, 0 lost**. All predeclared positives now admit; cyclic_merge
and memory_target remain excluded. Runtime divisor enumeration and wide-u128
expectations continue to pass. Composition tests cover nested/short-circuit/match
values, break exits, changing loop-guard outcomes, loop-containing alternatives
and a value expression after divergence. Consistently corrupted paired guards,
snapshots, frontiers, successors and cyclic controllers fail independent validation
and strict admission. A separate exhaustive small-graph simple-path oracle checks
the must-evaluation fixed point, including repeated CFG vertex labels.

Frozen rerun `target/hir-coverage/run-h7tfyznr/` preserves every identity, outcome
and denominator from `run-kb15su8j`: **13 admitted / 40 excluded / 7 inactive;
0 gained, 0 lost** across the same 60 selections. There are 30 verified documents
(14 lz4_flex, 0 regex-syntax, 16 time-core). All 14 inventory tests passed.
Final workspace validation passed 787 tests, with 5 existing ignores, including
previous assertion-receiver and three-target reader regressions. The additional
stronger frontier/successor mutations passed their focused rerun. All-targets
clippy with warnings denied, formatting and diff checks passed. Repository-built
rstyle checked 2517 functions with 0 violations; patterns reported 0 candidates.
The value checker now reuses its already parsed operation map for lookups instead
of repeatedly scanning its JSON records. No new persisted provenance or caches
were introduced; no broad performance or detector-quality claim follows.

The selected scalar leaf family has no remaining blockers in its declared
context. F10 is Partial: value merges inside iterations need iteration-specific
proof. `Frac::new` still needs the separately audited closure/conversion/Option /
Result / `?` chain; admitting its leaf does not certify that caller. Next target
audits remain connected checked-access/Option families and S09 mixed-content
ownership, preserving strict source completeness and nominal/drop evidence.

### Next connected target: checked UTF-8 boundary predicate

Selected from source before admission: `regex-automata` 0.4.18
`util::utf8::is_boundary`, in its complete default-feature Cargo library context.
The [development target and independent expectations](eval/coverage-development/README.md#checked-utf-8-boundary-target-specified-before-capture)
connect slice get, concrete Option discrimination, reference payload binding,
scalar short circuits and helper/saved-origin composition. The five-function
separate corpus starts at 0 admitted / 5 excluded; its runtime oracle passes
against standard string boundaries and all 256 saved byte payloads.

Full natural capture is currently **incomplete**, not excluded: selected-unit
inherent association validation aborts with ambiguous source-definition spans.
Resolve the concrete collision before claiming any natural admission gain;
removing uniqueness checks or omitting neighboring definitions is not a fix.
Existing get callee/library identities are captured, while normalized Option
results and payload bindings are absent. This bounds the next investigation
without adding persisted provenance or changing frozen selections. D17 remains
Missing, S09 remains Partial, and no extraction gain is claimed by this baseline.

### Expanded inherent associations no longer abort natural capture

Resolved the first checked-boundary source-context blocker: the two index-type
macro invocations in regex-automata each produce twelve inherent methods at a
shared callsite. Expanded signatures now retain facts and macro exclusions
without claiming original inherent-definition associations. Ordinary method
associations and the ambiguity validator remain intact. No persisted provenance
or caches were added.

The whole default-feature library now compiles and captures completely; strict
assessment remains incomplete at independent source inventory (`expected fn`).
No natural admission gain is claimed. This advances the declared checked-access
chain to its next confirmed boundary; Option get/discriminants/payload matching
still require the connected contract. The focused retained-function regression
and workspace suite pass (788 tests, five existing ignores); clippy, rstyle
(2,518 functions, zero violations) and patterns (zero candidates) pass.

The unchanged frozen selection was rerun in `run-p_h_oqof` and compared with
`run-h7tfyznr`: all 60 identities, outcomes and denominators agree. Results remain
13 admitted / 40 excluded / 7 inactive, 30 verified documents, zero gains/losses.

### Receiver handoff validates observations before source semantics

The checked-boundary context's next failure was a `thread_local!` initializer:
its generated function retains the macro invocation as source, while initializer
methods contributed from the argument have unexpanded spans. Handoff and library
resolution inventory checks now use observational receiver validation. Full
receiver source correspondence remains mandatory for semantic call resolution
and paired document verification; no generated-function support is claimed.
See the [receiver contract](docs/extraction-profiles/receiver-verification-v1.md)
and [development evidence](eval/coverage-development/README.md#receiver-observations-do-not-require-function-source-certification).

The real Cargo regression retains the generated initializer as excluded and
admits an unrelated scalar leaf. Independent strict bundle import gives the
initializer no graph; full receiver validation still rejects its unavailable
function-source correspondence, and observational validation rejects a forged
receiver identity. No serialized fields, provenance records, caches, matching
changes or extra whole-unit passes were added. Validation: 789 workspace tests
pass with five existing ignores; all-targets clippy, formatting, diff checks,
rstyle (2,521 functions, zero violations) and patterns (zero candidates) pass.

Frozen run `run-wxw_9e2e` agrees with `run-p_h_oqof` on all 60 source identities,
outcomes and denominators: 13 admitted / 40 excluded / 7 inactive, 30 verified
documents, zero gains/losses. The independently selected natural boundary leaf
still requires the connected get/Option/payload contract; fixture scalar gains
are not counted as natural coverage gains.

Natural regex-automata validation now completes capture and assessment of all
2,582 functions: six admitted / 2,576 excluded, including all 550 generated
functions excluded. The selected `is_boundary` is still excluded for the declared
get/Option/payload chain. Its earlier natural baseline was incomplete, so the
six current imports are not claimed as measured new admissions. Matching was
intentionally resource-incomplete at max-pairs=0 (15 pairs required).

Reassessment: the two source-context foundations have resolved the observed
context blockers but produced no selected-leaf gain. Next deliver the connected
Option type, checked-access operation, resolved variant/reference payload binding
and guarded/saved-origin composition; defer additional capture infrastructure.
S09 nominal Position/Span proof remains open. Full readable reports expand unit
sources per function (7.9 GB here); use text stdout and stream retained reports
for further exploration, without introducing new provenance or caches.

Natural-run execution limit: the persisted `run-HCmpVF/report.json` was complete
and streamed successfully, but its redundant JSON stdout renderer was terminated
at the memory boundary (exit 143). Capture/assessment results above are validated
artifact results, not a successful CLI-exit claim. Frozen and regression/style
runs completed normally. Use text stdout for subsequent full-library runs.


### Bounded core Option observations retain payload bindings

Data-v3 now observes core Option with scalar or shared-scalar payloads and retains
resolved Some/None patterns, immutable reference patterns and their nested
bindings. Renamed imports retain compiler identities; lookalike enums, mutable
and nested Option payloads remain unsupported. Historical Data-v1/v2 records
remain unchanged. Owning nominal structs containing Option remain unsupported
as a whole; no fields are omitted. See the
[observation contract](docs/extraction-profiles/checked-option-observations-v1.md).

This is a foundation for the selected checked UTF-8 boundary, not semantic
Option support: semantic type parsing and admission remain closed. The unchanged
five-function development selection remains zero admitted / five excluded, now
retaining the natural leaf's `b` binding and Some/None relationship. Remaining
blockers are registered checked-get bounds and conditional reference origins,
variant guards and guarded payload loads, then saved Option/helper composition
through independent verification, strict admission and matching. D17 stays open.

Frozen run `run-uhc11x7c` matches `run-wxw_9e2e` on all 60 identities, outcomes
and denominators: 13 admitted / 40 excluded / 7 inactive, 30 verified documents,
zero gains/losses. Validation: 792 workspace tests pass with five existing
ignores, including the three reported assertion/subslice regressions; focused
Option tests and all-targets clippy pass. Formatting/diff checks, rstyle (2,529
functions, zero violations) and patterns (zero candidates) pass.


The unmodified default-feature natural library run `run-8UPUKk` completed with
text stdout: capture=true and assessment=true, 2,582 assessed, six admitted /
2,576 excluded, zero unknown/failed. The retained exact `is_boundary` source
identity and its payload binding were inspected in the bundle. Admissions are
unchanged from `run-HCmpVF`; the selected leaf remains excluded. Exit 2 reflects
intentional max-pairs=0 incompleteness (enumeration/evaluation=false), not a
capture failure; no end-to-end matching gain is claimed.


### Checked Option source and type contract

The selected boundary chain now has independently checked get/Some/None
compiler authority, exact shared-slice/usize/Option instantiation, original
operand spans, exhaustive unguarded variant shapes and scalar-copy binding
identity. Real Cargo captures and contract checks pass on aarch64 Linux,
x86_64 Linux and Apple Silicon macOS, including renamed imports, reversed arms
and saved operands. See the
[bounded contract](docs/extraction-profiles/checked-option-source-contract-v1.md).

Review of pinned core slice indexing establishes `i < len` as Some of the
reference at i, otherwise normal None; forming the Option does not load the
byte. The independent host oracle now additionally verifies receiver-before-index
evaluation and saved-reference identity during index-side rebinding. All three
oracle tests pass, alongside guard/reference-pattern/array/range negatives and
corrupted authority, operand, type and binding checks.

No complete function family becomes supported in this source-contract increment.
Semantic Option parsing, summaries and admission remain closed; the extraction
policy/capabilities and matching profile are unchanged. The concrete remaining
chain is explicit discriminators and conditional payload origins, guarded scalar
loads, saved Option flow, and helper composition through independent serialized
verification and strict matching. D17 stays Missing. Further observation or
source-context infrastructure is deferred: the next integration must exercise
that complete graph chain rather than count these local checks as admissions.

The contract reuses the existing target definition map and borrowed source nodes;
there are no new serialized records, provenance tables, caches or per-function
capture passes. Patterns identified duplicate original-expression parsing with
tuple assignments; both now use one parser, preserving expansion rejection.
The workspace passes 795 tests with five existing ignores; all-targets clippy,
formatting/diff checks, rstyle (2,545 functions, zero violations) and patterns
(zero candidates, unchanged existing ignores) pass.


Frozen run `run-efb6__3d` was compared with `run-uhc11x7c`: all 60 source
identities, outcomes, counts and denominators are identical. Results remain
13 admitted / 40 excluded / 7 inactive, 30 verified documents, zero gains/losses.
The unchanged five-function development selection retains identical source
hashes and outcomes, with complete capture/assessment and zero admitted / five
excluded. No new full-library natural run or matching gain is claimed here.

Direct inspection of the retained development facts through the new APIs also
verifies both local contracts for the exact natural `is_boundary` excerpt,
`checked_byte` and `saved_byte`, and rejects `mutable_byte`. The helper caller
has no local get/match contract to check; its eventual support still depends on
the connected callee summary. These local results do not alter their excluded
admission outcomes.


### CI regression revalidation (2026-10-20)

The reported assertion-receiver and shared-subslice failures are covered by
`1f867f1`, already on `feature/rust-feature-coverage`. Subslice authority uses
the registry selected by exact compiler commit, target and pointer width,
then verifies the core crate and definition hashes; it does not bypass
authority checks for another architecture.

Revalidation passes the three originally reported CLI tests: assertion argument
receivers, shared subslice endpoints/saved origins, and base snapshots before
endpoint rebinding. The installed-target regression also passes real Cargo
capture and independent document verification on aarch64 Linux, x86_64 Linux
and Apple arm64 macOS. These are cross-target captures, not native macOS runtime
execution. No selection, denominator, threshold or admission rule changed for
this revalidation.

The separate Option graph integration remains uncommitted and requires its
remaining semantic validation and style cleanup. A style scan of that working
tree reports 21 violations and four patterns candidates (15 existing ignores
unchanged); this regression revalidation does not certify that
uncommitted increment as complete or style-clean.

### Connected checked scalar reads (Data-v3 3.36.0)

The selected `regex-automata` 0.4.18 `util::utf8::is_boundary` now completes its
whole declared chain: slice get, exhaustive None/Some(&scalar), conditional
payload origins, guarded scalar load, scalar binding and short circuits. Saved
Option values, receiver/index evaluation order and scalar-result helper calls
compose through the [connected contract](docs/extraction-profiles/checked-slice-option-v1.md).
D17 is Partial; general Option interfaces/construction, reference payload
bindings, guards, arrays/ranges, mutation, Result and `?` remain open.

Natural evidence uses all 2,582 raw functions from the unmodified default-feature
library Cargo capture `run-O3vn08`, including neighboring/generated items. That
run's historical report remains 6 admitted / 2,576 excluded: it exposed a final
no_std prelude display-path blocker. The core Some/None hashes are identical to
the std prelude hashes. Only those exact aliases now share the pinned compiler,
core and definition checks; cross-target tests exercise both preludes.

After that correction, the old bundle correctly fails strict import because its
root inventory lacks the newly eligible function. All 2,582 raw frontend records
were reassessed in a fresh UnitStore, its complete proof inventory serialized,
and that bundle independently strictly imported. Current classification is
**7 admitted / 2,575 excluded / 0 unknown**, with no failed or omitted functions.
Every retained name, source digest and span matches the earlier natural baseline
`run-8UPUKk`; the only changed outcome is `util::utf8::is_boundary` becoming
admitted. All previous admissions and generated exclusions remain intact.
The replay artifact is `/tmp/rstyle-option-natural-replayed.unit.json`; the old
report was preserved rather than relabeled as a new successful capture.

Each admitted natural pair passes independent data-document and producer
verification and strict ccgraph admission. The exact natural `is_boundary`
(source SHA256 `e676b73906d493177219bcc72dbbb29fce630d0c07570e1ec1b7c3378b10c826`)
and development copy match with score 1, Accepted, without incomplete evaluation
or diagnostics. Full-library matching was not run: its capture used max-pairs 0.
This establishes extraction coverage for the selected family, not held-out
quality, abstraction activation or rewrite safety.

The unchanged five-function development selection improves from 0 admitted /
5 excluded to **4 admitted / 1 excluded**, four gains and zero losses. All six
admitted pairs evaluate completely. Mutable access remains excluded. Final frozen
replay `run-579vcwmp` matches `run-efb6__3d` on all 60 identities, outcomes and
denominators: **13 admitted / 40 excluded / 7 inactive**, 30 verified documents,
zero gains/losses. No thresholds, scores, source selections or historical
artifacts changed.

The independent runtime oracle passes three tests covering standard Unicode
boundaries, all 256 bytes, out-of-range indices, saved-reference pointer identity
and receiver-before-index rebinding. Cargo tests strictly import composed graphs
and check equivalent matching and exact saved/helper locations. Corruptions of
get semantics, no-read claims, operands, variants, payload reads, bindings and
Some guards are rejected. Both std and no_std contract tests pass for aarch64
Linux, x86_64 Linux and Apple arm64 macOS; these cross-target captures do not
claim native macOS runtime execution.

The next caller is still excluded. Independent assessment of
`util::search::Input::haystack` reports missing exact owning receiver impl and an
unsupported receiver type. `Input::is_char_boundary` additionally needs the
haystack method summary and returned-reference composition. Source inspection
shows `Input<'h>` has a Clone derive, a borrowed slice, Span, Anchored and bool;
Anchored contains a PatternID alternative. Precise owner/module evidence and
those nominal dependencies must be resolved before admitting this caller.
Admitting the scalar leaf does not certify it. S09 and cyclic F10 remain open;
no general enum, macro, async, unsafe or FFI scope is added here.

The implementation adds semantic operations and a conditional checked-element
formal projection, using existing source, summary, location and proof tables.
It adds no capture inventory, source copies, provenance cache or per-invocation
identity. The source visitor checks once whether Option patterns exist, avoiding
extra per-binding searches in ordinary functions. No broad performance claim is
made from these functional checks.

Final validation passes 796 workspace tests (five existing ignores), all-targets
clippy with warnings denied, formatting and diff checks, rstyle (2,579 functions,
zero violations) and patterns (zero candidates, 15 existing ignores unchanged).
No suppression, ignore entry, threshold or scoring change was introduced.


### S09 prerequisite: builtin Copy declaration observations (2026-10-20)

The existing declaration-origin record now optionally carries the compiler-resolved
core Copy trait, builtin derive macro, and original derive-token span for Data-v3.
The observer uses rustc's type-indexed nonblanket implementation lookup and
requires a unique local automatically derived implementation whose expansion is
the external core Copy derive. Manual implementations and custom derives named
Copy do not receive this observation. Absence means unavailable evidence, never
non-Copy or drop-free. Earlier extraction profiles remain unchanged.

This is capture evidence only: no admission rule, capability, policy version or
matching behavior consumes it yet. Copy's prohibition on Drop provides a concrete
route to a precise selected-type drop proof, but the independent checker must
first pin both definition identities and bind the active derive attribute to the
exact original declaration. It must then establish selected module/name/source
closure without ignoring neighboring items or expansions. General derive purity
and general macro support are outside this contract.

Actual full-Cargo replay `run-o52m9whk` observes builtin Copy for both ast::Position
and ast::Span in published regex-syntax 0.8.11, default-feature library context.
The original Copy tokens are at bytes 13835..13839 and 12999..13003 respectively.
All four source-selected constructors/getters remain excluded: zero gains and
zero losses. These observations advance their shared derive/drop blocker; root
attributes, selected module/import ownership and mixed-content boundary proof
remain unresolved. Input's lifetime/enum/receiver dependencies remain separate.

The same replay preserves all 60 frozen identities, outcomes and denominators
relative to `run-579vcwmp`: 13 admitted, 40 excluded, seven inactive, with 30
verified documents. No historical report or selection changed. The optional
fields reuse existing declaration/source sharing; no separate capture inventory,
source snapshot, cache or global trait scan was added. No performance improvement
or absence of regression is claimed without comparative measurement.

Validation passes 801 workspace tests (five existing ignores), five focused Copy
capture tests, all-targets clippy with warnings denied, formatting and diff checks.
Tests cover strict bundle round trips, std/no_std on the three installed CI
targets, active/inactive cfg_attr, manual Copy, Drop owners, custom proc-macro
Copy derives and earlier profiles. Revalidation also passes the three reported
CI regressions and all three architecture tests, including real cross-target
capture and independent document verification. rstyle reports 2,581 functions
and zero violations; patterns reports zero candidates with 15 existing ignores.
S09 and D17 remain partial; this milestone does not support the natural family.


### S09 Copy observation verification and boundary replanning (2026-10-20)

Selected-unit binding now independently checks every present builtin Copy
observation. The existing pinned target registries distinguish the core trait
and derive namespaces despite their shared display path. The std/core display
aliases require identical exact crate/definition hashes; an implementation
payload, wrong path, unregistered hash or trait/derive substitution fails binding.
The original struct index retains only derive/cfg/cfg_attr metadata needed for
this check. It reuses the existing per-file parse and snapshot. Other attributes
remain in the original source and still require boundary proof; this index does
not certify their effects or omit them from that proof.

The checker requires the exact original derive path token on the exact struct
and evaluates its cfg_attr using captured build cfgs. It does not treat a correctly
spelled token in an inactive derive as evidence. Missing observations remain
unavailable, not negative or drop-free. Ancestor module closure and general
expansion ownership are not certified by this local attribute check.

Strict import of the complete 956-function regex-syntax bundle from
`run-o52m9whk` succeeds under these checks. All four selected function digests
remain exactly those recorded in the development target; Position has one and
Span has two checked declaration observations. All four remain excluded, with
zero new/lost admissions. The frozen 60-selection identity/outcome comparison
remains 13 admitted / 40 excluded / seven inactive. No scoring, policy capability,
threshold, selection or historical artifact changed.

Two consecutive S09 prerequisites have produced no admission gains. The next
milestone must connect the checked declarations to the complete natural family,
not add another capture inventory or independent syntax row. Inspection confirms
that Environment construction still demands the recursive unit whitelist before
resolution can use these observations. Its indexing also rejects unrelated generic
structs and glob/external imports globally. A precise replacement must establish
selected cfg/module ownership and relevant name/declaration dependencies while
retaining neighboring items and unknown expansion effects. The declaration span
convention must also remain consistent: origin spans exclude preceding attributes,
while the current resolution index uses syn item spans. Do not relax each global
check in isolation and infer support from clean miniature crates.

Validation includes the 801-test workspace suite (five existing ignores), then
all five Copy Cargo tests and all 108 PDG library tests after the attribute-retention
refinement. Negative checks reject forged hashes, swapped namespaces, unrelated
spans, implementation payloads and a forged Copy claim on the inactive struct's
own token. Original-byte tests include BOM, Unicode and CRLF. All three installed
CI targets retain std/no_std behavior. Clippy, fmt and diff checks pass; rstyle
reports 2,586 functions / zero violations and patterns zero candidates, with 15
existing ignores unchanged. No new suppression or ignore was added.


A local debug-build import comparison against `8d12e4c` uses the same retained
956-function bundle, identical dependency lockfiles and compiler, and six
alternating baseline/checked pairs. Median elapsed time is 5.335 s baseline /
5.405 s checked; median peak RSS is 508,850 / 508,838 KiB. This diagnostic finds
no material memory increase and about 70 ms additional median time on this input;
it is not a full-capture performance guarantee. An earlier comparison resolved
different dependency versions and is discarded. No benchmark threshold or
production cache was introduced to obtain these figures.

Strict replay also imports all three retained frozen-package bundles under the
new verifier: time-core 226 functions / eight graphs, lz4_flex 150 / seven, and
regex-syntax 956 / zero. No root inventory changed.


### S09 useful natural admissions through selected Copy declarations (2026-10-20)

Data-v3 is now 3.37.0 with 49 capabilities. The
[Copy nominal boundary](docs/extraction-profiles/copy-nominal-boundary-v1.md)
connects the existing checked observations to exact source declarations in
cfg-selected mixed modules. The legacy closed-unit proof remains available for
ordinary non-Copy structures. The new path requires nongeneric structs containing
only scalars and recursively checked structs, with a positive pinned builtin Copy
proof for every selected declaration. This establishes the relevant no-Drop law
without assuming that unrelated destructors are absent or that neighboring methods
are pure. Selected module/function/impl ownership, complete fields and lexical
expansion/name dependencies remain mandatory.

The index retains all original active items and all import forms, including
unresolved globs; a glob never supplies a type witness. Unknown expansions or
custom derives/attributes in an ancestor lexical scope block the selected scope.
Relevant proc-macro-name imports, globs and primitive aliases also remain closed.
Unrelated sibling contents remain retained, with generated/unsupported functions
still classified independently. Module cfg removal is explicit; rewritten paths,
unsafe module ownership, orphan files and unresolved module attributes fail.
`use path::{self, Name}` now correctly binds the module's actual name rather than
creating a spurious binding named self. Declaration ranges use the exact
attribute-excluding convention captured for the original struct.

Fresh real Cargo replay `run-9t3zkk5i` preserves all 60 frozen function identities,
source digests, spans and denominators relative to `run-o52m9whk`. It improves
**13 admitted / 40 excluded / seven inactive** to **18 admitted / 35 excluded /
seven inactive**, five gains and zero losses. Gains are Position::new, Span::new,
Span::splat, Span::is_empty and Span::is_one_line. The splat digest remains
`5e4fb92183b0c3b033fc8e3aaf9a41809e6c2af59f4902c7c11fb295b406afa0`;
the other four are the original development-target identities. The four-function
initial selection improves from zero to four admissions; splat exercises the
natural constructor-call chain. The three packages produce 48 independently
verified documents (14 lz4_flex, 18 regex-syntax, 16 time-core).

Streaming comparison of every regex-syntax record preserves all 956 names,
function spans and source digests. Its complete Cargo unit improves from zero
admissions to nine, with zero losses. The additional four admitted endpoints are
ClassBytesRange::start/end and Utf8Range::new/matches. These are observed additional
gains, not independently preselected family-completion claims. All other records
remain excluded. Capture and assessment are complete. Full-library matching was
intentionally not run (max-pairs 0); the resulting pair-enumeration/evaluation
incompleteness is not relabeled as complete.

The separate ten-function Position/Span development selection is nine admitted /
one excluded; all 36 admitted pairs evaluate completely. It tests nested constructor
results consumed by direct fields, Span::splat, and a shared-reference getter
caller in the presence of a sibling generic struct, enum, Drop impl and generated
function. The generated function remains retained and excluded. All four natural
endpoints strictly import and match their development counterparts with score 1;
all 28 pairs across that eight-function selection evaluate without diagnostics
or incompleteness. Exact offset/line/column read paths are checked independently.

The family is **not complete**. The source-selected composed function constructs
a local Span then invokes its shared getter, requiring an implicit borrow of the
owned local. It remains excluded by exact helper receiver/signature and location
checks. Existing shared-receiver calls do not prove this bridge. The next connected
work must handle the captured autoref adjustment and concrete local aggregate
read-summary substitution, preserving evaluation order, snapshots and aliases.
By-value self updates and cyclic loop/value merges remain separate open blockers.
Do not move on to checklist syntax merely because the four endpoints now pass.

An initial negative matching expectation was not met: changing the scalar getter
from offset to column preserves distinct verified field paths, but the existing
shape matcher still returns Accepted / score 1. This is a documented detector
limitation. It does not authorize equivalence or rewriting, and no scoring or
threshold was changed. Detector-quality work remains separate from extraction.

Independent runtime tests against the unmodified published regex-syntax 0.8.11
pass two oracles: distinct constructor components/full nested values, and all 64
binary combinations of offset/line/column getter inputs. Negative Cargo cases
reject missing Copy observations, active unknown macros, primitive aliases,
unresolved relevant globs, and an additional custom derive even when valid builtin
Copy evidence is present. Other tests check cfg-selected exact module closure,
orphan/missing files and grouped self/renamed/external imports.

The implementation adds no capture records, source buffers or serialized proof
flags. It reuses the existing parsed-unit environment and declaration handles.
The existing cache now distinguishes cfg contexts and retains failed environment
construction per immutable census and verification policy. An early full-unit
experiment exposed repeated deep retries on rejected functions and was stopped;
retaining that failure prevents the repeated parse. A regression test checks
strict/generated isolation without publishing function evidence on failure. No
whole-capture performance guarantee is inferred from these functional results.

The sequential workspace run passes 805 tests with five existing ignores, followed
by the focused source-closure/import tests. All-targets clippy with warnings denied,
formatting and diff checks pass. rstyle reports 2,606 functions and zero violations;
patterns reports zero candidates with 15 existing ignores unchanged. No source
selection, historical artifact, suppression, ignore, score or threshold was changed.
S09, D11/D12, D17 and F10 remain Partial and the overall roadmap remains active.

### Reported cross-platform regressions: expanded revalidation

The existing target-specific authority fix remains in place; no admission policy
was relaxed. The installed-CI-target regression now also checks saved and nested
subslice views, scalar endpoint rebinding, and the reported assertion/length
composition on Linux ARM64, Linux x86_64 and macOS ARM64. Inclusive subslicing is
explicitly expected to remain excluded. All 20 functions per target retain their
independently specified outcomes (16 admitted, 4 excluded), with produced documents
verified. The three originally reported tests also pass on the local host.
These are cross-target Cargo capture checks, not native execution on each OS.
The frozen selection and its historical counts are unchanged by this test-only
increment. rstyle reports 2,606 functions and zero violations; patterns reports
zero candidates, with existing ignores unchanged.
CLI-test clippy with warnings denied, formatting and diff checks also pass.


### Complete constructor-to-owned-getter composition (2026-10-20)

Data-v3 3.38.0 / 50 capabilities connects the existing Position/Span constructors
to a shared getter invoked on the constructed owned value. The
[bounded contract](docs/extraction-profiles/owned-copy-receiver-v1.md) was specified
before enabling the call. Existing receiver capture supplies the exact single
shared-borrow adjustment; the independent nominal environment supplies the Copy
and source proof. Callee reads are typed projections of the CALL's existing owned
input snapshot. Every occurrence is checked against the verified helper summary;
shared arguments retain formal-origin reads. No local memory origin, alias
independence, new capture inventory, source buffer or cache is invented. The
ordinary equal-signature path borrows its operand types without copying them.

The unchanged ten-function mixed-module development selection advances from
9 admitted / 1 excluded to 10 admitted / 0 excluded: `data::composed` is the gain,
with zero losses. All 45 admitted comparisons complete. Additional complete Cargo
cases admit owned parameters, constructor-result receivers, repeated getters
separated by rebinding, and calls mixing an owned receiver with a shared argument.
The independent runtime oracle compares the actual published natural constructor
and getter chain and the development composition against `x == y`, including
`usize::MAX`. The existing constructor and 64-combination getter oracles remain.
Missing/changed receiver adjustments and missing, altered or duplicated snapshot
reads are rejected. Owned escaping-reference calls and the existing non-Copy
implicit-receiver regression remain excluded.

Initial frozen Cargo replay `run-s9hi3ne7` retains all 60 source identities and
18 admitted / 35 excluded / 7 inactive; there are zero gains and zero losses.
The complete natural regex-syntax unit retains all 956 names, source digests and
spans, with nine admissions and no changes. This bridge completes the declared
development composition; it does not create a new natural-library admission.
Frozen capture/assessment is complete; `--max-pairs 0` deliberately does not
complete full-library matching. No detector-quality or rewrite claim follows.

Next connected natural targets remain `ast::Span::with_start` and `with_end`,
selected from source before admission: both consume a Copy Span and replace one
Position while preserving every component of the other endpoint. Their source
digests are respectively `2e112b065c7220cb98d037f3a3bda22061438f8f492a3de753391027118c645c`
and `fdf6e2c413bf3d5543398a97c79b9e86cd65379e3c86b043dd53984f5cad1c60`.
Current source and replay identify the connected blockers: signature receiver
validation assumes shared self, inherent definition ownership assumes shared self,
and method-call correspondence lacks the exact empty adjustment for owned self.
Struct-update value lowering exists, but its complete by-value receiver composition
has not passed independent verification. These remain excluded, not inferred
supported from the constructor fixtures. Mixed non-Copy nominal boundaries,
cyclic F10 merges and general Option/payload control flow remain open; broad
async/unsafe/FFI/macro work remains deferred.

Final frozen Cargo replay `run-th89oo5m` confirms the same complete capture and
assessment, all 60 identities/denominators/outcomes unchanged, and all 956 natural
function identities/outcomes unchanged. The final workspace run passes 808 tests
with five existing ignores. The added exact snapshot-path and adjustment
corruption test also passes separately. All-targets workspace clippy with warnings
denied, formatting and diff checks pass. rstyle reports 2,612 functions with zero
violations; patterns reports zero candidates with existing ignores unchanged.
No thresholds, scoring, suppressions, source selections or historical artifacts
were changed. The roadmap remains active.

A scoped performance diagnostic compares `a1d6c5d` with this implementation using
identical compiler, dependency lockfiles, raw 956-function input and four selected
natural constructor/getter records. Each run decodes the same raw input and
freshly captures those four functions under its own policy; both produce four
graphs. Across six alternating pairs, median elapsed time is 3.22 s baseline /
3.26 s current, and median peak RSS is 505,572 / 505,640 KiB. This finds no material
increase for that workload; it is not a whole-capture performance guarantee.
Earlier attempts importing retained policy-specific graph inventories were not
comparable and are discarded, without relaxing strict inventory verification or
changing the retained artifacts. No production benchmark cache was introduced.


### Checked Copy value-self updates (2026-10-20)

Data-v3 3.39.0 completes the selected seven-method Position/Span family with
`Span::with_start` and `Span::with_end` in the same default-feature natural build
context and exact source identities recorded above. The bounded
[value-self contract](docs/extraction-profiles/owned-copy-self-v1.md) requires
checked Copy ownership, immutable untyped self, exact callee signature and empty
method adjustment evidence. Existing source snapshots and receiver records are
reused; no new provenance table, cache or source copy is introduced.

Final frozen replay `run-o2jhsw07` yields **20 admitted / 33 excluded / 7 inactive**:
two gains and zero losses versus `run-th89oo5m`. The 60-function selection and
its denominator remain fixed. The 956-function natural unit has eleven admissions.
Capture and assessment complete; max-pairs zero does not claim full-library matching.

The separate `span-updates.rs` development selection was specified from source
before activation: seven positive cases and three negative receiver forms. Against
the same mixed-unit source, policy 3.38 admits zero of these ten and 3.39 admits
seven, with zero losses. Combined with the unchanged original ten cases, seventeen
functions admit and all 136 comparisons complete. Independent runtime expectations
check complete endpoint replacement, opposite-endpoint preservation, retained Copy
values and update-to-getter composition against published regex-syntax behavior.

A remaining composition gap is direct owned-formal field getters: `get(self) -> u8
{ self.n }` can leave its caller excluded because the current helper read contract
requires shared formal locations. This milestone does not claim all by-value
method bodies. Mutable/typed self, borrowed-to-owned adjustments, non-Copy owners,
cyclic F10 merges and general Option/payload control flow remain bounded or open.
The next audit should select natural functions blocked by those combinations.


The final verifier imports the complete seven-method natural/development family:
all fourteen functions strictly admit, all 91 comparisons complete with no
diagnostics, and each of the seven corresponding pairs is Accepted at score 1.
These scores are not equivalence or rewrite-safety claims. The four independent
runtime oracles pass. Cross-target Cargo capture tests pass on installed Linux
ARM64, Linux x86_64 and macOS ARM64 contexts, including exact negative receiver
expectations. Formatting, all-targets clippy with warnings denied and diff checks
pass; rstyle checks 2,618 functions with zero violations, and patterns has zero
candidates. No scoring, thresholds, suppressions or existing ignore entries change.

A final six-pair alternating performance diagnostic against 58addd6 decodes the
same raw 956-function input and freshly captures the same four constructor/getter
records in each version. Both produce four graphs. Median elapsed time is 3.255 s
baseline /3.230 s current; median peak RSS is 505742 /505724 KiB. No material
regression is observed for this bounded workload; this is not a whole-capture
performance guarantee. No production cache or instrumentation is added.

The final corrected workspace run passes 811 tests with 5 existing ignores.
The signature-binding regression proves an internally consistent borrowed-receiver
observation cannot substitute for the verified by-value callee signature.


### Next natural family: borrowed scalar owners

Source inspection selected `utf8::ScalarRange::is_valid` and `is_ascii` from
regex-syntax 0.8.11 before assessing their admission. The unchanged complete
natural replay contains both exact source identities: **0 admitted /2 excluded**.
The leaf fails the mixed-unit Copy requirement; its caller also lacks the leaf
summary. The [development contract](docs/extraction-profiles/borrowed-scalar-owner-v1.md)
records their contexts, full known blocker chain, remaining unknowns and positive
and negative expectations. No support or admission gain is claimed yet.

This family targets the shared S09 bottleneck without requiring general enums or
iterators from its neighboring module. A shared scalar getter does not consume
its owner or invoke Drop; that is not a proof that the owner is drop-free. The
implementation must distinguish borrowed-only permission from checked Copy
ownership before relaxing admission. Reusing existing source/nominal evidence is
preferred over another inventory, source copy or drop certificate. The frozen
selection stays fixed. F07/F13 table statuses now reflect the already implemented
3.36 checked-access subset; this documentation correction adds no capability.


### Connected borrowed scalar owners (Data-v3 3.40.0)

The selected `utf8::ScalarRange::is_valid` and `is_ascii` family now composes
through shared-field reads, the verified leaf summary and short-circuit caller.
A verification-local set distinguishes trees with independently verified Copy
origins from borrowed-only scalar owners; checked_owned_copy no longer infers
ownership permission from mere membership in the selected environment. There are
no new serialized observations, source copies, cache layers or drop certificates.
Existing source/module/import/attribute and helper evidence remains mandatory.
Owned occurrences of a borrowed-only owner anywhere in interface/body type tokens
are rejected. Destructor-bearing owners retain the existing frontend rejection.

Fresh unmodified default-feature natural capture preserves all 956 names, source
digests and spans: **11 to 13 admissions**, exactly the two selected getters and
zero losses. Capture/assessment complete; max-pairs zero intentionally leaves
full-library matching incomplete. The separate ten-function development corpus
has **0 to 4 admissions**, six retained exclusions and zero losses versus f2d0152
on identical source. All six development comparisons complete. Strict import of
the two natural and two development methods completes all six comparisons without
diagnostics; both corresponding pairs are Accepted at score 1. No detector quality,
equivalence, abstraction activation or rewrite-safety claim follows.

Independent runtime checks use a manually specified boundary table against the
actual published private methods and the development caller; this oracle plus
four existing natural UTF-8 tests pass. Regression checks retain repeated helper
reads and reject an omitted read. Focused Cargo tests pass, including
installed Linux ARM64, Linux x86_64 and macOS ARM64 capture contexts. These are
cross-target captures, not native macOS runtime execution.


Four existing regression expectations required the new borrowed-only distinction:
plain/manual-Copy shared getters, an inactive-Copy shared getter and a shared
getter beside a constant-local hidden Drop implementation now admit. Their
Copy-observation/forgery checks remain unchanged. An added owned construction in
the hidden-Drop fixture stays excluded, and a separate historical-profile test
requires all ten development functions to remain excluded under Data-v2. No
historical artifacts are relabeled or discarded.

Final frozen replay `run-yf3i09jl` preserves all 60 identities, denominators and
outcomes: **20 admitted /33 excluded /7 inactive**, zero gains/losses. Its 56
verified documents include all thirteen admitted natural regex-syntax functions.
The two new natural methods belong to the separate development selection and do
not alter the frozen denominator.


The final workspace run passes **815 tests with five existing ignores**. All-targets
clippy with warnings denied, formatting and diff checks pass. rstyle reports
2,622 functions and **zero violations**; patterns reports **zero candidates**.
No thresholds, scores, suppressions or existing ignore entries changed.

A final six-pair alternating diagnostic against f2d0152 uses identical dependency
lock entries/compiler, the same raw 956-function input and four unchanged natural
constructor/getter records. Each version freshly captures four graphs. After
capture and test workloads finished, median elapsed time is **3.135 s baseline /
3.125 s current** and median peak RSS **494672 /494270 KiB**. No material regression
is observed on this bounded workload; this is not a whole-capture guarantee.
The earlier concurrent-workload timing is superseded by this quiescent comparison.

Remaining combinations include cyclic F10 merge snapshots; Input's reference/enum
owner plus returned-reference helper chain; and ScalarRange's owned Option/tuple
construction, checked conversion and Result unwrapping. Borrowed scalar access
does not certify these combinations. Prioritize a tractable natural F10 family
in the next audit rather than broadening async/unsafe/FFI or general macros.


### F10 natural target audit: cyclic prefix scans

The next selected family is regex-automata 0.4.18
`util::wire::skip_initial_padding` and data-encoding 2.11.1 `skip_ignore`, in their
unmodified default-feature library Cargo contexts. Selection follows source
inspection before admission assessment. The
[development contract](docs/extraction-profiles/cyclic-prefix-scans-v1.md) records
exact source digests, build contexts, independent expectations, all known gates
and explicit implementation unknowns. The frozen 60-function identities and
denominator remain unchanged. This is a target declaration, not new support.

These functions compose current-iteration short-circuit merges with bounds,
slice/array reads, scalar updates, length summaries and a scalar table marker.
The source gate is only the first blocker: the serialized checker independently
rejects repeated controllers/snapshots and currently proves cumulative rather
than evaluation-local availability. Removing that rejection would not establish
fresh values. Any extension must verify the latest evaluation using finite
existing CFG/PDG evidence, including skipped nested conditions, while retaining
the no-merge and outside-loop paths. No new serialized epoch/history, source
inventory or runtime cache is justified by the selected family.

Original data-encoding Cargo capture/assessment completes all 127 functions; the
selected exact source is excluded by the F10 source gate. A separate six-function
development selection captures/assesses completely: **0 admitted /6 excluded**.
Both scans, alternating loop-local values and a saved short-circuit result are
the four intended positives, all currently blocked by F10. Mutation and
value-carrying break remain negatives with distinct existing restrictions.
Three independent runtime tests pass, covering all 256 translation bytes,
explicit table changes, empty/maximal start cases, capped prefix lengths and
1025 alternating/final-condition cases. Runtime correctness does not certify
pipeline admission. The separate library caller chains still require Result,
generic parsing/buffer mutation and unsafe/source boundaries; they are not
claimed supported by this bounded prefix-scan target.


The original regex-automata Cargo baseline now completes all 2,582 functions:
**7 admitted /2,575 excluded /0 unknown**, with all retained names/digests/spans
unchanged. Data-encoding completes 127 functions: **3 admitted /124 excluded /
0 unknown**. Both selected digests match the source-first declaration and both
scanners are excluded by F10, establishing **0/2 admissions** before activation.
The commands use max-pairs zero, so full-library matching is intentionally
incomplete while capture and assessment complete. No gain is claimed yet.
rstyle reports 2,622 functions with zero violations; patterns has zero candidates.
The three independent runtime-oracle tests pass. No production extraction rule,
threshold, scoring, suppression or historical artifact changes in this audit.


### Connected cyclic prefix scans (Data-v3 3.41.0)

Adds `rust.cyclic-value-merges` (53 capabilities). The
[bounded contract](docs/extraction-profiles/cyclic-prefix-scans-v1.md) connects
repeated short-circuit/conditional values with existing scalar updates,
length summaries, array/slice reads, bounds and exact scalar constants. The
selected natural `skip_initial_padding` and `skip_ignore` functions both complete
independent document verification, strict admission and comparison with their
source-declared development counterparts. All six selected natural/development
comparisons complete without diagnostics; both corresponding pairs are Accepted
at score 1. This does not activate abstraction mining or establish detector
quality, equivalence or rewrite safety.

The temporary finite proof derives dominance and reducible natural-loop regions
from serialized CFG edges. Each backedge clears outcomes and must-evaluated
availability in its loop region. Regions sharing a header are united across
continue paths; nested-loop resets preserve surrounding-iteration values until
the outer backedge. The verifier retains exact alternatives, guards, completion
frontiers, consumers and scalar reaching definitions. No new serialized epochs,
histories, source copies or persistent caches are introduced. No-merge graphs
retain their previous path, and nonrepeating sites retain their previous proof
without an additional nonrepetition scan or allocating empty reset lookups.

The separate fixed six-function development selection improves **0 to 4
admissions**, with both independent mutation/value-exit negatives still excluded.
All six comparisons complete. Original data-encoding capture preserves all 127
names/digests/spans: **3 to 4 admissions**, exactly `skip_ignore`, zero losses.
Full-library matching is intentionally incomplete with max-pairs zero; it is
separate from the complete selected comparisons above.

Frozen replay `run-rncfhp4y` preserves all 60 identities, denominators and outcomes
versus `run-yf3i09jl`: **20 admitted /33 excluded /7 inactive**, zero gains/losses.
Its 56 verified documents retain the prior natural results. No historical result
is relabeled. Data-v2 remains excluded in new cyclic composition regressions.

Independent finite-state tests compare reset-aware availability with saturated
execution-history states on small cyclic graphs and kill masks. Vertex-deletion
reachability independently specifies the dominance expectations for sibling,
nested, self-loop, acyclic and irreducible cases. Cargo regressions cover saved
preloop values, nested loops, continue/break and final-condition behavior, and
reject unavailable snapshots, altered guards/controllers/frontiers/successors,
missing index reads/bounds and swapped operands. The three independent runtime
oracle tests retain all-byte translation, bounded zero-prefix and 1025
alternating/final-condition expectations. Existing outside-loop and no-merge
regressions remain intact; two former Data-v3 cyclic exclusions now correctly
admit without changing their historical baseline records.

Remaining blocker combinations are the declared natural scanner callers:
returned-reference parsing, Result/error payload flow and unsafe boundaries in
regex-automata's deserializer; generics, mutable buffers and error propagation
in data-encoding's decoder. General iterator protocols, memory mutation,
value-carrying/labelled exits and broader async/concurrency/FFI remain outside
this bounded support. Select the next connected family from source and its full
dependency chain instead of treating F10 as complete Rust loop support.

Original regex-automata capture/assessment preserves all 2,582 names/digests/spans:
**7 to 8 admissions /2,574 exclusions /0 unknown**, exactly
`util::wire::skip_initial_padding`, zero losses. Together the two unchanged
whole-package selections gain exactly the two declared scanners across 2,709
functions. Both capture and assessment complete; max-pairs zero intentionally
leaves full-library matching incomplete. The original baseline artifacts and
source-first identities remain unchanged.

Final validation passes **824 workspace tests with five existing ignores**,
all-targets clippy with warnings denied, formatting and diff checks. rstyle
reports **2,632 functions /zero violations**; patterns reports **zero candidates**.
No thresholds, scoring, suppressions or existing ignore entries changed.

A quiescent diagnostic against d63e2bc uses identical compiler/dependency-lock
identities and unchanged raw inputs. Five alternating constructor/getter sample
pairs (after a warmup pair overlapping the short style scan) each freshly capture
four unchanged natural graphs: median **3.13 s baseline /3.16 s current**, peak
RSS **494688 /494240 KiB**. Six alternating loop sample pairs each freshly
capture three existing no-merge/outside-loop functions twenty times: median
**10.840 s /10.855 s**, peak RSS **18158 /18264 KiB**. No material regression is
observed on these bounded workloads; this is not a whole-capture or general
state-space performance guarantee. The new repeated-merge path performs the
additional finite proof required for the selected scanner admissions.


### Next connected target: scalar Option byte widths

After the two practical F10 scanner gains, source inspection selects
regex-automata 0.4.18 util::utf8::len and the nested regex-syntax 0.8.11
debug::utf8_decode::len. The
[development contract](docs/extraction-profiles/scalar-option-byte-widths-v1.md)
fixes source hashes/spans, ordinary default-feature contexts, independent all-byte
expectations, the complete known blocker chain and explicit unknowns before
admission assessment. These classify byte widths as Option<usize>; they do not
validate complete Unicode scalars. Their surrounding char/Result decoder chains
remain separate unsupported functions, with no supported-prefix certification.

Both original Cargo-captured identities exactly match the declaration and are
excluded: regex-automata in the current complete 3.41.0 capture; regex-syntax in
the retained complete 3.40.0 capture. A current-policy natural comparison is still
required before gains can be claimed. The fixed six-function development corpus
completes current capture/assessment at **0 admitted /6 excluded /0 unknown**.
Four intended positives connect range/conditional Option construction, explicit
early returns, saved values, scalar payload binding and helper consumption;
two negatives keep nested Option and a custom similarly named enum outside scope.

Source/code audit identifies semantic Option-scalar type rejection, precise
source generic-type correspondence, absent None path authority and Some
constructor semantics, shared-only payload binding with inappropriate memory-read
semantics for scalar values, then merge/early-return/local/helper transport and
historical policy gating. The first source rejection comes from nominal
lifetime-only type parsing; a precise core Option route is needed without
weakening general nominal ownership/source rules. Existing scalar range support
should be reused. Nested-owner and all-target constructor identity details remain
explicit validation unknowns. No extra inventory, provenance table, source cache
or broader nominal-unit relaxation is justified by this family.

Two runtime oracle tests pass for the independently specified 256-byte partition,
fallback values 0/5/usize::MAX, saved-value rebinding and explicit C0/C1/F5/F6/F7
non-validation cases. These are semantic expectations, not support activation.
The frozen inventory/denominator, historical records, production policy,
thresholds/scoring and suppressions remain unchanged by this declaration.

The declaration's style audit checks 2,632 functions with zero violations;
patterns has zero candidates. This increment changes only the target contract,
corpus and independent oracle; it does not change extraction or admission.


### Scalar Option constructor authority capture checkpoint

The existing driver expression walk now retains core Some/None constructor
identity using the existing resolved Option-variant record; no new inventory,
source copy, provenance table, cache or traversal is introduced. Some calls and
None paths were missing this resolved evidence in the declared width-classifier
chain. This observational increment supplies it without registering construction
semantics, scalar Option admission, a capability or a policy-version change.

Exact pinned hashes pass Cargo capture tests for all installed ARM Linux,
x86_64 Linux and Apple ARM64 targets in both std and no_std contexts, including
renamed variants. User enums and a local function named Some returning actual
core Option do not acquire constructor authority. These are cross-target capture
tests, not native macOS runtime tests. Data-v2 capture remains unchanged; all
eight authority-test functions remain excluded.

The unchanged six-function development selection retains **0 admitted /6
excluded**, zero gains/losses. The source/type, semantic construction, owned
payload binding, flow and helper gates remain required; no support claim follows
from capture alone. Full workspace validation passes **826 tests with five
existing ignores**. All-targets clippy with warnings denied, formatting and diff
checks pass. rstyle checks **2,633 functions /zero violations**; patterns reports
**zero candidates**, with no new suppressions or ignore entries.

A bounded quiescent diagnostic alternates the previous 576660d driver and the
current driver through identical real Cargo captures of the unchanged six-function
corpus, using the same compiler and current CLI. After one warmup pair, seven
retained pairs have median elapsed **0.299 s baseline /0.274 s current**, peak RSS
**103748 /104076 KiB**. All source identities/outcomes match in every sample.
No material regression is observed here; this small corpus is not a whole-package
performance guarantee or a speedup claim.

### Scalar Option source structure (unactivated checkpoint)

The source checker now checks an explicit bool/integer Option argument separately
from nominal lifetime arguments. It requires the exact payload spelling and an
independently resolved enclosing type; familiar names and qualified core/std paths
alone cannot certify it. Existing nominal lifetime and shared-payload rules remain
closed. This uses borrowed syntax nodes and the existing resolution interface,
with no source copies, serialized provenance or additional cache.

The unchanged six-function development selection retains all hashes, spans and
outcomes: **0 admitted /6 excluded**, complete capture/assessment, zero unknowns
and zero gains/losses. The classifiers now reach the missing scoped-path proof.
No end-to-end family support or new natural admission is claimed, and Data-v3
remains **3.41.0 /53 capabilities**. The contract in
`docs/extraction-profiles/scalar-option-byte-widths-v1.md` records the precise
boundary. The next dependency is exact core/prelude/import resolution in the
existing checked source environment, including the natural nested helper owner;
construction, owned payload projection and complete helper flow must then connect
before activation. Broadening unrelated syntax would not complete this chain.

Independent structural checks cover all 13 bool/integer payload spellings and
reject absent/wrong resolution, malformed arguments and unsupported payloads.
Workspace validation passes **829 tests**, with five existing skips, plus
doctests. All-targets clippy with warnings denied, fmt and diff checks pass.
rstyle checks **2,634 functions /zero violations**; patterns reports **zero
candidates**, with no new suppressions/ignores. No full natural recapture, frozen
replay or new performance measurement is claimed for this unactivated source
check; those remain required at connected semantic activation.

### Scalar Option lexical source boundary (activation remains closed)

The existing checked source environment now supplies the bounded core Option
path proof, without constructing a nominal ownership/Copy certificate. It checks
one exact original function, enclosing blocks for nested helpers, relevant
module/block bindings, direct core/std/prelude spellings and grouped/renamed
imports. It rejects scalar aliases, type/module shadowing, unresolved relevant
globs, active lexical macro invocations and unknown derives. An identically
spelled inactive nested function cannot replace the active source owner.

Both declared natural classifiers pass this source check on their complete
original captured source contexts, retaining the exact hashes/spans in
`docs/extraction-profiles/scalar-option-byte-widths-v1.md`. The audit found two
coarse dependencies: ancestor module globs do not supply child-module type
bindings, and regex-automata imports a closed source module with bare
`#[macro_use]`. The Option-only proof accounts for that name import while still
rejecting active invocations. Nominal ownership keeps its existing macro-import
rejection, tested with an otherwise Copy owner. No general macro execution,
neighboring-body purity or drop behavior is inferred.

Shared views reuse the existing owner/policy-local environment; standalone checks
use the same constructor. There is no additional cache, source copy or serialized
proof record. Unsupported semantic types are rejected before source lookup, so
this still-unactivated scalar Option route avoids work during ordinary capture.
The policy remains **3.41.0 /53 capabilities**. The six-function development
selection retains all identities and outcomes: **0 admitted /6 excluded**, complete
capture/assessment and zero unknowns. No end-to-end admission gain is claimed.
The next milestone must connect scalar types, Some/None construction, owned
payload projection, snapshots and helper transport for the complete selected
family; another independent syntax/evidence checkpoint would not complete it.

The workspace passes **831 tests**, with five existing skips, plus doctests.
Seven focused source/Copy regressions pass after the final owner-activity check
and shared standalone-constructor refactor, including rejection of overridden
core/std extern roots and hidden compiler arguments. All-targets clippy with warnings
denied, fmt and diff checks pass. rstyle checks **2,657 functions /zero
violations**; patterns reports **zero candidates**, with no new suppressions or
ignores. Frozen replay **run-wm2_93wr** matches **run-rncfhp4y** for all 60
identities/outcomes and every denominator: **20 admitted /33 excluded /7
inactive**, 56 verified documents, zero gains/losses.

A quiescent end-to-end capture diagnostic alternates the complete 8306481
CLI/driver pair with the current pair over the unchanged six-function corpus.
After one warmup pair, seven retained pairs have median **0.300 s baseline
/0.281 s current** and peak RSS **103544 /103920 KiB**. Every capture retains the
same hashes, spans and outcomes; no material regression is observed in this
bounded unsupported workload. This does not measure future activated source
resolution or establish a whole-package performance guarantee.

### Connected scalar Option byte-width family (Data-v3 3.42.0)

Adds `rust.scalar-option-values` (54 capabilities), completing the selected
[byte-width contract](docs/extraction-profiles/scalar-option-byte-widths-v1.md).
Owned bool/integer Option interfaces, direct pinned Some/None construction,
Option locals/parameters/results, exhaustive unguarded immutable payload binding,
condition/range-match merges, early returns and existing supported helper
transport now compose. Some retains the actual ordered operand and its effects;
None has no payload. Scalar payload extraction is a Some-guarded value operation,
without reference origins or content reads. Shared Option payloads retain the
previous guarded-load contract. No new serialized proof inventory, source copy,
persistent cache, threshold/scoring change, suppression or ignore was introduced.

The unchanged six-function development selection advances from **0 admitted /6
excluded** to **4 admitted /2 excluded**, four gains and zero losses. match_width,
branch_width, width_or and saved_width complete capture, independent verification,
strict admission and all six comparisons. nested/custom retain their exclusions.
Additional connected tests cover bool/i128, saved Option rebinding, helper-result
transport, operand effects and early return before construction. Unsupported
wildcard/guarded/mutable patterns, reference construction and explicit constructor
turbofish remain excluded. The potentially overflowing local helper named Some
keeps its actual identity and its existing nonpanicking-summary caller exclusion.

Both source-selected natural width classifiers pass strict bundle import,
independent document verification, producer verification and strict admission in
fresh original default-feature Cargo contexts. Each matches its corresponding
development classifier at score **1**, with all six comparisons among those four
functions complete. Independent original-body runtime checks cover all 256 bytes
for both natural classifiers; the development oracle also covers fallback values,
saved rebinding and deliberately non-validating byte classifications. These
results do not certify the enclosing decoders, abstraction activation, rewrite
safety or held-out detector quality.

Constructor core/variant identities and arity are checked by strict import;
independent graph checks reject changed kinds, semantics, types, operands and
fabricated effects. Historical 3.41 document-policy downgrade and Data-v2
construction remain closed. Constructor capture and independent document
verification pass in std/no_std for installed ARM Linux, x86 Linux and Apple
ARM64 targets; this is cross-target capture, not native macOS runtime execution.

The workspace passes **836 tests**, with five existing skips, plus doctests.
The final cross-target document-verification test also passes. All-targets
clippy with warnings denied, fmt and diff checks pass. rstyle checks **2,676
functions /zero violations**; patterns reports **zero candidates**.

Canonical frozen replay **run-lqdkmx_1** matches **run-wm2_93wr** for all 60
source identities, spans, hashes, outcomes and every denominator: **20 admitted
/33 excluded /7 inactive**, zero selected gains/losses. It verifies **58 documents**
across the complete captured package units; the additional natural classifier
is outside the frozen selection. The earlier concurrent replay **run-90z5qze9**
was terminated by the runner's memory limit and supplies no completion claim or
counts. Large-package validation is run sequentially afterward.

Fresh assessment against **6488a73 /3.41.0**, retaining every captured function
identity and semantic build context, measures:

| Original default-feature library | Captured functions | Baseline admitted/excluded | 3.42 admitted/excluded | Gains/losses |
| --- | ---: | ---: | ---: | ---: |
| regex-automata 0.4.18 | 2582 | 8 /2574 | 9 /2573 | 1 /0 |
| regex-syntax 0.8.11 | 956 | 13 /943 | 14 /942 | 1 /0 |

The only gains are the two declared classifiers. Their original hashes/spans
match the contract; all other captured outcomes remain unchanged, including the
outer decoders. Those callers still combine nested Option/Result and char values,
guarded/literal payload patterns, conversions, UTF-8/iterator summaries and, for
reverse decoding, additional slice/loop/helper compositions. Completing the
classifier does not complete those chains. General enum/trait, alias,
destructuring, mutable access and broader loop/call compositions remain roadmap
work; none is silently activated by this increment.

The large automata Cargo captures completed their exact unit bundles. Concurrent
full JSON report expansion exceeded the 16 GiB runner budget, so report generation
was stopped and the retained bundles were assessed sequentially with the matching
baseline/current libraries. This compact audit classifies **every one of the 2582
captured functions**, requires graph roots exactly for eligible functions, and
runs independent document verification, producer verification and strict admission
for each root. Unknown and excluded classifications remain distinct; missing
eligible roots are failures. This is a complete unit-roster comparison, not a
completed bulk CLI JSON report or a smaller replacement selection. The syntax
comparison uses complete fresh CLI reports. The original Cargo source census,
compiler invocation and all functions remain in the retained unit evidence.

A quiescent real-Cargo diagnostic alternates the full baseline/current CLI-driver
pairs, with one warmup pair and seven retained pairs per workload. The unchanged
six-function cyclic-prefix control retains all hashes, spans and outcomes:
median elapsed **0.61 /0.63 s** (ranges **0.60–0.62 /0.60–0.64 s**), median per-run
peak RSS **111636 /111596 KiB**. No material regression is observed on this bounded
unchanged workload. The unchanged Option selection takes **0.28 /0.40 s**, with
median per-run peak RSS **106304 /109124 KiB**; the current run additionally
verifies/admits four functions and evaluates their six pairs, so it performs more
work. These measurements are not a whole-library performance guarantee or a
speedup claim. No cache or serialized provenance was added to change the result.

### Connected checksum-state roundtrips (Data-v3 3.43.0)

The [source-selected contract](docs/extraction-profiles/checksum-state-roundtrips-v1.md)
fixes adler2 2.0.1's original constructor/getter source identities, std/no_std
contexts and independent expectations before assessment. The complete blocker
chain was exact builtin inline ownership → instantiated u16-to-u32 core From
identity/summary → existing field reads → owned helper-result receiver snapshots.
Existing precise mixed-unit Copy/module/import/derive proof composes without a
coarse-unit redesign. New selected-implementation capture is bounded to this
numeric signature and reuses the existing optional record; no additional
provenance table, source copy, cache or production dependency is added.

The unchanged development census has **11 functions, 1→5 admitted**, six retained
exclusions and ten complete comparisons. Original adler2 std has **17 functions,
0→2 admitted**; no_std has **16 functions, 0→2 admitted**. All names, spans,
source digests and build contexts match the baseline. Both selected original
methods gain admission in each context, with zero losses or missing functions.
Strict bundle import, independent documents, producer verification and admission
succeed. Each natural constructor/getter matches its development counterpart at
score 1; all six comparisons complete separately per context.

The numeric oracle checks the original public methods and complete development
compositions on 262144 whole-word inputs and every u16 widening, in both std and
no_std dependency builds. CI-target tests cover ARM Linux, x86 Linux and ARM
macOS capture in both modes, exact inline forms, qualified primitive calls,
partial operand returns and saved inputs. Custom, float, bool and alias
conversions remain excluded; identity/signature/operand/document-policy
corruptions fail closed. The full source census retains neighboring Default,
Hasher, mutable/SIMD helpers and generated derives.

Remaining combinations: Default/new require trait-call/definition support;
Hasher::finish additionally needs u32-to-u64 From; mutable checksum APIs add
mutation and loop/SIMD contracts. No checksum-update, general trait, abstraction
activation, rewrite-safety or detector-quality claim follows.

Canonical replay **`run-dfcyj99b`** preserves all 60 frozen identities, outcomes,
context counts and denominators versus `run-lqdkmx_1`: **20 admitted /33 excluded
/7 inactive**. Complete captured rosters also retain all identities/build
contexts: lz4_flex **150, 7→7 admitted**; time-core **226, 8→8**; regex-syntax
**956, 14→19**, with zero losses. The five incidental regex-syntax gains are
`hir::LookSet::{empty,full,subtract,union,intersect}`, formerly blocked by builtin
inline ownership. These were not source-selected for this milestone and do not
establish a complete LookSet-family claim. The canonical replay verifies **68
documents** across full units; that count does not replace the frozen denominator.

Quiescent alternating baseline/current measurements (one warmup pair plus seven
retained pairs) show no material regression on the unchanged cyclic-prefix
control: **0.66 /0.63 s**, ranges **0.61–0.66 /0.61–0.65 s**, median peak RSS
**111548 /111796 KiB**. The newly activated unchanged checksum corpus takes
**0.36 /0.41 s**, **118552 /119964 KiB**, while additionally verifying/admitting
four functions and evaluating ten pairs. This is additional supported work;
it is not an equal-work speedup/regression claim. See the contract for details.

Final validation: **846 workspace tests pass**, five existing skips; doctests,
all-target clippy with warnings denied, fmt and diff checks pass. rstyle reports
**2686 functions /0 violations** and patterns **0 candidates**. Fresh final
captures repeat the complete selections and strict matching results. No new
suppression, ignore entry or scoring/threshold change is used.

### CI regression follow-up: pinned subslice targets

The reported subslice failures used producer revision `4e18530`; the current
branch already carries exact compiler/core-definition registries for ARM Linux,
x86 Linux and ARM macOS (whose captured LLVM target is `arm64-apple-macosx`).
A dedicated real-Cargo cross-target regression now checks exclusive subslicing,
saved origins, base evaluation before endpoint rebinding, and assertion receiver
composition on all three installed targets. It verifies generated documents and
strict admission, matches equivalent windows, and rejects altered index/range
authorities, missing bounds, changed operands and fabricated reads. Inclusive
ranges and mutable subslices remain excluded. This adds regression coverage;
it changes neither supported contracts nor the frozen selection/denominators.

Validation: the three reported tests and the three-target regression pass;
workspace all-target clippy denies warnings; fmt is clean; rstyle reports zero
violations and patterns reports zero candidates. Cross-target checks compile and
verify evidence on Linux; they do not execute native macOS binaries.


### Next connected target: native unsigned prefix readers (not activated)

Selected from original source before admission assessment: regex-automata
0.4.18's `util::wire::{read_u16,read_u32,read_u128}` in its complete default-feature
Cargo library context. The [contract](docs/extraction-profiles/native-prefix-readers-v1.md)
fixes all three original spans/hashes and a separate unchanged 11-function
selection, including independent positives/negatives and std/no_std contexts.

The full blocker chain is RangeTo prefix representation/bounds and saved base →
exact core/std display authority → original size_of generic-call evidence →
2/4/16-byte copied Result/unwrap contracts → native u16/u32/u128 decodes. The
four-byte chain already exists; two/sixteen-byte type/signature observations are
already captured. Do not add redundant type tables. The unresolved size_of call
is hidden by prefix rejection, so the first eligibility reason is insufficient.
The natural crate's core paths add real failures absent from the std fixture;
the separate no_std module reproduces them without include/path rewriting.

Both development contexts capture/assess all 11 functions at **1 admitted /10
excluded /0 unknown**, with complete empty-pair enumeration/evaluation. Current
3.43 classification rejects the three original readers in the retained complete
3.42 Cargo unit, preserving their identities and all 2582 captured functions. This audit
is **not** a new complete 3.43 natural CLI report or whole-unit admission count;
fresh original capture and roster comparison remain required before activation.

Four independent runtime tests pass against exact original bodies and complete
development compositions: every two-byte word, 32/128-bit positions, short-input
panics, extra trailing bytes, helper choices and saved/base-rebinding behavior.
This source/semantic audit adds no production code, provenance table, cache or
admissions. Data-v3 remains 3.43.0 and the frozen 60-function inventory is unchanged.
The next implementation must connect this complete chain rather than count
isolated prefix, intrinsic or conversion fixtures as family support.

Validation of this audit: both development Cargo reports and strict choose
imports pass, all four runtime oracle tests pass, Python syntax/fmt/diff checks
pass, rstyle remains **2686 functions /0 violations**, patterns **0 candidates**.
No extraction engine, threshold, suppression or ignore-ledger change is made.

### Reported CI regressions: current-tree revalidation

Revalidated the three reported assertion/subslice failures while preserving the
in-progress native-prefix-reader implementation. All three pass, as does the
combined real-Cargo regression across installed ARM Linux, x86 Linux, and ARM
macOS targets. These cross-target captures verify exact pinned compiler/core
identities, strict documents/admission, saved base origins, and equivalent
matching; this is not a native macOS execution claim. The target-authority fixes
and dedicated regression are already published on this feature branch.

Current-tree validation: **850 workspace tests passed /5 existing skips**;
workspace doctests, all-target clippy with warnings denied, formatting and diff
checks passed; rstyle **2703 functions /0 violations**, patterns **0 candidates**.
All four independent prefix-reader runtime oracle tests also pass. These results
include uncommitted prefix-reader work; they do not activate its prospective
coverage milestone or replace the pending natural/frozen roster and performance
comparisons. Existing frozen identities and historical outcomes are unchanged
by this validation record.


### Data-v3 3.44.0: complete native unsigned prefix readers

The source-declared regex-automata 0.4.18 family now supports all three original
`util::wire::read_u16/read_u32/read_u128` functions end to end, in the unchanged
complete default-feature Cargo library context. Shared prefix bounds and saved
base evaluation, actual `size_of` instantiation, pinned core/std display paths,
concrete copied Results/unwrap and unsigned native conversions compose through
independent documents/producer verification, strict admission and matching.
The existing captured types, source census and origin/location machinery are
reused; only the actual scalar-size argument is new capture evidence. No general
provenance table, source copy/cache or purity/alias assumption is added.

The complete original **2582-function** roster moves **9 →14 admitted**, with
**five gains /no losses /no unknowns**, identical source identities and build
context. The three target readers account for three gains; the other two are
complete scalar-size wrappers `write_endianness_check_len` and `write_version_len`.
All 14 admissions independently verify after importing the full original unit.
The baseline's complete saved report was validated before its redundant JSON
stdout rendering was interrupted; that exit is recorded as interrupted. The
current complete report exits only for the explicit zero-pair inventory budget.
Neither whole-library inventory claims complete matching.

Each unchanged 11-function std/no_std development context moves **1 →8 admitted**
(**seven gains /no losses**), retaining the three declared negatives and fully
evaluating all 28 comparisons. The nine imported natural/development reader
graphs complete all 36 comparisons; all six same-width counterparts accept with
score 1. Cross-width structural comparisons also accept, so this is not a
semantic-equivalence, rewrite-safety or detector-quality claim.

Canonical frozen replay **run-sd5dhs2u** equals run-dfcyj99b on all 60 identities,
outcomes, contexts, counts and denominators: **20 admitted /33 excluded /7
inactive**, with 70 full-unit documents schema-validated. Whole rosters preserve
lz4_flex **150 /7 admitted** and time-core **226 /8**; regex-syntax **956 /19 →20**
gains the independently strictly verified `hir::LookSet::read_repr`, composing
prefix decoding with its original struct construction. No compared roster loses
an admission. These incidental gains do not redefine the declared target.

All **852 workspace tests** pass (five existing skips), as do the final focused
cross-target regression suite, independent four-test runtime oracle, doctests,
all-target clippy, fmt and diff checks. Rstyle: **2703 functions /0 violations**;
patterns: **0 candidates**. No thresholds, scoring, suppression or ignore entries
change. Cross-target capture verifies the three pinned CI targets from Linux;
it is not native macOS/x86 execution.

One warmup plus seven alternating quiescent comparisons against static 68c300c
find unchanged-control medians **0.86 /0.89 seconds** for copied windows and
**0.63 /0.63** for cyclic scans, with overlapping ranges and stable peak memory.
The measured copied-control +0.03-second shift is retained explicitly; no broader
performance guarantee is claimed. All retained unchanged-control matching reports
are identical. Activated corpus runtime **0.34 /0.67 seconds** includes seven
additional admissions and 28 additional comparisons. Full results and scope are
in the [contract](docs/extraction-profiles/native-prefix-readers-v1.md).

The next target still requires a source-first full blocker audit. Do not call
`try_read_*` supported: its chain combines partial-flow helpers, generic length
checks, concrete Result/tuple/error values and `?`. Suffix/inclusive/array/mutable
views, signed conversion, richer Option/Result payload patterns and broader
helper-result/effect transport remain unfinished. General async/concurrency,
unsafe/FFI and macros remain deferred under the reprioritized goal.

### Checked native reader dependency audit (2026-10-20)

The [source-first dependency audit](docs/extraction-profiles/checked-native-readers-audit.md)
selects original `try_read_u16/u32/u128` as the next candidate family, without
claiming activation. The full chain includes instantiated generic length checks,
nested nominal error/enum and drop dependencies, concrete Result tuples, residual
early exits, and partial native-reader helper effects. Existing extracted leaf
readers do not pass the nonpanicking helper boundary. General `?` alone cannot
yield admission. Inspect the existing complete unit facts and error dependency
closure before selecting the first implementation contract. No inventories,
capabilities, thresholds, capture metadata or measured outcomes change here.

The captured-fact follow-up confirms an additional concrete gap: Try-desugared
`expr_match` nodes currently have no captured children. Original checker calls
and residual operations cannot be recovered by adding Result types alone.
The complete-original-crate checked-reader runtime oracle now passes three tests
(all u16 words, wide bits, short/tail inputs and exact error-description payloads).
This establishes independent expectations while admissions remain 0/3; no
production capture, memory or runtime behavior changes in this increment.

### Retain Try operands and compiler continuations (2026-10-20)

Data-v3 now observes `MatchSource::TryDesugar` as a distinct unsupported
`expr_try_desugar`, retaining its scrutinee and both arms in the existing HIR
tree. This closes the missing-operand capture link identified by the checked
native-reader audit. It does not enable `?`, concrete Result, enum errors,
generic helpers or partial-helper admission. Historical profiles retain the
previous capture shape; no new source exception or capability is registered.

Two real-Cargo tests verify Result and nested Option captures, exact operand
occurrence and both continuations, continued exclusion, producer rejection,
and the Data-v2 historical boundary. The two selected test functions remain
excluded (zero gains/losses); no new natural or frozen inventory replay is
claimed. The original checked family remains unactivated. All 854 workspace
tests pass, with five existing skips; rstyle has zero violations and patterns
zero candidates. Capture uses existing nodes with no new provenance table,
cache, source-copy structure or summary format. Unsupported Try functions
now deliberately retain more HIR; no broad performance claim follows from the
test timings. Next work must connect type/error and helper semantics rather
than accumulate more observational syntax increments without natural gains.

### Reassessment: scope restrictions before broader error enums

The complete DeserializeError closure includes macro-generated PatternIDError
and StateIDError declarations. Reassessing after the foundational Try capture,
select original Epsilons::empty/is_empty as a practical S09 constructor/getter
family before expanding general error and generic semantics. The
[source-first scope contract](docs/extraction-profiles/scoped-import-flag-owners-v1.md)
records identities, baseline exclusions, bounded import proof and independent
expectations. The checked readers remain deferred within the active roadmap.

### Data-v3 3.45: connect scalar flag owners across closed imports

The [scope contract](docs/extraction-profiles/scoped-import-flag-owners-v1.md)
activates the original complete-crate Epsilons constructor/getter selection.
Closed local glob and macro-definition-only imports are inspected using the
existing selected census; unknown imports, relevant shadowing, active macros,
unproved Copy and drop behavior remain exclusions. Checked Copy scalar field
reads now compose through by-value helpers using existing value snapshots.
Independent verification enforces both permissions at document policy and
declaration scopes. No new provenance inventory, source-copy format or cache
is added; thresholds and scoring are unchanged.

On the unchanged 2582-function regex-automata library, admissions rise from
14 to 59: **45 gained, zero lost**, including both preselected methods.
The separate six-function development corpus rises from zero to five admissions;
its generated Clone remains excluded. All 59 natural and five development
admissions pass final independent verification and strict admission. All 21
comparisons among the selected seven graphs complete (two Accepted, 19 Rejected).
Whole-library matching is incomplete under the explicit capture pair budget.
The original complete-crate runtime oracle passes independently specified
zero/high/all-bit and constructor/getter expectations.

Validation: 861 workspace tests pass with five existing skips, all-target
Clippy with warnings denied, doctests, formatting and whitespace checks.
Rstyle has zero violations across 2714 functions; patterns has zero candidates.
Seven alternating benchmark pairs preserve every unchanged-control matching
report. Median byte-window runtime .88→.87s; cyclic .60→.62s (+3.3%,
overlapping ranges). Activated flags .26→.30s while adding five admissions
and ten comparisons. See the contract for memory/range limits; this is no
broad performance guarantee.

Remaining combinations: checked native readers still need concrete Result
payload/error closure, generic length checking, partial helper transport and
residual semantics; two error declarations are macro-generated. General macro
activation is not implied by inspecting unused local definitions. F10 loop/value
merge and checked-access composition remain active roadmap work. Choose the next
complete source-selected family by blocker-chain evidence before another syntax
increment; preserve the fixed frozen inventory and separate rewrite/detector
claims.

Frozen replay `run-_i41wnhy` preserves all 60 selected identities and denominators:
**20 admitted /33 excluded /7 inactive**, zero gains/losses. Context counts,
source blocker combinations and outcomes are unchanged. Full rosters also retain
150/7 admitted for lz4_flex, 956/20 for regex-syntax and 226/8 for time-core,
with identical source identities and no outcome changes. Seventy documents
verify. Build evidence differences are producer revision and invocation paths
after normalization of temporary source roots; compiler/settings stay fixed.

### Next source-selected family: nested-array byte membership

After the 45-gain scalar-owner milestone, select original regex-automata
`ByteSet::empty` and `ByteSet::contains` plus development constructor/getter
composition. The [byte-membership contract](docs/extraction-profiles/byte-membership-v1.md)
records exact source identities, independent all-byte/singleton/dense expectations,
fixed-array Copy proof, concrete u8-to-usize From, nested indexed-read composition,
range/partial-helper and dynamic snapshot blockers. Both selected functions
remain excluded at 3.45; this audit does not activate them or replace the frozen
inventory. Mutable updates, iterator/closure and Result serialization stay
outside this bounded family.

### Data-v3 3.46: connect nested-array byte membership

The [source-selected contract](docs/extraction-profiles/byte-membership-v1.md)
connects independently checked Copy scalar arrays, exact u8-to-usize core From,
unsigned division/remainder bounds, index/shift totality and existing owner-value
helper snapshots. It adds three capabilities (61 total), with unchanged matching
policy and thresholds. Bounds use the existing graph and reaching definitions;
missing evidence retains the full type range. No serialized range inventory,
source copy, cache or helper-summary format was added.

All five intended development functions now compose through real Cargo capture,
independent document/producer verification, strict admission and complete matching
on Linux ARM64, Linux x86_64 and Apple ARM64 capture contexts. The fifteen-function
corpus moves from four to ten admissions, zero losses: five complete intended
positives plus one explicitly partial incidental getter gain. Five partial
leaves and five exclusions retain their declared outcomes. Unknown indices,
rebinding, wider merges, narrower shifts and possibly-zero divisors do not
acquire nonpanicking helper permission. Dynamic shared indexing remains deferred.
The complete original runtime oracle's three tests pass, including every
singleton/query combination. The unchanged original 2582-function library gains seven admissions, zero losses
(59 to 66), including both selected methods. All 66 natural and five development
admissions pass independent strict unit import and document/producer verification.
The selected seven graphs have complete CFG evidence and complete all 21
comparisons (two Accepted, 19 Rejected). Whole-library matching remains incomplete
under the explicit zero-pair budget.

Final frozen replay `run-gl3pfp6d` preserves all 60 selected identities and
denominators: **20 admitted /33 excluded /7 inactive**, zero gains/losses.
Context counts and source blocker combinations are unchanged. Full package
rosters retain 150/7 admitted for lz4_flex, 956/20 for regex-syntax and
226/8 for time-core, with identical source identities and compilation contexts
(after temporary-root normalization), and no outcome changes. All 70 documents
verify.

All 867 workspace tests pass with five existing skips. Clippy with warnings
denied, doctests, formatting and whitespace checks pass; rstyle has zero
violations across 2733 functions and patterns zero candidates. Seven alternating
benchmark pairs preserve all fourteen unchanged-control matching reports.
Byte-window median .89/.89s; cyclic .63/.65s (+.02s, overlapping ranges).
Activated membership .51/.68s while adding six admissions and 39 comparisons.
The contract records RSS/ranges and limits; no broad performance guarantee or
rewrite/detector-quality claim follows.

### Checked-reader reassessment after 3.46

The [dependency audit](docs/extraction-profiles/checked-native-readers-audit.md#post-346-error-dependency-closure)
now checks the fresh original 3.46 roster: all three checked native readers,
their generic checker and error constructor preserve their source identities and
remain excluded. Error ownership closes through a ten-variant enum to two local
macro-generated ID errors and `SmallIndexError { attempted: u64 }`. These types
are not covered by the existing positive builtin-Copy ownership contract.

The complete-original-crate oracle adds an independent drop expectation for all
five error declarations and the wide reader Result. All four runtime tests pass
in ARM Linux default features; `needs_drop` is false. This is semantic evidence,
not a new source/declaration certificate or permission to ignore sibling variants.
Result/`?` alone would leave owned non-Copy and macro declaration proof, generic
instantiation and partial-helper transport unresolved. Reassess smaller natural
Option/read compositions before choosing the next production increment. No new
capability or admission is claimed; frozen selections remain unchanged.

### Next connected target: tagged byte-unit construction and projections

The [tagged-unit contract](docs/extraction-profiles/tagged-units-v1.md) selects
original regex-automata Unit::u8/as_u8/as_eoi before activation. Its two-variant
scalar payload enum has positive builtin-Copy derives, avoiding the checked
readers' non-Copy/macro error chain. The full dependency chain still includes
complete nominal variant capture/proof, resolved variant construction, currently
missing tuple-pattern children and binding declarations, conditional payload
matching, by-value helper snapshots and scalar Option composition. Existing
construct support does not certify this family. No ABI layout is inferred.

The independent complete-original-crate oracle passes three tests: all 256 byte
values, all 65536 u16 sentinel payloads, and valid/invalid public eoi arguments.
The fixed development corpus declares six complete intended positives, three
named negatives and two excluded generated Clone methods. Real Cargo baseline
assesses all eleven at 0 admitted /11 excluded, with complete capture/assessment
and empty matching. The original three selected methods remain excluded in the
fresh 3.46 roster. This is a source/semantic target boundary, not a new capability
or admission gain. General Result/Try, iterator/closure and macro activation
remain separate remaining work.

### Tagged-unit prerequisite: retain tuple-variant pattern payloads

Data-v3 capture now preserves exact local enum tuple-constructor identity and
ordered payload patterns in the existing HIR tree. The selected getters retain
their scalar binding declarations and matching body-read HirIds; wildcard arms
remain explicit. This repairs the selected family's missing pattern-child link
without adding an enum type token, capability, source gate exception, serialized
table/cache or admission permission. The fixed eleven-function development
selection remains entirely excluded, and historical profiles retain their
previous unsupported pattern shape.

Two real-Cargo regressions check identities, payload arity/bindings, historical
shape, reference/rest/tuple-struct boundaries, unchanged exclusions and producer
rejection. The remaining chain is complete enum nominal capture/proof, variant
construction and conditional payload matching through helper snapshots and
scalar Options. This foundational increment yields no claimed natural gain;
the next increment must connect that chain rather than add another isolated
observation. The full roadmap remains active.

All 869 workspace tests pass with five existing skips; all-target Clippy with
warnings denied, formatting and whitespace checks pass. Rstyle reports 2735
functions /zero violations and patterns zero candidates. Seven alternating
quiescent comparisons against 347ddce retain identical source identities,
outcomes and matching reports: byte-window median .89/.88s, cyclic .64/.65s
(overlapping ranges), excluded enum corpus .30/.29s. The contract records RSS,
ranges and limitations. No whole-library capture or new frozen replay is claimed
for this observational prerequisite.

### Data-v3 3.47: connect tagged byte-unit construction and projections

The [tagged-unit contract](docs/extraction-profiles/tagged-units-v1.md) connects
complete finite scalar enum declarations, positive Copy proof, exact variant
construction and exhaustive conditional payload matching through scalar Options
and by-value helper snapshots. It reuses existing declaration origins, lexical
source environments and operation evidence. Shared helpers cover aggregate
construction, conditional payload initialization and exact operation inventories;
no new serialized provenance table or cache is introduced. A bounded plain-local
Copy discard closes the saved-result composition without inventing a value read,
move, binding or destructor effect. General discarded expressions remain excluded.

The unchanged eleven-function development corpus gains all six independently
selected positives; its three named negatives and two generated Clone methods
remain excluded. Capture, independent serialized verification and matching
complete. Data-v2 retains its exclusions and historical pattern shape. Five Cargo
regressions include 24 serialized-operation mutations, a missing successful
payload guard, non-Copy/Drop, reference/generic payloads, guarded matches and
unsupported discards. The original runtime oracle passes all three exhaustive
tests, including all 256 bytes and all 65536 sentinel payloads.

The fresh complete-original default-feature regex-automata roster retains all
2582 exact identities/build contexts. Admissions increase **66 to 69**, with
precisely original **Unit::u8/as_u8/as_eoi** gaining: **three gains, zero losses**.
Capture and assessment complete with no unknown/failed outcomes; full enumeration
is explicitly budgeted out. Independent verification passes for all 69 natural
admissions and six development admissions. All 36 comparisons among the selected
nine functions complete with unchanged matching defaults and no diagnostics.

Frozen replay retains all 60 identities/denominators and **20 admitted /33 excluded
/7 inactive**, with 70 verified documents. Full package identities, contexts and
outcomes remain unchanged: lz4_flex 150 functions /7 admitted, regex-syntax 956 /20,
time-core 226 /8. Historical reports remain untouched.

Against isolated 209fbf4, one warmup and seven alternating quiescent pairs per
workload retain identical unchanged-control admissions and matching. Byte-window
median baseline/current .87/.89s (ranges .86–.88/.86–.91); cyclic .62/.62s
(ranges .60–.65/.61–.64). Enum work increases .29 to .47s while gaining six admissions and
completed matching. The contract records RSS and limits; these bounded samples
do not establish a broad performance guarantee. All 876 workspace nextest tests
pass with five existing skips; workspace doctests, all-target Clippy, formatting
and whitespace checks pass. Rstyle reports 2785 functions /zero violations;
patterns reports zero candidates. No thresholds, suppressions or ignore entries
change.

Remaining nearby chains are original Unit::as_usize (exact u16-to-usize From),
is_eoi (concrete Option::is_some), eoi (assertion plus checked usize-to-u16
conversion/Result/unwrap) and the Option callback/combinator callers. Their
simultaneous blockers require source-first contracts before another activation.
The initial three-method family is supported; the wider Unit/DFA caller chain is
not certified. Abstraction mining, rewrite safety and held-out detector quality
remain separate from extraction coverage. The full roadmap remains active.

### Next connected target: tagged byte-unit classifiers

The [classifier contract](docs/extraction-profiles/tagged-unit-classifiers-v1.md)
selects original Unit::as_usize and Unit::is_eoi from the complete default-feature
regex-automata library before enabling support. It records both exact identities,
independent all-byte/all-u16 expectations, simultaneous conversion authority and
Option tag-test/receiver/helper blockers, and deferred neighboring chains.
Existing receiver observations suffice; no new provenance table is planned.

A separate fixed extension preserves the old eleven-function corpus and adds six
intended composed positives, four named negatives, the counterfeit method and
its generated Clone. The real Cargo baseline regression passes: 23 functions,
six admitted and seventeen excluded, complete assessment and matching, with
independent verification of existing admissions. No production capability or
natural admission gain is claimed by this source-first audit. The next increment
must connect the complete classifier family and measure against the unchanged
3.47 natural roster (69/2582) and frozen inventory.


### Data-v3 3.48: connect tagged-unit classifiers and byte-class lookup

The [classifier contract](docs/extraction-profiles/tagged-unit-classifiers-v1.md)
connects the predeclared original Unit::as_usize and Unit::is_eoi through exact
u16-to-usize selected-implementation authority and the concrete scalar Option tag
test. Existing library summaries, receiver adjustments, source proof and independent
document checking carry both operations. No new provenance table, source census or
cache is introduced. Positive Copy and complete nominal/module proof remain required.

All six new development positives compose through constructors, enum projections
and helper results: twelve admissions among the unchanged 23-function combined development corpus,
with all eleven expected exclusions retained. Three Cargo regressions include
all three installed pinned targets and six summary/operand/policy mutations.
The five complete-original-source oracle tests pass, including every byte and
all 65536 sentinel payloads. The original eleven-function development corpus remains unchanged.

The unchanged complete-library selection retains all 2582 identities and build
context. Admissions increase **69 to 72**, with **three gains, zero losses**:
Unit::as_usize, Unit::is_eoi and ByteClasses::get_by_unit. The additional lookup
was discovered by the full delta, then inspected and independently tested: it
composes enum payload matching, the existing getter/array read and the new scalar
conversion. It was not retroactively added to the predeclared selection. Capture
and assessment complete with no unknown/failed outcomes. Independent verification
passes for all 72 natural and twelve development admissions. All 153 comparisons
among six original and twelve development functions complete with unchanged
matching defaults and no diagnostics. Full natural enumeration is explicitly
budgeted out; historical reports are preserved.

All 880 workspace nextest tests pass with five existing skips; Clippy with warnings
denied, doctests, formatting and whitespace pass. Rstyle reports 2786 functions
and zero violations; patterns reports zero candidates. Against exact a236541,
seven alternating quiescent pairs after warmup retain identical unchanged-control
matching: byte-window median .87/.87s and cyclic .64/.64s. The extension rises
.73 to .84s while gaining six admissions and completed comparisons. The contract
records ranges/RSS and the limits of these bounded measurements.

Remaining connected chains are Unit::eoi (source/HIR observations, formatted
assertion and TryFrom/Result/unwrap), is_byte/is_word_byte (Option map_or and
closure/callback identity/body/captures), and ByteClasses::eoi (checked_sub/Option
unwrap plus Unit::eoi). Their exclusions remain intact. The wider roadmap,
abstraction activation, rewrite safety and held-out detector quality remain open.

Frozen replay preserves all 60 identities/denominators and **20 admitted /33
excluded /7 inactive**, with 70 verified documents. Complete package source
identities, build contexts and outcomes match 3.47: lz4_flex 150 functions /7
admitted, regex-syntax 956 /20 and time-core 226 /8. Historical reports and the
frozen selection remain unchanged.

### Next connected target: tagged byte comparisons

The [byte-comparison contract](docs/extraction-profiles/tagged-byte-comparisons-v1.md)
selects original Unit::is_byte from its complete default-feature Cargo library,
with exact source identity and independent all-byte/query expectations. Its full
chain combines Option::map_or authority, separately owned closure body/parameter
capture, scalar shared-capture/drop proof, eager default evaluation, conditional
payload reads and helper/result composition. Existing Unit/Option support is
required throughout; no general closure capability or new provenance table is
activated by this audit.

A fixed separate extension preserves the original eleven-function corpus and
specifies seven new composed positives, six named negatives, the counterfeit
method and its generated Clone. The real Cargo 3.48 baseline passes: all 26
functions assessed, six admitted and twenty excluded, complete capture and
matching, with independent checking of existing admissions. The complete-original
oracle now passes six tests, including all byte/query pairs and all u16 sentinel
payloads at four query values. No new natural admission gain is claimed; the
unchanged natural baseline remains 72/2582 and the frozen selection remains 60.
The next implementation must close the complete bounded callback chain and
measure against c875a72, rather than add an isolated observation increment.

Audit validation passes the Cargo baseline, six original-source oracle tests,
CLI integration-test Clippy with warnings denied, formatting, Python compilation
and whitespace. Rstyle remains 2786 functions /zero violations and patterns zero
candidates. Production Rust and the 3.48 policy remain unchanged.

### Data-v3 3.49: connect tagged byte comparisons

The [bounded callback contract](docs/extraction-profiles/tagged-byte-comparisons-v1.md#connected-implementation-349)
connects exact core Option<u8>::map_or with original immediate u8 equality
predicates. Receiver and default evaluate once and eagerly; shared capture
creation performs no content read, and payload/body reads occur only for Some.
Complete source/body/environment capture and independent serialized checks
compose through strict import and matching on all three configured targets.
No ordinary closure data type, global provenance registry or cache is introduced.

The unchanged 26-function development corpus improves **six to thirteen
admissions**, seven gains /zero losses. Eight graph and six captured-fact
mutations fail closed. Exhaustive independent byte/sentinel/eager-default/None
oracles and all six complete-original-crate oracle tests pass.

The complete original regex-automata default library retains **2582 identities
and build contexts**, with **72 to 73 admissions**: Unit::is_byte is the sole gain,
with zero losses or unknown/failed outcomes. All 73 natural and thirteen development
admitted documents pass independent strict verification. All **190 comparisons**
among seven selected original and thirteen development functions complete with
unchanged matching defaults and no diagnostics. Full-crate enumeration is
explicitly budgeted out, distinct from completed capture/assessment and matching
of the selected family.

Frozen replay preserves sixty identities/denominators and **20 admitted /33
excluded /seven inactive**; seventy documents verify. Full frozen package rosters
also have zero outcome changes: lz4_flex 150 functions /seven admitted,
regex-syntax 956 /twenty and time-core 226 /eight. Historical results stay intact.

Against exact c875a72, one warmup plus seven alternating quiescent samples gives
median seconds baseline/current: Copy **0.89 /0.87**, cyclic **0.62 /0.62**,
callback corpus **0.78 /0.94**. Median peak RSS (KiB): **123064 /123204**,
**111760 /111572**, **121700 /124208**. Control matching is identical; added
callback proof and seven additional admissions explain unequal work in that
corpus. These controls establish no observed regression within this measurement,
not broad performance invariance.

Remaining complete-family blockers are Unit::eoi (assertion formatting,
TryFrom/Result/unwrap and source/helper evidence), Unit::is_word_byte (function
callback and constant-table body), and ByteClasses::eoi (checked_sub/Option unwrap
plus Unit::eoi). General closures/iterators remain open. No rewrite safety,
abstraction activation or detector-quality claim follows.

Workspace Clippy with warnings denied, doctests, formatting and whitespace pass.
Fresh rstyle: **2827 functions /zero violations**; patterns: **zero candidates**.
Final full workspace CI run: **884 passed /five skipped**, including the reported
assertion/subslice regressions and the corrected definition-count assertion.

### Next connected target: borrowed accelerator views

The [source-first contract](docs/extraction-profiles/borrowed-accel-views-v1.md)
selects original regex-automata Accel::len, Accel::is_empty and Accel::needles in
the complete default-feature library. Their chain combines a named literal array
length, borrowed-only array-field owner proof, range bounds/evaluation order,
formal field origins and helper-result/local/scalar-read composition. The
original owner derives Clone but not Copy: owned constructors/consumption remain
explicit negatives, not silently granted Copy or drop permission.

The fixed separate eighteen-function corpus specifies ten future positives and
eight negatives before baseline evaluation. At unchanged 3.49 it passes with
one admitted slice control /seventeen excluded and complete capture, independent
checking and matching. Both complete-original-crate runtime oracle tests pass,
including address identity and panics for metadata 8..=255. The natural HIR
confirms array-place Index dispatch; field origins and later helper restrictions
still need implementation evidence. This audit activates no new capability,
changes no frozen selection and claims no additional natural gain. Reuse the
existing checked source/module environment, observations and field-location
machinery; no additional global provenance registry or cache is justified.


### Data-v3 3.50: borrowed accelerator views

Implementation follows the
[bounded contract](docs/extraction-profiles/borrowed-accel-views-v1.md#connected-implementation-350).
The fixed eighteen-function corpus now meets ten admitted /eight excluded
expectations, including helper-returned views, saved reads and selected owners.
Focused strict verification/matching and corruption/constant-boundary checks pass.
The connected chain also required checked projected helper outputs with retained
memory reads and explicit panic propagation; no new capture/provenance inventory
or cache was introduced. Full CI passes 887 tests /five skipped, with zero rstyle violations and zero
pattern candidates. The unchanged 2582-function natural roster improves 73 to
76 admissions: precisely the three selected getters, zero losses. Alternating
control timings show overlapping ranges; the corpus now performs additional
proof/matching work for ten admissions instead of one. All 76 natural and ten development admissions pass independent verification and
strict import; all 78 selected comparisons complete without diagnostics. Frozen replay retains sixty identities/denominators and 20 admitted /33 excluded /
seven inactive outcomes; seventy documents verify.
Historical results above remain unchanged.

### Next connected target: bitset cardinality and emptiness

The [source-first contract](docs/extraction-profiles/bitset-cardinality-v1.md)
selects original util::look::LookSet::len/is_empty in the complete default library.
Its constructors and set operations already admit; the remaining getter chain
needs exact u32 count_ones, u32-to-usize TryFrom, concrete integer Result/unwrap,
and helper/value composition. The public u32 representation requires all high
bits, not only assigned assertion flags. Original derives/neighbors remain.

A fixed fourteen-function corpus declares eight future positives /six negatives
before admission evaluation. Unchanged 3.50 admits two constructor controls /
excludes twelve, with complete capture and matching and strict control checking.
Both independent original-crate runtime oracle tests pass. Pinned core confirms
this 64-bit conversion always produces Ok, but TryFromIntError contains an
IntErrorKind and must not be treated as a unit error. Current HIR leaves that
Result unnormalized. Inspect existing concrete-type and summary dependencies
before extending them; no additional provenance inventory or cache is justified.
This audit activates no capability and changes no frozen/historical result.

### Data-v3 3.51: connected bitset cardinality

The [bounded implementation](docs/extraction-profiles/bitset-cardinality-v1.md#connected-351-implementation)
connects count_ones → selected lossless TryFrom → immediate Result unwrap →
helper composition, reusing existing evidence without a new inventory/cache.
The unchanged natural roster gains precisely LookSet::len/is_empty: **76 → 78
admitted /2582, zero losses/unknown/failed**. Development expectations pass
**8 admitted /6 excluded** in std/no_std contexts across three targets; mutation
and unsupported-result boundaries reject. Final full suite: **892 passed /five skipped**; fresh Clippy passes. Fresh rstyle/patterns: **zero violations/candidates**. Control timing
ranges overlap; the expanded corpus does additional work. Frozen sixty retain **20 admitted /33 excluded /seven inactive** and every
identity/denominator; 74 package documents verify. The unchanged regex-syntax
roster gains its two selected counterparts (**20 → 22 /956**, zero losses);
lz4_flex/time-core are unchanged. All 78 natural admissions, eight development
positives and two counterparts pass independent strict checking; all **66
selected comparisons** complete without diagnostics. Fresh doctests/formatting pass; refreshed alternating control timing ranges
overlap (Copy 0.90/0.89s, cyclic 0.64/0.65s); the development corpus measures
0.39/0.50s for two/eight admissions. Historical selections/results remain unchanged.

### Next shared blocker chain: end-of-input helpers

The [bounded target](docs/extraction-profiles/end-of-input-v1.md) connects original
Unit::eoi and ByteClasses::eoi through formatted scalar assertions, narrowing
integer Result and checked-sub/Option unwrap. Existing nominal, array-read and
helper foundations already admit. Both methods remain excluded at 3.51; no new
coverage is claimed. The source-first thirteen-function corpus has **3 controls admitted /10
excluded** across std/no_std contexts on three installed targets, with complete
capture/matching and strict control checking. Both original-crate oracle tests
pass, including panic messages and all last-byte values. Validate the whole
chain rather than separate syntax rows; implementation remains pending.

### Data-v3 3.52: end-of-input and panic-aware helper composition

The [bounded contract](docs/extraction-profiles/end-of-input-v1.md#connected-352-contract)
connects scalar assertion diagnostics, narrowing TryFrom, checked-sub/Option
unwrap and verified read-only checked-Copy helpers with explicit panic evidence.
The unchanged natural roster improves **78 → 81 /2582, zero losses**:
Unit::eoi, ByteClasses::eoi and ByteClasses::is_singleton. Frozen sixty improve
**20 → 23 admitted /30 excluded /seven inactive**, gaining three time-core
calendar callers; identities, denominators, contexts and historical reports stay
intact. Full lz4_flex/regex-syntax rosters are unchanged; time-core gains three.
Development expectations pass **9 admitted /4 excluded** in std/no_std contexts
on three targets. Independent checking covers 81 natural and nine development
admissions plus all 40 frozen-package admissions; all **55 selected comparisons**
complete. Original-crate calendar and
end-of-input oracles, adversarial mutations and **897 tests /five skipped** pass;
Clippy/doctests/formatting pass; **zero rstyle violations/pattern candidates**.
Control timing ranges overlap; development does additional work. No new
provenance inventory/cache, weaker admission, purity or rewrite-safety claim.

Next source-first audit should compare the complete remaining chains:
Accel::as_accel_tys needs scalar-alias/source proof, comparison assertions and
its actual slice-array/native conversion composition; Accel::contains needs the
actual iter().position callback; Unit::is_word_byte needs a function callback
and const-built indexed table. Saved checked Results/Options, general payload
binding and `?` remain connected follow-up work. Do not substitute passing
constructs for these original bodies or treat a partial chain as coverage.

### Next connected target: accelerator packing

The [source-first contract](docs/extraction-profiles/accelerator-packing-v1.md)
selects original Accel::as_accel_tys and its callers, retaining the Clone-only
owner and all stored bytes. Its complete chain needs scalar-alias proof,
comparison assertions, implicit array-field byte-copy reads and checked helper
transport. Fixed development expectations are thirteen positives /six negatives;
the complete baseline admits two controls and excludes seventeen in six build
contexts. An unsupported receiver exposed a whole-unit capture abort; it now
excludes before emission. This repair adds no capability or measured admission.
All 898 tests pass /five skipped, with zero rstyle violations/pattern candidates.
Continue the connected implementation; frozen identities/history remain intact.

### Connected accelerator packing delivered

Data-v3 **3.53.0** connects direct scalar aliases, standard scalar comparison
assertions, implicit array-field byte copies and checked read-only helper
transport. Original `Accel::as_accel_tys` now admits in its complete unchanged
crate; lz4_flex's original `HashTable4KU16::get_hash_at` also gains admission.
Possible panic, conditional read occurrences and Clone-only ownership remain
explicit. No additional census/cache or relaxed scoring/admission is introduced.

Measured admissions: development **2 → 13 /19**, regex-automata **81 → 82 /2582**,
lz4_flex **7 → 8 /150**, regex-syntax **22/956** and time-core **11/226** unchanged.
**Zero losses**; frozen sixty retain **23 admitted /30 excluded /7 inactive**
and identical identities/denominators. All 136 admissions across these selections
pass independent verification and strict admission; **91 selected comparisons**
complete without diagnostics. Full regex-automata matching is budget-limited.

**905 tests pass /5 skipped**, Clippy/doctests/formatting pass; rstyle reports
**0 violations /2900 functions**, patterns **0 candidates**. See the compact
[contract/results](docs/extraction-profiles/accelerator-packing-v1.md) for scope,
negative expectations and validation. Remaining natural chains are iterator
position/Option composition for `Accel::contains` and named callback/const-table
proof for `Unit::is_word_byte`; saved payload control flow and `?` remain open.
Alternating performance controls retain unchanged matching/timing ranges;
packing grows from 2 to 13 admissions and 1 to 78 comparisons (0.56 → 0.75 s).

### Next connected family: fresh shared byte-iterator position

Select original regex-automata `Accel::contains` and its unchanged len/needles
dependencies. The [contract](docs/extraction-profiles/iterator-byte-position-v1.md)
fixes the full chain: fresh iterator authority/state, `|&b|` callback observations,
short-circuit read occurrences, existing scalar Option/is_some and helper
transport. The slice implementation overrides the generic position default;
current capture lacks that implementation identity and complete callback/type
evidence. S09 already supports the owner/getters and needs no new census.

The source-first development corpus fixes eleven positives /six negatives;
baseline **2/17 admit** in six std/no_std target contexts, with complete capture,
assessment and matching. Three independent original-crate runtime tests pass,
covering every queried byte, active payload/padding, invalid-header panic and
first-index/short-circuit visits. No production/policy change or new natural gain
is claimed; the next implementation must connect the nine blocked positives.

### Data-v3 3.54: connected fresh byte-iterator position

Original Accel::contains now admits through its unchanged len/needles chain,
with exact slice-implementation authority, shared capture proof, conditional
first-match prefix reads, scalar Option and checked helper transport. Development
improves **2 → 11 /17**; regex-automata **82 → 83 /2582**, zero losses.
The three other package rosters and frozen sixty remain unchanged:
**23 admitted /30 excluded /7 inactive**, identical identities/denominators.

All **135 admissions** pass independent strict verification; all **91 selected
comparisons** complete. **914 tests pass /5 skipped**, including six target/std
contexts and adversarial imports; original runtime oracles, Clippy, doctests and
formatting pass. Rstyle **0 violations /2937 functions**, patterns **0 candidates**.
Unchanged control medians/matching remain stable; position does additional work
(1 → 55 comparisons, 0.68 → 1.09s). See the compact
[contract/results](docs/extraction-profiles/iterator-byte-position-v1.md).

Next audit the complete original Unit::is_word_byte chain: its as_u8 dependency
already admits, but the named callback and utf8 helper's const-built 256-entry
table, nested declaration/source boundary and constant-index read all need proof.
Do not substitute a passing closure or replacement classifier. Stored iterator
state, arbitrary predicates, implicit array-field iterator places, saved checked
payloads and general Result/if-let/`?` composition remain separate blockers.
No abstraction activation, rewrite-safety or held-out quality claim is made.


### Next connected family: ASCII word classifiers

The [compact source-first contract](docs/extraction-profiles/word-byte-classifiers-v1.md)
fixes original regex-automata Unit/utf8 dependencies and regex-syntax's complete
classifier, plus independent positive/negative development selections. The audit
found a shared scalar gap: byte literal/range/OR source rejection and byte-bound
exhaustiveness. Resolve that complete syntax classifier first, then the original
const-table and named map_or chain. Unit::is_word_byte remains unsupported until
all dependencies compose; five LookMatcher callers have additional owner and
arithmetic blockers. No source replacement, new census/cache or admission gain
is claimed by this source-first selection.


### Data-v3 3.55: connected byte-scalar classifiers

Original regex-syntax is_word_byte now passes complete Cargo capture, independent
verification, strict admission and matching. Its unchanged package gains
**22 → 23 /956**; byte development **0 → 9 /11**, table/callback development
**3 → 4 /20**. Other natural rosters and the frozen sixty are unchanged;
**zero losses**, frozen **23 admitted /30 excluded /7 inactive**.

All **138 admissions** independently strict-verify. All **36 selected comparisons**
among nine unique function keys complete; both contexts of the verbatim classifier
copy verify, and development matching completes separately. **918 tests pass /5
skipped**, six target/std contexts, original/runtime composition oracles,
Clippy/doctests/fmt; **rstyle 0 /2952 functions**, **patterns 0**. The
[compact contract/results](docs/extraction-profiles/word-byte-classifiers-v1.md)
retains identities, negative expectations and validation limits. Clean-baseline
copy/cyclic control medians and matching remain stable; the byte corpus does
additional admission/matching work (0.28 → 0.36s).

Next connect the original immutable const-built bool table, its nested declaration
ownership and lossless `u8 as usize` range proof through helper admission; then
named map_or callback transport. Unit::is_word_byte and the five LookMatcher
callers remain incomplete. No new census/cache, weaker admission/scoring or
abstraction/rewrite-quality claim.

### Data-v3 3.56: connected compile-time bool tables

Original regex-automata utf8::is_word_byte now admits with complete evaluated
contents, owned compile-time declarations and saved lossless unsigned cast bounds.
Its runtime table builder still excludes for memory writes. Natural admissions
**83 → 84 /2582**; table/callback development **4 → 6 /20**. Other selections
are unchanged, zero losses; frozen **23 admitted /30 excluded /7 inactive /60**
retain their identities and denominators.

All **141 admissions** independently strict-verify; **15 selected comparisons**
complete. **921 tests pass /5 skipped**, including reported regressions and six
cross-target/std contexts; oracles, Clippy/doctests/fmt pass, **rstyle 0 /2983
functions**, **patterns 0**. See the compact
[contract/results](docs/extraction-profiles/word-byte-classifiers-v1.md) for bounded
performance observations and validation limits. Next connect exact named map_or
callbacks to finish Unit::is_word_byte and its selected callers; five LookMatcher
callers still need separate owner/arithmetic work. No new census/cache or weaker
admission/scoring.


### Data-v3 3.57: connected named Option callbacks

Original regex-automata Unit::is_word_byte now completes its unchanged
constructor/projection → Option → named callback → const-table chain through
Cargo capture, independent verification, strict admission and matching.
Natural **84 → 85 /2582**, development **6 → 13 /20**, zero losses. Other
selections and the frozen sixty are unchanged: **23 admitted /30 excluded
/7 inactive**, identical source/build identities and denominators.

All **149 admissions** independently strict-verify; all **91 selected comparisons**
complete. **924 tests pass /5 skipped**, six target/std contexts, original oracles,
Clippy/doctests/fmt, **rstyle 0 /3007 functions; patterns 0**. The
[compact contract/results](docs/extraction-profiles/word-byte-classifiers-v1.md)
records bounds, negative expectations, stable controls and validation limits.
Full natural matching remains explicitly budget-limited; no new census/cache or
weaker admission/scoring.

Next audit LookMatcher's five direct ASCII predicates and negating wrapper as
one chain. The current borrowed-owner gate rejects its nested checked-Copy field;
metadata length resolution also depends on that frontend validation. Confirm
those dependencies, then assess guarded slice-read and possible-panic helper
transport before promising the family. General stored callables, Result/if-let/?,
broader loop/value composition and cross-unit helpers remain open.

### Data-v3 3.58: connected nested getters and line boundaries

The [compact contract](docs/extraction-profiles/ascii-boundaries-v1.md) establishes
an exact checked-Copy child proof for borrowed-only owners. This completes the
six original LookMatcher start/end, LF and CRLF methods through their actual
helper chain, plus the nested getter/caller. Five direct ASCII predicates also
admit; negation and repeated indexed-read callers remain excluded.

Natural **85 → 97 /2582**, development **1 → 8 /19**, zero losses on unchanged
identities/build contexts. Earlier word/byte selections remain **13/20 and 9/11**;
other complete package selections remain **8/150, 23/956 and 11/226**. Frozen
**23 admitted /30 excluded /7 inactive /60** retains every identity/denominator.
All **169 admissions** independently strict-verify; all **210 selected comparisons**
complete. **928 workspace tests pass /5 skipped**,
six target/std contexts, four original runtime oracles, Clippy/doctests/fmt,
**rstyle 0 /3008 functions; patterns 0**. No new census/cache or scoring changes.

Next connect dynamic indexed-read helper summaries: retain each callee read's
selector/guard, exact argument snapshots, shared aliases and possible panic,
then complete the unchanged negating/repeated ASCII callers. Reuse the existing
unit links and verified graphs; copying an access-relative index projection onto
a CALL operand would be unsound. Broader ownership/Unicode/loop/Result work stays
outside this bounded family until a complete source-selected chain needs it.


### Data-v3 3.59: complete ASCII indexed-helper composition

The [compact contract](docs/extraction-profiles/indexed-read-helpers-v1.md) connects
shared scalar reads through nested/repeated verified helpers, retaining each
callee selector, execution guard and saved actual origin. Bounds requirements now
retain possible panic through all caller levels. The unchanged seven original
ASCII methods complete, including negation; development reaches all ten intended
positives. Ownership, recursive/looping helpers and reference-bearing results
remain outside this contract.

Natural **97 → 98 /2582**, development **8 → 10 /19**, zero losses on unchanged
identities/build contexts. Word/byte selections remain **13/20 and 9/11**; other
packages remain **8/150, 23/956 and 11/226**. Frozen **23 admitted /30 excluded
/7 inactive /60** preserves every identity/denominator.

All **172 admissions** independently strict-verify; all **276 selected comparisons**
complete. **929 workspace tests pass /5 skipped**, six target/std contexts,
original runtime oracles, adversarial scope/totality/historical-policy checks,
Clippy/doctests/fmt; **rstyle 0 /3014 functions; patterns 0**. Full natural matching
remains explicitly budget-limited. No new census/cache or weaker scoring/admission.

The declared ASCII family is complete. Before the next increment, select a new
small natural family from source and audit its full blocker chain. Adjacent
Unicode methods retain concrete-type and source-correspondence blockers; do not
expand into general Unicode merely by proximity. Prioritize shared S09/import
restrictions and F10/checked-Option composition where the selected family needs
them. General ownership/drop, Result/if-let/?, cross-unit helpers and broader
loop/value composition remain open.

### Data-v3 3.60: complete fieldless flag helper families

The [compact contract](docs/extraction-profiles/look-flags-v1.md) connects complete
fieldless enum declarations, source-checked discriminants, variant values/casts
and shared Copy receiver loads through the original Look/LookSet helper chain.
Historical policies retain their boundary; general enums/ownership stay open.

Unchanged complete selections: regex-automata **98 → 114 /2582**, regex-syntax
**23 → 36 /956**, lz4_flex **8/150** and time-core **11/226** unchanged; **zero
losses**. The syntax gains mirror thirteen byte-identical functions. Frozen
**23 admitted /30 excluded /7 inactive /60** retains every identity/denominator.

**193 admissions** (169 original, 24 development) independently strict-verify;
**182 original and 276 development comparisons** complete. **936 tests pass
/5 existing skips**, doctests/fmt/Clippy pass; **rstyle 0 /3042 functions, patterns
0 unignored**. The compact contract and [execution frontier](docs/rust-support-execution.md)
record timings, retained evidence and pending optimization CI. Next integrate the distinct
character scalar domain for original `is_meta_character`, then its escapable
character caller (exact ASCII predicate plus literal ranges). Stored-reference
results, lifetime-only callable owners, mixed enum children, Result/?, broader
loops, cross-unit linking and the full P01–P22 roadmap remain active.

### Data-v3 3.61: connected Unicode scalar classifiers

[Character contract](docs/extraction-profiles/character-leaf-v1.md): standalone
char values, exact literals, comparisons and literal/OR matching support original
`regex-syntax::is_meta_character` and incidental `ast::parse::is_hex` end to end.
Unchanged syntax selection **36→38 /956**, no losses; lz4_flex **8/150**,
time-core **11/226**, frozen **23 admitted /30 excluded /7 inactive /60** unchanged.
All three source-selected neighboring negatives remain excluded. Automata's prior
114/2582 result was not remeasured in this increment.

**57 original admissions** independently strict-verify; all **three original and
15 development comparisons** complete. Exhaustive original-library oracle passes
all1112064 valid Unicode scalars. **939 tests /5 existing skips**, doctests/fmt/Clippy
pass; **rstyle0 /3051 functions, patterns0 unignored**, existing ignores unchanged.
The contract records structural matching limits and performance evidence. Next
complete the escapable-character chain (ASCII predicate plus ranges); the broader
P01–P22 roadmap remains active.

### Data-v3 3.62: complete character escape classifier

[Compact contract](docs/extraction-profiles/character-escapes-v1.md): literal
Unicode scalar ranges and exactly pinned `char::is_ascii` connect the unchanged
original `is_escapeable_character` through its metacharacter helper and early
returns. Syntax **38→39 /956**, zero losses; lz4_flex **8/150**, time-core
**11/226** and frozen **23 admitted /30 excluded /7 inactive /60** unchanged.
Automata's prior114/2582 was not remeasured.

All **58 original admissions** strict-verify; **six original comparisons** complete.
Exhaustive oracle1112064 valid scalars; three targets × std/no_std composition and
resealed adversarial tests pass. **945 tests /5 skips**, fmt/Clippy/doctests pass;
**rstyle0 /3070 functions, patterns0 unignored**. Implementation native CI passes
Linux x86, Linux ARM and macOS. Existing structural detector limitations remain;
no equivalence, rewrite-safety or held-out quality claim follows.

Next source-selected family: original libm `k_sinf`/`k_cosf`, joining exact float
bits/constants, ordered arithmetic and f64→f32 narrowing. Its independent4202-input
oracle passes; actual full Cargo context is captured, with both kernels excluded
before production activation. The
broader P01–P22 roadmap and borrowed-result/Result/ownership blockers remain active.


### Data-v3 3.63: bounded float foundation; original activation pending

[Contract](docs/extraction-profiles/float-kernels-v1.md): standalone f32/f64,
exact literal/same-module constant bits, ordered f64 +/*, sign negation and
f64→f32 narrowing. Diagnostic composition spans three targets × std/no_std,
strict verification, historical-policy and resealed semantic attacks. General
float arithmetic/comparisons, nested floats and integer conversions remain closed.

Full unchanged original libm selection: **1/1131 admitted → unchanged**, zero
losses. Both selected kernels reach a previously hidden source-boundary blocker:
local `cfg_if!` invocations own21 module files that the closed census cannot yet
prove. No file is omitted or macro authorized. Next: bounded module-only cfg-if
ownership evidence (P22/P05), then repeat original strict admission/matching.
The independent4202-input oracle passes; diagnostic admission does not complete
the original family. This is the first foundational increment for this chain.

Validation: **955 tests passed /5 existing skips**, doctests, formatting and
workspace all-target Clippy (`-D warnings`) pass. **Rstyle0 /3119 functions;
patterns0 unignored**, existing ignore ledger unchanged. Frozen60 remains
**23 admitted /30 excluded /7 inactive**; complete package comparisons preserve
syntax39/956, lz4_flex8/150 and time-core11/226 with zero gains/losses. Automata
was not remeasured. Retained frozen run: `run-bgyl78v2` versus3.62 `run-15rr9we7`.

Equal-work3.60→3.63 controls (warmup +7 alternating samples): medians Copy
0.63→0.63s, cyclic0.62→0.62s, ASCII0.85→0.88s, flags8.00→7.97s. Admissions and
matching are unchanged. This combines3.61–3.63; it does not isolate float cost.
Ranges/RSS: `/tmp/rstyle-float-performance.log` and
`/tmp/rstyle-float-foundation-performance-results.json`.


### Data-v3 3.64: original float kernels through closed module proof

[Contract](docs/extraction-profiles/float-kernels-v1.md): exact local cfg-if
module expansion chains and exhaustive primitive dependency proof connect
unchanged original libm `k_sinf`/`k_cosf` through strict admission and matching.
All140 source files remain; general nominal import restrictions remain intact.
Changed/shadowed macros, forged/incomplete chains, local aliases/globs and hidden
calls/drop-bearing values reject. General macro permission is not established.

Unchanged libm **1→5/1131**, four gains and zero losses: both selected kernels
plus two independently audited/exhaustively checked digit classifiers. Frozen60
stays **23 admitted /30 excluded /7 inactive**; syntax39/956, lz4_flex8/150 and
time-core11/226 are unchanged. All **63 original admissions** strict-verify;
**seven selected comparisons** complete. Full-library matching stays budget-limited.

**976 tests /5 existing skips**, doctests/fmt/Clippy pass; **rstyle0/3174,
patterns0 unignored**, ignores unchanged. Numerical oracle4202 inputs; digit
oracles256 bytes each. Retained frozen run `run-qqq32cfl` versus `run-bgyl78v2`;
compact evidence paths are in the contract. Automata was not remeasured; native
three-platform CI for3.64 passes (run39643046602).

Next source-selected candidate: original `k_cos`, ordered f64 subtraction and an
independent two-input NaN oracle. Lifetime-only callable owners, stored-reference
results, mixed children, Result/?, broader loops and the P01–P22 roadmap remain open.


### Data-v3 3.65: original cosine tail with ordered subtraction

[Contract](docs/extraction-profiles/float-kernels-v1.md): checked binary64
subtraction completes unchanged original libm `k_cos`, including six constants
and shadowed saved operands. Libm **5→6/1131**, one gain, zero losses; source/build
identities unchanged. All six libm admissions strict-verify; three selected
comparisons complete. Independent original oracle: **4352 inputs**.

Frozen60 stays **23 admitted /30 excluded /7 inactive**; complete syntax39/956,
lz4_flex8/150 and time-core11/226 remain unchanged. Run `run-6qkc6ea9` versus
3.64 `run-qqq32cfl`. **980 tests /5 existing skips**, doctests/fmt/Clippy pass;
**rstyle0/3175, patterns0 unignored**, ignores unchanged. Three-target × std/no_std
composition, resealed semantic attacks and historical-policy rejection pass.
All **64 original admissions** strict-verify; **nine selected comparisons** complete.
Native CI for3.65 passes Linux x86, Linux ARM and macOS (run39646408115). Automata was not
remeasured; full-library matching remains budget-limited.

Next: original `k_sin` needs exact i32 equality and exhaustive float-result
branches. Borrowed `Input` getters still need complete mixed-enum owner proof,
lifetime-only callable ownership and stored-slot/referent origin distinction.
The broader P01–P22 roadmap remains active; evidence stays in implementation commits.

### Data-v3 3.66: original sine tail with checked scalar guards

[Contract](docs/extraction-profiles/float-kernels-v1.md): exact i32 formal/literal
equality and complete float-valued alternatives connect original `k_sin`'s six
coefficients and ordered arithmetic. Libm **6→7/1131**, exactly one gain, zero
losses; all source/build identities match. All7 admissions strict-verify; six
selected comparisons complete. Independent original native oracle: **5384 inputs**.
Three-target × std/no_std composition and resealed guard/source/binding/merge
attacks pass; unsupported effects in either branch remain closed.

**986 tests /5 existing skips**, doctests/fmt/Clippy pass; **rstyle0/3186,
patterns0 unignored**, ignores unchanged. Frozen run `run-008n2qyy` versus3.65
`run-6qkc6ea9` retains23 admitted /30 excluded /7 inactive and unchanged package
rosters. All **65 original admissions** strict-verify; **12 selected comparisons**
complete across the package replays. Identity comparison and replay logs are retained under
`/tmp/rstyle-float-sin-tail-*`. Native3.66 CI passes Linux x86, Linux ARM and macOS
([run39651876248](https://github.com/rstyled/rstyle/actions/runs/39651876248)).
Full-library matching remains budget-limited; automata was not remeasured.

Next shared blocker: complete borrowed `Input` ownership, starting with original
`get_earliest`. Its mixed `Anchored` child needs explicit unit/single-tuple shapes
and nested Copy `repr(transparent)` wrappers before lifetime-only callable proof.
Stored-reference results and caller substitution follow that coherent owner boundary.

### Data-v3 3.67: complete borrowed mixed-owner foundation

[Contract](docs/extraction-profiles/borrowed-result-chain-v1.md): complete mixed
unit/tuple enum children, nested Copy transparent wrappers and exact lifetime-only
impl/callee proof connect a diagnostic Input owner, bool getter and two callers
through capture, strict verification and matching on three targets × std/no_std.
Unsupported ownership, payload operations and unknown effects remain closed;
historical stored-slice owners retain their existing support. Independent native
execution of original `Input::get_earliest`: **1128 states /5640 observations**.

**997 tests /5 existing skips**, doctests/fmt/Clippy pass; **rstyle0/3215,
patterns0 unignored**, ignores unchanged. Frozen `run-jgt43t72` versus3.66
`run-008n2qyy`: **23 admitted /30 excluded /7 inactive**, unchanged identities.
Complete lz4_flex8/150, regex-syntax39/956 and time-core11/226 rosters have **zero
new/lost admissions**; all58 admissions strict-verify and six selected comparisons
complete. Logs: `/tmp/rstyle-input-owner-*`. Libm was not recaptured this increment.

**No original Input gain at this checkpoint.** Subsequent streaming recovery
validated complete3.66/3.67 reports despite CLI timeouts: automata114→120/2582,
six stored-slice flag/count reader gains, zero losses; all identities unchanged.
Full-library matching remains incomplete. Original Input source closure follows
in3.68. Native3.67 CI passes Linux x86/ARM/macOS
([run39660188327](https://github.com/rstyled/rstyle/actions/runs/39660188327)).


### Data-v3 3.68: original getters through precise Copy dependency proof

[Contract](docs/extraction-profiles/borrowed-result-chain-v1.md): exact builtin
Copy dependencies may remain inside a distinct shared owner despite neighboring
macros; complete declarations, derives, fields and owning-scope checks remain.
Original Input `get_earliest`, `get_span`, `start`, `end` and `is_done` now strictly
admit, including repeated helper-result field reads. Direct dependency operations,
macro effects, owned Input/drop and missing evidence remain closed.

Fresh complete3.67 Cargo evidence recertified under3.68: automata **120→132/2582**,
**12 gains /0 losses**, all source/build identities unchanged; all132 admissions
strict-verify. No driver grammar changed. Independent original native oracle:
**1128 states /11280 observations**. All10 selected comparisons complete; the
predeclared cross-result negative `get_span`/`get_earliest` unexpectedly matches
(7/8 negatives hold). This is a detector limitation, not equivalence evidence;
scoring/admission remain unchanged. Full-library matching stays incomplete.

**1002 tests /5 existing skips**, doctests/fmt/Clippy pass; **rstyle0/3223,
patterns0 unignored**, ignores unchanged. Frozen `run-kxk6hkj_` versus3.67
`run-jgt43t72` stays **23 admitted /30 excluded /7 inactive**; complete syntax39/956,
lz4_flex8/150 and time-core11/226 rosters are unchanged. All58 package admissions
strict-verify; six selected comparisons complete. Libm was not recaptured.

Next: stored-reference slot/referent origins and helper-result substitution for
`haystack → is_char_boundary → utf8::is_boundary`. Broader roadmap remains active.


### Data-v3 3.69: original stored-slice getter and checked consumer

[Contract](docs/extraction-profiles/borrowed-result-chain-v1.md): exact slot reads
and returned referents compose through original `Input::haystack →
Input::is_char_boundary → utf8::is_boundary`, including checked conditional reads.
Complete3.67 Cargo context recertified: **132→134/2582, two gains /zero losses**;
all source/build identities match, all134 roots strictly verify. All three selected
comparisons reject as predeclared; independent original native oracle:39168 cases.
**1006 tests /5 existing skips**, doctests/fmt/Clippy pass; **rstyle0/3233,
patterns0 unignored**, existing15 ignores unchanged. Frozen `run-hgf65atp` versus
`run-kxk6hkj_`: **23 admitted /30 excluded /7 inactive**; identities/denominators
and complete lz4_flex8/150, syntax39/956, time-core11/226 rosters unchanged. All58
package admissions strict-verify; six selected comparisons complete. Evidence:
`/tmp/rstyle-borrowed-result-*`. Full-library matching remains incomplete; libm was
not recaptured. Remaining adjacent gaps: aggregate/nested-reference helper results,
mixed-enum value operations and generic/mutable Input APIs. Broad roadmap active.

### Data-v3 3.70: original checked encoded lengths

Original base64 0.23.1 `encode::encoded_len` now composes exact 64-bit
`usize::checked_add/checked_mul`, immutable Some if-let with explicit else,
early None returns and nested scalar/Option results through strict admission and
matching. [Fixed source selection and expectations](eval/coverage-development/base64-encoded-len-selection.json).
Default context: **7→8/167, one gain /zero losses**, unchanged source/build
identities. Three targets × default/no-default retain **36 strict admissions and
102 complete comparisons**; encoded_len rejects every predeclared incompatible
peer. Independent execution of the unchanged original library: **16,552 cases**.
Exact impl-token-to-brace spans also repair cfg-attributed getter ownership;
independent attribute/activity and neighboring-item checks remain required.

Frozen `run-jis8b7db` versus 3.69 `run-hgf65atp`: **23 admitted /30 excluded /7
inactive**, unchanged identities/denominators. Complete lz4_flex 8/150,
syntax 39/956 and time-core 11/226 rosters have zero gains/losses; all 58 admissions
strict-verify and six selected comparisons complete. Historical compiler facts
recertified separately under 3.70 preserve base64's 7/167 and automata's 134/2582
admissions and source/build identities; all 134 automata admissions strict-verify
and three selected comparisons complete. Older policy artifacts remain unchanged;
this does not claim their serialized 3.69 labels are 3.70 certificates.

**1014 tests /5 existing skips**, doctests/fmt/Clippy pass; **rstyle 0/3278,
patterns 0 unignored**, existing 15 ignores unchanged. The accompanying immutable
proof reuse cuts the unchanged slowest CLI test 187.29→50.35s and the full suite
380.827→225.250s. Evidence: `/tmp/rstyle-base64-encoded-*`,
`/tmp/rstyle-checked-length-*`, `/tmp/rstyle-perf-*`. Libm was not recaptured.
Next blocker combinations remain borrowed Option payload/closure/helper-result
composition for Transition, and opaque field/drop proof for PatternSet; broader
Rust, rewrite safety and detector-quality claims remain separate.


### Strict comparison and fixture replay performance

Normalize JSON in place, serialize unordered sort keys once and compare documents
sequentially; preserve identifier mapping, ordering, duplicates and historical
exceptions. Fixtures reuse one strict unit import per context; all mutation,
admission and policy assertions remain. Equal-work alternating samples before
fixture reuse: flags 49.83–50.02→42.44–42.51s, slices 43.03–43.13→39.74–39.88s.
Final development-worktree suite, including the pending callback slice:
224.835→167.679s, 1014→1020 passing tests /5 existing skips. Flags 50.665→36.672s,
slices 43.520→34.448s, passive owners 29.801→13.857s include duplicate-import
removal. Doctests/fmt/Clippy pass; rstyle 0/3327, patterns 0 unignored, existing
15 ignores unchanged. Evidence: `/tmp/rstyle-slow-*`; no new coverage milestone.

Follow-up: exact-ID documents compare directly against regenerated typed evidence
before JSON normalization; reordered/renamed documents retain the full comparison.
Conformance profiles/inventories and all semantic fields remain checked. After
warmup, alternating equal-work samples: flags 36.00–36.24→34.51–34.69s, slices
33.98–34.00→32.83–32.90s; peak RSS remains comparable. Development-worktree suite:
**152.076s, 1022 passing tests /5 existing skips**; doctests/fmt/Clippy pass,
rstyle **0/3327**, patterns **0 unignored**, existing 15 ignores unchanged.
Evidence: `/tmp/rstyle-perf4-*`; extraction contracts and selections unchanged.

Receiver validation follow-up: borrow observations, reuse exact nominal shapes
within one inventory, and parse fieldless declarations once. Every occurrence
still traverses children and counts toward unchanged depth/entry limits; unfinished
or changed evidence cannot reuse a proof. Four regression tests cover these bounds.
Equal-work flags test: approximately **35→24s**; stored slices remain approximately
33s. Development worktree: **1027 passing tests /5 existing skips, 143.152s**;
doctests/fmt/Clippy pass, **rstyle 0/3337, patterns 0 unignored**, existing 15 ignores
unchanged. Evidence: `/tmp/rstyle-perf6-*`. No new extraction capability or natural
admission measurement; the pending callback slice remains separate.

Origin-query follow-up: successful results are reused only for exact immutable
source nodes, component selectors, owning analysis and verification policy; failed,
edited and over-budget queries recompute. Equal-work slices: **32.47→30.37s**;
flags remain approximately24s. Compact JSON retains every report field and cuts
the flags report **2,290,482→1,507,677 bytes (34%)**, with identical parsed matching
and outcomes. Development worktree: 1029 full-run passes plus the corrected new
origin regression passing separately; five existing skips. Doctests/fmt/Clippy
pass; **rstyle 0/3346, patterns 0 unignored**, existing15 ignores unchanged.
Evidence: `/tmp/rstyle-perf7-*`; no extraction capability or admission change.

Type-conversion performance follow-up: bounded process-local caches retain only
successful canonical type parses and exact complete concrete observations.
Capability, source and document verification still run; declaration ranges are
detached to avoid retaining whole source snapshots. Equal-work isolated tests:
slices **29.72→21.98s**, flags **23.84→15.73s**, all six build contexts retained.
Adversarial checks cover changed declaration/field evidence, noncanonical tokens,
policy checks, independent returned values and retention limits. Evidence:
`/tmp/rstyle-perf8-*`; no extraction-policy or admission claim is added.
**1039 tests pass /5 existing skips**, doctests/fmt/Clippy pass;
**rstyle 0/3353, patterns 0 unignored**, existing15 ignores unchanged.

### Data-v3 3.71: original Transition callback family

[Source selection, bounded contract and independent expectations](eval/coverage-development/transition-callback-selection.json).
Original `Transition::{matches,matches_unit}` now composes checked slice access,
owned/borrowed byte Options, an exact shared-formal closure environment and the
verified `matches_byte` helper. Every owner field, passive dependency, neighboring
macro and drop obligation remains checked; general closures remain deferred.
Unchanged native-default selection: **134→137/2582, zero losses**. Two intended
gains are the selected predicates; incidental `Match::is_empty` reappears through
its existing helper chain after fresh capture removes an older source diagnostic.
This incidental gain establishes no new receiver grammar.

Three targets × default/nfa-only features: **669 strict contextual admissions,
24 selected function admissions, 36 independently specified comparisons**.
Independent unchanged-library oracle passes all **16,777,216 byte triples per
native feature context**, both next-state identities and out-of-range controls.
Native execution is ARM Linux; x86 Linux and ARM macOS evidence is cross-target
capture/replay. Full-library matching remains incomplete under max-pairs zero.
Frozen `run-t14bfxdk` versus `run-jis8b7db`: **23 admitted /30 excluded /7 inactive**;
all source/build identities and complete package rosters remain unchanged
(lz4_flex8/150, syntax39/956, time-core11/226). All58 admissions strictly replay;
six selected comparisons complete. The optimized verifier also rechecks all137
original automata admissions and their six selected comparisons.
Validation and performance gates are recorded in the immediately preceding
checkpoint; evidence: `/tmp/rstyle-transition-*`. Next source-selected chain:
`Span::len → Match::len` needs the exact usize saturating-subtraction summary and
fresh helper-result audit. PatternSet still needs opaque Box/drop proof; broader
Result/?/generic and cyclic composition remains open.

### Data-v3 3.72: original Span/Match length family

[Selection, contract and independent expectations](eval/coverage-development/span-length-selection.json).
Pinned 64-bit usize saturating subtraction completes original `Span::len →
Match::len`, including full Copy owners and existing owned-result/shared-receiver
composition. No new receiver grammar or general primitive-method permission.
Three targets × default/no-feature contexts gain exactly these two functions:
**137→139/2582** default, **83→85/590** without features; zero losses and unchanged
source/build identities. **672 strict contextual admissions, 42 selected admissions,
30 predeclared comparisons** pass. Native unchanged-library oracle: **8748 cases
per feature context**, including inverted Spans and integer boundaries. x86 Linux
and ARM macOS results are cross-target capture/replay, not native execution.

Frozen `run-72emt3ji` versus `run-t14bfxdk`: **23 admitted /30 excluded /7 inactive**;
full package rosters remain lz4_flex8/150, syntax39/956, time-core11/226. All58
strictly replay; six selected comparisons complete. **1039 tests /5 existing
skips**, doctests/fmt/Clippy pass; **rstyle0/3353, patterns0 unignored**, existing15
ignores unchanged. Evidence: `/tmp/rstyle-span-*`; full-library matching remains
incomplete under max-pairs0. Next source audit: base64ct's bool-to-usize encoded
length chain. Range-valued helpers, opaque Box/drop owners, Result/?/generics and
looping-callee composition remain separate blockers; P01–P22 stays active.

Performance follow-up: exact declaration-shape reuse and indexed greedy Unicode
string matching reduce unchanged six-context CLI tests by **26%**: flags
15.67–15.69→11.52–11.62s; stored slices 22.07–22.11→16.35–16.38s.
Alternating baseline/current repeats preserve all admission/matching assertions;
peak RSS is slightly lower. No new capabilities or scoring changes. Differential
checks cover 14,400 nonempty Unicode string pairs and long repeated inputs;
changed declarations and type budgets remain checked. Evidence: `/tmp/rstyle-perf9-*`.
Validation: **1042 tests /5 existing skips**, doctests/fmt/Clippy pass;
**rstyle0/3356, patterns0 unignored**, existing15 ignores unchanged.


Capture follow-up: reuse the checked lexical environment across functions sharing
one immutable census, with function-local receiver/nominal proofs and separate
strict/generated policies. Alternating repeats reduce the six-context flags test
11.52–11.61→10.55–10.60s (~9%); stored-slice replay stays16.3–16.4s.
All existing admission/matching expectations remain exercised; no capability,
scoring or budget change. Evidence: `/tmp/rstyle-perf10-final-results.json`.

Receiver follow-up: retain immutable typed method inventories instead of repeatedly
cloning/decoding their nominal trees. Alternating unchanged six-context tests:
stored slices **16.33–16.36→14.11–14.16s (~14%)**; flags
**10.41–10.47→9.84s (~6%)**, with slightly lower peak RSS. Full receiver/source
checks remain mandatory; mutation invalidates typed ownership, and malformed
records and explicit nulls stay lossless. Evidence: `/tmp/rstyle-perf11-*`.
Validation: **1067 tests /5 existing skips**, doctests/fmt/Clippy pass;
**rstyle0/3398, patterns0 unignored**, existing15 ignores unchanged.


### Data-v3 3.73: original base64ct length helper

[Source selection and bounded expectations](eval/coverage-development/base64ct-length-selection.json).
Exact builtin bool-to-usize evaluates one saved operand (false→0, true→1).
Direct root-doc include_str inputs retain all bytes and exact pinned compiler,
owner, path and expansion proof; code includes, orphan files and macro imports
retain their existing restrictions. Historical policies gain neither capability.
Three pinned targets, unchanged default contexts: **3→6/141, zero losses**.
The selected encoded_len_inner gains admission; incidental Line::len/is_empty
also gain through their existing shared-slice chain. Original std contexts retain
**3/150**: active macro_use extern alloc remains an unproved source boundary.
Copied std/no_std fixtures do not establish original std-context support.

All **39 contextual original admissions** strictly replay (27 base64ct,12 base64);
**nine predeclared comparisons** complete, including rejection of the incompatible
original length helpers. Their overflow cutoffs differ; matching/scoring is unchanged.
Frozen run-gn15_b0l versus run-72emt3ji retains **23 admitted /30 excluded /7 inactive**;
full lz4_flex8/150, syntax39/956 and time-core11/226 rosters are unchanged, all58
strictly replay. Full-library matching remains explicitly incomplete under max-pairs0.
Native execution is ARM Linux; x86 Linux and ARM macOS evidence is cross-target.
Evidence: `/tmp/rstyle-length-*`. Original std macro imports are the next audited
source-boundary blocker; general casts, Result/? and opaque/drop owners remain open.

Independent unchanged-library oracle: **131074 cases per native feature context**;
**8320 exact private-source witness cases** separately exercise intermediate
and final overflow (52 incompatible peer outcomes). This private-source witness
is not execution of the private helper inside the full package.
**1053 tests /5 existing skips, 127.079s**, doctests/fmt/Clippy pass;
**rstyle0/3371, patterns0 unignored**, existing15 ignores unchanged.

### Data-v3 3.74: original std length/getter family

[Selection and bounded contract](eval/coverage-development/base64ct-std-import-selection.json).
Exact compiler/target/library identities and the complete public alloc macro
inventory (`format`, `vec`) prove the original root import. No macro invocation
permission is added. Source-input closure checks exact owning bytes separately
from nominal type permission, preserving existing nested scalar helpers.
Three targets: std **3→6/150**, default **6/141 unchanged**, zero losses;
encoded_len_inner, Line::len and Line::is_empty now complete both contexts.
All36 base64ct plus12 contextual peer admissions strictly replay; **30 predeclared
comparisons** complete, including rejection of the incompatible length helpers.
Native unchanged-library oracle:131074 cases per feature context; separate
private-source witnesses exercise overflow and getter behavior. Native execution
is ARM Linux; x86 Linux and ARM macOS are cross-target capture/replay.
Frozen `run-pk54tszf` versus `run-gn15_b0l`: **23 admitted /30 excluded /7 inactive**;
complete package rosters remain lz4_flex8/150, syntax39/956, time-core11/226;
all58 strictly replay. Full-library matching remains incomplete under max-pairs0.
**1067 tests /5 existing skips**, doctests/fmt/Clippy pass;
**rstyle0/3398, patterns0 unignored**, existing15 ignores unchanged.
Evidence: `/tmp/rstyle-alloc-*`. Range getters additionally require exact local
From dispatch and non-Copy Range/move/drop proof; Result/?/generic and looping
callee families remain separate active frontier work.

### Capture/replay performance checkpoint

Exact parser byte ranges replace repeated line-prefix scans. A checked-unit
function-span index replaces sibling-function scans during scalar-leaf ownership
lookup; exact byte comparison and ambiguous-owner rejection remain mandatory.
Tests cover Unicode/BOM/CRLF offsets and 16/128/1024 siblings with duplicate and
wrong-range candidates. Debug builds also optimize serde_json, syn2 and
proc-macro2; workspace code retains debug settings. The dependency rebuild took
41.64s locally; release settings and admission contracts are unchanged.
Saved-binary alternating comparisons and final-binary checks: stored slices
14.0–14.1→9.3–9.4s, flags9.8–9.9→7.44s, sine14.1–14.2→12.2s, retaining all
capture/admission/matching assertions. Evidence: `/tmp/rstyle-perf12-*`.
**1069 tests pass /5 existing skips in108.042s**, doctests/fmt/Clippy pass;
**rstyle0/3404, patterns0 unignored**, existing15 ignores unchanged.
Frozen identities/denominators are untouched; no fresh frozen admission count
was measured. Repeated evidence serialization remains a performance target;
Range dispatch/move/drop remains the next coverage blocker chain.

### Report serialization checkpoint

Full JSON stdout now reuses the successfully persisted report instead of
serializing it again. `--report-detail compact` retains every function outcome,
source identity, matching result and complete unit-bundle reference while omitting
inline facts/documents; default full output remains available. Strict bundle replay,
full/compact matching equality and incomplete-budget tests pass.
Equal-work 512-function source-heavy capture: full JSON5.32–5.36→3.91–4.00s;
compact2.58–2.63s, report57.4→0.94MB,
peak RSS198844–198908→165412–165548KiB. All512 admissions remain unchanged.
Actual fixtures, warmup then alternating repeated saved-binary samples:
stored slices9.25–9.39→8.53–8.65s; flags7.38–7.42→7.06–7.16s;
sine12.16–12.17→11.99–12.00s. Evidence: `/tmp/rstyle-report-{bench,equal-work}*`.
Final-binary 512-function recheck:5.31→3.96s, compact2.63s.
Integrated checkpoint: **1072 tests pass /5 existing skips in106.218s**,
doctests/fmt/Clippy pass; **rstyle0/3410, patterns0 unignored**,
existing15 ignores unchanged. Includes observational local-From tests;
no new extraction capability or admission permission is claimed.

### Shared inherent-inventory performance checkpoint

Unit-wide inherent inventories now share immutable storage, validation and an
identity index instead of being cloned/decoded per function. Bundle `/5` stores
explicit references to a single inventory table; historical `/4` import remains
supported. Missing, dangling, duplicate, contradictory and unused entries reject;
callee-specific source checks still run. Full readable JSON remains available.
Warmup plus alternating repeated equal-work captures (128/256/512 functions)
confirm the eliminated function-count × method-count duplication. At512 functions:
**10.91–11.03→4.62–4.67s**, RSS753040–753064→217800–218120KiB,
bundle118178664→21786910bytes, identical admissions/rosters. Unchanged original
regex-automata default capture (one before/after pair): **182.27→105.08s**,
RSS6417656→1047940KiB, bundle1184877865→200210380bytes; all143 admissions and
reasons unchanged. No-default20.69→15.69s; all89 unchanged. Timings are separate
from native CI and do not establish that all remaining costs are linear.
Evidence: `/tmp/rstyle-inherent-{scaling,natural-performance}*`.
Integrated combined checkpoint: **1090 tests pass /5 existing skips in108.282s**,
doctests/fmt/Clippy pass; rstyle and patterns pass with no new ignores. Range
activation and its strict natural/frozen replay are recorded separately.

### Data-v3 3.75: original Range getter family

[Contract](docs/extraction-profiles/fresh-range-results-v1.md) and
[source selection](eval/coverage-development/range-getters-selection.json).
Fresh exact non-Copy `Range<usize>` results now compose through original
`Span::range → <Range<usize> as From<Span>>::from`, `Input::get_range` and
`Match::range`. Pinned library/drop evidence and exact local From selection are
mandatory; Range inputs/storage/field consumption and general traits/moves/drop
remain unavailable. Existing subslice-bound grammar is preserved independently
of standalone Range receiver normalization.
All three targets: no-default **85→89/590**, default **139→143/2582**;
exactly four declared gains per context, zero losses, anchors retained and reverse
conversion excluded. All696 contextual original admissions strictly replay;
**24 selected comparisons** complete and match. Shared-transport native controls
retain all232 admissions and8 comparisons with identical full rosters/reasons.
Independent unchanged-library native oracle: **4704 cases per feature context**,
including endpoint extremes, inverted ranges, done-state and invalid Match panic
controls. Native execution is ARM Linux; x86 Linux/macOS are cross capture/replay.
Integrated gates: **1090 tests /5 existing skips in108.282s**, doctests/fmt/Clippy
pass; **rstyle0/3470, patterns0 unignored**, existing15 ignores unchanged.
Full-library matching remains incomplete under max-pairs0. Evidence:
`/tmp/rstyle-range-{natural-results,evidence}*`, `/tmp/rstyle-range-getters-native-*`.
Broader P07 sequences, P08 Result and P09 looping helpers remain active frontiers.
Frozen `run-tk89n2fv` versus `run-pk54tszf`: **23 admitted /30 excluded /7 inactive**,
no identity/context/outcome changes; complete package rosters remain8/150,
39/956 and11/226. All58 retained admissions independently strictly replay.

### Data-v3 3.76: bounded scalar loop helper composition

[Contract](docs/extraction-profiles/bounded-scalar-loop-helpers-v1.md) and
[source selection](eval/coverage-development/scalar-loop-callee-selection.json).
The unchanged ryu1.0.23 `pow5factor_32 → multiple_of_power_of_5_32` chain now
admits through a complete source/graph rank proof. Positive inputs execute at
most13 backedges; zero panics before arithmetic under the selected checked-debug
settings. Summaries preserve possible panic separately from proven absence of
divergence. General loops, recursion and disabled/unknown settings remain closed.
All three selected targets: **14→15 admitted /40**, exactly the original caller
gains, zero losses. All45 contextual admissions strictly replay;6 selected
comparisons Accepted, plus distinct alpha-peer/transitive-wrapper matching tests.
Independent native private-source witness: **42810 cases**, including5 expected
panics; this is not unchanged full-library execution. Frozen `run-8f3w87gy`
versus `run-tk89n2fv`: unchanged **23/30/7**, package8/150,39/956,11/226;
all58 retained admissions strictly replay. Cross-target capture/replay does not
claim native x86/macOS execution; full-library matching remains incomplete.
Gates: **1096 tests /5 existing skips in110.561s**, doctests/fmt/Clippy pass;
**rstyle0/3496, patterns0 unignored**, existing15 ignores unchanged.
Evidence: `/tmp/rstyle-loop-{gates.log,evidence.log,current-*,oracle.json,frozen-*}`.
P01–P22 remains active. Next audited P03 blocker is pinned primitive associated
constant proof for `u32::MAX` in original ryu::mul_shift_32; P08 separately needs
independent local-owner Drop exclusion before concrete Result activation.
Equal-work control (warmup then alternating samples): unchanged512 admissions,
baseline2.44–2.57s/current2.44–2.48s, RSS about164MiB in both. Shared-host
measurements show no slowdown in this control; they are not a general scaling
proof. Evidence: `/tmp/rstyle-loop-perf/results.json`.

### Data-v3 3.77: primitive constant authority

[Contract](docs/extraction-profiles/primitive-constants-v1.md) and
[selection](eval/coverage-development/primitive-constant-selection.json).
Original ryu1.0.23 `mul_shift_32` now admits with exact pinned `u32::MAX`
authority and independent lexical shadow checks; local constants retain their
existing proof. Three unchanged targets: **15→16/40**, only that target gains,
zero losses; all48 contextual admissions strictly replay,3 original comparisons
Accepted, distinct alpha/transitive peers covered in Cargo tests. Native
private-source bigint oracle: **45668 cases**; unsafe table wrappers remain
excluded. Cross capture/replay does not claim native x86/macOS execution.
Frozen `run-yaapzru3` versus `run-8f3w87gy`: unchanged **23/30/7**, package
8/150,39/956,11/226; all58 retained admissions strictly replay.
Gates: **1106 tests /5 existing skips in115.626s**, doctests/fmt/Clippy pass;
**rstyle0/3513, patterns0 unignored**, existing15 ignores unchanged.
Evidence: `/tmp/rstyle-primitive-{evidence.log,current-*,oracle.json,final-*}`.
Full-library matching remains incomplete. Next priority is the complete selected
UnicodeWordBoundaryError Result chain, then payload binding and propagation;
[replanned lanes](docs/rust-support-execution.md) supersede numeric-leaf ordering.
Equal-work scaling (warmup + alternating repeats,128/256/512 admissions):
512-function baseline2.99–3.01s/current2.98–3.04s, peak RSS about174MiB.
The source namespace scan is computed once per checked unit environment;
no measurable speedup is claimed. Evidence: `/tmp/rstyle-primitive-scaling-final/`.

### Data-v3 3.78: fresh Results with independently proved no-drop errors

[Contract](docs/extraction-profiles/fresh-concrete-results-v1.md) and
[unchanged selection](eval/coverage-development/scalar-result-next-selection.json).
The original UnicodeWordBoundaryError constructor → cfg-selected check → public
check chain now admits across three feature contexts and three targets. Exact
core Result authority, once-evaluated payloads and whole helper forwarding connect
to independent source proof for the non-Copy unit-field error. Relevant imports,
local traits, standard derives, macros and nested items remain checked; unknown
source stays closed. No general moves, payload extraction or `?` permission.
All21 selected instances admit; complete rosters per target **143→148/2582**,
**89→92/590**, **89→91/590**, zero losses. The three extra default-context gains
are source-reviewed CacheError/RetryQuadraticError unit constructors within the
same boundary. All993 contextual admissions strictly replay;21 selected mirror
comparisons complete, with distinct alpha/transitive peers in Cargo tests.
Native unchanged-library oracle passes all three Ok/Err/Ok expectations.
Cross-target capture/replay does not claim native x86/macOS execution.
Frozen `run-tmmfv8_2` versus `run-yaapzru3`: unchanged **23/30/7**;
package8/150,39/956,11/226; all58 retained admissions strictly replay.
Gates: **1132 tests /5 existing skips in113.003s**, doctests/fmt/Clippy pass;
**rstyle0/3666, patterns0 unignored**, existing15 ignores unchanged.
Default native capture108.42s versus recorded115.69s baseline; peak RSS about
1GiB. This single comparison is not a speedup or general scaling claim.
Next: guarded payload ownership and exact same-error `?` for original
[LookSet::available](eval/coverage-development/result-propagation-selection.json).
Its independent native oracle passes4,719,096 cases; extraction remains pending.
P01–P22 remains active; full-library matching is incomplete under max-pairs0.
Evidence: `/tmp/rstyle-result-{evidence.log,current-*,oracle.json,final-*,frozen-*}`.

### Data-v3 3.79: original same-error Result caller

[Contract](docs/extraction-profiles/same-error-result-propagation-v1.md) and
[unchanged selection](eval/coverage-development/result-propagation-selection.json).
Original LookSet::available now connects conditional flag queries to the complete
UnicodeWordBoundaryError check chain through exact same-error `?`. The operand
is evaluated once; Err forwards the saved whole Result to exit and Ok yields
unit. Exact selected core implementations/substitutions and independent no-drop
source proof remain required. Fresh Result method forwarding and nested/sequential
composition are included; general payload binding, stored Results, Option `?`,
other success types/conversions and cleanup remain excluded.
All57 selected contextual chain instances admit across9 contexts: **9 gains,
zero losses**. Per target **148→149/2582,92→93/590,91→92/590**; all1002
current admissions strictly replay and57 selected mirror comparisons complete.
Distinct alpha/transitive Cargo peers and resealed semantic attacks also pass.
Native unchanged-library oracle: **4,719,096 cases, zero mismatches**.
Cross-target capture/replay does not claim native x86/macOS execution. Historical
3.78 artifacts retain their exact policy and pinned verifier; they are not
relabeled for the current3.79-only DataV3 registration.
Frozen `run-rpku9v_n` versus `run-yaapzru3`: identical60 identities/contexts and
**23/30/7** outcomes; full package rosters unchanged, all58 strict admissions.
Gates: **1135 tests/5 existing skips in111.617s**, doctests/fmt/Clippy pass;
**rstyle0/3712, patterns0 unignored**, existing15 ignores unchanged.
The composition test now takes5.4–5.6s in observed runs, down from about22s after
removing repeated unit serialization/source proofs and reusing immutable evidence.
Transport remains streamed xxh3; persistent source identities are cached. Guard
verification shares adjacency/operation indexes rather than rescanning all edges.
These observations are not a general scaling or whole-suite speedup claim.
Next source-selected family: the three original Memchr prefilter prefix methods,
connecting Option reference extraction, Copy aggregate results and exact local
trait-member ownership. The parent core::fmt::Debug import is a demonstrated
source blocker; preserve namespace/derive proof rather than exempting its name.
P01–P22 remains active; full-library matching and held-out quality remain separate.
Evidence: `/tmp/rstyle-propagation-{current-*,green-*,final-*,frozen-*}`.

### Data-v3 3.80: complete checked-reference prefilter prefixes

[Contract](docs/extraction-profiles/checked-reference-option-propagation-v1.md) and
[unchanged selection](eval/coverage-development/prefilter-prefix/selection.json).
Original Memchr/Memchr2/Memchr3 prefix methods now compose checked byte access,
exact Option reference `?`, shared owner reads and independently proved Copy Span
results. None exits with a fresh output-typed value; Some retains the saved
referent and actual dereference read. Exact local trait-member ownership, pinned
Debug trait/derive bindings and closed irrelevant glob proof preserve neighboring
items and type/value shadow checks. No general trait dispatch or owned moves.
Across default/no-default × ARM Linux/x86 Linux/ARM macOS, all18 selected instances
admit: **54 contextual gains, zero losses**. Per target **149→158/2582** and
**93→102/590**. Eighteen gains are selected prefixes;36 are source-audited sibling
memory_usage/is_fast methods returning literal0/true. Their exact source was
already frozen in complete impls; the original selection remains unchanged.
All780 contextual admissions strictly replay;18 original mirror comparisons
complete. Distinct alpha/transitive Cargo peers and resealed semantic attacks pass.
Native original-library oracle checks7,966,720 prefix cases and2564 constructors
per context, plus5120 default sibling-query checks. No-default executes
constructors only, with no private prefix claim.
Cross-target capture does not claim native x86/macOS execution.
Frozen `run-etikyyi5` versus `run-yaapzru3`: identical60 identities/contexts,
**23 admitted/30 excluded/7 inactive**, full package rosters8/150,39/956,11/226
unchanged; all58 admissions strictly replay.
Gates: **1153 tests/5 existing skips in117.738s**, doctests/fmt/Clippy pass;
**rstyle0/3792, patterns0 unignored**, existing15 ignores unchanged.
Composition test4.014s. Default ARM capture138.46s/1.07GiB; changed admitted work
prevents interpreting comparison with the prior capture as an equal-work result.
Next: lz4_flex::block::uncompressed_size needs complete no-drop enum ownership,
differing-success Result propagation, range get/ok_or, borrowed four-byte LE
conversion, suffix views and reference-bearing tuple origins. Cross-package
calendar helpers separately require captured exact dependency/instance closure.
P01–P22 remains active; full-library matching/held-out quality remain separate.
Evidence: `/tmp/rstyle-prefix-{evidence.log,current-*,matching.json,native-*,frozen-*}`.

Follow-on identity work reuses immutable file identities during module/import
proof and memoizes each lexical declaration's exact identity for nominal/cleanup
checks. Published identities and evidence rules are unchanged; no admission gain
or measured speedup is claimed for this increment. Bundle transport remains XXH3.
Validation:1154 tests/5 existing skips, doctests/fmt/Clippy pass; rstyle0/3793,
patterns0 unignored with the existing15 ignores unchanged. Changed source gets a
fresh identity cache; serialized claims cannot populate the memo.

### Data-v3 3.81: complete borrowed Result block-header parser

[Contract](docs/extraction-profiles/checked-borrowed-result-flow-v1.md) and
[unchanged original selection](eval/coverage-development/uncompressed-size/selection.json).
`lz4_flex 0.11.6::block::uncompressed_size` now connects checked prefix access,
eager fresh no-drop error conversion, borrowed Result `?` with a different output
success type, four-byte little-endian conversion and a borrowed suffix tuple.
The complete non-Copy error enum includes its named scalar variant; no Copy or
general destructor permission is inferred. Source ownership retains exact
module/glob/alias bindings, pinned Debug/prelude traits and generated shadows.
Across three feature contexts and ARM Linux/x86 Linux/ARM macOS, all nine selected
instances admit: **9 contextual gains, zero losses**, unchanged identities/contexts.
Per target, default **8→9/144**, no-default **5→6/91**, safe-decode **5→6/96**.
All63 contextual admissions strictly replay; nine selected mirror comparisons
complete. Distinct alpha peers and resealed semantic attacks pass in real Cargo.
Native unchanged-library oracle: **5,787 cases ×3 contexts, zero mismatches**.
Cross-target capture/replay does not claim native x86/macOS execution.
Frozen `run-dwfr5fud` versus `run-yaapzru3`: exact60 identities/contexts and
**23 admitted/30 excluded/7 inactive**; package rosters8/150,39/956,11/226
unchanged. Final verifier strictly replays all58 contextual admissions.
Gates: **1218 tests/5 existing skips in120.927s**, doctests/fmt/Clippy pass;
**rstyle0/3975, patterns0 unignored**, existing15 ignores unchanged.
Reference queries now reuse one owned flow index per immutable function/policy
within the existing retention bound, including cached results when an index is
too large to retain. Foreign nodes and failed queries cannot populate the cache.
Distinct-query scaling and stale-input cycle attacks have explicit regressions.
The parser composition test takes3.311s on this runner; no controlled pipeline
speedup is claimed. Transport/paired documents use XXH3; recorded source pins and
historical SHA manifests retain their existing meanings.
Next: freeze the original Month/calendar dependency chain and oracle, then audit
dependency capture/instance closure, explicit repr/casts, strict helper linking
and owning namespaces before activation. General Result storage/payload binding,
cross-package summaries, generic/trait resolution and memory/cleanup remain open.
P01–P22 remains active; full-library matching and held-out quality remain separate.
Evidence: `/tmp/rstyle-parser-{green-*,final-*,frozen-*}` and
`/tmp/rstyle-uncompressed-{current-*,native.*}`.

### Calendar source-first checkpoint (not activated)

[Selection and oracle](eval/coverage-development/calendar-helpers/README.md);
[complete known blocker chain](eval/coverage-development/calendar-helpers/readiness.md).
Three original `time0.3.47` callers plus five `time-core0.1.8` dependencies are
frozen in six contexts. Native unchanged-library oracle passes **240,216
month/year cases +9 expected debug panics per feature context**, zero mismatches.
Capture/admission are **unmeasured**, with no claimed gain or loss. Frozen
inventories and historical results are unchanged. Next: exact dependency
capture-only checkpoint before cross-unit linking, integer operation contracts,
selected attributes and complete Month repr/Copy/cast/namespace proof.
The three reported CI regressions and the three-target authority regression
pass again on the current implementation; this checkpoint changes no semantics.
Source identities and staged diff checks pass; rstyle **0/3975**, patterns
**0 unignored** with the existing15 ignores unchanged. Full workspace tests were
not rerun for this corpus/documentation checkpoint.

Future milestone reports should state supported complete families, unchanged
selection gains/losses, remaining combined blockers and validation briefly;
keep detailed evidence in linked contracts/artifacts instead of repeating it.

### Selected dependency capture: calendar baseline

`--capture-dependency NAME[@VERSION]` now captures explicitly requested reachable
ordinary dependency check libraries with exact Cargo unit/build evidence. Primary
and dependency inventories stay distinct; duplicate, missing, unreachable,
ambiguous and unsupported compilation contexts fail closed. Foreign calls still
require separate linking proof. No new hashing or serialization lookup path.
The unchanged calendar selection is now classified in all six contexts: **five
selected core helpers admitted, three time callers excluded** in each. All84
contextual package admissions strictly replay. No complete-family gain is claimed;
this is the first classified dependency baseline. Existing arithmetic, cold-path
and core assertion support already suffices; the earlier audit was too pessimistic.
Remaining combined blockers: actual compiled-artifact/instance association and
foreign summaries, Month variant attributes/full repr/Copy/discriminant proof,
its exact variant-glob namespace, then connected owned receiver/cast wrappers.
See [the concise frontier](eval/coverage-development/calendar-helpers/readiness.md).
Gates: **1228 tests/5 existing skips**, doctests/fmt/Clippy pass; **rstyle0/3989,
patterns0 unignored**, existing15 ignores unchanged. Frozen run `run-4t0twce7`
retains all60 selected identities and denominators, with **23 admitted/30 excluded/7 inactive**.
The final verifier strictly replays all58 contextual frozen-package admissions;
complete package rosters retain exact identities/contexts with **zero gains/losses**.
Capture/roster evidence uses an explicit zero-pair budget and does not validate complete matching.

Next coordinated write sets (shared policy/integration owned by the parent):

| Lane | Assignment and gate | Exclusive focus |
| --- | --- | --- |
| A | Record actual rustc crate/loaded-artifact/empty-instance evidence; no local crate-hash query before encoding | Driver association observation leaf and tests |
| B | Join successful Cargo artifacts and exact graph edges; requires A evidence | Capture artifact associations, XXH3 transport seals |
| C | Resolve foreign scalar summaries and strict closure replay; requires A+B, complete selected closure and semantic attacks | UnitStore closure links and explicit external helper proof |
| D | Complete Month owner/variant/representation proof; source audit ready, new capability gated by parent | Fieldless enum capture/checks and exact enum-glob ownership |

Full P01–P22 remains active. Evidence: `/tmp/rstyle-dependency-*` and
`/tmp/rstyle-calendar-capture-{results,strict}.json`; native oracle from the prior
checkpoint remains a separate runtime check.


### Data-v3 3.82: selected cross-package calendar chain

The unchanged eight-function calendar selection now admits in all six contexts:
default/no-default features on ARM Linux, x86 Linux and ARM macOS. The three
`time` callers connect to five `time-core` helpers through exact Cargo, loaded/
emitted artifact and definition associations, plus complete Month repr(u8), Copy,
variant and lexical import proof. [Bounded contract](docs/extraction-profiles/cross-unit-scalar-helpers-v1.md).
Complete package rosters preserve identities/contexts: **18 contextual gains,
zero losses**; strict saved-run replay completes **153 pairs per context**.
Native unchanged-library oracle: **240,216 cases and nine expected debug panics
per feature context**, zero mismatches. Cross-target capture is not native execution.

Transport checks use cached **XXH3-64**, with no new SHA-256 or serialized-body
lookup. Existing source identities/pins remain unchanged and memoized. Dependency
scheduling is linear; diamond dependencies share analysis owners within each
construction. The earlier Result composition test now takes5.674s in the full
suite after removing repeated whole-bundle witness attacks; this is not an
isolated SHA timing or a controlled general speedup claim.

Frozen `run-k16z2tbg` preserves all60 identities/denominators and **23 admitted /
30 excluded /7 inactive**; complete package rosters have zero gains/losses and
all58 contextual admissions strictly replay. Gates: **1273 passed/5 existing
skips**, doctests/fmt/Clippy pass; **rstyle0/4093**, patterns0 unignored with15
existing ignores unchanged. Evidence: `/tmp/rstyle-calendar-final-{results,validation}.json`
and the corresponding six compact capture/strict replay reports.

Remaining combined blockers include concrete generic instance/substitution and
trait implementation/body proof, owned constructor component/origin transport,
foreign reference/aggregate summaries, local borrowing, mutation and cleanup.
Next source-selected gate: original regex-automata `Input::new<H>` and blanket
`From<&H>::from` for `[u8]`/`str`; audit recorded in
[the execution frontier](docs/rust-support-execution.md). No concrete-instance
admission forecast or broad P01–P22 completion claim.

### Concrete instance observation gate

Data-v3 now records requested/selected compiler instances with bounded structural
argument identities and original call-site anchors. Display strings supply no
identity; unresolved, unsupported and compiler-error outcomes remain distinct.
This is observation only: generic definitions, capabilities and admission rules
are unchanged. No new hashing or serialized lookup key is introduced.

The separate [Input selection](eval/coverage-development/input-instances/README.md)
records24 call observations across six unchanged contexts; all owning bundles
strictly import, with **zero admission gains/losses**. The native oracle passes
132,736 constructor observations per feature context and custom effect/origin/
panic controls. Complete constructor ownership and body substitution remain open.

Source audit identifies the smaller [num-traits clamp family](eval/coverage-development/clamp-instances/README.md)
as the next connected generic/trait target, retaining the Input selection.
Its native oracle passes272,852 scalar calls per feature/assertion context;
instance extraction/admission remains pending. Next gates: faithful scalar body
projection, exact primitive comparison proof, generic source correspondence and
instance-aware dependency replay. See [the execution frontier](docs/rust-support-execution.md).
Validation: **1279 tests/5 existing skips**, doctests/fmt/Clippy pass;
rstyle **0/4109**, patterns **0 unignored**, with15 existing ignores unchanged.

### Owner-local scalar instance projection gate

A diagnostic-only owning-compiler pass now projects original scalar generic bodies
without altering the definition roster or overloaded dispatch. All six num-traits
clamp instances project in four native feature/assertion contexts (24 projections,
56 resolved comparison observations); emitted metadata remains byte-identical and
the raw rosters retain 2,212 default / 1,965 no-default definitions. No admission expansion or policy bump;
frozen admission measurements above were not rerun. No new hashing or serialized
lookup keys. Evidence: `/tmp/rstyle-clamp-owner-native-final/summary.json`.

The independent projection verifier now checks complete unchanged source/HIR and
binding correspondence, concrete scalar flow, exact pinned primitive comparison
implementations/signatures and ordered operand adjustments. All 24 fresh original
clamp projections pass across the four contexts; metadata and definition rosters
remain unchanged. Evidence: `/tmp/rstyle-clamp-typed-proof/independent-proof.json`.
Its indexed traversal uses borrowed metadata and preindexed UTF-8 locations;
it introduces no hashing or serialized lookup keys.

Remaining activation gates: exact resolved generic-bound and assertion provenance
tied to the original source/owner,
caller/artifact-bound instance storage, concrete source/frontend bridging and
strict replay into callers. The projection certificate explicitly grants none of
these authorities. No admission expansion or policy bump; the frozen admission
selection was not rerun for this proof-only increment. Next work must connect
these gates into the declared clamp callers rather than add another syntax leaf.

Validation: 1,301 workspace tests / five existing skips and doctests pass
(122.559s nextest execution). Fmt/Clippy pass; rstyle 0 violations / 4,222
functions, patterns 0 unignored; all 15 existing ignores unchanged.

The next increment checks the complete resolved generic-bound roster (including
implicit Sized) and actual assertion macro identities, then binds each checked
projection to its original lexical Cargo/source owner in a separate instance
store. All 24 original clamp projections pass in the same four native contexts;
the original 2,212/1,965 definition rosters and emitted metadata are preserved.
Three-target std/core captures independently exercise the provenance boundary.
The capability remains unregistered: no instance graphs, ordinary export or
dependency linking are authorized. Caller/request/artifact binding and strict
instance replay remain the next connected activation gates; no admission gain
or frozen-inventory recapture is claimed.

An opt-in raw diagnostic adapter reuses the existing shared-source bundle codec
to avoid repeating the whole census per function. The default-context capture
retains all 2,212 functions and 19 source snapshots (233,032 UTF-8 bytes stored
once) in a 45,769,908-byte diagnostic file. This is representation evidence,
not a controlled timing comparison. No new hashing scheme or serialized lookup
key is introduced; the capture runner preserves the existing manifest identity.
Evidence: `/tmp/rstyle-clamp-provenance-owner/{summary,owned-proof}.json` and
`/tmp/rstyle-instance-provenance-typed/`.

Validation: the full run passed 1,318 tests with only the workspace style gate
failing; after fixing it, all 48 affected checks and doctests pass. Fmt and
warning-denied Clippy pass; rstyle reports zero violations across 4,271
functions, patterns zero unignored candidates, with all 15 ignores unchanged.
Comparison-site traversal reuses one stack instead of copying a path per node.

### Data-v3 3.83: original scalar generic instances through callers

Original num-traits0.2.19 `clamp`, `clamp_min`, `clamp_max` at i32/u32 now
connect owning-compiler substitution, independently checked source/bounds/
primitive comparison/assertion evidence, private instance owners, exact Cargo
artifact/call binding and strict replay. All 24 original instances admit across
four native feature/assertion contexts. Original generic definitions retain
source and ordinals in unchanged 2,212/1,965 definition rosters; instances are
separate owners. All 48 contextual caller fixtures admit, four u8 controls remain
excluded, and 264 pair evaluations complete, including all 24 identical peer pairs.
These are current observations, not a fresh measured revision/frozen-inventory
delta. [Contract and runner](eval/coverage-development/clamp-instances/README.md).

Dependency selection now distinguishes explicit build-script compile/run units
from the library while still rejecting multiple reachable library contexts.
Generic functions with different result shapes remain excluded without breaking
complete capture. Ordinary bundles cannot manufacture instance/caller authority;
missing/duplicated/unknown projection inventories and resealed source, substitution,
owner, artifact and provenance attacks remain closed. No general trait, reference,
aggregate, generic-method or cleanup support is claimed.

Native original oracle evidence remains 272,852 cases/context with 4,921 expected
assertion-enabled panics and six custom trait controls; exact selected source
bytes are rechecked by the composition runner. Evidence:
`/tmp/rstyle-clamp-connected/{summary,owned-replay}.json`. Captures 5.4–6.0s and
strict closure replays 1.9–2.1s locally; these are timings, not controlled speedups.
A profiled replay spends 1.6ms hashing source snapshots, versus 2.10s handoff;
there is no new SHA or serialized lookup key. The focused Cargo instance
regression initially runs 0.68s. Native macOS/x86 and fresh frozen comparison
remain pending. Next: the original Input constructor/From instance chain through
concrete AsRef evidence, complete result ownership and component-specific foreign
summaries, with all known blockers retained in the execution frontier.

Local integrated gates: 1,338 tests / five existing skips and doctests pass
(123.183s nextest). Fmt and warning-denied Clippy pass; rstyle zero violations
across 4,304 functions; patterns zero unignored candidates, with all 15 existing
ignores unchanged. The complete Cargo instance regression runs 0.67s.

The next owning diagnostic gate observes all eight selected original Input
new/From instances across native default/no-default contexts, with exact original
source/binders/inherited clauses, AsRef implementation/receiver evidence and
From-to-constructor association. Definition rosters remain2,582/590. Independent
verification, actual complete-module cleanup, outer From trait/impl proof and
owned/component foreign summaries still gate admission; test-only cleanup
witnesses cover the full modeled product and resealed Drop/Copy attacks.
No capability or admission expansion. [Runner](eval/coverage-development/input-instances/capture.py);
evidence: `/tmp/rstyle-input-reference-owners-final/summary.json`. Fresh frozen
`run-0qhwddyy` preserves all60 identities/denominators and23/30/7 outcomes versus
`run-etikyyi5`;116 graph documents pass schema checks, without an additional
strict-replay claim.

Local integrated gates: 1,349 tests / five existing skips and doctests pass
(122.802s nextest). Fmt and warning-denied Clippy pass; rstyle zero violations
across4,340 functions, patterns zero unignored candidates, all15 ignores unchanged.
The real Cargo reference diagnostic regression runs0.25s; no admission gain is claimed.

Independent reference proof now checks all four original Input new/From owner
observations in both native feature contexts, the complete constructor saved-value
flow and lexical nominal paths, and actual full-source cleanup of the entire
Input/Span/Anchored/PatternID/SmallIndex product. Unrelated lexical wildcard imports
no longer poison this separate proof; generated imports, Drop aliases and historical
cleanup restrictions remain guarded. No admission activation or new SHA/serialized
lookup keys. Opaque compiler type IDs, outer From selection, owning instance links
and component summaries remain prerequisites. Native oracle rerun:132,736
observations/context, zero mismatches. Full-source proof evidence:
`/tmp/rstyle-input-independent{,-no-default}-proof.log`.

Integrated checkpoint:1,385 tests pass/five existing skips, doctests/fmt/Clippy
pass; rstyle zero violations/4,496 functions and zero unignored patterns, all15
ignores unchanged. Existing Result composition takes5.816s in this run; no
controlled speedup or fresh admission measurement is claimed.


Data3.84 adds component-specific local helper results for complete memory-free
functions returning nested tuples/named products with shared-reference leaves
from shared formals. By-value reference-bearing formals and foreign component
summaries remain outside this contract. Each selected return component independently agrees between source origins and
serialized value operands; saved actuals, later rebinding, nested projections and
reads compose through real Cargo capture, strict replay and all55 fixture pairs.
Omitted/duplicate/wrong/spliced summaries and operand swaps remain rejected.
This is Input-chain mechanism coverage, not a newly admitted natural family.

The original Input proof now checks the selected outer From trait/method/impl
and exact constructor substitution in default/no-default slice/string captures.
A private constructor projection normalizes only the proved formal and reads,
preserving original source findings, generic resolution and nonformal types.
Next activation must connect the exact AsRef operation/source finding, concrete
struct slots and complete mixed Anchored type to an owning reference instance;
foreign aggregate linking remains separate. No new SHA or serialized lookup keys.

Integrated validation:1,406 tests/five existing skips, doctests/fmt/Clippy pass;
rstyle zero violations/4,543 functions; patterns zero unignored candidates,
all15 ignores unchanged. Composition1.583s, with no controlled speedup claim.
Fresh frozen `run-8o5jdd1c` matches `run-0qhwddyy`: all60 identities/denominators,
23 admitted/30 excluded/7 inactive, zero gains/losses and116 schema-checked graph
documents; no additional strict frozen replay claimed. Full original native
proofs: `/tmp/rstyle-384-original-{default,no-default}-proof.log`. Broad goal active.

### Data3.85: original checked byte constructor

The unchanged original `Input::new::<[u8]>` now admits in default/no-default
native contexts, with strict owning sidecar replay, producer admission, complete
identity matching and tag/literal/origin/summary/historical-policy attacks.
Each fixed four-instance selection gains one admission/zero losses; the string
constructor and both From instances remain unsupported. Original generic rosters
remain2,582/590; no foreign caller admission is claimed.

The exact AsRef source finding stays visible. Its checked summary saves one
receiver and preserves its shared byte origin; complete Input/Span/Anchored
products and no-drop ownership are required. Immutable source proof is reused
after minting; export/replay independently recheck it. No new SHA operation,
serialized lookup key, scoring change or ignore was introduced.

Gates:1,423 tests/five existing skips, doctests/fmt/Clippy pass; rstyle0/4,600,
patterns0 unignored/15 unchanged ignores. Component composition1.568s. Fresh
original native oracle132,736 observations/context, zero mismatches. Evidence:
`/tmp/rstyle-385-{final-*,input-capture,fresh-*-activation.log,input-oracle.json}`.
Next: explicit string-to-byte origin typing, complete From materialization and
instance/caller linking, then the next source-selected error/data chain. The broader goal
and full-library matching remain incomplete.

Fresh frozen `run-mj68hs_m` matches `run-8o5jdd1c`: all60 source identities and
denominators,23 admitted/30 excluded/7 inactive, zero gains/losses and116
schema-checked documents. This does not claim an additional strict frozen replay.

### Data3.86: original string constructor

`Input::new::<str>` now strictly admits/replays with an explicit whole-string
UTF-8 byte-view projection and independent returned-component expectations.
Each unchanged native four-instance selection gains1/loses0, now2 admitted/2
unsupported; both From callers still need checked body/dependency linking.
Fresh complete rosters remain2,582/590. Native oracle132,736 observations/context;
strict producer admission, identity matching and graph/policy attacks pass.
Gates1,427 tests/five skips, doctests/fmt/Clippy, rstyle0/4,605, patterns0/15
unchanged ignores. Composition1.546s. Frozen `run-6jdco7wi` versus `run-mj68hs_m`:
all60 identities/denominators unchanged,23/30/7, zero gains/losses,116 schema-checked
documents. Evidence: `/tmp/rstyle-386-*`; no additional strict frozen replay,
cross-target activation, foreign-caller gain or broad completion is claimed.


### Data3.87: original From callers and constructor dependencies

The same four original regex-automata0.4.18 Input instances now connect through
checked constructor/From owners, exact same-substitution helper links and
component origins. Fresh native default/no-default Cargo-owner probes admit4/4
(previously2/4), with2 gains and0 losses/context; original generic rosters remain
2,582/590. Both proofs replay strictly and pass producer admission and complete
identity matching. Ordinary foreign reference/component linking remains pending.
Private exact-call authority preserves unresolved raw dispatch and complete
source/trait/impl/no-drop proof. Sidecars carry typed nested proof inputs without
JSON round trips; replay sessions reuse their inspected immutable original owner.
No new hash operation, serialized lookup key, threshold or ignore is introduced.
Tampering with substitution, owner, definition, callee, dependency or historical
policy rejects. Evidence and final checkpoint results: `/tmp/rstyle-387-*`.

Next delivery: original base64ct `Line::slice_tail → len`, independently frozen
in `eval/coverage-development/line-tail/`: eager scalar ok_or, checked subtraction
Result payload `?`, suffix get, actual fieldless Copy error/import ownership and
borrowed-result origins. The public Decoder chain still needs loops/mutation.
Foreign Input linking needs production instance/artifact inventory and caller
component authority; audited foreign originals additionally require substantial
search/state foundations. Keep that lane separate from natural admission claims.
The fixed sixty and all existing selections remain unchanged.

Checkpoint:1,442 tests/five existing skips and doctests pass; fmt/Clippy pass;
rstyle0/4,641, patterns0 with15 existing ignores unchanged. Fresh frozen
`run-26yohht4` preserves all60 identities/denominators and23/30/7 against
`run-6jdco7wi`, with116 schema-checked documents and0 gains/losses. Native Input
oracle132,736 observations/context has0 mismatches. Four original/replayed identity
comparisons/context pass. Fresh complete probes take136.63s/default and20.22s/no-default;
these include additional From work and do not establish a controlled speedup.

### Data-v3 3.88: complete checked Line tail error flow

Original base64ct1.7.3 `Line::slice_tail → len` now connects checked subtraction,
eager Copy-error conversion, scalar same-error `?` and checked suffix access.
Actual Error source/import/Copy proof and non-Copy RangeFrom carrier authority
remain separate. Unchanged default/std selections on the three pinned targets
each gain1, lose0, reaching2/2; full rosters stay141/150. See the bounded
[contract and acceptance probe](eval/coverage-development/line-tail/README.md).
Borrowed Result-returning helper wrappers and public Decoder mutation/loop chains
remain unsupported; this does not establish general Result payloads or ownership.

Checkpoint:1,466 tests/five existing skips and doctests pass; fmt/Clippy pass;
rstyle0/4,709, patterns0 with15 existing ignores unchanged. All six complete
original bundles strictly import;12 selected identity comparisons accept and48
resealed semantic attacks reject. Native exact private-source witness remains
1,209,654 queries/context with0 mismatches; cross-target capture is not native
macOS/x86 execution. Fresh frozen `run-9bjlp2ny` preserves all60 identities,
every denominator and23/30/7 against `run-26yohht4`;116 graph documents pass
schema checks (not an additional frozen semantic replay). Evidence:
`/tmp/rstyle-388-*`. Existing parser/fieldless-enum regressions pass; Copy-error
classification requires the actual Result error type, not any neighboring enum.
The new composition test takes1.071s; parser regression3.537s with prebuilt driver.
Standalone test-side driver compilation is separate; no controlled speedup claimed.

Next source-audited ownership bridge: original `LineReader::new_unwrapped` needs
fresh non-Copy product Result ownership, nested scalar Option capture/no-drop
proof and guarded component origins. Its public Decoder callers still require
loops/mutation/dispatch. Also remove demonstrated full-report duplication from
the inventory harness using existing compact reports and complete bundle evidence.

### Data-v3 3.89: fresh reference products in Result

Original base64ct1.7.3 `LineReader::new_unwrapped` now admits with its actual
Clone-only owner, scalar Option field, checked Copy Error and guarded backing
origins. Default/std selections on ARM Linux, x86 Linux and ARM macOS each
move0/1→1/1, gain1/lose0; complete original rosters stay141/150. The bounded
[contract](docs/extraction-profiles/fresh-reference-product-results-v1.md)
keeps product storage, moves, helper export and aggregate `?` unavailable.
Exact census-proved alloc macro declarations are reused without omitting
neighboring items or granting arbitrary expansion permission.

Checkpoint:1,483 tests/five existing skips and doctests pass; fmt/Clippy pass;
rstyle0/4,743, patterns0 with15 existing ignores unchanged. Six complete original
bundles strictly replay; six selected identity comparisons accept and36 resealed
semantic attacks reject. Native private-source oracle:9,234 cases/default and/std
on ARM Linux,0 mismatches. Composition test0.878s with prebuilt driver; compilation
is separate, and no controlled speedup is claimed. Evidence: `/tmp/rstyle-389-*`.

Next complete caller is `LineReader::new_wrapped`: guarded product Result helper
summaries → exact same-product/same-error `?` → one initialized nonescaping local,
Copy Option field update and terminal whole move. Confirm MIN_LINE_WIDTH's exact
constant capture and generated Try selections before activation. This advances
callable summaries and normal ownership; public Decoder loops/decoding/memory
remain separate. No new production hash operation or serialized cache key.

Fresh frozen `run-49q4l17t` preserves all60 identities, outcomes, reasons/build
settings and denominators versus `run-gp8bnmsf`:23/30/7,0 gains/losses;
116 documents pass schema checks,21 Python tests pass. This adds no frozen
strict semantic replay claim.

### Data-v3 3.90: guarded product Result callers and affine updates

Original base64ct1.7.3 `LineReader::new_wrapped → new_unwrapped` now composes
same-unit guarded summaries, exact same-product/error `?`, one initialized
nonescaping no-drop local, Copy scalar/Option field replacement and terminal move.
Six unchanged default/std target selections each gain1/lose0 (0/1→1/1), complete
rosters141/150 unchanged. [Contract](docs/extraction-profiles/linear-product-result-flow-v1.md).
Six strict replays/identity comparisons and108 resealed attacks pass; independent
ARM Linux original-source oracle64,638 cases/context has0 mismatches. Full suite:
1,497 tests/five existing skips plus doctests; fmt/Clippy, style0/4,843, patterns0
with15 existing ignores unchanged. Expanded composition1.646s; no new production
hash or serialized key. Frozen `run-dm2kys18` preserves all60 identities, outcomes,
blockers/build settings and denominators against `run-49q4l17t`:23/30/7,116 schema
checks; no new frozen strict replay claim. Evidence:`/tmp/rstyle-390-*`.

Foreign mixed-product linking, general payload matches, generic/trait selection,
mutable memory, escaping storage and drop/unwind remain separate. Next source
readiness audit prioritizes exact `Match::new<Span/Range<usize>>` Into→From chains;
error-payload and mutable-formal alternatives remain in the execution frontier.
Broader P01–P22 work and native x86/macOS execution checks remain outstanding.

Match preparation now freezes both concrete substitutions in a separate
[development corpus](eval/coverage-development/match-construction/README.md).
Original-library oracle passes default/no-default (22,475 cases/context);
no-default owning diagnostics retain590 definitions and both concrete conversion
observations. This grants no admission. Remaining chain: active Copy/source
closure → verified blanket Into→From forwarding → complete scalar-product and
assertion composition → affine Range consumption. Data3.90 Linux ARM/x86 native
CI passed; macOS Tests failed, with detailed failure output still unavailable.

Data3.91 connects the separately owned original `Match::new<Span>` through
production, serialized verification, strict matcher admission and sidecar replay
on retained ARM Linux, x86 Linux and macOS captures (590 definitions each).
[Bounded contract](docs/extraction-profiles/product-constructor-instances-v1.md).
Raw generic/projected facts still reject; ordinary caller gains and the frozen
inventory have not been reassessed here. Both selected substitutions remain:
Range still needs its consuming formal, ordered endpoint projections, verified
local From body and an Into-to-From dependency adapter. Complete selected matching,
fresh six-context captures and native execution remain pending; this is an
owning-instance boundary, not completion of the two-instance family.

Data3.92 connects the second selected private owning instance,
`Match::new<Range<usize>>`, through the certified original local From body,
ordered scalar endpoint reads and once-only non-Copy/no-drop Range transfer.
The original Into CALL retains its actual dispatch and an exact From dependency;
it never receives Span's identity summary. Both selected instances pass strict
production, serialized verification, admission and replay on the three retained
590-definition owners (ARM Linux, x86 Linux, macOS), including resealed CALL
operator/callee/construct attacks. Owning-instance selection gains1/loses0 versus
3.91; ordinary callers and frozen admissions have not been reassessed. Historical
3.91 matching remains routed without the new capability. No new SHA computation,
serialized lookup key, Copy permission or general destructor contract is added.
Fresh six-context captures, complete selected matching and native foreign-target
execution remain pending. The two-instance family and broader P01–P22 goal remain
incomplete; this is a connected owning-instance checkpoint.
Checkpoint validation:1,559 workspace tests/five existing skips plus doctests;
fmt/Clippy pass, style0/5,163 and patterns0/15 unchanged ignores. Evidence:
`/tmp/rstyle-395-*`; source-only oracle passes, fresh native oracle not rerun.


Fresh Data3.92 follow-through: both selected private Match instances now pass
strict verification and all six original/replay pairs on fresh default/no-default
ARM Linux, x86 Linux and ARM macOS captures (590/2,582 definitions). Native
original-library oracle passes22,475 cases per feature context; native foreign
target execution and ordinary generic caller linking remain outstanding.
The source-index/nominal-error retention increment preserves matching outcomes
and reduces controlled equal-work native no-default replay16.27–16.79→12.14–12.65s.
No admission gain is attributed to performance work; frozen counts remain
unreassessed. [Evidence and remaining checksum migration](docs/rust-support-execution.md).


Routine source consistency now uses existing XXH3 for fresh Data-v3 function
claims and fresh manifest/file/declaration claims, with explicit legacy SHA
verification and unchanged reviewed pins/r0 conformance. No semantic capability
changes. Frozen sixty remain23 admitted/30 excluded/7 inactive (gain0/loss0);
retained SHA and fresh XXH3 native Match owners preserve all six strict selected
matching outcomes. [Current evidence](docs/rust-support-execution.md).

Mutable backtracking now has independently checked formal/access evidence
(five parameters/eight accesses/ten conservative alias pairs), without mutable
admission or a new profile capability. All1,578 tests pass; style/patterns remain
zero with15 existing ignores unchanged. Next connect cyclic memory dependencies
and explicit normal/failure post-state through strict replay; the distinct-cursor
oracle does not prove captured disjointness. D20/D21 remain missing.
[Contract](docs/extraction-profiles/mutable-scalar-formals-v1.md).

Data3.93 admits complete unchanged safe `lz4_flex0.14.0::backtrack_match` with
alias-conservative loop memory dependencies and explicit checked-failure
post-state. Frozen60:24 admitted/29 excluded/7 inactive, gain1/loss0, unchanged
identities/denominators; native150-definition owner passes strict original/replay
matching (one Accepted pair, score1). D20/D21 become Partial within the direct
`&mut usize` compound-write contract; stateful helper summaries, local storage,
reborrows, plain/field/indexed writes and general cleanup remain unfinished.
[Contract and validation](docs/extraction-profiles/mutable-scalar-formals-v1.md);
[next connected caller frontier](docs/rust-support-execution.md).


Time error-family readiness now captures exact concrete core From/TryFrom members
and the nominal Error projection in both original parsing contexts, plus a
diagnostic Copy/static-str source proof. No admission/profile capability changes.
Full-owner evidence exposes foreign globs and generated/multi-arm macro closure
before mixed Result/payload/origin activation; the four-function family remains
unavailable. [Selection, expectations and blocker chain](eval/coverage-development/time-error-conversions/README.md).

Data3.94 connects unchanged `adler2 2.0.1::Adler32::new → Default::default`
through exact core/local trait selection and existing verified Copy-product
helper summaries. All six std/no_std target contexts gain both originals and
lose none; complete owners retain17/16 definitions. Twelve strict original/replay
comparisons pass; independent native public/field/dispatch expectations pass.
Frozen60 remains24 admitted/29 excluded/7 inactive, unchanged identities and
denominators. All1,618 tests/five skips, doctests, fmt/Clippy, style0/5,449 and
patterns0/15 unchanged ignores pass. Generic/drop-sensitive/default-provided
bodies remain outside the [bounded contract](eval/coverage-development/adler-default/README.md).
No new SHA or serialized lookup keys. Broader P01–P22 coverage remains active;
[next connected constructor frontier](docs/rust-support-execution.md).

Data3.95 replaces the coarse expansion barrier for exact ordinary nongeneric
Copy-owner methods with complete source/cleanup receipts. Original native no-
default SmallIndex owners gain six admissions (103→109/590), lose none;
`one_more → as_usize` composes. Primitive aliases/import ambiguity, generated
callables and unknown cleanup remain closed. Six strict identity matches and
all15 selected original pairs finish; frozen60 identities/denominators/outcomes
remain24 admitted/29 excluded/7 inactive. All1,624 tests/five skips, doctests,
fmt/Clippy, style0/5,480 and patterns0/15 unchanged ignores pass. Checked
construction remains incomplete behind MAX, conversion and error transport;
[frontier and bounded evidence](docs/rust-support-execution.md).

Data3.96 connects exact local nongeneric `usize → u64` value-trait methods to
their verified bodies, including the reviewed 64-bit `u64::try_from(self).expect`
chain. The real error is `TryFromIntError`; totality follows from the selected
implementation, with the literal message retained as an evaluated operand.
Native unchanged owners gain `Usize::as_u64` (109→110/590, no losses); frozen60
remains24/29/7. General trait dispatch, arbitrary Result consumption and checked
SmallIndex construction remain incomplete. [Validation and next blockers](docs/rust-support-execution.md).

Data3.97 adds a bounded fresh `Result<CopyScalarProduct, NamedScalarError>`
boundary, with independently proved original no-drop error ownership and captured Result
error membership. Standalone owned constructors retain their restrictions. Explicit
early error returns and whole helper forwarding compose; owned storage, reuse and
payload binding remain outside this boundary. The unchanged natural owner stays
110/590 admitted with zero gains/losses: MAX evidence and exact TryFrom/source
ownership still block checked SmallIndex construction. This is a prerequisite,
not completion of that family. [Validation/frontier](docs/rust-support-execution.md).

Data3.98 connects original `SmallIndex::new → TryFrom<usize>` through MAX,
its checked scalar-product initializer and complete success/error helper chain.
The unchanged no-default owner gains exactly those two functions:110→112/590,
zero losses. Default-context strict replay also passes. Generated `PatternID::new`
still requires expansion ownership, nested products/errors and consuming constructor
maps. [Contract](docs/extraction-profiles/checked-index-construction-v1.md);
[validation and frontier](docs/rust-support-execution.md).

Generated-callable preparation retains Data3.98: opt-in raw expansion/node/constructor
observations and independently checked original macro substitution grant no admission.
Unchanged native selection remains112/590, gain0/loss0. Next join exact emitted
members and hygiene, nested success/error ownership, and once-only consuming
`Result::map/map_err` through whole original `PatternID::new`.

Data3.99 connects unchanged generated `PatternID::new` through the complete
checked SmallIndex/helper/constructor-map chain, with separate Copy success and
non-Copy no-drop error evidence. Native unchanged rosters gain only that function:
112→113/590 no-default and158→159/2,582 default, zero losses. Six target/feature
contexts strictly replay ten selected originals and complete all45 original pairs;
native oracle8,202 public cases per feature/assertion context has zero mismatches.
[Contract](docs/extraction-profiles/constructor-result-maps-v1.md) and
[validation/frontier](docs/rust-support-execution.md). General payload binding,
owned Result reuse/forwarding and sequential unknown macro lookup remain open.
Frozen60 remains24 admitted/29 excluded/7 inactive with identical identities,
denominators and blockers;118 schema checks pass. All1,773 tests/five existing
skips, doctests, fmt/Clippy, style0/6,025 and patterns0/15 unchanged ignores pass.
No new SHA or serialized lookup keys; unrelated source lookups avoid the eager
whole-HIR transport scan. Broad P01–P22 work remains active.

Data3.100 connects checked direct dependency owners to the exact original
anonymous `num-conv` prelude footprint. Constructor/getter/helper composition,
matching and strict replay pass in development; all10 original `time` import
footprints prove. The four selected error conversions still exclude behind
value/ownership and payload-shape blockers; no natural-family admission gain is
claimed. Frozen60 remains24/29/7 with identical identities, denominators and
reasons. [Contract](docs/extraction-profiles/anonymous-dependency-namespaces-v1.md);
[validation and remaining chain](docs/rust-support-execution.md).

Data3.101 connects supplied static-string Copy product constructors, ordinary
callers and saved-reference composition through existing aggregate/component
origins. Exact original lifetime, recursive Copy and exhaustive source proof are
required; erased reference tokens grant nothing. Development composition admits,
strictly verifies and matches. The unchanged natural time/num-conv roster remains
6/1,363 admitted, gain0/loss0: the two selected constructors now reach the unresolved
foreign generated export/source boundary. Complete conversion-family admission
still requires that source chain and nominal payload/member summaries.
[Contract](docs/extraction-profiles/static-string-copy-products-v1.md) and
[validation/frontier](docs/rust-support-execution.md).

Data3.102 adds exact constructor-site proof for unchanged
`ComponentRange::{unconditional,conditional}`. Positive Copy proves target
cleanup; separately checked complete source/HIR bodies prove only their own
namespaces. Neighboring functions retain source/effect barriers. The unchanged
1,407-definition development selection gains exactly those two admissions:
20→22, zero losses; strict replay preserves outcomes/reasons and completes all231
pairs. This does not complete the error-conversion family: tag-specific payload
origins, nominal Result flow and selected trait-member summaries remain connected
prerequisites. [Contract](docs/extraction-profiles/copy-constructor-sites-v1.md).

The next error-conversion prerequisite preserves reference-bearing tagged
observations and proves the two original time From/TryFrom members in both native
feature contexts. Guarded values and caller summaries remain closed. The unchanged
selection stays22/1,407 admitted; strict replay completes231 pairs and frozen60
is unchanged. [Validation and the connected frontier](docs/rust-support-execution.md#tagged-copy-conversion-prerequisite-data3102-no-activation).

Data3.103 connects the two original time Copy conversion methods through exact
member/source ownership, saved tagged inputs, guarded payload bindings and
component-specific reference origins. The nominal unit error remains distinct
from `()`, with an explicit empty-reference failure alternative. Fresh native
default/parsing capture on the unchanged 1,407-function selection gains exactly
the original `From` and inverse `TryFrom`: 22→24 admitted, zero losses, unchanged
remaining outcomes/reasons, and complete matching. Caller summaries still require
actual/formal substitution and caller-scoped guards; parser callers additionally
require their concrete instance, callback and Result composition dependencies.

Data3.104 carries those guarded conversion contracts through exact same-unit
explicit From/TryFrom calls and immediate whole-value forwarding. Saved actuals,
caller-scoped guards and complete callee summaries verify independently; flattened
origins and forged summaries reject. Wrappers are not themselves callable summaries.
The unchanged natural selection remains24/1,407 admitted, gain0/loss0, with all276
pair results unchanged under strict replay. This is a callable-composition
foundation, not admission of the production parser above these conversions.
The next source-selected frontier is time's three concrete ExactlyNDigits parse
methods: complete slice-prefix binding capture, exact suffix origins and
Option<ParsedItem<u8/u16>> product substitution/no-drop proof. Their method
identities are already concrete; generic-function instantiation is not required.

Data3.105 records the original digit-prefix slice/@range/rest nodes, exact binder
inventories and distinct ParsedItem<u8/u16> observations, retaining the generic
declaration source and its origin. Independent source checks reject changed
ranges, binding modes, scopes and child identities. This is capture/source proof
only: value admission, scoped no-drop proof and guarded component origins remain
closed. The unchanged selection stays24/1,407 admitted, gain0/loss0; all276 pair
results and all selected source identities are unchanged. The original runtime
oracle passes229,554 checks in each default/parsing and no-default/parsing context.

Data3.106 checks complete generated dependency namespaces against original source,
the full emitted member roster, exact exports and consumer imports. Lint-only
expectation metadata preserves neighboring items and expansions. The unchanged
development selection remains24/1,407 admitted, gain0/loss0, with all276 matching
results preserved under strict replay. The digit-parser family remains incomplete:
its next blocker is cleanup ownership of the original `ensure_ranged` expansion;
this import proof grants no callable or no-drop permission.
Validation:1,939 tests pass/five existing skips plus doctests; fmt, warning-denied
Clippy,style0/6,553 and patterns0/15 unchanged ignores pass. Strict evidence:
`/tmp/rstyle-430-namespace-replay.json` and `namespace-strict-comparison.json`.
After three foundation increments without gains, reassess after one bounded
whole-owner macro check; powerfmt association and connected value/origin/graph
proof remain required before any digit-family activation.

Data3.107 proves bounded source-owned declarative macro cleanup across exact local
imports/reexports. Ordered whole-rule matching preserves typed-fragment opacity,
compound punctuation and complete selected output; every emitted item reenters
the exhaustive cleanup visitor. Unknown/ambiguous rules, shadowed imports,
unsupported editions, external roots and resource exhaustion remain unproved.
This supplies no generated-callable, value or purity permission. Historical
cleanup grammar remains unchanged; strict mixed-unit tests retain generated
callable exclusions and the separate nominal-argument boundary.
The unchanged natural selection remains24/1,407 admitted, gain0/loss0; strict
replay preserves all276 pair results. Evidence:
`/tmp/rstyle-430-ownership-{natural,strict,comparison}.json`.
The next complete-owner check must join the original powerfmt producer/export
and grouped-self consumer import before connecting the pending exact product,
slice-prefix, saved-suffix and Option value graph. No digit-family support is
claimed; further source-only increments require this concrete blocker chain.
Validation:1,984 tests pass/five existing skips plus doctests; fmt, warning-denied
Clippy,style0/6,626 and patterns0/15 unchanged ignores pass. Retained gate logs:
`/tmp/rstyle-430-ownership-{fmt,clippy,style,patterns,nextest}8.*`.


Data3.108 binds the original powerfmt expression macro through its complete
producer/export/reexport and grouped-self consumer import. Every source argument
remains visited; this grants no callable or value/effect permission. Exact
comparison/Hash imports, bounded inert metadata and original same-module enum
globs retain historical, shadowing and hidden-Drop barriers. Routine checks use
cached XXH3; the Hash reexport adds no runtime source hashing.
Strict replay keeps all1,506 identities (the original1,407 plus99 separately
captured dependency definitions),24 admissions and276 matching results unchanged:
gain0/loss0, all completeness checks true. Original digit-parser source probes
now reach unresolved nested import/declaration shadowing in parsing::component;
the family remains excluded. Saved-prefix values, constructor/Option flow,
component origins and independent graph replay are still unactivated local work.
Further work must complete that connected chain rather than add source-only rows.
Evidence: `/tmp/rstyle-431-landing-{replay,comparison}.json` and replay stderr.
Validation:2,043 workspace tests pass/five existing skips plus doctests; fmt,
warning-denied Clippy,style0/6,709 and patterns0/15 unchanged ignores pass.
Retained logs: `/tmp/rstyle-431-landing-{nextest,clippy,style}-final.*`,
`/tmp/rstyle-431-landing-patterns3.json`. Foreign native CI remains unverified.


Data3.109 connects original time0.3.47 `ExactlyNDigits::<1/2/3>::parse`
in the default/parsing context through Cargo capture, independent document/producer
verification and strict admission. The bounded contract joins exact concrete ParsedItem<u8/u16>
construction/no-drop ownership, ordered length/digit guards, distinct selector
and body byte reads at identical physical locations, saved suffix components,
Option success/failure values and checked decimal intervals (0..9/99/999).
Neighboring source, macros and trait implementations remain exhaustively checked;
new cleanup identity rules require the new policy. No generic callable-summary,
N=4..9, parse_subsecond caller, rewrite-safety or detector-quality claim follows.

The unchanged default/parsing selection gains five functions: the three parsers
and the source-audited ComponentRange::{name,is_conditional} getters. Admissions
24→29/1,506, zero losses; all276 historical pair results are unchanged and all406
current pairs complete under strict replay. The unchanged no-default/parsing
selection remains22/1,406, gain0/loss0, preserving all231 pair results. Its distinct
lockfile resolves num-conv0.2.2 (default uses0.2.0): the three-export prelude remains
outside the independently proved two-export namespace contract. This is a named
remaining source dependency, not permission to ignore the glob or alter selection.
Original runtime expectations pass229,554 checks per feature context; the Cargo
composition includes graph/origin/guard/policy attacks and coherent owner tampering.
Final gate evidence and next frontier are recorded in docs/rust-support-execution.md.

Data3.110 independently closes num-conv0.2.2's complete three-trait anonymous
prelude (Extend/Truncate/Widen), including bounded literal deprecation metadata,
all original grouped/leaf coordinates and exact compiled routes. Historical
policies retain the two-trait contract. Cargo capture, strict replay and matching
exercise the constructor/getter family; altered routes, identities, metadata,
coordinates, hidden items and unsupported producer evidence remain rejected.
The unchanged no-default/parsing selection remains22/1,406, gain0/loss0; the
previous prelude rejection advances to powerfmt0.2.1 macro source ownership.
The latter changes the alternate expansion and requires a separate reviewed
contract. Deranged0.5.9 also differs from the default lockfile; downstream source
closure remains unproved. Option<ParsedItem> callable summaries remain separate.

Data3.111 closes the next no-default source dependency with a separately gated,
exact powerfmt0.2.1 root/module contract, retaining all14 rules and every value/
option expression. Mixed-release files, altered expansions/routes/coordinates
and inactive exports remain rejected; no formatting-effect summary is implied.
Strict replay now admits original ExactlyNDigits::<1/2/3>::parse in both declared
contexts. No-default gains these three and ComponentRange::{name,is_conditional}:
22→27/1,406, loss0; all231 previous pair records persist and all351 current pairs
complete. Caller support still requires Option/product summaries, residual/value
flow, generic callbacks and the other branches of each complete original caller.

Data3.112 prepares checked callable summaries for the original one-to-three-digit
parsers. Each summary retains its verified callee graph, distinct conditional
selector/body reads and Some-only suffix component; None and the scalar payload
remain in the complete tagged result graph. Source/cleanup authority is still
parser-specific: this does not admit forwarding callers. Historical policies do
not acquire the new summary requirement.
Unchanged selections retain27/1,406 and29/1,506 admissions, gain0/loss0, with all
351/406 strict pair records preserved. The separate source-selected hour/min
oracle passes184,194 checks in each feature context. Their remaining connected
blockers are caller source ownership, exact `::<2>` inherent routing and guarded
result/read substitution. No new driver instance transport is needed: both calls
already have exact unit links. See the compact execution evidence for validation.

Data3.113 connects original time0.3.47 ISO-8601 `hour` and `min` through real
Cargo capture, strict verification/admission and matching in both locked contexts.
Independent caller namespace/no-drop proof, exact lexical `::<2>` implementation
selection and the already-proved anonymous prelude now join the complete callee
return graph. One saved shared argument supplies every conditional selector/body
read; each occurrence retains its call scope. The whole Some/None result passes
unchanged, including decimal0..99 and the exact saved input[2..] suffix.
No-default gains these two:27→29/1,406; default29→31/1,506; zero losses.
All351/406 previous pair records persist; all406/465 current pairs complete,
including the independently expected hour/min match. Frozen membership and
selection identities remain unchanged. General payload consumption, transitive
forwarder summaries, NonZero/callback chains and N4..9 remain separate blockers;
this does not activate parse_subsecond or general generic call resolution.

Data3.114 connects original regex-automata0.4.18 checked native byte decoding:
SmallIndex::from_ne_bytes → generated PatternID::from_ne_bytes, including the
actual local U32::as_usize helper. Exact [u8;4] Result inputs, selected64-bit
u32→usize TryFrom/expect totality and u32→u64 From reuse existing source,
no-drop, helper and constructor-map proofs under separate capability gates.
The unchanged native no-default selection gains those three:113→116/590,
zero losses. Original native oracle passes270352 inputs/API in all four feature/
debug contexts. StateID macro/source ownership and unchecked array-result
receipts remain separate blockers; this is not general array/trait/ownership
support. Strict replay, matching and integrated gate evidence are recorded in
docs/rust-support-execution.md and the checked-index-decode development corpus.

Data3.115 connects original `StateID::{new, from_ne_bytes}` through the same
checked SmallIndex chain after the preceding PatternID expansion. Exact temporal
local binding and a complete bounded Struct/Impl predecessor roster preserve
textual scope; no root namespace, derive, cleanup or effect authority follows.
Unchanged native no-default selection:116→118/590, only those two gains, no losses.
Four wrapper originals strictly replay at score1; all six selected original pairs
complete and Accepted at unchanged thresholds. Unknown predecessors, cleanup-
sensitive generated owners and historical policies remain closed. No new runtime
hashing/serialized lookup. [Contract and evidence](eval/coverage-development/ordered-index-expansions/README.md);
remaining higher callers require transitive summaries and guarded payload consumption.

Data3.116 connects the original `time::{week,day,dayo}` family end to end:
checked digit-parser calls, original same-unit `ParsedItem::flat_map`, exact empty
callbacks and pinned native NonZero<u8/u16>. Complete source/cleanup proof keeps
the product non-Copy, saves its suffix before consumption, preserves conditional
reads and separates parser-None from zero-None. The earlier Data3.115 foundation
retained29/1,406 admissions and all406 pairs; connected native selections now gain
only these three:29→32/1,406 without defaults and31→34/1,506 with defaults, no losses.
Strict replay verifies all admissions and completes496/561 comparisons, preserving
all406/465 historical pair records. Independent oracle259,952 checks/context,
zero mismatches. Broader callbacks, generic bodies and drop/unwind stay separate.
The structural detector also accepts both behaviorally different cross-width
comparisons at score1: the negative matching expectation failed. Extraction
coverage establishes neither behavioral equivalence nor rewrite safety.
[Bounded source contract](eval/coverage-development/time-nonzero-digits/README.md)
and [validation/frontier](docs/rust-support-execution.md).

Data3.117 connects the complete original regex-automata SmallIndex unsigned
TryFrom family (u16/u32/u64/usize), including the selected local U16/U64 helpers
and exact core conversions. Native no-default admissions118→123/590, five gains,
zero losses; all identities/build settings persist. Original oracle270,438 API
checks/context across default/no-default and debug/release-body controls, zero
mismatches. Generated PatternID/StateID trait wrappers remain separately deferred.
[Contract, strict replay and validation](eval/coverage-development/unsigned-index-conversions/README.md).

Generated unsigned-conversion source foundation (Data3.117 unchanged): exact
macro/member/Error/selected-identity proof now covers the eight original
PatternID/StateID TryFrom wrappers. This grants no callable or cleanup authority:
native no-default admissions remain123/590, zero gains/losses. Next connect the
complete three-call roster and lexical/no-drop ownership through the existing
Result constructor transport. [Contract](eval/coverage-development/generated-index-conversions/README.md).


Data3.118 completes those eight original PatternID/StateID unsigned TryFrom
wrappers: exact three-call selection composes with existing Result constructor
transport, lexical ownership and no-drop error proof. Native no-default
admissions123→131/590, eight gains/zero losses; historical source/build identities
and all7,503 previous pair results persist. Full suite2,158 passes, fmt/Clippy
clean, style/patterns zero; independent oracle has zero mismatches in four
contexts. Frozen60 unchanged at24/29/7. The structural matcher accepts all28
wrapper pairs, including different input widths; extraction coverage establishes
neither behavioral equivalence nor rewrite safety. Higher transitive callers,
guarded payloads and broader cleanup remain separate.
[Contract](eval/coverage-development/generated-index-conversions/README.md) and
[validation/frontier](docs/rust-support-execution.md).


Wire add/mul error-flow preparation (Data3.118 unchanged): complete diagnostic
capture now preserves all10 named variants/15 payload fields, static-string
fields and both generated error children. Generated declaration source proof also works without a generated callable,
including declaration-only macros. Admission stays131/590, gain0/loss0;
all selected callers remain excluded pending exact generated imports, complete
no-drop ownership and guarded Option→Result/helper composition. Original oracle
443,721 checks in each of four contexts passes. This advances a concrete shared
error chain, with no additional feature row claimed supported.
[Contract and next proof](eval/coverage-development/wire-arithmetic-errors/README.md).

Data3.119 connects original `wire::{add,mul}` → `arithmetic_overflow` through
checked generated imports, complete non-Copy error no-drop proof and guarded
static-field constructor summaries. Native no-default admissions131→138/590,
seven gains/zero losses; all prior131 replay checks/8,515 pair records persist.
Strict276 imports and9,453 comparisons complete; frozen60 unchanged at24/29/7.
Full2,177 tests plus doctests pass; fmt/Clippy/style/patterns clean. The structural
matcher accepts behaviorally distinct add/mul at1, so extraction establishes no
equivalence or rewrite safety. Default capture remains incomplete at the existing
concrete-observation budget; foreign native CI is unrun. Callable arithmetic
summaries, map_err/? and broader ownership remain open. Broad goal active.
[Contract, limitations and evidence](eval/coverage-development/wire-arithmetic-errors/README.md).

Capture-work repair (Data3.119 unchanged): exact function-local root type/token
reuse restores complete default regex-automata assessment (2,582 functions/110
admissions) without raising budgets. The selected wire chain remains excluded
there by imported alloc macro source ownership; investigate that shared S09
barrier next. No-default all590 records/138 admissions/9,453 pair records persist.
Full2,181 tests/doctests and fmt/Clippy/style/patterns pass. Direct arithmetic
callers need unsafe/stateful iterator work; next safe parser target is concrete
`check_slice_len::<u8>` → `try_read_u32`, after macro ownership is resolved.


Data3.120 routes unique original alloc::{vec,format} and native std::thread_local
imports to their existing pinned cleanup-source contracts. Qualified alloc paths,
all invocation arguments, lexical shadowing, attributes, unknown globs and platform
requirements retain independent checks; no new function-effect authority follows.
Default regex-automata110→198/2,582,88 gains/zero losses; unchanged no-default
138/590 with all590 function records identical. This connects the selected original
wire arithmetic/error family under defaults and unlocks other supported families
behind the shared S09 barrier. Validation and remaining reader frontier:
[execution record](docs/rust-support-execution.md).


Fallible-reader proof inputs (Data3.120 unchanged): selected original
`check_slice_len::<u8>` → `try_read_u32` now retains concrete original-owner
type/receiver observations and complete unit/tuple Result error declarations.
These grant no source, obligation, cleanup or admission authority: default
198/2,582 and no-default138/590 remain unchanged, gain0/loss0. The original
generic `slice.len()` deferred source rejection remains intact. Next connect
independent instance verification, guarded error summaries and changing-success
same-error `?`; do not stack further observation-only milestones.
[Contract/oracle](eval/coverage-development/wire-fallible-readers/README.md) and
[validation/frontier](docs/rust-support-execution.md).


Data3.121 connects original `check_slice_len::<u8>` → `try_read_u32`:
whole-owner concrete instance proof, guarded no-drop error summaries and exact
unit→tuple same-error `?`, preserving native reads and panic evidence. Default
198→199/2,582 and no-default138→139/590; sole gain is the original reader, zero
losses. Generic originals and opaque-loop callers remain excluded. Inventory-only
receipts account for unavailable calls without granting call-site authority.
Targeted recapture follows the reverse caller closure and preserves existing
PDG/CFG bytes; equal-work no-default repeats improve36.75–38.40→29.84–30.26s.
The reader is a terminal root, not yet callable. Next connect tagged outputs with
read/panic summaries, tuple-success propagation/binding, static literal roots and
ordered mismatch constructors to complete both original validators.
[Contract](eval/coverage-development/wire-fallible-readers/README.md) and
[validation/frontier](docs/rust-support-execution.md). Broad goal active.

Data3.122 connects original `try_read_u32` → `read_version/read_endianness_check`
through checked read/panic summaries, tuple-success `?` and saved bindings, static
error roots and ordered scalar mismatch constructors. Default199→203/2,582 and
no-default139→143/590: exactly both validators and both constructors, zero losses.
No broader move/drop or unsafe-deserializer support follows. Next source-audited
consumer: default-only `Flags::from_bytes`, requiring complete aggregate-success
ownership and tuple→aggregate propagation.
[Bounded contract and remaining blockers](eval/coverage-development/wire-fallible-readers/README.md).


Data3.123 connects original `try_read_u32` → `dfa::dense::Flags::from_bytes`:
independent Copy/no-drop success ownership, aggregate-result `?`, saved local
fields/count and complete read/error/origin composition. Default203→204/2,582;
no-default143/590 unchanged, zero losses. Strict historical comparisons and the
frozen60 persist. General aggregate Results and unsafe DFA callers remain open.
[Contract/oracle](eval/coverage-development/wire-flag-parser/README.md) and
[validation/next connected frontier](docs/rust-support-execution.md).


Data3.124 connects original `DeserializeError::{state_id_error,pattern_id_error}`
through exact one-use generated-error moves, whole no-drop/source proof, static
message origins and distinct verified callable summaries. Default204→206/2,582
and no-default143→145/590: exactly both constructors, zero losses. This advances
the frozen ID-reader chain; readers still require nominal tuple Results, generated
SIZE/import authority, capturing `map_err`, same-error `?` and complete read/panic
composition. General moves/drop remain open.
[Contract/oracle](eval/coverage-development/wire-id-readers/README.md) and
[validation/frontier](docs/rust-support-execution.md).
