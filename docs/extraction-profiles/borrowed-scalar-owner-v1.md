# Borrowed scalar owners: selected next family

This is a development target, not an active admission contract.

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

The proposed extension is borrowed-only ownership for these scalar-field getters.
Reading scalar fields through a shared reference neither consumes the owner nor
runs its destructor. It must not certify that a non-Copy owner is drop-free.
An owner's custom Drop implementation is therefore a distinct semantic question
from whether this particular borrowed body can invoke it. Any owned construction,
by-value receiver, mutation, returned aggregate, generic field or effectful helper
remains outside this proposal. All reachable operations must be checked; a
supported prefix is insufficient.

The current checked_owned_copy query relies on every selected tree having passed
Copy verification. Relaxing selected::verify alone would silently authorize owned
receivers. Before activation, verified Copy trees and borrowed-only trees must
remain distinguishable using existing nominal resolution and origin evidence;
no new serialized source inventory or compiler drop certificate is justified by
this family. Original source/cfg/import/expansion correspondence is still required.

Expected negative cases include non-Copy value-self calls, constructing and then
dropping an owned value with observable Drop, mutable receiver access, reference
or generic fields, relevant unknown macros, and altered helper field paths or
omitted repeated reads. Unknown neighboring expansion effects cannot become a
purity or ownership certificate. Precisely which existing source checks need
adjustment remains to be established by real Cargo composition tests.

## Measurement policy

Keep the frozen 60 identities and denominator unchanged; these two methods form
a separate development selection in the unchanged 956-function natural unit.
Current expected baseline is 0/2 admitted, 2/2 excluded, no unknown/incomplete
outcome substituted. Require natural Cargo capture, independent verification,
strict admission, complete selected matching, independent runtime expectations,
negative/composition cases and existing regressions before declaring support.
This family is an S09/shared-helper bottleneck increment. Cyclic F10 and general
Option/Result flow remain priorities for the following family audit; broad
async/unsafe/FFI/macros remain deferred.
