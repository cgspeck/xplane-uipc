## Context

The fsuipc-test-client is a C# (.NET 10, Windows-only) Spectre.Console TUI that connects to FSUIPC and displays live offset values in a table. The current architecture has a single `AnsiConsole.Live()` block that runs until quit. Offsets are loaded from a text file at startup and can be reloaded with R, but cannot be modified interactively.

Spectre.Console's `TextPrompt` and `SelectionPrompt` cannot run inside a `Live()` context — they conflict with the live rendering. The prompts must run outside Live.

## Goals / Non-Goals

**Goals:**
- Add, edit, and delete watched offsets without leaving the TUI
- Write modified offset list back to the source file
- Visual indicator when in-memory state differs from file
- Discoverable help key hint in the main UI
- Cancel support at every prompt step

**Non-Goals:**
- Inline editing within the Live table (too complex for the benefit)
- Undo/redo
- Multi-select operations
- Auto-save

## Decisions

### D1: Exit/re-enter Live for prompts

| Option | Verdict |
|---|---|
| Custom form rendered inside Live context | Requires reimplementing text input and selection widgets from scratch |
| Inline single-field editing at bottom of Live | Still needs custom key-by-key input handling |
| **Exit Live, run Spectre prompts, re-enter Live** | **Adopted** — uses native SelectionPrompt/TextPrompt, minimal code, proven UX |

The main loop becomes a `while` loop that runs Live until an action exits it, handles the action (prompts, delete, write, etc.), then re-enters Live.

### D2: Cancel mechanism

| Option | Verdict |
|---|---|
| CancellationToken + background Escape listener | Fragile, can leave console in bad state |
| **Magic value for TextPrompt, Cancel choice for SelectionPrompt** | **Adopted** — simple, self-documenting, no hacks |

- TextPrompt for address: typing `c` cancels
- SelectionPrompt for type: `Cancel` as first choice
- TextPrompt for size: entering `0` cancels

### D3: Address range

Addresses are 0x00000 to 0xFFFFF (20-bit). Display format: 4 hex digits for values <= 0xFFFF, 5 hex digits for values > 0xFFFF. The text file format and prompts both use `0x` prefix.

### D4: Dirty state tracking

A `bool isDirty` flag, set to `true` on add/edit/delete, cleared on reload (R) or write (W). Displayed as `[modified]` in the table caption when true.

### D5: Dirty quit guard

When Q is pressed and `isDirty` is true, exit Live and show `ConfirmationPrompt("Unsaved changes to mappings. Quit anyway?")`. If declined, re-enter Live.

### D6: Write file format

W key serializes the current `defs` list back to the original text file format:
```
0x02BC,i32
0x3160,string,24
```
One line per offset. Fixed-size types omit the size column. Variable-size types include it.

### D7: Main loop restructure

```
Current:
  await AnsiConsole.Live(...).StartAsync(async ctx => {
      while (true) { ... handle keys inside ... }
  });

New:
  while (true) {
      action = await RunLive(defs, client, state);
      switch (action) {
          Quit     → if dirty: confirm prompt; if confirmed or clean: return
          Add      → run add prompts; if not cancelled: add to defs, re-register, dirty=true
          Edit     → run edit prompts; if not cancelled: replace in defs, re-register, dirty=true
          Delete   → remove from defs, clamp selectedIndex, re-register, dirty=true
          Reload   → parse file, replace defs, re-register, dirty=false
          Save     → snapshot to JSON (existing)
          WriteFile→ serialize defs to file, dirty=false
          None     → (shouldn't happen, but continue)
      }
  }
```

`RunLive` returns the action that caused it to exit. All mutable state (defs, selectedIndex, isDirty, lastError) lives outside the Live block and is passed in.

### D8: Re-registration after mutation

After add/edit/delete, call `client.ClearOffsets()` then `client.RegisterOffsets(defs)` — the same path used by reload today. This is simple and correct; the FSUIPC library handles disconnect/reconnect of individual offsets.

### D9: Help hint placement

Add `h for help` to the right side of the caption bar:
```
File: sample-offsets.txt [modified]  |  4 offsets  |  CONNECTED  |  h for help
```

## Risks / Trade-offs

| Risk | Mitigation |
|---|---|
| Brief screen flicker when exiting/re-entering Live | Acceptable for an interactive tool; the transition is fast |
| Monitoring pauses during prompts | Non-issue — user is actively editing, not watching values |
| No Escape key in Spectre prompts | Cancel choice/magic value provides equivalent functionality at every step |
| Overwriting file with W | User must explicitly press W; dirty flag makes state visible |
