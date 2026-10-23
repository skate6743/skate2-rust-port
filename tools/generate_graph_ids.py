"""Writes the named id constants of the skater and pedestrian state graphs.

Usage:
  python generate_graph_ids.py GRAPH [--check | --missing]   GRAPH: one of GRAPHS below, or all

Reads the graph's declaration data and writes its ids module beside it:
  condition   every condition declaration, named by its class and attributes
  expression  every expression a state or transition evaluates, named by its users
  state       every state, named by its name and index

An expression with one user takes that user's name: TAKEOFF_1355 for a state's guard,
TAKEOFF_1355_TO_DESCENT_1356 for a transition. A shared expression takes what all its users
have in common (every FromGrind state, every Into -> EarlyOut transition), and the names in
SHARED below cover the expressions whose users have nothing in common. --check compares
instead of writing and fails when the file is stale; --missing lists the shared expressions
that still need a name in SHARED.
"""

import re
import sys
from collections import Counter, defaultdict
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import graph_query as gq  # noqa: E402

SRC = gq.ROOT.parents[1]
PEDESTRIANS = SRC / "living_world/pedestrians"
# graph: (declaration data, module inside it or None, ids output, the graph's name in prose)
GRAPHS = {
    "motion": (gq.FILES["motion"], None, gq.ROOT / "motion/generated_motion_ids.rs", "Motion"),
    "action": (gq.FILES["action"], None, gq.ROOT / "action/generated_action_ids.rs", "Action"),
    "pedestrian_motion": (
        gq.ROOT / "pedestrian_motion_data.rs", None, gq.ROOT / "pedestrian_motion_ids.rs",
        "pedestrian Motion",
    ),
    "pedestrian": (
        PEDESTRIANS / "pedestrian_graph_data.rs", None, PEDESTRIANS / "pedestrian_graph_ids.rs",
        "`HCDelegate_Pedestrian`",
    ),
    "big_black": (
        PEDESTRIANS / "pedestrian_big_black_graph_data.rs", "big_black",
        PEDESTRIANS / "pedestrian_big_black_graph_ids.rs", "`HCDelegate_Big_Black`",
    ),
}


def module_text(text, module):
    """The body of `pub(crate) mod module { ... }` in text."""
    start = text.index(f"pub(crate) mod {module} {{")
    depth, i = 0, text.index("{", start)
    for j in range(i, len(text)):
        if text[j] == "{":
            depth += 1
        elif text[j] == "}":
            depth -= 1
            if depth == 0:
                return text[i + 1:j]
    raise ValueError(module)


def load(graph):
    path, module, _, _ = GRAPHS[graph]
    text = path.read_text(encoding="utf-8")
    if module:
        text = module_text(text, module)
    return {
        "attributes": gq.blocks(gq.array_body(text, "ATTRIBUTES"), "Attribute"),
        "behaviours": gq.blocks(gq.array_body(text, "BEHAVIOURS"), "BehaviourDeclaration"),
        "conditions": gq.blocks(gq.array_body(text, "CONDITIONS"), "ConditionDeclaration"),
        "states": gq.blocks(gq.array_body(text, "STATES"), "StateDeclaration"),
        "transitions": gq.blocks(gq.array_body(text, "TRANSITIONS"), "TransitionDeclaration"),
        "indices": [int(v) for v in re.findall(r"\d+", gq.array_body(text, "INDICES"))],
    }


# Attribute keys whose value alone names the condition (`HasIntent` `intent` = `Kickflip`).
IDENTITY = {
    "intent", "state", "grindName", "attribute", "grab", "skater", "anim", "group",
    "landingType", "foot", "direction", "side", "dir", "want", "reason", "timerName",
    "intentName", "handprop", "BranchAnimInfo",
}
COMPARISON = {"greater", "less", "greaterEqual", "lessEqual"}
KEY_NAMES = {
    "InTime": "IN", "inTime": "IN", "waitForTransitions": "WAIT",
    "anglestart": "FROM", "angleend": "TO", "StartTime": "START", "WindowFrameLength": "LENGTH",
}
KEY_ORDER = {"anglestart": 0, "angleend": 1}
# Attribute values without case boundaries, spelled as words.
VALUE_NAMES = {
    "offboardair": "OffboardAir", "earlyout": "EarlyOut", "midout": "MidOut",
    "sequencetoair": "SequenceToAir", "aggressivecapture": "AggressiveCapture",
    "lostinterest": "LostInterest", "alertto": "AlertTo", "angrychase": "AngryChase",
    "joinchase": "JoinChase", "nearbycollisionreaction": "NearbyCollisionReaction",
    "returngreet": "ReturnGreet", "slamreaction": "SlamReaction",
    "startconversation": "StartConversation", "throwhandprop": "ThrowHandProp",
}
# Condition classes too long to lead every name; the doc comment keeps the real class.
CLASS_NAMES = {"HasSpecificWantThatNeedsToBeAddressed": "HasWant"}

# Shared expressions named by hand: their users have no common name, or only a vague one.
SHARED = {
    "motion": {
        2: "WILL_EXPIRE_IN_0",
        3: "IS_RIDING_FAKIE",
        4: "WILL_EXPIRE_IN_0_25_NOT_WAIT",
        5: "WILL_EXPIRE_IN_0_1",
        6: "OB_TIME_TO_LAND_LESS_EQUAL_0",
        10: "MVOBJ_PUSH_LEFT",
        11: "WILL_EXPIRE_IN_0_05",
        12: "MVOBJ_PUSH_RIGHT",
        13: "MVOBJ_PULL_LEFT",
        14: "MVOBJ_PULL_RIGHT",
        15: "MVOBJ_TO_PUSH_RIGHT",
        18: "MVOBJ_TO_PUSH_LEFT",
        19: "MVOBJ_TO_PULL_RIGHT",
        20: "MVOBJ_TO_PULL_LEFT",
        21: "MVOBJ_NOT_MOVING",
        22: "IDLE_TO_RUN_AROUND",
        23: "OFF_BOARD_WILL_EXPIRE_IN_0_1",
        28: "PHYS_FILTERED_STATE_OFFBOARD",
        34: "STRIDE_TO_RUN_AROUND",
        35: "HAS_INTENT_OB_DISMOUNT",
        39: "PHYSICS_WANTS_RUNOUT",
        41: "WILL_EXPIRE_IN_0_01",
        75: "INTO_HAS_INTENT_GRIND_FS_GRAB",
        76: "GRAB_CYCLE_HAS_INTENT_GRIND_FS_GRAB",
        77: "OUT_NOT_HAS_INTENT_GRIND_FS_GRAB",
        78: "INTO_HAS_INTENT_GRIND_BS_GRAB",
        79: "GRAB_CYCLE_HAS_INTENT_GRIND_BS_GRAB",
        80: "OUT_NOT_HAS_INTENT_GRIND_BS_GRAB",
        81: "INTO_HAS_INTENT_GRIND_DOUBLE_GRAB",
        82: "GRAB_CYCLE_HAS_INTENT_GRIND_DOUBLE_GRAB",
        86: "IS_PRO_SKATER_MIKE_CARROLL",
        87: "HAS_INTENT_KICKFLIP",
        88: "WILL_EXPIRE_IN_0_05_NOT_WAIT",
        89: "WILL_EXPIRE_IN_0_02_NOT_WAIT",
        91: "IS_PRO_SKATER_MARK_GONZALES",
        123: "PLANTING_FOOT_LEFT",
        124: "PLANTING_FOOT_RIGHT",
        127: "HAND_PLANT_INTO",
        128: "WILL_EXPIRE_IN_0_02",
        129: "WILL_EXPIRE_IN_0_15",
        130: "HAND_PLANT_ANTIC",
        131: "HAND_PLANT_OUT",
        132: "HAND_PLANT_DISMOUNT_INTO",
        134: "PHYS_FILTERED_STATE_AIR",
        136: "CYC_TO_OUT_HAS_TWEAK_LESS_0_2",
        140: "HAS_INTENT_FS_CHRIST",
        141: "HAS_INTENT_BS_CHRIST",
        143: "HAS_ANIM_ATTRIBUTE_CHRIST_EARLY_OUT",
        145: "HAS_ANIM_ATTRIBUTE_ONE_FOOT_EARLY_OUT",
        146: "OUT_NOT_HAS_INTENT_RIGHT_ONE_FOOT_AIR",
        149: "OUT_NOT_HAS_INTENT_LEFT_ONE_FOOT_AIR",
        154: "NOT_HAS_INTENT_COFFIN",
        161: "HAS_INTENT_MUTE_GRAB",
        162: "CYC_TO_OUT_TWEAK_Y_ABS_LESS_0_2",
        164: "HAS_INTENT_STALE_GRAB",
        168: "HAS_INTENT_BOARD_ADJUST_UP",
        170: "OUT_NOT_HAS_INTENT_NO_FOOT_AIR_WALK",
        175: "HAS_INTENT_BOARD_ADJUST_DOWN",
        195: "IN_PARENT_STATE_FOR_TIME_GREATER_0_1",
        199: "HAS_INTENT_GROUND_BS_GRAB_INTO",
        201: "IS_ENTERING_SKITCH",
        214: "RANDOM_COND",
        216: "BRAKE_INTO",
        217: "OUT_NOT_HAS_INTENT_BRAKE",
        219: "WILL_EXPIRE_IN_0_25",
        221: "SLIDE_OUT_EARLY",
        222: "SLIDE_OUT_MID",
        226: "REVERT_BS",
        227: "REVERT_FS",
        230: "NOT_HAS_INTENT_GROUND_BS_GRAB",
        231: "HAS_INTENT_GROUND_BS_GRAB_CYCLE",
        235: "HAS_INTENT_GROUND_FS_GRAB_CYCLE",
        236: "NOT_HAS_INTENT_GROUND_FS_GRAB",
        243: "NOT_HAS_INTENT_MANUAL_BRAKE",
        250: "NOT_HAS_INTENT_MANUAL_GREATER_0",
        254: "HAS_INTENT_RIGHT_PUSH",
        256: "WILL_EXPIRE_IN_0_01_NOT_WAIT",
        257: "HAS_INTENT_NEW_PUSH",
        267: "CAN_LAND_ON_BOARD",
        281: "STEP_TO_RUN_AROUND",
        285: "HAS_INTENT_GRIND_FS_OR_DOUBLE_GRAB",
        286: "NOT_HAS_INTENT_GRIND_FS_OR_DOUBLE_GRAB",
        287: "HAS_INTENT_GRIND_BS_OR_DOUBLE_GRAB",
        288: "NOT_HAS_INTENT_GRIND_BS_OR_DOUBLE_GRAB",
        289: "GRAB_CYCLE_DIRECT_FS_GRAB_TAIL",
        290: "GRAB_CYCLE_DIRECT_BS_GRAB_NOSE",
        291: "GRIND_DOUBLE_GRAB_OUT",
        292: "HAS_INTENT_OB_DISMOUNT_IN_AIR",
        300: "NEITHER_GROUND_NOR_AIR",
        301: "HAND_PLANT_ONE_FOOT_RIGHT_INTO",
        302: "HAND_PLANT_ONE_FOOT_LEFT_INTO",
        303: "HAND_PLANT_DISMOUNT_OUT",
        304: "TRUCKS_OR_DECK_IN_CONTACT_OR_NOT_AIR",
        305: "INTO_FS_GRAB_RIGHT_ONE_FOOT_AIR",
        306: "INTO_FS_GRAB_LEFT_ONE_FOOT_AIR",
        307: "INTO_BS_GRAB_RIGHT_ONE_FOOT_AIR",
        308: "INTO_BS_GRAB_LEFT_ONE_FOOT_AIR",
        309: "FS_GRAB_OUT_TO_CYC",
        310: "FS_GRAB_OUT_TO_IN_AIR",
        313: "BS_GRAB_OUT_TO_CYC",
        314: "BS_GRAB_OUT_TO_IN_AIR",
        336: "INTO_RIGHT_ONE_FOOT_AIR",
        337: "INTO_LEFT_ONE_FOOT_AIR",
        340: "HAS_INTENT_ANY_KICK_TURN",
        353: "HAS_INTENT_GROUND_DBL_GRAB_OR_COFFIN",
        363: "HAS_INTENT_RIGHT_AND_LEFT_PUSH",
        382: "GRAB_CYCLE_DIRECT_FS",
        383: "GRAB_CYCLE_DIRECT_NOSE",
        384: "GRAB_CYCLE_DIRECT_BS",
        385: "GRAB_CYCLE_DIRECT_TAIL",
        386: "GRAB_CYCLE_DIRECT_DOUBLE",
        403: "HAND_PLANT_ONE_FOOT_RIGHT_OUT",
        404: "HAND_PLANT_ONE_FOOT_LEFT_OUT",
        415: "PUSH_TO_ANTIC",
        420: "TO_WIPE_OUT",
        424: "LEFT_GROUND_TO_TRICK_CYC_1_505",
        425: "LEFT_GROUND_TO_TRICK_CYC_1_510",
        435: "CYC_TO_WIPE_OUT_TWEAK_0_4",
        436: "CYC_TO_WIPE_OUT_TWEAK_0_2",
        438: "PUSH_START_OR_CONTACT_TO_PUSH_END",
        444: "PUSH_CYCLE_TO_PUSH_END",
    },
    "action": {
        4: "NOT_IS_MIRRORED",
        5: "IS_MIRRORED",
        13: "ENTER_NOT_MIRRORED",
        14: "ENTER_MIRRORED",
        20: "HAS_AG_INTENT_TRICK",
        43: "LEFT_OR_STALE_GRAB",
        44: "GRAB_ONE_FOOT_RIGHT",
        45: "GRAB_ONE_FOOT_LEFT",
        46: "RIGHT_OR_MUTE_GRAB",
        74: "IS_LANDING_ON_BOARD_OFFBOARD",
        76: "GRAB_FINGER_FLIP_SHUV",
        78: "STALE_MUTE_FINGER_FLIP_SHUV",
        79: "BOARD_ADJUST_GRAB_ONE_FOOT_RIGHT",
        80: "BOARD_ADJUST_GRAB_ONE_FOOT_LEFT",
        82: "IDLE_TAIL_OR_NOSE_GRAB",
        104: "BOARD_ADJUST_FINGER_FLIP_SHUV",
    },
    "pedestrian_motion": {
        0: "HAS_INTENT_COLLISION",
        3: "WILL_EXPIRE_IN_0_01",
        7: "WILL_EXPIRE_ALWAYS_IN_0_01",
        26: "HAS_INTENT_AVOID",
        27: "HAS_INTENT_FORWARD",
        28: "HAS_INTENT_LEFT",
        29: "HAS_INTENT_RIGHT",
        30: "HAS_INTENT_BACK",
        33: "HAS_INTENT_JUMP",
        35: "BRANCH_WINDOW_ENABLED",
        36: "IN_TURN_BRANCH_WINDOW",
        61: "WILL_EXPIRE_ALWAYS_IN_0_03",
        62: "LAST_STATE_LOCOMOTION_CHASE_RUN",
        64: "WILL_EXPIRE_ALWAYS_IN_0_1",
        81: "LAST_STATE_LOCOMOTION_STRAFE",
    },
    "pedestrian": {
        2: "NOT_IS_PEDESTRIAN_COLLIDING",
        21: "SIMPLE_TIMER_EXPIRED",
        33: "IS_CHASING",
        45: "NOT_ENTITY_OF_INTEREST_IS_IN_ZONE_BOUNDARY_OR_BUFFER",
    },
    "big_black": {},
}


def snake(text):
    text = text.replace(".", "_").replace("-", "NEG_")
    text = re.sub(r"(?<=[a-z0-9])(?=[A-Z])", "_", text)
    text = re.sub(r"(?<=[A-Z])(?=[A-Z][a-z])", "_", text)
    text = re.sub(r"(?<=[A-Za-z])(?=[0-9])", "_", text)
    text = re.sub(r"(?<=[0-9])(?=[A-Za-z])", "_", text)
    text = re.sub(r"[^A-Za-z0-9_]", "_", text)
    return re.sub(r"_+", "_", text).strip("_").upper()


def value_name(value):
    if re.fullmatch(r"-?\d+\.0+", value):
        value = value.split(".")[0]
    return snake(value.replace(".", "_"))


def condition_base(data, number):
    declaration = data["conditions"][number]
    attributes = gq.attrs(data, declaration["attributes"])
    parts = [snake(CLASS_NAMES.get(declaration["name"], declaration["name"]))]
    ordered = [a for a in attributes if a[0] in IDENTITY] + sorted(
        (a for a in attributes if a[0] not in IDENTITY), key=lambda a: KEY_ORDER.get(a[0], 0)
    )
    for key, value in ordered:
        if key == "state":
            parts.append(snake(VALUE_NAMES.get(value, ".".join(value.split(".")[-3:]))))
        elif key in IDENTITY:
            parts.append(snake(VALUE_NAMES.get(value, value)))
        elif key in COMPARISON:
            parts.append(f"{snake(key)}_{value_name(value)}")
        elif value in ("true", "false"):
            parts.append(snake(KEY_NAMES.get(key, key)) if value == "true" else f"NOT_{snake(KEY_NAMES.get(key, key))}")
        else:
            parts.append(f"{snake(KEY_NAMES.get(key, key))}_{value_name(value)}")
    return "_".join(parts)


def state_name(data, number):
    name = snake(data["states"][number]["name"])
    return f"{name}_{number}"


def identifier(name):
    return f"STATE_{name}" if name[:1].isdigit() else name


def ordinal(number):
    """2ND, 3RD, 4TH: a repeat's suffix, never read as part of a decimal value."""
    suffix = "TH" if 10 <= number % 100 <= 20 else {1: "ST", 2: "ND", 3: "RD"}.get(number % 10, "TH")
    return f"{number}{suffix}"


def condition_names(data):
    bases = [condition_base(data, n) for n in range(len(data["conditions"]))]
    seen = Counter()
    names = []
    for base in bases:
        seen[base] += 1
        names.append(identifier(base if seen[base] == 1 else f"{base}_{ordinal(seen[base])}"))
    return names, bases


def users_of(data):
    users = defaultdict(list)
    for number, state in enumerate(data["states"]):
        if state["expression"] != gq.NULL:
            users[state["expression"]].append(("state", number))
    for number, transition in enumerate(data["transitions"]):
        if transition["expression"] != gq.NULL:
            users[transition["expression"]].append(("transition", number))
    return users


def owners_of(data):
    owners = defaultdict(list)
    for number, state in enumerate(data["states"]):
        count, index = state["transitions"]
        for position in range(index, index + count):
            owners[data["indices"][position]].append(number)
    return owners


def user_label(data, owners, kind, number):
    if kind == "state":
        return state_name(data, number)
    transition = data["transitions"][number]
    sources = owners.get(number, [])
    source = state_name(data, sources[0]) if len(sources) == 1 else "ANY"
    return f"{source}_TO_{state_name(data, transition['target'])}"


def common_name(data, owners, users):
    kinds = {kind for kind, _ in users}
    if kinds == {"state"}:
        names = {data["states"][n]["name"] for _, n in users}
        if len(names) == 1:
            return snake(names.pop())
        return None
    if kinds == {"transition"}:
        pairs = set()
        targets = set()
        for _, n in users:
            sources = owners.get(n, [])
            source = data["states"][sources[0]]["name"] if len(sources) == 1 else None
            target = data["states"][data["transitions"][n]["target"]]["name"]
            pairs.add((source, target))
            targets.add(target)
        if len(pairs) == 1:
            source, target = pairs.pop()
            if source is not None:
                return f"{snake(source)}_TO_{snake(target)}"
        if len(targets) == 1:
            return f"TO_{snake(targets.pop())}"
    return None


def expression_names(graph, data):
    users = users_of(data)
    owners = owners_of(data)
    names = {}
    for expression, use in users.items():
        if len(use) == 1:
            names[expression] = identifier(user_label(data, owners, *use[0]))
    shared = {e: u for e, u in users.items() if len(u) > 1}
    common = {e: common_name(data, owners, u) for e, u in shared.items()}
    counts = Counter(name for name in common.values() if name)
    overrides = {k: v for k, v in SHARED[graph].items() if v}
    missing = []
    for expression in shared:
        if expression in overrides:
            names[expression] = overrides[expression]
        elif common[expression] and counts[common[expression]] == 1:
            names[expression] = identifier(common[expression])
        else:
            missing.append(expression)
    duplicates = [n for n, c in Counter(names.values()).items() if c > 1]
    return names, users, owners, missing, duplicates


def describe_user(data, owners, kind, number):
    if kind == "state":
        return f"state `{gq.label(data, number)}`"
    transition = data["transitions"][number]
    sources = owners.get(number, [])
    source = "/".join(f"`{gq.label(data, s)}`" for s in sources) or "?"
    return f"{source} -> `{gq.label(data, transition['target'])}`"


def attribute_text(data, number):
    declaration = data["conditions"][number]
    attributes = gq.attrs(data, declaration["attributes"])
    inner = ", ".join(f"{key} = {value}" for key, value in attributes)
    return f"`{declaration['name']}({inner})`" if inner else f"`{declaration['name']}`"


def state_path(data, number):
    """The state's path below the root; the root's own name for the root."""
    chain = gq.ancestry(data, number)
    return "/".join(data["states"][a]["name"] for a in (chain[1:] or chain))


def doc(text, indent="    "):
    """`///` lines of at most 100 columns; a code span is never split."""
    words = re.findall(r"`[^`]*`[^\s`]*|\S+", text)
    lines, line = [], ""
    for word in words:
        if line and len(indent) + 4 + len(line) + 1 + len(word) > 100:
            lines.append(line)
            line = word
        else:
            line = f"{line} {word}" if line else word
    lines.append(line)
    return [f"{indent}/// {part}" for part in lines]


def render(graph, data):
    conditions, bases = condition_names(data)
    first = {}
    for number, base in enumerate(bases):
        first.setdefault(base, conditions[number])
    expressions, users, owners, missing, duplicates = expression_names(graph, data)
    if missing or duplicates:
        raise SystemExit(f"unnamed shared expressions {sorted(missing)}; duplicate names {duplicates}")
    title = GRAPHS[graph][3]
    out = [
        f"//! The {title} state graph's ids under names from its own data, written by",
        "//! `tools/generate_graph_ids.py`: conditions by class and attributes, expressions by the",
        "//! states and transitions that evaluate them, states by name and index.",
        '#![allow(dead_code, reason = "the complete graph; code names the ids it reads")]',
        "",
        "/// Condition declarations: what each condition index tests.",
        "pub(crate) mod condition {",
    ]
    for number, name in enumerate(conditions):
        text = attribute_text(data, number)
        if first[bases[number]] != name:
            text += f", declared again: the test of `{first[bases[number]]}` under its own index"
        out += doc(text + ".")
        out.append(f"    pub(crate) const {name}: u16 = {number};")
    out += ["}", "", "/// Expressions: which states and transitions evaluate each expression index.", "pub(crate) mod expression {"]
    for expression in sorted(expressions):
        use = users[expression]
        shown = [describe_user(data, owners, kind, n) for kind, n in use[:4]]
        more = f" and {len(use) - 4} more" if len(use) > 4 else ""
        if len(use) == 1:
            out += doc(f"{shown[0][0].upper()}{shown[0][1:]}.")
        else:
            out += doc(f"Shared by {len(use)}: {', '.join(shown)}{more}.")
        out.append(f"    pub(crate) const {expressions[expression]}: u16 = {expression};")
    out += ["}", "", "/// States by name and index, with their path below the root state.", "pub(crate) mod state {"]
    for number in range(len(data["states"])):
        out += doc(f"`{state_path(data, number)}`.")
        out.append(f"    pub(crate) const {identifier(state_name(data, number))}: u16 = {number};")
    out.append("}")
    return "\n".join(out) + "\n"


def main():
    for graph in GRAPHS if sys.argv[1] == "all" else [sys.argv[1]]:
        run(graph)


def run(graph):
    data = load(graph)
    if "--missing" in sys.argv:
        expressions, users, owners, missing, duplicates = expression_names(graph, data)
        for expression in sorted(missing):
            shown = [describe_user(data, owners, kind, n) for kind, n in users[expression][:3]]
            print(f"{expression} x{len(users[expression])}: {', '.join(shown)}")
        print("duplicates:", duplicates)
        return
    text = render(graph, data)
    path = GRAPHS[graph][2]
    if "--check" in sys.argv:
        current = path.read_text(encoding="utf-8") if path.exists() else ""
        if current != text:
            raise SystemExit(f"{path.name} is stale; rerun without --check")
        print(f"{path.name} is current")
        return
    path.write_text(text, encoding="utf-8", newline="\n")
    print(f"wrote {path.name}")


if __name__ == "__main__":
    main()
