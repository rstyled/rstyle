# Checked anonymous dependency namespaces

Data-v3 3.100 (`rust.checked-anonymous-dependency-namespaces`) permits an
original direct module glob whose complete checked footprint consists of two
anonymous trait reexports. The bounded source shape is the original
`num-conv 0.2.0` prelude: `pub use crate::{Extend as _, Truncate as _};`.
This proves absence of lexical bindings. It grants no method, effect, Copy or
cleanup contract for either trait or its implementations.

The consumer must select the producer dependency explicitly. The existing Cargo
graph, invocation, emitted/loaded metadata and artifact checks establish its
exact owner. Captured observations alone, unselected/transitive owners and raw
standalone facts grant nothing. Original producer source, root attributes,
trait declarations, grouped and leaf spans, visibility, export identities and
ordered reexport routes are checked independently. Root attributes are bounded
to literal documentation and `no_std`.

The analysis joins each receipt to the original active consumer module and exact
unattributed use declaration. Only an ungrouped direct glob can discharge source
ambiguity; all occurrences of an indexed scope/path must qualify. Generated and
block-local imports cannot reuse the receipt. A neighboring or grouped unknown
glob retains its ambiguity. The source inventory, remaining imports, derives,
macros, trait implementations and cleanup checks remain exhaustive.

Checked overlays belong to one immutable dependency analysis and verification
policy; source-cache keys include the namespace capability. Source-input and
nominal queries preserve the owning evidence view. Historical documents do not
acquire this capability. Routine consistency uses cached XXH3; historical SHA
claims and reviewed source pins remain compatible.

Real Cargo tests exercise constructor/getter/helper composition, complete pair
evaluation and strict replay, plus unselected-owner and neighboring/grouped-glob
negatives. Independent proof adversaries reseal source coordinates and test root
and consumer attributes, missing exports, changed routes/identities and ownership.
These are development fixtures, not natural-family admission measurements.

This advances the selected `time 0.3.47` error-conversion chain. It does not close
the foreign generated `time_core::convert` namespace, other source/macro and
local-glob obligations, static-string ownership or guarded nominal payload flow.
The four selected original conversions remain the acceptance target.
