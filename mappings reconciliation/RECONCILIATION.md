# Mappings reconciliation

Sources:

- **Guide**: `FSUIPC SDK/FSUIPC for Programmers.pdf`, main offset table plus the FS2000 Panels token table. It's extracted to `fsuipc-guide-offsets.csv` by `parse_guide.py` (1,131 main rows, 182 token rows). The guide dates from the FS2004 era, so offsets FSUIPC added later (FSX/P3D/MSFS) aren't in it.
- **X-Plane datarefs**: `DataRefs.txt` from the sim machine (X-Plane 12, build 1210).
- **Mappings**: `xplane_uipc/mappings.toml` (88 mappings).
- **Log**: `uipc.log` in this folder.

Generated files:

| File | Made by | Contents |
| --- | --- | --- |
| `fsuipc-guide-offsets.csv` | `parse_guide.py` | Offset, size and description from the guide |
| `mappings-vs-guide.tsv` | `reconcile.py` | Each mapping's size against the guide |
| `datarefs-check.tsv` | `check_datarefs.py` | Each dataref a mapping uses, checked against `DataRefs.txt` |

To regenerate everything, run these from this folder. They need Python 3.11 or newer (`.python-version` pins 3.14) and `pdftotext` on the PATH. Git for Windows provides `pdftotext`, but only inside Git Bash.

```bash
py parse_guide.py "../FSUIPC SDK/FSUIPC for Programmers.pdf" > fsuipc-guide-offsets.csv
py reconcile.py fsuipc-guide-offsets.csv ../xplane_uipc/mappings.toml > mappings-vs-guide.tsv
py check_datarefs.py DataRefs.txt ../xplane_uipc/mappings.toml > datarefs-check.tsv
```

## 1. Datarefs used by mappings.toml

The mappings use 105 datarefs:

| Result | Count |
| --- | --- |
| Exists in X-Plane, writability matches | 88 |
| Not in `DataRefs.txt` | 17 |

- **Not in DataRefs.txt (expected):** all 17 come from add-ons or our own helper: Zibo 737 (`laminar/B738/...`), the 737 doors (`737u/...`), BetterPushback (`bp/started`) and the Lua helper (`xplane-uipc-helper/...`). They're only present when that add-on is loaded.


## 2. mappings.toml against the guide

| Result | Count |
| --- | --- |
| Size matches the guide | 72 |
| Size differs | 2 |
| Inside a larger documented field | 3 |
| Not in the guide | 11 |

**Size differs**

| Offset | Mapping | Guide | Suggestion |
| --- | --- | --- | --- |
| `0x3308` | `u32` static `0xFADEFFFF` | 2 bytes, FS version | Intentional. The SDK reads 4 bytes here and checks `0xFADE` in the high word (`0x330A`). Keep it. |
| `0x3102` | `u16` (battery expr) | **1 byte**, "read for state, write to control" | Change to `u8`. Reads work either way, but if this is made writable, a 1-byte client write would fail the size check (the same problem as FSInterrogate and 0x028C). |

**Inside a larger documented field** (fine as is)

- `0x0574`: the integer metres half of the 8-byte altitude at `0x0570`. That's intentional.
- `0x3306`: the high word of the FSUIPC version at `0x3304`. That's intentional.
- `0x7B91`: inside a block the guide marks "Reserved" (`0x78D0`). Later FSUIPC versions assigned it.

**Not in the guide** (later FSUIPC offsets; these need a newer "FSUIPC Offsets Status" document to check)

`0x0B52`, `0x0B54`, `0x0B5C` (APU), `0x126C`, `0x2480` (livery), `0x341D` (seat belts), and the helper date offsets `0x023D`, `0x0242`, `0x0244`, `0x0245`, `0x024A`.

## 3. Offsets missing in the log

The log comes from a build with zero-fill, so all of these already read as zeros. Mapping them stops the warnings and gives the client real values where we have them.

| Offset | Bytes | Guide says | Suggestion |
| --- | --- | --- | --- |
| `0x3124` | 1 | FS2002 only: "electric always available" flag | Static `0` (u8). |
| `0x0C18` | 2 | International units: 0=US, 1=Metric+feet, 2=Metric+metres | Static `0` (u16). `DataRefs.txt` has no matching unit setting. |
| `0x0272`, `0x0273` | 2, 2 | Not in the guide | Unknown. Reading 2 bytes at two neighbouring offsets is odd, and could be two 1-byte fields read sloppily. Check a newer offsets document before mapping. Zero-fill already answers `0`. |
| `0x030C` | 4 | VS at touchdown (copy of `0x02C8` while airborne) | **Already mapped**, but not served. It uses `update_if_expr` (`$OnGround 0 ==`), and starting on the ground means no value has ever been stored, so the offset stays absent until the first time airborne. Suggested code fix: when `update_if_expr` is false and nothing has been stored yet, serve `0` rather than nothing. |
| `0x0658` | 4 | Not in the guide | Unknown. Check a newer offsets document. |
| `0x8320` | 1 | View mode. FS2004: 1=cockpit, 2=virtual cockpit, 3=tower, 4=spot, 5=top down | `sim/graphics/view/view_type` exists (int, read-only), but `DataRefs.txt` doesn't list its codes. The expression below assumes 1000 = 2D panel and 1026 = 3D cockpit; **confirm those in DataRefTool by switching views**. Static `2` (virtual cockpit) is a safe alternative. |
| `0x3126` | 1 | Set view direction, **write only** | Static `0`. Reads mean nothing. |
| `0x31E8` | 4 | Surface type (0=concrete, 1=grass, 2=water, 4=asphalt, 8=snow, 12=dirt, 14=gravel, 254=unknown) | `sim/flightmodel/ground/surface_texture_type` exists (int) with its own codes (0=None, 1=Water, 2=Concrete, 3=Asphalt, 4=grass, 5=Dirt, 6=Gravel, 7=Lake, 8=Snow, 9=Shoulder, 10=Blastpad, 11=Ground, 12=Object). The expression below translates them. It was checked with `expr-calculator` for all 13 codes. |
| `0x3C00` | 256 | Path of the current AIR file, zero padded. `0x32FC` counts changes | `sim/aircraft/view/acf_relative_path` exists (`byte[]`, read-only, "Path of current ACF"). It gives X-Plane's `.acf` path rather than an FS `.air` path, which is fine for identifying the aircraft. If clients watch `0x32FC` for changes, that needs a plugin-side counter like `0x337E`. |
| `0x31F4` | 4 | Pushback control: write 0–3 | Static `3` ("off"). **`0x31F0` (status) is already mapped** to BetterPushback's `bp/started` and X-Plane's `pushback_attached`, so leave it alone. |
| `0x66F8` | 8 | Inside `0x66C0` (64 bytes): "Free for general use" | The client is using FSUIPC's scratch area to store its own data. Suggested code change: a built-in 64-byte store for `0x66C0`–`0x66FF` that keeps whatever is written and returns it on read. |
| `0x0D6C` (write) | 4 | Parameter for the next Macro/Lua request | See `0x0D70`. |
| `0x0D70` (write) | 40 | "Write the identity string of a Macro or Lua control for FSUIPC to execute" | The client is asking FSUIPC to run a macro or Lua script, which we can't do. Suggested code change: accept and log the request string, as with the `0x8001` key write, instead of warning about an unmapped offset. A capture would show which macro it wants. |

### Suggested mappings

These need no code changes:

```toml
[[mapping]]
offset       = 0x3124  # "electric always available" (FS2002 only)
fsuipc_type  = "u8"
static_value = 0

[[mapping]]
offset       = 0x0C18  # International units: 0 = US
fsuipc_type  = "u16"
static_value = 0

[[mapping]]
offset       = 0x3126  # Set view direction (write only)
fsuipc_type  = "u8"
static_value = 0

[[mapping]]
offset       = 0x31F4  # Pushback control: 3 = off
fsuipc_type  = "u32"
static_value = 3

[[mapping]]
offset      = 0x31E8  # Surface type, X-Plane codes translated to FSUIPC's
fsuipc_type = "u32"
datarefs    = { S = "sim/flightmodel/ground/surface_texture_type" }
# none→254 unknown, water/lake→2, concrete/object→0, asphalt/shoulder/blastpad→4,
# grass→1, dirt/ground→12, gravel→14, snow→8
expr = "$S 0 == 254 * $S 1 == 2 * + $S 3 == 4 * + $S 4 == 1 * + $S 5 == 12 * + $S 6 == 14 * + $S 7 == 2 * + $S 8 == 8 * + $S 9 == 4 * + $S 10 == 4 * + $S 11 == 12 * +"

[[mapping]]
offset      = 0x3C00  # Path of the current aircraft
fsuipc_type = "string"
size        = 256
dataref     = "sim/aircraft/view/acf_relative_path"

[[mapping]]
offset      = 0x8320  # View mode: 1 = 2D cockpit, 2 = virtual cockpit, 4 = other
fsuipc_type = "u8"
datarefs    = { V = "sim/graphics/view/view_type" }
# Verify 1000 / 1026 in DataRefTool before relying on this.
expr = "$V 1000 == 1 * $V 1026 == 2 * + $V 1000 != $V 1026 != * 4 * +"
```

### Suggested code changes (each worth its own openspec change)

1. **`update_if_expr` initial value**: serve `0` until the first update, so `0x030C` exists on a cold start on the ground.
2. **User scratch area `0x66C0`–`0x66FF`**: keep the bytes a client writes and return them on read.
3. **Macro/Lua requests (`0x0D6C`/`0x0D70`)**: accept and log the request instead of warning.
