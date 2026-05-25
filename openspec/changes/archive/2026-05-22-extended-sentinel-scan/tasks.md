## 1. Add `view_size` parameter to `iterate_records`

- [x] 1.1 Change signature: `iterate_records(mapped_view_ptr: *const u8, view_size: usize, on_record: F)`
- [x] 1.2 Update `process_mapped_view` to accept and forward `view_size`
- [x] 1.3 Update `capture-inspect.rs` to pass `data.len()`
- [x] 1.4 Update `wnd_proc` in `lib.rs` to pass view size from `VirtualQuery`

## 2. Two-phase sentinel recovery

- [x] 2.1 When Phase 1 (`find_next_record` with 16-byte window) returns `None`, call Phase 2 with remaining view size: `view_size.saturating_sub(sentinel_offset + 1 + 12)`
- [x] 2.2 If Phase 2 also returns `None`, break (end of data — same as current Phase-1-fail behavior)
- [x] 2.3 If Phase 2 returns `Some(n)`, advance and continue the parse loop

## 3. Once-logging of bad sentinel values

- [x] 3.1 Add `LOGGED_SENTINEL_VALUES: LazyLock<Mutex<HashSet<u32>>>` — log hex value of each distinct bad sentinel on first occurrence (per-value, not one guard for all)
- [x] 3.2 On bad sentinel, before Phase 1/2, log the sentinel hex value once at `warn` level: `"Bad sentinel at offset {:#x}: value {:#010x}"`
- [x] 3.3 Use `HashSet::insert` return value for once-guard: lock, insert, log only if newly inserted

## 4. `:FSD` text detection and logging

- [x] 4.1 Add `FSD_TEXT_LOGGED: AtomicBool` — guard for once-logging `:FSD` trailing text
- [x] 4.2 On bad sentinel, if the 4 sentinel bytes equal `0x4453463A` (`:FSD` LE), extract trailing text (up to 255 bytes of printable ASCII)
- [x] 4.3 Log once at `info` level: `"Bad sentinel at offset {:#x}: ':FSD' followed by: \"{}\""`
- [x] 4.4 If no printable text follows (immediate null/control bytes), skip logging

## 5. Tests

- [x] 5.1 `test_bad_sentinel_with_fsd_and_extended_recovery` — orphan records + `:FSD` + zeros + valid frames; asserts recovery and correct value
- [ ] 5.2 `test_unknown_sentinel_logged_once` — skipped (AtomicBool is static across tests in same process; mechanism-verified by 3.3)
- [ ] 5.3 `test_fsd_text_logged_once` — skipped (same static AtomicBool constraint as 5.2)
- [x] 5.4 `test_fsd_no_text_after` — `:FSD` followed by null bytes; no text log, clean error count
- [x] 5.5 Covered by 5.1 (the recovery test exercises the exact orphan+`:FSD`+zeros+frame scenario)
- [x] 5.6 `test_view_size_too_small_safe` — 4-byte buffer; no UB, clean break

## 6. Verification

- [x] 6.1 `cargo fmt`
- [x] 6.2 `cargo test` — 24 passed, 0 failed
- [x] 6.3 `cargo build` — clean compile
- [x] 6.4 `cargo xtask dist` — distribution builds succeed
