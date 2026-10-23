# Continuous Action input and original debug-event producer

2026-09-11 implementation packet. Confidence **high** for raw control/data flow;
host numeric estimates, exceptional FP payloads and source allocator failures
remain explicit boundaries. No deterministic Xenon tick trace or playable loop
is claimed. Root owns independent review, integration and Cargo validation.

## Ownership and caller

`Sk8::Skater::ActionGraphInputListener::Fill(float,Sk8::Intents&)`,
`?Fill@ActionGraphInputListener@Skater@Sk8@@QAAXMAAVIntents@3@@Z`,
825fb4c8..825fd664, belongs to the Actor input/graph path. Decompiler extra map
arguments are false: raw retains the sole supplied Action intents argument r5.
825fb54c..b56c resolves the retained player's Input; Actor PhysOut is called at
b574, state sampled at b594, then same inline Derived owner updated at b5b0.
Physics states501/104 select pushing/skitching, respectively.

Actual `Actor::GenerateActionGraphIntents`,825fda28, fetches its Action facade
at+1f4 virtual+10; frees map nodes/resets count; chooses AI+214 or listener+1f0
unless force-brake04000000; clears events825fdabc after producer; handles
pending wipeout80000000 using current PhysOut and clears that bit; finally
ForceBrake overwrites via operator[]. These outer gates are root-owned.

The new `try_fill_selected` operates on the same retained listener/player and
supplied map: Derived, entire continuous body, ordered InputEvent/pattern suffix,
late Juice. It never clears the map/events or synthesizes the Actor scheduler.

## Derived review correction: decode BO/BI, not pseudo-mnemonics

Constructor825f9f28..9ff8 initializes raw modeled fields to zero, edge ages to
7effffff and held timers to zero. Update825f1c30..20f8 copies current to previous,
updates raw, exact-full triggers, stick edges, resamples faces A/B/X/Y, then
fresh LX/LY/RX/RY. Length squares preserve x*x then fused y*y+x*x. Both numeric
estimates execute before the late global check and one threshold fetch.

Initial mnemonic-only review was withdrawn. At20a4 word40990010 and20c4
word40990014, BO4/BI25 means **not GT**, including unordered. Therefore only
ordered greater increments magnitude ages; NaN length or threshold resets to
positive zero. Implementation/tests now reflect this. An initial suspicion
about shared PointGraph was likewise withdrawn:824e2d10 word40990030 is the
same not-GT predicate, so existing unordered-width upper-sample behavior is
correct. **PointGraph was not edited.**

All conditional branches of the continuous prefix were separately enumerated
with raw BO/BI. This also matters for sign selection, wipeout age comparisons,
hard-turn limits, manual amount and new-toggle allowance.

## Numeric and insertion ordering

825fb5b4..b92c evaluates right angle/length before left. Zero sticks skip their
angle/length calls. Angle uses -Y reciprocal estimate, one refinement, original
XMVectorATan82581658, X-sign-bit quadrant PI/HALF_PI selections; no atan2.
Right squared is rx.mul_add(rx,ry*ry); left squared is ly.mul_add(ly,lx*lx).
Late unconditional left length825fc5c0 uses lx.mul_add(lx,ly*ly), even at zero.
Object movement recomputes left angle late and performs fractional-turn floor
wrapping, subtracting1 only when fraction>0.5, then multiplies by2PI.

All response reads and numeric operations finish before first intent insertion
825fcb24. Steer combination and trigger-average-plus-RY use source fused
operations. Curve argument computes abs(angle)*3ea2f983 then fused1-product*2.
Clamps use source fsel order, including signed zero and unordered selection.
Native crouch includes multiplication by literal zero; it is preserved.

Intent spellings were independently read from dynamic initializers82f81700..
82f81e78, not inferred by stripping symbol suffixes. The ordered native stream
is written explicitly in action_graph_input_fill.rs, including duplicate
Pushing and paired Tweak writes. Seven offboard/object controls are unconditional.
Neutral steering produces negative zero. Map first-insertion semantics remain
the actual Actor map's responsibility.

## Runtime Vault keys in source read order

Same global inputlistener_default+194, class5149ae9f16190257,
collectiond7edbd362d7d2152. Every read uses original runtime node search and
inheritance; missing pointer only uses zero DefaultDataArea. No tuning copied
from corpus is substituted for runtime reads.

| Call | Full key | Payload / use |
|---|---|---|
|825fc3f4|cd28e026c7a45f98|80 bytes,8 points,left steer|
|825fc464|ebca2e67f2c3ac51|80 bytes,8 points,right steer|
|825fc4e4|cac40ccb7d0e2c8c|144 bytes,16 points,left locomotion steer|
|825fc554|8bab9bef5c8daa02|144 bytes,16 points,right locomotion steer|
|825fc66c|2d55eb3b4b38e247|80 bytes,8 points,magnitude|
|825fc6dc|3099c9cc4360a049|4 bytes,magnitude-held threshold|
|825fc754|9b941c683f390916|144 bytes,16 points,locomotion magnitude|
|825fc9c4|9adfc2e222938c1e|144 bytes,16 points,object rotation|
|825fca3c|1a1a7ac37a72df87|80 bytes,8 points,object X|
|825fcab4|05ba8b52c23b3481|80 bytes,8 points,object Z|

Last three reads occur only for nonzero left stick. Curves use x+16,y+48/80
and fixed8/16 sample counts, regardless of payload metadata. Shared
PointGraphEval824e2c40 preserves endpoint/search/interpolation behavior.

Runtime cvar refs: NewPushAllowance830484b0, UseTriggersForExplicitWipeout8304811c,
TimeToConsiderWipeout83048484, UseReleaseForWipeout83048460,
NewToggleOffboardAllowance830484dc, UseSprintButtonInput830480f8.
Explicit provider captures each target at its source point; repeated A/X
allowance and jump/sprint value reads retain the same captured target.
Float values use target+D8; bool values target+CD. No production constant cvar
provider is supplied by this component.

## Late Juice and retained InputEvent

Fill825fd5e4 calls IsTrickActive826e39e8 (mutex-protected byte read). If active,
GetTrickName826e3a38 copies to64-byte destination and clears active under its
mutex **before** returning. Then Insert Trick,name,TrickHeight,GestureSpeed,
all1.0. A failed insertion does not restore consumed Juice state. The explicit
collaborator has no fake disconnected/always-false production implementation.

GenerateGameInputEvents826dfdc0 evaluates map GetState59,60,61,62,63, in order;
each true result queues cMsgInputEvent kind7381d2e7,size28,id0, same Input
occurrence and player index. Native caller GameInputManager::Update826e2578
calls UpdateRecognizer826e2614, this producer826e2620, then shared queue flush
826e2638 after the player loop. This producer is debug controls, not a
replacement gesture-to-jump path.

CompileMap826de030..78 names exactly DebugPrev,DebugNext,DebugPause,
DebugReplaceSkateboard,DebugResetWorld. Original data/config/input.cfg from
miscboot.bh hash5efcbd5b, miscboot.big offset5440, stored3017, unpacked10121
was independently decoded read-only: definitions respectively use
DPadL.pressed,DPadR.pressed,Start.pressed,DPadU.pressed,Back.pressed.
Explicit parse_with_input_events requires those names; original gameplay-only
parse retains unsupported-map refusal for59..63. New pressed expression uses
same retained InputPad pressed flags, source GetButton82988fe8; value/held are
not substitutes. Missing authored definitions are errors, not zero controls.

ReceiveInputEventMsg825f9408 compares msg player index then appends only event
identity to the same ordered event sequence. Candidate/held latches are untouched.
Fill825fd418..d53c maps ids through table82fe5100 and inserts their exact names.
The typed enum admits only the five real identities, preserving pattern/input
event interleaving. Root corpus now filters pattern variants explicitly.

## Focused validation requested

New tests cover neutral signed zeros/unconditional controls, duplicate push
ordering and insertion refusal, cvar capture/reread, numeric call ordering,
late invalid-attribute refusal before writes, actual runtime curve layout and
inheritance/full64 key discrimination, unordered wipeout ages, trigger edges,
same-listener late Juice consumption, mixed pattern/debug queue ordering,
pressed-vs-value reads, missing debug definitions and retained queue prefixes.
Derived tests distinguish NaN threshold and one NaN stick magnitude.
Root owns test execution/results and real-data producer-to-map integration.


## Concrete Juice owner and current cvar targets (2026-09-11)

`juice_trick_input.rs` owns one clone-shared mutex/name/latch occurrence. Original
name8304e540 is 64 zero image bytes; active8304e606 is zero. Static initializer
82f5cee0 constructs the EA mutex8304e6b8 with NULL parameters/true and registers
destructor82faffc8. DoTrick826e3f40 locks, calls GetString, ignores its boolean,
copies through snprintf64 `%s`, sets active, unlocks. GetTrickName826e3a38 copies
up to63 bytes plus NUL and clears active under the same lock. The host adapter
consumes the latch before reporting invalid UTF8, an explicit unsupported
malformed-name boundary; no lossy replacement. RPC transport/server admission
is separate, not fabricated. Four focused tests exercise shared owner mutation,
truncation/terminator/trailing output, ignored status and missing reader pointer,
and latch consumption on malformed UTF8.

`input_cvar_runtime.rs` retains six actual selected static reference occurrences,
not a full global cVarMan. References begin with NULL targets. Initializer table
82fd1058..1078 corroborates selected order: sprint, triggers, release, wipeout age,
push age, toggle age. Constructors push to reference-list head, so manager Init
82a8ac38 links in reverse. cVarBoolRef::Link82a89ca0 / cVarFloatRef::Link82a8a010
reuse exact case-sensitive section+label targets, preserving existing current
values; otherwise labels initialize defaults. AddSortedSection82a882c0 orders
section names, AddSortedLabel82a88388 orders titles, inserting equal names after.
All six callbacks are NULL (raw float ctor r10=0; decompiler extra arguments
are bogus). Selected current/default values and source titles:

- Animation/kUseSprintButtonInput, true, ?Use Sprint Button Input?, init82f56b88.
- Controller/kUseTriggersForExplicitWipeout, true, ?Use triggers for explicit wipeout?, init82f57230.
- Controller/kUseReleaseForWipeout, false, ?Also use release for wipeout?, init82f57280.
- Controller/kTimeToConsiderWipeout, 3e4ccccd, ?Time to consider wipeout?, init82f572d0.
- Controller/kNewPushAllowance, 3e99999a, ?New Push Allowance Time?, init82f57330.
- Controller/kNewToggleOffboardAllowance, 3cf5c28f, ?New Toggle Offboard Allowance Time?, init82f57390.

Float source UI limits are min0 and max(.5,1,1) respectively. This bounded current
value owner does not project UI/editor metadata. It never clamps serializer
writes: ReadLabel82a8a5a8 stores float current+D8 only on successful GetFloat;
bool current+CD is passed directly to GetBool. Defaults remain unchanged.
A Fill provider captures target identity at each explicit source capture, then
rereads current value for every access. Relinking a reference to another manager
does not retroactively change an already captured target. Tests mutate through
actual deserializer between reads and distinguish old/new target ownership.

Boot InitBootDebug8272e390 constructs cVarMan then cVarSerializer and actually
calls Deserialize8272e490 for `data\startup.ini`. The original loose startup
file has Config labels for debugging and installation but no six selected input
overrides. Their absence does not authorize ignoring runtime configuration:
try_deserialize_startup accepts actual admitted bytes (None represents real load
failure), parses them before applying existing selected labels in source order.
ReadSection82a8ab28 recursively visits sections then labels; all label lookups
use literal ?Config?, regardless of cvar section name.

`input_cvar_ini.rs` reconstructs cIniFile::Read82a88de0, StringTrim82a88b80,
FindSection82a88d08, FindKey82a88d78 and ReadLine82a88b08. Raw ReadLine branches
82a88b6c..70 return NULL on an LF-only empty line, terminating the entire read;
a CRLF blank line contains CR, trims empty and continues. Trim removes bytes
09..0d,20,a0 only at whole-line edges; spaces around '=' remain in keys/values.
Names are case-sensitive; first duplicate key wins. Semicolon at first trimmed
byte comments a line. Embedded NUL terminates the line string. Field lengths
>=256 are an explicit checked unsupported termination boundary, not silent
truncation. This selected reader exposes Config keys only; unrelated section
storage is not a global INI service claim.

GetBool82a89620 calls custom ConvertAToI82bc3ce0: scan to first digit or minus,
consume initial optional minus then decimal digits with 32-bit wrap. Thus
?true? ->0 and ?prefix+12tail? ->12. GetFloat82a89718 calls custom
ConvertAToF82bc3e68, NOT host atof/parse. Raw82bc3f34 is fmadds for each digit;
fractional divisor grows by10 after any dot, minus may appear anywhere in the
mantissa, and raw82bc3f78 is fdivs. Exponent accepts digits/minus but no plus,
uses wrapping integer accumulation and grouped multiplications by10000 or
38d1b717, followed by up to3 factors10 or3dcccccd. Repeated dots continue the
fractional flag. Extremely large exponent multiplication loops short-circuit
only once the IEEE value is bitwise stationary, preserving observable result.
No claim about Xenon NaN payload or nondefault floating-point modes is made.

Confidence high for source-owned state/order and ordinary numeric behavior;
no deterministic Xenon tick trace yet. Focused tests cover null linkage, exact
initial bits, current target mutation/rebind, source INI lexical quirks, custom
converters and sorted label application. Root owns compilation and original
corpus integration; these additions were not Cargo-tested by this agent.


### Seventh selected reference: actual board hand-drive gate

Added `kSKC_UseHandDrives`, not a fabricated kUseHandDrives label. Reference
830db1e4 initializer82f98540 (table82fd5dc8) registers section
Physics_Skateboard_Controller, title ?Use Hand Drives?, defaulttrue, callbackNULL
(raw r7=1/r8=0). Its initializer runs after the six input refs, so it is first
among this selected subset during head-list linkage. Nested section lookup
splits underscores; for the selected single Physics leaf the flattened retained
section path gives the same section/label order as native recursive traversal.
Fill's six existing indices remain unchanged.

`InputCvarReferences` implements actual `SkateboardControllerCvars` using opaque
captured target ownership. Caller sites82dc74a8/82dc7520/82dc763c/82dcc390/
82dcc428/82dcc63c dereference this reference target then read byte+CD. Added a
focused test for initialtrue, actual deserialized false, reference rebind with
old target retention, and later old-manager mutation visibility. The runtime
manager now owns seven selected labels; earlier six-label descriptions refer
to the continuous Fill subset. No global cvar registry completeness claim.


### Recognizer retained targets recovered for next integration

Original full-Generate corpus reviewed after root replaced late suffix/manual
clear: queued native gesture now feeds actual same Actor map clear, continuous
Fill, ordered events, Juice seam and event clear. Controlled samples/physics
remain explicit; graph execution and outer scheduling are not claimed.

Additional source cvars all belong to Input and have NULL callback:

| Ref | Initializer | Label/title | Default bits |
|---|---|---|---|
|8304e13c|82f5c6b0|kDisplayLeftStickInput / Display Left Stick Input|false|
|8304e178|82f5c700|kDisplayRightStickInput / Display Right Stick Input|false|
|8304e19c|82f5c750|kDisplayLeftStickGestures / Display Left Stick Gesture|false|
|8304e1d0|82f5c7a0|kDisplayRightStickGestures / Display Right Stick Gesture|false|
|8304e240|82f5c7f0|kInputHistoryDeadZone / Dead Zone|3dcccccd,3dcccccd,0,0|
|8304e2a0|82f5c958|kFastestGestureRatio / Fastest Ratio|3fe00000|
|8304e2cc|82f5c9b8|kSlowestGestureRatio / Slowest Ratio|408ccccd|
|8304e350|82f5cad8|kMinError / Minimum Error|3e19999a|
|8304e37c|82f5cb38|kMaxError / Maximum Error|3e19999a|

Initializer table82fd150c..153c confirms order. Raw float initializers put0 in
r10 callback; limits are fastest0..3, slowest0..10, minimum38d1b717..1,
maximum38d1b717..2. Deadzone source initializes all4 lanes, min[0,0,0,0],
max[1,1,0,0], callback0; reference ctor82a8b028 leaves targetNULL.
Vector2 Link82a8a1d0 reuses exact section+label like bool/float. New target
CreateVector2Ref82a887d0 alloc110 bytes, default+D0,min+E0,max+F0,current+100.
Input UpdateInput826e18d8 reads current+100 once before filtering both axes.

GetVector282a89790 uses GetFirstTokenInString82bc45e8 / GetNextTokenInString
82bc46d0 with comma delimiter; consecutive commas skip empty tokens. It requires
at least2 nonempty tokens, converts first2 only, ignores later tokens. Crucially
ConvertAToF receives the original string tail without a token-length bound,
so ?word,2? gives[2,2]. ReadLabel82a8a7d8..838 writes target only after GetVector2
success, preserving previous target on one-token input and preserving ZW lanes
through SetX/SetY. This needs discriminator tests, not standard split/parse.

Successful match UpdateInput lazily copies both gesture cVarBoolRef objects
once (static flag set first), including NULL/nonnull target pointers and other
reference metadata. Copies are shared across players; later original ref Link
does not rebind these copies. Ordinary input-display reads instead capture the
live left/right reference each call. No fake local-player/overlay provider is
implied when actual source display defaults are false; enabling display requires
those unreconstructed debug owners and must surface the checked boundary.

CommandManager::DoCommand825e56e0 syncLevel is NOT a cvar. It reads actual
CommonBackEnd::sGameInfo.syncLevel and enables CRC at signed>=2 only when
addToCRC. A retained cvar extension cannot replace this backend ownership;
diagnostic sync selection must remain explicitly controlled until the actual
backend owner is mounted.

Animation collaborator additionally requests kUseVBR ref8305904c, init82f73178,
initializer-table82fd2a24: Animation / kUseVBR / Use VBR Codec / true / NULL.
This initializer is later than recognizer refs and earlier than hand-drive ref.
Its source consumer samples twice across loading, requiring fresh live-reference
reads each time rather than holding one copied bool.


The recognizer/VBR additions are implemented in the bounded 17-reference owner
and registered child `input_recognizer_cvars.rs`. API: refs.recognizer_provider()
implements InputPatternCvars and exposes the cvar-only recognizer operations;
local-player/backend/overlay services stay with actual caller collaborators.
refs.try_use_vbr() freshly reads the live reference per call. Existing Fill and
hand-drive indices are preserved; REGISTRATION_ORDER explicitly follows static
initializer table order rather than API array indices. Three adapter tests
exercise exact float defaults, live vector/float mutation, capture identity,
shared once-copied gesture targets and NULL copies. Two added runtime tests
exercise source vector tokenizer/string-tail behavior, failed partial override,
ZW preservation and VBR live-reference rereads. Root owns further Cargo gates.


### Ground manual debug cvar: eighteenth selected reference

kShowVelRemoval ref830d7584, initializer82f8efd8, table82fd5714 registers
Physics_Manual / kShowVelRemoval / Show Vel Removal / false / NULL.
Raw82f8effc sets callback r8=0 and82f8f000 sets default r7=0. This initializer
follows VBR and precedes hand-drive; explicit REGISTRATION_ORDER preserves it.
refs.try_show_vel_removal() rereads the live reference target and current byte
for each Ground collaborator call. Added default/unlinked/current override/rebind
test. Actual false comes from linked native default; enabling via startup config
is observed rather than masked. Ground owns downstream enabled debug behavior.

Strict Clippy follow-up fixes local style without behavior changes: source-named
cvar adapters retain narrowly explained unused-self expectations for paired
capture/read service methods. Only non-test builds of retained cvar/Juice module
roots expect dead-code while production boot/RPC ownership is not mounted;
actual corpus/tests remain linted. Root remains sole Cargo runner.

### Later actual boot and StateChanger cvar consumers

The same retained cvar owner now includes `kUseJobsForSkaterAnimUpdate`
(init82f56ae8/ref830480b0, Animation, title Use Jobs for Skater Anim Update,
defaulttrue, NULL update). The actual reader rereads the reference and target;
it preserves missing-until-Link, runtime config changes and rebinding. This
controls the source animation attribute-pool choice, not omission of the
unconditional BlendSpace entrypoint construction.

`kbUseGeneratedStateGraph` (init82f56a48/ref83048068, State, Use Generated
State Graph, true) and `kbTestDeterminism` (init82f56a98/ref8304808c, State,
Use Ghost To Test Determinism, false) are actual NULL-callback references used
by InitPlayer826000b0. These precede Jobs in source static registration order.
Runtime flag adapters reread targets and are tested against config changes
and complete reference rebinding. Generated gEnabled image bytes are zero;
the actual InitPlayer producer must enable the selected delegates.

The integer reference `kFallThroughWorldResetFrames` is independently checked
at init82f9be60/ref830dd6a0/table82fd604c: section
Physics_States_StateChanger, title Fall Through World Reset Frames, default400,
minimum0, maximum1000, callbackNULL. Int Link82a89e40 reuses same section+label
without reinitializing the target. CreateIntRef82a88520 raw stores default+cc
at858c, minimum+d0 at8590, maximum+d4 at8598, current+d8 at85a0. ReadLabel's
integer branch82a8a654..674 calls GetInt82a896a0 then writes current+d8 only
on success; no clamp or update callback. GetInt uses the existing exact
ConvertAToI82bc3ce0 converter, including malformed-text zero and wrapping.
The host target retains range metadata and updates only its current integer.
Tests distinguish -7 and5000 from any0..1000 clamp, preserve default400 and
range metadata, check malformed0 and rebinding back to400. The concrete
StateChangerCvars implementation on InputCvarReferences reads the same target.

The same StateChanger adapter exposes `kDisableSkitching`, independently raw
checked init82f9bfd0/ref830dc914/table82fd605c: section
Physics_States_StateChanger, label/title kDisableSkitching, defaultfalse,
callbackNULL (raw bff4/bff8). Its bool is sampled only when the actual
StateChanger branch requests it. No false callback substitutes for that read;
tests cover unlinked error, sourcefalse, configtrue and rebindfalse.

There are now23 selected references, with the original six Fill indices
unchanged. The manager still parses actual startup.ini bytes and retains
source title ordering; the title-order test was extended for these real
registrations. These changes await root's next combined validation gate.

## Retained Actor runtime integration, September 12

`actor_input_runtime.rs` retains the startup-created Fill attribute instance,
live cvar provider and shared Juice owner across the two actual Action/Motion
updates in the original selected corpus. Physical state and wiping-out status
come from that Actor's cached physical allocation, at the branches that consume
them. This does not add OS polling or claim a complete input/frame scheduler.

Independent review against825fda28 caught an early missing-owner error before
the native intent-map clear. The wrapper now captures the checked owner result
without returning its error until the actual query. This also avoids recursively
locking the Actor prefix from inside Fill. ForceBrake without pending wipeout
requires no physical read; missing-owner normal/pending branches retain the
source prefix. The regression covers all three cases. Actual retained AI input
controller presence is consulted once the auxiliary allocation exists; the
constructor-null field is used only before that allocation.

Workspace11 passed this regression and original11 passed the selected Actor
input/animation/publication path. Those overall checkpoints failed elsewhere
(cvar-order expectation and duplicate query-world test setup respectively), so
they are not a full accepted workspace snapshot. Numerical parity continues to
require reference tick traces.

## Title Update 2 off-board stick

1.02 replaces the off-board steer and magnitude curves inside the same Fill
(1.02 `Fill` 824e54e8, block 824e6314..68f4). While the left stick is held, its
X and Y are the stick; the side is `-x` (824e64c4), scaled by 1.75 and squared
with its sign until a turn latches. A latched turn keeps the stick's world
direction (listener +0x80, flag +0x91) until the skater faces it. With B held in
state 500, `PickOffBoardWalkDirection` (824e53b8) picks one of eight `OB_Walk*`
intents from `-x, -y`. A positive `OB_Steer` selects `TurningRight` for an
unmirrored skater and the off-board update turns by `-OB_Turn` at 500 deg/s.
1.00 steered by `+sign(x)` through the Vault curves (824e4dd8).

Off the board the Fill does not read the pad's stick. 1.02 also adds
`sub_826F7668` (826f7668..78b4), which `PlayerUI::Update` (8232f550, grown from
226 to 261 instructions) calls on the copied pad line at 8232f82c, before the
focus test and the `PlayerInputData` pack. It runs while the local skater
(`GetLocalActor`: the ghost, else `FindByIndex`) is off its board
(`PhysOut_State::mIsOffboard`), not holding an object
(`PhysOut_OffBoard::mIsGrabbingObject`) and not in state 505. It takes
`(LStickL - LStickR, 0, LStickU - LStickD)` through the first three rows of
`CameraPresentation::GetMatrix(true)`, clears the height, applies
`Math::NormalizeSafe` with a zero default, and takes the result into the frame of
`PhysOut_Skeleton::mEffectiveSkeletonRootMatrix` through its adjugate inverse,
with the arithmetic of the latch at 824e63d0. The local X and Z are split back
into buttons 16..19, so `GP_LStickX` and `GP_LStickY` become a unit vector in the
skater's frame, whose X is up x forward.

With the camera behind the skater, a push right therefore reaches the Fill as
X = -1 and turns the skater right. A held direction stays where the camera sees
it while the skater turns, so a stick held behind turns the skater round until
it is past 0.7 forward, where `OBL_SteerMagnitude` starts the run. A retail 1.02
trace shows the listener's stick sweeping from (0.24, -0.97) to (0.05, 0.99) at
the turn rate during one held push, then physics state 504.

The port applies the same transform in `try_player_sample`
(`input_selected_player_sample.rs`, `OffBoardStickFrames`). The session supplies
the shown camera, an online game's view or else the game camera, and the
skater's effective root matrix. `GetMatrix(true)` would prefer a challenge camera
or an ICE camera while one is active; the port does not show either while the
simulation runs, so the stick follows the shown camera there too. AI skaters
insert these intents themselves and are unaffected.

1.02 also changes the run. `RunAround/LocoNew` attaches `ob_Turn` to `OBL_Steer`,
and `CreateLocomotionTrajectory` (1.02 82a9a990, 1.00 82a96e28) turns each frame
by the `PointGraph` of the bounded turn (key e8c68cdbc3a35230) times pi/180, the
sign and the step time. 1.00 multiplies that by 1.5. 1.02 multiplies it by 80, by
0.75 when the desired speed is above 0.85, and stores `(target - turn) * 0.04` on
the turn it cleared (82a9b224..b374): 3.2 times the step at running speed, so a
held turn runs a loop about twice as tight. Its sign is the raw `ble`, so an
unordered turn takes -1. `UpdateLocomotionTrajectory` (1.02 82a98e60) applies the
stop rate once the desired speed is at or below 0 (82a9a0ec), where 1.00 needed
it below -0.1, so a released stick ends the run at once instead of easing off by
0.15 per frame. `Skeleton::ProcessAnimAttributes` (1.02 82993b58) negates
`OB_JumpX` (8299525c); `StandingJump` attaches it to `OB_ObjectMvX`, which follows
the stick in the skater's frame. The port carries the three changes in
`actor_animation_locomotion_create.rs`, `actor_animation_locomotion_update.rs`
and `skeleton_processed_anim_attributes.rs`. The other 1.02 physics and camera
changes concern grinds and stay out of the off-board port.
