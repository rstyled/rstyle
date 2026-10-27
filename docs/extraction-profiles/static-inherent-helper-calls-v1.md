# Static inherent helper calls

Data-v3 extraction 3.26.0 adds `rust.static-inherent-helper-calls` (38
capabilities); matching remains `rstyle.ccgraph.data-calls/1.0.0`.
Receiver-free functions in same-unit nongeneric inherent struct implementations
can compose through the existing verified helper boundary. Qualified paths,
import aliases and `Self::helper` resolve against the closed lexical source index.
Zero arguments, unit results and transitive calls use existing helper summaries.

A static call requires exact nominal association with its implementation, but does
not require a value or supported field layout of the owner. Capture reuses the
existing inherent definition and unit association records, recording the original
struct declaration identity. Independent verification checks implementation and
function ranges, selected package, nominal path and declaration digest against the
existing closed source census. Caller paths resolve to that same declaration.
Shared receiver methods retain their concrete owner and receiver checks.
No new serialized record, source snapshot or retained cache is introduced.

The Cargo fixture covers an owner with a `String` field that is never constructed
or received, aliases, qualified paths, transitive `Self` calls, zero arguments and
unit results. All admitted documents pass independent and strict verification;
equivalent callers reach accepted matching. Changed owner, implementation identity,
implementation span and record identity are rejected. The body-only nominal fixture
also composes a static helper that constructs `Self` and returns a scalar.

This does not broaden callee effects, reference-result contracts or memory
substitution. Unsupported local borrows remain excluded through callers. Trait or
generic implementations, mutable/by-value receivers, cross-unit helpers and general
ownership/drop remain outside this capability. Earlier extraction profiles retain
their boundaries. C06 remains Partial.
