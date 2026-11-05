//! Original TU3 low-speed KickTurn behaviors, connected to MotionHost.
use super::motion_animation::MotionAnimation;
use skate3_core::{
    animation::{kickturn, playback_parameters::ParameterInputs, skeleton_input::name::encode},
    point_graph::PointGraph,
};
use skate_data::{collections::Collections, state_graph::attributes::Attributes};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Operation {
    ResetTimer,
    Steering(kickturn::Parameters),
}

impl Operation {
    pub fn parse(a: &Attributes<'_>) -> Option<Self> {
        match a.text("name")? {
            "ResetTimeSinceKickturn" => Some(Self::ResetTimer),
            //82BC7218: both constructor defaults are literal8209975C=0.5.
            "KickTurnSteering" => Some(Self::Steering(kickturn::Parameters {
                max_height: f32::from_bits(a.float_bits("maxHeight", 0x3f000000)),
                spin_scale: f32::from_bits(a.float_bits("spinScale", 0x3f000000)),
            })),
            _ => None,
        }
    }
}

pub fn load_settings(data: &Collections) -> Result<kickturn::Settings, String> {
    //Original8289FB90..FBB0 binds Globals324 to A168897919752875,
    //the independently verified stock anim_motion/kickturn collection key.
    let curve = |name| -> Result<PointGraph<8>, String> {
        let words = data.words::<20>("anim_motion", "kickturn", name)?;
        Ok(PointGraph {
            x: std::array::from_fn(|i| f32::from_bits(words[4 + i])),
            y: std::array::from_fn(|i| f32::from_bits(words[12 + i])),
        })
    };
    Ok(kickturn::Settings {
        spin: curve("kickturn_spin")?,      //layout240, source X256/Y288
        height: curve("kickturn_balance")?, //layout320, X336/Y368
    })
}

pub fn execute(
    state: &mut kickturn::State,
    parameters: kickturn::Parameters,
    settings: &kickturn::Settings,
    animation: &mut MotionAnimation,
    dt: f32,
    phase: u8,
) -> Result<(), String> {
    if phase == 0 {
        state.begin(settings);
        return Ok(());
    }
    //Native End82B61BB8 is the no-op lifecycle method.
    if phase != 1 {
        return Ok(());
    }
    if state.needs_animation_length() {
        //82BAE51C..560: IAnimatable84 applies pending parameters through
        //82D18F98, then v48 supplies the tree for GetLength(v20).
        animation.apply_parameters()?;
        state.capture_animation_length(animation.current_length()?);
    }
    //Original830BE1E0 initializer82F84BC8 identifies KickTurn. HasIntent's
    //destination is initialized to0; an absent/zero intent retains the sign.
    let output = state.update(
        dt,
        animation.motion_intent("KickTurn").unwrap_or(0.0),
        parameters,
        settings,
    );
    //MGspecific v16=8258F6E0 appends attributes, not animation parameters.
    //82F85A08/82F858D0 identify the two original names and this order.
    animation.emit_packet(encode(b"Balance"), output.balance);
    animation.emit_packet(encode(b"Spin"), output.spin);
    Ok(())
}
