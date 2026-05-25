## Context

The plugin has two concurrent execution paths:
1. **IPC thread** — spawned in `XPluginEnable()` via `create_ipc_window_and_run()`, processes client read/write requests against the shared `Table`
2. **Flight loop** — runs at 20Hz, calls `PluginState::update()` which populates `Table.entries` from resolved mappings

The `Table` is a global `LazyLock<RwLock<Arc<RwLock<Table>>>>` that starts empty (`Table::new()`). Entries are only added during `update()`. The IPC thread can process requests before the first flight loop iteration runs.

Additionally, `update()` writes directly to `table.entries[m.offset as usize]` bypassing `Table::insert()`, so `active` and `writable` vectors are never populated. The `WarnedSet` permanently records "not found" bits — once an offset triggers a warning, it is never re-checked even after the entry appears.

## Goals / Non-Goals

**Goals:**
- Eliminate "not in table" warnings for valid mappings that exist but haven't been populated yet
- Ensure `active` and `writable` vectors are correctly maintained for all entries
- Allow the warning system to reflect current table state, not just first observation

**Non-Goals:**
- Do not change the mapping file format or loading logic
- Do not change the 20Hz flight loop update rate
- Do not add new dependencies

## Decisions

### Decision 1: Populate table synchronously before IPC thread starts

**Approach:** Add a `populate_table()` method to `PluginState` that performs a one-time initial population of all mappings into the `Table` before the IPC thread is spawned. Call this in `XPluginEnable()` after `load_mappings_and_init()` but before `thread::spawn()`.

**Rationale:** Static mappings (like `0x3304`, `0x3308`) have values immediately available — they don't need X-Plane datarefs. For dataref-based mappings, we resolve datarefs during `ResolvedMapping::new()` already, so we can read values at this point. This guarantees the table is ready before any IPC client connects.

**Alternatives considered:**
- *Block IPC reads until first update()* — adds latency and complexity to the read path
- *Pre-populate static values only* — partial fix, dataref mappings would still race
- *Signal from flight loop when table is ready* — IPC thread would need to queue/delay requests

### Decision 2: Use `Table::insert()` in `update()` instead of direct array assignment

**Approach:** Replace `table.entries[m.offset as usize] = Some(...)` with `table.insert(m.offset, entry)` in `PluginState::update()`.

**Rationale:** `Table::insert()` already exists and correctly maintains `active` and `writable` vectors. The direct assignment was an oversight. This fixes the write path which checks `table.active.contains()` and `table.writable.contains()`.

**Trade-off:** `insert()` pushes to `active`/`writable` vectors on every call (even if entry already exists). Since `update()` runs at 20Hz, these vectors will grow with duplicates. We need to either:
- (a) Clear `active`/`writable` at the start of `update()` and rebuild them, or
- (b) Modify `insert()` to deduplicate, or
- (c) Add a `replace()` method that updates an existing entry without pushing to vectors

**Chosen: (a)** — Clear and rebuild `active`/`writable` at the start of `update()`. This is simplest and ensures correctness. The vectors are small (hundreds of entries at most) and rebuilt 20 times/sec.

### Decision 3: Add per-offset state tracking to `WarnedSet`

**Approach:** Add a `clear_key(&self, key: u16, category: WarnCategory)` method to `WarnedSet` that clears the bit for a specific offset+category. After `update()` populates the table, call `warned_set.clear_key(offset, ReadNotExist)` for each newly-added offset.

**Rationale:** The `WarnedSet` already has `clear()` and `clear_all()` methods. Adding `clear_key()` is a minimal extension. This allows offsets that were "not found" to be re-evaluated after they appear in the table.

**Alternatives considered:**
- *Replace WarnedSet with a TTL-based cache* — more complex, overkill for this use case
- *Remove deduplication entirely* — would spam logs on every poll cycle
- *Clear entire category on each update()* — would re-warn for offsets that are genuinely missing

## Risks / Trade-offs

| Risk | Mitigation |
|------|------------|
| `populate_table()` may read datarefs before X-Plane is fully initialized | `ResolvedRef::resolve()` returns null handles for missing datarefs; `read_xplane()` returns `None` for null handles — these entries simply won't be populated until the first `update()` |
| Clearing `active`/`writable` vectors each update could briefly cause write rejections | The vectors are rebuilt within the same `write()` lock hold, so no window exists where they're empty while IPC reads occur |
| `clear_key()` on `WarnedSet` is a new atomic operation | Uses the same `fetch_and` pattern as existing `fetch_or` — well-tested atomic pattern |
| Duplicate entries in `active`/`writable` if we don't clear | Addressed by Decision 2 choice (a) — vectors are cleared at start of each `update()` |
