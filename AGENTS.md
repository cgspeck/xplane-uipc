# Agents

## Verification

Before declaring work complete, always run formatter, tests, build and dist, e.g.

```bash
cargo fmt
cargo test
cargo build
cargo xtask dist
```

## Committing Changes

Agents (and peers) may run `git commit`. When you do, group changes into logical, self-contained commits rather than one large commit for a whole task:

- Each commit should cover one coherent piece of work (e.g. one component, one endpoint, one refactor) so it can be reviewed on its own.
- Prefer several small commits over one large one — this makes review easier group-by-group, and if a merge conflict happens later, it's easier to cherry-pick or apply the parts that merge clean and resolve just the parts that don't.
- Don't bundle unrelated changes (e.g. a feature change and an unrelated lint fix) into the same commit.
- Give each commit a message that describes that commit's own change, not the whole task.

Do not add `Co-Authored-By: ...` to the commit message.

## Use openspec for non-trivial changes

This repo has [openspec](openspec/config.yaml) set up for spec-driven changes. For anything beyond a trivial fix or tweak, propose the change via the openspec workflow (e.g. `openspec-propose`) before implementing, so there's a spec/task trail to point to. Trivial changes (typos, small refactors, doc tweaks) don't need this ceremony.

## Git / Commits

- Commit changes in logical groups rather than one large commit (e.g. a
  behavioral fix and its new regression test are separate commits).
- Never add a `Co-Authored-By: Claude ...` (or similar AI attribution) line,
  or a "Generated with Claude Code" footer, to commit messages or PR
  descriptions.