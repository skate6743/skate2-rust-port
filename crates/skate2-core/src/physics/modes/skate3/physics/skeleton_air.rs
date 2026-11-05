//! Production animated-board and COM-controlled air Skeleton updates.
//! Original TU3 82BDDA10 / 82BDE600. All owners are the same objects used by
//! ProcessData, GeneralUpdate, the shared solve and final pose publication.
use super::skeleton_input_runtime::{CollisionInput, SkeletonInputRuntime, SkeletonOwners};
use skate3_core::{
    animation::output::{NativeMatrix, physics_packet::PhysicsPosePacket},
    math::{Basis3, Vector3},
    physics::{
        board::BodyId,
        board_animation::{BoardAnimation, BoardAnimationSettings, target_velocity},
        board_runtime::BoardRuntime,
        drive_frames::RetailAffineTransform,
        rigid_body::RetailSimulationStep,
        skeleton_air_frames::{self, AirDismountRevert},
        skeleton_animation_record::AnimationPartTransform as Transform,
    },
    player::input_phase::ProcessedPhysicsInput,
    point_graph::PointGraph,
};
use skate_data::collections::Collections;

///The original board's persistent blend history, shared across Ground/Air
///state changes. This must live alongside the physical board, not per entry.
pub(crate) struct SkeletonAir {
    pub board_animation: BoardAnimation,
    settings: BoardAnimationSettings,
}
impl SkeletonAir {
    pub fn load(data: &Collections) -> Result<Self, String> {
        let graph = |name| -> Result<PointGraph<8>, String> {
            //Original82C01900: Globals260 layout4, graphs0/80;
            //stock physics_airstates/default has80-byte PointNegGraphData8.
            let words = data
                .words::<20>("physics_airstates", "default", name)?
                .map(f32::from_bits);
            Ok(PointGraph {
                x: words[4..12].try_into().unwrap(),
                y: words[12..20].try_into().unwrap(),
            })
        };
        Ok(Self {
            board_animation: BoardAnimation::default(),
            settings: BoardAnimationSettings {
                slow: graph("PhysToAnimSlow")?,
                fast: graph("PhysToAnimFast")?,
            },
        })
    }

    ///Original82D38000 GroundUpdate ->82D38008 UpdatePhysicsError. Call after
    ///Skeleton Ground GeneralUpdate and before the physical solve, every tick.
    pub fn capture_physics_error(&mut self, board: &BoardRuntime, animated_target: &Transform) {
        self.board_animation.capture_physics_error(
            animated_target,
            &matrix(board.part_transforms()[BodyId::Deck.index()]),
        );
    }

    ///82C0606C; preserves the source's independently retained blending flag.
    pub fn reset_board(&mut self) {
        self.board_animation.reset();
    }

    ///82C041C0: publish the effective matrix to the separate deck constraint
    ///anchor. It does not teleport the dynamic board assembly.
    pub(crate) fn apply_board(
        &mut self,
        board: &mut BoardRuntime,
        target: &Transform,
        fast: bool,
    ) -> Transform {
        let effective = self.board_animation.apply(target, fast, &self.settings);
        board.set_hook_transform(affine(effective));
        effective
    }
}

impl SkeletonInputRuntime {
    ///82BDDA10. PhysicsAir passes fast=false; GroundAnimation passes true.
    ///Returns Processed0..48 / Skeleton15952, the UNBLENDED animation target.
    ///The effective blended matrix is published to Skeleton12496 separately.
    pub fn update_animated(
        &mut self,
        air: &mut SkeletonAir,
        board: &mut BoardRuntime,
        reckoning: &Transform,
        p: &mut ProcessedPhysicsInput,
        owners: &mut SkeletonOwners<'_>,
        globals: &[NativeMatrix],
        collision: &CollisionInput,
        simulation: RetailSimulationStep,
        fast_blend: bool,
    ) -> Result<Transform, String> {
        let s = &mut owners.animated;
        //Unlike Ground82BDF530, this does NOT force initialize_heading=true.
        s.update_roots(board, reckoning, p.timestep_2604);
        let target = skeleton_air_frames::prepare_animated(
            &s.roots,
            &mut s.board_frames,
            &self.drive_frames[0],
            &mut p.flags_2468,
        );
        s.board_frames.physical_board = air.apply_board(board, &target, fast_blend);
        self.general_update(p, owners, globals, collision, simulation)?;
        //82BDDC20..54 clears only next trajectory, after GeneralUpdate.
        owners.animated.finish_ground();
        owners.animation_input.fields.flags2468 = p.flags_2468;
        Ok(target)
    }

    ///82BDE600. target_com is PhysicsAir's actually integrated COM trajectory
    ///position (82D34A28 r5), not a predicted board position or skeleton root.
    ///The animation packet is the real82B985E8 publication, including its
    ///consumed dismount-revert request and retained frame count.
    pub fn update_known_air(
        &mut self,
        air: &mut SkeletonAir,
        board: &mut BoardRuntime,
        reckoning: &Transform,
        target_com: [f32; 4],
        packet: &PhysicsPosePacket,
        p: &mut ProcessedPhysicsInput,
        owners: &mut SkeletonOwners<'_>,
        globals: &[NativeMatrix],
        collision: &CollisionInput,
        simulation: RetailSimulationStep,
    ) -> Result<Transform, String> {
        let s = &mut owners.animated;
        skeleton_air_frames::update_known_air_roots(
            &mut s.roots,
            reckoning,
            target_com,
            s.record.centre_of_mass,
            AirDismountRevert {
                requested: packet.flags & (1 << 28) != 0,
                //The native load zero-extends all32bits before fcfid.
                frames: packet.air_dismount_revert_frames as u32,
                goofy: p.flags_2476 & (1 << 2) != 0,
            },
        );
        let target = skeleton_air_frames::prepare_known_air(
            &s.roots,
            &mut s.board_frames,
            &self.drive_frames[0],
            &mut p.flags_2468,
        );
        let effective = air.apply_board(board, &target, true);
        s.board_frames.physical_board = effective;
        //82C04270 uses the deck BODY COM, not Part6's mass-offset transform.
        let pos = board.bodies()[BodyId::Deck.index()].rates.position;
        let velocity = target_velocity(effective[3], [pos.x, pos.y, pos.z, 0.0], p.timestep_2604);
        for body in board.bodies_mut() {
            body.rates.linear_velocity = Vector3::new(velocity[0], velocity[1], velocity[2]);
        }
        skeleton_air_frames::finish_known_air(&mut s.roots, &mut s.board_frames, effective);
        self.general_update(p, owners, globals, collision, simulation)?;
        owners.animation_input.fields.flags2468 = p.flags_2468;
        //KnownAir does not reset next_trajectory at the end of this method.
        Ok(target)
    }
}

fn matrix(t: RetailAffineTransform) -> Transform {
    let mut m = [[0.0; 4]; 4];
    for i in 0..3 {
        m[i][..3].copy_from_slice(&t.basis.columns[i]);
    }
    m[3] = [t.translation.x, t.translation.y, t.translation.z, 0.0];
    m
}
fn affine(m: Transform) -> RetailAffineTransform {
    RetailAffineTransform {
        basis: Basis3 {
            columns: std::array::from_fn(|i| m[i][..3].try_into().unwrap()),
        },
        translation: Vector3::new(m[3][0], m[3][1], m[3][2]),
    }
}
