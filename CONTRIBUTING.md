# Commits

PRs into `main` are squash-merged through the merge queue, so the PR title becomes the commit subject.

- Use conventional-commit subjects (`feat(ts): ...`, `fix: ...`, `chore: ...`, `docs: ...`).
- AI commit attribution goes in a `Co-Authored-By:` trailer, not the commit body.
- Never commit binaries or build artifacts.

# PRs

Keep the body short and structured, not narrated.
Have at least these sections:

- **Problem**: a summary of the problem and why this PR is needed.
- **Approach**: a summary of the approach taken to solve the problem.
- **Impact**: a bullet point for every public API change. A breaking one also gets a `MIGRATE.md` entry.
- **Alternatives**: any alternative approaches considered.
- **Follow-ups**: any issues encountered or quests created.

Never rebase or force-push a pushed branch; merge the base branch in instead.
Open a draft PR while working, run `just ci` before every push, and mark the PR ready yourself when the work is done.

# Merge queue

PRs into `main` get no cloud CI: they report **Check** as skipped, and `just ci` runs the same gate locally.
The merge queue runs Check on the PR combined with the latest `main` and the PRs queued ahead of it.
Enqueue the reviewed head with `quest merge <pr> <token> --land <sha>` (`/quest-merge` does this).
A dequeued PR failed that combined check, timed out, or no longer meets the ruleset: read why in the PR timeline, fix the cause, and enqueue again.
The admin bypass of `main`'s ruleset is for emergencies.

# Reviews

Review each PR locally with `/quest-review`, as the quest guide describes; merging never waits on an online reviewer, and never request one.
The PR body's `## Review` record counts as the required review when it names the head being merged, and so does a recorded skip of a change too trivial to review.
Address online reviewers' findings already posted on that head before merging: fix them, or reply where you disagree.

# Follow-ups

Record out-of-scope findings as follow-up quests with `/quest-plan`, not GitHub issues.

# Forks

A hard fork never merges upstream: fixes are cherry-picked by hand, and each carried change is listed in the fork's README with its upstream PR or the reason it has none.
Each imported component keeps its own license and records its upstream source and revision.
