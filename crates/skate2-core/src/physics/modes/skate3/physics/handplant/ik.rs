//! HandPlantManager82D633B0 and Skeleton82BD9728/97D0, shared four-limb IK.
use super::*;
pub(super) fn update(skater: &mut Skate3ControllerState, ground: bool) {
    let p = &skater.player_input.processed;
    let h = &mut skater.handplant;
    if ground && (p.flags_2480 & (1 << 29) != 0 || p.flags_2480 & (1 << 28) == 0) {
        h.reset_ik();
    }
    let right = (h.flags & 0x2000_0000 != 0) ^ (p.flags_2476 & (1 << 2) != 0);
    let side = usize::from(right);
    let limb = 2 + side;
    let animated = &skater.animated_skeleton;
    let world = |part: usize| {
        point(
            &animated.roots.animation_to_world,
            animated.record.pose[part][3],
        )
    };
    let mut hand = world(3 + side * 4);
    let shoulder = world(6 + side * 4);
    let attached = h.phase > -h.settings.hand_release;
    if !attached {
        h.ik_released = true;
    }
    let shoulder_delta = sub(shoulder, h.anchor);
    if !h.ik_released && length(shoulder_delta) > f32::from_bits(0x37800000) {
        let axis = normalize(shoulder_delta);
        let projection = dot(sub(hand, h.anchor), axis);
        if projection <= 0.0 {
            h.ik_released = true;
        } else {
            let on_axis = madd(axis, projection, h.anchor);
            if h.ik_latched {
                hand = on_axis;
            } else {
                let radial = sub(hand, on_axis);
                let radius = h.settings.hand_radius.evaluate(projection);
                if length(radial) > radius {
                    hand = madd(normalize(radial), radius, on_axis);
                }
            }
        }
        let delta = sub(hand, h.anchor);
        let distance = length(delta);
        if h.ik_distance < 0.0 {
            h.ik_distance = distance;
        }
        h.ik_distance = distance.min(h.ik_distance - h.settings.hand_approach);
        if h.ik_distance < f32::from_bits(0x37800000) || distance < f32::from_bits(0x37800000) {
            h.ik_released = true;
        } else {
            hand = madd(normalize(delta), h.ik_distance, h.anchor);
        }
        if !h.ik_released && !h.ik_latched {
            let on_axis = madd(axis, dot(sub(hand, h.anchor), axis), h.anchor);
            if length(sub(hand, on_axis)) < 0.02 {
                h.ik_latched = true;
            }
        }
    }
    if h.ik_released {
        hand = h.anchor;
    }
    let delta = sub(hand, shoulder);
    if length(delta) > 0.65 {
        hand = madd(normalize(delta), 0.65, shoulder);
    }
    h.ik_blend = clamp01(
        h.ik_blend
            + if attached {
                STEP / h.settings.hand_into
            } else {
                -STEP / h.settings.hand_out
            },
    );
    if h.ik_blend > 0.0 {
        //82BD9728 clamps against the authored reparented hand target.
        let original = point(
            &animated.roots.animation_to_world,
            animated.targets[limb][3],
        );
        let delta = sub(hand, original);
        if length(delta) > 0.65 {
            hand = madd(normalize(delta), 0.65, original);
        }
        skater.foot_ik.state.external_targets[limb].world_position = hand;
        skater.foot_ik.state.limbs[limb].external_target_set = true;
        skater.foot_ik.state.limbs[limb].target_blend = h.ik_blend;
    }
}
