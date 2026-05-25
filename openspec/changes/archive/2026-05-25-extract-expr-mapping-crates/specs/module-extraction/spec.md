## ADDED Requirements

*(This is a structural refactor — no user-facing capabilities are introduced. Requirements below capture the extraction contract.)*

### Requirement: uipc-expr crate

The system SHALL provide a `uipc-expr` workspace crate that exports `Expr` with `parse()`, `eval()`, and `vars()` methods, identical in behavior to the current `xplane_uipc::expr::Expr`.

#### Scenario: uipc-expr builds standalone

- **WHEN** running `cargo build -p uipc-expr`
- **THEN** it succeeds without the Windows SDK or X-Plane SDK

#### Scenario: uipc-expr tests pass standalone

- **WHEN** running `cargo test -p uipc-expr`
- **THEN** all tests from the original `expr.rs` module pass

### Requirement: uipc-mapping crate

The system SHALL provide a `uipc-mapping` workspace crate that exports `FsuipcType`, `FSUIPC_DATA_SIZE`, `MappingSource`, `DatarefMapping`, `MappingConfig`, `load_mappings()`, and `parse_dataref_with_index()`, identical in behavior to the current `xplane_uipc::mapping` and `xplane_uipc::fsuipc_offsets::FsuipcType`.

#### Scenario: uipc-mapping builds standalone

- **WHEN** running `cargo build -p uipc-mapping`
- **THEN** it succeeds without the Windows SDK or X-Plane SDK

#### Scenario: uipc-mapping tests pass standalone

- **WHEN** running `cargo test -p uipc-mapping`
- **THEN** all tests from the original `mapping.rs` module pass

#### Scenario: uipc-mapping re-exports Expr

- **WHEN** a downstream crate depends only on `uipc-mapping`
- **THEN** it can use `uipc_mapping::Expr` directly

### Requirement: xplane_uipc unchanged behavior

The `xplane_uipc` crate SHALL continue to build and produce an X-Plane plugin with identical runtime behavior.

#### Scenario: workspace builds

- **WHEN** running `cargo build --workspace`
- **THEN** all crates in the workspace compile without errors

#### Scenario: all workspace tests pass

- **WHEN** running `cargo test --workspace`
- **THEN** all existing tests pass
