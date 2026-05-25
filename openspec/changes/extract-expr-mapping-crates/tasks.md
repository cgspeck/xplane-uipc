## 1. Create uipc-expr crate

- [ ] 1.1 Create `uipc-expr/Cargo.toml` with package name `uipc-expr`, edition 2024, no dependencies
- [ ] 1.2 Create `uipc-expr/src/lib.rs` from `xplane_uipc/src/expr.rs` (identical content — no import changes needed)
- [ ] 1.3 Add `uipc-expr` to workspace `members` in root `Cargo.toml`
- [ ] 1.4 Verify: `cargo build -p uipc-expr` and `cargo test -p uipc-expr` pass

## 2. Create uipc-mapping crate

- [ ] 2.1 Create `uipc-mapping/Cargo.toml` with deps: `uipc-expr`, `serde` (derive), `toml`
- [ ] 2.2 Create `uipc-mapping/src/types.rs` — extract `FsuipcType` (enum + `size()` + `FromStr`) and `FSUIPC_DATA_SIZE` from `fsuipc_offsets.rs`
- [ ] 2.3 Create `uipc-mapping/src/mapping.rs` — adapted from `xplane_uipc/src/mapping.rs`; change imports from `crate::expr::Expr` → `crate::Expr` and `crate::fsuipc_offsets::*` → `crate::*`
- [ ] 2.4 Create `uipc-mapping/src/lib.rs` — declare `mod types; mod mapping;`, `pub use types::*;`, `pub use mapping::*;`, `pub use uipc_expr::Expr;`
- [ ] 2.5 Add `uipc-mapping` to workspace `members` in root `Cargo.toml`
- [ ] 2.6 Verify: `cargo build -p uipc-mapping` and `cargo test -p uipc-mapping` pass

## 3. Update xplane_uipc to use new crates

- [ ] 3.1 Remove `mod expr;` and `mod mapping;` from `xplane_uipc/src/lib.rs`
- [ ] 3.2 Remove `xplane_uipc/src/expr.rs` and `xplane_uipc/src/mapping.rs`
- [ ] 3.3 Add `uipc-mapping = { path = "../uipc-mapping" }` to `xplane_uipc/Cargo.toml` `[dependencies]`
- [ ] 3.4 Update `xplane_uipc/src/fsuipc_offsets.rs` — remove `FsuipcType` and `FSUIPC_DATA_SIZE` definitions; add `use uipc_mapping::{FsuipcType, FSUIPC_DATA_SIZE};`
- [ ] 3.5 Update `xplane_uipc/src/plugin_state.rs` — change `use crate::expr::Expr` → `use uipc_mapping::Expr`; change `use crate::mapping::*` → `use uipc_mapping::*`
- [ ] 3.6 Update `xplane_uipc/src/lib.rs` — change `mapping::load_mappings(...)` to `uipc_mapping::load_mappings(...)`
- [ ] 3.7 Verify: `cargo build -p xplane_uipc` and `cargo test -p xplane_uipc` pass

## 4. Verify and format

- [ ] 4.1 Run `cargo build --workspace` (all crates compile)
- [ ] 4.2 Run `cargo test --workspace` (all tests pass)
- [ ] 4.3 Run `cargo fmt` across the workspace
- [ ] 4.4 Run `cargo xtask dist` (distribution still works)
