"""Regenerates 92-changes-since-smokeys-mod.md: the port's Skate 3 files against Smokey's mod.

Usage: python changes_since_smokeys_mod.py <path to Smokey's mod checkout>
"""
import difflib
import sys
from pathlib import Path

OURS = Path(__file__).resolve().parents[3]
THEIRS = Path(sys.argv[1]) if len(sys.argv) > 1 else sys.exit("usage: changes_since_smokeys_mod.py <path to Smokey's mod checkout>")
OUT = OURS / "docs" / "skate-3-physics" / "92-changes-since-smokeys-mod.md"

AREAS = [
    ("Skate 3 physics module", "crates/skate2-core/src/physics/modes/skate3"),
    ("Vendored skate-core", "vendor/skate3-engine/crates/skate-core"),
    ("Vendored skate-data", "vendor/skate3-engine/crates/skate-data"),
    ("Vendored skate-dynamics", "vendor/skate3-engine/crates/skate-dynamics"),
]
INTEGRATION = [
    "crates/skate2-core/src/physics/skate3_player.rs",
    "crates/skate2-core/src/physics/skate3_bridge.rs",
    "crates/skate2-core/src/physics/skate3_terrain.rs",
    "crates/skate2-core/src/physics/skate3_grabs.rs",
    "crates/skate2-core/src/physics/skate3_grinds.rs",
    "crates/skate2-core/src/physics/skate3_pushing.rs",
    "crates/skate2-core/src/physics/assembly/skate3_world.rs",
    "crates/skate2-core/src/physics/assembly/skate3_player_backend.rs",
    "crates/skate2-core/src/physics/assembly/parts/skate3_part_mirror.rs",
    "crates/skate2-core/src/physics/solver/bodies/external_player_driver.rs",
    "crates/skate2-core/src/physics/player/output/physical_player_phys_out_skate3.rs",
    "crates/skate2-core/src/physics/player/skate3_actor_input.rs",
    "crates/skate2-core/src/physics/player/skate3_camera_rate.rs",
    "crates/skate2-core/src/physics/player/skate3_exit.rs",
    "crates/skate2-core/src/physics/player/skate3_spawn.rs",
    "crates/skate2-core/src/physics/player/skate3_tuning.rs",
    "crates/skate2-core/src/session/selected_player_skate3.rs",
    "crates/skate2-core/src/session/selected_player_tick.rs",
    "crates/skate2-core/src/session/skate3_input_capture.rs",
    "crates/skate2-core/src/session/skate3_input_focus.rs",
    "crates/skate2-core/src/camera/frame/camera_skate3.rs",
    "crates/skate2-core/src/animation/runtime/actor_skate3_gestures.rs",
    "crates/skate2-core/src/animation/runtime/skate3_gesture_playback.rs",
    "crates/skate2-core/src/animation/runtime/skate3_gesture_streams.rs",
    "crates/skate2-core/src/actors/skate3_auto_push.rs",
    "crates/skate2-core/src/physics/player/output/conditioning/scoring/skate3_catalog.rs",
    "crates/skate2-core/src/physics/player/output/conditioning/scoring/skate3_hom.rs",
    "crates/skate2-core/src/physics/player/output/conditioning/scoring/wipeout_skate3.rs",
    "crates/skate2-core/src/physics/player/output/conditioning/scoring/attributes.rs",
    "crates/skate2-core/src/living_world/pedestrians/pedestrian_collision_shove.rs",
]


def lines(path):
    return path.read_bytes().decode("utf-8", "replace").replace("\r\n", "\n").split("\n")


def delta(a, b):
    added = removed = 0
    for line in difflib.unified_diff(a, b, lineterm="", n=0):
        if line.startswith("+") and not line.startswith("+++"):
            added += 1
        elif line.startswith("-") and not line.startswith("---"):
            removed += 1
    return added, removed


out = ["# Changes since Smokey's mod (generated)", "",
       "Smokey's mod (a fork of this repository at 0.2.9) brought the Skate 3 Rust Engine's player",
       "into Skate 2. This port took that code as its starting point; this file lists, per file, what",
       "the port changed since then (line counts from a line diff, line endings ignored).",
       "", "- **same**: identical to the mod", "- **changed**: lines added/removed by the port",
       "- **new**: not in the mod (written for the port)", "- **removed**: in the mod, not in the port", ""]
summary = []
for title, rel in AREAS:
    ours, theirs = OURS / rel, THEIRS / rel
    our_files = {p.relative_to(ours).as_posix() for p in ours.rglob("*.rs")} if ours.exists() else set()
    their_files = {p.relative_to(theirs).as_posix() for p in theirs.rglob("*.rs")} if theirs.exists() else set()
    rows = []
    counts = {"same": 0, "changed": 0, "new": 0, "removed": 0}
    total_add = total_rem = 0
    for name in sorted(our_files | their_files):
        if name in our_files and name in their_files:
            a, b = lines(theirs / name), lines(ours / name)
            if a == b:
                counts["same"] += 1
                continue
            add, rem = delta(a, b)
            total_add += add
            total_rem += rem
            counts["changed"] += 1
            rows.append(f"| `{name}` | changed | +{add} / -{rem} |")
        elif name in our_files:
            n = len(lines(ours / name))
            total_add += n
            counts["new"] += 1
            rows.append(f"| `{name}` | new | {n} lines |")
        else:
            counts["removed"] += 1
            rows.append(f"| `{name}` | removed | |")
    summary.append((title, rel, counts, total_add, total_rem))
    out += [f"## {title} (`{rel}`)", "",
            f"{counts['same']} same, {counts['changed']} changed, {counts['new']} new, {counts['removed']} removed; "
            f"+{total_add} / -{total_rem} lines.", ""]
    if rows:
        out += ["| File | Status | Lines |", "|---|---|---|"] + rows + [""]

rows = []
add_all = rem_all = 0
for rel in INTEGRATION:
    ours, theirs = OURS / rel, THEIRS / rel
    if ours.exists() and theirs.exists():
        a, b = lines(theirs), lines(ours)
        if a == b:
            rows.append(f"| `{rel}` | same | |")
        else:
            add, rem = delta(a, b)
            add_all += add
            rem_all += rem
            rows.append(f"| `{rel}` | changed | +{add} / -{rem} |")
    elif ours.exists():
        n = len(lines(ours))
        add_all += n
        rows.append(f"| `{rel}` | new | {n} lines |")
    elif theirs.exists():
        rows.append(f"| `{rel}` | removed | |")
summary.append(("Integration layer", "(listed files)", None, add_all, rem_all))
out += ["## Integration layer", "", f"+{add_all} / -{rem_all} lines over these files.", "",
        "| File | Status | Lines |", "|---|---|---|"] + rows + [""]
OUT.write_text("\n".join(out) + "\n", encoding="utf-8")
for title, rel, counts, add, rem in summary:
    print(title, counts, f"+{add}/-{rem}")
