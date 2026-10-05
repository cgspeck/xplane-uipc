# Agents

## Verification

Before declaring work complete, run the formatter, clippy, tests and a dist build. CI runs the same checks and fails on any clippy warning.

On Windows:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test
cargo xtask dist
```

On Linux, run `make`. It runs fmt, clippy, the cross-compile check and the tests that can run there. The Windows crates can't link or run on Linux, so a dist build and the `ipc_host`/`xplane_uipc` tests still need a Windows machine or CI.

## Committing Changes

Agents (and peers) may run `git commit`. When you do, group changes into logical, self-contained commits rather than one large commit for a whole task:

- Each commit should cover one coherent piece of work (e.g. one component, one endpoint, one refactor) so it can be reviewed on its own.
- Prefer several small commits over one large one — this makes review easier group-by-group, and if a merge conflict happens later, it's easier to cherry-pick or apply the parts that merge clean and resolve just the parts that don't.
- Don't bundle unrelated changes (e.g. a feature change and an unrelated lint fix) into the same commit. A behavioral fix and its new regression test are separate commits.
- Give each commit a message that describes that commit's own change, not the whole task.
- Never add a `Co-Authored-By: ...` line or other AI attribution (such as a "Generated with Claude Code" footer or a session link) to commit messages or PR descriptions.

## Use openspec for non-trivial changes

This repo has [openspec](openspec/config.yaml) set up for spec-driven changes. For anything beyond a trivial fix or tweak, propose the change via the openspec workflow (e.g. `openspec-propose`) before implementing, so there's a spec/task trail to point to. Trivial changes (typos, small refactors, doc tweaks) don't need this ceremony.
