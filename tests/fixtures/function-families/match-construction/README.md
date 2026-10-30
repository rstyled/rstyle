# Original Match construction

This separate development selection fixes two original `regex-automata@0.4.18`
`Match::new` instances before admission evaluation: `S=Span` and
`S=Range<usize>`. Existing selections and frozen denominators remain unchanged.

The result must preserve `(pattern,start,end)` after exactly one conversion.
The original assertion rejects converted `start>end`; equal endpoints and
maximum offsets are valid. Range is non-Copy and consumed once. Complete Span,
Match, PatternID/SmallIndex declarations and neighboring implementation source
references remain in the selection. No general Into or macro support is assumed.

`oracle.py` defaults to exact selected source/range checks and reused historical
identity references, without hashing, compiling or executing. Scheduled
`--run-native` checks the unchanged public library API in default/no-default
contexts, all fields and boundary endpoints, inversion panics, and custom
conversion side effects, drop order, repair and panic propagation. It also checks
that `Range<usize>: Copy` fails compilation. Custom conversion cases are explicitly
outside the extraction selection. Native execution covers the current host;
three-target capture, strict verification, admission and matching remain separate.

```sh
python3 tests/fixtures/function-families/match-construction/oracle.py
# Only when the coordinator schedules native builds:
python3 tests/fixtures/function-families/match-construction/oracle.py --run-native --output /tmp/rstyle-match-native.json
```

Whole-package identity is not recomputed by this script. Original Cargo capture
must still supply complete source inventory, selected implementation/instance
and build-context authority. A source check or green native oracle is not an
admission, detector-quality or rewrite-safety claim.

The diagnostic capture defaults to one native no-default context (590 original
definitions); `--context default` selects the full 2,582-definition context.
Both substitutions retain the original generic owner once and observe the
selected blanket Into and reflexive/local From separately. These observations
do not prove that Into delegates without additional effects.

```sh
python3 tests/fixtures/function-families/match-construction/capture.py --output /tmp/rstyle-match-capture
```

Admission still requires complete active Copy/source ownership for
Match/PatternID/SmallIndex/Span, verified blanket forwarding, full returned scalar
components and assertion order, plus affine Range consumption. Bare diagnostic
source scans remain incomplete; the enriched owning roster retains source
correspondence and the deferred `span.into()` finding. Neither may be discarded
when materializing a concrete instance. Existing source pins can be reused;
these diagnostics add no hash operations or serialized cache keys.
