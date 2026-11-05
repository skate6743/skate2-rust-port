//!82D94C10 copies physics_jump layout0..456 into the board toolkit cache.
use skate3_core::{
    air::ground_jump::{GroundJumpMode, GroundJumpSettings},
    point_graph::PointGraph,
};
use skate_data::collections::Collections;
pub(crate) struct GroundAnimationSettings {
    pub jump: GroundJumpSettings,
    pub modes: [GroundJumpMode; 5],
}
impl GroundAnimationSettings {
    pub fn load(d: &Collections) -> Result<Self, String> {
        let f = |name| d.float("physics_jump", "default", name);
        let words = d.words::<32>("physics_jump", "default", "VerticalResponse")?;
        let mut modes = [GroundJumpMode {
            minimum_height_64: 0.0,
            minimum_height_68: 0.0,
            maximum_height: 0.0,
        }; 5];
        for (slot, key) in modes
            .iter_mut()
            .zip(["easy", "normal", "hardcore", "motorized", "test"])
        {
            *slot = GroundJumpMode {
                minimum_height_64: d.float("physics_mode", key, "Hash_BE3F74F978D777E5")?,
                minimum_height_68: d.float("physics_mode", key, "JumpMinHeight")?,
                maximum_height: d.float("physics_mode", key, "JumpMaxHeight")?,
            };
        }
        Ok(Self {
            modes,
            jump: GroundJumpSettings {
                vertical_response: PointGraph {
                    x: std::array::from_fn(|i| f32::from_bits(words[i])),
                    y: std::array::from_fn(|i| f32::from_bits(words[16 + i])),
                },
                y_scalar_vs_normal_y: graph(d, "JumpYScalarVsGroundNormalY", true)?,
                speed_scalar_vs_angle: graph(d, "JumpSpeedScalarVsAngle", true)?,
                minimum_height_vs_speed: graph(d, "MinHeightVsSpeed", false)?,
                maximum_height_vs_speed: graph(d, "MaxHeightVsSpeed", false)?,
                speed_response_max_speed: f("SpeedResponseMaxSpeed")?,
                minimum_scalar: f("MinScalar")?,
                maximum_y_bonus: f("JumpYBonusMax")?,
                adjust_z_factor: f("JumpAdjustZFactor")?,
                adjust_x_factor: f("JumpAdjustXFactor")?,
                absolute_minimum_height: f("AbsoluteMinHeight")?,
            },
        })
    }
}
fn graph(d: &Collections, name: &str, negative: bool) -> Result<PointGraph<8>, String> {
    let (x, y) = if negative {
        let w = d.words::<20>("physics_jump", "default", name)?;
        (
            std::array::from_fn(|i| f32::from_bits(w[4 + i])),
            std::array::from_fn(|i| f32::from_bits(w[12 + i])),
        )
    } else {
        let w = d.words::<16>("physics_jump", "default", name)?;
        (
            std::array::from_fn(|i| f32::from_bits(w[i])),
            std::array::from_fn(|i| f32::from_bits(w[8 + i])),
        )
    };
    Ok(PointGraph { x, y })
}
