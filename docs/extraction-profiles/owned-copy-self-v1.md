# By-value self for checked Copy structs

Source-selected targets: published regex-syntax 0.8.11 `ast::Span::with_start`
and `ast::Span::with_end`, default-feature library context on the pinned compiler.
Their exact source digests are recorded in the root roadmap. Before admission,
the expected behavior is to replace exactly one complete Position and retain all
three components of the other Position. Constructing, updating and reading the
result must compose through actual Cargo capture, strict verification/admission
and complete matching. Frozen selection identities and denominators stay fixed.

Known blockers are the shared-only capture ownership filter, source signature
receiver check, exact inherent implementation-member check, inherent owner/type
check and call adjustment correspondence. Existing aggregate struct-update
lowering and helper result propagation must be exercised together; their separate
fixtures do not prove this chain. Further composition failures remain unknown
until the complete case is run.

The bounded addition accepts untyped immutable `self` on an independently checked
Copy scalar/nested-struct owner. It reuses existing ownership and receiver records,
the nominal environment and aggregate value snapshots. Method syntax requires
an empty adjustment list and exact receiver/adjusted/formal types. UFCS retains
its existing explicit-argument checks. Existing supported body, effect, result,
source and helper restrictions remain mandatory. Non-Copy, mutable/typed self,
trait/generic/overloaded receivers and borrowed-to-owned coercions remain excluded.
No new source buffers, caches, alias assumptions or purity certificates are added.

Independent expectations cover both updates with distinct components, retention
of the original Copy value, chained updates followed by getters, helper-result
receivers and explicit UFCS. Negative expectations cover the unsupported receiver
forms above plus corrupted owner, adjustment and result-update evidence.


## Measured boundary

Frozen replay `run-o2jhsw07` preserves all 60 selected identities/denominators:
18/35/7 becomes 20 admitted /33 excluded /7 inactive, exactly the two selected
Span updates and no losses. The unchanged natural 956-function unit gains those
two admissions. The separate ten-case update selection moves from zero admitted
to seven, preserving three negative exclusions and all original ten development
admissions. All 136 development comparisons complete. Strict import of seven
natural and seven development methods completes all 91 comparisons without
diagnostics; corresponding method pairs score 1. Matching scores do not prove
semantic equivalence or rewrite safety.

Direct owned-formal field getter calls remain outside the connected helper-read
contract: an immutable owned formal field currently uses formal memory locations,
while helper read summaries require shared formal roots. No field read is omitted
to make such a caller admit. Selected updates use the existing aggregate update
and shared getter contracts and do not require that extension.

A six-pair alternating diagnostic against 58addd6 uses the same raw 956-function
input and four unchanged constructor/getter records, freshly capturing four graphs
in both versions. Median elapsed time is 3.255 s baseline /3.230 s current and
median peak RSS is 505742 /505724 KiB. This limited workload finds no material
regression; it is not a full-library performance guarantee. No benchmark cache
or production instrumentation was added.
