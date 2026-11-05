//! Borrow the actual completed collision and physical owners; no default contacts.
use skate3_core::{
    air::reckoning::AirState,
    math::Vector3,
    physics::{
        board_ground::BoardGroundState, skeleton_animation_record::AnimationPartTransform,
        skeleton_body::SkeletonCollisionFeedback,
    },
    player::{
        input_phase::ProcessedPhysicsInput,
        wipeout::{Frame, RequestInput},
    },
};
pub(crate) struct Observations<'a> {
    pub processed: &'a ProcessedPhysicsInput,
    pub board: &'a BoardGroundState,
    pub collision: &'a SkeletonCollisionFeedback,
    pub deck: AnimationPartTransform,
    ///Actual Processed0/16/32 publication, supplied by its retained producer.
    pub input_board: AnimationPartTransform,
    pub world_to_animation: AnimationPartTransform,
    pub pose_error: [f32; 4],
    ///Skeleton16384 is unavailable before the first actual postphysics feedback.
    pub maximum_pose_error: Option<f32>,
    pub jump_fix_frames: u32,
    pub air: &'a AirState,
    ///Reckoning816's up column Y, original832.Y.
    pub system_up_y: f32,
    pub grind_locked_to_middle: bool,
    pub grind_normal: Option<[f32; 4]>,
}
impl Observations<'_> {
    pub(crate) fn frame(&self) -> Result<Frame, String> {
        let p = self.processed;
        let c = self.collision;
        let b = self.board;
        Ok(Frame {
            flags_2468: p.flags_2468,
            flags_2472: p.flags_2472,
            flags_2476: p.flags_2476,
            flags_2480: p.flags_2480,
            flags_2484: p.flags_2484,
            category: p.category_2512,
            timestep: p.timestep_2604,
            time_on_ground: p.time_on_ground_2752,
            speed: p.scalar_2652,
            animation_up: p.vectors_544_560_592_608[0].map(f32::from_bits),
            landing_angle: p.scalar_2736,
            deck_velocity: p.vectors_400_416[0].map(f32::from_bits),
            com_velocity: p.vectors_544_560_592_608[3].map(f32::from_bits),
            jump_fix_frames: self.jump_fix_frames as i32,
            deck: self.deck,
            input_board: self.input_board,
            world_to_animation: self.world_to_animation,
            closing_velocity: vector(b.closing_velocity),
            board_material_flags: b.collision_flags,
            //82C08494/82C08640 publish the actual counts to868/869.
            board_contact: b.part_contact_count != 0,
            wheel_contact: b.wheel_contact_count != 0,
            board_contact_normal: vector(b.overall_normal),
            opposing_contact: b.opposing_contact,
            regions_force: c.regions.map(|r| r.force),
            maximum_skater_force: c.maximum_skater_force,
            vehicle_force: c.maximum_group_8_force,
            group_8: c.flags.group_8,
            conflicting: c.flags.conflicting,
            compliant: c.flags.compliant,
            highest_normal: c.highest_normal,
            pose_error: self.pose_error,
            maximum_pose_error: self
                .maximum_pose_error
                .ok_or("Wipeout requires completed Skeleton pose-error feedback")?,
            flip_active: self.air.flip_active,
            flip_requested_speed: self.air.flip_requested_speed,
            system_up_y: self.system_up_y,
            grind_selected: self.grind_locked_to_middle,
            grind_normal_valid: self.grind_normal.is_some(),
            //Unused when invalid; retain native default up without publishing a hit.
            grind_normal: self.grind_normal.unwrap_or([0.0, 1.0, 0.0, 0.0]),
        })
    }
}
pub(super) fn request_input(p: &ProcessedPhysicsInput) -> RequestInput {
    RequestInput {
        flags_2468: p.flags_2468,
        flags_2476: p.flags_2476,
        flags_2480: p.flags_2480,
        flags_2484: p.flags_2484,
        animation_up_y: f32::from_bits(p.vectors_544_560_592_608[0][1]),
        category: p.category_2512,
    }
}
fn vector(v: Vector3) -> [f32; 4] {
    [v.x, v.y, v.z, 0.0]
}
