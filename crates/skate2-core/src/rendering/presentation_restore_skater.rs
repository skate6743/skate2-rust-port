//! `SkaterPresentation`'s restore of one skater between two submitted frames:
//! `UpdateSkater` 0x827d9e10's root and opacity blend, `UpdateBody` 0x827de868's pose
//! blend (`AnimCommandSystem::Blend`), the root move, expressions and final pose of
//! `UpdateBodies` 0x827de988, and `DoSimRestore` 0x827e6438's entity matrix, bounds and
//! render opacity.

use super::math::slerp;
use crate::{
    actor_animation_construction::initialization::body_dispatch::tree_commands::numeric::blend,
    actor_face_playback::presentation::{
        corrections::SliceChannels,
        expressions::{TwistAttributeSource, TwistRatio, try_compute_expression_channels},
        frame::selected_runtime::render_opacity::{
            RenderOpacityInputs, SkaterRenderOpacity, try_restore,
        },
        move_root::{matrix_inverse::try_inverse, multiply, rigid_matrix, try_move_root_channels},
    },
    animation::{AnimationMatrix, AnimationSkeleton, LocalPose, compose_pose_matrix},
    attribute_runtime::AttributeRuntimeInstance,
    character::{
        material_admission::CharacterMaterialAdmission,
        palette::{CharacterPalette, CharacterPaletteIdentity},
    },
    reckoning_numeric::HostReckoningNumeric,
    reckoning_sin_cos::HostReckoningSinCosNumeric,
    skater_ropa_runtime::ClothClock,
};
use skate2_formats::andale::AndaleSqt;
use std::sync::{Arc, Mutex};

/// `SkaterPresentation::Data`: the per-skater header `WriteData` submits.
pub(crate) const HEADER_BYTES: usize = 208;

/// What a skater's restore reads that its submissions do not change.
pub(crate) struct SkaterRig {
    pub(crate) skeleton: AnimationSkeleton,
    pub(crate) bind: Vec<AndaleSqt>,
    pub(crate) inverse_bind: Arc<[AnimationMatrix]>,
    pub(crate) hips: usize,
    pub(crate) board: usize,
    pub(crate) twist_bones: [usize; 14],
    /// `m_localBB`, min then max.
    pub(crate) local_bounds: [[f32; 3]; 2],
    /// GlobalAttributeCollection+0x198 `rendering_skaters`.
    pub(crate) rendering_skaters: AttributeRuntimeInstance,
}

/// One tick's submitted skater: its `Data` header and `ReadSkater`'s decoding of the
/// frame's pose, before the root move and expressions.
pub(crate) struct SkaterFrame {
    pub(crate) rig: Arc<SkaterRig>,
    /// The skater's unposed palette, sharing the drawn palette's identity.
    pub(crate) palette: Arc<CharacterPalette>,
    pub(crate) header: [u8; HEADER_BYTES],
    pub(crate) pose: Vec<AndaleSqt>,
    /// The four `animation_twistbones` ratios the expressions sample.
    pub(crate) twist_ratios: [f32; 4],
    /// The entity's materials, whose ROPA cloth `ClothUpdate` steps.
    pub(crate) materials: Option<Arc<Mutex<CharacterMaterialAdmission>>>,
}

impl SkaterFrame {
    pub(crate) fn palette_identity(&self) -> CharacterPaletteIdentity {
        self.palette.identity()
    }

    fn word(&self, offset: usize) -> u32 {
        u32::from_be_bytes(
            self.header[offset..offset + 4]
                .try_into()
                .expect("fixed header word"),
        )
    }

    fn vector(&self, offset: usize) -> [f32; 4] {
        std::array::from_fn(|lane| f32::from_bits(self.word(offset + 4 * lane)))
    }

    /// `DoSimRestore` 827e68d4..694c's pairing: the same `peerId` and player nibble.
    pub(crate) fn pairs_with(&self, other: &Self) -> bool {
        self.identity() == other.identity() && Arc::ptr_eq(&self.rig, &other.rig)
    }

    /// `peerId` and the player nibble of word 0xbc, which pair a skater across frames.
    fn identity(&self) -> (u64, u32) {
        let peer = u64::from_be_bytes(self.header[160..168].try_into().expect("peer id"));
        (peer, self.word(188) & 0xf0)
    }

    fn teleported(&self) -> bool {
        self.word(192) & 0x1000 != 0
    }

    fn opacity(&self) -> u8 {
        self.header[0xbe]
    }
}

/// `ComputeExpressions`' ratios as sampled with the frame.
struct FrameRatios([f32; 4]);

impl TwistAttributeSource for FrameRatios {
    type Error = std::convert::Infallible;

    fn read_ratio(&mut self, ratio: TwistRatio) -> Result<f32, Self::Error> {
        Ok(self.0[match ratio {
            TwistRatio::Arm => 0,
            TwistRatio::Forearm => 1,
            TwistRatio::ForearmSecond => 2,
            TwistRatio::UpperLeg => 3,
        }])
    }
}

/// `UpdateBodies` 0x827de988's root move and expressions, then the hierarchy's final
/// matrices with the moved Hips at the root: the palette's pose, and the Hips.
fn posed(
    rig: &SkaterRig,
    mut pose: Vec<AndaleSqt>,
    twist_ratios: [f32; 4],
) -> Result<(Vec<AnimationMatrix>, AnimationMatrix, Vec<AnimationMatrix>), SkaterRestoreError> {
    let mut hips_transform = None;
    try_move_root_channels(
        &mut SliceChannels {
            bind: &[],
            replay: &mut pose,
        },
        rig.hips,
        rig.board,
        &mut hips_transform,
        &mut HostReckoningNumeric,
    )
    .map_err(|_| SkaterRestoreError::Pose("MoveRootToHips"))?;
    try_compute_expression_channels(
        &mut SliceChannels {
            bind: &rig.bind,
            replay: &mut pose,
        },
        &rig.twist_bones,
        &mut FrameRatios(twist_ratios),
        &mut HostReckoningNumeric,
        &mut HostReckoningSinCosNumeric,
    )
    .map_err(|_| SkaterRestoreError::Pose("ComputeExpressions"))?;
    let hips = hips_transform.ok_or(SkaterRestoreError::Pose("hips transform"))?;
    let global = rig
        .skeleton
        .global_matrices(&LocalPose::from_andale(&pose))
        .map_err(|_| SkaterRestoreError::Pose("hierarchy"))?;
    if global.len() != rig.inverse_bind.len() {
        return Err(SkaterRestoreError::Pose("inverse bind count"));
    }
    let mut matrices: Vec<AnimationMatrix> = global
        .iter()
        .zip(rig.inverse_bind.iter())
        .map(|(global, inverse)| compose_pose_matrix(*global, *inverse))
        .collect();
    *matrices
        .first_mut()
        .ok_or(SkaterRestoreError::Pose("root bone"))? = hips;
    Ok((matrices, hips, global))
}

/// A point through a matrix of rows (the axes, then the translation).
fn transform_point(matrix: AnimationMatrix, [x, y, z]: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|lane| matrix[12 + lane] + matrix[8 + lane].mul_add(z, matrix[4 + lane].mul_add(y, matrix[lane] * x)))
}

/// `AnimCommandSystem::Blend` keeps thirteen bits of the coefficient.
fn native_blend_coefficient(percent: f32) -> f32 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the command word's thirteen-bit field"
    )]
    let bits = (percent * 8191.0).floor() as i32 as u32 & 0x1fff;
    #[expect(clippy::cast_precision_loss, reason = "a thirteen-bit value")]
    let coefficient = bits as f32;
    coefficient * f32::from_bits(0x3900_0400)
}

/// A skater as the restore leaves it for drawing.
pub(crate) struct SkaterRestore {
    pub(crate) palette: CharacterPalette,
    pub(crate) opacity: SkaterRenderOpacity,
    /// The restored root (`GetSkaterRoot`): rotation rows, then the translation row.
    pub(crate) root: AnimationMatrix,
    /// `GetHipsPos` 0x827e3ef0.
    pub(crate) hips_position: [f32; 3],
    /// `GetHipsMatrix` (0x826c70d8)'s Z row.
    pub(crate) hips_forward: [f32; 3],
    /// `m_BB` as `SceneRenderView::Cull` 0x828501c0 leaves it, min then max.
    pub(crate) bounds: [[f32; 3]; 2],
    /// The entity's materials, as the last restored frame gave them.
    pub(crate) materials: Option<Arc<Mutex<CharacterMaterialAdmission>>>,
    /// Bones whose world positions the restore keeps (the head and feet, for an online player's
    /// scoring of a remote skater), and those positions as the last restore posed them.
    pub(crate) body_bones: Option<[usize; 3]>,
    pub(crate) body_points: Option<[[f32; 3]; 3]>,
}

#[derive(Debug)]
pub(crate) enum SkaterRestoreError {
    Pose(&'static str),
    Palette(crate::character::palette::CharacterPaletteError),
    Cloth(String),
    Opacity(String),
}

impl std::fmt::Display for SkaterRestoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pose(error) => write!(f, "skater restore pose: {error}"),
            Self::Palette(error) => write!(f, "skater restore palette: {error}"),
            Self::Cloth(error) => write!(f, "skater restore cloth: {error}"),
            Self::Opacity(error) => write!(f, "skater restore opacity: {error}"),
        }
    }
}

impl SkaterRestore {
    pub(crate) fn new(mut palette: CharacterPalette, opacity: SkaterRenderOpacity) -> Self {
        // The restored copy publishes in its own range, apart from the tick palette's.
        palette.set_generation(1 << 62);
        Self {
            palette,
            opacity,
            root: [0.0; 16],
            hips_position: [0.0; 3],
            hips_forward: [0.0; 3],
            bounds: [[0.0; 3]; 2],
            materials: None,
            body_bones: None,
            body_points: None,
        }
    }

    /// Restore `next`, blending from `previous` when it is the same skater.
    /// `camera` is the restored camera's position, which the proximity fade reads.
    /// `replayed`: a replay-controlled scene's `SceneReplayTimeController` picked the frames.
    /// `elapsed`: the presentation time this restore stands for, which the fade steps by.
    pub(crate) fn restore(
        &mut self,
        previous: Option<&SkaterFrame>,
        next: &SkaterFrame,
        clock: ClothClock,
        camera: [f32; 3],
        replayed: bool,
        elapsed: f32,
    ) -> Result<(), SkaterRestoreError> {
        let percent = clock.fraction;
        let rig = &next.rig;
        let matched = previous.filter(|previous| previous.pairs_with(next));
        // DoSimRestore 827e67f0..680c: a teleport in either frame restores the next
        // frame alone.
        let teleported = next.teleported() || previous.is_some_and(SkaterFrame::teleported);
        let root_source = matched.filter(|_| !teleported);
        let [next_rotation, mut next_translation] = [next.vector(128), next.vector(144)];
        next_translation[3] = 1.0;
        let (root, opacity_byte, header_source) = match root_source {
            Some(previous) => {
                let mut translation = previous.vector(144);
                translation[3] = 1.0;
                let rotation = slerp(previous.vector(128), next_rotation, percent);
                let lerped: [f32; 4] = std::array::from_fn(|lane| {
                    (next_translation[lane] - translation[lane]).mul_add(percent, translation[lane])
                });
                let mut lerped = lerped;
                lerped[3] = 1.0;
                let before = f32::from(previous.opacity());
                let after = f32::from(next.opacity());
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "fctidz then the low byte"
                )]
                let opacity = (after - before).mul_add(percent, before) as i64 as u8;
                (rigid_matrix(rotation, lerped), opacity, previous)
            }
            None => (
                rigid_matrix(next_rotation, next_translation),
                next.opacity(),
                next,
            ),
        };
        // UpdateBody 0x827de868: the post-blend teleport bit (0x8000) reads the next
        // frame alone and skips the blend.
        let blended = root_source.is_some_and(|previous| previous.pose.len() == next.pose.len());
        let pose = match root_source {
            Some(previous) if blended => {
                let coefficient = native_blend_coefficient(percent);
                previous
                    .pose
                    .iter()
                    .zip(&next.pose)
                    .map(|(a, b)| blend(*a, *b, coefficient))
                    .collect()
            }
            _ => next.pose.clone(),
        };
        let (matrices, hips, global) = posed(rig, pose, next.twist_ratios)?;
        self.palette
            .update_bone_transforms(&matrices)
            .map_err(SkaterRestoreError::Palette)?;
        let local_to_world = multiply(root, hips);
        // A bone's world position: its global translation through the skinning's local-to-world
        // (the palette's matrices are the globals over the inverse bind, uploaded through it).
        self.body_points = self.body_bones.and_then(|bones| {
            let point = |bone: usize| global.get(bone).map(|m| transform_point(local_to_world, [m[12], m[13], m[14]]));
            Some([point(bones[0])?, point(bones[1])?, point(bones[2])?])
        });
        // BeginSkinJob 0x827d7c48: ClothUpdate steps the entity's ROPA cloth with the
        // hips' rotation and the restored time. A replay's DoSimRestore runs it on the
        // pose it restored (StartJobs 0x827dbb90 at 0x827e7098), by the replay's frame and
        // fraction, each restore. The real-time restore draws at any rate; the original
        // drew, and so stepped, at most 60 times a second, and its constraints act once a
        // step: that cloth steps once a submitted frame, at that frame's own pose, and a
        // frame between two submissions blends their cloth as UpdateBody blends the body.
        if let Some(materials) = &next.materials {
            let mut materials = materials
                .lock()
                .map_err(|_| SkaterRestoreError::Pose("material lock"))?;
            if replayed {
                let (cloth_hips, inverse) = cloth_hips_transform(hips)?;
                materials
                    .step_restored_cloth(
                        &self.palette,
                        cloth_hips,
                        inverse,
                        clock,
                        next.word(192) >> 16,
                    )
                    .map_err(|error| SkaterRestoreError::Cloth(error.to_string()))?;
            } else if materials.cloth_needs_step(clock.frame) {
                let frame_hips = if blended {
                    let (frame_matrices, frame_hips, _) =
                        posed(rig, next.pose.clone(), next.twist_ratios)?;
                    self.palette
                        .update_bone_transforms(&frame_matrices)
                        .map_err(SkaterRestoreError::Palette)?;
                    frame_hips
                } else {
                    hips
                };
                let (cloth_hips, inverse) = cloth_hips_transform(frame_hips)?;
                let step = ClothClock {
                    frame: clock.frame,
                    fraction: 0.,
                };
                materials
                    .step_cloth(
                        &self.palette,
                        cloth_hips,
                        inverse,
                        step,
                        next.word(192) >> 16,
                    )
                    .map_err(|error| SkaterRestoreError::Cloth(error.to_string()))?;
                if blended {
                    self.palette
                        .update_bone_transforms(&matrices)
                        .map_err(SkaterRestoreError::Palette)?;
                }
            }
            if materials.cloth_disabled() == Some(false) {
                // A replay's step leaves the positions this restore draws.
                let fraction = if replayed { 1. } else { clock.fraction };
                materials
                    .publish_cloth(&self.palette, local_to_world, fraction)
                    .map_err(|error| SkaterRestoreError::Cloth(error.to_string()))?;
            }
        }
        self.materials.clone_from(&next.materials);
        self.palette
            .pack_for_upload(local_to_world)
            .map_err(SkaterRestoreError::Palette)?;
        self.root = root;
        self.hips_position = hips_position(root, hips);
        self.hips_forward = hips_forward(root, hips);
        let translation = [local_to_world[12], local_to_world[13], local_to_world[14]];
        self.bounds = [
            std::array::from_fn(|lane| rig.local_bounds[0][lane] + translation[lane]),
            std::array::from_fn(|lane| rig.local_bounds[1][lane] + translation[lane]),
        ];
        // UpdateSkater 827da1dc..1f0 takes the main-camera-only bit from the frame its
        // root came from; DoSimRestore 827e6eb0 takes the collidable bit from the next.
        let inputs = RenderOpacityInputs {
            opacity_byte,
            collidable: next.word(192) & 0x4000 != 0,
            main_camera_only: header_source.word(192) & 0x2000 != 0,
            hips: translation,
            up: [root[4], root[5], root[6]],
            in_replay: replayed,
        };
        try_restore(
            &mut self.opacity,
            inputs,
            camera,
            &rig.rendering_skaters,
            &mut HostReckoningNumeric,
            elapsed,
        )
        .map_err(|error| SkaterRestoreError::Opacity(format!("{error:?}")))
    }
}

/// `StartJobs` 0x827dbbf0..0x827dbc88: `mHipsTransform` is the Hips with the w of its
/// axes cleared; the cloth also takes its inverse.
fn cloth_hips_transform(
    mut hips: AnimationMatrix,
) -> Result<(AnimationMatrix, AnimationMatrix), SkaterRestoreError> {
    hips[3] = 0.;
    hips[7] = 0.;
    hips[11] = 0.;
    let inverse = try_inverse(hips, &mut HostReckoningNumeric)
        .map_err(|_| SkaterRestoreError::Pose("cloth hips inverse"))?;
    Ok((hips, inverse))
}

/// `GetHipsMatrix` (`0x826c70d8`)'s Z row: the saved Hips Z row through the root rows.
fn hips_forward(root: AnimationMatrix, hips: AnimationMatrix) -> [f32; 3] {
    let [x, y, z, w] = [hips[8], hips[9], hips[10], hips[11]];
    std::array::from_fn(|lane| {
        root[12 + lane].mul_add(
            w,
            root[8 + lane].mul_add(z, root[4 + lane].mul_add(y, root[lane] * x)),
        )
    })
}

/// `GetHipsPos`: the root rows scaled by the saved Hips translation row.
fn hips_position(root: AnimationMatrix, hips: AnimationMatrix) -> [f32; 3] {
    let [x, y, z, w] = [hips[12], hips[13], hips[14], hips[15]];
    std::array::from_fn(|lane| {
        root[12 + lane].mul_add(
            w,
            root[8 + lane].mul_add(z, root[4 + lane].mul_add(y, root[lane] * x)),
        )
    })
}
