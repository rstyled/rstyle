# Stream executable identities

After expanded-evidence retirement (`2c27062`), a minimal clones run still spent
about two seconds hashing its executables. `capture::hash_file` read each entire
binary into a temporary vector before SHA-256: the debug driver was about
175 MiB and the CLI about 157 MiB. The buffers were sequential, but their peak
allocation dominated this small command's memory use.

Hashing now opens the file and copies its bytes into the SHA-256 writer using
`std::io::copy`, retaining a bounded streaming buffer rather than a binary-sized
vector. Every invocation still hashes all bytes. There is no metadata cache or
new reuse policy. Hex encoding and SHA-256 identity meaning are unchanged;
read errors retain the file path. The existing empty, `abc`, and million-`a`
known-vector regression now also checks a same-length edit (`abc` to `abd`).

## Measurements

Linux aarch64, pinned nightly-2026-09-30, locked dependencies, existing debug
profile, eight-CPU quota and 16 GiB memory. No concurrent builds or tests ran.
The baseline CLI executable was copied before rebuilding; both variants used
the same unchanged driver through `RSTYLE_DRIVER`. Each command used a fresh
two-function fixture and isolated Cargo target directory, with warm executable
page-cache conditions. Builds of the tool itself were excluded. Three
alternating before/after samples used `/usr/bin/time` and `RSTYLE_PERF=1`;
instrumentation was identical for both variants. All reports completed pair
evaluation successfully. Nested stages are not added to total command time.

| Measurement (median of three) | Before | After |
| --- | ---: | ---: |
| Command wall seconds | 2.27 | 2.19 |
| Peak RSS KiB reported by time | 188,488 | 73,764 |
| Driver plus CLI hashing seconds | 2.042823 | 1.946954 |

RSS falls about 61% on this workload. The small runtime change is local evidence,
not a general speedup claim. Streaming still performs the same full SHA-256
work; compiler startup, verification and matching remain. The structural change
is removal of two sequential executable-sized allocations per clones command.

Measurement script and raw JSONL are retained locally at
`/tmp/rstyle-stream-hash-measure.py`, `/tmp/rstyle-stream-hash-single.py` and
`/tmp/rstyle-stream-hash-alternating.jsonl`; per-command diagnostic logs use
`/tmp/rstyle-stream-hash-{before,after}-alternate-{0,1,2}-0.err`.

## Validation

All commands completed successfully on this revision:

- `cargo fmt --all -- --check`.
- `cargo test -p cargo-rstyle --locked --bin cargo-rstyle executable_digest_matches_standard_vectors_and_recaptures_edits`.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`.
- `CARGO_TARGET_DIR=target timeout 600 ./scripts/test.sh --nextest`: 642 passed,
  zero skipped, 108.674 seconds; workspace doctests passed. No slow warnings.
- `./scripts/style.sh --format json`: 1,970 functions, zero violations.
- `./scripts/style.sh patterns --fail-on-candidates --show-ignored`: zero
  actionable candidates, 15 existing reviewed ignored candidates.

Logs use `/tmp/rstyle-stream-hash-{fmt,focused,clippy,nextest,patterns}.log`,
`/tmp/rstyle-stream-hash-style.json` and `/tmp/rstyle-stream-hash-style.err`.
The complete-suite duration is an observation, not a controlled suite speedup.
No rules, ignores, thresholds or test-concurrency settings changed.
