## Why

`expr.rs` and `mapping.rs` contain pure logic (RPN evaluation, TOML config loading) that currently lives inside the `xplane_uipc` plugin crate, which requires the Windows SDK and X-Plane SDK bindgen to compile. This prevents testing these modules on non-Windows platforms and prevents reuse by a standalone debugging CLI tool. Extracting them into separate workspace crates decouples pure logic from platform-specific plugin code.

## What Changes

- Extract `xplane_uipc/src/expr.rs` into a new `uipc-expr` workspace crate
- Extract `xplane_uipc/src/mapping.rs` into a new `uipc-mapping` workspace crate
- Move `FsuipcType` and `FSUIPC_DATA_SIZE` from `xplane_uipc/src/fsuipc_offsets.rs` into `uipc-mapping`
- `uipc-mapping` re-exports `uipc_expr::Expr`
- Add both new crates to the workspace `members` list
- Update `xplane_uipc` to depend on `uipc-mapping` instead of its local modules
- Remove the local `expr` and `mapping` modules from `xplane_uipc/src/`

## Capabilities

### New Capabilities

*(None — this is a structural refactor. No user-facing capabilities are introduced.)*

### Modified Capabilities

*(None — no existing spec-level requirements change.)*

## Impact

- **Workspace**: grows from 3 members to 5 (`uipc-expr`, `uipc-mapping` added)
- **`xplane_uipc`**: loses `expr.rs` and `mapping.rs` modules; `fsuipc_offsets.rs` keeps `read_value`, `write_value`, well-known offset constants (now imports `FsuipcType`/`FSUIPC_DATA_SIZE` from `uipc-mapping`)
- **`plugin_state.rs`**: imports from `uipc-mapping` instead of `crate::mapping`
- **`uipc-expr`**: zero dependencies, pure Rust
- **`uipc-mapping`**: depends on `uipc-expr`, `serde`, `toml`
- **Side effects**: none — public API and behavior unchanged
