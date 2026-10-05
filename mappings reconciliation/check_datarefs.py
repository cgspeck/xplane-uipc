"""Check every dataref used in mappings.toml against X-Plane's DataRefs.txt.

Usage:
    py check_datarefs.py DataRefs.txt ../xplane_uipc/mappings.toml > datarefs-check.tsv

Status per dataref: "ok", "NOT FOUND" (a sim/ dataref X-Plane doesn't have),
"third-party" (an add-on or plugin dataref, not listed in DataRefs.txt) or
"READ-ONLY BUT WRITABLE MAPPING".
"""

import io
import re
import sys
from typing import TypedDict

from mapping_blocks import mapping_blocks


class DatarefInfo(TypedDict):
    type: str  # e.g. "int", "float[8]", "byte[]"
    writable: str  # "y" or "n"
    units: str
    desc: str


def load_datarefs(path: str) -> dict[str, DatarefInfo]:
    """DataRefs.txt rows: name, type, writable (y/n), units, description."""
    datarefs: dict[str, DatarefInfo] = {}
    with open(path, encoding="utf-8", errors="surrogateescape") as f:
        next(f)  # version line
        for line in f:
            fields = line.rstrip("\r\n").split("\t")
            if not fields[0] or "/" not in fields[0]:
                continue
            fields += [""] * (5 - len(fields))
            name, type_name, writable, units, desc = fields[:5]
            datarefs[name] = {"type": type_name, "writable": writable, "units": units, "desc": desc}
    return datarefs


def referenced_datarefs(block: str) -> list[str]:
    refs: list[str] = []
    if m := re.search(r'^\s*dataref\s*=\s*"([^"]+)"', block, re.M):
        refs.append(m[1])
    if m := re.search(r"^\s*datarefs\s*=\s*\{([^}]*)\}", block, re.M):
        refs += re.findall(r'"([^"]+/[^"]+)"', m[1])
    return refs


def main() -> None:
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    datarefs = load_datarefs(sys.argv[1])

    if isinstance(sys.stdout, io.TextIOWrapper):  # LF, UTF-8 output on Windows too
        sys.stdout.reconfigure(encoding="utf-8", errors="surrogateescape", newline="\n")
    print("offset\twritable_mapping\tdataref\tstatus\txp_type\txp_writable\tunits\tdescription")
    for offset_text, block in mapping_blocks(sys.argv[2]):
        writable = "yes" if re.search(r"^\s*writable\s*=\s*true", block, re.M) else "no"
        for ref in referenced_datarefs(block):
            base = re.sub(r"\[\d+\]$", "", ref)  # dataref[N] → dataref
            entry = datarefs.get(base)
            if not entry:
                status = "NOT FOUND" if base.startswith("sim/") else "third-party"
            elif writable == "yes" and entry["writable"] != "y":
                status = "READ-ONLY BUT WRITABLE MAPPING"
            else:
                status = "ok"
            details = [entry["type"], entry["writable"], entry["units"], entry["desc"][:80]] if entry else [""] * 4
            print("\t".join([offset_text, writable, ref, status, *details]))


if __name__ == "__main__":
    main()
