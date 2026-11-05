//! Completed physical publication -> both stock graphs. No state selection.
use crate::skate3_native::graph_host::{motion::MotionHost, motion_grind};
use skate3_core::{
    animation::{output::attributes::AttributeName, skeleton_input::name::encode},
    graph::conditions::PhysicalStateInputs,
    physics::filtered_state::{FilteredStateOutput, GrindState},
    player::input_phase::PhysicalPlayerInput,
};

pub(super) fn publish(
    host: &mut MotionHost,
    physical: &PhysicalPlayerInput,
    filtered: Option<&FilteredStateOutput>,
    height: f32,
    mirrored: bool,
    board_forward: [f32; 3],
) -> Result<PhysicalStateInputs, String> {
    let (state, grind, conditions) =
        observations(physical, filtered, height, mirrored, board_forward)?;
    host.grind_physical = Some(grind);
    host.grind_conditions = Some(conditions);
    Ok(state)
}

fn observations(
    physical: &PhysicalPlayerInput,
    filtered: Option<&FilteredStateOutput>,
    height: f32,
    mirrored: bool,
    board_forward: [f32; 3],
) -> Result<
    (
        PhysicalStateInputs,
        motion_grind::Physical,
        motion_grind::conditions::Physical,
    ),
    String,
> {
    let (grinding, grind) = match filtered {
        Some(output) => {
            if output.category as u32 != physical.filtered_state_0 {
                return Err("Grind graph received inconsistent completed filtered outputs".into());
            }
            (output.grinding, output.grind)
        }
        // FilteredState reset82DE5588: category0, null names and crouch0.
        // This is the initial reset publication, not a substitute grind state.
        None if physical.filtered_state_0 == 0 => (false, GrindState::default()),
        None => return Err("Grind graph requires the completed filtered state owner".into()),
    };
    let grind_name = name_text(grind.name)?;
    let grind_observation = motion_grind::Physical {
        grinding,
        grind_name: grind.name,
        // Bundle4 is Motion, not Ground. Basic128 is supplied by the
        // completed BoardMotionOutput effective basis, not Motion128.
        ground_axis: xyz(physical.skateboard.vector_80),
        board_axis: board_forward,
        ground_flag_273: physical.ground.flag_273 != 0,
        // Execution refreshes bit30 for the two branches which consume it.
        animation_mirrored: mirrored,
        height,
        crouch: grind.crouch,
        twist: physical.skeleton.twist_504,
    };
    let conditions = motion_grind::conditions::Physical {
        filtered_grinding_80: grinding,
        blunting_136: physical.grinds.words_136_140[0],
        approach_268: physical.grinds.animation_chromosome_268[0],
        trick_out_240: physical.grinds.trick_out_240,
        air_grind_443: physical.air.flag_443 != 0,
        air_time_184: physical.air.scalar_184,
        dropping_in_324: physical.grinds.dropping_in_324 != 0,
    };
    Ok((
        PhysicalStateInputs {
            category: physical.filtered_state_0,
            grinding,
            grind_name,
        },
        grind_observation,
        conditions,
    ))
}

fn xyz(raw: [u32; 4]) -> [f32; 3] {
    [raw[0], raw[1], raw[2]].map(f32::from_bits)
}

/// Lossless adapter for CURRENT's string-based IsGrinding boundary. Stock grind
/// short names use only the native base38 alphabet 0-9,A-Z,_. This reverses
/// the existing encoder, then validates its round trip; it chooses no name.
fn name_text(name: AttributeName) -> Result<String, String> {
    let mut text = String::new();
    let mut ended = false;
    for mut word in name.0 {
        for weight in [79_235_168, 2_085_136, 54_872, 1_444, 38, 1] {
            let digit = word / weight;
            word %= weight;
            if digit == 0 {
                ended = true;
                continue;
            }
            if ended {
                return Err("Filtered grind name contains a nonterminal NUL".into());
            }
            let byte = match digit {
                1..=10 => b'0' + (digit as u8 - 1),
                11..=36 => b'A' + (digit as u8 - 11),
                37 => b'_',
                _ => return Err("Filtered grind name is outside the stock alphabet".into()),
            };
            text.push(byte as char);
        }
    }
    if encode(text.as_bytes()) != name {
        return Err("Filtered grind name did not round trip".into());
    }
    Ok(text)
}

#[cfg(any())]
#[path = "animation_grind_tests.rs"]
mod tests;
