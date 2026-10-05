## 1. Loader (`uipc-mapping`)

- [ ] 1.1 Add `write_exprs: Option<HashMap<String, String>>` to `RawMapping`, and `write_exprs: BTreeMap<String, Expr>` (sorted, empty when absent) to `MappingSource::Expr`
- [ ] 1.2 Validate per the spec: needs `expr` and `writable = true`; non-empty; keys must name `datarefs` entries; each parses; variables (`Expr::vars()`) are `value` or `datarefs` names; no `datarefs` entry named `value`. Each failure is a load error naming the offset, and the mapping is skipped
- [ ] 1.3 Unit tests: valid fan-out mapping loads; each rejection case above gives a load error; a mapping without `write_exprs` loads unchanged

## 2. Plugin write path (`xplane_uipc`)

- [ ] 2.1 Carry the parsed write expressions into `ResolvedSource::Expr`
- [ ] 2.2 In `ResolvedMapping::write_xplane`, for `Expr` with write expressions: read all refs once (unavailable → `0.0`), add `value`, evaluate every write expression, return with a warning if any result isn't finite, otherwise write each target in key order through `ResolvedRef::write`, skipping unavailable targets with a warning
- [ ] 2.3 Factor the evaluate step into a pure function (variables in, ordered `(name, value)` results or an error out) so it can be unit tested without X-Plane
- [ ] 2.4 Extend the load-time writability check: warn for each write target X-Plane reports as read-only; keep the "writes will have no effect" warning only for writable expression mappings without `write_exprs`
- [ ] 2.5 Unit tests for the pure evaluate step: fan-out, current-value use, snapshot semantics (`A = $B`, `B = $A`), non-finite result blocks everything

## 3. Mappings

- [ ] 3.1 `0x023B`, `0x023C`, `0x023A`, `0x0238`, `0x0239`: convert to expression mappings that keep reading the `clock_timer` datarefs and write through `sim/time/zulu_time_sec` (see design table); `writable = true`
- [ ] 3.2 `0x0D0C`: add `write_exprs` for the nav, beacon, landing, taxi and strobe switches; `writable = true`
- [ ] 3.3 `0x0BD0`: add the spoiler write expression; `writable = true`
- [ ] 3.4 `0x3102`, `0x281C`: write `$value` to both battery datarefs; `writable = true`. Change `0x3102` to `u8` to match the guide (a 1-byte client write would otherwise be rejected by the size check)
- [ ] 3.5 `0x7B91`: add the transponder write expression; `writable = true`
- [ ] 3.6 Re-run `mappings reconciliation/check_datarefs.py` and confirm no write target is reported read-only (it will need to learn about `write_exprs` targets)

## 4. Docs

- [ ] 4.1 Document `write_exprs` (variables, all-or-nothing, validation) in the `mappings.toml` header comments
- [ ] 4.2 Add a "Write expressions" section to `uipc-expr/README-EXPR.md` with the lights and Zulu-hour examples

## 5. Verification

- [ ] 5.1 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test`, `cargo xtask dist`
- [ ] 5.2 Sim check with FSInterrogate: write `0x023B` (Zulu hour, 1 byte), `0x0D0C` (lights bitfield, 2 bytes), `0x0BD0` (spoilers armed = 4800, 4 bytes) and confirm the effect in X-Plane; confirm no read-only-target warnings in `uipc.log`
