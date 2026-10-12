# Compositional data coverage contract, draft 1

Frozen implementation baseline: `9ba773fec4b5dea4381974eaf1c20b8e58b623fd`
(origin/main, inspected clean). Initial checkout was
`b6f7745a5b7fbdde337a27bc3850667bd932a528`; the sole subsequent commit is the
reviewed merge, with identical trees. Work branch:
`ccgraph/compositional-data-coverage`. Authoritative compat revision:
`04d9aa0db8b1f8e4a3fcad16622e2a4e3bc3e53f`, from the separate compat repository.
Vendored contracts are unchanged. This draft does not register admission.

Planned selection is `clones --extraction-profile hir-data-v1`. Planned extraction
identity is `rstyle.rust.hir-data/1.0.0`; planned matching identity is
`rstyle.ccgraph.data/1.0.0`. Neither identifier may authorize input until capture,
independent verification, serialization, strict import and real-command tests
exist. R0 and hir-expanded-v1 retain their identities, defaults and semantics.
Historical scalar contracts and reports remain immutable evidence.

## Evidence boundary

Every owned original-source operation must have unique resolved HIR correspondence
under the captured compilation context. Unsupported unreachable or cfg-removed
source prevents certification. Source identities comprise package identity,
relative file, exact owned signature/body bytes and file byte offsets. Compilation
contexts remain distinct. Nested definitions belong to their own owners; interval
containment is not function correspondence. Missing captures are unknown, never
proof of inactivity. Duplicates and conflicting mappings fail explicitly.

Exact manifests bind paired documents, frontend facts and evidence dependencies.
All selected units are captured before resolving calls. No evidence cache, old
artifact scanning or compiler-order dependence is permitted. Required evidence
must cross producer verification and an independent serialized semantic verifier;
re-emission alone is insufficient. Arithmetic/resource failure is incomplete
execution. No silent truncation is permitted.

## Scalar composition

Direct calls resolve to concrete function definitions in captured selected units.
Arguments evaluate left to right exactly once; the result depends on every actual
argument and the verified callee summary. Calls remain source-backed CALL operations,
projecting to STATE. Callee bodies are never inlined or normalized into callers.
Only acyclic call graphs are supported. Summary identities bind resolved callee,
source digest, signature, semantic build settings, verified document identities,
argument/result dependencies, effects, returning and panic behavior, and transitive
dependencies. Any changed dependency invalidates the caller. Successful compilation,
names and supplied purity flags do not prove effects. Outside captured scope,
only independently tested registered summaries are supported. Recursion, unresolved
dispatch, unsupported callees and unavailable/corrupt dependencies fail closed.

Assertions initially accept only compiler-resolved built-in assert! and
debug_assert! with one boolean condition and optional trailing comma. Record the
exact original invocation, expansion identity/callsite, condition inventory and
evaluation order. Enabled assertions evaluate once, branch to normal continuation
when true, and have disclosed panic behavior when false. Disabled debug assertions
retain original-source evidence and build-setting identity; absence from HIR does
not authorize missing evidence. No formatting arguments or same-spelled user macro
is accepted. Panic/unwind cleanup is outside the bounded normal-flow contract.

Scalar match adds negative literals, OR alternatives, inclusive/exclusive scalar
ranges and resolved scalar constant/associated-constant paths. Retain pattern
operations and ordered pattern/guard decisions; evaluate the scrutinee once.
Constant evidence binds resolved definition, scalar type, value, original source
and build settings. Only literal scalar definitions and explicitly verified
references to them are supported; arbitrary const execution and unresolved globals
are excluded. Guards, local writes, returns and required value merges retain
their actual dependence. A construct exceeding an explicit resource budget fails
incomplete rather than certifying a prefix.

## Concrete data and locations

The finite type grammar consists of the existing scalar domain, concrete structs,
tuples, fixed arrays, shared references, shared slices, str referents, and concrete
Option of a supported scalar/shared reference. Struct identity binds package
name/version/source identity, definition module path and definition source digest;
field identity is declaring type plus field index and verified declared type.
No debug strings or session-local compiler IDs serve as nominal identity.
Types preserve integer width/sign, array length and reference mutability. Lifetimes
are omitted from matching text but reference-origin constraints remain verified.
Nominal identity is not structurally erased to match unrelated interfaces.

Supported aggregates must have a finite, recursively checked drop-free and
freeze referent boundary, with concrete resolved fields/elements and no hidden
interior mutation. Raw pointers, unsafe, mutable references, drop-bearing values,
interior mutability, unresolved generics/trait dispatch and unknown call effects
are excluded. Shared reference syntax alone is insufficient evidence.

Construction evaluates components in Rust source order. Field reads record
resolved field identity and nested projection paths, with dependencies on the
actual aggregate/reference value. Local binding/rebinding creates distinct value
definitions. Location evidence records formal referents, field/element paths,
reference origins, possible aliases and reads. Shared formal inputs may alias;
no disjointness is inferred. Borrowed returns must originate in verified input
referents/fields and preserve all branch alternatives. No generic memory node
substitutes for absent location facts. No write is fabricated for a shared read.

Shared slice len/is_empty and borrowed str len/is_empty have resolved, nonmutating,
always-returning summaries. String length is UTF-8 bytes. Scalar element reads
evaluate base/index once and retain bounds-panic limitations. Byte-slice subslicing
records each bound, reference origin and required ordering/bounds checks. Invalid
str indexing is never reinterpreted. Checked access may use concrete Option
construction, discriminant tests and bounded payload extraction. General enums,
Vec/String mutation, iterators, closures and arbitrary library methods are excluded.

## Matching extension and compatibility

The new matching identity extends only resolved interface vocabulary and read
location interpretation. Interfaces retain compact JSON role/type/variadic tuples;
aggregate/reference types use deterministic compact JSON type trees as the type
string, with scalar leaves unchanged. Projection retains all source-backed reads,
construction and borrow operations as OTHER, calls as STATE, and required value
entities with verified reachable sites. Memory summaries remain excluded.

NumOfReference counts distinct retained read locations identified by root binding
and verified projection path; plain scalar reads use the existing binding policy.
Dynamic element paths retain their source-backed evaluated index identity, never
invented constant locations. Alias uncertainty is evidence, not independence.
The original nine numerical features, Jaro-Winkler convention, sequential gates,
directed WL, depth, weights, multiplicity and exact overlap/cosine scores are reused
unchanged. No threshold change or canonicalization is authorized.

Pairs require equal registered extraction/matching identities, verified concrete
type vocabulary and compatible overflow/debug-assertion/panic settings. Native-width
types anywhere in interfaces or operations require equal known target widths.
Every function independently establishes source/build/dependency evidence. Different
nominal type tokens can affect the existing string score; they are not aliased.
No policy/type/build check is bypassed to increase pairs. Enumerate every unordered
distinct admitted-function pair; incompatible pairs remain explicit decisions.

## Flow and acceptance obligations

Fixed-point reaching definitions and divergence-sensitive dependence remain in
force. Merges outside cycles may be checked with acyclic histories only when their
evaluation cannot cross a loop epoch. Cyclic value merges require separately
specified independent epoch-aware verification; until then they are unavailable.
No virtual executable exit or bounded iteration sample proves completion.

Independent fixtures must specify full composed helper families and concrete
read-only methods, including cross-module/package calls, aliases, transitive edits,
assertion settings/shadowing, negative/OR/range/constant patterns, nested fields,
borrowed branches, shared aliases, slice loops/bounds and unsupported mixtures.
Real temporary Cargo workspaces must exercise the entire source-to-pair handoff,
multiple targets/concurrent compilers, corrupt/missing evidence, warm runs and
trace invariance. Legacy decisions and inherited RSTYLE_* isolation are regressions.

A broader source-only inventory must freeze selection, build/source identities,
independent classifications, project/function minima, tier targets and budgets
before running the detector. Target majority active-function coverage in each
declared tier and at least 95% admission of independently expected in-scope source.
Exact numerical counts require the source inventory freeze, not outcome-guided
sampling. Until that freeze exists, no new coverage acceptance claim is available.
The old 60-definition denominator remains intact; the reported 6/50 observation
is a separate inventory unless its original run identity can be recovered.

This work authorizes coverage/correctness only. Mining, grouping, proposals,
rewriting and calibration remain inactive. A later quality study requires separate
preregistration and untouched project/family holdouts with sufficient natural pairs.
