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

6. Commit with a semantic (Conventional Commits) message:
   ```
   refactor(<module>): <what changed, imperative, <=72 chars>

   <Which functions were duplicates, which one survives, and the rstyle
   evidence: candidate kind, support, holes.> No behavior change.
   ```
   Type: `refactor` for dedupe; scope is the module that lost code. The subject describes the change, not the tool.
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
refactor(sigmine): reuse shape::short instead of a duplicate path-tail helper

`sigmine::short_name` was an exact clone of `shape::short`: both return the
last `::` segment of a path. `rstyle patterns` flagged it as the top
parameterize candidate (support 3, zero holes).

Drop `short_name`, widen `shape::short` to `pub(crate)`, and call it from
`shared_name_types`. No behavior change.
```
