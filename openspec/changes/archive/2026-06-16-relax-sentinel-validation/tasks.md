## 1. Remove sentinel gating and HashSet logging

- [x] 1.1 In `iterate_records` in `ipc_host/src/mapped_view.rs`: change the
       bad-sentinel branch so it sets `sentinel_ok = false` and falls through
       to normal processing (with `payload_ptr` pointing to offset+16) instead
       of entering recovery mode.
- [x] 1.2 Remove the `LOGGED_SENTINEL_VALUES` HashSet, the `FSD_LOGGED_TEXTS`
       HashSet, and the `LazyLock`/`Mutex` wrappers.
- [x] 1.3 Remove `reset_logged_sentinels()` function (callers too).
- [x] 1.4 Downgrade the per-record `tracing::debug!` at line 173
       ("Bad sentinel: reqID=...") to `tracing::trace!`.
- [x] 1.5 The recovery-scanning code (Phase 1 / Phase 2) remains for the
       zero-reqID terminator case but is no longer reached from the
       bad-sentinel path. Verify it still compiles.

## 2. Update `capture-inspect` output

- [x] 2.1 In `ipc_host/examples/capture-inspect.rs`: widen the output to
       handle `sentinel_ok = false` records — show them as valid records
       with a `?` marker instead of `"BAD SENTINEL"`.
- [x] 2.2 Remove `any_corrupted` tracking that only fired on sentinel
       errors; update to still set exit code based on true structural
       errors from `iterate_records`.

## 3. Fixture-based integration tests

- [x] 3.1 Copy capture files to `ipc_host/tests/fixtures/` with stable names:
       - `captures/WORKING-SLC-2026-06-16T09-59-20.585Z.bin` →
         `ipc_host/tests/fixtures/slc-3-reads.bin`
       - `captures/BROKEN-FSINTERROGATE-2026-06-15T23-24-01.573Z.bin` →
         `ipc_host/tests/fixtures/fsinterrogate-2-reads.bin`
- [x] 3.2 Create `ipc_host/tests/capture_records.rs`:
       - `test_working_slc_capture`: embed SLC fixture, verify 3 records,
         0 errors, all `sentinel_ok: true`, correct offsets (0x3304, 0x3308,
         0x3124)
       - `test_fsinterrogate_capture`: embed FSInterrogate fixture, verify
         2 records, 0 errors, all `sentinel_ok: false`, correct offsets
         (0x3304, 0x3308)

## 4. Verify

- [x] 4.1 Run `cargo test -p ipc_host` to confirm all tests pass
- [x] 4.2 Run `cargo build` to confirm compilation
- [x] 4.3 Run `cargo fmt` to format code
