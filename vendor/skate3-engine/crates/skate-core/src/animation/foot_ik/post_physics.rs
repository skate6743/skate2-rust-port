//! PostWipeoutCheck's feet82BF0680, hands82BF0C28 and external82BEDC10.
use super::{
    drive::Geometry,
    physical_solve,
    post_contact::{self, SettingsPost},
    settings::Settings,
    state::{FootIkState, LIMBS},
};
use crate::physics::{
    native_arithmetic::dot3,
    skeleton_animation_record::{AnimationPartTransform as Transform, compose_affine},
    skeleton_body::SkeletonBody,
};

pub struct Input<'a> {
    pub state_id: u32,
    pub category_id: u32,
    ///Actual BoardBody868, used by original state200/201 hand/foot gates.
    pub board_body_flag_868: bool,
    pub wipeout: bool,
    pub flags_2468: u32,
    pub flags_2484: u32,
    ///Processed2664 and2520, read only when2468bit18 is set.
    pub value_2664: f32,
    pub state_2520: u32,
    pub world_to_animation: &'a Transform,
    ///Actual board GetPartTransform, replaced by wobble-adjusted record when sampled.
    pub board: &'a Transform,
}

pub fn update(
    state: &mut FootIkState,
    body: &mut SkeletonBody,
    geometry: &Geometry,
    settings: &Settings,
    post: &SettingsPost,
    input: Input<'_>,
) -> [bool; 4] {
    let mut updated = [false; 4];
    if input.state_id != 702 {
        let airborne = matches!(input.state_id, 200 | 201);
        if (!airborne || input.board_body_flag_868) && input.category_id != 500 {
            feet(state, body, geometry, settings, post, &input, &mut updated);
        }
        if airborne && input.board_body_flag_868 {
            for limb in 2..4 {
                if state.frames[limb].within_contact_bounds {
                    state.frames[limb].world =
                        compose_affine(input.board, &state.frames[limb].board);
                    let part = LIMBS[limb].part;
                    let offset = sub(state.frames[limb].world[3], body.record.pose[part][3]);
                    if physical_solve::solve(body, geometry, part, offset) {
                        body.set_part_transform(part, state.frames[limb].world);
                        updated[limb] = true;
                    }
                }
            }
        }
    }
    //Always run the external-target pass, including state702.
    for limb in 0..2 {
        if !(state.limbs[limb].external_blend <= 0.0) {
            let binding = LIMBS[limb];
            let adjacent = binding.parent_part.unwrap();
            let offset = sub(
                state.frames[limb].parent_world[3],
                body.record.pose[adjacent][3],
            );
            if physical_solve::solve(body, geometry, adjacent, offset) {
                body.set_part_transform(binding.part, state.frames[limb].world);
                body.set_part_transform(adjacent, state.frames[limb].parent_world);
                updated[limb] = true;
            }
        }
    }
    for limb in 0..4 {
        state.external_targets[limb].normal_set = false;
        state.limbs[limb].external_target_set = false;
        state.limbs[limb].local_target_set = false;
    }
    updated
}

fn feet(
    state: &mut FootIkState,
    body: &mut SkeletonBody,
    geometry: &Geometry,
    settings: &Settings,
    post: &SettingsPost,
    input: &Input<'_>,
    updated: &mut [bool; 4],
) {
    let mut effective_board = *input.board;
    if input.flags_2484 & (1 << 21) != 0 {
        effective_board[0] = effective_board[0].map(|v| v * -1.0);
        effective_board[1] = effective_board[1].map(|v| v * -1.0);
    }
    let up = effective_board[1];
    let up_y = up[0] * input.world_to_animation[0][1];
    let up_y = up[1].mul_add(input.world_to_animation[1][1], up_y);
    let up_y = up[2].mul_add(input.world_to_animation[2][1], up_y);
    let constrained = input.flags_2468 & (1 << 18) != 0;
    if input.flags_2484 & (1 << 20) != 0
        || post.minimum_board_up > up_y
        || constrained && input.value_2664 > f32::from_bits(0x3D23_D70A)
        || constrained && input.state_2520 == 500
    {
        return;
    }
    for limb in 0..2 {
        let binding = LIMBS[limb];
        let adjacent = binding.parent_part.unwrap();
        if state.frames[limb].within_contact_bounds && !input.wipeout {
            state.frames[limb].world = compose_affine(input.board, &state.frames[limb].board);
            state.frames[limb].parent_world =
                compose_affine(input.board, &state.frames[limb].parent_board);
            let offset = sub(
                state.frames[limb].parent_world[3],
                body.record.pose[adjacent][3],
            );
            if physical_solve::solve(body, geometry, adjacent, offset) {
                body.set_part_transform(binding.part, state.frames[limb].world);
                body.set_part_transform(adjacent, state.frames[limb].parent_world);
                updated[limb] = true;
            }
        } else if !state.limbs[limb].external_target_set {
            let mut foot = body.record.pose[binding.part];
            if dot3(foot[1], up) <= 0.0 {
                continue;
            }
            let Some(target) = post_contact::target(
                &effective_board,
                foot[3],
                body.record.pose[adjacent][3],
                state.limbs[limb].part_position,
                input.wipeout,
                settings,
                post,
            ) else {
                continue;
            };
            let distance = dot3(up, sub(target, foot[3]));
            let offset = up.map(|v| v * distance);
            if physical_solve::solve(body, geometry, adjacent, offset) {
                foot[3] = target;
                body.set_part_transform(binding.part, foot);
                let mut adjacent_frame = body.record.pose[adjacent];
                for lane in 0..4 {
                    adjacent_frame[3][lane] += offset[lane];
                }
                body.set_part_transform(adjacent, adjacent_frame);
                updated[limb] = true;
            }
        }
    }
}
fn sub(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    core::array::from_fn(|i| a[i] - b[i])
}
