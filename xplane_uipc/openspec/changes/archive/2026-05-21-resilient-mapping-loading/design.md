## Context

The mapping loading system in `mapping.rs` currently uses a fail-fast approach: `load_mappings()` reads the entire TOML file, parses it, then iterates over each raw mapping. If any single mapping fails validation (offset bounds, expression parse, missing fields, invalid type), the function returns an `Err` immediately and no mappings are loaded. This is called from `load_mappings_and_init()` in `lib.rs` during plugin enable and from the reload menu handler.

The only soft failures currently are at runtime: datarefs that don't resolve produce a `tracing::warn!` but the mapping is still created with a null handle.

## Goals / Non-Goals

**Goals:**
- Individual mapping failures during load should not prevent other valid mappings from loading
- Each failed mapping is logged with `tracing::error!` including offset and error details
- Unrecoverable errors (file not found, TOML parse failure, zero mappings loaded) still return `Err`
- Callers receive information about how many mappings succeeded and failed
- Minimal changes to existing data structures and public interfaces

**Non-Goals:**
- No changes to the mapping file format (TOML schema stays the same)
- No changes to runtime behavior (dataref resolution, expression evaluation)
- No interactive error recovery or auto-fix suggestions
- No changes to the reload flow beyond handling the new result type

## Decisions

### 1. Return type: `Result<MappingConfig, String>` with error summary in `MappingConfig`

**Decision**: Keep the existing `Result<MappingConfig, String>` signature but add optional error tracking to `MappingConfig`. Add a `load_errors: Vec<String>` field to `MappingConfig` that collects per-mapping errors. The function returns `Ok(config)` if at least one mapping loaded (even if some failed), and `Err(...)` only for unrecoverable errors or when zero mappings loaded.

**Rationale**: This avoids changing the caller's error handling path significantly. Callers already check `Result`. They can additionally inspect `config.load_errors` to log a summary. An alternative would be a new `LoadResult` enum, but that requires more changes across call sites.

**Alternatives considered:**
- New `enum LoadResult { Success { config, errors }, Partial { config, errors }, Failure(String) }` — more expressive but requires updating all callers.
- Return `(MappingConfig, Vec<String>)` — loses the clear success/failure boundary for unrecoverable errors.

### 2. Error collection: continue on each mapping error, collect all errors

**Decision**: In the mapping iteration loop, wrap each mapping's processing in a closure that catches errors. On error, push a formatted error string to a `Vec<String>` and continue. After the loop, if `mappings.is_empty()` and errors exist, return `Err` with a summary. Otherwise return `Ok(config)` with the collected errors.

**Rationale**: This ensures all mappings are attempted, giving the user a complete picture of what's wrong in a single load attempt.

### 3. Error format: include offset and concise error message

**Decision**: Each error string follows the format: `"mapping at offset 0xXXXX: <error details>"`. This matches the existing error messages but makes them collectible rather than terminal.

**Rationale**: Consistent with existing error messages, provides enough context to locate the problematic mapping.

### 4. Caller-side logging: summary + individual errors

**Decision**: In `load_mappings_and_init()`, after a successful load, check if `config.load_errors` is non-empty. If so, log a summary with `tracing::error!` (e.g., `"Loaded N mappings with M errors"`) followed by each individual error. This ensures errors appear in the X-Plane log even on partial success.

**Rationale**: Users need to know when mappings failed to load. Using `tracing::error!` for the summary ensures visibility. Individual errors are also logged at error level for easy scanning.

## Risks / Trade-offs

- **[Risk]** Silent degradation: users may not notice that some mappings failed to load if they don't check logs. → **Mitigation**: Log summary at error level; consider future X-Plane UI notification.
- **[Risk]** Accumulation of many errors could produce a very long log. → **Mitigation**: Each error is one line; even with 100 bad mappings, the log is manageable. Could cap at a reasonable number in the future.
- **[Trade-off]** Keeping `Result<MappingConfig, String>` means the error type is still a string, not structured. → **Acceptable** for now; the `load_errors` field on `MappingConfig` provides structured access to per-mapping errors.
