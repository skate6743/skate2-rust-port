//! Original Skeleton::UpdateWipeout82BDFA88 on the live physical owners.
use crate::skate3_native::physics::{
    foot_ik::PhysicalInput,
    skeleton_input_runtime::{SkeletonInputRuntime, SkeletonOwners},
};
use skate3_core::{
    animation::output::NativeMatrix,
    physics::{skeleton_animation_record::IDENTITY, skeleton_body::SkeletonTargetInput},
    player::{
        input_phase::ProcessedPhysicsInput,
        wipeout_state::{drives, skeleton},
    },
};

///The original r8 byte skips only target-position updates. It does not skip
///root construction, COM frames, first-state-tick IK, or ragdoll drive updates.
pub(crate) fn update(
    input: &mut SkeletonInputRuntime,
    owners: &mut SkeletonOwners<'_>,
    processed: &ProcessedPhysicsInput,
    globals: &[NativeMatrix],
    settings: &drives::Settings,
    start: f32,
    end: f32,
    controlled: f32,
    extra: f32,
    skip_targets: bool,
) -> Result<f32, String> {
    let s = &mut owners.animated;
    s.roots.initialize_heading = false;
    skeleton::update_roots(
        &mut s.roots,
        &s.animation_hips,
        &owners.body.record.pose[23],
        owners.body.record.velocities[23],
        s.board_frames.centre_of_mass,
        processed.vectors_544_560_592_608[3].map(f32::from_bits),
        processed.state_timer_2664,
    );
    let target_input = SkeletonTargetInput {
        animation_hips: &s.animation_hips,
        animation_board: &s.animation_board,
        animation_to_world: &s.roots.animation_to_world,
        inverse_board: &s.roots.inverse_board,
        skate_root: &s.board_frames.skate_root,
        com_frame: &s.board_frames.com_frame,
        lifted_com_frame: &s.board_frames.lifted_com_frame,
        teleporting: input.teleporting,
    };
    //82BDFAC8 skips the whole82BE1618 call. Its nested82BE1870 call to
    //82BE18B8 is skipped too: retain all four targets and extra-body velocities.
    if !skip_targets {
        input.animation_board_to_physics =
            owners.drives.targets.update_hook_positions(&target_input);
        let positions = owners.drives.targets.update_extra_targets(
            owners.body,
            target_input.com_frame,
            target_input.lifted_com_frame,
        );
        input.extra_target_positions =
            [positions.com, positions.lifted_com, positions.following_com];
        owners.pose_errors.set_targets(positions);
    }
    //82BDFA88 ignores the target continuity result: no GeneralUpdate reset.
    s.board_frames.skate_root = s.roots.animation_to_world;
    s.board_frames.update_com_lift(
        &s.roots.animation_to_world,
        s.board_frames.centre_of_mass,
        0.0,
    );
    if processed.state_timer_2664 == 0.0 {
        let contacts = std::array::from_fn(|i| {
            let query = processed.line_tests_960_1008_1056[i];
            (query.valid != 0).then(|| query.position.map(f32::from_bits))
        });
        let result = owners.ik.update(
            s,
            globals,
            PhysicalInput {
                state_id: processed.state_2508,
                flags_2468: processed.flags_2468,
                flags_2472: processed.flags_2472,
                flags_2480: processed.flags_2480,
                contact_bone: owners.animation_input.contacts.bone as usize,
                physical_board: &s.board_frames.physical_board,
                toe_contact_normal: owners.ik.toe_contact_normal,
                current_contacts: contacts,
            },
        )?;
        input.drive_frames = result.frames;
    }
    //82BDFB84 passes Skeleton+12624, not the solved physical pose at8016.
    //ProcessData rewrites this animation-volume buffer each tick; first-tick
    //IK may modify it above.82BEB0C8 builds its inverses from that supplied
    //buffer (r4+64..), so physical parts cannot substitute for the drive target.
    let residual = drives::update(
        owners.drives,
        &input.drive_frames,
        settings,
        drives::Weights {
            start,
            end,
            controlled,
            upper_extra: if processed.flags_2484 & 0x20 != 0 {
                extra
            } else {
                0.0
            },
            lower_extra: if processed.flags_2484 & 0x10 != 0 {
                extra
            } else {
                0.0
            },
        },
    );
    s.motion.next_trajectory = IDENTITY;
    Ok(residual)
}

#[cfg(any())]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires private stock skater and collection assets"]
    fn retained_velocity_skips_all_wipeout_target_writes() {
        use crate::skate3_native::physics::{Skate3PhysicsState, Skate3ControllerState};
        use skate3_core::math::Vector3;
        let root = std::env::var_os("SKATE3_ASSET_ROOT").expect("set SKATE3_ASSET_ROOT");
        let root = std::path::Path::new(&root);
        let assets = skate_data::GameAssets::load(root).unwrap();
        let graphs = crate::skate3_native::graph_runtime::StockGraphs::load(root, &assets).unwrap();
        let physics = Skate3PhysicsState::load(root).unwrap();
        let mut skater = Skate3ControllerState::load(root, &graphs, &physics, "normal").unwrap();
        skater.player_input.processed.state_2508 = 300;
        skater.player_input.processed.state_timer_2664 = 1.0;
        // Distinct synthetic animation target exposes accidental substitution
        // of the real physical parts; no world-space body is moved here.
        skater.skeleton_input.drive_frames = [IDENTITY; 24];
        skater.skeleton_input.drive_frames[17][3][0] = 0.25;
        for part in [24, 25] {
            skater.skeleton.bodies_mut()[part].rates.linear_velocity =
                Vector3::new(1.25, -2.5, 3.75);
        }
        let velocities = [24, 25].map(|part| skater.skeleton.bodies()[part].rates.linear_velocity);
        let targets: [_; 4] = core::array::from_fn(|i| skater.skeleton_drives.targets.transform(i));
        let positions = skater.skeleton_input.extra_target_positions;
        let board_target = skater.skeleton_input.animation_board_to_physics;
        let mut owners = SkeletonOwners {
            animated: &mut skater.animated_skeleton,
            body: &mut skater.skeleton,
            drives: &mut skater.skeleton_drives,
            ik: &mut skater.foot_ik,
            animation_input: &mut skater.animation_input,
            correction: &mut skater.skeleton_output.correction,
            pose_errors: &mut skater.pose_errors,
        };
        update(
            &mut skater.skeleton_input,
            &mut owners,
            &skater.player_input.processed,
            &skater.animation.packet.hierarchy,
            &skater.wipeout_state.drives,
            0.0,
            0.0,
            0.0,
            0.0,
            true,
        )
        .unwrap();
        assert_eq!(
            [24, 25].map(|part| owners.body.bodies()[part].rates.linear_velocity),
            velocities
        );
        assert_eq!(
            core::array::from_fn::<_, 4, _>(|i| owners.drives.targets.transform(i)),
            targets
        );
        assert_eq!(skater.skeleton_input.extra_target_positions, positions);
        assert_eq!(
            skater.skeleton_input.animation_board_to_physics,
            board_target
        );
        let knee = owners.drives.bones[17].as_ref().unwrap();
        for channel in 0..2 {
            if knee.active[channel] {
                assert_eq!(
                    knee.frames[channel].body_b.translation.x, 0.25,
                    "UpdateRagdoll must consume the animation-volume buffer12624"
                );
            }
        }
    }
}
