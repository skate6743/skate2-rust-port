//! TU3 BipedGround Sync82D31DC0, air decision82D32338 and board update82D324B0.
//! Call only after the Biped job completes. Geometry submitted here is consumed
//! by the next PreUpdate; this function never fabricates collision observations.
mod math;
use super::{
    ground_entry::{Frame, State, Vector},
    ground_query,
};
use math::*;

#[derive(Clone, Copy, Debug)]
pub struct CompletedMotion {
    pub contact_position_192: Vector,
    pub contact_flags_368: u32,
    pub physical_frame_848: Frame,
    pub animation_frame_912: Frame,
    pub vector_976: Vector,
    pub vector_992: Vector,
    pub vector_1008: Vector,
    pub vector_1040: Vector,
    pub vector_1056: Vector,
    pub launch_1076: bool,
    pub flag_1077: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct Processed {
    pub frame_192: Frame,
    pub position_592: Vector,
    pub flags_2476: u32,
    pub flags_2480: u32,
    pub flags_2484: u32,
    pub flags_2488: u32,
    pub elapsed_2664: f32,
    pub value_2852: f32,
    pub query_context: ground_query::QueryContext,
}
/// Actual state40 settings. Values are loaded attributes, with no inferred defaults.
#[derive(Clone, Copy, Debug)]
pub struct BoardSettings {
    pub extent_0: Vector,
    pub extent_16: Vector,
    pub offset_32: Vector,
    pub angle_436: f32,
    pub angle_440: f32,
    pub margin_444: f32,
    pub angle_452: f32,
    pub angle_456: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct Bounds {
    pub frame: Frame,
    pub extents: Vector,
}
#[derive(Clone, Copy, Debug)]
pub struct BoardLimits {
    pub margin: f32,
    pub angle_a: f32,
    pub angle_b: f32,
}
///82D2DCC0's seven vectors are all overwritten by Sync before82D811C8.
#[derive(Clone, Copy, Debug)]
pub struct ToolkitInput(pub [Vector; 7]);

/// Canonical physical owners implement these operations against live state.
/// Associated values carry the actual Biped launch packet and queried candidate;
/// they are not boolean observations supplied in advance of their producers.
pub trait Services: ground_query::GroundQueryScene {
    type Launch;
    type Candidate;
    fn processed(&self) -> Processed;
    ///82D2DB98 initialization then82D7BA78 with final argument false.
    fn prepare_biped_launch(&mut self) -> Self::Launch;
    fn skeleton_point_10960(&self) -> Vector;
    /// Store point at launch32, then execute82D6CA58 against BipedAir.
    fn launch_air(&mut self, launch: Self::Launch, point: Vector);
    fn update_skeleton(&mut self, frame: Frame, vector_1056: Vector);
    fn board_settings(&self) -> BoardSettings;
    fn board_flags_12836(&self) -> u8;
    ///82D2CF68 initialized candidate,82D4D150 mode0 and initially zero key.
    fn probe_board(&mut self, position: Vector) -> Option<Self::Candidate>;
    fn candidate_key_188(&self, candidate: &Self::Candidate) -> [u32; 2];
    /// Actual82BE3220 bone23 transform's position48.
    fn bone_23_position(&mut self) -> Vector;
    ///82E08DB8, including candidate projection and82E08EE8 geometry tests.
    fn classify_board(
        &mut self,
        candidate: &Self::Candidate,
        bone: Vector,
        bounds: Bounds,
        limits: BoardLimits,
    ) -> bool;
    ///82D74200 mode4. This runs even when probe/classification fails.
    fn query_board(&mut self, position: Vector, bounds: Bounds, limits: BoardLimits);
    fn bind_board(&mut self, key: [u32; 2]);
    /// Board action virtual44, with live manager12828 after binding.
    fn commit_bound_board(&mut self);
    /// Board action virtual24 with original processed frame and live manager fields.
    fn commit_free_board(&mut self, frame: Frame);
    ///82D749D0 followed by clearing manager12780.
    fn reset_board(&mut self);
    fn update_toolkit(&mut self, input: ToolkitInput);
    fn deck_half_wheelbase(&self) -> f32;
    /// Submit all seven real collision lines; retain until next PreUpdate consumes.
    fn submit_ground_query(&mut self, packet: ground_query::GroundQueryPacket);
}

///82D32338: special contact/input flags suppress only the explicit jump request.
/// Loss of contact and the controller's launch byte still trigger an air launch.
pub fn wants_air(state: &State, motion: &CompletedMotion, p: &Processed) -> bool {
    let mut launch =
        motion.contact_flags_368 & 1 == 0 && p.elapsed_2664 > f32::from_bits(0x3d4c_cccd);
    launch |= motion.launch_1076;
    if motion.contact_flags_368 & 8 == 0
        && p.flags_2480 & 0x0002_0180 == 0
        && !state.flags_144_to_150[6]
    {
        launch |= p.flags_2476 & 0x0008_0000 != 0;
    }
    launch
}

///82D2E250 with argument7=false: retain forward.w while flattening only y.
pub fn board_bounds(frame: Frame, offset: Vector, extents: Vector) -> Bounds {
    let mut forward = frame[2];
    forward[1] = 0.0;
    let forward = normalize_or(forward, [0.0, 0.0, 1.0, 0.0]);
    let up = [0.0, 1.0, 0.0, 0.0];
    let right = cross(up, forward);
    let position = madd(
        up,
        offset[1],
        madd(forward, offset[2] + extents[2], frame[3]),
    );
    Bounds {
        frame: [right, up, forward, position],
        extents,
    }
}

///82D324B0. The caller clears144; a failed probe deliberately preserves that byte.
pub fn update_board<S: Services>(state: &mut State, services: &mut S) {
    let settings = services.board_settings();
    let original_frame = services.processed().frame_192;
    let bounds = board_bounds(original_frame, settings.offset_32, settings.extent_16);
    let position = services.processed().position_592;
    if services.board_flags_12836() & 0x40 != 0 {
        if let Some(candidate) = services.probe_board(position) {
            // The probe may change live processed/settings state.
            let settings = services.board_settings();
            let frame = services.processed().frame_192;
            let inner = board_bounds(
                frame,
                settings.offset_32,
                scale(settings.extent_0, f32::from_bits(0x3f73_3333)),
            );
            let settings = services.board_settings();
            let limits = limits(settings.margin_444, settings.angle_452, settings.angle_436);
            let bone = services.bone_23_position();
            state.flags_144_to_150[0] = services.classify_board(&candidate, bone, inner, limits);
            if state.flags_144_to_150[0] {
                let key = services.candidate_key_188(&candidate);
                state.counter_152 = key[0];
                state.counter_156 = key[1];
            }
        }
    }
    let settings = services.board_settings();
    services.query_board(
        position,
        bounds,
        limits(settings.margin_444, settings.angle_456, settings.angle_440),
    );
    if state.flags_144_to_150[0] {
        services.bind_board([state.counter_152, state.counter_156]);
        services.commit_bound_board();
    } else {
        services.commit_free_board(original_frame);
    }
}
fn limits(margin: f32, a: f32, b: f32) -> BoardLimits {
    let radians = f32::from_bits(0x3c8e_fa35);
    BoardLimits {
        margin,
        angle_a: a * radians,
        angle_b: b * radians,
    }
}

/// Full Sync order; returns whether a new geometry packet was submitted.
/// Scene failure is propagated after the already completed physical updates.
pub fn sync<S: Services>(
    state: &mut State,
    motion: &CompletedMotion,
    services: &mut S,
) -> Result<bool, S::Error> {
    state.flags_144_to_150[2] = wants_air(state, motion, &services.processed());
    if state.flags_144_to_150[2] {
        let launch = services.prepare_biped_launch();
        let point = services.skeleton_point_10960();
        let frame = motion.animation_frame_912;
        let point = madd(
            frame[2],
            point[2],
            madd(frame[1], point[1], madd(frame[0], point[0], frame[3])),
        );
        services.launch_air(launch, point);
    }
    state.flags_144_to_150[0] = false;
    state.flags_144_to_150[3] = motion.flag_1077;
    state.frame_80 = motion.physical_frame_848;
    if motion.contact_flags_368 & 1 != 0 {
        let up = state.frame_80[1];
        let distance = dot(sub(motion.contact_position_192, state.frame_80[3]), up);
        if !(distance <= 0.0) {
            state.frame_80[3] = madd(up, distance, state.frame_80[3]);
        }
    }
    let mut animation = motion.animation_frame_912;
    if state.flags_144_to_150[6] {
        state.duration_180 -= f32::from_bits(0x3c88_8889);
        if !(state.duration_180 > 0.0) {
            state.duration_180 = 0.0;
            state.flags_144_to_150[6] = false;
        }
        state.angle_172 *= f32::from_bits(0x3f66_6666);
        let angle = (-state.angular_velocity_176).mul_add(state.duration_180, state.angle_172);
        let (sin, cos) = crate::trigonometry::sin_cos(angle);
        let forward = madd(
            [sin, 0.0, cos, sin],
            animation[2][2],
            madd(
                [0.0, 1.0, 0.0, 0.0],
                animation[2][1],
                scale([cos, 0.0, -sin, cos], animation[2][0]),
            ),
        );
        // Native branch has no zero fallback on either cross normalization.
        let right = normalize_unchecked(cross(animation[1], forward));
        animation[0] = right;
        animation[1] = normalize_unchecked(cross(forward, right));
        animation[2] = forward;
    }
    services.update_skeleton(animation, motion.vector_1056);
    let p = services.processed();
    if p.flags_2484 & 0x2000_0000 == 0 && p.flags_2480 & 0x0004_0000 == 0 && !(p.value_2852 > 0.0) {
        if p.flags_2476 & 0x0040_0000 != 0 {
            if !state.flags_144_to_150[4] {
                update_board(state, services);
            } else if !(p.elapsed_2664 <= 0.5) {
                state.flags_144_to_150[4] = false;
            }
        } else {
            services.reset_board();
        }
    }
    services.update_toolkit(ToolkitInput([
        state.frame_80[3],
        motion.vector_1008,
        motion.vector_992,
        motion.vector_976,
        motion.vector_1040,
        animation[1],
        animation[0],
    ]));
    let p = services.processed();
    let frame = query_frame(state.frame_80);
    let search = ground_query::edge_search(
        frame,
        p.query_context,
        xyz(motion.vector_1040),
        p.flags_2488,
    );
    let candidates = services.edge_candidates(&search)?;
    if let Some(edge) = ground_query::select_edge(search, &candidates) {
        if let Some(packet) = ground_query::prepare_packet(
            frame,
            p.query_context,
            edge,
            services.deck_half_wheelbase(),
        ) {
            services.submit_ground_query(packet);
            return Ok(true);
        }
    }
    Ok(false)
}
fn xyz(v: Vector) -> crate::math::Vector3 {
    crate::math::Vector3::new(v[0], v[1], v[2])
}
fn query_frame(f: Frame) -> ground_query::Frame {
    ground_query::Frame {
        right: xyz(f[0]),
        up: xyz(f[1]),
        forward: xyz(f[2]),
        position: xyz(f[3]),
    }
}
#[cfg(test)]
mod tests;
