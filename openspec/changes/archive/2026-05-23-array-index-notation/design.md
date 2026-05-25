## Context

Simple `[[mapping]]` entries have two ways to specify an array element: the `dataref` string and the separate `array_index` field. Expression-based mappings (`datarefs` map) already consolidate this into a single `path[N]` notation via the `parse_dataref_with_index` helper. The simple path bypasses this helper, meaning `[N]` in the dataref string is passed literally to `XPLMFindDataRef`, which fails.

The fix is to route the simple path through the same parser, then remove the now-redundant `array_index` field.

### Data flow

```
Before:
  dataref = "path"  +  array_index = N
    → MappingSource::Simple { dataref_path: "path", array_index: N }
    → ResolvedRef::resolve("path", N)
    → XPLMFindDataRef("path"), index N used in getter

After:
  dataref = "path[N]"
    → parse_dataref_with_index → ("path", N)
    → MappingSource::Simple { dataref_path: "path", array_index: N }
    → ResolvedRef::resolve("path", N)
    → XPLMFindDataRef("path"), index N used in getter

Same internal representation, different parsing layer.
```

## Goals / Non-Goals

**Goals:**
- Simple mappings accept `dataref = "path[N]"` and correctly resolve the dataref + index
- `array_index` field is removed from TOML — serde rejects unknown fields on `RawMapping`
- The one active `array_index = 0` in `mappings.toml` is removed (redundant with existing `[0]` in dataref)

**Non-Goals:**
- Not changing `MappingSource::Simple` internal enum — it keeps `array_index` as a derived field
- Not changing `ResolvedRef`, `plugin_state.rs`, or the read/write paths
- Not touching expression mappings (they already work correctly)
- Not updating commented-out `# array_index = N` entries in mappings.toml (cosmetic, optional)

## Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Where to parse | In `load_mappings`, simple branch | Reuse existing `parse_dataref_with_index`; no new abstraction needed |
| Error on unknown TOML field | serde default behavior | If someone writes `array_index`, TOML deserialization will reject it. Minimal surprise — they get an error immediately at parse time |
| Non-numeric `[foo]` | Treat as literal (no index parsed, stays scalar -1) | Same behavior as expression path; avoids silent corruption |
| What if both `[N]` and old-style `array_index` are present | N/A — field is removed, serde rejects | Breaking change is intentional |

## Risks / Trade-offs

- **Breaking change**: Any user with `array_index` in their mappings.toml will get a parse error. Mitigation: clean error message from serde; easy migration to `dataref = "path[N]"`.
- **Commented examples stale**: ~300 commented `# array_index = N` entries in `mappings.toml` still use the old style. They're commented out so no runtime impact, but would confuse anyone uncommenting them. Optionally update them.
