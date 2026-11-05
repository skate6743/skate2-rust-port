//! Stock pushing settings and authored clip metadata (TU382B953E8..82B9584C).
use skate3_core::{
    animation::{clip_clock::ClipClock, skeleton_input::name::encode},
    point_graph::PointGraph,
    riding::{
        push_animation::PushAnimationCurves,
        push_behaviors::{PushAttributes, PushClipMetrics},
    },
};
use skate_data::{animation_metadata::AnimationMetadata, collections::Collections};
use std::path::Path;

/// A tree's ordered attributes. The value is its first payload lane, including
/// non-scalar types: native82B95620 does not filter the attribute type.
#[derive(Clone, Debug)]
pub struct PushTreeAttribute {
    pub name: String,
    pub first_payload: f32,
}
#[derive(Clone, Debug)]
pub struct PushTreeMetadata {
    pub length: f32,
    pub attributes: Vec<PushTreeAttribute>,
}
pub trait PushTreeSource {
    /// Obtain the actual animation tree's length and GetAttributes(mask=15)
    /// result in stored order. Missing assets must return an error.
    fn metadata(
        &mut self,
        tree_name: &str,
        attribute_mask: u32,
    ) -> Result<PushTreeMetadata, String>;
}

impl PushTreeSource for AnimationMetadata {
    fn metadata(
        &mut self,
        tree_name: &str,
        attribute_mask: u32,
    ) -> Result<PushTreeMetadata, String> {
        let clip = self.clip(tree_name)?;
        let frames = f32::from_bits(clip.frames_bits);
        let fps = f32::from_bits(clip.fps_bits);
        let base_speed = f32::from_bits(clip.base_speed_bits);
        // Init827B8AB0 sets speed1/time0, then recomputes length with this
        // specific multiply order (different from later SetSpeed).
        let length = (frames - 1.0) / ((1.0 * base_speed) * fps);
        let clock = ClipClock {
            frames,
            fps,
            base_speed,
            speed: 1.0,
            length,
            time: 0.0,
            previous_time: 0.0,
            loops_since_evaluation: 0,
            looping: clip.flags_word & 0x1000_0000 != 0,
            phase_controlled: clip.flags_word & 0x4000_0000 != 0,
        };
        let mut attributes = Vec::new();
        // GetAttributes82D25E30 appends in stored order. Untimed records
        // bypass both mask tests; timed records use GetAttributeStatus.
        for attribute in &clip.attributes {
            let begin = f32::from_bits(attribute.begin_bits);
            if begin != -1.0 {
                let status =
                    u32::from(clock.attribute_status(begin, f32::from_bits(attribute.end_bits)));
                if attribute_mask & 0x1c & status == 0 || attribute_mask & 3 & status == 0 {
                    continue;
                }
            }
            // Attribute::Init82D164F8 copies lane0 for types0/1/3. Type2
            // samples a curve; unknown types leave lane0 untouched. Neither
            // can be replaced by the first serialized word as a fallback.
            if !matches!(attribute.type_id, 0 | 1 | 3) {
                return Err(format!(
                    "{tree_name}: attribute {} needs type{} evaluation",
                    attribute.name, attribute.type_id,
                ));
            }
            attributes.push(PushTreeAttribute {
                name: attribute.name.clone(),
                first_payload: f32::from_bits(attribute.payload_words[0]),
            });
        }
        Ok(PushTreeMetadata { length, attributes })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PushingSettings {
    pub curves: PushAnimationCurves,
    pub teleport_window: f32,
    pub maximum_holding_acceleration: f32,
    pub out_speed_weight: f32,
    pub maximum_out_factor: f32,
    pub regular: PushAttributes,
    pub mongo: PushAttributes,
}
impl PushingSettings {
    /// Standalone settings check; game startup shares its already loaded banks.
    pub fn load_stock(data: &Collections, asset_root: &Path) -> Result<Self, String> {
        let mut trees =
            skate_data::animation_banks::AnimationBanks::load(asset_root)?.metadata()?;
        Self::load(data, &mut trees)
    }
    pub fn load(data: &Collections, trees: &mut impl PushTreeSource) -> Result<Self, String> {
        let float = |name| data.float("anim_motion", "pushing", name);
        Ok(Self {
            curves: PushAnimationCurves {
                button_time_max: curve(data, "button_time_max")?,
                button_time_to_dv: curve(data, "button_time_to_dv")?,
                blend_speed_over_frames: curve(data, "blend_speed_over_frames")?,
                blend_acc_over_frames: curve(data, "blend_acc_over_frames")?,
            },
            // Relative to the anim_motion/pushing layout pointer:1704,1788,1840,1808.
            teleport_window: float("pushing_usemaxpushfromteleporttime")?,
            maximum_holding_acceleration: float("max_holding_acc")?,
            out_speed_weight: float("dynamic_out_factor_speed_vs_acc")?,
            maximum_out_factor: float("last_push_out_time")?,
            regular: attributes(
                trees,
                [
                    "R_PUSHLSP_HSTR_N_0_CYC1",
                    "R_PUSHHSP_HSTR_N_0_CYC1",
                    "R_PUSHLSP_LSTR_N_0_CYC1",
                    "R_PUSHHSP_LSTR_N_0_CYC1",
                ],
            )?,
            mongo: attributes(
                trees,
                [
                    "R_PUSHLSP_HSTR_MONGO_0_CYC1",
                    "R_PUSHHSP_HSTR_MONGO_0_CYC1",
                    "R_PUSHLSP_LSTR_MONGO_0_CYC1",
                    "R_PUSHHSP_LSTR_MONGO_0_CYC1",
                ],
            )?,
        })
    }
    /// TU382BAD3B4..3DC/82BADBB8..BE0; GetIsSwitch, not natural stance.
    pub fn attributes(&self, regular_attributes: bool, is_switch: bool) -> &PushAttributes {
        if regular_attributes != is_switch {
            &self.regular
        } else {
            &self.mongo
        }
    }
}

fn curve(data: &Collections, name: &str) -> Result<PointGraph<8>, String> {
    let words = data.words::<20>("anim_motion", "pushing", name)?;
    Ok(PointGraph {
        x: std::array::from_fn(|i| f32::from_bits(words[4 + i])),
        y: std::array::from_fn(|i| f32::from_bits(words[12 + i])),
    })
}
fn attributes(trees: &mut impl PushTreeSource, names: [&str; 4]) -> Result<PushAttributes, String> {
    let mut metrics = Vec::with_capacity(4);
    for name in names {
        let tree = trees.metadata(name, 15)?;
        let mut begin = None;
        let mut end = None;
        for attribute in tree.attributes {
            // Native compares five-word FastStrings, whose encoding folds
            // ASCII case. Decoded bank names are uppercase; XML need not be.
            let key = encode(attribute.name.as_bytes());
            if key == encode(b"HStr_Vel_B") || key == encode(b"LStr_Vel_B") {
                begin = Some(attribute.first_payload);
            } else if key == encode(b"Vel_E") {
                end = Some(attribute.first_payload);
            }
        }
        // Native leaves missing outputs untouched. The host loader rejects
        // incomplete metadata rather than supplying arbitrary initial values.
        metrics.push(PushClipMetrics {
            length: tree.length,
            begin_velocity: begin
                .ok_or_else(|| format!("{name}: missing begin-velocity attribute"))?,
            end_velocity: end.ok_or_else(|| format!("{name}: missing Vel_E attribute"))?,
        });
    }
    Ok(PushAttributes::from_clips(metrics.try_into().unwrap()))
}

#[cfg(any())]
#[path = "tests/pushing_metadata.rs"]
mod metadata_tests;
