//! Original SkeletonDrives82BEA670/82BEB488: two channels per physical bone
//! and four independent static targets, all solved in the same island.
use super::{
    ANIMATION_PART_COUNT, BoneDriveDynamics, BoneDriveSettings, PART_COUNT, SkeletonTargets,
    TARGET_COUNT, bone_drive_frames, prepare_bone_drive_frames,
    targets::{HARD_STRENGTH, HARD_VELOCITY, TARGET_PARTS, hard},
};
use crate::physics::{
    assembly::BodySnapshot,
    board_pose::orthonormalize_part_basis,
    drive_frames::RetailDriveFrames,
    drive_solver::{RetailDriveBodyState, RetailDriveRows, build_drive_rows},
    rigid_body::{RetailSimulationStep, pack_world_inverse_inertia},
    skeleton_animation_record::{AnimationPartTransform, compose_affine},
    skeleton_root::inverse_rigid,
};

#[derive(Clone, Copy, Debug)]
pub struct SkeletonDriveSettings {
    pub bone: BoneDriveSettings,
    pub enabled: bool,
    pub strength: [f32; 2],
    /// InitAttribData82BE7860 applies CollisionDriveScalar to each part's
    /// LOCAL and LOCAL_ROOT collision strengths before use in Update.
    pub collision_strength: [[f32; 2]; ANIMATION_PART_COUNT],
}
pub struct BoneDrives {
    pub parent: [usize; 2],
    pub active: [bool; 2],
    pub frames: [RetailDriveFrames; 2],
    pub dynamics: BoneDriveDynamics,
}
pub struct SkeletonDrives {
    pub targets: SkeletonTargets,
    pub bones: [Option<BoneDrives>; ANIMATION_PART_COUNT],
    pub settings: SkeletonDriveSettings,
}
/// Identity stays parallel to eligible rows. Native m_spy requests force
/// observation; it does not disable a drive or remove it from the solve.
#[derive(Clone, Copy, Debug)]
pub enum SkeletonDriveIdentity {
    Target(usize),
    Bone { part: usize, channel: usize },
}
pub struct SkeletonDriveBatch {
    pub rows: Vec<RetailDriveRows>,
    pub identities: Vec<SkeletonDriveIdentity>,
    pub spy: Vec<bool>,
}
impl SkeletonDrives {
    pub fn new(
        initial_bones: &[AnimationPartTransform; ANIMATION_PART_COUNT],
        initial_mapped: &[AnimationPartTransform; ANIMATION_PART_COUNT],
        parents: &[Option<usize>; ANIMATION_PART_COUNT],
        animation_to_world: AnimationPartTransform,
        spawn: AnimationPartTransform,
        simulation: RetailSimulationStep,
        settings: SkeletonDriveSettings,
    ) -> Result<Self, &'static str> {
        if parents.iter().flatten().any(|p| *p >= ANIMATION_PART_COUNT)
            || parents[0].is_some()
            || parents[23].is_some()
            || parents[1..23].iter().any(Option::is_none)
        {
            return Err("Skeleton drives require22 bone pairs and two roots");
        }
        let inverses = initial_bones.map(|m| inverse_rigid(&m));
        let bones = std::array::from_fn(|part| {
            parents[part].map(|parent| {
                let frames = [parent, 23].map(|p| {
                    bone_drive_frames(&initial_bones[part], &inverses[part], &inverses[p])
                });
                let mut dynamics = BoneDriveDynamics::default();
                // Init82BED180: LOCAL mode2 followed by LOCAL_ROOT mode0.
                dynamics.mode = 2;
                dynamics.enable(0, 1.0, settings.bone);
                dynamics.mode = 0;
                dynamics.enable(1, 1.0, settings.bone);
                BoneDrives {
                    parent: [parent, 23],
                    active: [true, true],
                    frames,
                    dynamics,
                }
            })
        });
        let mut targets = SkeletonTargets::new(simulation);
        let frame = initial_mapped[23];
        let normalized =
            orthonormalize_part_basis(std::array::from_fn(|i| frame[i / 4][i % 4].to_bits()));
        let hips =
            std::array::from_fn(|i| std::array::from_fn(|j| f32::from_bits(normalized[i * 4 + j])));
        targets.reset(compose_affine(&animation_to_world, &hips), spawn);
        Ok(Self {
            targets,
            bones,
            settings,
        })
    }

    ///82BEB488 consumes the24 IK-adjusted frames in animation coordinates.
    /// Collision weight comes from the actual SkeletonCollision state owner.
    pub fn update(
        &mut self,
        pose: &[AnimationPartTransform; ANIMATION_PART_COUNT],
        partial: bool,
        collision_weight: f32,
    ) {
        let inverses = pose.map(|frame| inverse_rigid(&frame));
        let settings = self.settings;
        let base = if settings.enabled {
            settings.strength
        } else {
            [0.0; 2]
        };
        self.targets.dynamics[0].linear = hard(HARD_VELOCITY, HARD_STRENGTH);
        for part in 1..23 {
            let Some(bone) = self.bones[part].as_mut() else {
                continue;
            };
            let selected = !partial || matches!(part,1..=5|7..=9);
            let strengths = if !(collision_weight >= 1.0) && selected {
                bone.dynamics.mode = 3;
                std::array::from_fn(|channel| {
                    let c = settings.collision_strength[part][channel];
                    //82BEB6F0/F4 are fmadds, followed by scalar fmuls.
                    (1.0 - c).mul_add(collision_weight, c) * settings.strength[channel]
                })
            } else {
                bone.dynamics.mode = 0;
                base
            };
            bone.dynamics.strengths = strengths;
            for channel in 0..2 {
                if !bone.active[channel] {
                    continue;
                }
                bone.frames[channel] = bone_drive_frames(
                    &pose[part],
                    &inverses[part],
                    &inverses[bone.parent[channel]],
                );
                bone.dynamics
                    .enable(channel, strengths[channel], settings.bone);
            }
        }
    }

    /// Native order: four targets, then LOCAL and LOCAL_ROOT for parts1..22.
    /// The caller appends rows to the board/contact/joint solve before any
    /// integration and borrows targets.bodies directly into that same step.
    pub fn build(
        &mut self,
        bodies: &[BodySnapshot; PART_COUNT],
        reaction_base: usize,
        target_reaction_base: usize,
        time_step: f32,
    ) -> SkeletonDriveBatch {
        let mut batch = SkeletonDriveBatch {
            rows: Vec::with_capacity(48),
            identities: Vec::with_capacity(48),
            spy: Vec::with_capacity(48),
        };
        for target in 0..TARGET_COUNT {
            self.targets.frames[target] = prepare_bone_drive_frames(self.targets.frames[target]);
            let part = TARGET_PARTS[target];
            let a = bodies[part];
            let b = self.targets.bodies[target];
            if (a.state_flags | b.state_flags) & 4 != 0 {
                batch.rows.push(build_drive_rows(
                    drive_body(a, reaction_base + part),
                    drive_body(b, target_reaction_base + target),
                    self.targets.frames[target],
                    self.targets.dynamics[target],
                    time_step,
                ));
                batch.identities.push(SkeletonDriveIdentity::Target(target));
                batch.spy.push(target >= 2);
            }
        }
        for part in 1..23 {
            let Some(bone) = self.bones[part].as_mut() else {
                continue;
            };
            for channel in 0..2 {
                if !bone.active[channel] {
                    continue;
                }
                bone.frames[channel] = prepare_bone_drive_frames(bone.frames[channel]);
                let parent = bone.parent[channel];
                let a = bodies[part];
                let b = bodies[parent];
                if (a.state_flags | b.state_flags) & 4 != 0 {
                    batch.rows.push(build_drive_rows(
                        drive_body(a, reaction_base + part),
                        drive_body(b, reaction_base + parent),
                        bone.frames[channel],
                        bone.dynamics.channels[channel],
                        time_step,
                    ));
                    batch
                        .identities
                        .push(SkeletonDriveIdentity::Bone { part, channel });
                    batch.spy.push(true);
                }
            }
        }
        batch
    }
}
fn drive_body(body: BodySnapshot, reaction_index: usize) -> RetailDriveBodyState {
    let rates = body.rates;
    RetailDriveBodyState {
        reaction_index,
        state: body.state_flags,
        orientation: rates.orientation,
        basis: rates.basis,
        center_of_mass: rates.position,
        linear_velocity: rates.linear_velocity,
        angular_velocity: rates.angular_velocity,
        force_acceleration: rates.force_acceleration,
        torque_acceleration: rates.torque_acceleration,
        inverse_mass: body.inertia.inverse_mass,
        world_inverse_inertia: pack_world_inverse_inertia(rates.world_inverse_inertia),
    }
}
