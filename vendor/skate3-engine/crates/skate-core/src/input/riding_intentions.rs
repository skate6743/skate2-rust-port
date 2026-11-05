//! Push, brake, body-spin, steering, kick-turn, crouch, powerslide and world-grab intents
//! from the TU3 ActionGraph input listener (`825999F0`).
use super::{
    angle::left_stick_angle,
    controller::{DerivedControllerInput, magnitude},
};

#[derive(Clone, Copy, Debug, Default)]
pub struct PushPreferences {
    /// PhysOut+0x34 (the Ground intents record) byte 58: automatic push below the mode's speed.
    pub automatic_push_enabled: bool,
    /// PhysOut+0x38 (PhysOutAnimation) byte 156, the animation mirror: the automatic push's foot.
    pub automatic_push_right: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RidingIntent {
    pub name: &'static str,
    pub value: f32,
}

/// Consumed-observation boundary for the automatic-push branch825999F0.
/// The caller supplies its actual normalized stick and established push/lock
/// observations; this function owns no controller history or preferences.
pub fn automatic_push(
    left: [f32; 2],
    manual_push: bool,
    allowed: bool,
    preferences: PushPreferences,
) -> Option<bool> {
    let angle = left_stick_angle(left[0], left[1]);
    let from_forward = if angle >= 0. {
        f32::from_bits(0x40490fdb) - angle
    } else {
        -f32::from_bits(0x40490fdb) - angle
    };
    let length = magnitude(left[1].mul_add(left[1], left[0] * left[0]));
    choose_automatic_push(from_forward, length, manual_push, allowed, preferences)
}
fn choose_automatic_push(
    from_forward: f32,
    length: f32,
    manual_push: bool,
    allowed: bool,
    preferences: PushPreferences,
) -> Option<bool> {
    if preferences.automatic_push_enabled
        && allowed
        && !manual_push
        && from_forward.abs() < 0.6
        && length > 0.8
    {
        Some(preferences.automatic_push_right)
    } else {
        None
    }
}

/// TU3 Fill 8259A6C0..A75C, emitted at 8259ADC8 as `NoInput` 1.0: no stick, push, trigger,
/// brake (bit 20), right bumper (bit 28) or board toggle (bit 22 without bit 29, off a locked air
/// trajectory). `pushing` is any final push, new or held, after the automatic push and the
/// actor's simultaneous-push suppression. The motion graph attaches the intent as the
/// `NoInput` attribute, which runs the coasting friction's no-input timer.
pub fn no_input(controller: &DerivedControllerInput, pushing: bool, air_reckoning_active: bool) -> bool {
    let words = controller.words();
    let axis = |slot: usize| f32::from_bits(words[slot]);
    let held = |bit: u32| words[13] & (1u32 << bit) != 0;
    let toggle = !air_reckoning_active && !held(29) && held(22);
    !(axis(7) != 0.0
        || axis(8) != 0.0
        || axis(9) != 0.0
        || axis(10) != 0.0
        || held(20)
        || pushing
        || axis(11) == 1.0
        || axis(12) == 1.0
        || axis(11) > 0.0
        || axis(12) > 0.0
        || held(28)
        || toggle)
}

pub fn produce(
    controller: &DerivedControllerInput,
    actor_flags: u32,
    preferences: PushPreferences,
) -> Vec<RidingIntent> {
    let words = controller.words();
    let previous = words[6];
    let current = words[13];
    let left = [f32::from_bits(words[7]), f32::from_bits(words[8])];
    let allowed = actor_flags & (1 << 11) == 0;
    let held = |bit: u32| current & (1u32 << bit) != 0 && allowed;
    let new = |bit: u32, timer: usize| {
        held(bit) && (previous & (1u32 << bit) == 0 || f32::from_bits(words[timer]) < 0.3)
    };
    let mut right = held(21);
    let mut left_push = held(23);
    let mut new_right = new(21, 20);
    let mut new_left = new(23, 22);
    let angle = left_stick_angle(left[0], left[1]);
    let from_forward = if angle >= 0.0 {
        f32::from_bits(0x40490fdb) - angle
    } else {
        -f32::from_bits(0x40490fdb) - angle
    };
    let length = magnitude(left[1].mul_add(left[1], left[0] * left[0]));
    if let Some(push_right) = choose_automatic_push(
        from_forward,
        length,
        right || left_push || new_right || new_left,
        allowed,
        preferences,
    ) {
        if push_right {
            right = true;
            new_right = true;
        } else {
            left_push = true;
            new_left = true;
        }
    }
    if actor_flags & (1 << 6) != 0 && (right || new_right) && (left_push || new_left) {
        right = false;
        left_push = false;
        new_right = false;
        new_left = false;
    }
    let mut output = Vec::with_capacity(20);
    let mut emit = |name, value| output.push(RidingIntent { name, value });
    if current & (1 << 20) != 0 && actor_flags & (1 << 7) == 0 {
        emit("Brake", 1.0);
    }
    //8259A214..A2C8, emitted at 8259AAA4: a new press of either push button or the brake.
    let rising = |bit: u32| current & (1u32 << bit) != 0 && previous & (1u32 << bit) == 0;
    if rising(23) || rising(21) || rising(20) {
        emit("ExitGrind", 1.0);
    }
    //8259A510..A53C, emitted at 8259AD94: the left stick's angle from forward and its
    //magnitude drive the stick pump, unless the automatic push owns the stick.
    if !(preferences.automatic_push_enabled && allowed) && (left[0] != 0.0 || left[1] != 0.0) {
        emit("AutoPumpAngle", from_forward);
        emit("AutoPumpMag", length);
    }
    if new_right || new_left {
        emit("NewPush", 1.0);
    }
    if right {
        emit("RightPush", 1.0);
        emit("Pushing", 1.0);
    }
    if left_push {
        emit("LeftPush", 1.0);
        emit("Pushing", 1.0);
    }
    //8259AC1C..AC54 emits this ordered pair from current left X. Actor1908
    //bit1 gates both; steering's bit0 and the physical state do not gate it.
    //BodySpin's later animation-attribute handler owns its sign conversion.
    if left[0] != 0.0 && actor_flags & (1 << 1) == 0 {
        emit("BodySpin", left[0]);
        emit("PhysBodySpin", left[0]);
    }
    //8259ACDC..ACE8 selects KickTurn from current left X, not shaped Turn.
    if left[0] != 0.0 && actor_flags & 1 == 0 {
        emit("KickTurn", left[0]);
    }
    let steering = super::steering_intentions::produce(left, actor_flags);
    if let Some(value) = steering.hard_turn_crouch {
        emit("HardTurnCrouch", value);
    }
    if let Some(value) = steering.hard_turn {
        emit("HardTurn", value);
    }
    //8259A398..A424, emission key8259AE30..AE3C: the stick-based crouch ramp is
    //multiplied by literal zero (f24). The two fsel instructions select the
    //MAXIMUM of that result and the two trigger values, not a subtraction.
    //An active gate can therefore intentionally emit Crouch with value zero.
    let absolute = angle.abs();
    let start = f32::from_bits(0x3ff5_c28f); //1.92
    let end = f32::from_bits(0x4016_6666); //2.35
    let stick_crouch = if absolute <= start {
        0.0
    } else if absolute < end {
        ((absolute - start) * f32::from_bits(0x4014_d655)) * length
    } else {
        length
    } * 0.0;
    let [left_trigger, right_trigger] = [words[11], words[12]].map(f32::from_bits);
    if absolute > start || left_trigger > 0.0 || right_trigger != 0.0 {
        let trigger = if left_trigger - right_trigger >= -0.0 {
            left_trigger
        } else {
            right_trigger
        };
        emit(
            "Crouch",
            if stick_crouch - trigger >= -0.0 {
                stick_crouch
            } else {
                trigger
            },
        );
    }
    if let Some(value) = steering.turn {
        emit("Turn", value);
    }

    //8259A430..A50C: start queries and continuous values have distinct open
    //heading windows. Both continuous descriptors can be present together.
    //8259AECC..AF5C gates only actor bit8, independently of push inhibition.
    if actor_flags & (1 << 8) == 0 && length > 0.89999998 {
        const SLIDE_SCALE: f32 = 0.28004956;
        const HALF_PI: f32 = f32::from_bits(0x3fc90fdb);
        if angle > 0.0 && angle < 0.91000003 {
            emit("RightSlideStart", 1.0);
        }
        if angle > -0.91000003 && angle < 0.0 {
            emit("LeftSlideStart", 1.0);
        }
        if angle > -2.0 && angle < HALF_PI {
            emit("LeftSlide", -((angle + 2.0) * SLIDE_SCALE));
        }
        if angle > -HALF_PI && angle < 2.0 {
            emit("RightSlide", (2.0 - angle) * SLIDE_SCALE);
        }
    }
    //8259A54C/554 reads current raw flags bit28;8259AF68..7C emits
    //GrabWorld (830BE780, named by initializer82F84EB0) while held.
    //No actor inhibition or rising-edge gate applies to this intention.
    if current & (1 << 28) != 0 {
        emit("GrabWorld", 1.0);
    }
    output
}

#[cfg(test)]
#[path = "riding_intentions_tests.rs"]
mod tests;

#[cfg(test)]
mod automatic_push_tests {
    use super::*;
    #[test]
    fn automatic_push_observes_forward_manual_and_actor_gates_for_both_feet() {
        for right in [false, true] {
            let p = PushPreferences {
                automatic_push_enabled: true,
                automatic_push_right: right,
            };
            assert_eq!(automatic_push([0., 1.], false, true, p), Some(right));
            for stick in [[0., 0.], [0., -1.], [1., 0.], [0., 0.7], [f32::NAN, 1.]] {
                assert_eq!(automatic_push(stick, false, true, p), None);
            }
            assert_eq!(automatic_push([0., 1.], true, true, p), None);
            assert_eq!(automatic_push([0., 1.], false, false, p), None);
            assert_eq!(
                automatic_push(
                    [0., 1.],
                    false,
                    true,
                    PushPreferences {
                        automatic_push_enabled: false,
                        ..p
                    }
                ),
                None
            );
            let mut words = [0; 26];
            words[8] = 1f32.to_bits();
            words[13] = 1 << if right { 23 } else { 21 };
            let manual = produce(&DerivedControllerInput::from_words(words), 0, p);
            let has = |name| manual.iter().any(|i| i.name == name);
            assert!(has(if right { "LeftPush" } else { "RightPush" }));
            assert!(!has(if right { "RightPush" } else { "LeftPush" }));
        }
    }
}
