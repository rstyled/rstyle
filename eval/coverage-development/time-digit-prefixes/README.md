# Original time digit-prefix oracle

The frozen selection is the unchanged `time 0.3.47` methods
`ExactlyNDigits::<1/2/3>::parse`. All three are in scope by their original source
contract. Their current admission status does not define the expectation.
`selection.json` retains the audited XXH3 file/function identities, byte ranges,
concrete method identities and both feature contexts.

Verify the frozen files and source regions without building:

```sh
python3 eval/coverage-development/time-digit-prefixes/oracle.py
```

Run both native contexts when the coordinator schedules builds:

```sh
python3 eval/coverage-development/time-digit-prefixes/oracle.py --run-native --target-dir /tmp/rstyle-time-digit-prefix-oracle-target --output /tmp/rstyle-time-digit-prefix-oracle.json
```

Run one context independently with `--context default-parsing` or
`--context no-default-parsing`. The native runner requires the frozen
`nightly-2026-09-30` compiler commit and `aarch64-unknown-linux-gnu` host. Other
targets require separate frozen validation contexts. `--source-root` can point
to another byte-identical original crate installation. Python uses system
`libxxhash` to check the retained identities; Cargo runs offline. The temporary
runner receives its own Cargo lockfile; the original crate manifest and lockfile
are preserved byte for byte.

The runner copies the whole original crate into a temporary directory, then
appends the child in `oracle.rs` to the original combinator module and a public
root trampoline to `lib.rs`. A tiny external runner binary invokes that
trampoline through a path dependency. Both feature contexts use ordinary
`cargo run`; the original library stays `test=false`, so its dev-dependencies
are neither resolved nor compiled. Wrapper features forward `default` and
`parsing` explicitly to the dependency.

Every original copied byte, including the original manifest and lockfile, is
checked before and after execution. Only the two append-only instrumentation
regions may differ. The selected methods and all existing original source
bytes are unchanged. The appended child calls the private original methods
directly. It does not run a rewritten diagnostic parser.

The oracle uses a ten-entry byte table and a `u32` positional sum independent of
the original subtraction, casts and arithmetic sequence. Every observation
checks acceptance/value, exact remainder pointer/length/contents and unchanged
input backing. Per feature context, it checks:

- All 256 one-byte inputs and all 65,536 two-byte inputs for each selected width.
- All 1,000 valid three-digit prefixes with seven suffix variants and each width.
- Every possible suffix byte for each width, including NUL and non-UTF8 bytes.
- All 246 non-digit bytes at each prefix position, with three valid baselines
  and both empty and nonempty suffixes.
- Every short length with every repeated byte, plus exhaustive mixed two-byte
  short inputs for width three from the exhaustive domain above.
- Distinct equal-content allocations, nonzero-offset windows, exact-length
  one-past empty suffixes, and a saved remainder retained while a separate caller
  input binding changes.

The runner requires the complete 229,554-check inventory per context and zero
mismatches. Native behavior does not establish capture completeness, guarded
memory/origin correspondence, strict admission, matching, or support for the
upstream `parse_subsecond` caller. The test harness uses host `std`; in the
no-default context the original crate's `std` and `alloc` features remain off.
