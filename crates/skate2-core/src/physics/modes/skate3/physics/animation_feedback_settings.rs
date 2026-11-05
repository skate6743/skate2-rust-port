//! PhysOutConditioner_Animation turn settings, TU3 global204/physics_animation.
use skate3_core::{input::turn_conditioner::Settings, point_graph::PointGraph};
use skate_data::collections::Collections;
pub(crate) fn load(data: &Collections) -> Result<Settings, String> {
    let coefficients = |name| {
        data.words::<4>("physics_animation", "default", name)
            .map(|w| w.map(f32::from_bits))
    };
    let plain = |name| -> Result<PointGraph<8>, String> {
        let w = data
            .words::<16>("physics_animation", "default", name)?
            .map(f32::from_bits);
        Ok(PointGraph {
            x: w[..8].try_into().unwrap(),
            y: w[8..].try_into().unwrap(),
        })
    };
    let speed = data
        .words::<20>("physics_animation", "default", "ScaleQuicknessAtSpeed")?
        .map(f32::from_bits);
    let smoothing = data
        .words::<12>("physics_animation", "default", "SpeedWobbleTurnBlend")?
        .map(f32::from_bits);
    let mut parameters = [0.0; 13];
    for (target, name) in parameters.iter_mut().zip([
        "QuicknessMaxDelta",
        "InputQuicknessMax",
        "InputQuicknessBlendVal",
        "HoldingMinVal",
        "HoldingMaxAcceleration",
        "HoldingIntegralBlend",
        "HoldingBlendUp",
        "HoldingBlendDown",
        "AnimationTurnWobbleScalar",
        "AnimationTurnPhysicsWeight",
        "AnimationTurnMaxLean",
        "AnimationTurnGraphX",
        "AnimationTurnDirMax",
    ]) {
        *target = data.float("physics_animation", "default", name)?;
    }
    Ok(Settings {
        filter_coefficients: [
            coefficients("AnimationTurnLeanFilterSlow")?,
            coefficients("AnimationTurnLeanFilter")?,
            coefficients("AnimationTurnDirFilter")?,
        ],
        input_curve: plain("AnimationTurnSpeedGraph")?,
        quickness_curve: plain("QuicknessVsQuickness")?,
        speed_curve: PointGraph {
            x: speed[4..12].try_into().unwrap(),
            y: speed[12..20].try_into().unwrap(),
        },
        smoothing_curve: PointGraph {
            x: smoothing[4..8].try_into().unwrap(),
            y: smoothing[8..12].try_into().unwrap(),
        },
        parameters,
    })
}
