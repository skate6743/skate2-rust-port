"""Regenerates 90-file-index.md and 91-address-index.md from the source.

Run from anywhere: python docs/skate-3-physics/tools/extract_indexes.py
"""
import re
import subprocess
from collections import defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
OUT = REPO / "docs" / "skate-3-physics"
OUT.mkdir(parents=True, exist_ok=True)
CORE = REPO / "crates/skate2-core/src"
MODULE = CORE / "physics/modes/skate3"

S3_WORDS = re.compile(r"TU3|Skate ?3|skate3|S3\b", re.I)
EXCLUDE = re.compile(r"challenge/|/ui/|skate3/skate3\.rs|dlc|marker_ui|online|weather|rain|skate3_link|render|gpu_|map_|district|living_world/assets|stream|edit_skater|selected_session_(career|freeskate|public_facade)|world_painter|main\.rs|live_skating_client|game_audio|census|nav_power|_corpus\.rs")


def rel(path):
    return path.relative_to(REPO).as_posix()


def read(path):
    return path.read_bytes().decode("utf-8", "replace").replace("\r\n", "\n")


module_files = sorted(MODULE.rglob("*.rs"))
integration = set()
for path in list(CORE.rglob("*.rs")) + list((REPO / "crates/skate2-audio/src").rglob("*.rs")) + list((REPO / "apps/skate2-pc/src").rglob("*.rs")):
    if MODULE in path.parents:
        continue
    text = read(path)
    if "skate3_native" in text or "skate3" in path.name:
        r = rel(path)
        if "skate3" in path.name and not re.search(r"challenge/|skate3/skate3\.rs|skate3_link", r):
            integration.add(path)
        elif not EXCLUDE.search(r):
            integration.add(path)
shared = set()
for path in list(CORE.rglob("*.rs")) + list((REPO / "crates/skate2-audio/src").rglob("*.rs")):
    if MODULE in path.parents or path in integration or EXCLUDE.search(rel(path)):
        continue
    text = read(path)
    if len(S3_WORDS.findall(text)) >= 3:
        shared.add(path)
integration = sorted(integration)
shared = sorted(shared)

ITEM = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:unsafe\s+)?(struct|enum|trait|fn|type|const|static|mod)\s+([A-Za-z_][A-Za-z0-9_]*)", re.M)


def summary(path):
    text = read(path)
    lines = text.split("\n")
    doc = []
    for line in lines:
        stripped = line.strip()
        if stripped.startswith("//!"):
            doc.append(stripped[3:].strip())
        elif doc and not stripped.startswith("//!"):
            break
        elif stripped.startswith("#!") or not stripped:
            continue
        elif not doc:
            break
    paragraph = []
    for line in doc:
        if not line:
            if paragraph:
                break
            continue
        paragraph.append(line)
    text_doc = " ".join(paragraph)
    if len(text_doc) > 420:
        text_doc = text_doc[:417].rsplit(" ", 1)[0] + "..."
    items = []
    for kind, name in ITEM.findall(text):
        if kind in ("struct", "enum", "trait") and name not in items:
            items.append(name)
    fns = []
    for kind, name in ITEM.findall(text):
        if kind == "fn" and name not in fns and not name.startswith("test"):
            fns.append(name)
    return len(lines), text_doc, items[:10], fns[:8]


def is_test(path):
    r = rel(path)
    return "/tests/" in r or r.endswith("_tests.rs") or r.endswith("/tests.rs") or "probe" in path.name or "corpus" in path.name


def section(title, intro, files, base):
    out = [f"## {title}", "", intro, ""]
    groups = defaultdict(list)
    for path in files:
        parent = path.parent
        key = rel(parent)
        groups[key].append(path)
    for key in sorted(groups):
        out.append(f"### `{key}/`")
        out.append("")
        for path in groups[key]:
            count, doc, items, fns = summary(path)
            tag = " *(tests)*" if is_test(path) else ""
            line = f"- **`{path.name}`** ({count} lines){tag}"
            if doc:
                line += f": {doc}"
            out.append(line)
            extra = []
            if items:
                extra.append("types: " + ", ".join(f"`{i}`" for i in items))
            if fns and not doc:
                extra.append("fns: " + ", ".join(f"`{f}`" for f in fns))
            if extra:
                out.append("  - " + "; ".join(extra))
        out.append("")
    return out


lines = [
    "# File index (generated)",
    "",
    "Generated from the source by reading each file's own module documentation (`//!`) and its",
    "type names; files without module docs list their first functions. Paths are relative to the",
    "repository root. *(tests)* marks unit tests, probes and fixtures.",
    "",
    f"- Skate 3 physics module: {len(module_files)} files",
    f"- Integration layer: {len(integration)} files",
    f"- Skate 2 systems carrying Skate 3 behaviour: {len(shared)} files",
    "",
]
lines += section("1. The Skate 3 physics module (`crate::skate3_native`)",
                 "The captured Skate 3 controller/physics code and its additions, compiled inside `skate2-core` "
                 "with the `skate3-runtime` feature (default on).", module_files, MODULE)
lines += section("2. Integration layer (Skate 2 host <-> Skate 3 physics)",
                 "Files that create, drive, feed or read the Skate 3 player from Skate 2's player, assembly, "
                 "scoring, animation, camera, input and session code.", integration, CORE)
lines += section("3. Skate 2 systems that carry Skate 3 (TU3) behaviour",
                 "Shared Skate 2 code with Skate 3 branches or TU3 values: scoring, wipeouts, audio, "
                 "breadcrumbs, collision, pedestrians. Each file names its Skate 3 parts in its own comments.",
                 shared, CORE)
(OUT / "90-file-index.md").write_text("\n".join(lines) + "\n", encoding="utf-8")

# Address index.
ADDR_CANDIDATE = re.compile(r"(?<![0-9])(8[23][0-9A-Fa-f]{6})(?![0-9A-Fa-f])")
HEX = set("0123456789abcdefABCDEF")


class _Addr:
    """Addresses written as 0x82..., Name82..., `Name`82... or bare 82...: a candidate glued to a
    run of hex characters counts only when that run has no digit (a word such as Update, not a
    longer hash)."""

    @staticmethod
    def findall(line):
        found = []
        for match in ADDR_CANDIDATE.finditer(line):
            start = match.start()
            if start > 0 and line[start - 1] in "xX":
                if start >= 2 and line[start - 2] == "0":
                    found.append(match.group(1))
                continue
            run = start
            while run > 0 and line[run - 1] in HEX:
                run -= 1
            prefix = line[run:start]
            if any(ch.isdigit() for ch in prefix):
                continue
            found.append(match.group(1))
        return found


ADDR = _Addr()
entries = defaultdict(list)
for path in module_files + integration + shared:
    text = read(path).split("\n")
    in_module = MODULE in path.parents
    s3_file = in_module or "skate3" in path.name
    for index, line in enumerate(text):
        found = ADDR.findall(line)
        if not found:
            continue
        window = " ".join(text[max(0, index - 2): index + 1])
        if re.search(r"TU3|Skate ?3|skate3|S3\b", window):
            binary = "TU3"
        elif re.search(r"Skate ?2|S2\b|sk82", window):
            binary = "Skate 2"
        else:
            binary = "TU3?" if s3_file else "Skate 2?"
        comment = line.strip()
        if len(comment) > 220:
            comment = comment[:217] + "..."
        for address in found:
            entries[address.lower()].append((binary, rel(path), index + 1, comment))

out = [
    "# Address index (generated)",
    "",
    "Every executable address the Skate 3 physics files, the integration layer and the shared Skate 2",
    "files name in their comments, sorted by address, with the line that names it.",
    "",
    "**Which executable:** `TU3` = Skate 3 title update 3 (`default.xex`, the version the port",
    "matches); `Skate 2` = Skate 2's executable. The label comes from the line or the two lines",
    "before it; `TU3?` / `Skate 2?` mean the line does not say and the label is the file's context",
    "(a Skate 3 file or a Skate 2 file): check the line. Both games load at 0x82000000, so the same",
    "number can name different functions in the two executables.",
    "",
    f"{len(entries)} addresses, {sum(len(v) for v in entries.values())} mentions.",
    "",
    "| Address | Exe | Where | Line |",
    "|---|---|---|---|",
]
for address in sorted(entries):
    for binary, path, line_no, comment in entries[address]:
        safe = comment.replace("|", "\\|")
        out.append(f"| `0x{address}` | {binary} | `{path}:{line_no}` | {safe} |")
(OUT / "91-address-index.md").write_text("\n".join(out) + "\n", encoding="utf-8")

counts = defaultdict(int)
for v in entries.values():
    for binary, *_ in v:
        counts[binary] += 1
print("module", len(module_files), "integration", len(integration), "shared", len(shared))
print("addresses", len(entries), dict(counts))
print("integration files:")
for p in integration:
    print("  ", rel(p))
