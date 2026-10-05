"""Compare each mapping in mappings.toml with the sizes documented in the FSUIPC guide.

Usage:
    py reconcile.py fsuipc-guide-offsets.csv ../xplane_uipc/mappings.toml > mappings-vs-guide.tsv

Status per mapping: "ok", "SIZE DIFFERS", "INSIDE FIELD" (the offset falls
inside a larger documented field) or "NOT IN GUIDE".
"""

import io
import re
import sys
from typing import NotRequired, TypedDict

from mapping_blocks import mapping_blocks


class GuideEntry(TypedDict):
    size: str  # bytes as text, "Varies" etc., or "-" for an enclosing field
    desc: str
    inside: NotRequired[bool]  # set when the offset lies inside a larger field


TYPE_WIDTHS = {"i8": 1, "u8": 1, "i16": 2, "u16": 2, "i32": 4, "u32": 4, "f32": 4, "i64": 8, "u64": 8, "f64": 8}
GUIDE_ROW = re.compile(r'^0x([0-9A-F]+),([^,]*),"(.*)",(\S+)$')


def load_guide(csv_path: str) -> dict[int, GuideEntry]:
    guide: dict[int, GuideEntry] = {}
    with open(csv_path, encoding="utf-8", errors="surrogateescape") as f:
        next(f)
        for line in f:
            m = GUIDE_ROW.match(line.rstrip("\r\n"))
            if m:
                # The first row for an offset wins; tokens can repeat an offset.
                guide.setdefault(int(m[1], 16), {"size": m[2], "desc": m[3]})
    return guide


def enclosing_field(guide: dict[int, GuideEntry], offset: int) -> GuideEntry | None:
    """The guide entry for the nearest lower offset, if its size covers `offset`."""
    lower = [base for base in guide if base < offset]
    if not lower:
        return None
    base = max(lower)
    field = guide[base]
    if field["size"].isdigit() and base + int(field["size"]) > offset:
        return {"size": "-", "desc": f"(inside 0x{base:04X}, {field['size']} bytes) {field['desc']}", "inside": True}
    return None


def describe_source(block: str) -> str:
    if m := re.search(r'^\s*dataref\s*=\s*"([^"]+)"', block, re.M):
        source = m[1]
    elif re.search(r"^\s*expr", block, re.M):
        source = "expr"
    elif re.search(r"^\s*static_(value|str)", block, re.M):
        source = "static"
    else:
        source = "?"
    if re.search(r"^\s*writable\s*=\s*true", block, re.M):
        source += " (writable)"
    return source


def main() -> None:
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    guide = load_guide(sys.argv[1])

    if isinstance(sys.stdout, io.TextIOWrapper):  # LF, UTF-8 output on Windows too
        sys.stdout.reconfigure(encoding="utf-8", errors="surrogateescape", newline="\n")
    print("offset\ttype\twidth\tdoc_size\tstatus\tsource\tdoc_description")
    for offset_text, block in mapping_blocks(sys.argv[2]):
        offset = int(offset_text, 16)
        type_name = m[1] if (m := re.search(r'^\s*fsuipc_type\s*=\s*"(\w+)"', block, re.M)) else "?"
        if type_name == "string":
            m = re.search(r"^\s*size\s*=\s*(\d+)", block, re.M)
            width = m[1] if m else "?"
        else:
            width = str(TYPE_WIDTHS.get(type_name, "?"))

        doc = guide.get(offset) or enclosing_field(guide, offset)
        if not doc:
            status = "NOT IN GUIDE"
        elif doc.get("inside"):
            status = "INSIDE FIELD"
        elif doc["size"].isdigit() and width.isdigit() and int(doc["size"]) != int(width):
            status = "SIZE DIFFERS"
        else:
            status = "ok"

        print("\t".join([
            "0x" + offset_text[2:].upper(),
            type_name,
            width,
            doc["size"] if doc else "-",
            status,
            describe_source(block),
            doc["desc"][:90] if doc else "",
        ]))


if __name__ == "__main__":
    main()
