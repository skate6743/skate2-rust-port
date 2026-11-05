//! Original82D6FD60 ->82D6F5F0 ->82D6F910, invoked by82D36728.
use super::{CandidatePose, Footplant, V, math::*};
use crate::skate3_native::physics::{
    air_trajectory::AirTrajectoryRuntime, animated_skeleton::AnimatedSkeleton, foot_ik::FootIk,
};
use skate3_core::{
    air::{
        known::KnownAirFootplantInput,
        trajectory::{QueryRequest, Trajectory},
    },
    physics::{
        board_toolkit::BoardToolkit,
        board_world::BoardWorld,
        skeleton_body::{SkeletonBody, SkeletonCollisionMode},
    },
    player::input_phase::ProcessedPhysicsInput,
};
pub(crate) struct FootplantFrame<'a> {
    pub processed: &'a ProcessedPhysicsInput,
    pub toolkit: &'a BoardToolkit,
    pub animated: &'a AnimatedSkeleton,
    pub body: &'a SkeletonBody,
    pub gravity: V,
    pub world: &'a BoardWorld,
    pub edges: &'a [skate3_core::physics::grind_contact::Primitive],
}
impl Footplant {
    pub fn update_candidate(&mut self, input: &KnownAirFootplantInput, frame: &FootplantFrame<'_>) {
        let root=&frame.animated.roots.animation_to_world;
        self.update_candidate_from_pose(input,&CandidatePose {
            inverse_effective:frame.toolkit.inverse_effective,
            physical_toes:[frame.body.record.pose[15][3],frame.body.record.pose[19][3]],
            physical_com:frame.animated.board_frames.centre_of_mass,
            animation_com:point(root,frame.animated.record.centre_of_mass),
            animation_toes:[15,19].map(|toe|point(root,frame.animated.record.pose[toe][3])),
        });
    }
    /// Same source calculation, independent of the character's solver owner.
    pub(crate) fn update_candidate_from_pose(&mut self, input: &KnownAirFootplantInput, pose: &CandidatePose) {
        self.candidate = false;
        let offset = [0.0, self.settings.deck_bounds_y_offset, 0.0, 0.0];
        let upper = add(self.settings.deck_bounds, offset);
        let lower = sub(offset, self.settings.deck_bounds);
        let toes = pose.physical_toes;
        let board = toes.map(|p| point(&pose.inverse_effective, p));
        let outside = |p: V| (0..3).any(|i| lower[i] > p[i] || p[i] > upper[i]);
        self.physical_com = madd(
            input.trajectory.velocity,
            STEP,
            pose.physical_com,
        );
        self.animation_com = madd(
            input.trajectory.velocity,
            STEP,
            pose.animation_com,
        );
        let side = if outside(board[0]) {
            0
        } else if outside(board[1]) {
            1
        } else {
            return;
        };
        let toe = [15, 19][side];
        self.selected_toe = Some(toe);
        self.candidate = true;
        self.selected_world = pose.animation_toes[side];
        self.selected_record = toes[side];
        self.leg_direction = normalize(sub(self.selected_record, self.physical_com));
    }
    pub fn update_launch(&mut self, input: &KnownAirFootplantInput) {
        use skate3_core::air::reckoning::clamp_vector_within_max_angle as limit;
        if !self.hit {
            self.contact_time = input.remaining_collision_time;
        }
        let future = madd(
            input.trajectory.acceleration,
            self.contact_time,
            input.trajectory.velocity,
        );
        let candidate = sub(normalize(future), input.landing_normal);
        let from = scale(self.current_up, -1.0);
        let angle = f32::from_bits(0x420c0000) * f32::from_bits(0x3c8efa35);
        let direction = normalize(limit(candidate, from, angle));
        //82D6F7A8's second pure limiter result is discarded; v125 retains direction.
        let _ = limit(sub(self.adjusted_contact, self.physical_com), from, angle);
        self.launch_direction = normalize(direction);
        self.launch_valid = true;
        self.request = Trajectory {
            position: madd(
                self.launch_direction,
                self.settings.leg_length_on_landing,
                self.physical_com,
            ),
            velocity: input.trajectory.velocity,
            //830BD330 initializer82F826B0 uses822F8B40, independently read original bytes.
            acceleration: [0.0, f32::from_bits(0xc11ccccd), 0.0, 0.0],
            duration: -1.0,
        };
    }
    pub fn consume_and_submit(
        &mut self,
        input: &KnownAirFootplantInput,
        frame: &FootplantFrame<'_>,
        ik: &mut FootIk,
        collision: &mut SkeletonCollisionMode,
    ) -> Result<(), String> {
        if self.consume_completed_query(input,frame.processed.flags_2480 & (1<<23)!=0,frame.edges,frame.toolkit.deck[3]) {
            self.update_pose(input,frame.animated,ik,collision);
        }
        let query=self.next_query(frame.processed.vectors_544_560_592_608[0].map(f32::from_bits),frame.gravity);
        let submitted=query.trajectory;
        let next=AirTrajectoryRuntime::query(frame.world,query)?;
        self.accept_completed_query(next,submitted);
        Ok(())
    }
    /// Consume only the preceding query; retain native contact/request ordering.
    pub(crate) fn consume_completed_query(&mut self,input:&KnownAirFootplantInput,requested:bool,
        edges:&[skate3_core::physics::grind_contact::Primitive],board_position:V) -> bool {
        self.hit = false;
        if self.enabled && self.result.contact_time >= 0.0 {
            self.contact_time = clamp01(self.result.contact_time);
            self.contact = self.result.contact_position;
            self.surface = self.result.surface;
            self.hit = true;
            self.nearby_edge(edges,board_position);
            self.adjusted_contact = madd(
                input.landing_normal,
                self.settings.foot_volume_y_offset,
                self.contact,
            );
            if !self.contact_active {
                if self.candidate
                    && self.contact_time > f32::from_bits(0x3de147ae)
                    && self.contact_time < 0.3
                {
                    self.contact_active = true;
                    self.active_elapsed = 0.0;
                } else {
                    self.clear_contact();
                }
            } else if self.active_elapsed + self.contact_time < 0.0 {
                self.contact_active = false;
                self.clear_contact();
                self.active_elapsed = 0.0;
            }
            if self.contact_active {
                self.requested |= requested;
                self.active_elapsed += STEP;
                return true;
            }
        }
        false
    }
    pub(crate) fn next_query(&mut self,up:V,gravity:V) -> QueryRequest {
        self.current_up=up;
        QueryRequest {
            trajectory:Trajectory {
                position:self.request.position,velocity:self.request.velocity,
                acceleration:gravity,duration:1.,
            },
            radius:f32::from_bits(0x3d23d70a),start_error:1.,end_error:1.,
        }
    }
    /// Synchronous source execution and the host predictor both publish the
    /// completed result at their own boundary; it is consumed on the next tick.
    pub(crate) fn accept_completed_query(&mut self,result:skate3_core::air::trajectory::QueryResult,submitted:Trajectory) {
        self.result=result;
        self.completed_trajectory=submitted;
        self.enabled=true;
    }
    fn clear_contact(&mut self) {
        self.surface = 0;
        self.hit = false;
        self.contact_time = 0.0; //82D6FA80/82D6FAC4, distinct from Reset's -1.
        self.contact = [0.0; 4];
    }

    ///82D6FBB0: query the real edge geometry within the native two-metre box.
    fn nearby_edge(&mut self, edges: &[skate3_core::physics::grind_contact::Primitive], board_position: V) {
        use skate3_core::{math::Vector3, player::offboard::{air_selector::ledge::filter_edges as visible_edges, ground_query::Edge}};
        let xyz = |v: V| Vector3::new(v[0], v[1], v[2]);
        let lanes = |v: Vector3| [v.x, v.y, v.z, 0.0];
        let edges: Vec<_> = edges.iter().filter(|e| (0..3).all(|i|
            e.start[i].min(e.end[i]) <= self.contact[i] + 2.0 && e.start[i].max(e.end[i]) >= self.contact[i] - 2.0
        )).map(|e| Edge { start: xyz(e.start), end: xyz(e.end) }).collect();
        let original_height = self.contact[1];
        let mut best = 0.25; //820C6D98
        for edge in visible_edges(&edges, board_position) {
            let start = lanes(edge.start);
            let delta = sub(lanes(edge.end), start);
            let raw_normal = cross(cross(delta, [0.0,1.0,0.0,0.0]), delta);
            let normal = if length(raw_normal) > f32::from_bits(0x3586_37bd) { normalize(raw_normal) } else { [0.0,1.0,0.0,0.0] };
            let Some(time) = skate3_core::air::trajectory::grind::descending_plane_time(self.completed_trajectory, start, normal) else { continue; };
            let arc = self.completed_trajectory.position_at(time);
            let edge_length = length(delta);
            let direction = if edge_length > f32::from_bits(0x3780_0000) { scale(delta, reciprocal(edge_length)) } else { delta };
            let point = madd(direction, dot(direction, sub(arc, start)).min(edge_length).max(0.0), start);
            let distance = length(sub(arc, point));
            //Globals3644 at822F943C permits at most 3cm below the triangle hit.
            if distance < best && point[1] - original_height > f32::from_bits(0xbcf5_c28f) {
                self.contact_time = time;
                self.current_up = normal;
                self.contact = point;
                self.surface = 0;
                self.hit = true;
                best = distance;
            }
        }
    }
}
