//! Ground-only conditioning82D77330 and history82D775D8 around the shared
//! FeetIK evaluator. Air's per-update timer resets do NOT belong here.
use skate3_core::{
    animation::foot_ik::state::FootIkState,
    player::offboard::{biped_air::recovered::feet as core, board_possession::manager::State},
};

/// The host supplies actual current bone/line/root observations to core::Input.
/// This borrows state56; it never creates or retains another FeetIK owner.
pub(crate) fn update(manager: &mut State, mut input: core::Input, ik: &mut FootIkState) {
    //82D77330 only conditions when BOTH incoming line tests are valid.
    if input.lines.iter().all(|line| line.valid) {
        let d = std::array::from_fn::<_, 2, _>(|i| {
            input.lines[i].position[1] - input.world_foot_pairs[i][0][1]
        });
        for i in 0..2 {
            if d[i].abs() > 0.3 && (d[i] - d[1 - i]).abs() > 0.3 {
                input.lines[i].valid = false;
            }
        }
    }
    let targets = core::update(manager, &input);
    //82D77558 applies IK before Ground correction/history82D775D8.
    if input.flags_2484 & 0x8000_0000 == 0 {
        for (i, target) in targets.into_iter().enumerate() {
            let limb = &mut ik.limbs[i];
            let external = &mut ik.external_targets[i];
            if target.world {
                external.world_position = target.position;
                limb.external_target_set = true;
            } else {
                external.animation_position = target.position;
                limb.local_target_set = true;
            }
            limb.target_blend = target.blend;
            if let Some(normal) = target.normal {
                core::set_normal(external, normal);
            }
        }
    }
    update_history(manager);
}

fn update_history(s: &mut State) {
    //Both invalid deliberately leaves316 unchanged;312 still increments.
    let mut both_bad = true;
    if s.hands.iter().any(|foot| foot.flag_84) {
        s.words_308_to_316[2] = s.words_308_to_316[2].wrapping_add(1);
        for foot in &s.hands {
            let height = foot.vectors_0_to_64[1][1] - foot.vectors_0_to_64[3][1];
            if foot.flag_84 && !(height > 0.18) {
                s.words_308_to_316[2] = 0;
            }
            let bad = !foot.flag_84 || height > 1. || foot.vectors_0_to_64[4][1] < 0.6;
            both_bad &= bad;
        }
    }
    if both_bad {
        s.words_308_to_316[1] = s.words_308_to_316[1].wrapping_add(1);
        if s.words_308_to_316[1] as i32 > 20 || s.words_308_to_316[2] as i32 > 40 {
            s.flags_304_to_307[1] = true;
        }
    } else {
        s.flags_304_to_307[1] = false;
        s.words_308_to_316[1] = 0;
    }
    s.words_308_to_316[0] = 0;
    s.flags_304_to_307[2] = false;
    s.vectors_224_to_272[0] = [0.; 4];
    s.vectors_224_to_272[1] = [0.; 4];
}
