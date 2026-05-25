## 1. Update MappingConfig to track load errors

- [x] 1.1 Add `load_errors: Vec<String>` field to `MappingConfig` struct in `src/mapping.rs`
- [x] 1.2 Update `MappingConfig` construction to initialize `load_errors` as empty vec

## 2. Refactor load_mappings for per-mapping error handling

- [x] 2.1 Replace early `return Err(...)` in the mapping loop with error collection: push formatted error string to a `load_errors` vec and `continue`
- [x] 2.2 Handle offset bounds validation error: collect error and continue instead of returning `Err`
- [x] 2.3 Handle expression parse error: collect error and continue instead of returning `Err`
- [x] 2.4 Handle missing source fields error: collect error and continue instead of returning `Err`
- [x] 2.5 After the loop, if `mappings.is_empty()` and `load_errors` is non-empty, return `Err` with summary of all errors
- [x] 2.6 Construct `MappingConfig` with both `mappings` and `load_errors` and return `Ok`

## 3. Update caller to log partial-success summary

- [x] 3.1 In `load_mappings_and_init()` in `src/lib.rs`, after successful `load_mappings()`, check if `mapping_config.load_errors` is non-empty
- [x] 3.2 If errors exist, log summary with `tracing::error!("Loaded {} mappings with {} errors", success_count, error_count)`
- [x] 3.3 Log each individual error from `load_errors` using `tracing::error!`
- [x] 3.4 Update the existing "Loaded N mappings" info log to reflect the actual count of successfully loaded mappings

## 4. Verify and test

- [x] 4.1 Run `cargo check` to verify compilation
- [x] 4.2 Test with a mappings.toml that has one bad mapping among valid ones — verify valid mappings load and error is logged
- [x] 4.3 Test with a mappings.toml where all mappings are bad — verify error is returned
- [x] 4.4 Test with a valid mappings.toml — verify no errors and all mappings load
