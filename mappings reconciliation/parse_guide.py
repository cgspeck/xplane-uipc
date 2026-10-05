"""Extract the offset tables from "FSUIPC for Programmers.pdf" into CSV.

Usage:
    py parse_guide.py "../FSUIPC SDK/FSUIPC for Programmers.pdf" > fsuipc-guide-offsets.csv

Needs `pdftotext` (poppler or xpdf) on PATH. Reads two tables: the main
"Offset Size Use" table, and the FS2000 Panels token table that follows it.
Output columns: offset, size, description, source ("main" or "panels-token").
"""

import re
import subprocess
import sys

# A row's status column ("Ok Ok") can share a line with the start of the next row.
MERGED_ROW = re.compile(r"^(.*?\b(?:Ok|No|Yes))\s+([0-9A-F]{4}\s+(?:\d+|Varies)\s+.*)$", re.ASCII)
MAIN_ROW = re.compile(
    r"^([0-9A-F]{4})(?:\s*[-/]\s*[0-9A-F]{2,4})?"
    r"\s+(?i:(\d+(?:\s*x\s*\d+)?|Varies|var\.?|\d+\+))"
    r"(?:\s+(.*))?$",
    re.ASCII,
)
# The FS2002 / FS2004 status columns that trail each row's description.
TRAILING_STATUS = re.compile(r"\s+((?:Ok|No|Yes|\?|-)(?:[ ,]+[^ ]+){0,8})\s*$", re.ASCII)
TOKEN_ROW = re.compile(r"^([0-9A-F]{4})\s+(\S+)\s+(\d+)\s+(\w+)\b(.*)$", re.ASCII)

TOKEN_TYPE_SIZES = {
    "SINT32": 4, "UINT32": 4, "BOOL": 4, "ENUM": 4, "FLOAT64": 8, "SINT16": 2,
    "UINT16": 2, "SINT8": 1, "UINT8": 1, "FLAGS": 4, "VAR32": 4, "PVOID": 4,
    "SINT64": 8, "UINT64": 8, "FLOAT32": 4,
}


def pdf_lines(pdf_path):
    text = subprocess.run(
        ["pdftotext", "-raw", pdf_path, "-"], check=True, capture_output=True
    ).stdout.decode("utf-8", errors="surrogateescape")
    lines = []
    for line in text.split("\n"):
        line = line.replace("\r", "").replace("\f", "")
        merged = MERGED_ROW.match(line)
        lines.extend(merged.groups() if merged else [line])
    return lines


def parse_main(lines, start, end):
    rows = []
    current = None
    last = -1
    for line in lines[start + 1 : end]:
        if line.startswith("Body Frame Of Reference"):  # prose after the table
            break
        m = MAIN_ROW.match(line)
        # Offsets only increase down the table; a "row" far below the last one
        # is a number inside a description.
        if m and int(m[1], 16) >= last - 0x10:
            if current:
                rows.append(current)
            current = {"off": m[1], "size": m[2], "desc": m[3] or ""}
            last = int(m[1], 16)
        elif current:
            current["desc"] += " " + line
    if current:
        rows.append(current)
    for row in rows:
        desc = TRAILING_STATUS.sub("", row["desc"], count=1)
        row["desc"] = re.sub(r"\s+", " ", desc, flags=re.ASCII).replace('"', '""')
    return rows


def parse_tokens(lines, start):
    rows = []
    for line in lines[start + 1 :]:
        m = TOKEN_ROW.match(line)
        if not m:
            continue
        off, name, _token_id, type_name, _status = m.groups()
        size = TOKEN_TYPE_SIZES.get(type_name.upper())
        if size is None:
            bits = re.search(r"(8|16|32|64)$", type_name)
            size = int(bits[1]) // 8 if bits else "?"
        rows.append({"off": off, "size": size, "desc": f"Panels token {name} ({type_name})"})
    return rows


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    lines = pdf_lines(sys.argv[1])
    main_start = next(i for i, l in enumerate(lines) if l.startswith("Offset Size Use FS2002"))
    token_start = next(i for i, l in enumerate(lines) if l.startswith("Offset Token Name Token Id Type"))
    main_rows = parse_main(lines, main_start, token_start)
    token_rows = parse_tokens(lines, token_start)

    sys.stdout.reconfigure(encoding="utf-8", errors="surrogateescape", newline="\n")
    print("offset,size,description,source")
    for row in main_rows:
        print(f'0x{row["off"]},{row["size"]},"{row["desc"]}",main')
    for row in token_rows:
        print(f'0x{row["off"]},{row["size"]},"{row["desc"]}",panels-token')
    print(f"{len(main_rows)} main rows, {len(token_rows)} token rows", file=sys.stderr)


if __name__ == "__main__":
    main()
