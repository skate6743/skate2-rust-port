//! Concrete MotionGraph condition leaves and stock attributes.
use super::motion::MotionHost;
use skate3_core::graph::controller::Frame;
use skate3_core::{
    animation::{output::attributes::AttributeName, skeleton_input::name::encode},
    graph::conditions::{ActionCondition, Comparison, NumericCondition},
};
use skate_data::state_graph::attributes::Attributes;

#[derive(Clone, Debug, PartialEq)]
pub enum MotionCondition {
    Grind(super::motion_grind::conditions::Condition),
    HasTweak(NumericCondition),
    CurrentGrabType(super::motion_stock_gameplay::GrabType),
    ManualOutTimerIsActive,
    Gesture(crate::skate3_native::input::gesture_catalog::Group),
    DisableDismount(super::motion_dismount::Condition),
    Landing(super::motion_landing::Condition),
    Wipeout(super::motion_wipeout::Condition),
    PushOff(super::motion_push_off::IsPushOffEnabled),
    Shared(ActionCondition),
    Intent {
        name: String,
        filtered: bool,
        numeric: NumericCondition,
    },
    AnimationAttribute {
        name: AttributeName,
        sequence_id: i32,
        numeric: NumericCondition,
    },
    ExpireInTime(NumericCondition),
    WillExpire {
        in_time: f32,
        tag: Option<String>,
        wait_for_transitions: bool,
    },
    InTimeWindow {
        start: f32,
        length: f32,
    },
    InStateForTime {
        name: String,
        target: Option<usize>,
        numeric: NumericCondition,
    },
    InParentStateForTime {
        target: Option<usize>,
        numeric: NumericCondition,
    },
    ProSkater(AttributeName),
    BreakOutOfPush,
    ShouldLeaveSlide {
        right: bool,
    },
    RidingGoofy,
    RidingSwitch,
    MongoPushFootTooFar,
    LastState {
        name: String,
        target: Option<usize>,
    },
    Gameplay(super::motion_gameplay_conditions::GameplayCondition),
    Riding(super::motion_riding_conditions::MotionRidingCondition),
    /// TimeToLand82BA7250: valid Air trajectory remaining time.
    TimeToLand(NumericCondition),
    /// Native off-board trajectory time (PhysOutOffBoard+32).
    ObTimeToLand(NumericCondition),
    /// Native off-board trajectory time used by dismount/runout branches.
    ObTrajTime(NumericCondition),
    /// Biped locomotion bucket published by the physical off-board owner.
    LocoState(super::motion_offboard_cadence::LocoState),
    GroundSlopeType(super::motion_ground_slope::GroundSlopeType),
    /// Thin-ground branch of the stock BipedGround state.
    IsBipedGroundThin,
    IsHoldingSkateboard,
    /// Static gameplay has no moving actors yet; the stock condition is a
    /// resolved physical flag and must remain false until a moving support is
    /// published by the collision owner.
    IsStandingOnMovingObject,
    /// AngleAdjustByAnimationEndLessThan (factory 0x82bc6398: MaxDeltaAngle, default 2).
    AngleAdjustByAnimationEndLessThan { max_delta_angle: f32 },
    /// IsInDebugAnimationsMode is a retail debug gate. TU3 has no gameplay
    /// producer for this flag; the shipped runtime never enables debug
    /// animation mode, so the condition is a resolved false leaf rather than
    /// an unsupported graph node.
    DebugAnimationsMode,
    /// Stock gameplay predicates whose concrete values are published by the
    /// grind/handplant/skitch physical owners.
    StockGameplay(super::motion_stock_conditions::Condition),
}
impl MotionCondition {
    pub fn parse(a: &Attributes<'_>) -> Result<Option<Self>, String> {
        if let Some(condition) = super::motion_grind::conditions::Condition::parse(a)? {
            return Ok(Some(Self::Grind(condition)));
        }
        if let Some(condition) = super::motion_landing::Condition::parse(a) {
            return Ok(Some(Self::Landing(condition)));
        }
        if let Some(condition) = super::motion_wipeout::Condition::parse(a)? {
            return Ok(Some(Self::Wipeout(condition)));
        }
        if let Some(condition) = super::motion_push_off::IsPushOffEnabled::parse(a) {
            return Ok(Some(Self::PushOff(condition)));
        }
        if let Some(condition) = super::motion_dismount::Condition::parse(a) {
            return Ok(Some(Self::DisableDismount(condition)));
        }
        let numeric = || super::condition_nodes::numeric(a);
        let name = a
            .text("name")
            .unwrap_or("")
            .trim_matches(|c: char| c.is_whitespace() || c == '\0');
        Ok(Some(match name {
            "HasTweak" => Self::HasTweak(numeric()),
            "CurrentGrabType" => Self::CurrentGrabType(
                super::motion_stock_gameplay::GrabType::parse(
                    a.text("grab").ok_or("CurrentGrabType requires grab")?,
                )?,
            ),
            "ManualOutTimerIsActive" => Self::ManualOutTimerIsActive,
            "HasGestureIntent" => Self::Gesture(crate::skate3_native::input::gesture_catalog::Group::parse(
                a.text("group").unwrap_or(""),
            )?),
            //82BA59D8/82BA6AC0: missing intent is false even for NotEqual.
            "HasIntent" | "HasFilteredMotionGraphIntent" => Self::Intent {
                name: a.text("intent").unwrap_or("").into(),
                filtered: a.text("name") == Some("HasFilteredMotionGraphIntent"),
                numeric: numeric(),
            },
            "HasAnimAttribute" => Self::AnimationAttribute {
                name: encode(a.text("attribute").unwrap_or("").as_bytes()),
                sequence_id: f32::from_bits(a.float_bits("sequenceid", (-1.0f32).to_bits())) as i32,
                numeric: numeric(),
            },
            "ExpireInTime" => Self::ExpireInTime(numeric()), //82BA65A0
            "WillExpire" => Self::WillExpire {
                //82BA6620/82BA6708
                in_time: f32::from_bits(a.float_bits("InTime", 0)),
                tag: a
                    .text("InTimeTag")
                    .filter(|v| !v.is_empty())
                    .map(str::to_owned),
                wait_for_transitions: a.boolean_byte("waitForTransitions", 1) != 0,
            },
            "InTimeWindow" => Self::InTimeWindow {
                //82BC4558/82BA6808
                start: f32::from_bits(a.float_bits("StartTime", 0)),
                length: f32::from_bits(a.float_bits("WindowFrameLength", 0)),
            },
            "InStateForTime" => Self::InStateForTime {
                //82BC3788/82BA4658
                name: a.text("state").unwrap_or("").into(),
                target: None,
                numeric: numeric(),
            },
            "InParentStateForTime" => Self::InParentStateForTime {
                target: None,
                numeric: numeric(),
            }, //82BA45D0
            "IsProSkater" => Self::ProSkater(encode(a.text("skater").unwrap_or("").as_bytes())), //82BC56C0/82BA6A48
            "BreakOutOfPush" => Self::BreakOutOfPush,
            //Factory82BC54B8 and Activation82BA7130.
            "ShouldLeaveSlide" => Self::ShouldLeaveSlide {
                right: a.boolean_byte("right", 1) != 0,
            },
            "IsRidingGoofy" => Self::RidingGoofy,
            "IsRidingSwitch" => Self::RidingSwitch,
            "MongoPushFootToFar" => Self::MongoPushFootTooFar,
            "LastState" => Self::LastState {
                name: a.text("state").unwrap_or("").into(),
                target: None,
            },
            name if super::motion_gameplay_conditions::GameplayCondition::recognizes(name) => {
                Self::Gameplay(super::motion_gameplay_conditions::GameplayCondition::parse(
                    a,
                )?)
            }
            name if super::motion_riding_conditions::MotionRidingCondition::recognizes(name) => {
                Self::Riding(super::motion_riding_conditions::MotionRidingCondition::parse(a)?)
            }
            "TimeToLand" => Self::TimeToLand(numeric()),
            "OBTimeToLand" => Self::ObTimeToLand(numeric()),
            "OBTrajTime" => Self::ObTrajTime(numeric()),
            "LocoState" => Self::LocoState(super::motion_offboard_cadence::LocoState::parse(a)?),
            "GroundSlopeType" => Self::GroundSlopeType(super::motion_ground_slope::GroundSlopeType::parse(a)?),
            "IsBipedGroundThin" => Self::IsBipedGroundThin,
            "IsHoldingSkateboard" => Self::IsHoldingSkateboard,
            "IsStandingOnMovingObject" => Self::IsStandingOnMovingObject,
            "AngleAdjustByAnimationEndLessThan" => Self::AngleAdjustByAnimationEndLessThan {
                max_delta_angle: f32::from_bits(a.float_bits("MaxDeltaAngle", 0x4000_0000)),
            },
            "IsInDebugAnimationsMode" => Self::DebugAnimationsMode,
            name if super::motion_stock_conditions::Condition::recognizes(name) => {
                Self::StockGameplay(super::motion_stock_conditions::Condition::parse(a))
            }
            _ => return Ok(super::condition_nodes::parse(a)?.map(Self::Shared)),
        }))
    }
    pub fn evaluate(&self, host: &MotionHost, frame: &Frame) -> Result<bool, String> {
        use skate3_core::animation::playback_parameters::ParameterInputs;
        Ok(match self {
            Self::Grind(condition) => condition.evaluate(
                host.grind_conditions.as_ref()
                    .ok_or("Grind condition requires completed physical output")?
            ),
            // 82BA7760/82BA7848 compares the authored type through
            // ISkaterAnim; SetGrabType Begin/End writes that same owner.
            Self::CurrentGrabType(grab) => host.animation.grab_type == Some(*grab),
            // TU3 82BA6B90 uses the filtered MG map (virtual +12),
            // requires at least one axis, then compares vector magnitude.
            Self::HasTweak(numeric) => {
                let x = host.animation.filtered_intent("TweakX");
                let y = host.animation.filtered_intent("TweakY");
                (x.is_some() || y.is_some())
                    && (numeric.comparison == Comparison::None
                        || numeric.matches((x.unwrap_or(0.0).powi(2)
                            + y.unwrap_or(0.0).powi(2)).sqrt()))
            }
            // Native 82BA78B0: strictly positive retained manual-out timer.
            Self::ManualOutTimerIsActive => host.riding.manual_out_timer > 0.0,
            Self::ShouldLeaveSlide { right } => host.slide_latch.should_leave(*right),
            Self::Gesture(group) => group.has_intent(|name| host.action_controls.has(name)),
            Self::DisableDismount(condition) => condition.evaluate(host.condition_inputs.push_brake.as_ref())?,
            Self::Shared(condition) => condition
                .evaluate(
                    &host.condition_inputs,
                    &host.action_intents,
                    frame.current,
                    &host.state_parents,
                )
                .map_err(str::to_owned)?,
            Self::Gameplay(condition) => condition.evaluate(host)?,
            Self::Riding(condition) => condition.evaluate(host)?,
            Self::TimeToLand(n) => {
                let p = host.gameplay_conditions.as_ref()
                    .ok_or("TimeToLand requires physical condition publication")?;
                p.time_to_land_valid && n.matches(p.time_to_land)
            }
            Self::ObTimeToLand(n) => n.matches(
                host.gameplay_conditions.as_ref()
                    .ok_or("OBTimeToLand requires physical condition publication")?
                    .offboard_time_to_land,
            ),
            Self::ObTrajTime(n) => {
                let p = host.gameplay_conditions.as_ref()
                    .ok_or("OBTrajTime requires physical condition publication")?;
                p.offboard_trajectory_valid && n.matches(p.offboard_trajectory_time)
            }
            Self::DebugAnimationsMode => false,
            Self::LocoState(condition) => condition.evaluate(
                host.offboard_locomotion_state
                    .ok_or("LocoState requires retained Biped locomotion through OffBoard84")?,
            ),
            Self::GroundSlopeType(condition) => condition.matches(
                host.ground_slope_type
                    .ok_or("GroundSlopeType requires the completed ground slope publication")?,
            ),
            Self::IsBipedGroundThin => host
                .biped_ground_thin
                .ok_or("IsBipedGroundThin requires the native ground geometry publication")?,
            Self::IsHoldingSkateboard => host.toggle_board_physical
                .is_some_and(|p| p.holding_board),
            Self::IsStandingOnMovingObject => host
                .gameplay_conditions
                .ok_or("IsStandingOnMovingObject requires the physical state publication")?
                .moving_object,
            Self::AngleAdjustByAnimationEndLessThan { max_delta_angle } => {
                super::motion_moving_object::angle_adjust_by_animation_end_less_than(
                    &host.moving_objects.coefficients,
                    &host.animation,
                    *max_delta_angle,
                )?
            }
            Self::StockGameplay(condition) => condition.evaluate(host)?,
            Self::PushOff(condition) => condition.evaluate(),
            Self::Wipeout(condition) => condition.evaluate(host.wipeout_physical)?,
            Self::Landing(condition) => condition.evaluate(
                host.landing_physical,
                &host.flags,
                host.prelanding_physical
                    .map(|p| p.override_prelanding(&host.spin)),
            )?,
            Self::LastState { target, .. } => {
                //82BA4798 obtains the last state, then uses the same native
                //ancestor membership test as CurrentState82C13820.
                let mut cursor = frame.last;
                let mut matched = false;
                if let Some(target) = target {
                    while let Some(state) = cursor {
                        if state == *target {
                            matched = true;
                            break;
                        }
                        cursor = host.state_parents.get(state).copied().flatten();
                    }
                }
                matched
            }
            Self::Intent {
                name,
                filtered,
                numeric,
            } => {
                let value = if *filtered {
                    host.animation.filtered_intent(name)
                } else {
                    host.animation.motion_intent(name)
                };
                value.is_some_and(|value| {
                    numeric.comparison == Comparison::None || numeric.matches(value)
                })
            }
            Self::AnimationAttribute {
                name,
                sequence_id,
                numeric,
            } => super::condition_nodes::animation_attribute(
                host.animation.tree_attributes(),
                *name,
                *sequence_id,
                *numeric,
            ),
            Self::ExpireInTime(numeric) => {
                numeric.matches(host.animation.property().remaining_before_wrap)
            }
            Self::WillExpire {
                in_time,
                tag,
                wait_for_transitions,
            } => {
                //8296E988 uses authored InTime if tag is empty/component absent.
                let time = match (tag, &host.time_tags) {
                    (Some(tag), Some(tags)) => *tags
                        .get(tag)
                        .ok_or_else(|| format!("Missing MotionGraph float tag {tag}"))?,
                    _ => *in_time,
                };
                if *wait_for_transitions && host.animation.in_transition() {
                    false
                } else {
                    let p = host.animation.property();
                    p.crossed_end || !(p.remaining_before_wrap > time)
                }
            }
            Self::InTimeWindow { start, length } => {
                if host.animation.in_transition() {
                    false
                } else {
                    let time = host.animation.current_time()?;
                    !(time < *start) && !(time > *start + *length)
                }
            }
            Self::InStateForTime {
                target, numeric, ..
            } => super::condition_nodes::state_time(frame, *target, *numeric),
            Self::InParentStateForTime { target, numeric } => {
                super::condition_nodes::state_time(frame, *target, *numeric)
            }
            Self::ProSkater(name) => host.playback_context.pro_skater == *name,
            Self::BreakOutOfPush => {
                //82BA6258: current tree time >= out factor * length, and
                // the push lifecycle has not latched continue_push.
                let push = host
                    .push_state
                    .as_ref()
                    .ok_or("BreakOutOfPush requires initialized native push state")?;
                !(host.animation.current_time()?
                    < push.out_factor * host.animation.current_length()?)
                    && !push.continue_push
            }
            Self::RidingGoofy => {
                let (first, second) = host
                    .physical_stance
                    .ok_or("IsRidingGoofy requires PhysOutAnimation stance bytes")?;
                first == second
            }
            Self::RidingSwitch => host
                .playback_context
                .is_switch
                .ok_or("IsRidingSwitch requires actual relative stance")?,
            Self::MongoPushFootTooFar => {
                //82BA6378 queries the FIRST cached tree record via82D19010.
                //The event payload is a toe-bone FastString, not its weight.
                if let Some(event) = host
                    .animation
                    .tree_attributes()
                    .iter()
                    .find(|a| a.name == encode(b"push_contact"))
                {
                    let matches = |name| {
                        event.payload.0[..5]
                            .iter()
                            .zip(encode(name).0)
                            .all(|(word, key)| *word == Some(key))
                    };
                    let right = matches(b"RightToeBase");
                    let left = matches(b"LeftToeBase");
                    let mirrored = host
                        .playback_context
                        .is_mirrored
                        .ok_or("MongoPushFootToFar requires actual mirrored stance")?;
                    if (!mirrored && right) || (mirrored && left) {
                        host.physical
                            .and_then(|p| p.foot_frame)
                            .ok_or("MongoPushFootToFar requires actual foot frame")?
                            .out_distance(right)[0]
                            < -0.5
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
        })
    }
}

pub fn bind_states(
    graph: &crate::skate3_native::graph_runtime::LoadedGraph,
    operations: &mut [super::motion_nodes::MotionOperation],
) {
    let mut parents = vec![None; graph.source.elements.len()];
    for (parent, element) in graph.source.elements.iter().enumerate() {
        for &child in &element.children {
            parents[child] = Some(parent);
        }
    }
    for &id in &graph.runtime.operations.conditions {
        let super::motion_nodes::MotionOperation::Condition(condition) = &mut operations[id] else {
            continue;
        };
        let (name, target) = match condition {
            MotionCondition::Shared(ActionCondition::CurrentState { name, target })
            | MotionCondition::InStateForTime { name, target, .. }
            | MotionCondition::LastState { name, target } => (Some(name), target),
            MotionCondition::InParentStateForTime { target, .. } => (None, target),
            _ => continue,
        };
        let mut element = parents[graph.binding.operations[id].element];
        while let Some(id) = element {
            if let Some(state) = graph.binding.states.iter().position(|s| s.element == id) {
                // GetParentState82C11B88 climbs expression nodes to the
                // containing state; it does not select that state's parent.
                *target = name.as_ref().map_or(Some(state), |name| {
                    graph.binding.find_state(state, name, true)
                });
                break;
            }
            element = parents[id];
        }
    }
}
