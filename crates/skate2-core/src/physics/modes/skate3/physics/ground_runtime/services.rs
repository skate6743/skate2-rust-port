//! Ground's physical operations execute against the live board, in core's
//! recovered order. Detached queue/inertia ownership is published before the
//! solver runs; no force, drag or steering output is left in a test-only object.
use super::{
    GroundRuntime, GroundSettings,
    launch::{GroundLaunchInfo, GroundLaunchPhysical},
};
use skate3_core::{
    physics::{
        board::BodyId,
        board_runtime::BoardRuntime,
        board_world::BoardWorld,
        contact::RetailContactMaterial,
        ground_hang_geometry::HangGeometryInput,
        manual::{controller::ManualAngleMeasurement, state::ManualState},
        skeleton_animation_record::SkeletonAnimationRecord,
    },
    riding::{
        collision_response::{CollisionResponsePhysical, signed_angle},
        ground_contact_response::{WallRidePhysical, wall_ride_response},
        ground_correction_math,
        grounded::{
            drag::BodyInertias,
            state::{
                board::{self, GroundBoardOutcome},
                board_types::{
                    CollisionForceResponse, GroundBoardComponents, GroundBoardInput,
                    GroundBoardServices, GroundContactFrame, GroundContactResponse,
                },
                corrections::{
                    AntiFlipNudgeMath, HalfpipeWheelCatchServices, HangUpServices, PinningServices,
                },
                data::PhysicsGroundState,
            },
        },
        speed_model::SpeedModelState,
        speed_wobble::SpeedWobbleState,
        steering::TruckSteeringState,
    },
};

pub(crate) struct GroundControllers<'a> {
    pub speed_wobble: &'a mut SpeedWobbleState,
    pub truck_steering: &'a mut TruckSteeringState,
    pub speed_model: &'a mut SpeedModelState,
    pub manual: &'a mut ManualState,
    pub heading_previous: &'a mut f32,
}
pub(crate) struct GroundPhysicalFrame<'a> {
    pub skeleton_record: &'a SkeletonAnimationRecord,
    pub wall_ride: WallRidePhysical,
    pub collision: CollisionResponsePhysical,
    pub hang_geometry: HangGeometryInput,
    pub processed_velocity: &'a mut [f32; 4],
    ///The same material record used by colliders::world_volumes for all4wheels.
    pub wheel_material: &'a mut RetailContactMaterial,
    pub wipeout: &'a mut skate3_core::player::wipeout::Requests,
    pub time_step: f32,
    ///Needed only when the native wall-jump branch is selected. Missing data
    ///is an error on that branch, never an invented physical skeleton pose.
    pub launch_physical: Option<GroundLaunchPhysical<'a>>,
    ///Own host call boundary for the full native TrajectorySelector Launch
    ///and Update. Callers must finish prediction before advancing this board.
    pub launch_and_update: &'a mut dyn FnMut(&GroundLaunchInfo) -> Result<(), String>,
}
struct BoardServices<'a, 'p> {
    runtime: &'a mut GroundRuntime,
    board: &'a mut BoardRuntime,
    world: &'a BoardWorld,
    settings: &'a GroundSettings,
    physical: GroundPhysicalFrame<'p>,
    launch: Option<GroundLaunchInfo>,
}
impl GroundRuntime {
    pub fn update_board(
        &mut self,
        board: &mut BoardRuntime,
        world: &BoardWorld,
        settings: &GroundSettings,
        state: &mut PhysicsGroundState,
        controllers: GroundControllers<'_>,
        mut input: GroundBoardInput,
        physical: GroundPhysicalFrame<'_>,
    ) -> Result<GroundBoardOutcome, String> {
        input.speed_wobble.activation_threshold = settings.wobble_activation;
        input.speed_wobble.amplitude_multiplier = settings.wobble_amplitude;
        let mut queue = std::mem::take(board.forces_mut());
        let mut inertias = std::array::from_fn::<_, 7, _>(|i| board.bodies()[i].inertia);
        let mut bindings = BodyInertias {
            part_inertia_indices: &[0, 1, 2, 3, 4, 5, 6],
            inertias: &mut inertias,
        };
        let mut services = BoardServices {
            runtime: self,
            board,
            world,
            settings,
            physical,
            launch: None,
        };
        let result = board::update(
            state,
            GroundBoardComponents {
                speed_wobble: controllers.speed_wobble,
                truck_steering: controllers.truck_steering,
                speed_model: controllers.speed_model,
                manual: controllers.manual,
                heading_previous: controllers.heading_previous,
                force_queue: &mut queue,
                inertias: &mut bindings,
            },
            settings.board(),
            input,
            &mut services,
        );
        //Only SetLinearDrag changes the detached inertia records. Publish that
        //field alone: direct body torque/velocity operations above are retained.
        for (body, inertia) in services.board.bodies_mut().iter_mut().zip(inertias) {
            body.inertia.linear_drag = inertia.linear_drag;
        }
        *services.board.forces_mut() = queue;
        result.map_err(|e| format!("Native Ground board update: {e:?}"))
    }
}
impl ManualAngleMeasurement for BoardServices<'_, '_> {
    type Error = String;
    fn angle_between(&mut self, a: [f32; 4], b: [f32; 4], axis: [f32; 4]) -> Result<f32, String> {
        Ok(signed_angle(super::xyz(a), super::xyz(b), super::xyz(axis)))
    }
}
impl AntiFlipNudgeMath for BoardServices<'_, '_> {
    type Error = String;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, String> {
        Ok(ground_correction_math::dot_product(a, b))
    }
    fn scale_to_magnitude(
        &mut self,
        v: [f32; 4],
        square: f32,
        size: f32,
    ) -> Result<[f32; 4], String> {
        Ok(ground_correction_math::scale_to_magnitude(v, square, size))
    }
}
impl HangUpServices for BoardServices<'_, '_> {
    type Error = String;
    fn build_hang_force(&mut self) -> Result<[f32; 4], String> {
        let g = self.physical.hang_geometry;
        Ok(self
            .runtime
            .build_hang_force(self.board, vector(g.edge_start), vector(g.edge_end)))
    }
    fn apply_hang_force(&mut self, f: [f32; 4]) -> Result<(), String> {
        self.runtime.apply_hang_force(self.board, f);
        Ok(())
    }
    fn detect_hung_up_geometry(&mut self) -> Result<bool, String> {
        self.runtime
            .detect_hung_up_geometry(self.world, self.physical.hang_geometry)
    }
    fn request_wipeout(&mut self) -> Result<(), String> {
        //82D3983C..54 writes byte32 (reason12), value104 and increments200.
        self.physical.wipeout.request(12, 0.0);
        Ok(())
    }
}
impl HalfpipeWheelCatchServices for BoardServices<'_, '_> {
    type Error = String;
    fn angular_displacement(&mut self) -> Result<[f32; 4], String> {
        let f = self.board.part_transforms()[BodyId::Deck.index()]
            .basis
            .columns;
        Ok(ground_correction_math::wheel_catch_displacement(
            [f[1][0], f[1][1], f[1][2], 0.],
            [f[2][0], f[2][1], f[2][2], 0.],
        ))
    }
    fn apply_angular_displacement(&mut self, v: [f32; 4]) -> Result<(), String> {
        self.runtime.apply_angular_displacement(self.board, v);
        Ok(())
    }
}
impl PinningServices for BoardServices<'_, '_> {
    type Error = String;
    fn pin_to_captured_position(&mut self, x: f32, z: f32) -> Result<(), String> {
        self.runtime
            .pin_to_position(self.board, x, z, self.physical.time_step);
        Ok(())
    }
}
impl GroundBoardServices for BoardServices<'_, '_> {
    type BoardError = String;
    type AnimatedPose = GroundLaunchInfo;
    fn center_of_mass_height_82d38838(&mut self) -> Result<f32, String> {
        Ok(ground_correction_math::center_of_mass_height(
            self.physical.skeleton_record.com_to_deck_world,
        ))
    }
    fn set_contact_wheel_materials(&mut self) -> Result<(), String> {
        *self.physical.wheel_material = self.settings.wheel_material;
        Ok(())
    }
    fn contact_response_82d93df0(
        &mut self,
        frame: GroundContactFrame,
        previous: [f32; 4],
    ) -> Result<GroundContactResponse, String> {
        self.runtime.contact = wall_ride_response(
            &self.runtime.wall_ride,
            self.physical.wall_ride,
            frame,
            previous,
        );
        Ok(self.runtime.contact)
    }
    fn update_body_accumulator_82d389dc(&mut self) -> Result<(), String> {
        self.runtime.update_body_accumulator(self.board);
        Ok(())
    }
    fn set_animated_velocity_82c04168(&mut self, v: [f32; 4]) -> Result<(), String> {
        self.runtime.set_animated_velocity(self.board, v);
        Ok(())
    }
    fn write_processed_velocity_400(&mut self, v: [f32; 4]) -> Result<(), String> {
        *self.physical.processed_velocity = v;
        Ok(())
    }
    fn build_animated_pose_82d33448(&mut self) -> Result<GroundLaunchInfo, String> {
        Ok(GroundLaunchInfo::default())
    }
    fn publish_animated_pose_82be33d0(&mut self, info: &GroundLaunchInfo) -> Result<(), String> {
        let physical = self.physical.launch_physical.as_ref().ok_or(
            "Ground wall jump requires actual Skeleton/Reckoning launch observations82BE33D0",
        )?;
        let mut info = info.clone();
        info.fill(
            physical,
            self.runtime.launch_cone_x,
            self.runtime.launch_cone_z,
        );
        self.launch = Some(info);
        Ok(())
    }
    fn update_external_player_82d67848(
        &mut self,
        _info: &GroundLaunchInfo,
        velocity: [f32; 4],
    ) -> Result<(), String> {
        self.launch
            .as_mut()
            .ok_or("Ground launch packet was not filled")?
            .wall_jump(velocity);
        Ok(())
    }
    fn commit_external_player_82d68800(&mut self) -> Result<(), String> {
        let info = self
            .launch
            .as_ref()
            .ok_or("Ground launch packet was not prepared")?;
        (self.physical.launch_and_update)(info)
    }
    fn finalize_animated_board_sk83_na_f_01a4(&mut self) -> Result<(), String> {
        //TU382B61BB8 is one blr instruction. This confirmed native no-op
        //does not replace the preceding required trajectory calculation.
        Ok(())
    }
    fn collision_force_82d944e8(
        &mut self,
        normal: [f32; 4],
    ) -> Result<Option<CollisionForceResponse>, String> {
        let mut physical = self.physical.collision;
        physical.ground_normal = normal;
        Ok(self.runtime.calculate_collision_force(physical))
    }
    fn collision_force_dot_velocity_82d38e18(
        &mut self,
        f: [f32; 4],
        v: [f32; 4],
    ) -> Result<f32, String> {
        Ok(self.runtime.collision_force_projection(f, v))
    }
    fn apply_vector_82c07000(&mut self, v: [f32; 4]) -> Result<(), String> {
        self.runtime.apply_angular_target(self.board, v);
        Ok(())
    }
    fn apply_angular_displacement_82c075b8(&mut self, v: [f32; 4]) -> Result<(), String> {
        self.runtime.apply_angular_displacement(self.board, v);
        Ok(())
    }
}
fn vector(v: skate3_core::math::Vector3) -> [f32; 4] {
    [v.x, v.y, v.z, 0.]
}
