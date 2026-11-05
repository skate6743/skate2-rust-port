use super::*;
use skate3_core::physics::skeleton_animation_record::IDENTITY;

#[test]
fn biped_ik_bindings_keep_adjacent_foot_volumes_distinct() {
    use skate3_core::animation::foot_ik::state::LIMBS;
    //TU3 Init82BED74C..770:1120 target parts and1136 adjacent parts.
    //These are physical part indices, not animation hierarchy indices.
    assert_eq!(LIMBS.map(|limb| limb.part), [15, 19, 3, 7]);
    assert_eq!(
        LIMBS.map(|limb| limb.parent_part),
        [Some(16), Some(20), None, None]
    );
}

#[test]
fn disabling_feet_target_ik_fades_board_weight_without_disabling_the_pose() {
    use skate3_core::animation::foot_ik::status::{BlendSettings, LimbStatus, Mode, update_blends};
    let mut limbs = [LimbStatus::default(); 4];
    for foot in &mut limbs[..2] {
        foot.mode = Mode::OnDeck;
        foot.board_blend = 0.5;
    }
    //Synthetic exact-binary test step, not a replacement stock setting.
    let settings = BlendSettings {
        hand_inner_padding: [0.; 4],
        hand_outer_padding: [1.; 4],
        external_blend_step: 0.125,
        board_blend_step: 0.25,
    };
    //82BEEE58..88 selects target zero from2480bit27 and bounds its delta.
    update_blends(&mut limbs, 1., 1., 0x0800_0000, &settings);
    assert_eq!([limbs[0].board_blend, limbs[1].board_blend], [0.25; 2]);
    update_blends(&mut limbs, 1., 1., 0x0800_0000, &settings);
    update_blends(&mut limbs, 1., 1., 0x0800_0000, &settings);
    assert_eq!([limbs[0].board_blend, limbs[1].board_blend], [0.; 2]);
    assert_eq!([limbs[0].mode, limbs[1].mode], [Mode::OnDeck; 2]);
    assert_eq!([limbs[2].mode, limbs[3].mode], [Mode::Disabled; 2]);
    update_blends(&mut limbs, 1., 1., 0, &settings);
    assert_eq!([limbs[0].board_blend, limbs[1].board_blend], [0.25; 2]);
}
#[test]
fn frozen_board_uses_retained_pose_and_flips_complete_axes() {
    let mut roots = SkeletonRootFrames::default();
    let mut retained = IDENTITY;
    retained[3] = [2., 3., 4., 0.];
    let mut mapped = IDENTITY;
    mapped[3] = [9.; 4];
    let mut world = IDENTITY;
    world[0][3] = 7.;
    world[2][3] = 8.;
    let mut flags = 2;
    let output = prepare(&mut roots, &world, &mapped, &mut retained, 4, 1, &mut flags);
    assert_eq!(retained[3], [2., 3., 4., 0.]);
    assert_eq!(roots.animation_to_world[0][3], -7.);
    assert_eq!(roots.animation_to_world[2][3], -8.);
    assert_eq!(output[3][..3], [-2., 3., -4.]);
    assert_eq!(flags, 0x80002);
    assert!(roots.initialize_heading);
}

#[test]
fn biped_com_and_skate_root_use_old_basis_without_overwriting_physical_board() {
    let mut roots = SkeletonRootFrames::default();
    let old = [
        [0., 0., -1., 0.],
        [0., 1., 0., 0.],
        [1., 0., 0., 0.],
        [10., 20., 30., 0.],
    ];
    roots.reset_initial_alignment(old);
    let mut frames = SkeletonBoardFrames::default();
    frames.physical_board[3] = [90., 80., 70., 0.];
    let physical = frames.physical_board;
    let mut animation = IDENTITY;
    animation[3] = [2., 3., 4., 0.];
    let mut mapped = IDENTITY;
    mapped[3] = [5., 6., 7., 0.];
    let mut new_world = IDENTITY;
    new_world[3] = [100., 200., 300., 0.];
    let com = [11., 22., 33., 0.];
    let mut retained = IDENTITY;
    let mut flags = 0;
    let target = prepare_pose(
        &mut roots,
        &mut frames,
        &animation,
        &mapped,
        &mut retained,
        super::super::Input {
            world_frame: &new_world,
            centre_of_mass_1056: com,
        },
        0,
        0,
        &mut flags,
    );
    assert_eq!(frames.skate_root[3], [14., 23., 28., 0.]);
    assert_eq!(frames.com_frame[3], com);
    for axis in 0..3 {
        for lane in 0..3 {
            assert!((frames.com_frame[axis][lane] - old[axis][lane]).abs() < 1e-5);
        }
    }
    assert_eq!(roots.animation_to_world, new_world);
    assert_eq!(retained, mapped);
    assert_eq!(target[3], [105., 206., 307., 0.]);
    assert_eq!(frames.physical_board, physical);

    //Holding2484bit0 preserves LOCAL16016 while a new root moves the target.
    new_world[3][0] = 110.;
    let target = prepare_pose(
        &mut roots,
        &mut frames,
        &animation,
        &IDENTITY,
        &mut retained,
        super::super::Input {
            world_frame: &new_world,
            centre_of_mass_1056: com,
        },
        0,
        1,
        &mut flags,
    );
    assert_eq!(retained, mapped);
    assert_eq!(target[3], [115., 206., 307., 0.]);
    assert_eq!(frames.physical_board, physical);
}
