## 1. Add IMod operator variant

- [x] 1.1 Add `IMod` to the `Op` enum in `uipc-expr/src/lib.rs`
- [x] 1.2 Add `"imod"` token parsing in the `parse` match arm
- [x] 1.3 Add `Op::IMod` display string (`"imod"`) in the `Display`/token_str implementation

## 2. Change operator semantics

- [x] 2.1 Change `Op::Mod` evaluation from `((a as i64) % (b as i64)) as f64` to `a % b` with the existing div-by-zero guard
- [x] 2.2 Add `Op::IMod` evaluation using the old integer logic: `((a as i64) % (b as i64)) as f64` with div-by-zero guard

## 3. Tests

- [x] 3.1 Update existing `test_mod` to verify floating-point remainder (e.g., `304.75 1 %` = `0.75`)
- [x] 3.2 Add `test_mod_negative` for negative fractional modulo (`-5.3 1 %` = `-0.3`)
- [x] 3.3 Add `test_imod` for integer modulo (`304.75 3 imod` = `1.0`, `10 3 imod` = `1.0`)
- [x] 3.4 Add `test_imod_by_zero` (`5 0 imod` = `0.0`)
- [x] 3.5 Add `test_token_strs_imod` to verify display/parse round-trip for `imod`
- [x] 3.6 Run `make` to verify all tests pass and clippy is clean
