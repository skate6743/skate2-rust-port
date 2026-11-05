//! Direct original bindings82D94EC8..4F48,82D3AB30/AD3C/AE68 and stock schema.
use super::SlideState;
use skate3_core::{
    physics::contact::RetailContactMaterial,
    player::slide_state::{SlideSettings, SlideSurface},
    point_graph::PointGraph,
};
use skate_data::collections::Collections;
impl SlideState {
    pub fn load(data: &Collections) -> Result<Self, String> {
        let f = |class, field| data.float(class, "default", field);
        let c = |field| curve(data, "physics_slide", "default", field);
        let mut surfaces = Vec::new();
        for key in ["smooth", "rough", "slow", "slippery", "veryslow"] {
            surfaces.push((
                SlideSurface {
                    speed_to_force: curve(
                        data,
                        "physics_surfaces",
                        key,
                        "Powerslide_SpeedToForce",
                    )?,
                    yaw_strength: data.float("physics_surfaces", key, "Powerslide_YawStrength")?,
                    yaw_damping: data.float("physics_surfaces", key, "Powerslide_YawDamping")?,
                },
                RetailContactMaterial {
                    static_friction: data.float("physics_surfaces", key, "WheelStaticFriction")?,
                    dynamic_friction: data.float(
                        "physics_surfaces",
                        key,
                        "WheelDynamicFriction",
                    )?,
                    restitution: f("physicswheels", "WheelRestitution")?,
                },
            ));
        }
        Ok(Self {
            state: skate3_core::player::slide_state::SlideState::new(),
            settings: SlideSettings {
                input_remap: c("slide_input_remap")?,
                remap_vs_speed: c("Hash_DE162591FD9D91D7")?,
                // Toolkit528 copies layout160;608 copies layout80, not table order.
                force_vs_angle: c("Hash_91282E2CC4252731")?,
                force_vs_speed: c("Hash_2AD93E5ABA231D2")?,
                softest_wheel_force: f("physicswheels", "SoftestWheelPowerslideFactor")?,
                softest_wheel_spin: f("physicswheels", "SoftestWheelPowerslideSpinFactor")?,
                angular_force: f("physics_slide", "AngularForceScalar")?,
                force_y_offset: f("physics_slide", "PowerSlideForceYOffset")?,
            },
            surfaces,
            manual_scalar: f("physics_manual", "PowerSlideScalar")?,
        })
    }
}
fn curve(d: &Collections, c: &str, k: &str, n: &str) -> Result<PointGraph<8>, String> {
    let w = d.words::<20>(c, k, n)?.map(f32::from_bits);
    Ok(PointGraph {
        x: w[4..12].try_into().unwrap(),
        y: w[12..20].try_into().unwrap(),
    })
}
