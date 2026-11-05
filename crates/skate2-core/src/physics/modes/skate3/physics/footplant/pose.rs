//! Footplant82D70070 publishes real FootIK targets and temporary collision disables.
use super::{Footplant, PoseTarget, math::*};
use crate::skate3_native::physics::{animated_skeleton::AnimatedSkeleton, foot_ik::FootIk};
use skate3_core::{
    air::known::KnownAirFootplantInput, physics::skeleton_body::SkeletonCollisionMode,
};
impl Footplant {
    pub(super) fn update_pose(
        &mut self,
        input: &KnownAirFootplantInput,
        skeleton: &AnimatedSkeleton,
        ik: &mut FootIk,
        collision: &mut SkeletonCollisionMode,
    ) {
        if let Some(target)=self.calculate_pose_target(input,&skeleton.roots.world_to_animation) {
            let side=if target.toe==15 {0} else {1};
            ik.state.external_targets[side].animation_position=target.animation_position;
            ik.state.limbs[side].local_target_set=true;
            ik.state.limbs[side].target_blend=target.blend;
            if target.disable_collision {
                for part in if side==0 {[15,16,17]} else {[19,20,21]} {
                    collision.pending_reenable=true;
                    collision.disable_count[part]=2;
                    collision.parts[part].enabled=false;
                }
            }
        }
    }
    /// Source manager math without a dependency on either character's IK owner.
    /// Applying the returned writes commutes with the remaining manager stores:
    /// those stores read no IK/collision fields, and the native writes cannot fail.
    pub(crate) fn calculate_pose_target(&mut self,input:&KnownAirFootplantInput,
        world_to_animation:&skate3_core::animation::output::NativeMatrix) -> Option<PoseTarget> {
        let mut output=None;
        //82453298, shared source-derived polynomial and independent PC estimate seed.
        let angle = skate3_core::trigonometry::acos(dot(self.launch_direction, self.leg_direction));
        let turns = angle * f32::from_bits(0x3e22f983);
        let fraction = turns - turns.floor();
        let wrapped =
            (fraction - if fraction > 0.5 { 1.0 } else { 0.0 }) * f32::from_bits(0x40c90fdb);
        if self.requested && self.candidate && self.contact_time < 0.3 {
            let contact_length = length(sub(self.adjusted_contact, self.animation_com));
            let leg_length = length(sub(self.selected_world, self.animation_com));
            let distance = if leg_length < contact_length {
                leg_length
            } else {
                contact_length
            };
            let inverse = world_to_animation;
            let com = point(inverse, self.animation_com);
            let velocity = rotate(inverse, scale(input.trajectory.velocity, -STEP));
            let direction = rotate(inverse, self.launch_direction);
            let mut target = madd(direction, distance, add(com, velocity));
            if self.lock_valid {
                target = add(
                    self.locked_target,
                    clamp_length(sub(target, self.locked_target), 0.15),
                );
            }
            self.locked_target = target;
            self.lock_valid = true;
            output=Some(PoseTarget {
                toe: if self.selected_toe==Some(15) {15} else {19},
                animation_position:target,
                blend:self.target_blend,
                disable_collision:self.contact_time<0.2,
            });
            self.target_blend = clamp01(self.target_blend + 0.12);
        }
        if self.requested {
            self.perform = self.hit
                && !(self.contact_time > 0.017)
                && self.settings.max_leg_angle_error * f32::from_bits(0x3c8efa35) > wrapped;
        } else {
            self.target_blend = 0.0;
            self.perform = false;
        }
        output
    }
}
