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
  (`mov word:[r14+8], value`).
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
