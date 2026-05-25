## Context

The `xplane_uipc` crate is an X-Plane plugin — it requires the Windows SDK, X-Plane SDK bindgen, and the XPLM runtime to compile. Two of its modules, `expr` and `mapping`, contain pure logic with no platform dependencies:

- `expr` — RPN parser/evaluator (stdlib only, zero deps)
- `mapping` — TOML config loader for dataref→FSUIPC mappings (depends on serde, toml, and `Expr` + `FsuipcType` from sibling modules)

Extracting them into workspace-local crates makes them compilable and testable on any platform, and reusable by future tools (e.g., a standalone CLI debugger).

Current workspace:

```
workspace members = ["ipc_host", "xplane_uipc", "xtask"]
```

## Goals / Non-Goals

**Goals:**
- `expr` as a standalone zero-dependency crate (`uipc-expr`)
- `mapping` + `FsuipcType` as a standalone crate (`uipc-mapping`)
- Both crates compilable and testable without Windows SDK or X-Plane SDK
- `xplane_uipc` continues to build and pass all tests unchanged in behavior
- `FsuipcType` and `FSUIPC_DATA_SIZE` live in `uipc-mapping`, not in `xplane_uipc`

**Non-Goals:**
- No new generic resolver or evaluation abstraction (each consumer implements its own resolver)
- No changes to the TOML mapping file format or public API of `load_mappings`
- No changes to `ipc_host` or `xtask`
- No user-facing behavior changes

## Decisions

### 1. Crate placement and naming

Prefix `uipc-` to group with project identity; placed at workspace root alongside `ipc_host`.

```
xplane-uipc/
├── Cargo.toml          (workspace: + uipc-expr, uipc-mapping)
├── uipc-expr/
│   ├── Cargo.toml
│   └── src/lib.rs
├── uipc-mapping/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs       ← re-exports + pub use
│       ├── mapping.rs   ← from xplane_uipc/src/mapping.rs
│       └── types.rs     ← FsuipcType, FSUIPC_DATA_SIZE
├── xplane_uipc/         ← slimmed: loses expr.rs, mapping.rs
│   ├── Cargo.toml       (dep: uipc-mapping)
│   └── src/
│       ├── fsuipc_offsets.rs  ← keeps read_value/write_value/offset consts
│       ├── plugin_state.rs   ← imports from uipc-mapping
│       └── ...
├── ipc_host/
└── xtask/
```

### 2. Re-export `Expr` from `uipc-mapping`

`uipc-mapping` re-exports `pub use uipc_expr::Expr;` so downstream crates get the type without a second dependency. `xplane_uipc` depends on `uipc-mapping` only.

### 3. What stays in `xplane_uipc/src/fsuipc_offsets.rs`

After extracting `FsuipcType` and `FSUIPC_DATA_SIZE`, the file keeps:
- `FSUIPC_SHM_NAME` — name of Win32 shared memory object
- All `OFFSET_*` well-known offset constants
- `write_value()`, `read_value()`, `write_u32()`, `write_i32()`, `write_u16()` — operate on `[u8; FSUIPC_DATA_SIZE]`

It imports `use uipc_mapping::{FsuipcType, FSUIPC_DATA_SIZE};`.

### 4. `uipc-mapping` internal structure

| File | Contents |
|------|----------|
| `src/lib.rs` | Module declarations, `pub use types::*`, `pub use mapping::*`, `pub use uipc_expr::Expr` |
| `src/types.rs` | `FsuipcType` enum + `size()` + `FromStr`, `FSUIPC_DATA_SIZE` constant |
| `src/mapping.rs` | `MappingSource`, `DatarefMapping`, `MappingConfig`, `GlobalSettings`, `load_mappings()`, `parse_dataref_with_index()` — adapted from `xplane_uipc/src/mapping.rs` |

The `mapping.rs` module changes imports from `crate::expr::Expr` / `crate::fsuipc_offsets::*` to `crate::Expr` / `crate::FsuipcType` / `crate::FSUIPC_DATA_SIZE` (all available via `lib.rs` re-exports).

### 5. `uipc-expr` Cargo.toml

```toml
[package]
name = "uipc-expr"
version = "0.1.0"
edition = "2024"
```

No dependencies. No `[lib]` section needed (default rlib).

### 6. `uipc-mapping` Cargo.toml

```toml
[package]
name = "uipc-mapping"
version = "0.1.0"
edition = "2024"

[dependencies]
uipc-expr = { path = "../uipc-expr" }
serde = { version = "1.0", features = ["derive"] }
toml = "=0.7.8"
```

### 7. `xplane_uipc` dependency update

Add `uipc-mapping = { path = "../uipc-mapping" }` to `[dependencies]`, remove local `mod expr; mod mapping;`.

## Risks / Trade-offs

| Risk | Mitigation |
|------|------------|
| **Version drift** between `uipc-expr`/`uipc-mapping` and the plugin crate | Workspace-local path deps keep them in lockstep; bump all together if needed |
| **Tests get left behind** during the move | Tests move with the source code into the new crates; no logic changes |
| **Circular dep if `ipc_host` later wants `FsuipcType`** | Won't happen — `ipc_host` has its own `Value` types unrelated to FSUIPC typing. If needed later, `FsuipcType` could move to its own crate |
| **`uipc-mapping` crate feels mixed** (config loader + type definitions) | Conscious choice to keep the split simple (2 new crates total). Acceptable. |
