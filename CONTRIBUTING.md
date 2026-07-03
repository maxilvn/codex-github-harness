# Contributing

Contributions are welcome when they make the workflow clearer, safer, or easier
to adopt.

## Good Contributions

- Clearer `AGENTS.md` wording
- Better verification guidance
- Small, reusable skills
- Documentation for real-world setup variants
- Examples that reduce ambiguity

## Avoid

- Large rewrites without a concrete problem
- Tool-specific assumptions that do not generalize
- Extra automation that hides important git or verification steps
- Claims that Codex can safely merge or deploy without human review

## Pull Requests

Keep pull requests focused. Explain:

- What changed
- Why it improves the workflow
- How you checked the docs or examples

For docs-only changes, at minimum run:

```sh
git diff --check
```
