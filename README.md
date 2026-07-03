# Codex Autonomous Setup

An opinionated open-source workflow for running Codex like a disciplined GitHub
coding agent.

This repository packages a reusable `AGENTS.md`, focused Codex skills, and
practical documentation for teams that want Codex to work through implementation
tasks end to end: isolate work, make changes, verify them, self-review the diff,
push a branch, and open a pull request.

## What This Is

Codex is most useful when the operating rules are explicit. This setup gives
Codex a consistent workflow for repository work:

- Use clean task branches and git worktrees.
- Keep unrelated dirty work out of feature changes.
- Verify changes before reporting done.
- Run a post-implementation self-review loop.
- Commit, push, and open a pull request when the task is intended for GitHub.
- Report the final branch, worktree, commit, and PR state clearly.

The goal is not to make Codex "magical." The goal is to make its behavior
predictable, reviewable, and close to how a careful human engineer would work.

## Quick Start

1. Copy `AGENTS.md` into the root of a repository where you use Codex.
2. Install the included skills or keep them in the repo as team documentation.
3. Adjust the GitHub workflow rules in `AGENTS.md` for your branch names,
   worktree directory, and merge policy.
4. Start Codex in the repository and ask it to implement a specific task.

For a lighter setup, start from `examples/AGENTS.minimal.md`.

## Skill Installation

To install the bundled skills for your local Codex setup:

```sh
mkdir -p ~/.codex/skills
cp -R skills/github-pr-workflow ~/.codex/skills/
cp -R skills/post-implementation-review ~/.codex/skills/
```

Then mention the skill by name in your task or reference it from your
`AGENTS.md`.

## Included Files

- `AGENTS.md` - the full autonomous workflow instruction set.
- `skills/github-pr-workflow/SKILL.md` - reusable GitHub branch, worktree,
  commit, push, and PR workflow.
- `skills/post-implementation-review/SKILL.md` - reusable post-change review and
  cleanup loop.
- `examples/AGENTS.minimal.md` - smaller version for teams that want fewer
  rules.
- `examples/AGENTS.full.md` - full version with skill references and stricter
  reporting.
- `docs/workflow.md` - how the workflow runs from task intake to PR.
- `docs/customization.md` - what to change before using this in your own
  environment.
- `docs/faq.md` - common questions and tradeoffs.
- `CONTRIBUTING.md` - contribution guidelines for improving the setup.

## Recommended Setup

Use the full `AGENTS.md` when:

- Codex is allowed to create branches and PRs.
- Your repo has meaningful tests, linting, or type checks.
- You want Codex to keep working through verification failures.
- You want a consistent completion report after changes.

Use the minimal example when:

- You mainly want local edits.
- You do not want Codex opening PRs automatically.
- You are still evaluating how much autonomy to allow.

## Requirements

- Git
- GitHub CLI (`gh`) for PR creation
- A GitHub account authenticated through `gh auth login`
- A repository with clear test, lint, and type-check commands

Codex can still use the workflow without GitHub CLI, but it should report that
PR creation could not run.

## Safety Model

This setup intentionally asks Codex to be autonomous, but not careless:

- It uses worktrees to isolate changes.
- It avoids mixing unrelated dirty files into commits.
- It requires diff review before completion.
- It requires verification before reporting done.
- It reports exact git state at the end of code-changing turns.

You should still review every pull request before merging it.

## License

MIT. See `LICENSE`.
