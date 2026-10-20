# Borrowed scalar owners: selected next family

Data-v3 3.40.0 implements this bounded admission contract.

## Source selection and expectations

The selected natural functions are regex-syntax 0.8.11 default-feature library
`utf8::ScalarRange::is_valid` (source SHA256
`3ab772b71af536ce0998471235d9a221f85188c69e79ab6959b70c2ea9dc82ac`)
and `utf8::ScalarRange::is_ascii` (SHA256
`1e78b140726ca5e9378e6142bee803d0e675a724a40f76b65d5a40c4edd165b5`).
Selection follows source inspection before admission assessment. ScalarRange is
a private nongeneric struct containing start/end u32 fields, without Copy or
other derives. Its module also contains enums, iterators, trait implementations,
formatting macros and methods constructing Option and mutating buffers.

Independent expectations: is_valid returns start <= end; is_ascii returns
start <= end && end <= 127. Equal endpoints are valid, reversed endpoints are
invalid, 127 is ASCII and 128 is not, and u32::MAX remains an ordinary endpoint.
The complete caller must preserve short-circuit evaluation and the two reads of
end (callee and caller), rather than merging them or inventing alias independence.
A shared wrapper calling is_ascii should compose under the same contract.

## Current blockers and bounded proposal

Current full natural capture run-o2jhsw07 includes both exact functions. is_valid
is excluded for missing builtin Copy evidence. is_ascii additionally reports an
unsupported/not-yet-certified helper. Source audit identifies the connected
requirements: selected declaration/module/name evidence, shared receiver proof,
scalar field locations, verified is_valid read summary, and short-circuit caller
composition. Existing body/source/dispatch contracts must remain mandatory.

The extension is borrowed-only ownership for these scalar-field getters.
Reading scalar fields through a shared reference neither consumes the owner nor
runs its destructor. It must not certify that a non-Copy owner is drop-free.
An owner's custom Drop implementation is therefore a distinct semantic question
from whether this particular borrowed body can invoke it. Any owned construction,
by-value receiver, mutation, returned aggregate, generic field or effectful helper
remains outside this proposal. All reachable operations must be checked; a
supported prefix is insufficient.

The current checked_owned_copy query relies on every selected tree having passed
Copy verification. Relaxing selected::verify alone would silently authorize owned
receivers. Verified Copy trees and borrowed-only trees remain distinguishable in the
existing nominal resolution using independently checked origin evidence;
no new serialized source inventory or compiler drop certificate is justified by
this family. Original source/cfg/import/expansion correspondence is still required.

Expected negative cases include non-Copy value-self calls, constructing and then
dropping an owned value with observable Drop, mutable receiver access, reference
or generic fields, relevant unknown macros, and altered helper field paths or
omitted repeated reads. Unknown neighboring expansion effects cannot become a
purity or ownership certificate. Existing selected module/name/attribute/expansion checks remain mandatory.
The new gate additionally checks every normalized interface and body type for
owned occurrences of each non-Copy selected owner. Owned occurrences fail even
when nested in aggregate tokens. This traversal is verification-local and adds
no serialized observations, caches or source buffers.

## Measurement policy

Keep the frozen 60 identities and denominator unchanged; these two methods form
a separate development selection in the unchanged 956-function natural unit.
The measured baseline is 0/2 admitted, 2/2 excluded, no unknown/incomplete
outcome substituted. Require natural Cargo capture, independent verification,
strict admission, complete selected matching, independent runtime expectations,
negative/composition cases and existing regressions before declaring support.
This family is an S09/shared-helper bottleneck increment. Cyclic F10 and general
Option/Result flow remain priorities for the following family audit; broad
async/unsafe/FFI/macros remain deferred.


## Results and retained restrictions

Unmodified default-feature regex-syntax Cargo capture admits both selected
methods. All 956 names, source digests and spans match run-o2jhsw07. Exactly these
two methods gain admission, with zero losses: eleven natural admissions become
thirteen. Capture and assessment are complete. Max-pairs zero intentionally
leaves full-library matching incomplete; it is not a capture failure.

The separately specified ten-function development selection moves from zero to
four admissions on identical source under baseline f2d0152 versus this policy.
Both getters and two shared wrappers admit; all six comparisons complete. Owned
construction, local owned construction and by-value methods remain excluded.
The existing frontend also rejects destructor-bearing owners before normalized
capture. Thus even their shared getter stays excluded in this increment; no
missing normalized observation is treated as no-drop evidence.

The natural and development getter pairs are independently imported, verified
and strictly admitted. All six comparisons among the four functions complete
without diagnostics; the two corresponding pairs are Accepted at score 1.
These are extraction/matching results, not equivalence or rewrite-safety claims.
The caller retains the leaf's start/end reads and its separate end read. Removing
a helper read fails document verification and strict admission.

The runtime oracle appends borrowed-scalar-oracle.rs to an otherwise byte-identical
copy of the published utf8.rs in a temporary rustc test harness, with the
separate development module. Its manually specified boundary table checks equal,
reversed, 127/128 and u32::MAX endpoints against both natural methods and the
complete development caller. The independent oracle and four existing natural
UTF-8 tests pass. It does not modify the sources used for Cargo capture.


Existing regression expectations now distinguish borrowed admission from a builtin
Copy observation: plain/manual-Copy getters and a getter with an inactive Copy
derive admit as borrowed-only, while the absent/forged Copy evidence checks remain.
A shared getter beside a constant-local hidden Drop impl also admits without
certifying that neighbor. An owned construction in the same source remains
excluded. Historical Data-v2 still excludes all ten development functions.


Final validation: 815 workspace tests pass with five existing ignores; all four
focused tests cover supported composition, three installed CI targets, retained
negative field/ownership cases and historical-profile exclusion. All-targets
clippy with warnings denied, formatting and diff checks pass. rstyle checks 2622
functions with zero violations; patterns finds zero candidates. Frozen replay
run-yf3i09jl retains 20 admitted /33 excluded /7 inactive and all identities.

A six-pair alternating performance diagnostic against f2d0152, run after capture
and tests finished, freshly imports the same four unchanged natural records from
the same raw 956-function input. Dependency lock entries and compiler agree;
both versions produce four graphs. Median elapsed time is 3.135 s baseline /
3.125 s current, peak RSS 494672 /494270 KiB. This bounded workload shows no
material regression and is not a full-library performance guarantee.
