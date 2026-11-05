//! Stock ControlAirLegExtension configuration82BAF450 and settings Globals336.
use skate3_core::{
    animation::{
        air_leg_extension::{Bone, Settings},
        output::attributes::AttributeName,
        skeleton_input::name::encode,
    },
    point_graph::PointGraph,
};
use skate_data::{collections::Collections, state_graph::attributes::Attributes};

#[derive(Clone, Debug, PartialEq)]
pub struct Operation {
    pub height: AttributeName,
    pub bone: Bone,
}
impl Operation {
    pub fn parse(a: &Attributes<'_>) -> Self {
        Self {
            height: encode(a.text("driveDistToCom").unwrap_or("disttocog").as_bytes()),
            bone: match a.text("bone").unwrap_or("Skateboard_Root") {
                "LeftToeBase" => Bone::LeftToe,
                "RightToeBase" => Bone::RightToe,
                _ => Bone::Board,
            },
        }
    }
}
pub fn load(data: &Collections) -> Result<Settings, String> {
    let value = |name| data.float("anim_motion", "inair_disttocog", name);
    //PointNegGraphData4 includes four metadata words before x/y. Update uses
    //82481E10 on the four authored knots, not the negative-graph evaluator.
    let w = data.words::<12>("anim_motion", "inair_disttocog", "lo_air_arm_extend")?;
    Ok(Settings {
        going_up_speed: value("goingup_disttocog_speed")?,
        going_down_speed: value("goingdown_disttocog_speed")?,
        minimum_height: value("min_inair_disttocog")?,
        preland_velocity: value("preland_disttocom_vel")?,
        preland_final_height: value("preland_final_disttocom")?,
        offboard_multiplier: value("offboard_preland_disttocom_vel_multiplier")?,
        arm_extension: PointGraph {
            x: std::array::from_fn(|i| f32::from_bits(w[4 + i])),
            y: std::array::from_fn(|i| f32::from_bits(w[8 + i])),
        },
    })
}
