//! Owned native ground contact/force services for the live BoardRuntime.
//! The coordinator supplies the processed physical packet in native phase order.
mod corrections;
mod entry;
mod state;
mod update;
pub(crate) use entry::GroundEntryTargets;
pub(crate) use state::GroundState;
pub(crate) use update::{GroundUpdateFrame, GroundUpdateTargets};
mod input;
mod pumping;
pub(crate) use input::GroundInputObservations;
pub(crate) use pumping::GroundPumping;
mod launch;
mod services;
mod settings;
pub(crate) use launch::{GroundLaunchInfo, GroundLaunchPhysical};
pub(crate) use services::{GroundControllers, GroundPhysicalFrame};
pub(crate) use settings::{GroundSettings, GroundProfiles};
use skate3_core::{
    math::Vector3,
    physics::{
        board::BodyId, board_runtime::BoardRuntime, deck_angular_correction,
        force_queue::QueuedPointForce,
    },
    point_graph::PointGraph,
    riding::{
        collision_response::{
            CollisionResponsePhysical, CollisionResponseSettings, collision_response,
        },
        ground_contact_response::{WallRidePhysical, WallRideSettings, wall_ride_response},
        grounded::state::board_types::{
            CollisionForceResponse, GroundContactFrame, GroundContactResponse,
        },
    },
};
use skate_data::collections::Collections;

pub(crate) struct GroundRuntime {
    retained_board_normal: [f32; 4],
    wall_ride: WallRideSettings,
    collision: CollisionResponseSettings,
    deck_center_to_truck: f32,
    launch_cone_x: f32,
    launch_cone_z: f32,
    pub contact: GroundContactResponse,
    pub collision_force: Option<CollisionForceResponse>,
}
impl GroundRuntime {
    /// Skateboard Reset82C060B0 publishes [0,1,0,0] to the retained
    /// toolkit normal192. Do not reset Ground contact-response state here.
    pub fn reset_board_toolkit(&mut self) {
        self.retained_board_normal = [0.0, 1.0, 0.0, 0.0];
    }
    pub fn load(data: &Collections) -> Result<Self, String> {
        let feet = |field| data.float("physics_feet", "default", field);
        let collision = |field| data.float("physics_collision", "default", field);
        Ok(Self {
            //SkateboardReset82C05F50 explicitly clears192..208, then sets196=1.
            retained_board_normal: [0.0, 1.0, 0.0, 0.0],
            launch_cone_x: data.float("physics_trajectory", "default", "ConeAngleX")?,
            launch_cone_z: data.float("physics_trajectory", "default", "ConeAngleZ")?,
            deck_center_to_truck: data.float("physics_grinds", "default", "DeckCenterToTruck")?,
            wall_ride: WallRideSettings {
                anti_gravity_vs_time: curve(data, "physics_feet", "WallRideAntiGravityVsTime")?,
                max_dot_floor_wall: feet("WallRideMaxDotFloorWall")?,
                foot_force_time: feet("WallRideFootForceTime")?,
                auto_jump_height: feet("WallRideAutoJumpHeight")?,
                max_time: feet("WallRideMaxTime")?,
                velocity_time_to_consider: feet("WallRideVelTimeToConsider")?,
                auto_jump_y_down_scalar: feet("WallRideAutoJumpYDownScalar")?,
                auto_jump_force: feet("WallRideAutoJumpForce")?,
            },
            collision: CollisionResponseSettings {
                maximum_velocity_delta: collision("MaxVelDelta")?,
                force_y_offset: collision("ForceYOffset")?,
                force_scalar: collision("CollisionForceScalar")?,
                target_displacement_velocity: collision("TargetDisplacementVel")?,
                torque_vs_angle: curve(data, "physics_collision", "CollisionTorqueVsAngle")?,
            },
            //PhysicsGround ctor/reset initializes this output vector tozero.
            contact: GroundContactResponse {
                active_2731: false,
                tag_16_force: QueuedPointForce {
                    tag: 16,
                    ..Default::default()
                },
                vector_2688: [0.; 4],
                scalar_2704: 0.,
                animated_board_2708: false,
            },
            collision_force: None,
        })
    }
    /// Slide82D3AA74..AA9C passes a fresh zero previous vector each update.
    /// This call does not overwrite Ground's separately retained contact state.
    pub fn contact_response_with_previous(
        &self,
        frame: GroundContactFrame,
        physical: WallRidePhysical,
        previous: [f32; 4],
    ) -> GroundContactResponse {
        wall_ride_response(&self.wall_ride, physical, frame, previous)
    }
    /// Called immediately after contact response and before animated/physical
    /// branch selection at82D389DC. This changes the solver's actual deck.
    pub fn update_body_accumulator(&mut self, board: &mut BoardRuntime) {
        deck_angular_correction::apply_ground_body_torque(
            &mut board.bodies_mut()[BodyId::Deck.index()].rates,
        );
    }
    /// Native82D944E8 output publication. Do not discard an existing ground
    /// collision vector on a false return; caller's collision-fade state owns it.
    pub fn calculate_collision_force(
        &mut self,
        physical: CollisionResponsePhysical,
    ) -> Option<CollisionForceResponse> {
        let result = collision_response(&self.collision, physical)?;
        if !result.applied {
            return None;
        }
        let output = CollisionForceResponse {
            force_2528: result.force,
            point_2544: result.point,
            vector_2592: result.angular_displacement,
        };
        self.collision_force = Some(output);
        Some(output)
    }
    pub fn apply_angular_target(&mut self, board: &mut BoardRuntime, displacement: [f32; 4]) {
        deck_angular_correction::apply_limited_displacement(
            &mut board.bodies_mut()[BodyId::Deck.index()].rates,
            xyz(displacement),
        );
    }
    pub fn apply_angular_displacement(&mut self, board: &mut BoardRuntime, displacement: [f32; 4]) {
        deck_angular_correction::apply_angular_displacement(
            &mut board.bodies_mut()[BodyId::Deck.index()].rates,
            xyz(displacement),
        );
    }
    ///82C04168 publishes the requested linear velocity to all seven real body
    /// parts, preserving angular velocity and accumulated forces.
    pub fn set_animated_velocity(&mut self, board: &mut BoardRuntime, velocity: [f32; 4]) {
        for body in board.bodies_mut() {
            body.rates.linear_velocity = xyz(velocity);
        }
    }
}
fn curve(data: &Collections, class: &str, field: &str) -> Result<PointGraph<8>, String> {
    let words = data.words::<20>(class, "default", field)?;
    Ok(PointGraph {
        x: std::array::from_fn(|i| f32::from_bits(words[4 + i])),
        y: std::array::from_fn(|i| f32::from_bits(words[12 + i])),
    })
}
fn xyz(v: [f32; 4]) -> Vector3 {
    Vector3::new(v[0], v[1], v[2])
}

mod surface;
pub(crate) use surface::{active_surface, surface_key};
