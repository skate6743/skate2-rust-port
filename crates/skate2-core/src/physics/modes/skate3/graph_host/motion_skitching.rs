//! The TU3 skitching leaves of the MotionGraph: EnterSkitchingBehaviour (Begin 0x82BBC538,
//! Update 0x82BBC5E0), SkitchingBehaviour (Begin 0x82BBC2D8, Update 0x82BBC390),
//! SkitchShimmyingBehaviour (Begin 0x82BBC7C8, Update 0x82BBC858, End 0x82BBCAB8) and the
//! conditions IsSkitchingWithAbsorb 0x82BBBD00, SkitchingPosition 0x82BBC068 and
//! IsSkitchShimmying 0x82BBC1A0. All read the anim_skitching default collection.
use super::motion_animation::MotionAnimation;
use skate3_core::{
    animation::{
        channel_playback::ChannelSettings,
        playback_parameters::{AttributeSink, SettableAttribute},
        skeleton_input::name::encode,
    },
    point_graph::PointGraph,
};
use skate_data::collections::Collections;

const LEFT_CHANNEL: &str = "SKCH_2H_SHIMMY_LEFT_CHANNEL";
const RIGHT_CHANNEL: &str = "SKCH_2H_SHIMMY_RIGHT_CHANNEL";

/// anim_skitching/default: the absorb graph (layout 0), the reach window (layout 80 and 96),
/// the shimmy speed range and step (layout 84, 88 and 92) and the crouch offset (layout 100).
pub(crate) struct Settings {
    absorb: PointGraph<8>,
    short_reach: f32,
    long_reach: f32,
    crouch_offset: f32,
    shimmy_minimum: f32,
    shimmy_maximum: f32,
    shimmy_delta: f32,
}

impl Settings {
    /// A missing field reads as the zero default data, as Attrib::Collection::GetData does.
    pub(crate) fn load(data: &Collections) -> Self {
        let float = |key: &str| data.float("anim_skitching", "default", key).unwrap_or(0.0);
        let absorb = data
            .words::<20>("anim_skitching", "default", "0x1C04030E5B00A8EA")
            .unwrap_or([0; 20]);
        Self {
            absorb: PointGraph {
                x: std::array::from_fn(|i| f32::from_bits(absorb[4 + i])),
                y: std::array::from_fn(|i| f32::from_bits(absorb[12 + i])),
            },
            short_reach: float("0x8738E31E171E2A87"),
            long_reach: float("0xBDFD21F2A96828FD"),
            crouch_offset: float("0x6E6EFE7CF9BEF4B8"),
            shimmy_minimum: float("0xB2D3A764E1D86D13"),
            shimmy_maximum: float("0xF146CFA3B03529A5"),
            shimmy_delta: float("0x1C7DF104E24101FF"),
        }
    }
}

/// The completed PhysOut values these leaves read.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Physical {
    pub state: u32,
    /// Animation +132/+136/+140.
    pub crouch: f32,
    pub hand_speed: f32,
    pub grip_hand: i32,
    /// Ground +276/+280/+284/+308.
    pub time_to_skitch: f32,
    pub spline_height: f32,
    pub contact_speed: f32,
    pub push_speed: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Enter {
    initial_crouch: f32,
    elapsed: f32,
    reach_blend: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Holding {
    initial_crouch: f32,
    absorb_speed: f32,
}

fn fsel(test: f32, positive: f32, negative: f32) -> f32 {
    if test >= 0.0 { positive } else { negative }
}

fn limit(value: f32, previous: f32, delta: f32) -> f32 {
    let minimum = previous - delta;
    let maximum = previous + delta;
    let raised = fsel(minimum - value, minimum, value);
    fsel(maximum - raised, raised, maximum)
}

fn set(animation: &mut MotionAnimation, name: &[u8], value: f32) {
    animation.set_attribute(SettableAttribute {
        name: encode(name),
        value,
        normalized: false,
        sequence_id: -1,
    });
}

fn physical(physical: Option<Physical>) -> Result<Physical, String> {
    physical.ok_or_else(|| "Skitching leaves require the completed PhysOut".into())
}

/// EnterSkitchingBehaviour: Begin keeps the crouch and resets the reach blend; Update writes
/// Crouch (spline height plus the offset) and reachspeed from the time left to the spline.
pub(crate) fn enter(
    state: &mut Enter,
    phase: u8,
    animation: &mut MotionAnimation,
    input: Option<Physical>,
    settings: &Settings,
) -> Result<(), String> {
    match phase {
        0 => {
            let p = physical(input)?;
            *state = Enter {
                initial_crouch: p.crouch,
                elapsed: 0.0,
                reach_blend: -1.0,
            };
        }
        1 => {
            let p = physical(input)?;
            let crouch = p.spline_height + settings.crouch_offset;
            let time = animation.current_time()?;
            let length = animation.current_length()?;
            let remaining = 1.0 - time / length;
            let reach_time = if remaining > 0.0 {
                p.time_to_skitch / remaining
            } else {
                length
            };
            let span = settings.long_reach - settings.short_reach;
            let value = reach_time - settings.short_reach;
            let raised = fsel(-value, 0.0, value);
            let mut blend = fsel(span - raised, raised, span) / span;
            // 0x82BBC734 bge: an unordered or non-negative previous blend limits the step.
            if !(state.reach_blend < 0.0) {
                blend = limit(blend, state.reach_blend, f32::from_bits(0x3dcc_cccd));
            }
            state.reach_blend = blend;
            set(animation, b"Crouch", crouch);
            set(animation, b"reachspeed", 1.0 - blend);
        }
        _ => {}
    }
    Ok(())
}

/// SkitchingBehaviour: Begin keeps the crouch and the absorb target; Update writes Crouch,
/// PushSpeed and absorbspeed, the latter moving at most 0.04 a tick toward the absorb graph.
pub(crate) fn holding(
    state: &mut Holding,
    phase: u8,
    animation: &mut MotionAnimation,
    input: Option<Physical>,
    settings: &Settings,
) -> Result<(), String> {
    match phase {
        0 => {
            let p = physical(input)?;
            state.initial_crouch = p.crouch;
            state.absorb_speed = settings.absorb.evaluate(p.contact_speed);
        }
        1 => {
            let p = physical(input)?;
            let crouch = p.spline_height + settings.crouch_offset;
            let target = settings.absorb.evaluate(p.contact_speed);
            state.absorb_speed = limit(target, state.absorb_speed, f32::from_bits(0x3d23_d70a));
            set(animation, b"Crouch", crouch);
            set(animation, b"PushSpeed", p.push_speed);
            set(animation, b"absorbspeed", state.absorb_speed);
        }
        _ => {}
    }
    Ok(())
}

/// SkitchShimmyingBehaviour, direction 1 left and 2 right: the hands' speed normalized over the
/// authored range drives ShimmySpeed on that side's looping channel; End fades both out.
pub(crate) fn shimmy(
    speed: &mut f32,
    direction: u8,
    phase: u8,
    animation: &mut MotionAnimation,
    input: Option<Physical>,
    settings: &Settings,
) -> Result<(), String> {
    match phase {
        0 => *speed = physical(input)?.hand_speed,
        1 => {
            let hand = physical(input)?.hand_speed;
            // 0x82BBC910..930 clamps against the current speed, not the retained one.
            let maximum = settings.shimmy_delta + hand;
            let minimum = hand - settings.shimmy_delta;
            let bounded = fsel(minimum - hand, minimum, hand);
            let bounded = fsel(maximum - bounded, bounded, maximum).abs();
            let bounded = fsel(settings.shimmy_minimum - bounded, settings.shimmy_minimum, bounded);
            let bounded = fsel(settings.shimmy_maximum - bounded, bounded, settings.shimmy_maximum);
            *speed = if bounded > settings.shimmy_minimum {
                (bounded - settings.shimmy_minimum) / (settings.shimmy_maximum - settings.shimmy_minimum)
            } else {
                0.0
            };
            let (wanted, other) = if direction == 1 {
                (LEFT_CHANNEL, RIGHT_CHANNEL)
            } else {
                (RIGHT_CHANNEL, LEFT_CHANNEL)
            };
            let present = animation.channels.has(wanted);
            if animation.channels.has(other) {
                animation.channels.end(other);
            }
            if !present {
                animation.new_channel(
                    wanted,
                    wanted,
                    ChannelSettings {
                        priority: 0,
                        keep_alive: true,
                        mirrored: false,
                        speed: 1.0,
                        blend_in: f32::from_bits(0x3e4c_cccd),
                        hold_during_blend_in: false,
                        blend_out: f32::from_bits(0x3e4c_cccd),
                        hold_during_blend_out: false,
                        use_attributes: false,
                    },
                )?;
            }
            set(animation, b"ShimmySpeed", *speed);
        }
        _ => {
            for channel in [LEFT_CHANNEL, RIGHT_CHANNEL] {
                animation.channels.end_with(channel, f32::from_bits(0x3e99_999a), false);
            }
        }
    }
    Ok(())
}

/// IsSkitchingWithAbsorb: Skitching, and the absorb graph positive at the contact speed.
pub(crate) fn with_absorb(input: Option<Physical>, settings: &Settings) -> Result<bool, String> {
    let p = physical(input)?;
    Ok(p.state == 104 && settings.absorb.evaluate(p.contact_speed) > 0.0)
}

/// SkitchingPosition, desired 1 front, 2 back, 3 both: the grip hand, flipped by the
/// PhysOutAnimation +158/+157 stance parity.
pub(crate) fn position(
    input: Option<Physical>,
    stance: Option<(bool, bool)>,
    desired: u8,
) -> Result<bool, String> {
    let p = physical(input)?;
    let (a, b) = stance.ok_or("SkitchingPosition requires the PhysOut stance")?;
    let parity = a == b;
    let side = match p.grip_hand {
        1 => u8::from(parity) + 1,
        2 => u8::from(!parity) + 1,
        _ => 3,
    };
    Ok(side == desired)
}

/// IsSkitchShimmying, desired 1 left, 2 right, 0 still: the hands' speed past 0.25 either way,
/// negated for a mirrored skater.
pub(crate) fn shimmying(input: Option<Physical>, mirrored: Option<bool>, desired: u8) -> Result<bool, String> {
    let p = physical(input)?;
    let mirrored = mirrored.ok_or("IsSkitchShimmying requires the mirrored stance")?;
    let speed = if mirrored { -1.0 } else { 1.0 } * p.hand_speed;
    let direction = if speed > 0.25 {
        1
    } else if speed < -0.25 {
        2
    } else {
        0
    };
    Ok(direction == desired)
}
