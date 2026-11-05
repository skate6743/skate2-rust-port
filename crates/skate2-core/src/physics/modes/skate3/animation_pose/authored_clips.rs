//! Optional authored body clips. Stock graph timing, board, trajectory and attributes remain authoritative.
use super::*;
use glam::{Mat4, Quat, Vec3};
use serde::Deserialize;
use std::collections::BTreeMap;
#[derive(Default)]
pub(super) struct Replacements(pub(super) BTreeMap<String, ClipFrames>);
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    version: u32,
    bone_names: Vec<String>,
    clips: BTreeMap<String, AuthoredClip>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AuthoredClip {
    fps: f32,
    frames: Vec<Vec<[f32; 16]>>,
}
impl Replacements {
    pub fn load(root: &Path, frames: &AnimationFrames) -> Result<Self, String> {
        let path = root.join("private/custom/crouch-treflip.json");
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(format!("{}: {e}", path.display())),
        };
        Self::parse(&text, frames).map_err(|e| format!("{}: {e}", path.display()))
    }
    pub(super) fn parse(text: &str, frames: &AnimationFrames) -> Result<Self, String> {
        let file: File = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if file.version != 1 || file.bone_names != frames.bone_names || file.clips.len() > 32 {
            return Err("Authored clip format or skeleton mismatch".into());
        }
        let bind = &frames.named_pose("RIG_TPOSE")?.samples;
        let index = |name: &str| {
            frames
                .bone_names
                .iter()
                .position(|n| n == name)
                .ok_or_else(|| format!("Missing {name}"))
        };
        let hips = index("HIPS")?;
        let board = index("SKATEBOARD_ROOT")?;
        let helpers = [
            (index("RIGHTTOEBASE_REPARENTED")?, index("RIGHTTOEBASE")?),
            (index("LEFTTOEBASE_REPARENTED")?, index("LEFTTOEBASE")?),
            (index("RIGHTHAND_REPARENTED")?, index("RIGHTHAND")?),
            (index("LEFTHAND_REPARENTED")?, index("LEFTHAND")?),
        ];
        let mut output = BTreeMap::new();
        for (name, authored) in file.clips {
            let allowed = matches!(
                name.as_str(),
                "R_ANTIC_OLLIE_N_0_INTO"
                    | "R_ANTIC_OLLIE_N_0_CYC"
                    | "R_ANTIC_360SHUVIT_N_0_CYC"
                    | "360FLIP_D_HIGH_G"
                    | "360FLIP_D_HIGH_A"
                    | "360FLIP_D_LOW_G"
                    | "360FLIP_D_LOW_A"
            );
            if !allowed {
                return Err(format!("Unsupported authored slot {name}"));
            }
            let stock = frames.clip(&name)?;
            if authored.fps != f32::from_bits(stock.fps_bits)
                || authored.frames.len() != stock.frames.len()
            {
                return Err(format!("{name}: timing differs from stock slot"));
            }
            let mut baked = stock.frames.clone();
            for (row, absolute) in baked.iter_mut().zip(authored.frames) {
                if absolute.len() != frames.bone_names.len() {
                    return Err(format!("{name}: missing bones"));
                }
                let mut globals = Vec::with_capacity(absolute.len());
                for values in absolute {
                    let m = Mat4::from_cols_array(&values);
                    let (scale, rotation, _) = m.to_scale_rotation_translation();
                    if !m.is_finite()
                        || m.determinant() <= 0.0
                        || scale.min_element() < 0.9
                        || scale.max_element() > 1.1
                        || !rotation.is_finite()
                        || (m.w_axis.w - 1.).abs() > 0.001
                        || m.x_axis.w.abs() + m.y_axis.w.abs() + m.z_axis.w.abs() > 0.001
                    {
                        return Err(format!("{name}: invalid joint matrix"));
                    }
                    globals.push(m);
                }
                // Reconstruct the stock board hierarchy to express helper targets in its moving space.
                let mut stock_globals = vec![Mat4::IDENTITY; row.len()];
                for bone in 1..row.len() {
                    let parent = frames.parents[bone];
                    let local = matrix(pose_add::add(sqt(row[bone]), sqt(bind[bone]), true));
                    stock_globals[bone] = if parent > 0 {
                        stock_globals[parent as usize] * local
                    } else {
                        local
                    };
                }
                for bone in board..helpers.iter().map(|p| p.0).min().unwrap() {
                    globals[bone] = stock_globals[bone];
                }
                for &(helper, joint) in &helpers {
                    globals[helper] = globals[joint];
                }
                for bone in (hips..board).chain(helpers.iter().map(|p| p.0)) {
                    let parent = frames.parents[bone];
                    let local = if parent > 0 {
                        globals[parent as usize].inverse() * globals[bone]
                    } else {
                        globals[bone]
                    };
                    row[bone] = words(remove_reference(local, sqt(bind[bone])));
                }
            }
            output.insert(
                name.clone(),
                ClipFrames {
                    name,
                    source_offset: stock.source_offset,
                    fps_bits: stock.fps_bits,
                    loop_translation_bits: stock.loop_translation_bits,
                    loop_rotation_bits: stock.loop_rotation_bits,
                    channel_animation: stock.channel_animation,
                    channel_weights: stock.channel_weights.clone(),
                    frames: baked,
                },
            );
        }
        Ok(Self(output))
    }
    pub fn clip(&self, name: &str) -> Option<&ClipFrames> {
        self.0.get(name)
    }
}
fn matrix(s: Sqt) -> Mat4 {
    Mat4::from_scale_rotation_translation(
        Vec3::from_slice(&s.scale),
        Quat::from_array(s.rotation),
        Vec3::from_slice(&s.translation),
    )
}

fn remove_reference(local: Mat4, reference: Sqt) -> Sqt {
    let (scale, rotation, translation) = local.to_scale_rotation_translation();
    let inverse = Quat::from_array(reference.rotation).inverse();
    let scale = scale / Vec3::from_slice(&reference.scale);
    // AddSQT rotates translation without applying the reference scale.
    let translation = inverse * (translation - Vec3::from_slice(&reference.translation));
    Sqt {
        scale: [scale.x, scale.y, scale.z, 1.0],
        rotation: (inverse * rotation).normalize().to_array(),
        translation: [translation.x, translation.y, translation.z, 1.0],
    }
}

fn words(s: Sqt) -> SampleWords {
    [
        s.scale[0],
        s.scale[1],
        s.scale[2],
        s.rotation[0],
        s.rotation[1],
        s.rotation[2],
        s.rotation[3],
        s.translation[0],
        s.translation[1],
        s.translation[2],
    ]
    .map(f32::to_bits)
}

#[cfg(any())]
mod tests {
    use super::*;
    fn fixture() -> (AnimationFrames, String) {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let banks = AnimationBanks::load(&root).unwrap();
        let frames = AnimationFrames::from_banks(&banks).unwrap();
        let text =
            std::fs::read_to_string(root.join("private/custom/crouch-treflip.json")).unwrap();
        (frames, text)
    }
    #[test]
    #[ignore = "requires private stock banks and the captured crouch/treflip asset"]
    fn authored_body_preserves_board_trajectory_and_slot_timing() {
        let (frames, text) = fixture();
        let replacements = Replacements::parse(&text, &frames).unwrap();
        assert_eq!(replacements.0.len(), 7);
        let board = frames
            .bone_names
            .iter()
            .position(|n| n == "SKATEBOARD_ROOT")
            .unwrap();
        for (name, clip) in &replacements.0 {
            let stock = frames.clip(name).unwrap();
            assert_eq!(clip.fps_bits, stock.fps_bits);
            assert_eq!(clip.frames.len(), stock.frames.len());
            assert_eq!(clip.channel_weights, stock.channel_weights);
            for (a, b) in clip.frames.iter().zip(&stock.frames) {
                assert_eq!(a[0], b[0]);
                assert_eq!(&a[board..32], &b[board..32]);
            }
            assert!(
                clip.frames
                    .iter()
                    .zip(&stock.frames)
                    .any(|(a, b)| a[2] != b[2])
            );
        }
        assert!(replacements.clip("NOLLIE_360FLIP_D_HIGH_G").is_none());
    }
    #[test]
    #[ignore = "requires private stock banks and the captured crouch/treflip asset"]
    fn authored_idle_body_loop_closes() {
        let (frames, text) = fixture();
        let replacements = Replacements::parse(&text, &frames).unwrap();
        for name in ["R_ANTIC_OLLIE_N_0_CYC", "R_ANTIC_360SHUVIT_N_0_CYC"] {
            let c = replacements.clip(name).unwrap();
            assert_eq!(&c.frames[0][1..25], &c.frames.last().unwrap()[1..25]);
        }
    }
    #[test]
    #[ignore = "requires private stock banks and the captured crouch/treflip asset"]
    fn authored_clip_rejects_bad_timing_and_skeleton() {
        let (frames, text) = fixture();
        let mut v: serde_json::Value = serde_json::from_str(&text).unwrap();
        v["bone_names"][1] = "WRONG".into();
        assert!(Replacements::parse(&v.to_string(), &frames).is_err());
        let mut v: serde_json::Value = serde_json::from_str(&text).unwrap();
        v["clips"]["360FLIP_D_HIGH_G"]["fps"] = 24.into();
        assert!(Replacements::parse(&v.to_string(), &frames).is_err());
    }
}
