# Project list

## 1. Telemetry ingestion workspace

**Shape:** Cargo workspace with three crates: `telemetry-core`, `file-agent`, and `http-agent`.

**Scenario:** Two small ingestion agents normalize incoming telemetry and write it to a shared sink.

**Reuse already present:**

- `EventSink` trait shared by both agents.
- Shared `Event` domain type and `VecSink` test/demo implementation.
- `event!` macro for constructing events consistently.

**Deliberate refactoring opportunities:**

- Both agents own nearly identical three-attempt write/retry loops.
- Both perform input normalization and timestamp derivation in locally named helpers.
- Error construction is similar but not expressed through a shared ingestion outcome/policy.

## 2. Backup synchronizer

**Shape:** Single small binary crate split into scanner, checksum, store, and sync modules.

**Scenario:** Files discovered under a demo directory are converted to backup keys, checksummed, and uploaded to an object store.

**Reuse already present:**

- `ObjectStore` trait separates synchronization logic from storage implementation.
- `progress!` macro gives status messages one consistent shape.

**Deliberate refactoring opportunities:**

- Path-to-object-key normalization exists independently in scanning and synchronization code.
- Checksum and upload code each implement almost the same fixed-size byte-chunk traversal.
- Backup identity is spread across `Path`, plain `String` keys, checksum values, and conflict naming.

## 3. Queue worker

**Shape:** Single binary crate with a registry plus email and image worker modules.

**Scenario:** A small job runner dispatches payloads to typed handlers.

**Reuse already present:**

- `Handler` trait gives the runner a common dispatch interface.
- `register_jobs!` macro keeps registry setup terse.

**Deliberate refactoring opportunities:**

- Email and image jobs model success/retry/permanent failure differently even though the runner needs the same operational information.
- Retry delay calculation is duplicated as different local policies.
- Metric labels are assembled as repeated strings in each worker.

## 4. Feature gate evaluator

**Shape:** Single binary crate with two rule sources and an evaluator.

**Scenario:** Request overrides and configuration rules participate in deciding whether a feature is enabled.

**Reuse already present:**

- `RuleSource` trait lets the evaluator process heterogeneous sources.
- `audit!` macro centralizes the audit log format.

**Deliberate refactoring opportunities:**

- Rule sources both do lookup/match/decision work, but represent “no opinion” inconsistently (`None` vs `Decision::Pass`).
- Decision provenance is reconstructed later by the evaluator rather than being returned with the decision.
- Similar matching semantics could potentially compile into a common rule representation.
