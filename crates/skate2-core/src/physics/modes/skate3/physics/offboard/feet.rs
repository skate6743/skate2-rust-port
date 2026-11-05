//! State56 manager updates real foot targets before Skeleton GeneralUpdate.
use crate::skate3_native::physics::Skate3ControllerState;
use skate3_core::{physics::skeleton_animation_record::compose_affine, player::offboard::feet};
pub(crate) fn update_air(skater: &mut Skate3ControllerState) {
    let p = &skater.player_input.processed;
    let root = skater.animated_skeleton.roots.animation_to_world;
    let mut effective = root;
    if p.flags_2476 & 4 != 0 {
        for axis in [0, 2] {
            effective[axis] = effective[axis].map(|v| -v);
        }
    }
    let local_pairs = [15, 19].map(|index| {
        [
            skater.animated_skeleton.record.pose[index][3],
            skater.animated_skeleton.record.pose[index + 1][3],
        ]
    });
    let world_pairs = [15, 19].map(|index| {
        [
            compose_affine(&root, &skater.animated_skeleton.record.pose[index])[3],
            compose_affine(&root, &skater.animated_skeleton.record.pose[index + 1])[3],
        ]
    });
    for foot in &mut skater.offboard.feet.hands {
        foot.scalars_96_100[1] = 0.;
    }
    skater.offboard.feet.words_308_to_316 = [0; 3];
    let targets = feet::update(
        &mut skater.offboard.feet,
        &feet::Input {
            lines: std::array::from_fn(|i| {
                let line = p.line_tests_960_1008_1056[i];
                feet::Line {
                    position: line.position.map(f32::from_bits),
                    normal: line.normal.map(f32::from_bits),
                    surface: line.surface,
                    valid: line.valid != 0,
                }
            }),
            root,
            inverse_root: skater.animated_skeleton.roots.world_to_animation,
            effective_root: effective,
            local_foot_pairs: local_pairs,
            world_foot_pairs: world_pairs,
            position: p.vectors_544_560_592_608[2].map(f32::from_bits),
            velocity: p.vectors_544_560_592_608[3].map(f32::from_bits),
            flags_2476: p.flags_2476,
            flags_2480: p.flags_2480,
            flags_2484: p.flags_2484,
            state: p.state_2508,
        },
    );
    if p.flags_2484 & 0x80000000 == 0 {
        for (index, target) in targets.into_iter().enumerate() {
            let status = &mut skater.foot_ik.state.limbs[index];
            let external = &mut skater.foot_ik.state.external_targets[index];
            if target.world {
                external.world_position = target.position;
                status.external_target_set = true;
            } else {
                external.animation_position = target.position;
                status.local_target_set = true;
            }
            status.target_blend = target.blend;
            if let Some(normal) = target.normal {
                feet::set_normal(external, normal);
            }
        }
    }
}
