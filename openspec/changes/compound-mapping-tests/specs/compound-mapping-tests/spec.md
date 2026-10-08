# Spec Delta

## Purpose

Tests the shipped compound expression mappings against a file of read and write cases, and fails the build when a compound mapping or one of its parts has no case, so a change to one part of an expression can't silently break another.

## ADDED Requirements

### Requirement: Compound mappings

A mapping SHALL be compound when it is an expression mapping with two or more `datarefs` entries, or with `write_exprs` or `write_commands`. Single-dataref expression mappings without writes, simple mappings and static mappings SHALL NOT be compound.

#### Scenario: Lights bitmask

- **WHEN** a mapping has `datarefs` with several entries and an `expr`
- **THEN** it is compound

#### Scenario: Single dataref with writes

- **WHEN** a mapping has one `datarefs` entry and `write_exprs` (e.g. the Zulu hour `0x023B`)
- **THEN** it is compound

#### Scenario: Single-dataref scaling

- **WHEN** a mapping has one `datarefs` entry, an `expr` such as `"$elev floor"`, and no writes
- **THEN** it is not compound and needs no cases

### Requirement: Test-case file

`xplane_uipc/mappings.tests.toml` SHALL hold `[[case]]` entries. Each case SHALL have an `offset`, a `name`, and a `state` table mapping the mapping's `datarefs` names to values. Names left out of `state` SHALL be `0`, as an unavailable dataref is. A case with `write` SHALL be a write case, and any other case SHALL be a read case. The file SHALL NOT be copied into `dist`.

#### Scenario: Read case

- **GIVEN** a case with `offset = 0x0D0C`, `state = { Bcn = 1 }` and `expect = 2`
- **THEN** it is a read case for the lights offset with every other light off

#### Scenario: Write case

- **GIVEN** a case with `offset = 0x0D0C`, `write = 2` and `expect_datarefs = { Nav = 0, Bcn = 1, Land = 0, Taxi = 0, Strb = 0 }`
- **THEN** it is a write case

### Requirement: Read cases

A read case SHALL evaluate the mapping's `expr` with `state`. Its `expect` SHALL be compared with the value the offset would serve: rounded and wrapped to the type for integer types, and equal within `1e-6` relative for float types. A case with `expect_retain = true` SHALL instead pass only when `update_if_expr` evaluates to `<= 0`, and a case with `expect` SHALL fail when `update_if_expr` would retain the value.

#### Scenario: Matching value

- **GIVEN** the lights mapping and `state = { Nav = 1, Strb = 1 }`
- **WHEN** the case expects `17`
- **THEN** the case passes

#### Scenario: Integer rounding

- **GIVEN** an `i32` mapping whose `expr` gives `1023.9999`
- **WHEN** the case expects `1024`
- **THEN** the case passes

#### Scenario: Retained value

- **GIVEN** the touchdown vertical speed mapping `0x030C` and `state = { OnGround = 1 }`
- **WHEN** the case has `expect_retain = true`
- **THEN** the case passes

### Requirement: Write cases

A write case SHALL evaluate the mapping's `write_exprs` and `write_commands` with `state` and `$value` set to `write`, as the plugin does. `expect_datarefs` SHALL list every `write_exprs` target, each equal within `1e-9`. `expect_commands` SHALL give the run count of each command that runs, and any command left out SHALL run 0 times. A case with `expect_blocked = true` SHALL pass only when the write is blocked by a non-finite result.

#### Scenario: Writing one light leaves the others alone

- **GIVEN** the lights mapping and `state = { Nav = 1, Land = 1 }`
- **WHEN** a case writes `5` and expects `{ Nav = 1, Bcn = 0, Land = 1, Taxi = 0, Strb = 0 }`
- **THEN** the case passes
- **AND** if an edit to the beacon expression also changed `Land`, the case fails naming `Land`

#### Scenario: Missing target in expectations

- **WHEN** a write case's `expect_datarefs` leaves out a `write_exprs` target
- **THEN** the case fails naming the missing target

#### Scenario: Unexpected command run

- **WHEN** a command runs but isn't in `expect_commands`
- **THEN** the case fails naming the command and its run count

### Requirement: Coverage

The test SHALL fail, listing each gap by offset, when a compound mapping in the shipped `mappings.toml`:

- has no read case;
- is writable and has no write case;
- has a `datarefs` name used by its `expr` or `update_if_expr` that is not non-zero in any of its read cases;
- has a `write_exprs` target not expected non-zero in any of its write cases;
- has a `write_commands` command not expected to run in any of its write cases.

#### Scenario: New compound mapping without cases

- **WHEN** a compound mapping is added to `mappings.toml` with no cases
- **THEN** `cargo test` fails naming its offset

#### Scenario: Untested part

- **GIVEN** the lights mapping has read cases, but none sets `Logo` non-zero
- **THEN** `cargo test` fails naming `0x0D0C` and `Logo`

### Requirement: Invalid cases

The test SHALL fail when a case names an offset with no mapping or with a non-compound mapping, uses a `state` name that isn't in the mapping's `datarefs`, gives a write case for a mapping that isn't writable, or has an expectation name or command that the mapping doesn't have.

#### Scenario: Typo in a state name

- **WHEN** a lights case has `state = { Bcon = 1 }`
- **THEN** the test fails naming the case and `Bcon`

#### Scenario: Stale offset

- **WHEN** a case names an offset that has been removed from `mappings.toml`
- **THEN** the test fails naming the case

### Requirement: Failures report every case

The test SHALL evaluate every case and coverage rule before failing, and its failure message SHALL list every failing case by offset and name with the expected and actual values.

#### Scenario: Two broken cases

- **WHEN** two cases fail
- **THEN** one test run reports both
