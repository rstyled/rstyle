# Character escape classification

Connected development implementation validating, 2026-10-21; complete-original
acceptance pending. This separate development selection preserves the frozen 60.

Original: regex-syntax **0.8.11**, `src/lib.rs::is_escapeable_character(char)`
with required helper `is_meta_character`. The adjacent selection JSON freezes
both exact spans/digests, original file/manifest hashes and the same full pinned
default-feature Cargo context as `character-leaf-selection.json`: edition 2021,
nightly-2026-09-30 commit 5c543b0b8c73c7b72bc8284ced4fb22ead15734d,
aarch64-unknown-linux-gnu, debug, overflow checks, unwind. No target was chosen
because of its admission result.

Complete known chain: existing char scalar and metacharacter helper proof →
helper call and early return → exact core `char::is_ascii` with shared-autoref
scalar receiver → negation and early return → inclusive char literal ranges,
OR/literal/wildcard matching. No Result, Unicode tables, general traits or
ownership extension is required by this source. Current character proof rejects
`pat_range`; the ASCII call needs a bounded registered operation contract.

The pinned core implementation (`char/methods.rs`, line 1860) is
`pub const fn is_ascii(&self) -> bool { *self as u32 <= 0x7F }`, diagnostic item
`char_is_ascii`; pinned core file SHA-256
`17ced91be9e97808deffcb1686e213cbe18a71cd86b3b5243cc94c0cb6a9455f`.
Accept only the exact builtin authority and captured receiver
adjustment/value, never a familiar method name. Diagnostic capture finds a scalar
char receiver, one shared-borrow adjustment, signature `(&char)->bool`. The
method-only observational exception retains real `&char` evidence without opening
public reference vocabulary. Reviewed definition fingerprints (first,second):
Linux ARM `(5614226571623188065,6519353580426412939)`, Linux x86
`(2625310331494506890,16256494501121032715)`, macOS ARM
`(9963928104983068931,12767430517691631255)`; existing registry pins compiler,
target and core crate hash. Probe evidence `/tmp/rstyle-character-ascii-probe*.json`.
Complete-caller development source correspondence and strict tests pass; original
admission remains to be measured.

Independent expectation: true on precisely 64 ASCII scalar values, false on
all other valid Unicode scalars. The standalone oracle encodes the disjoint
numeric intervals `[0,47]`, `[58,59]`, `{61}`, `[63,64]`, `[91,96]`, `[123,127]`.
Their complement within ASCII is decimal digits, Latin letters and `<`/`>`.
It tests all 1,112,064 valid scalars, explicitly excludes surrogates and checks
that values above U+10FFFF are invalid. It calls the unchanged linked package;
it copies neither target body nor helper nor `is_ascii` logic. The existing
metacharacter oracle independently specifies its 18 positives.

Required negatives: non-ASCII scalars including U+0080, surrogate neighbors and
U+10FFFF; ASCII interval endpoints and neighbors; changed char type/value/range,
removed/altered source authority, wrong method receiver adjustment/identity,
wrong saved helper operand, and historical policy downgrade. Rebuild ordinary
integrity seals in semantic tampering cases. Reject invalid Unicode endpoints,
custom ASCII methods, unsupported casts and unrelated Unicode APIs.

Acceptance requires unchanged complete original Cargo capture, independent strict
verification/import, complete selected comparison evaluation with expectations
fixed separately, all applicable target/settings checks and quality gates. No new transport inventory or cache is proposed. Independent original-library
oracle passes all1112064 valid scalars with64 positives; log
`/tmp/rstyle-character-escapes-original-oracle.log`.

Development evidence: five positives/five unavailable neighbors across three
installed targets × std/no_std; all ten comparisons complete. Independent runtime
and resealed receiver/authority/type/saved-operand corruptions reject; old3.61
policy rejects. Four range functions add six comparisons and exhaustive numeric
boundary laws. Focused character tests pass; rstyle0/3070, patterns pending final
completion. Full checkpoint and original acceptance are pending. The first full
run passed944 tests with only the known spacing gate failing; corrected before
final validation. Logs `/tmp/rstyle-character-escapes-{final-focus.log,final-style.json}`.
