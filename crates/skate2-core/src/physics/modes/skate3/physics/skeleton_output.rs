//! Production adapter for the original physical-pose job82DB6698/82DB95E8.
use super::animated_skeleton::AnimatedSkeleton;
use skate3_core::{
    animation::foot_ik::drive::Geometry,
    physics::{
        board::BodyId,
        board_runtime::BoardRuntime,
        drive_frames::RetailAffineTransform,
        skeleton_animation_record::AnimationPartTransform as Transform,
        skeleton_body::SkeletonBody,
        skeleton_output::{self, board, wobble},
        truck_frames::steering_truck_transforms,
    },
    point_graph::PointGraph,
};
use skate_data::{animation_frames::AnimationFrames, collections::Collections};

pub(crate) struct SkeletonOutput {
    pub pose: skeleton_output::SkeletonOutput,
    pub wobble: wobble::Wobble,
    wobble_settings: wobble::Settings,
    pub deck_wobble: wobble::Output,
    pub correction: skeleton_output::correction::CorrectionState,
    compression_rest_height: f32,
}
impl SkeletonOutput {
    pub fn load(
        data: &Collections,
        animation: &AnimationFrames,
        skeleton: &AnimatedSkeleton,
    ) -> Result<Self, String> {
        let mut parents = [None; 24];
        for (part, &bone) in skeleton.bone_indices.iter().enumerate() {
            let mut ancestor = *animation
                .parents
                .get(bone)
                .ok_or("Missing output bone ancestor")?;
            let mut visited = 0;
            while ancestor >= 0 {
                let index = ancestor as usize;
                if index >= animation.parents.len() || visited >= animation.parents.len() {
                    return Err("Invalid physical output hierarchy".into());
                }
                if let Some(parent) = skeleton.bone_indices.iter().position(|&b| b == index) {
                    parents[part] = Some(parent);
                    break;
                }
                ancestor = animation.parents[index];
                visited += 1;
            }
        }
        let bone = |name: &str| {
            animation
                .bone_names
                .iter()
                .position(|n| n.eq_ignore_ascii_case(name))
                .ok_or_else(|| format!("Missing stock output bone {name}"))
        };
        let scalar = |key| data.float("physics_skeleton", "default", key);
        //82BD7DD0..7E88 names and stores these six ids; no authored index literals.
        let board_bones = board::BoneIndices {
            front_truck: bone("Truck_Front")?,
            back_truck: bone("Truck_Back")?,
            front_left_wheel: bone("Left_WheelFront")?,
            front_right_wheel: bone("Right_WheelFront")?,
            back_left_wheel: bone("Left_WheelBack")?,
            back_right_wheel: bone("Right_WheelBack")?,
        };
        let graph = |key| -> Result<PointGraph<8>, String> {
            let words = data
                .words::<20>("physics_deck_wobble", "default", key)?
                .map(f32::from_bits);
            Ok(PointGraph {
                x: words[4..12].try_into().unwrap(),
                y: words[12..20].try_into().unwrap(),
            })
        };
        Ok(Self {
            pose: skeleton_output::SkeletonOutput {
                bone_indices: skeleton.bone_indices,
                geometry: Geometry::new(parents, &skeleton.physics_frames)?,
                board_bones,
                board_settings: board::Settings {
                    truck_tilt_scalar: scalar("TruckTiltScalar")?,
                    truck_tilt_max_angle: scalar("TruckTiltMaxAngle")?,
                    truck_tilt_wobble_scalar: scalar("TruckTiltWobbleScalar")?,
                    truck_displacement_max: scalar("TruckDisplacementMax")?,
                },
            },
            wobble: wobble::Wobble::default(),
            deck_wobble: wobble::Output::default(),
            correction: skeleton_output::correction::CorrectionState::default(),
            //8289F128 global220 = AF781D7B614DAB50 physicstrucks/default;
            //82C08D04..18 copies layout16 to SkateboardBody8380.
            compression_rest_height: data.float("physicstrucks", "default", "TruckYPos")?,
            wobble_settings: wobble::Settings {
                takeoff_tilt: graph("TiltVsTimeTakeOff")?,
                landing_tilt: graph("TiltVsTimeLanding")?,
                takeoff_squish: graph("SquishVsTimeTakeOff")?,
                landing_squish: graph("SquishVsTimeLanding")?,
                maximum_time: data.float("physics_deck_wobble", "default", "MaxTime")?,
            },
        })
    }
    ///82BF2F18 is called on the actual landing/takeoff transition; reverse is
    ///the native stance decision passed by that transition's caller.
    pub fn trigger_wobble(&mut self, landing: bool, reverse: bool) {
        self.wobble.trigger(landing, reverse);
    }
    ///82C08968 observes the solved COM frames, not geometric part frames.
    pub fn average_compressions(&self, board: &BoardRuntime) -> [f32; 2] {
        let deck = to_matrix(board.body_transform(BodyId::Deck));
        let wheels = core::array::from_fn(|i| to_matrix(board.body_transform(BodyId::ORDER[i]))[3]);
        skate3_core::physics::contact_feedback::average_wheel_compressions(
            deck,
            wheels,
            self.compression_rest_height,
        )
    }
    ///Call in82BD83E0's post-wipeout-check phase, before the pose job.
    pub fn advance_wobble(&mut self, physical: &mut SkeletonBody, deck: &Transform) -> Transform {
        self.deck_wobble = self.wobble.update(&self.wobble_settings);
        if self.deck_wobble.sampled {
            wobble::apply(self.deck_wobble, &mut physical.record.pose[0]);
            physical.record.pose[0]
        } else {
            *deck
        }
    }
    pub fn publish(
        &self,
        skeleton: &AnimatedSkeleton,
        physical: &SkeletonBody,
        board_runtime: &BoardRuntime,
        base_trucks: [RetailAffineTransform; 2],
        current_truck_targets: [f32; 2],
        average_compression: [f32; 2],
        globals: &mut [Transform],
        locals: &mut [Transform],
    ) -> Result<(), String> {
        let bodies =
            core::array::from_fn(|i| to_matrix(board_runtime.body_transform(BodyId::ORDER[i])));
        let trucks = steering_truck_transforms(base_trucks, current_truck_targets).map(to_matrix);
        let physical_parts = core::array::from_fn(|i| physical.record.pose[i]);
        self.pose
            .publish(
                skeleton_output::Input {
                    physical_parts: &physical_parts,
                    world_to_animation: &skeleton.roots.world_to_animation,
                    board: board::Input {
                        bodies: &bodies,
                        skeleton_board: &physical_parts[0],
                        truck_frames: &trucks,
                        deck_wobble_tilt: self.deck_wobble.tilt,
                        deck_wobble_squish: self.deck_wobble.squish,
                        average_compression,
                    },
                },
                globals,
                locals,
            )
            .map_err(str::to_string)
    }
}
fn to_matrix(value: RetailAffineTransform) -> Transform {
    let mut result = [[0.0; 4]; 4];
    for axis in 0..3 {
        result[axis][..3].copy_from_slice(&value.basis.columns[axis]);
    }
    result[3] = [
        value.translation.x,
        value.translation.y,
        value.translation.z,
        0.0,
    ];
    result
}
