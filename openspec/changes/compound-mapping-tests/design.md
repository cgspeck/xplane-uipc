# Design

## Context

The pieces already exist:

- `uipc_mapping::load_mappings` parses `mappings.toml` into `DatarefMapping`s. `MappingSource::Expr` holds `datarefs`, `expr`, `update_if_expr`, `write_exprs` and `write_commands`.
- `uipc_mapping::eval_writes` evaluates a write against one snapshot exactly as the plugin does, and returns a `WritePlan` or the non-finite error.
- `Expr::eval` evaluates a read. The plugin then converts the result with `Value::from_f64` (round, then wrap) in `ipc_host`.
- `uipc-debug` (the "mapping test program") has an `EvalEngine` that does the read side against a state CSV keyed by dataref path. It is a binary-only crate and has no write side.

## Goals / Non-Goals

**Goals:**

- A data file of cases that someone editing `mappings.toml` can extend without writing Rust.
- The runner fails on missing coverage, so new compound mappings can't land untested.
- It runs on Linux under `make test` and in CI.

**Non-Goals:**

- Automatic isolation checks, such as flipping each input and diffing the output. You chose explicit cases. Isolation for writes still comes from requiring every write target in each write case.
- Testing simple or static mappings, or the plugin's dataref I/O, threading or IPC.
- Testing a user's own `mappings.toml` in the sim folder. Only the shipped file is tested.

## Decisions

### The runner is an integration test in `uipc-mapping`

`uipc-mapping/tests/compound_mappings.rs` loads `../xplane_uipc/mappings.toml` and `../xplane_uipc/mappings.tests.toml`. `uipc-mapping` is portable (no Windows deps), already owns the loader and `eval_writes`, and `make test` already runs it.

Alternative considered: running the cases from `uipc-debug` and its `EvalEngine`. That would need a library target, a write-side engine and path-keyed state. `EvalEngine::evaluate_one`'s expression arm reduces to `expr.eval(&vars)` once the state is keyed by name, so it adds nothing here. `uipc-debug` could load the case file later as a state source, but that isn't part of this change.

### State is keyed by `datarefs` name, not dataref path

Cases read like the mapping (`{ Nav = 1, Bcn = 1 }`), and the same path under two names (as in the battery mappings) can't be confused. Missing names are `0`, which matches the plugin's handling of unavailable datarefs in expressions. Since one name per mapping maps to one variable, `array_index` needs no special handling.

### Served-value comparison is reimplemented

`Value::from_f64` lives in `ipc_host`, which is Windows-only. The runner applies the same rule as the `offset-value-semantics` spec: round half away from zero, cast through `i64` and wrap to the type width, or compare with a relative tolerance for `f32`/`f64`. It's about ten lines. Moving the conversion into `uipc-mapping` would mean a refactor of `ipc_host`, which is out of scope.

### Case file shape

```toml
[[case]]
offset = 0x0D0C
name   = "beacon only"
state  = { Bcn = 1 }
expect = 0x0002

[[case]]
offset = 0x0D0C
name   = "writing beacon on keeps landing on"
state  = { Land = 1 }
write  = 0x0006
expect_datarefs = { Nav = 0, Bcn = 1, Land = 1, Taxi = 0, Strb = 0 }
expect_commands = { "laminar/B738/toggle_switch/position_light_off" = 1, "laminar/B738/toggle_switch/taxi_light_brightness_off" = 1 }
```

The file is parsed with `serde` and `deny_unknown_fields`, so a misspelled key such as `expect_dataref` is a parse error. Exactly one of `expect`, `expect_retain` (read) or `expect_datarefs`/`expect_blocked` (write) must be present. Cases are grouped under a comment header per offset, in the same order as `mappings.toml`.

### Coverage is derived, not declared

The runner works out the compound set from the loaded mappings, and the parts from `expr.vars()`, the `write_exprs` keys and the `write_commands` keys. Nothing has to be registered by hand, so a new mapping or a new bit is caught automatically. "Exercised" means non-zero in some case. That's a cheap check that each part has been turned on at least once.

### One test, all failures

The runner collects failures into a `Vec<String>` and asserts it's empty at the end, so a broken edit shows every affected case at once rather than one per run.

## Risks / Trade-offs

- [Cases encode current behaviour, including known quirks such as "generic battery always seems to be on"] → Cases document what the mapping does now, and a quirk fix updates the case. A case `name` can say "quirk:".
- [The `feat/BCD-expr` branch adds writable radio mappings, which are compound] → Whichever branch merges second needs cases for them. The coverage test will name the offsets.
- [Non-zero coverage doesn't prove every bit combination] → It's accepted. Isolation for each part is covered by explicit cases, which the review of each mapping change should check.

## Open Questions

None.
