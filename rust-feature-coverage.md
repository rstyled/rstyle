# Rust feature coverage and roadmap

A practical master checklist for processing real-world Rust through the **strict
HIR source-to-pair (`cargo rstyle clones`) pipeline**. This is not a checklist for
legacy `patterns`, a Rust language specification, or a claim of complete Rust
semantics. Real-world coverage is a ladder, not one final syntax switch.

Audited against `4a0e346` (2026-10-15), after the performance work. The latest
implemented selection is **`hir-data-v3`**, extraction
`rstyle.rust.hir-data/3.1.0`, matching `rstyle.ccgraph.data-calls/1.0.0`.
The default remains `r0`; `hir-expanded-v1` does **not** select the Data features.
The continuation log below records inventory work without changing extraction semantics.

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
| V12 | Named constants and associated constants | Missing | `x + LIMIT`, `S::LIMIT` | Definition/source/type/value evidence and use dependencies; arbitrary const execution is a further step. |
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
| F04 | Bounded scalar literal/wildcard match | Partial | `match n { -1 => x, _ => y }` | Integer/bool scrutinee, 1–16 arms, literal arms including negative integer literals, final unguarded wildcard; not general exhaustive pattern matching. |
| F05 | Scalar match guards | Partial | `match n { 0 if b => x, _ => y }` | Ordered pure scalar guard subset; arbitrary effectful guards and patterns are not supported. |
| F06 | OR/range/constant patterns | Partial | `1 \| 2 => x`, `1..=9 => y`, `LIMIT => z` | Data-v3 3.1.0 supports flat integer/bool literal OR alternatives, including negatives and guards; see [contract](docs/extraction-profiles/scalar-or-patterns-v1.md). Ranges and resolved constants remain missing. |
| F07 | Destructuring, reference, slice, and enum patterns | Missing | `Some(x) => x`, `[a, ..] => a` | Payload/field/element dependencies, binding modes, and ownership/borrow effects. |
| F08 | `if let`, `while let`, let chains, `let else` | Missing | `if let Some(x) = v { ... }` | Pattern decision/binding flow and divergence requirements; not ordinary boolean branching. |
| F09 | Scalar `while`/`loop` and innermost exits | Covered | `while n > 0 { n -= 1; }` | Scalar state, nesting, early return, unlabelled valueless `break`/`continue`; divergence-sensitive flow. |
| F10 | Value merges in loop-containing functions | Missing | `while b { n = if c { x } else { y }; }` | Current blanket exclusion includes nonunit conditional and short-circuit merges anywhere in a function containing a loop. Separate safe outside-cycle merges from epoch-sensitive cyclic ones. |
| F11 | Labelled and value-carrying exits | Missing | `'outer: loop { break 'outer; }`, `break x` | Exact target resolution and value/frontier dependencies; also labelled block expressions. |
| F12 | Iterator `for` loops | Missing | `for x in xs { ... }` | Iterator protocol, yielded values, effects, drop, and source/desugaring correspondence; not covered by scalar `while`. |
| F13 | `Option`/`Result` branching and `?` | Missing | `let x = parse()?;` | Concrete discriminants/payloads, early return, conversions, and cleanup; `?` is not merely a scalar branch. |
| F14 | Assertion macros | Missing | `assert!(b);`, `debug_assert!(b);` | Planned read-only slice: original invocation/expansion, condition order, build settings, and disclosed panic behavior. Formatting arguments need additional work. |
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
| D09 | By-value reference-bearing struct/tuple inputs | Partial | `s: Borrowed<'_>` | Verified formal component origins; construction, non-scalar local reads, and general by-value memory-reading helper substitution remain missing. |
| D10 | Lifetime-only nominal declarations | Partial | `struct View<'a> { bytes: &'a [u8] }` | Unbounded lifetime-only grammar and exact region/arity proof; no type/const parameters, bounds, or where clauses. Callable inherent impls have a narrower nongeneric boundary. |
| D11 | Aggregate construction and struct update | Missing | `S { value: x }`, `(x, y)`, `[x; 4]`, `S { value: x, ..old }` | Source-order components, aggregate value definitions, fields/elements, moves/copies and reference origins. Empty unit tuple `()` already works. |
| D12 | General non-scalar local values/reference rebindings | Missing | `let r = s.bytes; r.len()`, `let t = s; t.value` | Current data-path verifier rejects non-scalar local binding reads; formal origins and directly composed expressions do not provide general local correspondence. |
| D13 | Borrows of locals and temporaries | Missing | `let r = &local;`, `&S { value: x }` | Storage identity/lifetime, temporary scope, escape/origin proof, and local value dependence. |
| D14 | Explicit and broader implicit dereference | Partial | `*r`, `nested_ref.field` | Explicit `*` is unavailable. Only verified shared field/receiver transitions exist; repeated implicit dereferences lacking storage-read evidence fail closed. |
| D15 | Array/slice element reads | Missing | `xs[i]` | Evaluate base/index once, preserve element location/index identity and bounds behavior. |
| D16 | Slice ranges and borrowed subslices | Missing | `&xs[start..end]` | Bound evaluation, range representation, source origins, checks and panic limitations; str has separate UTF-8 constraints. |
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
| C06 | Concrete inherent method calls and UFCS | Partial | `s.value()`, `S::value(s)` | Data-v3: same-unit nongeneric inherent impl/method, shared receiver, exact owner/source/adjustment proof, supported summary. No trait dispatch. |
| C07 | Receiver adjustments and coercions | Partial | `s.value()` | Data-v3 inherent helper method syntax requires an exact shared type and one builtin shared dereference followed by one shared borrow. Registered integer methods have scalar by-value receivers; metadata methods also accept an empty adjustment chain. General autoref, repeated/overloaded deref, unsizing and mutable/by-value inherent receivers remain unavailable. |
| C08 | Registered integer methods | Partial | `x.unsigned_abs()`, `x.wrapping_mul(y)` | Four instantiations only: i32/i64 unsigned_abs and u32/u64 wrapping_mul. Not general primitive-method support. |
| C09 | Slice/str length and emptiness | Covered | `xs.len()`, `text.is_empty()` | Four registered nonmutating operations; exact supported compiler/core/target resolution and receiver evidence. Slice elements must be supported non-reference-bearing types; a shared-slice interface alone does not authorize these calls on reference-bearing elements. No buffer-content read is invented. |
| C10 | General core/std/third-party APIs | Missing | `u32::from_le_bytes(bytes)`, `hint::unlikely(b)` | Additional resolved summaries or captured verified bodies, effect/result dependencies and toolchain authority. Familiar names are not proof. |
| C11 | By-value memory-reading helper arguments/results | Missing | `inspect_value(s)`, `make_value(x)` | Aggregate substitution/construction, memory effects, value components and ownership. Supported interface types do not establish this call boundary. |
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
| S03 | Active conditional-source correspondence | Partial | A cfg-controlled statement present in resolved HIR | Requires complete original-source/resolved-HIR correspondence in the actual build; not full cfg coverage. |
| S04 | Cfg-removed source and inactive definitions | Missing | `#[cfg(feature = "x")] statement` absent from HIR | No independent removed-source resolver. Removed supported operations remain unknown; known unsupported source remains excluded. Missing capture does not prove inactivity. |
| S05 | Conditional/path-selected modules | Missing | `#[path = "other.rs"] mod m;` | Resolved module presence/path/ownership; the current nominal census rejects all module attributes. |
| S06 | Declarative macros | Missing | `macro_rules! m { ... }`, `m!()` | Original invocation, expansion ownership, argument/operation correspondence and hidden item/effect evidence. Assertions are a proposed narrow first slice. |
| S07 | Derives and procedural/attribute macros | Missing | `#[derive(Clone)]`, `#[some_attribute]` | Generated definitions/impls, expansion provenance, source correspondence and effects; source spelling alone is insufficient. |
| S08 | Included/build-generated code | Missing | `include!(...)`, generated `OUT_DIR` files | Expansion/input/build ownership and original-source identity beyond the current admitted subset. Build scripts compiling is not graph support for generated source. |
| S09 | Nominal proof in ordinary mixed-content crates | Partial | A getter beside an enum, const, trait impl or body macro | Current unit whitelist accepts ordinary functions, structs, modules, uses and restricted inherent impls. Other items, associated constants, macros/derives and unsafe forms can block otherwise simple nominal APIs. Need finer resolved ownership/drop/effect proofs. |
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
| X09 | Current natural-crate coverage inventory | Covered | Rerun frozen samples with Data-v3 | Frozen Data-v3 3.1 inventory: 9 admitted, 44 excluded, 7 unknown out of 60; see [refresh report](eval/hir-coverage/REPORT-data-v3.md). |
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

## Suggested delivery ladder

This is a proposed sequencing framework, **not measured priority or a new frozen
acceptance contract**. Rows overlap: library or source-evidence work may be needed
before a syntax slice can admit a natural function.

1. **Re-establish current coverage:** X09. Extend the unchanged frozen sample
   assessment to Data-v3; record all blocker combinations and unknowns, not just
   the first error. Do not replace the historical denominator or infer inactivity.
2. **Finish scalar composition:** V12, F06, F14, S03–S04. Assertions,
   resolved constants and richer patterns target scalar sample families without
   requiring a general mutable-memory model.
3. **Finish connected read-only data:** D11–D18, C10–C13, with S09 and F10.
   Construction, non-scalar locals, precise reference origins, indexing/subslicing,
   concrete Option, calls and a usable nominal proof must compose end to end.
4. **Expand idiomatic safe Rust:** D19–D25, F07–F08, F12–F13, C14–C17,
   T01–T10. Enums, traits/generics, mutation/ownership/drop, closures and iterators
   are interacting workstreams, not one small slice each.
5. **Expand runtime/system boundaries:** C18, D26, A01–A04, U01–U04,
   alongside exceptional flow. IO, async, synchronization and unsafe/FFI need
   explicitly bounded effect and execution models.
6. **Validate usefulness separately:** X10–X11. Freeze natural coverage targets
   and later held-out quality criteria before claiming real-world applicability
   or detector quality. Rewrite safety is not an implicit final rung.

Each capability should advance from missing/partial only after capture, source and
semantic evidence, CFG/PDG lowering, applicable effect/origin summaries, independent
verification/import, projection, and a composed real-Cargo fixture are connected.
Resource exhaustion remains incomplete execution. Keep a small independent
example and a negative case with each slice, and rerun the frozen natural inventory
to measure actual gains.

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
