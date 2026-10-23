# Named ids

The state-graph evaluators and the physics code name the native ids they read
instead of writing the numbers. Every name comes from the game's own data or
enums, so a name says what the native id is, and the number is one click away.

## State graph ids

Each state graph has an ids module generated from its declaration data by
`tools/generate_graph_ids.py`. Paths are under `crates/skate2-core/src`.

| Graph | Declarations | Ids module | Code that names its ids |
|---|---|---|---|
| Motion | `animation/graphs/motion/generated_motion_load_data.rs` | `animation/graphs/motion/generated_motion_ids.rs` | Pre `generated_motion_runtime_selection.rs`, Sustain `generated_motion_runtime_sustain.rs`, `callbacks/actor_motion_runtime_grounded_selection.rs` |
| Action | `animation/graphs/action/generated_action_load_data.rs` | `animation/graphs/action/generated_action_ids.rs` | `generated_action_runtime_selection.rs`, `generated_action_runtime_ground_selection.rs`, `generated_action_runtime_trick_selection.rs`; states in `generated_action_runtime.rs` and the `*_dispatch.rs` slot tables |
| Pedestrian Motion | `animation/graphs/pedestrian_motion_data.rs` | `animation/graphs/pedestrian_motion_ids.rs` | `pedestrian_motion_expressions.rs` (`precondition`) |
| `HCDelegate_Pedestrian` | `living_world/pedestrians/pedestrian_graph_data.rs` | `living_world/pedestrians/pedestrian_graph_ids.rs` | `pedestrian_graph_expressions.rs` (`evaluate`) |
| `HCDelegate_Big_Black` | `living_world/pedestrians/pedestrian_big_black_graph_data.rs` | `living_world/pedestrians/pedestrian_big_black_graph_ids.rs` | `pedestrian_graph_big_black.rs` |

Each ids module has three parts, and every constant has a doc comment:

- `condition`: every condition declaration, named by its class and attributes.
  `HasIntent(intent = N_360Hardflip)` is `HAS_INTENT_N_360_HARDFLIP`;
  `WillExpire(InTime = 0.05, waitForTransitions = false)` is
  `WILL_EXPIRE_IN_0_05_NOT_WAIT`; `IsMovingObject(anglestart = 0, angleend = 110)`
  is `IS_MOVING_OBJECT_FROM_0_TO_110`. A decimal point becomes `_`, a minus sign
  `NEG_`. A declaration that repeats an earlier one keeps its own index, which the
  evaluators read separately, and ends in `_2ND`.
- `expression`: every expression a state or transition evaluates, named by its
  users. A state's guard takes the state's label (`TAKEOFF_1355` for `Takeoff1355`), a
  transition's takes source and target (`TAKEOFF_1355_TO_DESCENT_1356`). An expression
  several users share takes what they have in common (`FROM_GRIND` guards every
  `FromGrind` state, `TO_SUPERMAN` both `ToSuperman` states) or a name from the
  generator's `SHARED` table that says what it tests (`WILL_EXPIRE_IN_0_05_NOT_WAIT`
  is shared by 276 transitions). The doc comment lists the users.
- `state`: every state as name plus index, the label `tools/graph_query.py` prints
  (`TAKEOFF_1355`), with the state's path below the root in the doc comment. A name
  that starts with a digit gets a `STATE_` prefix (`STATE_360_FLIP_199`).

Evaluators import the modules under short aliases:

```rust
use crate::generated_motion_load::ids::{condition as cond, expression as expr};

Ok(match index {
    expr::TAKEOFF_1355_TO_DESCENT_1356 => c(cond::WILL_EXPIRE_IN_0_017)?,
    expr::LANDING_ON_BOARD_28 => {
        c(cond::CAN_LAND_ON_BOARD)? && c(cond::OB_TIME_TO_LAND_GREATER_0_05)?
    }
    // ...
```

Action code that keys on states imports `ids::state as state_id`, because
`state` is already a common local name there:
`self.current_state == state_id::ACCIDENTAL_DISMOUNT_31`.

A path pattern such as `expr::TAKEOFF_1355` cannot become a catch-all binding: a
missing or misspelled name fails to compile.

### Regenerating

```powershell
python tools/generate_graph_ids.py all
python tools/generate_graph_ids.py motion --check
python tools/generate_graph_ids.py motion --missing
```

`--check` fails when a module is stale. The generator refuses to write when a
shared expression has no name; `--missing` lists those, and their names go into
`SHARED`.

### Verification

`tools/compare_graph_expressions.py` reads the names back to indices, so the proof
of the Motion and Action evaluators against the native `EvaluateExpression` switch
paths (extracted with `tools/ida_graph_switch_paths.py`) runs on the named code:
all 894 Motion and 214 Action decision cases match. The Motion graph carries the
off-board Idle of Title Update 2 (1.02); against that build's switch, mapped to the
port's indices, 896 of 898 cases match, and the other two are `DebugAnimations`
(272), which the retail build compiles to false.

## Physics states

`physics/player/lifecycle/physics_state_ids.rs` names
`Sk8::Physics::ePhysicsState` (`PHYSICS_STATE_*`, what each `PhysState_*::GetType`
returns) and `Sk8::Physics::ePhysicsCategory` (`PHYSICS_CATEGORY_*`). A category is
its states' hundred: `PHYSICS_STATE_GRIND_5050` (401) is in
`PHYSICS_CATEGORY_GRIND` (400). `PhysState_OffBoard_Jump::GetType` returns
`PHYSICS_STATE_OFFBOARD_AIR`, not `PHYSICS_STATE_OFFBOARD_JUMP`.

These fields hold a state: `PhysOut_State` `physics_state` and
`teleport_exit_state`, the processed `current`, `last` and `next`. These hold a
category: `physics_state_category`, the processed `category` and `last_category`,
and `jphy_state`.

## What stays numeric

- Behaviour slot lists in the per-state active-set tables (the Motion lifecycle,
  grind, moving and handplant tables and the Action runtime) are verification data
  keyed by state and native update address; the states in them are named where the
  code branches on them, and each row names its state in a comment.
- The marquee and photographer graphs, whose evaluators map most expressions
  straight to the condition of the same index, and the ten pedestrian plugin graphs
  (`living_world/pedestrians/pedestrian_plugin_expressions.rs`, one `Family` each).
- Native addresses, which stay in comments as references into the executable.
