# Checked index construction

Data-v3 3.98 joins the source-selected `regex-automata 0.4.18`
`SmallIndex::new → TryFrom<usize>` chain to MAX, `as_usize`,
`Usize::as_u64` and `new_unchecked`. Original selections, contexts and independent
boundary expectations remain in
[the development corpus](../../tests/fixtures/function-families/checked-index-construction/README.md).

`rust.nominal-product-constants` covers original associated constants initialized
by the checked local scalar tuple constructor from reviewed `core::i32::MAX`,
a bounded usize subtraction and a u32 field conversion. Independent source
arithmetic must agree with compiler evaluation. Selected definitions, original
use spans, constructor/impl/declaration ownership, Copy proof and core dependency
are mandatory. The reviewed core/std display aliases retain the exact same core
definition checks. The graph contains a typed product value, without invented runtime
calls or effects. This is not general aggregate constant evaluation.

`rust.checked-concrete-tryfrom` covers exact original nongeneric local
`TryFrom<usize>` implementations returning a supported Copy scalar product and
named scalar error. Requested core member, selected local method, concrete
substitutions, associated Error, full impl roster and source signature must agree.
Success Copy and error no-drop receipts are independent. The existing helper
pipeline still verifies the complete body, build context and fresh Result flow.
Source closure checks neighboring expansions; generated prelude shadows reject.
No generated callable, generic trait solver, purity or blanket macro permission
follows from this contract.

Independent positives fix the limit at 2,147,483,646 and retain the exact attempted
u64 on error. Negatives alter values, bounds, source/definition/selection identity,
dependencies, error tags/operands, build context and historical profile versions;
ordinary integrity seals are refreshed before semantic rejection checks.
Routine consistency uses XXH3 or typed comparisons; SHA remains only for
historical claims and reviewed compiler-source pins.

Generated `PatternID::new` still needs callable/error expansion ownership,
nested success/error transport and ordered constructor `map`/`map_err` summaries.
Owned Result storage/reuse, general payload binding and arbitrary conversions
remain outside this boundary. Admission does not establish rewrite safety or
held-out detector quality.
