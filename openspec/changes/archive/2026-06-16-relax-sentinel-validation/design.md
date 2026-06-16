## Context

Both captures share the same wire format:

    [reqID:4][dwOffset:4][nBytes+flags:4][4th_field:4][payload:nBytes]

At offset +12, different clients write different values:
- **SLC**: `0x5061756C` ("luaP") — the well-known FSUIPC sentinel
- **FSInterrogate**: `0x0105FFF8` / `0x0105FFFC` — per-record writeback
  pointers (client-side bookkeeping, not meaningful to the server)

In both cases, the payload area at offset+16 is where the server writes
read responses, and where the client reads them from afterwards.

Currently the code rejects the second case entirely because
`4th_field != SENTINEL` gates record processing. The fix is to decouple
the sentinel **validation** (diagnostic) from the sentinel **gating**
(processing gate).

## Current flow

```
                 ┌──────────────────────┐
                 │  Parse header        │
                 │  (reqID, offset,     │
                 │   nBytes, 4th)       │
                 └──────────┬───────────┘
                            │
                     ┌──────┴──────┐
                     │  reqID == 0 │────→ find_next_record(...) scan/recovery
                     └──────┬──────┘
                            │ != 0
                            ▼
                 ┌──────────────────────┐
                 │  4th == "luaP"?      │
                 └──────────┬───────────┘
                      YES   │   NO
                      ┌─────┴─────┐
                      ▼           ▼
                process      LOGGED_SENTINEL_VALUES
                normally      .insert() ─→ warn! (once per value)
                              ↓
                      enter recovery scan
                      (skip or break)
```

## After change

```
                 ┌──────────────────────┐
                 │  Parse header        │
                 │  (reqID, offset,     │
                 │   nBytes, 4th)       │
                 └──────────┬───────────┘
                            │
                     ┌──────┴──────┐
                     │  reqID == 0 │────→ find_next_record(...) scan/recovery
                     └──────┬──────┘       (unchanged — still needed for
                            │ != 0         terminator detection)
                            ▼
                 ┌──────────────────────┐
                 │  4th == "luaP"?      │───→ sentinel_ok = true
                 │      or else?        │───→ sentinel_ok = false, +trace!
                 └──────────┬───────────┘    (no set, no warn)
                            │
                            ▼
                 ┌──────────────────────┐
                 │  Process record      │  ◄── NEW: no longer skipped
                 │  payload_ptr→offset+16│
                 └──────────┬───────────┘
                            │
                            ▼
                 ┌──────────────────────┐
                 │  Advance cur_ptr     │
                 │  by 16 + nBytes      │
                 └──────────┬───────────┘
                            │
                   (loop back to top)
```

## Goals / Non-Goals

**Goals:**
- Accept records with any non-zero value in the 4th field
- Write read responses to the inline payload area (same location every time)
- Remove `LOGGED_SENTINEL_VALUES` HashSet (memory leak risk)
- Downgrade record-level logging of non-"luaP" to `trace!`
- Maintain error counting for truly bad records (zero reqID, buffer overrun)
- Add fixture-based integration tests for both capture files

**Non-Goals:**
- Writing responses to an external pointer address (we assume the client
  reads from the inline payload area — standard FSUIPC convention)
- Setting the 4th field to "luaP" after processing (completion-signal
  not needed by current clients)
- Changing the recovery-scanning logic for zero-reqID terminators

## Decisions

### 1. Inline payload writeback (not pointer-based)

The FSInterrogate capture shows per-record pointer values that look like
heap addresses. These are client-side bookkeeping — FSInterrogate uses them
to know where to copy the result *from* shared memory after the server
writes the response inline. We always write to offset+16, which is where
all standard FSUIPC clients read from.

### 2. Log unexpected sentinels at `trace!` only, remove HashSet

The existing `LOGGED_SENTINEL_VALUES` HashSet stores every unique
non-"luaP" 4th-field value. But FSInterrogate writes a unique writeback
pointer per record, so the set grows without bound (memory leak).

**Remove** the HashSet entirely. The per-record `tracing::debug!` (line 173)
already logs each non-"luaP" record with full context — downgrade it to
`tracing::trace!` so it's visible during protocol debugging but silent at
default INFO level.

Also remove `FSD_LOGGED_TEXTS` and `reset_logged_sentinels()` — the FSD
text set is bounded in theory, but removing it keeps the code consistent
and eliminates the remaining leak risk. The ":FSD" detection logic is
preserved; only the "log once" set is dropped.

### 3. Preserve `sentinel_ok` as a flag

`ParsedRecord.sentinel_ok` stays `true` only for "luaP", `false` otherwise.
Callers like `capture-inspect` can still distinguish between SLC and other
clients. But `process_mapped_view` writes the response regardless.

### 4. Recovery scan unchanged for zero-reqID

The `find_next_record` / Phase 1 / Phase 2 recovery logic remains intact
for the zero-reqID case (terminator detection). It is no longer entered
for non-"luaP" 4th fields.

## Risks / Trade-offs

- **[False acceptance of corrupt data]** → Low. The reqID, dwOffset,
  and nBytes fields are still parsed as before. If the client wrote a
  garbled record, the reqID (checked for non-zero) and nBytes range
  provide basic validation.
- **[Sentinel no longer marks record boundary]** → Not a regression.
  The record boundary was always `16 + nBytes`, not sentinel-based.
  The sentinel was purely a validity marker.
- **[Backward compat for existing SLC records]** → None. All-zero payload
  with "luaP" sentinel behaves identically.
- **[Lost visibility of new clients]** → Low. The per-record `trace!` log
  still captures every non-"luaP" 4th field. Enable trace-level logging
  to see them. The capture-on-error mechanism also preserves raw buffers
  for offline analysis.
