# Verifying against TU3

Everything in this port was checked against Skate 3's title update 3 executable (`default.xex`
from the TU3 patch, the "patched xex"; the module names its SHA-256 where a byte-exact read
mattered, e.g. `431b8eba...` in `graph_host/motion_offboard/cadence.rs` and
`grind_world/spline.rs`) and, where behaviour depends on runtime values, against the running game.

## How addresses are written in the code

- `0x82d45c00`, `Update82BA2BC8`, `` `Name`82bb64b8 ``, `ctor82DB3008`, bare `82D4D418`: all are
  addresses, in TU3 inside the Skate 3 module and in Skate 2 elsewhere unless the comment says
  "TU3" / "Skate 3" (`91-address-index.md` labels each one).
- `0x82D33BF8/33DB8/33C70`: shorthand for 0x82D33BF8, 0x82D33DB8, 0x82D33C70.
- `0x8289fd40..fd78`: an instruction range inside one function.
- Offsets like `Processed+2468`, `2476 bit 22`, `PhysOut+0x48 byte 0x130`: fields of TU3 structures
  (ProcessedPhysIn, PhysOut records) as the code names them (`flags_2468`, `flags_2476`, ...: the
  decimal offset is in the Rust field name).
- Vault: 64-bit class and collection keys (`EB844CFDBF928EFD` physics_wipeout,
  `D7EDBD362D7D2152` = "default", `7CE1B6B581709C6F` physics_reckoning, ...). Layout offsets inside a
  class are decimal in field names (`physics_jump` adjust X at 436).

## IDA pitfalls (Xenon PowerPC)

1. **Skate 2 names in the TU3 IDB.** Enums and structure fields imported from Skate 2 are wrong
   at TU3 offsets: `CollisionGroup` (TU3 numbering in `04-shared-skate2-skate3.md`),
   `GlobalAttributeCollection` members (read the TU3 initializer 0x8289D5C8: FindCollection, then
   `stw` into r28+offset), SkateboardBody `mBack`/`mFront` (swapped against raw offsets). Verify by
   initializer, constructor or raw offset, never by the field name.
2. **Adjusted `this`** in multiple-inheritance callbacks: Hex-Rays applies the full class type to
   the adjusted pointer, so every printed field is shifted. Read the `stw/stb` displacements and
   add the sub-object offset.
3. **64-bit values** are split into HIDWORD/LODWORD halves (a −2 printed as
   `0xFFFFFFFE00000000`); check `li`, `cmpdi`, `ldx`/`std`.
4. **`fsel`** operands vanish from the decompile: clamps and min/max built from `fsel` must be read
   from the instructions.
5. **VMX128**: IDA prints the VA operand of VMX128 ops (vmulfp128, vmaddfp128, vmrghw128, vor128,
   vsubfp128, vmsum*fp128) with bits 5 and 6 swapped (a displayed v64–v95 is really v32–v63 and the
   reverse); vD and vB are right. `vmaddfp vD,vA,vB,vC` as IDA prints it computes vA·vC + vB;
   `vmaddfp128 vD,vA,vB` computes vA·vB + vD. `vpermwi128` lane i = (imm >> (6 − 2i)) & 3.
6. **Vtables vs `.pdata`**: a table of function addresses in `.pdata` is not a vtable (the shove
   probe's methods were first misread this way).
7. **Calls through vtables** hide callers: search for loads of the singleton or the vtable, not
   for direct `bl`.

## Measuring the running game

Function-level reading answers most questions; values that depend on runtime state (stick
quantization, steering tilt, truck angles, push timing) were measured in a running TU3 build.

- A rexglue recompilation of Skate 3 TU3 for Windows runs the original code; guest memory sits at
  host 0x1_0000_0000 + guest address (`MZ` at guest 0x82000000), readable with
  ReadProcessMemory.
- Find live objects by scanning the guest heap for a vtable, e.g. PhysicsGround 0x82327244 (one
  live instance while riding):
  - +0x4 board controller (+256 smoothed deck tilt; +12 → SkateboardBody with truck angles
    +7680/+7684 and timers +8364/+8368);
  - +0x10 ProcessedPhysIn (+400 velocity, +2616 |speed|, +2676 turn, +2716 hard turn,
    +2760/+2764 trucks/wheels, +2604 timestep, +192 effective transform);
  - +0x20 steering settings, +0xA50 push scalar, +0xA54 damped turn, +0xA58 ground time.
  - The simulation manager: scan for the timestep 0x3C888889 at +16; solver iterations at sim+176.
- Keyboard input in that build is digital (full stick only). For partial stick values, attach a
  debugger and write the XInput state's thumbstick words after `XamInputGetState` returns
  (`mov word:[r14+8], value`). For a timed script, give that breakpoint a command that runs only
  for controller 0 (`edi==0`, about four calls per tick), bumps a counter and writes all four
  big-endian thumbstick words at once (`mov qword:[r14+8], ...`) from that counter.
- Body spin: the Reckoning (PhysicsGround +0x14) holds BodySpin at +0 (history index +0xA8, input
  +0x78, flick +0x80, speed +0x8C, credit +0x94, credit time +0x9C, air time +0x98), the heading
  at +0x4B0 and the spin speed at +0x624. The Actor (IActorQuery vtable 0x823009D0 at +0x38, player
  id +0x6E8) keeps its ActionGraphInputListener at +0x700, whose current left X is at +0x24 and
  right Y at +0x30. ProcessedPhysIn +0x9CC is the current state, +0xA0C the jump frame count.
- Grind grabs:
  - The skater's Action and Motion graphs are compiled into the executable (StateGraph::Gen:
    `HC_ActionGraph_OnBoard`, vtable 0x822F9C30, and `TemplateHC<HCDelegate_MotionGraph_OnBoard>`,
    vtable 0x822F9C90). For the grind grabs they match the shipped `.stategraph` state by state
    and expression by expression (Into 0x1CD, GrabCycleDirect 0x269). The current state is the
    u16 at controller +8, an index into the delegate's kStates (MotionGraph 0x82292130, 32-byte
    StateDesc: name, interruptable, active, interrupt target, parent, expression, then (count,
    start) sections for children and behaviours, and for transitions through kIndexArray
    0x8228AA18). Actor +0x704 is the ActionGraph and +0x708 the SkaterMotionGraph interface (object
    at −0xA7C); each object's +8 points at a record whose +0x1C is the controller.
  - The crouched-enough height (IsCrouchedEnoughForBlendToGrabCycle 0x82BBDE40, below 0.6):
    PhysicalPlayerHiLOD (vtable 0x82328688) +0x708 → PhysOut → +0x38 → +0x48. ProcessedPhysIn is
    HiLOD +0x70C (state +0x9CC, deck position +0x70).
  - The skater's animation TransitionStore sits at (actor +0x70C) − 0x320: transitions at +0xC,
    blends at +0x10C, maxima at +0x0 (mMaxNumTrees, 4) and +0x4C (3).
  - Placing the skater: the SessionMarker (vtable 0x82315940, written by the constructor
    0x8289B1A0) keeps its matrix at +0x1500 (position +0x1530), foot forward +0x1544, the set flag
    +0x1548 and on-board +0x154C. Write it, then hold LB + D-pad up (longer for a farther marker,
    at most 1 s). Skate School has no marker object and the marker buttons there make the game
    call through a null marker, so travel to a district first (pause menu, Challenge Map,
    Locations).
  - Timed input without pausing the game: give the breakpoint after XamInputGetState (controller
    0, once per tick) a command that ORs buttons and triggers into the state, and optionally
    replaces the sticks, from a small block of memory a script writes, and stores a call counter
    there for the script to follow.
  - A darkslide both games produce: on board at (332.88, 3.0, 354.25) facing yaw −1.5558
    (University, east of the 0.63 m box whose south edge runs from (328.30, 3.57, 354.96) to
    (320.87, 3.57, 353.09)); A held 6 ticks at ticks 10 and 80; the right stick down over ticks
    135–150 and up-right over 150–153; RB over 165–225. That is a kickflip, DarkCatchHalf,
    InAirDark and a darkslide on that edge from about tick 195 for 40–45 ticks.
- Measure in a quiet place (Skate School, the university plaza), with no push active: TU3 halves
  steering while a push event runs (push scalar 0.5 until ~0.75–1 s after the last push), and
  this confounded the first turn comparison.

## Comparing with the port

- Turn on the matching trace (`06-porting-guide.md`, "Debugging switches"): `SKATE2_TRACE_SKATE3_STEER`
  prints the same fields as the live TU3 read above.
- Script the input (`SKATE2_DIAGNOSTIC_HOLD`, `SKATE2_DIAGNOSTIC_LEFT_STICK`, ...) so both runs get
  the same sticks.
- Compare per tick: TU3 runs at 1/60 s, and state changes land on the same tick when the order in
  `02-tick-pipeline.md` is kept.
- Body spin, measured with the same scripted ollie chain in both (`SKATE2_TRACE_SKATE3_SPIN`,
  `SKATE2_DIAGNOSTIC_SPIN_OLLIE`): take-off comes 13 ticks after the right stick goes up and 1 tick
  after the jump frame, into KnownAir. A left stick snapped full 6 ticks before that flick is 19
  ticks old at take-off and earns a flick credit of 0.38 (Easy 0.35); in 43 air ticks the body
  turns 176.5 degrees on Normal (Easy: 45 ticks, 179.2). The same chain with the re-pop 9 ticks
  after touchdown takes off 37 ticks after each landing. All of these match tick for tick. Snapped
  at the flick instead, the credit is 1.0 and the body turns about 272-279 degrees.
- Board adjust (right-stick tweak), the same chain with `SKATE2_DIAGNOSTIC_RIGHT_STICK` windows:
  - the deck leads the heading through BLEND_ADJUST_* and touchdown turns the heading by that lead;
    a full sideways stick leads by 75.8 degrees in one stance and 86.5 in the other (Left/Switch),
    so a 282-degree body spin lands at 357;
  - pushing the stick to 0.6 up 10 ticks before touchdown (a nose manual) keeps the Right state but
    swings the adjust angle, and the lead falls to 3.8-9.8 degrees (283-286 in all);
  - releasing it 10 ticks before touchdown keeps 38.4-40.3 degrees (314.5-322.5 in all);
  - a held diagonal (x 0.6, y 0.5) keeps 33.8, full up 3.5 with the deck pitched -18;
  - all of these match TU3 to 0.1 degree; ticks where the TU3 heading is already updated but the
    deck is not (a sample taken mid-tick) show a lead about one tick of spin too large.
- Touchdown (`SKATE2_TRACE_SKATE3_LAND`): per tick the deck body's solved velocities and position,
  ProcessedPhysIn +400, +720, +464, +2652, +2612, the flags and wheel count, the balance attribute
  and the manual controller's Skateboard fields. In TU3 the Skateboard is PhysicsGround +0x4
  (+0x108 filtered error, +0x10C target, +0x110 measured angle, +0x114 correction, +0x118 elapsed)
  and the deck body is SkateboardBody (+0xC) -> assembly (+0x18) -> parts (+0x8) -> +0x28C
  (+0x20 linear velocity, +0x30 angular velocity, +0x40 basis, +0x70 position). A read of the
  TU3 body can land before or after that tick's solve; ProcessedPhysIn holds the last solve's
  output on both sides, so compare those fields tick for tick.
