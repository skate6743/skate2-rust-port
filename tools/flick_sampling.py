"""One stick flick, read at an even sixtieth of a second or at the uneven moments measured at an
uncapped 70 frames a second: prints the SKATE2_DIAGNOSTIC_RIGHT_STICK windows that replay each
reading, one ollie every GAP frames, and the per-tick values of each.

usage: flick_sampling.py RISE_MS [HOLD_MS]
"""
import sys

RISE = float(sys.argv[1]) if len(sys.argv) > 1 else 70.0
HOLD = float(sys.argv[2]) if len(sys.argv) > 2 else 90.0
TICK = 1000.0 / 60.0
GAP = 360
CROUCH = 40
FULL = 32767


def stick(t):
    """The right stick's y at t ms after the flick begins: pulled down, up in RISE, held, let go."""
    if t < 0:
        return -FULL
    if t < RISE:
        return round(-FULL + 2 * FULL * t / RISE)
    if t < RISE + HOLD:
        return FULL
    return 0


def instants(first, spacings, count=16):
    out, t = [], first
    for k in range(count):
        out.append(t)
        t += spacings[k % len(spacings)]
    return out


# The readings: the moment of the first read after the flick begins, then the reads' spacing.
PATTERNS = [
    ("even, first read 2 ms in", 2.0, [TICK]),
    ("even, first read 8 ms in", 8.0, [TICK]),
    ("even, first read 14 ms in", 14.0, [TICK]),
    ("uneven 11.5/21.8, first read 2 ms in", 2.0, [11.5, 21.8]),
    ("uneven 21.8/11.5, first read 2 ms in", 2.0, [21.8, 11.5]),
    ("uneven 11.5/21.8, first read 8 ms in", 8.0, [11.5, 21.8]),
    ("uneven 21.8/11.5, first read 8 ms in", 8.0, [21.8, 11.5]),
    ("uneven 25/11/14, first read 5 ms in", 5.0, [25.0, 11.0, 14.0]),
]

windows = []
for index, (name, first, spacings) in enumerate(PATTERNS):
    base = 120 + index * GAP
    windows.append((base, base + CROUCH, -FULL))
    values = [stick(t) for t in instants(first, spacings)]
    for k, value in enumerate(values):
        windows.append((base + CROUCH + k, base + CROUCH + k + 1, value))
    print("# %d %-40s %s" % (index, name, " ".join(str(v) for v in values[:8])), file=sys.stderr)

period = 120 + len(PATTERNS) * GAP
first = windows[0]
spec = "%d:%d:%d:%d:0:%d" % (int(sys.argv[3]) if len(sys.argv) > 3 else 1500, period, first[0], first[1], first[2])
spec += "".join(";%d:%d:0:%d" % w for w in windows[1:])
print(spec)
