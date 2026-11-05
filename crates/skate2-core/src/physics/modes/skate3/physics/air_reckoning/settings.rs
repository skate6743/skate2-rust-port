//! Stock classes are verified against original8289D180 and828A0054..00A0:
//! physics_reckoning7CE1B6B581709C6F, physics_bodyspin05EC9CB03A5B2EEE.
use skate3_core::{
    air::{body_flip::BodyFlipSettings, body_spin::BodySpinSettings, reckoning::Settings},
    point_graph::PointGraph,
};
use skate_data::collections::Collections;

#[derive(Clone, Copy)]
pub(super) struct Mode {
    pub easy_body_spins: bool,
    pub perfect_body_flips: bool,
}
pub(super) fn load(data: &Collections) -> Result<(Settings, [Mode; 5]), String> {
    let float = |class, field| data.float(class, "default", field);
    let control = data
        .words::<4>("physics_reckoning", "default", "GroundNormalSmoothing")?
        .map(f32::from_bits);
    let mut modes = Vec::with_capacity(5);
    //Player ctor82DB1B08..1B78 installs these exact physics_mode keys.
    for name in ["easy", "normal", "hardcore", "motorized", "test"] {
        modes.push(Mode {
            easy_body_spins: data.boolean("physics_mode", name, "EasyBodySpins")?,
            perfect_body_flips: data.boolean("physics_mode", name, "PerfectBodyFlips")?,
        });
    }
    Ok((
        Settings {
            ground_normal_smoothing: control,
            max_up_angle_delta: negative_graph(
                data,
                "physics_reckoning",
                "Air_MaxUpVectAngleDelta",
            )?,
            tilt_vs_rotation: graph(data, "TiltVsRotAir")?,
            tilt_vs_slope: graph(data, "TiltVsSlopeAir")?,
            body_spin: BodySpinSettings {
                derivative_floor: float("physics_bodyspin", "MinDerivativeScalar")?,
                acceleration_limit: float("physics_bodyspin", "MaxDeltaOppositeDirection")?,
                //Original82D8BE38 copies layouts160,240,320,480,400,80,0.
                curves: [
                    negative_graph(data, "physics_bodyspin", "Hash_BEA30B5DC6AFF26A")?,
                    negative_graph(data, "physics_bodyspin", "MaxDeltaVsTime")?,
                    negative_graph(data, "physics_bodyspin", "MaxDeltaForAutoVsTime")?,
                    negative_graph(data, "physics_bodyspin", "Hash_3974F0228331EB22")?,
                    negative_graph(data, "physics_bodyspin", "DerivativeBodySpinVsTime")?,
                    negative_graph(data, "physics_bodyspin", "Hash_D7C6855B7814D048")?,
                    negative_graph(data, "physics_bodyspin", "PropBodySpinVsTime")?,
                ],
                //82F82690 splats original821647E0 into830BD300.
                input_fade_threshold: [f32::from_bits(0x3780_0000); 4],
            },
            body_flip: BodyFlipSettings {
                smoothing: Some(float("physics_reckoning", "FlipSpeedSmoothingFactor")?),
                maximum_speed: Some(float("physics_reckoning", "FlipMaxSpeed")?),
                spin_scale: Some(float("physics_reckoning", "FlipBodySpinScalar")?),
                //All three attributes are mandatory above; no missing-attribute
                //branch is reachable through this production settings loader.
                missing_attribute_value: 0.0,
            },
        },
        modes
            .try_into()
            .map_err(|_| "Expected five physics modes".to_owned())?,
    ))
}
fn negative_graph(data: &Collections, class: &str, name: &str) -> Result<PointGraph<8>, String> {
    let values = data
        .words::<20>(class, "default", name)?
        .map(f32::from_bits);
    //Source directly calls PointGraphEval on +16/+48, ignoring sign metadata.
    Ok(PointGraph {
        x: values[4..12].try_into().unwrap(),
        y: values[12..20].try_into().unwrap(),
    })
}
fn graph(data: &Collections, name: &str) -> Result<PointGraph<8>, String> {
    let values = data
        .words::<16>("physics_reckoning", "default", name)?
        .map(f32::from_bits);
    Ok(PointGraph {
        x: values[..8].try_into().unwrap(),
        y: values[8..].try_into().unwrap(),
    })
}
