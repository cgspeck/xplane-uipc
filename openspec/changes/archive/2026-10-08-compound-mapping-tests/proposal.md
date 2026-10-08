# Proposal

## Why

Many mappings in `mappings.toml` pack several things into one offset. The lights bitmask `0x0D0C` has seven bits plus per-aircraft write commands. Other examples are the doors bitmask, the battery OR across aircraft, the time writes that must keep the other fields of `zulu_time_sec`, and the piecewise spoiler encoding. Editing one part of these expressions can silently break another part, and nothing catches it until someone notices in the sim. Only the loader is tested today: `shipped_mappings_load_cleanly` proves the file parses, not that the expressions give the right numbers.

## What Changes

- Add a test-case file, `xplane_uipc/mappings.tests.toml`, next to `mappings.toml`. Each `[[case]]` names an offset and is one of two kinds:
  - **Read case:** given dataref values (by the mapping's `datarefs` names), the offset serves this value, or retains its last value when `update_if_expr` is false.
  - **Write case:** given dataref values and a client write, these are the exact dataref writes and command runs, or the write is blocked.
- Add a test runner that `cargo test` runs. It evaluates every case against the shipped mappings and reports each failing case by offset and name.
- Add a coverage check that fails `cargo test` when a compound mapping isn't fully covered. A compound mapping is an expression mapping with two or more datarefs, or with `write_exprs`/`write_commands`. Full coverage means:
  - it has a read case, and a write case if it's writable;
  - each dataref its read expression uses is non-zero in some read case;
  - each write target is written non-zero, and each command runs, in some write case.
- Write cases must list every write target, so a write to one part that changes another part fails the test. Read isolation is covered by explicit before/after read cases for each part.
- Cases that name an unknown offset, a non-compound mapping, or a dataref name the mapping doesn't have are errors, so stale cases don't linger.
- Write cases for every compound mapping currently in `mappings.toml`.
- `mappings.tests.toml` is not shipped in `dist`.

## Capabilities

### New Capabilities

- `compound-mapping-tests`: the mapping test-case file format, how read and write cases are evaluated, and the coverage rules that compound mappings must meet.

### Modified Capabilities

(none. Plugin behaviour doesn't change.)

## Impact

- New: `xplane_uipc/mappings.tests.toml` and `uipc-mapping/tests/compound_mappings.rs`, an integration test. It runs natively on Linux and Windows, so `make test` and CI pick it up.
- `uipc-mapping`: may need small public helpers to evaluate a mapping's read expression and `update_if_expr`. `eval_writes` already covers writes.
- Docs: a short section in the `mappings.toml` header and in the README on adding cases.
- Contributors who add or change a compound mapping must add or update its cases.
