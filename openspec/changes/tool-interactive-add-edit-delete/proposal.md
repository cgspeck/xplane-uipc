## Why

The fsuipc-test-client TUI currently requires editing a text file to change which offsets are watched. This breaks the workflow — you have to quit, edit the file, restart (or press R to reload). Interactive add/edit/delete lets you iterate on offset lists without leaving the tool.

## What Changes

- **Add offset (A key)**: Exits Live context, runs Spectre.Console prompts (address as validated hex, type via SelectionPrompt, size if variable-length), re-enters Live with the new offset registered
- **Edit offset (E key)**: Same prompt flow as Add, pre-filled with the selected offset's current values
- **Delete offset (D key)**: Immediately removes the selected offset, no confirmation
- **Write to file (W key)**: Serializes the current offset list back to the text file format, clearing the dirty flag
- **Dirty flag**: `[modified]` indicator in the caption bar when in-memory state differs from file
- **Dirty quit guard**: When quitting with unsaved changes, exits Live and shows a confirmation prompt
- **Help hint**: `h for help` shown in the caption/footer area so discoverability doesn't depend on guessing
- **Help panel updated**: New keybindings (A, E, D, W) documented

## Prompt UX

All prompts use Spectre.Console's native prompt widgets outside the Live context (Option A pattern — exit Live, prompt, re-enter Live):

- **Address**: `TextPrompt<string>` with hex validation (0x00000-0xFFFFF range). Typing `c` cancels.
- **Type**: `SelectionPrompt<string>` with `Cancel` as first choice, followed by all 12 offset types (u8, i8, u16, i16, u32, i32, f32, u64, i64, f64, string, bytes)
- **Size**: `TextPrompt<int>` shown only for string/bytes types. Entering 0 cancels.
- **Edit**: Same prompts with `.DefaultValue()` pre-filled from the selected offset

Cancel at any prompt step aborts the entire add/edit flow and returns to Live with no changes.

## Capabilities

### New Capabilities
*(none — this is a developer tool enhancement, not a spec-level capability)*

### Modified Capabilities
*(none)*

## Impact

- **Modified files**:
  - `fsuipc-test-client/TuiMode.cs` — main loop restructured to exit/re-enter Live; new KeyAction variants; dirty state tracking; prompt flows; updated caption and help panel
  - `fsuipc-test-client/Models.cs` — possible addition of serialization helper for writing offset definitions back to text format
  - `fsuipc-test-client/OffsetParser.cs` — possible addition of a `Serialize` method to write definitions back to the text file format
- **No new dependencies** — all prompt types already available in Spectre.Console v0.49.1
- **No breaking changes** — existing keybindings and file format unchanged
