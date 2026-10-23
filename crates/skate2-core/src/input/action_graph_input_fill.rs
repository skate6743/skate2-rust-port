//! Continuous `ActionGraphInputListener::Fill`, 0x825fb5b4..825fd3c0, with the
//! off-board stick of Title Update 2 (1.02 `Fill` 824e54e8, 824e6314..68f4) in
//! place of the original Vault steer and magnitude curves.
//! Runtime Vault fetches and cvar sampling remain ordered operations. Numerical
//! estimates retain the established host boundary; this is not a Xenon trace.
#![expect(
    clippy::type_complexity,
    reason = "distinct runtime collaborator failure prefixes"
)]
#![expect(
    clippy::neg_cmp_op_on_partial_ord,
    reason = "raw branches distinguish unordered from ordered comparisons"
)]

use super::{
    ActionGraphInputListener, ActionInputDerivedError, ActionInputIntentSink,
    ActionInputSuffixError, DerivedControllerInput, DerivedInputAttributes,
};
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault},
    camera_frame::composition_math::look_basis,
    physics::PointGraph,
    physics_state_ids::{PHYSICS_STATE_OFFBOARD, PHYSICS_STATE_OFFBOARD_LOCOMOTION},
    reckoning_numeric::{
        ReckoningNumeric, ReckoningVector, cross_typed, length_from_squared_and_estimate, vmx_flush,
    },
    skeleton_query_rotation::try_atan,
    skeleton_root_transforms::SkeletonRootVector,
    skeleton_two_bone::try_inverse_basis_from_cofactors,
};
use std::sync::Arc;

#[cfg(test)]
#[path = "action_graph_input_fill_tests.rs"]
mod tests;

const PI: f32 = f32::from_bits(0x4049_0fdb);
const HALF_PI: f32 = f32::from_bits(0x3fc9_0fdb);
const INV_PI: f32 = f32::from_bits(0x3ea2_f983);
const STICK_RELEASED: f32 = f32::from_bits(0x38d1_b717);
const STEER_SCALE: f32 = f32::from_bits(0x3fe0_0000);
const RIGHT_STICK_STEER: f32 = f32::from_bits(0x3e99_999a);
const WALK_START: f32 = f32::from_bits(0x3e99_999a);
const WALK_RANGE: f32 = f32::from_bits(0x3fb6_db6e);
const WALK_MAGNITUDE: f32 = f32::from_bits(0x3e4c_cccd);
const LATCH_STEER: f32 = f32::from_bits(0x3e4c_cccd);
const RUN_START: f32 = f32::from_bits(0x3f33_3333);
const RUN_RANGE: f32 = f32::from_bits(0x4055_5555);
const OBJECT_ROTATE: u64 = 0x9adf_c2e2_2293_8c1e;
const OBJECT_X: u64 = 0x1a1a_7ac3_7a72_df87;
const OBJECT_Z: u64 = 0x05ba_8b52_c23b_3481;

/// Same selected global `inputlistener_default`, class5149ae9f16190257.
pub(crate) struct ActionInputFillAttributes {
    pub(crate) derived: DerivedInputAttributes,
    instance: AttributeRuntimeInstance,
}
impl ActionInputFillAttributes {
    pub(crate) fn new(vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self {
            derived: DerivedInputAttributes::new(vault),
            instance: vault.instance(super::derived::INPUT_LISTENER_CLASS, 0xd7ed_bd36_2d7d_2152),
        }
    }
    fn curve<const N: usize>(&self, key: u64, value: f32) -> Result<f32, AttributeRuntimeError> {
        let mut curve = PointGraph::<N>::default();
        if let Some(data) = self.instance.try_get_data(key, 0)? {
            let bytes = data.bytes();
            let expected = 16 + 8 * N;
            if bytes.len() != expected {
                return Err(AttributeRuntimeError::ScalarSize { size: bytes.len() });
            }
            for i in 0..N {
                curve.x[i] = f32::from_bits(u32::from_be_bytes(
                    bytes[16 + 4 * i..20 + 4 * i]
                        .try_into()
                        .expect("four bytes"),
                ));
                curve.y[i] = f32::from_bits(u32::from_be_bytes(
                    bytes[16 + 4 * N + 4 * i..20 + 4 * N + 4 * i]
                        .try_into()
                        .expect("four bytes"),
                ));
            }
        }
        // Missing pointer uses exactly 0x50/0x90 zero DefaultDataArea.
        Ok(curve.evaluate(value))
    }
}

/// 1.02 listener fields: +0x80, the stick's world direction when a turn
/// latched, and +0x91, the latch. Constructor 824e51a8..51b4.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct OffBoardStickLatch {
    pub(crate) direction: [f32; 4],
    pub(crate) latched: bool,
}
impl Default for OffBoardStickLatch {
    fn default() -> Self {
        Self {
            direction: [0.0, 0.0, 1.0, 0.0],
            latched: false,
        }
    }
}

/// The physical output Fill reads: `PhysOut_State` state and the 1.02 read of
/// `PhysOut_Skeleton::mEffectiveAnimZ`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ActionInputPhysOut {
    pub(crate) physics_state: u32,
    pub(crate) effective_anim_z: SkeletonRootVector,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ActionInputFloatCvar {
    /// cVarFloatRef830484b0, target+D8; same target for A then X.
    NewPushAllowance,
    /// cVarFloatRef83048484, target+D8.
    TimeToConsiderWipeout,
    /// cVarFloatRef830484dc, target+D8.
    NewToggleOffboardAllowance,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[expect(clippy::enum_variant_names, reason = "exact original cvar labels")]
pub(crate) enum ActionInputBoolCvar {
    /// cVarBoolRef8304811c, target+CD.
    UseTriggersForExplicitWipeout,
    /// cVarBoolRef83048460, target+CD.
    UseReleaseForWipeout,
    /// cVarBoolRef830480f8, target+CD; same target for jump then sprint.
    UseSprintButtonInput,
}
/// Supplies current cvar targets/values. Repeated calls for one cvar in a Fill
/// must retain the same target while rereading its value; no cached tuning.
pub(crate) trait ActionInputFillCvars {
    type Error;
    fn capture_float(&mut self, key: ActionInputFloatCvar) -> Result<(), Self::Error>;
    fn capture_boolean(&mut self, key: ActionInputBoolCvar) -> Result<(), Self::Error>;
    fn float(&mut self, key: ActionInputFloatCvar) -> Result<f32, Self::Error>;
    fn boolean(&mut self, key: ActionInputBoolCvar) -> Result<bool, Self::Error>;
}
/// Late source Juice service, 826e39e8/826e3a38. The name request must copy at
/// most 63 bytes (native64-byte destination) and clear active BEFORE returning.
/// No disconnected/always-false production implementation is supplied here.
pub(crate) trait ActionInputJuice {
    type Error;
    fn is_trick_active(&mut self) -> Result<bool, Self::Error>;
    fn take_trick_name(&mut self) -> Result<String, Self::Error>;
}
#[derive(Debug)]
pub(crate) enum ActionInputContinuousError<N, C, S> {
    Numeric(N),
    Cvar(C),
    MissingGlobalAttributes,
    Attribute(AttributeRuntimeError),
    Insert(S),
}
#[derive(Debug)]
pub(crate) enum ActionInputFillError<N, C, S, J> {
    Derived(ActionInputDerivedError<N>),
    Continuous(ActionInputContinuousError<N, C, S>),
    Pattern(ActionInputSuffixError<S>),
    Juice(J),
    Insert(S),
}

impl<K> ActionGraphInputListener<K> {
    /// Selected same-player/same-listener Fill: Derived update, entire continuous
    /// body, retained pattern events, then late Juice request. Does not clear the
    /// Action map/events or perform Actor AI/force-brake gates. The currently
    /// mounted event route preserves native InputEvent/pattern interleaving.
    #[expect(
        clippy::too_many_arguments,
        reason = "explicit retained runtime collaborators"
    )]
    pub(crate) fn try_fill_selected<
        N: ReckoningNumeric,
        C: ActionInputFillCvars,
        S: ActionInputIntentSink,
        J: ActionInputJuice,
    >(
        &mut self,
        dt: f32,
        phys_out: ActionInputPhysOut,
        attributes: Option<&ActionInputFillAttributes>,
        cvars: &mut C,
        numeric: &mut N,
        sink: &mut S,
        juice: &mut J,
    ) -> Result<(), ActionInputFillError<N::Error, C::Error, S::Error, J::Error>> {
        self.try_update_derived(
            dt,
            phys_out.physics_state,
            attributes.map(|a| &a.derived),
            numeric,
        )
        .map_err(ActionInputFillError::Derived)?;
        try_continuous(
            self.derived.as_ref().expect("updated retained Derived"),
            &mut self.off_board_stick,
            phys_out,
            attributes,
            cvars,
            numeric,
            sink,
        )
        .map_err(ActionInputFillError::Continuous)?;
        self.try_pattern_intent_suffix(sink)
            .map_err(ActionInputFillError::Pattern)?;
        if juice
            .is_trick_active()
            .map_err(ActionInputFillError::Juice)?
        {
            let name = juice
                .take_trick_name()
                .map_err(ActionInputFillError::Juice)?;
            for key in ["Trick", name.as_str(), "TrickHeight", "GestureSpeed"] {
                sink.insert(key, 1.0)
                    .map_err(ActionInputFillError::Insert)?;
            }
        }
        Ok(())
    }
}

/// Source fsel: both signs of zero select nonnegative, unordered selects other.
fn select(value: f32, nonnegative: f32, other: f32) -> f32 {
    if value >= 0.0 { nonnegative } else { other }
}
fn clamp_signed(value: f32) -> f32 {
    let lower = select(-1.0 - value, -1.0, value);
    select(1.0 - lower, lower, 1.0)
}
fn rising(previous: bool, current: bool) -> bool {
    !previous && current
}

/// Angle about -Y, one reciprocal refinement then source `XMVectorATan`.
/// Quadrant selection uses X SIGN BITS; +/-0 denominator selects signed pi/2.
fn stick_angle<N: ReckoningNumeric>(x: f32, y: f32, numeric: &mut N) -> Result<f32, N::Error> {
    let denominator = -y;
    let seed = numeric.reciprocal_estimate(denominator)?;
    let residual = (-seed).mul_add(denominator, 1.0);
    let inverse = seed.mul_add(residual, seed);
    let angle = try_atan(x.mul_add(inverse, 0.0), numeric)?;
    let sign = x.to_bits() & 0x8000_0000;
    let quadrant = if 0.0 > denominator {
        f32::from_bits(PI.to_bits() | sign) + angle
    } else {
        angle
    };
    Ok(if denominator == 0.0 {
        f32::from_bits(HALF_PI.to_bits() | sign)
    } else {
        quadrant
    })
}
fn length<N: ReckoningNumeric>(squared: f32, numeric: &mut N) -> Result<f32, N::Error> {
    let seed = numeric.reciprocal_sqrt_estimate(squared)?;
    Ok(length_from_squared_and_estimate(squared, seed))
}

/// 1.02 `PickOffBoardWalkDirection` 824e53b8: eighths of a turn around the
/// stick angle measured from straight back. Each bound is a raw bge, so an
/// unordered angle takes the first sector after Backward.
fn walk_direction(angle: f32) -> &'static str {
    const SECTORS: [(u32, &str); 6] = [
        (0x3f96_cbe4, "OB_WalkBackwardLeft"),
        (0x3ffb_53d2, "OB_WalkLeft"),
        (0x402f_ede0, "OB_WalkForwardLeft"),
        (0x4062_31d6, "OB_WalkForward"),
        (0x408a_3ae7, "OB_WalkForwardRight"),
        (0x40a3_5ce2, "OB_WalkRight"),
    ];
    if angle < f32::from_bits(0x3ec9_0fdb) || angle >= f32::from_bits(0x40bc_7edd) {
        return "OB_WalkBackward";
    }
    for (bound, walk) in SECTORS {
        if !(angle >= f32::from_bits(bound)) {
            return walk;
        }
    }
    "OB_WalkBackwardRight"
}

/// 824e63d0..6488: the latched world direction in the current facing frame,
/// through the frame's adjugate inverse, not its transpose. 1.02 `sub_826F7668`
/// (826f7780..7840) takes the off-board stick into the skater's frame alike.
pub(crate) fn try_local_direction<N: ReckoningNumeric>(
    frame: [SkeletonRootVector; 3],
    world: [f32; 4],
    numeric: &mut N,
) -> Result<[f32; 4], N::Error> {
    let [x, y, z] = frame;
    let cofactors = [cross_typed(y, z), cross_typed(z, x), cross_typed(x, y)];
    let basis =
        try_inverse_basis_from_cofactors(x, cofactors, numeric)?.map(ReckoningVector::operand);
    Ok(std::array::from_fn(|lane| {
        let first = basis[0][lane] * world[0];
        let second = basis[1][lane].mul_add(world[1], first);
        basis[2][lane].mul_add(world[2], second)
    }))
}

/// 824e6864..6884: the stick in world space when a turn latches. 1.02
/// `sub_826F7668` (826f7718..772c) takes the stick through the camera's rows alike.
pub(crate) fn world_direction(frame: [SkeletonRootVector; 3], stick: [f32; 4]) -> [f32; 4] {
    let frame = frame.map(ReckoningVector::operand);
    std::array::from_fn(|lane| {
        let first = frame[0][lane] * stick[0];
        let second = frame[1][lane].mul_add(stick[1], first);
        frame[2][lane].mul_add(stick[2], second)
    })
}

#[expect(
    clippy::too_many_lines,
    reason = "one ordered native producer before first insertion"
)]
#[expect(
    clippy::float_cmp,
    reason = "source exact nonzero/full-trigger predicates"
)]
fn try_continuous<N: ReckoningNumeric, C: ActionInputFillCvars, S: ActionInputIntentSink>(
    derived: &DerivedControllerInput,
    latch: &mut OffBoardStickLatch,
    phys_out: ActionInputPhysOut,
    attributes: Option<&ActionInputFillAttributes>,
    cvars: &mut C,
    numeric: &mut N,
    sink: &mut S,
) -> Result<(), ActionInputContinuousError<N::Error, C::Error, S::Error>> {
    use ActionInputBoolCvar::{
        UseReleaseForWipeout, UseSprintButtonInput, UseTriggersForExplicitWipeout,
    };
    use ActionInputContinuousError::{Attribute, Cvar, Insert, MissingGlobalAttributes, Numeric};
    use ActionInputFloatCvar::{
        NewPushAllowance, NewToggleOffboardAllowance, TimeToConsiderWipeout,
    };
    let p = &derived.previous;
    let r = &derived.current;
    let (lx, ly, rx, ry) = (
        r.left_stick_x,
        r.left_stick_y,
        r.right_stick_x,
        r.right_stick_y,
    );
    let right_active = rx != 0.0 || ry != 0.0;
    let right_angle = if right_active {
        stick_angle(rx, ry, numeric).map_err(Numeric)?
    } else {
        0.0
    };
    let right_mag = if right_active {
        length(rx.mul_add(rx, ry * ry), numeric).map_err(Numeric)?
    } else {
        0.0
    };
    let left_active = lx != 0.0 || ly != 0.0;
    let left_angle = if left_active {
        stick_angle(lx, ly, numeric).map_err(Numeric)?
    } else {
        0.0
    };
    let left_mag = if left_active {
        length(ly.mul_add(ly, lx * lx), numeric).map_err(Numeric)?
    } else {
        0.0
    };
    let left_abs = left_angle.abs();
    let gesture_allowed = !r.left_bumper && !r.right_bumper;
    let gesture_up = gesture_allowed && rising(p.dpad_up, r.dpad_up);
    let gesture_down = gesture_allowed && rising(p.dpad_down, r.dpad_down);
    let gesture_left = gesture_allowed && rising(p.dpad_left, r.dpad_left);
    let gesture_right = gesture_allowed && rising(p.dpad_right, r.dpad_right);
    let new_a = rising(p.face_a, r.face_a);
    let new_x = rising(p.face_x, r.face_x);
    let recover = new_a || new_x;
    cvars.capture_float(NewPushAllowance).map_err(Cvar)?;
    let new_push_right = new_a
        || (r.face_a && derived.face_held[0] < cvars.float(NewPushAllowance).map_err(Cvar)?);
    let new_push_left = new_x
        || (r.face_x && derived.face_held[2] < cvars.float(NewPushAllowance).map_err(Cvar)?);
    let left_full = r.left_trigger == 1.0;
    let right_full = r.right_trigger == 1.0;
    let left_air = r.left_trigger > 0.0;
    let right_air = r.right_trigger > 0.0;
    let new_b = rising(p.face_b, r.face_b);
    let exit_grind = new_x || new_a || new_b;
    let wipeout_gesture_y = clamp_signed((r.right_trigger + r.left_trigger).mul_add(0.5, ry));
    let new_right_trigger = p.right_trigger != 1.0 && right_full;
    let new_left_trigger = p.left_trigger != 1.0 && left_full;
    let retrieve = new_left_trigger || new_right_trigger;
    cvars
        .capture_boolean(UseTriggersForExplicitWipeout)
        .map_err(Cvar)?;
    let wipeout = if cvars.boolean(UseTriggersForExplicitWipeout).map_err(Cvar)? {
        left_full
            && right_full
            && r.left_stick_in
            && r.right_stick_in
            && (new_left_trigger
                || new_right_trigger
                || rising(p.left_stick_in, r.left_stick_in)
                || rising(p.right_stick_in, r.right_stick_in))
    } else {
        cvars.capture_float(TimeToConsiderWipeout).map_err(Cvar)?;
        let threshold = cvars.float(TimeToConsiderWipeout).map_err(Cvar)?;
        // Raw bge is BO4/BI24 (!LT), including unordered.
        let rise = derived.stick_edge_age[0] < threshold && derived.stick_edge_age[2] < threshold;
        cvars.capture_boolean(UseReleaseForWipeout).map_err(Cvar)?;
        if cvars.boolean(UseReleaseForWipeout).map_err(Cvar)? {
            rise && derived.stick_edge_age[1] < threshold && derived.stick_edge_age[3] < threshold
        } else {
            rise
        }
    };
    let signed_left_mag = select(lx, left_mag, -left_mag);
    let turn = if ly > 0.0 {
        if left_abs > f32::from_bits(0x4043_3333) {
            0.0
        } else if left_abs > f32::from_bits(0x4016_6666) {
            ((f32::from_bits(0x4043_3333) - left_abs) * f32::from_bits(0x3fb6_db6d))
                * signed_left_mag
        } else {
            signed_left_mag
        }
    } else {
        lx
    };
    let hard_turn = left_mag > f32::from_bits(0x3f66_6666)
        && !(left_abs > f32::from_bits(0x3fb1_eb85))
        && !(left_abs < f32::from_bits(0x3f68_f5c3));
    let hard_amount = (left_abs - f32::from_bits(0x3fb1_eb85)) * f32::from_bits(0xc005_5556);
    let hard_amount = select(
        hard_amount - f32::from_bits(0x3a83_126f),
        hard_amount,
        f32::from_bits(0x3a83_126f),
    );
    let hard_amount = select(left_angle, hard_amount, -hard_amount);
    let hard_crouch = if hard_turn {
        (hard_amount * f32::from_bits(0x3f59_999a)).abs()
    } else {
        0.0
    };
    let crouch_angle = left_abs > f32::from_bits(0x3ff5_c28f);
    let crouch_stick = if crouch_angle {
        if left_abs < f32::from_bits(0x4016_6666) {
            ((left_abs - f32::from_bits(0x3ff5_c28f)) * f32::from_bits(0x4014_d655)) * left_mag
        } else {
            left_mag
        }
    } else {
        0.0
    };
    // This multiplication by zero is present in the executable, including NaN.
    let crouch_stick = crouch_stick * 0.0;
    let crouch = crouch_angle || left_air || r.right_trigger != 0.0;
    let trigger_max = select(
        r.left_trigger - r.right_trigger,
        r.left_trigger,
        r.right_trigger,
    );
    let crouch_amount = select(crouch_stick - trigger_max, crouch_stick, trigger_max);
    let slide_mag = left_mag > f32::from_bits(0x3f66_6666);
    let right_slide = slide_mag && left_angle < 2.0 && left_angle > -HALF_PI;
    let right_slide_start =
        slide_mag && left_angle < f32::from_bits(0x3f68_f5c3) && left_angle > 0.0;
    let right_slide_amount = if right_slide {
        (2.0 - left_angle) * f32::from_bits(0x3e8f_62a8)
    } else {
        0.0
    };
    let left_slide = slide_mag && left_angle > -2.0 && left_angle < HALF_PI;
    let left_slide_start =
        slide_mag && left_angle > f32::from_bits(0xbf68_f5c3) && left_angle < -0.0;
    let left_slide_amount = if left_slide {
        -((left_angle + 2.0) * f32::from_bits(0x3e8f_62a8))
    } else {
        0.0
    };
    let manual_brake = right_mag > f32::from_bits(0x3f66_6666);
    let brake_amount = (right_mag - f32::from_bits(0x3f66_6666)) * f32::from_bits(0x411f_fffe);
    let brake_amount = if ry > 0.0 {
        brake_amount
    } else {
        -brake_amount
    };
    let brake_amount = if manual_brake { brake_amount } else { 0.0 };
    let manual_amount = if ry > 0.0 {
        right_mag
    } else {
        select(ry, 0.0, -right_mag)
    };
    let pump_angle = select(left_angle, PI - left_angle, -PI - left_angle);
    let no_input = !(left_active
        || right_active
        || r.face_b
        || new_push_right
        || r.face_a
        || new_push_left
        || r.face_x
        || left_full
        || right_full
        || left_air
        || right_air
        || r.right_bumper);
    let new_toggle = if r.left_bumper {
        false
    } else if rising(p.face_y, r.face_y) {
        true
    } else if r.face_y {
        cvars
            .capture_float(NewToggleOffboardAllowance)
            .map_err(Cvar)?;
        derived.face_held[3] < cvars.float(NewToggleOffboardAllowance).map_err(Cvar)?
    } else {
        false
    };
    let toggle = !r.left_bumper && r.face_y;
    let b_held = derived.face_held[1] > 0.0;
    cvars.capture_boolean(UseSprintButtonInput).map_err(Cvar)?;
    let jump = new_x || (new_a && !cvars.boolean(UseSprintButtonInput).map_err(Cvar)?);
    let sprint =
        !b_held && cvars.boolean(UseSprintButtonInput).map_err(Cvar)? && derived.face_held[0] > 0.0;
    // 1.02 824e6314..68f4: the stick steers by its side component, a latched
    // turn keeps the stick's world direction until faced, and B held off-board
    // walks in one of eight directions. Off its board the stick already arrives
    // camera-relative in the skater's frame (PlayerUI's sub_826F7668).
    let stick = [lx, 0.0, ly, 0.0];
    let frame = look_basis(phys_out.effective_anim_z, numeric).map_err(Numeric)?;
    let squared = numeric.dot3(stick, stick).map_err(Numeric)?;
    let facing = if STICK_RELEASED > squared && latch.latched {
        try_local_direction(frame, latch.direction, numeric).map_err(Numeric)?
    } else {
        stick
    };
    let side = if 0.0 > vmx_flush(facing[2]) {
        if -facing[0] <= 0.0 { -1.0 } else { 1.0 }
    } else {
        -facing[0]
    };
    let side = side * STEER_SCALE;
    let side = if latch.latched {
        side
    } else {
        let lane = vmx_flush(side);
        let sign = if lane > 0.0 {
            1.0
        } else if lane >= 0.0 {
            0.0
        } else {
            -1.0
        };
        vmx_flush(sign * lane) * side
    };
    let mut steer = clamp_signed(rx.mul_add(RIGHT_STICK_STEER, side));
    let locomotion_steer = rx.mul_add(2.0, side);
    let walk_scale = if WALK_START > vmx_flush(facing[2]) {
        0.0
    } else {
        (facing[2] - WALK_START) * WALK_RANGE
    };
    let late_left_mag = length(lx.mul_add(lx, ly * ly), numeric).map_err(Numeric)?;
    let mut magnitude = late_left_mag * walk_scale;
    let mut locomotion_magnitude = late_left_mag;
    let off_board = phys_out.physics_state == PHYSICS_STATE_OFFBOARD.cast_unsigned();
    if r.face_b && off_board {
        if !(late_left_mag <= WALK_MAGNITUDE) {
            let angle = stick_angle(-lx, -ly, numeric).map_err(Numeric)? + PI;
            sink.insert(walk_direction(angle), 1.0).map_err(Insert)?;
        }
        *latch = OffBoardStickLatch::default();
        steer = 0.0;
    }
    if !off_board {
        latch.latched = false;
    }
    let latched = latch.latched;
    if latched {
        magnitude = 1.0;
    }
    if !(steer.abs() >= LATCH_STEER) {
        latch.latched = false;
    } else if !latched {
        *latch = OffBoardStickLatch {
            direction: world_direction(frame, stick),
            latched: true,
        };
    }
    let mag_held = magnitude > 0.0;
    if !sprint && phys_out.physics_state != PHYSICS_STATE_OFFBOARD_LOCOMOTION.cast_unsigned() {
        locomotion_magnitude = if ly <= RUN_START {
            0.0
        } else {
            ((ly - RUN_START) * late_left_mag) * RUN_RANGE
        };
    }
    let (mut object_rotate, mut object_x, mut object_z) = (0.0, 0.0, 0.0);
    if left_active {
        let attributes = attributes.ok_or(MissingGlobalAttributes)?;
        let angle = stick_angle(lx, ly, numeric).map_err(Numeric)?;
        let turns = angle * f32::from_bits(0x3e22_f983);
        let fraction = turns - turns.floor();
        let wrapped =
            (fraction - if fraction > 0.5 { 1.0 } else { 0.0 }) * f32::from_bits(0x40c9_0fdb);
        let sign = select(wrapped, 1.0, -1.0);
        let curve_input = wrapped.abs() * INV_PI;
        object_rotate = (attributes
            .curve::<16>(OBJECT_ROTATE, curve_input)
            .map_err(Attribute)?
            * sign)
            * left_mag;
        object_x = attributes
            .curve::<8>(OBJECT_X, curve_input)
            .map_err(Attribute)?
            * lx;
        object_z = attributes
            .curve::<8>(OBJECT_Z, curve_input)
            .map_err(Attribute)?
            * ly;
    }
    let object_rotation = clamp_signed(object_rotate + rx);
    // 825fcb10..d3c0 (1.02 824e6b74..73ec): apart from the walk direction, no
    // intent is written until ALL preceding reads finish.
    let values = [
        (recover, "WipeOutRecover", 1.0),
        (wipeout, "WipeOutRequest", 1.0),
        (exit_grind, "ExitGrind", 1.0),
        (gesture_down, "GestureDown", 1.0),
        (gesture_left, "GestureLeft", 1.0),
        (gesture_right, "GestureRight", 1.0),
        (gesture_up, "GestureUp", 1.0),
        (left_full, "LeftGroundGrab", 1.0),
        (right_full, "RightGroundGrab", 1.0),
        (left_air, "LeftAirGrab", 1.0),
        (right_air, "RightAirGrab", 1.0),
        (r.face_b, "Brake", 1.0),
        (rx != 0.0, "WipeoutGestureX", rx),
        (
            wipeout_gesture_y != 0.0,
            "WipeoutGestureY",
            wipeout_gesture_y,
        ),
        (lx != 0.0, "WipeoutControlX", lx),
        (ly != 0.0, "WipeoutControlY", ly),
        (new_push_right || new_push_left, "NewPush", 1.0),
        (r.face_a, "RightPush", 1.0),
        (r.face_a, "Pushing", 1.0),
        (r.face_x, "LeftPush", 1.0),
        (r.face_x, "Pushing", 1.0),
        (lx != 0.0, "BodySpin", lx),
        (lx != 0.0, "PhysBodySpin", lx),
        (rx != 0.0 || ry != 0.0, "TweakX", rx),
        (rx != 0.0 || ry != 0.0, "TweakY", ry),
        (right_mag != 0.0, "BoardAdjustAngle", right_angle),
        (right_mag != 0.0, "BoardAdjustMag", right_mag),
        (lx != 0.0, "KickTurn", lx),
        (-lx != 0.0, "GrindBalanceX", -lx),
        (right_active, "AnticMag", right_mag),
        (right_active, "AnticAngle", right_angle),
        (rx != 0.0, "PhysGrindTranslation", rx),
        (lx != 0.0, "PhysGrindStabilityNudge", lx),
        (ry != 0.0, "PhysGrindUpDown", ry),
        (left_active, "AutoPumpAngle", pump_angle),
        (left_active, "AutoPumpMag", left_mag),
        (rx != 0.0, "HandPlantTweakX", rx),
        (ry != 0.0, "HandPlantTweakY", ry),
        (r.face_b, "HandPlantDismount", 1.0),
        (r.face_a, "HandPlantOneFootRight", 1.0),
        (r.face_x, "HandPlantOneFootLeft", 1.0),
        (no_input, "NoInput", 1.0),
        (hard_turn, "HardTurnCrouch", hard_crouch),
        (hard_turn, "HardTurn", hard_amount),
        (crouch, "Crouch", crouch_amount),
        (r.face_b, "Dismount", 1.0),
        (r.face_b && new_b, "NewDismount", 1.0),
        (turn != 0.0, "Turn", turn),
        (right_slide_start, "RightSlideStart", 1.0),
        (left_slide_start, "LeftSlideStart", 1.0),
        (left_slide_amount != 0.0, "LeftSlide", left_slide_amount),
        (right_slide_amount != 0.0, "RightSlide", right_slide_amount),
        (ry != 0.0, "Manual", manual_amount),
        (manual_brake, "ManualBrake", brake_amount),
        (r.right_bumper, "GrabWorld", 1.0),
        (new_toggle, "NewToggleOffBoardState", 1.0),
        (toggle, "ToggleOffBoardState", 1.0),
        (jump, "OB_Jump", 1.0),
        (sprint, "OB_Sprint", 1.0),
        (rx != 0.0, "OB_LookAtX", rx),
        (ry != 0.0, "OB_LookAtY", ry),
        (new_left_trigger, "OB_DropBoard", 1.0),
        (new_right_trigger, "OB_ThrowBoard", 1.0),
        (retrieve, "OB_RetrieveBoard", 1.0),
        (true, "OB_Steer", steer),
        (true, "OBL_Steer", locomotion_steer),
        (true, "OB_Mag", magnitude),
        (
            true,
            "OBL_Mag",
            if b_held { -1.0 } else { locomotion_magnitude },
        ),
        (true, "OB_ObjectMvZ", object_z),
        (true, "OB_ObjectMvX", object_x),
        (true, "OB_ObjectMvRot", object_rotation),
        (mag_held, "OB_MagHeld", 1.0),
    ];
    for (enabled, name, value) in values {
        if enabled {
            sink.insert(name, value).map_err(Insert)?;
        }
    }
    Ok(())
}

impl<N: std::fmt::Debug, C: std::fmt::Debug, S: std::fmt::Debug> std::fmt::Display
    for ActionInputContinuousError<N, C, S>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Numeric(error) => write!(f, "input numerical service: {error:?}"),
            Self::Cvar(error) => write!(f, "input cvar service: {error:?}"),
            Self::MissingGlobalAttributes => f.write_str("input global attributes unavailable"),
            Self::Attribute(error) => write!(f, "input attributes: {error}"),
            Self::Insert(error) => write!(f, "input intent insertion: {error:?}"),
        }
    }
}
