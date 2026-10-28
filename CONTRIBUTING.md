# Commits

PRs land on `main` only through `quest land`, each as one squash commit: the PR title becomes the commit subject and the PR's commit messages become the body.
Queue a PR with `quest merge <pr> <token> --land <sha>` (`/quest-merge` does this), and the Land workflow runs Check's steps once on the batch, landing the longest passing prefix.
The admin bypass of `main`'s ruleset is for emergencies.

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
Open a draft PR while working, run `just check` and `just test` before every push, and mark the PR ready yourself when the work is done.

# Reviews

AI agents review pushes on their own. Never explicitly request a review.

For each finding:

- If you don't agree with it, reply to the finding and move on.
- If it's a relatively easy improvement, fix it and push.

Queue a PR only after a review from a reviewer other than Grok has no findings, or every finding is fixed or replied to.

# Follow-ups

Record out-of-scope findings as follow-up quests with `/quest-plan`, not GitHub issues.

# Forks

A hard fork never merges upstream: fixes are cherry-picked by hand, and each carried change is listed in the fork's README with its upstream PR or the reason it has none.
Each imported component keeps its own license and records its upstream source and revision.
