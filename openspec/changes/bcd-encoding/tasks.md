# Tasks

## 1. BCD operators (`uipc-expr`)

- [ ] 1.1 Add `pub fn to_bcd(f64) -> f64` and `pub fn from_bcd(f64) -> f64` (round first, `NaN` for negative, non-finite or out-of-range input or a nibble above 9). Verify with unit tests: `2770 → 10096`, `2344.5 → 9029`, `0 → 0`, `-1`/14 digits → `NaN`, `9029 → 2345`, `0x12AB → NaN`, and a round trip over 0..=9999
- [ ] 1.2 Add `tobcd` and `frombcd` unary operators (parse, eval, `Display`) using those functions. Verify with `cargo test -p uipc-expr` covering each operator scenario in the spec, including `7700 tobcd frombcd` and `Expr::parse("2770 tobcd").to_string()` round-tripping
- [ ] 1.3 Add a `uipc-mapping` `eval_writes` test for the COM pattern: `$value frombcd 10000 +` with `value = 0x2250` gives `12250`, and `value = 0x12AB` gives an error. Verify with `cargo test -p uipc-mapping`
- [ ] 1.4 Document both operators in `uipc-expr/README-EXPR.md` with the transponder and COM examples, and check them in the `expr-calculator` REPL

## 2. Shipped mappings and docs

- [ ] 2.1 Document the BCD read/write pattern in the `mappings.toml` header (Expression and Writes sections), including the COM/NAV `10000 -` / `10000 +` pair
- [ ] 2.2 Rewrite `0x0354` (transponder) as a writable expression mapping (`$T tobcd` / `$value frombcd`) and correct its comment. Add writable mappings for COM1 `0x034E`, COM2 `0x3118`, NAV1 `0x0350`, NAV2 `0x0352` and standbys `0x311A`/`0x311C`/`0x311E`/`0x3120` using the `*_frequency_hz` datarefs. Verify that `uipc-debug` offline mode loads the file with no load errors and gives `0x2770` for transponder `2770` and `0x2345` for COM `12345`
- [ ] 2.3 Check in the sim (DataRefTool) what `adf1_frequency_hz` holds for a known NDB frequency (e.g. 362 kHz), and adjust the ADF expressions in 2.4 if it isn't plain kHz
- [ ] 2.4 Add ADF1 `0x034C` (expr `$Adf 1000 % tobcd`, write `$value frombcd $Adf 1000 \ 1000 * +`) and `0x0356` (expr `$Adf 1000 \ 256 *`, write `$value 256 \ 1000 * $Adf 1000 % +`). Verify in `uipc-debug` offline mode that `Adf = 1234` gives `0x0234` / `0x0100`. Add ADF2 `0x02D4` / `0x02D6` the same way using `adf2_frequency_hz`
- [ ] 2.5 Add the new offsets to `docs/OFFSETS.md` if it lists supported offsets

## 3. Integration

- [ ] 3.1 Run `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test` and `cargo xtask dist`. All pass
- [ ] 3.2 In X-Plane with a FSUIPC client (e.g. `fsuipc-test-client` or SPAD.neXt): read `0x0354`/`0x034E` and see BCD values, then write `0x7700` to `0x0354` and `0x2250` to `0x034E` and see the cockpit show 7700 and 122.50
