## 1. Loader (`uipc-mapping`)

- [x] 1.1 Add `write_exprs: Option<HashMap<String, String>>` to `RawMapping`, and `write_exprs: BTreeMap<String, Expr>` (sorted, empty when absent) to `MappingSource::Expr`
- [x] 1.2 Validate per the spec: needs `expr` and `writable = true`; non-empty; keys must name `datarefs` entries; each parses; variables (`Expr::vars()`) are `value` or `datarefs` names; no `datarefs` entry named `value`. Each failure is a load error naming the offset, and the mapping is skipped
- [x] 1.3 Reject a writable expression mapping without `write_exprs`
- [x] 1.4 Reject a writable single-dataref mapping with `scale = 0`, and keep loading read-only ones
- [x] 1.5 Reject a mapping with both `dataref` and `expr`
- [x] 1.6 Unit tests: a valid fan-out mapping loads; each rejection case above (1.2–1.5) gives a load error naming the offset; a read-only mapping without `write_exprs` loads unchanged; a read-only `scale = 0` mapping loads; `expr = "5"` with `write_exprs` and no `datarefs` is rejected
- [x] 1.7 Confirm the shipped `mappings.toml` loads with no new load errors

## 2. Plugin write path (`xplane_uipc`)

- [x] 2.1 Carry the parsed write expressions into `ResolvedSource::Expr`
- [x] 2.2 In `ResolvedMapping::write_xplane`, for `Expr` with write expressions: read all refs once (unavailable → `0.0`), add `value`, evaluate every write expression, return with a warning if any result isn't finite, otherwise write each target in key order through `ResolvedRef::write`, skipping unavailable targets with a warning
- [x] 2.3 Factor the evaluate step into a pure function (variables in, ordered `(name, value)` results or an error out) so it can be unit tested without X-Plane
- [x] 2.4 Extend the load-time writability check to warn for each write target X-Plane reports as read-only. Remove the "only single-dataref mappings can be written" warning, which is now a load error (1.3)
- [x] 2.5 Unit tests for the pure evaluate step: fan-out, current-value use, snapshot semantics (`A = $B`, `B = $A`), non-finite result blocks everything
- [x] 2.6 Make `ResolvedRef::write` return whether it wrote: `false` for a null handle or a dataref type it can't write
- [x] 2.7 Make `write_xplane` return whether any dataref was written. Add a once-per-load "unavailable dataref" flag to `ResolvedMapping`: the first skip logs a warning naming the offset and the dataref, and later skips log at debug. This covers both single-dataref writes and write-expression targets
- [x] 2.8 In `write_offset`, log "Wrote value…" only when `write_xplane` returns `true`
- [x] 2.9 Drop the `scale.abs() < 1e-12` fallback in `write_xplane` now that the loader rejects writable `scale = 0` (keep a debug assertion)

## 3. Mappings

- [x] 3.1 `0x023B`, `0x023C`, `0x023A`, `0x0238`, `0x0239`: convert to expression mappings that keep reading the `clock_timer` datarefs and write through `sim/time/zulu_time_sec` (see design table); `writable = true`
- [x] 3.2 `0x0D0C`: add `write_exprs` for the nav, beacon, landing, taxi and strobe switches; `writable = true`
- [x] 3.3 `0x0BD0`: add the spoiler write expression; `writable = true`
- [x] 3.4 `0x3102`, `0x281C`: write `$value` to both battery datarefs; `writable = true`. Change `0x3102` to `u8` to match the guide (a 1-byte client write would otherwise be rejected by the size check)
- [x] 3.5 `0x7B91`: add the transponder write expression; `writable = true`
- [x] 3.6 Re-run `mappings reconciliation/check_datarefs.py` and confirm no write target is reported read-only (it will need to learn about `write_exprs` targets)

## 4. Docs

- [x] 4.1 Document `write_exprs` (variables, all-or-nothing, validation) in the `mappings.toml` header comments
- [x] 4.2 Add a "Write expressions" section to `uipc-expr/README-EXPR.md` with the lights and Zulu-hour examples
- [ ] 4.3 Document the new load errors (`writable` expression mapping needs `write_exprs`, no writable `scale = 0`, no `dataref` together with `expr`) in the `mappings.toml` header, and list them as breaking in the release notes

## 5. Verification

- [x] 5.1 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test`, `cargo xtask dist`
- [ ] 5.2 Sim check with FSInterrogate: write `0x023B` (Zulu hour, 1 byte), `0x0D0C` (lights bitfield, 2 bytes), `0x0BD0` (spoilers armed = 4800, 4 bytes) and confirm the effect in X-Plane; confirm no read-only-target warnings in `uipc.log`
- [ ] 5.3 Sim check, unavailable dataref: write to an offset whose dataref belongs to an add-on that isn't loaded, twice. Confirm one warning and no "Wrote value" in `uipc.log`
