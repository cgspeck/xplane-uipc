# Proposal

## Why

Several FSUIPC offsets store their value as binary coded decimal (BCD): the transponder at `0x0354` (squawk 2770 is `0x2770`), COM1/NAV1/NAV2 at `0x034E`/`0x0350`/`0x0352` (123.45 is `0x2345`, with the leading 1 assumed), the standby radios at `0x311A`–`0x3120`, and ADF1 split across `0x034C`/`0x0356`. X-Plane's datarefs hold plain decimal numbers. The mapping language has no way to turn 2770 into `0x2770` or back, so these offsets can't be served correctly. The `0x0354` mapping in `mappings.toml` currently serves the raw code and has a note about it. Clients such as SPAD.neXt also set radios and the squawk by writing these offsets, so the conversion has to work both ways.

## What Changes

- Add two RPN operators to `uipc-expr`: `tobcd` (decimal → BCD, `2770 tobcd` = `0x2770` = 10096) and `frombcd` (BCD → decimal, `9029 frombcd` = 2345). Invalid input gives `NaN`: negative, non-finite or too large, or for `frombcd` a hex digit above 9. Existing rules already turn `NaN` into "no value" on read and into a blocked write.
- BCD offsets are written as expression mappings: `expr` uses `tobcd` and `write_exprs` uses `frombcd`. No new mapping fields are added.
- Fix the transponder mapping and add writable mappings for COM1/COM2, NAV1/NAV2 (active and standby) and ADF1 in the shipped `mappings.toml`.
- Document the operators in `README-EXPR.md`, and the BCD mapping pattern in the `mappings.toml` header.

No breaking changes.

## Capabilities

### New Capabilities

- `bcd-encoding`: the `tobcd`/`frombcd` expression operators and their handling of invalid input.

### Modified Capabilities

(none. The existing expression, write-expression and offset-value rules cover how these operators' results are served and written.)

## Impact

- `uipc-expr`: two new operators. No changes are needed in `uipc-mapping`, `xplane_uipc` or `uipc-debug`, because they evaluate expressions through `uipc-expr`.
- `xplane_uipc/mappings.toml`, `uipc-expr/README-EXPR.md`, and `docs/OFFSETS.md` if it lists the radio offsets.
