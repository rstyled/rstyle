# Checked reference Option propagation

Data-v3 3.80 connects the source-selected Memchr, Memchr2 and Memchr3 prefix
methods in regex-automata 0.4.18. Activation evidence is recorded in the root
roadmap; the [selection](../../eval/coverage-development/prefilter-prefix/selection.json)
fixes original bytes, build contexts and expectations before admission.

The bounded contract accepts `Option<&u8>` propagation into an
`Option<Copy product>` result. Products contain only supported scalars, tuples
and independently source-proved Copy structs. An observed product shape alone
supplies no Copy or drop permission. The input is evaluated once and retained;
None returns a fresh None of the output type, while Some yields the saved shared
reference under the opposite guard. The surrounding dereference performs the
actual bounds-checked content read. Propagation adds no content read.

Exact pinned core Try/FromResidual implementations, substitutions, ControlFlow
variants and source authority are required on each supported compiler/target.
The pinned residual is `Option<!>`. Generated continuation machinery does not
create user helper-call obligations; original operand calls retain their full
ordinary obligations. Strict document checks require the saved input, tag guard,
reference origin, fresh output None and early-return-to-exit edges. Historical
policies do not gain product Option or propagation permission.

Original local trait-member source ownership is separate from trait dispatch.
The selected member must have an exact original source range, concrete shared
byte-field owner and direct local nongeneric trait declaration with no defaults
or associated items. Complete source census, cfg, neighboring items and derives
remain checked. The exact core Debug trait/builtin-derive binding requires pinned
source and unshadowed library-root proof. This does not authorize arbitrary
imports, derive aliases, trait calls or sibling implementations.

Independent native expectations use the unchanged public library. A prefix
checks only span.start, bounds and one/two/three needle-byte membership; span.end
is unused. Empty/reversed spans can therefore produce Some. Out-of-bounds and
usize::MAX return None before the addition or content read. No valid-span
precondition is invented. The no-default public constructor cannot select these
private implementations: its native checks cover constructors only, while
cross-target extraction selects the still-active original methods explicitly.

General Option payload matching, scalar Option `?`, error conversion, mutable
payloads, non-Copy products, nested owned storage, generic/trait dispatch and
cleanup remain separate contracts. Tests combine complete Cargo composition,
strict replay, alpha matching, source-root/derive/cfg attacks and independently
resealed witness/graph mutations. Full-library matching and detector-quality
claims remain separate from extraction coverage.
