# Fresh Range results

Data3.75 (`rust.fresh-range-results`) targets the unchanged regex-automata
0.4.18 `Span::range` → local `<Range<usize> as From<Span>>::from` chain and its
`Input::get_range` and `Match::range` callers. The
[source selection](../../tests/fixtures/function-families/range-getters-selection.json)
fixes source identities, six build contexts, anchors and the reverse-conversion
negative before activation. Frozen60 membership and denominators stay unchanged.

The bounded contract permits exact fresh `Range<usize>` construction and whole
result forwarding through checked helpers, including explicit returns and blocks.
Range is **not Copy**. Range parameters, local storage, discarded values, field
consumption, borrowing, nesting, update bases, generic ranges and other traits
remain unavailable. Range literal syntax as a returned value remains outside
this constructor contract; subslice bounds retain their established proof.

Mandatory rustc observations bind the legacy Range lang item, core definition and
concrete usize arguments, ordered start/end fields, non-Copy status, absence of
a destructor and absence of cleanup obligations. Independent verification checks
the pinned compiler/target registry and reviewed `core/src/ops/range.rs` digest.
No layout or general move/drop behavior is inferred.

Source verification binds complete original methods and their exact trait impl,
unshadowed library imports, checked local Copy input and neighboring source
closure. Calls additionally bind pinned core From identity and the selected
local implementation to the certified helper body. Fresh-use checks cover the
whole function; helper summaries permit verified reads only with complete normal
flow. Owned receiver adjustment still requires the existing exact Copy proof.
Constructor slots and ordered operands use the existing aggregate graph contract.

Independent expectations preserve both endpoints, including inverted ranges and
usize extremes; Input's done state and invalid Match constructor panics remain
separate cases. Tests cover std/no_std across the three pinned targets, complete
helper composition, strict document/producer/admission checks, matching and
resealed type/drop/source/selection/field-order/operand/policy attacks. Native
execution and cross-target capture are distinct evidence. Current measured
natural admissions and validation are recorded in the root roadmap checkpoint.
