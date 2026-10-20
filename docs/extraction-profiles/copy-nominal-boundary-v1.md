# Copy nominal declarations in mixed source units

Data-v3 3.37.0 adds `rust.copy-nominal-boundary`. The existing closed-unit proof
remains available. When it cannot establish a unit-wide boundary, the new path
requires exact, nongeneric local struct declarations whose complete field trees
contain only scalars and recursively checked structs. Every selected struct must
carry independently checked builtin Copy evidence. A shared function parameter
may refer to such a struct; this does not add reference-bearing fields.

The original census is retained completely. Module presence is evaluated from
captured cfgs; ordinary inline/external module paths must have unique exact
ownership, without orphan or omitted files. Root no_std and effective doc/lint/cfg
attributes are supported. Custom module attributes, path rewriting and unsafe
module ownership remain outside this contract. Every original active item is
retained in its parsed module. Unrelated enums, generic structs, trait impls,
function bodies and imports are not certified merely because the module closes.

The lexical index retains named imports, external-path distinctions and unresolved
globs. Only explicit declarations and checked named paths can resolve a selected
type. `use path::{self, Name}` binds the module under its actual name, including
renames. Original declaration ranges exclude preceding attributes consistently
with compiler capture. Selected types must match exact source file, span, package,
manifest, field inventory, spelling and nominal dependencies.

Copy trait and derive identities are pinned separately to the compiler/core/target
registry. The original active Copy token must belong to that exact struct. The
Rust Copy law prohibits Drop on that type. This replaces the need to prove the
absence of every unrelated destructor in the unit; it is not an invented purity
certificate. Scalar-only transitive fields exclude interior mutability and owned
destructor-bearing payloads. No derived comparison, cloning or neighboring method
is invoked by this proof.

Relevant lexical scopes must also have resolved expansion/name behavior. Unknown
module-level macro invocations, custom derives or attributes, proc-macro name
imports, globs, and primitive aliases block the selected scope and descendants.
Builtin derive names are recognized only where original scope bindings cannot
replace them; known inert attributes on unrelated items do not authorize calls.
Macros and derives in unrelated sibling modules remain retained and unadmitted.
Selected function and implementation attributes must independently be ordinary
and active. Generated functions cannot claim original lexical ownership.

The proof reuses existing snapshots, parsed unit state, Copy observations and
origin records. The environment's existing cache is keyed by the immutable census,
constant contract, selected cfgs and verification policy. It now retains environment
construction failures too, preventing repeated full-file parsing for each rejected
function. It does not cache a successful function proof after a failure, add a
serialized certificate flag, or copy additional source buffers.

This 3.37.0 boundary admits the natural Position/Span endpoints; it alone does
not establish implicit receiver borrowing or read substitution. Data-v3 3.38.0
connects those pieces under the separate
[owned Copy receiver contract](owned-copy-receiver-v1.md), completing the selected
constructor → owned local → shared scalar getter composition. General local
storage borrowing and by-value self updates remain unsupported.

Matching remains similarity detection. The offset and column getter variants
retain distinct verified source/field paths but currently receive score 1 from
the existing matcher. This is an unmet detector distinction expectation, not an
extraction equivalence or rewrite-safety result. Scoring and thresholds are
unchanged. See the root roadmap for measured capture and validation results.
