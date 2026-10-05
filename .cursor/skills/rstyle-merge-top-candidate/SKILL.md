---
name: rstyle-merge-top-candidate
description: Run `rstyle patterns`, take the top-ranked duplication candidate, refactor it away, and open a PR with a semantic commit message describing what changed. Use when the user asks to fix/merge/take off the top rstyle patterns candidate.
disable-model-invocation: true
---

# Merge the top `rstyle patterns` candidate

## Steps

1. Build and run (from repo root):
   ```bash
   cargo build -q -p rstyle-driver -p cargo-rstyle
   ./target/debug/cargo-rstyle rstyle patterns
   ```
   `cargo run` alone fails: the driver binary must sit next to `cargo-rstyle`.

2. Take candidate `#1`. Read every listed site and judge from the code, not the label:
   - `parameterize` / `holes 0` (exact clone): keep one definition, delete the rest, reuse it.
   - `generic_fn`: one generic definition over the differing type.
   - Skip to the next candidate if sites are clones only by coincidence (different meaning that happens to share shape).

3. Branch from `main` (not the current branch): `git checkout -b refactor/<slug> main`. Candidates can reference code not yet on `main`; re-check each site exists there and drop the ones that don't.

4. Refactor minimally. Widen visibility only as far as needed (`pub(crate)`). No behavior change.

5. Verify: `cargo test -q -p rstyle-core && cargo clippy -q -p rstyle-core`. Re-run `rstyle patterns` and confirm the candidate is gone.

6. Commit with a plain-text Conventional Commits message. No markdown (no backticks, bullets, or headings):
   - Subject: `type(scope): imperative summary`, 72 chars max. Type is `refactor` for a dedupe. Scope is the module or crate that lost the duplicate code.
   - Body (wrapped at about 72): motivation first (what was duplicated and why that costs something), then what the change does, then "No behavior change." when true.
   - Do not mention rstyle or its scores unless the tool itself is what changed. The reader cares about the code, not how it was found.
   If GPG signing fails (no tty), ask the user before using `--no-gpg-sign`.

7. Push and open the PR against `main` with `tea`:
   ```bash
   git push -u gitea-origin HEAD
   tea pr create --login agentstuff --repo zcstarr/rstyle --base main \
     --head refactor/<slug> --title "<commit subject>" --description "<commit body>"
   ```
   Change an existing PR's base/title via `tea api -X PATCH repos/zcstarr/rstyle/pulls/<n> -f base=main -f title=...` (`tea pr edit` has no base flag).

8. Append an entry to `LLMLOG.MD` (gitignored; local only).

## Example

```
refactor(driver): share one span-to-line helper across fact collectors

effects, pdg and mutations each carried their own copy of the same
span-to-line lookup (outermost macro call site, 1-based line). A change
to how lines are resolved had to be made in three places.

Add extract::line_of and call it from all three. No behavior change.
```
