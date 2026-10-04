# Refactoring concerns that could be unified

These examples intentionally stop one step short of a few useful abstractions. The goal is to leave enough friction to discuss *when* a unification is justified, what shape it could take, and what might be lost by forcing unrelated code behind one interface.

## 1. `telemetry_workspace`: ingestion policy

The workspace already shares `Event`, `EventSink`, and the `event!` macro through `telemetry-core`, so basic domain vocabulary is centralized.

What is still separate:

- `file-agent` trims, validates, timestamps, and retries failed writes in its own sequence.
- `http-agent` performs almost the same stages, but calls them `normalize_body`, `received_at`, and `send_with_retry`.
- Both agents decide whether an input is empty, how to derive a timestamp-like value, how many attempts to make, and how to turn a failure into a message.

Possible unifying concept:

- An `IngestionPolicy` or `IngestionPipeline` that owns normalization, timestamping, and retry behavior.
- A smaller alternative would be a reusable `RetryPolicy` plus one normalization helper, keeping the pipelines separate.

Questions worth asking before refactoring:

- Are file and HTTP ingestion really expected to evolve together?
- Is retry an infrastructure concern while normalization is a domain concern, meaning they should *not* be one abstraction?
- Would a trait with hooks make control flow harder to read than two explicit functions?

## 2. `backup_sync`: object identity and chunk traversal

The project already has an `ObjectStore` trait, so local and remote storage use the same basic API. The `progress!` macro also centralizes a tiny piece of repeated status output.

What is still separate:

- `scanner::to_backup_key` and `sync::remote_key_for` both convert a path into a slash-separated object key, but with slightly different signatures and assumptions.
- `checksum::rolling_checksum` and `sync::upload_in_chunks` both walk a byte slice in fixed-size chunks with nearly identical loop structure.
- Conflict naming is string-based in `sync`, while path scanning works with `Path`/`PathBuf`.

Possible unifying concept:

- A `BackupKey` newtype responsible for canonical path-to-key conversion.
- A `Chunks` iterator/helper used by checksum and upload code.
- Potentially a `BackupObject` value object combining source path, backup key, size, and checksum.

Questions worth asking before refactoring:

- Is the apparent chunking duplication stable, or could checksum and transport chunk sizes diverge?
- Should key normalization happen at the boundary of the store API instead of in scanning/sync code?

## 3. `queue_worker`: execution outcome and retry semantics

The project already shares job behavior through the `Handler` trait. The `register_jobs!` macro keeps the registry setup concise.

What is still separate:

- `email_worker` and `image_worker` both classify failures into retryable vs permanent outcomes, but use different enums and branching shapes.
- Each worker computes its own retry delay using a slightly different formula.
- Both workers manually build metric labels such as `job=email,status=retry`.

Possible unifying concept:

- A shared `JobOutcome` enum such as `Completed`, `RetryAfter(Duration)`, and `Rejected`.
- A `RetryPolicy` trait or data type that maps attempt number + error class to a retry decision.
- A typed metrics label builder or `JobMetrics` helper.

Questions worth asking before refactoring:

- Would a common outcome erase useful worker-specific error information?
- Should retry decisions be made by the handler, by the runner, or by a policy injected into the runner?

## 4. `feature_gate`: rule evaluation and decision provenance

The project already shares flag evaluation through the `RuleSource` trait and uses `audit!` for consistent audit messages.

What is still separate:

- `config_rules` and `request_rules` both perform a lookup, compare an attribute, and return a boolean-like decision.
- Each source has its own interpretation of “no opinion”: one uses `None`, another uses `Decision::Pass`.
- The evaluator separately constructs human-readable reasons after the source has already made its decision.

Possible unifying concept:

- A `RuleDecision { effect, reason, source }` value returned by every rule source.
- A small rule algebra such as `Equals`, `Present`, and `Percentage` that multiple sources compile into.
- A `DecisionTrace` that records provenance without coupling the evaluator to each source’s implementation details.

Questions worth asking before refactoring:

- Do all rule sources truly have the same semantics, or is “no opinion” materially different from “explicit pass”?
- Would a rule algebra simplify the system, or merely move source-specific complexity into a generic enum?

## Cross-project themes

A few patterns repeat across the examples and could themselves become discussion topics:

- **Policy vs mechanism** — retry, normalization, and evaluation rules are mixed into otherwise straightforward orchestration code.
- **Stringly typed identity** — backup keys, metric labels, reasons, and source names are often plain `String`s even when they have domain meaning.
- **Outcome types** — multiple places encode success/retry/reject/pass using project-local enums or booleans.
- **Boundary normalization** — several projects normalize data near call sites rather than at one obvious system boundary.
- **Control-flow abstraction** — traits are already used for substitutable behavior, but repeated *sequences of steps* are mostly left explicit.

A useful refactoring exercise is to avoid unifying everything at once. Pick one concept, state the invariant it should own, refactor only the duplicated behavior that actually shares that invariant, and then see whether the call sites became easier to reason about.
