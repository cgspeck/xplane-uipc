# Tasks

## 1. Runner

- [x] 1.1 Add `uipc-mapping/tests/compound_mappings.rs` with `serde` types for the case file (`deny_unknown_fields`) and a compound-mapping predicate. Verify with an in-file unit test that the lights mapping is compound and a single-dataref read-only `expr` mapping isn't
- [x] 1.2 Implement read cases: evaluate `update_if_expr` and `expr` with name-keyed state (missing = 0), and compare against the served value (integer round and wrap, float relative 1e-6). Verify with inline-TOML runner tests: a passing case, an `i32` `1023.9999` → `1024` case, an `expect_retain` case and a mismatching case that reports expected and actual
- [x] 1.3 Implement write cases on top of `eval_writes`: exact `expect_datarefs` (every target, 1e-9), `expect_commands` (unlisted means 0 runs) and `expect_blocked`. Verify with inline-TOML runner tests for the missing-target, unexpected-command and blocked scenarios
- [x] 1.4 Implement case validation (unknown or non-compound offset, unknown state name, write case on a non-writable mapping, unknown expectation name or command, wrong mix of expect fields). Verify with an inline-TOML test per scenario in the spec's "Invalid cases" requirement
- [x] 1.5 Implement coverage (read case, write case if writable, each `expr` var non-zero in a read case, each write target non-zero and each command run in a write case), collecting all failures into one assertion. Verify with an inline-TOML test where the lights mapping lacks a `Logo` case and the message names `0x0D0C` and `Logo`
- [x] 1.6 Add the test that runs the shipped `mappings.toml` against `mappings.tests.toml`, starting with an empty case file. Verify it fails and lists every compound offset. That list is the checklist for group 2

## 2. Cases for the shipped mappings

Each task adds read cases (one per part, plus a combined case), write cases that list every target (one per part, from a state where the other parts are on), and any blocked or retained cases. Verify each by running `cargo test -p uipc-mapping --test compound_mappings`, checking that the offsets drop off the coverage list and every case passes. For a case that fails on a real mapping bug, stop and report it rather than encoding the bug.

- [ ] 2.1 Lights `0x0D0C`: each of the seven bits on its own, B738 taxi switch versus `Taxi`, a write for each bit with the others on, and the B738 command runs (strobe/steady/off, taxi on/off, logo/wing toggles only when the state differs)
- [ ] 2.2 Electrical: battery `0x281C` and `0x3102` (Zibo OR generic, and each command's condition) and avionics `0x2E80` (toggle only when the state differs, for each aircraft)
- [ ] 2.3 Time writes `0x0238`–`0x023C`: each write keeps the other fields of `zulu_time_sec`, including wrap past midnight and across days for local hour and minute
- [ ] 2.4 Doors `0x3367`, pushback `0x31F0` and time of day `0x115E`: each bit or branch on its own
- [ ] 2.5 Spoilers `0x0BD0`: the armed, retracted and deployed branches, and both write branches (4800 → armed, 5620..16383 → 0..1, and below 5620 clamps to 0)
- [ ] 2.6 Sums and combinations: altitude `0x3324`, fuel total `0x126C` (each tank contributes), body acceleration `0x3068`, and touchdown vertical speed `0x030C` (value in the air, retained on the ground)
- [ ] 2.7 Any remaining offsets on the coverage list. Verify that `cargo test -p uipc-mapping` passes with no coverage gaps

## 3. Docs and packaging

- [ ] 3.1 Document the case format and coverage rules in a comment header at the top of `mappings.tests.toml`, add a pointer from the `mappings.toml` header ("compound mappings need cases in mappings.tests.toml"), and add a short "Mapping tests" section to the README. Verify the README command runs as written
- [ ] 3.2 Confirm `cargo xtask dist` doesn't copy `mappings.tests.toml` (it copies files by name). Verify by listing `dist/xplane-uipc/` after a dist build

## 4. Integration

- [ ] 4.1 Run `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test` and `cargo xtask dist`. All pass
- [ ] 4.2 Isolation sanity check: temporarily break one part (e.g. change `Bcn = "$value 2 & 0 !="` to `"$value 6 & 0 !="`) and confirm the suite fails naming the affected cases, then revert
