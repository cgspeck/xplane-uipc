## 1. Restructure main loop

- [x] 1.1 Extract Live rendering into a `RunLive` method that returns a `KeyAction` when an action exits the loop
- [x] 1.2 Move mutable state (defs list, selectedIndex, isDirty, lastError) outside the Live block
- [x] 1.3 Wrap `RunLive` in an outer `while` loop that dispatches on the returned action
- [x] 1.4 Add new `KeyAction` variants: `Add`, `Edit`, `Delete`, `WriteFile`

## 2. Add offset flow

- [x] 2.1 Implement `PromptAdd()` method using Spectre prompts:
  - `TextPrompt<string>` for address (hex validation, 0x00000-0xFFFFF range, `c` to cancel)
  - `SelectionPrompt<string>` for type (`Cancel` first, then all 12 types)
  - `TextPrompt<int>` for size (only for string/bytes, `0` to cancel)
- [x] 2.2 On successful add: append to defs list, set dirty=true
- [x] 2.3 After add: `client.ClearOffsets()` + `client.RegisterOffsets(defs)`

## 3. Edit offset flow

- [x] 3.1 Implement `PromptEdit(OffsetDefinition existing)` method — same prompts as Add but with `.DefaultValue()` pre-filled
- [x] 3.2 On successful edit: replace def at selectedIndex, set dirty=true
- [x] 3.3 After edit: `client.ClearOffsets()` + `client.RegisterOffsets(defs)`

## 4. Delete offset

- [x] 4.1 On D key: remove def at selectedIndex, clamp selectedIndex to valid range
- [x] 4.2 Set dirty=true
- [x] 4.3 Re-register: `client.ClearOffsets()` + `client.RegisterOffsets(defs)`

## 5. Write file

- [x] 5.1 Add serialization method (in OffsetParser or TuiMode) that converts `List<OffsetDefinition>` to text format: `0x{addr:X4},{type}[,{size}]` (use 4 hex digits for addr <= 0xFFFF, 5 for addr > 0xFFFF)
- [x] 5.2 On W key: write serialized defs to `filePath`, set dirty=false
- [x] 5.3 Show confirmation in lastError: `"Written {count} offsets to {filename}"`

## 6. Dirty state and quit guard

- [x] 6.1 Add `isDirty` bool, set true on add/edit/delete, false on reload/write
- [x] 6.2 Show `[modified]` in table caption when dirty
- [x] 6.3 On Q with dirty: exit Live, show `ConfirmationPrompt("Unsaved changes to mappings. Quit anyway?")`, re-enter Live if declined

## 7. UI updates

- [x] 7.1 Add `h for help` to the right side of the caption bar
- [x] 7.2 Update `HelpPanel()` with new keybindings: A (add), E (edit), D (delete), W (write to file)
- [x] 7.3 Update address display format: 4 hex digits for <= 0xFFFF, 5 for > 0xFFFF

## 8. Verification

- [x] 8.1 Manual test: add an offset, verify it appears and reads values
- [x] 8.2 Manual test: edit an offset, verify re-registration works
- [x] 8.3 Manual test: delete an offset, verify selection clamps correctly
- [x] 8.4 Manual test: W writes correct file format, R reloads it cleanly
- [x] 8.5 Manual test: dirty flag appears/clears correctly
- [x] 8.6 Manual test: cancel at each prompt step returns to Live with no changes
- [x] 8.7 Manual test: quit with dirty state shows confirmation
