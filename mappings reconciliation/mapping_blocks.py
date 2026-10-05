"""Split mappings.toml into its [[mapping]] blocks.

Shared by reconcile.py and check_datarefs.py. The blocks are kept as raw text
(rather than parsed with tomllib) so the scripts work on files that are only
partly valid while being edited.
"""

import re
from collections.abc import Iterator


def mapping_blocks(toml_path: str) -> Iterator[tuple[str, str]]:
    """Yield (offset text as written, e.g. "0x028C", block text) for each [[mapping]]."""
    with open(toml_path, encoding="utf-8", errors="surrogateescape") as f:
        text = f.read().replace("\r", "")
    for block in re.split(r"^\[\[mapping\]\]", text, flags=re.M):
        if m := re.search(r"^\s*offset\s*=\s*(0x[0-9A-Fa-f]+)", block, re.M):
            yield m[1], block
