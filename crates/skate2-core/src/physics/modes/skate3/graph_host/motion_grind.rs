//! Original S3 grind graph lifecycle, using the current animation owner.
//! No animation choice, physical-state transition or synthetic observation.
pub mod animation;
pub mod conditions;
mod settings;
pub use settings::Settings;
#[cfg(any())]
mod tests;

use animation::{Animation, endpoints, set};
use skate3_core::animation::{
    grind_control::{Fade, crouch, facing, mirrored_twist},
    output::attributes::AttributeName,
    skeleton_input::name::encode,
};
use skate_data::state_graph::attributes::Attributes;

#[derive(Clone, Debug, PartialEq)]
pub enum Operation {
    Attributes,
    Crouch {
        height: AttributeName,
    },
    Fade {
        height: AttributeName,
        twist: AttributeName,
        intent: String,
    },
}
impl Operation {
    pub fn parse(a: &Attributes<'_>) -> Result<Option<Self>, String> {
        let required = |name| {
            a.text(name)
                .ok_or_else(|| format!("GrindControlFade requires {name}"))
        };
        Ok(Some(match a.text("name") {
            Some("CreateGrindAttributes") => Self::Attributes,
            Some("ControlGrindCrouch") => Self::Crouch {
                height: encode(a.text("driveDistToCom").unwrap_or("disttocog").as_bytes()),
            },
            Some("GrindControlFade") => Self::Fade {
                height: encode(required("distBoardToCogAnimAttribute")?.as_bytes()),
                twist: encode(required("twistAnimAttribute")?.as_bytes()),
                intent: required("twistMGIntent")?.into(),
            },
            _ => return Ok(None),
        }))
    }
}

/// Per behavior instance, never shared between different graph nodes.
#[derive(Default)]
pub struct State {
    fade: Option<Fade>,
    height: Option<f32>,
}

/// Required genuine physical fields. No Default: an unavailable producer must
/// not silently become a fabricated all-zero observation.
#[derive(Clone, Copy, Debug)]
pub struct Physical {
    /// Filtered bundle64 byte80 and five-word encoded name at+8.
    pub grinding: bool,
    pub grind_name: AttributeName,
    /// Motion bundle4 vector80 (deck velocity), Basic bundle0 vector128
    /// (completed effective board forward). Historical field name retained.
    pub ground_axis: [f32; 3],
    pub board_axis: [f32; 3],
    /// Motion273: Processed2468 bit20, not a ground-contact flag.
    pub ground_flag_273: bool,
    /// Live animation mirror, not a cached physical stance.
    pub animation_mirrored: bool,
    /// Animation bundle56 float72, Filtered bundle64 float56.
    pub height: f32,
    pub crouch: f32,
    /// RAW Skeleton bundle20 float504; this handler mirrors it on Begin.
    pub twist: f32,
}

/// phase0=Begin,1=Update,2=End. Graph execution owns lifecycle/order.
pub fn execute(
    state: &mut State,
    op: &Operation,
    phase: u8,
    dt: f32,
    settings: &Settings,
    physical: &Physical,
    animation: &mut impl Animation,
) -> Result<(), String> {
    match op {
        Operation::Attributes if phase == 1 && physical.grinding => {
            // Actor1800 slot16 ->8258F6E0 AddMgAttribute, NOT InsertIntent.
            animation.emit_packet(physical.grind_name, 1.0);
            let backwards = facing::backwards(
                physical.ground_axis,
                physical.board_axis,
                physical.ground_flag_273,
                physical.animation_mirrored,
            );
            animation.emit_packet(
                encode(if backwards {
                    b"GrindFacingBackwards"
                } else {
                    b"GrindFacingForwards"
                }),
                1.0,
            );
        }
        Operation::Crouch { .. } if phase == 0 => state.height = Some(physical.height),
        Operation::Crouch { height } if phase == 1 => {
            let previous = state
                .height
                .ok_or("ControlGrindCrouch Update before Begin")?;
            let [min, max, rate] = settings.height;
            let next = crouch(
                previous,
                animation.intent("Crouch").unwrap_or(0.0),
                physical.crouch,
                min,
                max,
                rate,
                dt,
            );
            state.height = Some(next);
            set(animation, *height, next, false);
        }
        Operation::Fade {
            height,
            twist,
            intent,
        } => {
            if phase == 0 {
                set(animation, *height, physical.height, false);
                let bounds = endpoints(animation, *twist)?;
                let mut fade = Fade::default();
                let value = fade.begin(
                    bounds[0],
                    bounds[1],
                    mirrored_twist(physical.twist, physical.animation_mirrored),
                );
                set(animation, *twist, value, false);
                state.fade = Some(fade);
            } else if phase == 1 {
                let fade = state
                    .fade
                    .as_mut()
                    .ok_or("GrindControlFade Update before Begin")?;
                if let Some(value) =
                    fade.update(dt, animation.intent(intent).unwrap_or(0.0), settings.fade)
                {
                    set(animation, *twist, value, false);
                }
            }
        }
        _ => {}
    }
    Ok(())
}
