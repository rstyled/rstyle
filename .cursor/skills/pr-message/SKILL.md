---
name: pr-message
description: Write a PR description in Google engineering style (summary, background, what changed, testing with actual commands and results, risks, follow-ups) for the current branch's commits. Use when the user asks for a PR message, PR description, or pull request text.
---

# Write a PR message

Write a pull request description for the commits on the current branch, in the style used by Google engineering teams.

## Steps

1. Identify the branch and its base: `git branch --show-current`, then `git log --oneline <base>..HEAD` and `git diff --stat <base>..HEAD`. If the base is ambiguous, ask the user.
2. Read the actual diffs (not just stat) for each commit so every claim in the message is backed by code. Skim tests to report what is covered.
3. Write the message in Markdown, using this structure:

```markdown
## Summary

<2-4 sentences: what this PR does and why. No superlatives, no marketing.>

## Background

<The contract, bug, or gap that motivates the change. Link authoritative
sources: pinned revisions, schema paths, design docs.>

## What changed

<One bullet section per commit or area. State the behavior change and the
reason, not the code mechanics. Name new public APIs and new/changed
invariants.>

## Testing

<Exact commands run and their actual results (pass/fail counts, suites).
Include unit/integration tests, linters, and any self-check gates. Never
claim a test was run if it was not.>

## Risks and rollback

<What could break, who is affected, and how to revert.>

## Follow-ups (not in this PR)

<Known gaps left for later work, each with a one-line reason.>
```

4. Rules:
   - Output Markdown only: start with the PR title as `# <title>`, then the
     base line as a plain `Base:` line, then the `##` sections above. The
     whole message must be copy-pasteable into the GitHub PR body as-is.
   - Facts only. Every statement must trace to a commit, a test result, or a linked source. If something was not verified, put it under Follow-ups, not Testing.
   - Report exact commands and real output counts. Rerun the validation suite if the tree changed since the last run.
   - Keep the summary short; put details under headings. Use code formatting for identifiers, commands, and file paths.
   - Do not include emojis, praise, or filler. Do not invent risks or tests.
   - Do not push, merge, or open the PR unless the user asked. Output the message text only.
