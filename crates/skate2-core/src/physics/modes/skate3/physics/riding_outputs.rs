//! Live board/query/solver outputs used by the riding gameplay owner.
//! Physical calculations stay in core; this owns stock settings and scheduling
//! state. Solved contacts and line probes remain distinct native inputs.
mod ground_input;
mod probes;
pub(crate) use probes::BoardProbes;
use skate3_core::{
    air::body_spin::{self, BodySpinState},
    graph::conditions::SpeedInputs,
    math::Vector3,
    physics::{
        board::BodyId,
        board_ground::{BoardGroundState, WheelLineHit, WheelLineState, wheel_lines},
        board_motion_output::BoardMotionOutput,
        board_runtime::BoardRuntime,
        board_world::BoardWorld,
    },
    point_graph::PointGraph,
    riding::{
        ground_orientation::{
            GroundOrientation, GroundOrientationInput, GroundOrientationSettings,
        },
        reckoning_frames::ReckoningFrames,
        speed_and_slope::SpeedAndSlopeSettings,
    },
};
use skate_data::collections::Collections;

/// Pose-side results which the native Skeleton/complete Reckoning transform own.
/// They cannot be inferred from the board body's center of mass or its up axis.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RidingPoseInputs {
    /// Skeleton11008 -> Processed752 in82BD8918.
    pub com_to_deck: Vector3,
    ///Processed2812, supplied by the actual PhysBodySpin animation attribute.
    pub body_spin: f32,
}

/// The Reckoning fields that PhysicsGround's job 82D4E250 computes on its own copy.
pub(crate) struct GroundReckoningJob {
    reckoning: GroundOrientation,
    frames: ReckoningFrames,
    body_spin: BodySpinState,
}

pub(crate) struct RidingOutputs {
    pub ground: BoardGroundState,
    pub wheel_lines: WheelLineState,
    pub probes: BoardProbes,
    pub reckoning: GroundOrientation,
    pub reckoning_frames: ReckoningFrames,
    pub body_spin: BodySpinState,
    pub motion: BoardMotionOutput,
    orientation_settings: GroundOrientationSettings,
    tilt_vs_rotation: PointGraph<8>,
    tilt_vs_slope: PointGraph<8>,
    speed_settings: SpeedAndSlopeSettings,
    maximum_ground_angle: f32,
    heading_adjust_factor: f32,
    pending_wheel_queries: Option<[Option<WheelLineHit>; 4]>,
}
impl RidingOutputs {
    ///Board82C0D680 resets CollisionInfo and both probes, preserving7692.
    pub fn reset_for_teleport(&mut self) {
        let elapsed = self.ground.time_without_wheel_contact;
        let drag = self.ground.wheel_angular_drag;
        self.ground = BoardGroundState::default();
        self.ground.time_without_wheel_contact = elapsed;
        self.ground.wheel_angular_drag = drag;
        self.wheel_lines = WheelLineState::default();
        self.probes.reset_results();
        //Original82C0D680 preserves query handles203C..2050. EndBoard
        //still consumes the batch submitted before this input-phase reset.
    }
    pub fn load(
        data: &Collections,
        board: &BoardRuntime,
        processed_flags_2468: u32,
    ) -> Result<Self, String> {
        let f = |field| data.float("physics_reckoning", "default", field);
        let control = |field| {
            data.words::<4>("physics_reckoning", "default", field)
                .map(|v| v.map(f32::from_bits))
        };
        let curve = |field| graph(data, "physics_reckoning", field);
        let deck_curve = data
            .words::<20>("physics_reckoning", "default", "DeckAngleUsageVsSpeed")?
            .map(f32::from_bits);
        let orientation_settings = GroundOrientationSettings {
            ground_normal_smoothing: control("GroundNormalSmoothing")?,
            up_vector_smoothing_slow: control("UpVectorSmoothingSlow")?,
            up_vector_smoothing_fast: control("UpVectorSmoothingFast")?,
            dynamic_up_vs_ground_y: curve("DynamicUpVsGroundY")?,
            ground_vector_blend: curve("GroundVectorBlendGraph")?,
            deck_angle_usage_vs_speed: PointGraph {
                x: deck_curve[4..12].try_into().unwrap(),
                y: deck_curve[12..20].try_into().unwrap(),
            },
            up_vector_smoothing_vs_speed: curve("UpVectorSmoothingVsSpeed")?,
            up_vector_max_delta_vs_speed: curve("UpVectorMaxDeltaVsSpeed")?,
            ground_blend_max_delta: f("GroundVMaxBlendDelta")?,
            up_vector_max_acceleration: f("UpVectorMaxAcceleration")?,
            anti_wobble_damping: f("UpVectAntiWobbleDamping")?,
            extra_side_damping: f("ExtraSideDamping")?,
            minimum_wheels_for_ground_blend: data.integer(
                "physics_reckoning",
                "default",
                "MinWheelsToUseGroundVector",
            )? as i32,
        };
        let speed_settings = SpeedAndSlopeSettings {
            turn_torque_vs_speed: graph(data, "physics_heading", "TurnTorqueVsSpeed")?,
            turn_torque_vs_slope: graph(data, "physics_heading", "TurnTorqueVsSlope")?,
            heading_adjust_max_speed: data.float(
                "physics_heading",
                "default",
                "HeadingAdjustMaxSpeed",
            )?,
        };
        let reckoning = GroundOrientation::new(&orientation_settings);
        let motion =
            BoardMotionOutput::from_board(board, reckoning.ground_normal, processed_flags_2468);
        let ground = BoardGroundState::default();
        let heading_adjust_factor =
            speed_settings.calculate(ground.wheel_normal.y, motion.forward_speed);
        Ok(Self {
            ground,
            wheel_lines: WheelLineState::default(),
            probes: BoardProbes::default(),
            reckoning,
            reckoning_frames: ReckoningFrames::new(),
            body_spin: BodySpinState::new(),
            motion,
            orientation_settings,
            tilt_vs_rotation: curve("TiltVsRotGround")?,
            tilt_vs_slope: curve("TiltVsSlopeGround")?,
            speed_settings,
            maximum_ground_angle: f("MaxAllowedGroundNormalFromUp")?,
            heading_adjust_factor,
            pending_wheel_queries: None,
        })
    }

    ///Original82DB5E10 reads current Processed464.y and2612 after Skeleton.
    pub fn update_input_heading(&mut self, normal_y: f32, speed: f32) -> f32 {
        self.heading_adjust_factor = self.speed_settings.calculate(normal_y, speed);
        self.heading_adjust_factor
    }

    pub fn heading_adjust_factor(&self) -> f32 {
        self.heading_adjust_factor
    }

    /// PreUpdate 82D38018 copies the live Reckoning to PhysicsGround+896 (82D380E8) and runs
    /// 82D4E250 on that copy, so Update 82D37C88 (the world's 3-State pass) still reads the
    /// previous tick's Reckoning: CalcManualEffect's reference frame, the heading frame, the
    /// ground normal and the pumping axes. This computes the job now and keeps its result aside;
    /// the live fields stay as they were until `complete_ground_reckoning`.
    pub fn submit_ground_reckoning(
        &mut self,
        board: &BoardRuntime,
        pose: RidingPoseInputs,
        processed_flags_2468: u32,
        animation_balance: f32,
        coffin: bool,
        processed: &skate3_core::player::input_phase::ProcessedPhysicsInput,
    ) -> GroundReckoningJob {
        let reckoning = self.reckoning.clone();
        let frames = self.reckoning_frames.clone();
        let body_spin = self.body_spin.clone();
        self.update_ground_reckoning(board, pose, processed_flags_2468, animation_balance, coffin, processed);
        GroundReckoningJob {
            reckoning: std::mem::replace(&mut self.reckoning, reckoning),
            frames: std::mem::replace(&mut self.reckoning_frames, frames),
            body_spin: std::mem::replace(&mut self.body_spin, body_spin),
        }
    }

    /// The world's Post 3-State pass (PhysicsGround slot 3, 82D37F38) waits on the job and copies
    /// PhysicsGround+896 back over the live Reckoning, before the hand plant IK and
    /// UpdatePhysicsError, and before the engine step.
    pub fn complete_ground_reckoning(&mut self, job: GroundReckoningJob) {
        self.reckoning = job.reckoning;
        self.reckoning_frames = job.frames;
        self.body_spin = job.body_spin;
    }

    ///Ground preupdate82D37C50 ->82D38018 submits82D4E250. The host computes
    ///that job directly: its heading seed, BodySpin, up filters, full transform,
    ///dynamic lean and tilt. This must complete before ground forces consume it.
    pub fn update_ground_reckoning(
        &mut self,
        board: &BoardRuntime,
        pose: RidingPoseInputs,
        processed_flags_2468: u32,
        animation_balance: f32,
        coffin: bool,
        processed: &skate3_core::player::input_phase::ProcessedPhysicsInput,
    ) {
        let deck = board.part_transforms()[BodyId::Deck.index()];
        // Ordinary ground riding follows the physical deck (82D4E250).
        self.update_ground_reckoning_with_heading(
            board, pose, processed_flags_2468, animation_balance, coffin, processed,
            [deck.basis.columns[2][0], deck.basis.columns[2][1], deck.basis.columns[2][2], 0.0],
        );
    }

    /// Animated takeoff retains the rider's heading while the deck begins its
    /// authored spin. Feeding the driven deck axis back into this frame would
    /// apply that spin to the whole skater again on each ground-animation tick.
    pub fn update_ground_reckoning_with_heading(
        &mut self,
        board: &BoardRuntime,
        pose: RidingPoseInputs,
        processed_flags_2468: u32,
        animation_balance: f32,
        coffin: bool,
        processed: &skate3_core::player::input_phase::ProcessedPhysicsInput,
        heading: [f32; 4],
    ) {
        let observations = ground_input::GroundPacketInputs::from_processed(processed);
        let deck = board.part_transforms()[BodyId::Deck.index()];
        //The source takes the previous final frame X for the damping step.
        let previous_right = self.reckoning_frames.system[0];
        #[cfg(debug_assertions)]
        super::dev_trace::event("ground_reckoning_inputs", format!("heading={heading:?} pose={pose:?} flags={processed_flags_2468:08x} balance={animation_balance} coffin={coffin} wheels={} speed={} dt={} ground={:?} dynamic_up={:?} board_up={:?} previous_up={:?} previous_up_velocity={:?} previous_right={previous_right:?}", processed.wheel_count_2556,observations.speed,processed.timestep_2604,observations.wheel_normal,observations.dynamic_up,deck.basis.columns[1],self.reckoning.up,self.reckoning.up_velocity));
        self.reckoning_frames.heading = heading;
        body_spin::update_ground(&mut self.body_spin, pose.body_spin);
        let effective = BoardMotionOutput::from_board(
            board,
            self.reckoning.ground_normal,
            processed_flags_2468,
        );
        self.reckoning.update(
            &self.orientation_settings,
            GroundOrientationInput {
                com_to_deck: pose.com_to_deck,
                ground_normal: observations.wheel_normal,
                dynamic_up: observations.dynamic_up,
                speed: observations.speed,
                wheel_contact_count: observations.wheel_count,
                animation_balance,
                deck_angle_curve_input: observations.absolute_speed,
                board_up: vector(deck.basis.columns[1]),
                board_forward: vector(deck.basis.columns[2]),
                effective_board_forward: vector(effective.effective_basis.columns[2]),
                previous_reckoning_right: Vector3::new(
                    previous_right[0],
                    previous_right[1],
                    previous_right[2],
                ),
                prevent_up_behind_board: coffin,
            },
        );
        #[cfg(debug_assertions)]
        super::dev_trace::event("before_reckoning_transform", format!("up={:?} heading={:?} target={:?} up_velocity={:?} ground_normal={:?} body_flip={:?}",self.reckoning.up,self.reckoning_frames.heading,self.reckoning.target,self.reckoning.up_velocity,self.reckoning.ground_normal,self.reckoning_frames.body_flip));
        let up = lanes(self.reckoning.up);
        self.reckoning_frames
            .calculate_transform(up, lanes(self.reckoning.ground_normal));
        self.reckoning_frames
            .calculate_dynamic_lean(up, lanes(self.reckoning.dynamic_up));
        self.reckoning_frames.calculate_tilt(
            processed_flags_2468 & 0x100000 != 0,
            &self.tilt_vs_rotation,
            &self.tilt_vs_slope,
        );
    }

    /// Slide82D3A890 seeds1136/1200 from Processed528/96 and calls82D8C8F0
    /// directly. Its data block is exactly the Processed snapshot82D8E5C0.
    pub fn update_slide_reckoning(
        &mut self,
        p: &skate3_core::player::input_phase::ProcessedPhysicsInput,
        toolkit: &skate3_core::physics::board_toolkit::BoardToolkit,
        physical_body_spin: f32,
        balance: f32,
    ) {
        let raw = |v: [u32; 4]| {
            let v = v.map(f32::from_bits);
            Vector3::new(v[0], v[1], v[2])
        };
        let xyz = |v: [f32; 4]| Vector3::new(v[0], v[1], v[2]);
        let previous_right = self.reckoning_frames.system[0];
        self.reckoning_frames.heading = toolkit.deck[2];
        self.reckoning.dynamic_up = raw(p.vectors_464_480_496_512_528[4]);
        body_spin::update_ground(&mut self.body_spin, physical_body_spin);
        self.reckoning.update(
            &self.orientation_settings,
            GroundOrientationInput {
                com_to_deck: raw(p.animation_com_to_deck_752),
                ground_normal: raw(p.vectors_464_480_496_512_528[0]),
                dynamic_up: raw(p.vectors_464_480_496_512_528[4]),
                speed: p.scalar_2652,
                wheel_contact_count: p.wheel_count_2556 as i32,
                animation_balance: balance,
                deck_angle_curve_input: toolkit.absolute_speed,
                board_up: xyz(toolkit.deck[1]),
                board_forward: xyz(toolkit.deck[2]),
                effective_board_forward: xyz(toolkit.effective[2]),
                previous_reckoning_right: xyz(previous_right),
                prevent_up_behind_board: p.flags_2476 & 0x4000_0000 != 0,
            },
        );
        let up = lanes(self.reckoning.up);
        self.reckoning_frames
            .calculate_transform(up, lanes(self.reckoning.ground_normal));
        self.reckoning_frames
            .calculate_dynamic_lean(up, lanes(self.reckoning.dynamic_up));
        self.reckoning_frames.calculate_tilt(
            p.flags_2468 & 0x100000 != 0,
            &self.tilt_vs_rotation,
            &self.tilt_vs_slope,
        );
    }

    /// StartSkateboardLineTests82DB6310 ->82C07788: four current wheel positions,
    /// each queried0.2m along negative Reckoning up, radius0 and static-world
    /// backend. World8275EA20 submits these before actor SetUpPhysics.
    pub fn start_wheel_queries(
        &mut self,
        board: &BoardRuntime,
        world: &BoardWorld,
    ) -> Result<(), String> {
        if self.pending_wheel_queries.is_some() {
            return Err("Wheel queries were started twice without result publication".into());
        }
        let lines = wheel_lines(board, self.reckoning.up);
        let mut results = [None; 4];
        for (i, line) in lines.into_iter().enumerate() {
            results[i] = world
                .query_thin_line(line.start, line.end)
                .map_err(|e| format!("Wheel{i} stock line query: {e}"))?
                .map(|hit| WheelLineHit {
                    fraction: hit.geometry.fraction,
                    normal: hit.geometry.normal,
                    surface_tag: hit.tag,
                });
        }
        self.probes.start(board, world)?;
        self.pending_wheel_queries = Some(results);
        Ok(())
    }

    ///World8275EB10 calls EndBoard8275E6E8 after PlayerInput and before
    ///PostInput/state work. These results are available to this frame's state
    ///selector, independently of the subsequent solve's contact reports.
    pub fn finish_wheel_queries(&mut self) -> Result<(), String> {
        let hits = self
            .pending_wheel_queries
            .take()
            .ok_or("EndBoard requires its submitted wheel query batch")?;
        self.wheel_lines.publish(hits);
        self.probes.publish()
    }

    ///UpdatePostPhysics82C07D20 consumes current solved reports.
    ///Contact-dependent inertia drag affects the next step. ProcessOutput
    ///82C02A80 publishes actual body rates with filtered normal1216.
    /// `groups`: the collision group of each attached body other than the skater's own, by its
    /// index in the shared solve.
    pub fn finish_post_physics(
        &mut self,
        board: &mut BoardRuntime,
        board_wiping_out: bool,
        processed_flags_2468: u32,
        time_step: f32,
        groups: &std::collections::BTreeMap<usize, u32>,
    ) -> Result<(), String> {
        // SKATE2_DIAGNOSTIC_NO_BOARD_GROUPS: classify board contacts without the other bodies'
        // groups (the pre-82C07D20 behaviour), for A/B comparisons.
        static NO_GROUPS: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        let no_groups = *NO_GROUPS.get_or_init(|| std::env::var_os("SKATE2_DIAGNOSTIC_NO_BOARD_GROUPS").is_some());
        self.ground.update_with_groups(
            board.contact_reports(),
            &self.wheel_lines,
            self.reckoning.up,
            self.maximum_ground_angle,
            board_wiping_out,
            board.body_transform(BodyId::Deck),
            |index| if no_groups { None } else { groups.get(&index).copied() },
        );
        // SKATE2_TRACE_SKATE3_WIPEOUT: each tick a board contact sets a group flag (bits 26..30).
        static TRACE_GROUPS: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        if self.ground.collision_flags & 0x7c00_0000 != 0
            && *TRACE_GROUPS.get_or_init(|| std::env::var_os("SKATE2_TRACE_SKATE3_WIPEOUT").is_some())
        {
            eprintln!("skate3 board group contact: flags={:08x}", self.ground.collision_flags);
        }
        self.ground.sample_accelerations(
            core::array::from_fn(|i| board.bodies()[i].rates.linear_velocity),
            time_step,
        );
        self.ground.advance_contact_time(time_step);
        for (body, drag) in board.bodies_mut()[..4]
            .iter_mut()
            .zip(self.ground.wheel_angular_drag)
        {
            // S3 82C08634 writes inertia+36; S2 82B372E0 names this
            // mAngularDrag. Linear drag belongs to separate state controls.
            body.inertia.angular_drag = drag;
        }
        self.motion = BoardMotionOutput::from_board(
            board,
            self.reckoning.ground_normal,
            processed_flags_2468,
        );
        Ok(())
    }

    pub fn graph_speeds(&self) -> SpeedInputs {
        SpeedInputs {
            speed: self.motion.ground_speed,
            forward_speed: self.motion.forward_speed,
            speed_and_slope: self.heading_adjust_factor,
        }
    }
}
fn vector(v: [f32; 3]) -> Vector3 {
    Vector3::new(v[0], v[1], v[2])
}
fn lanes(v: Vector3) -> [f32; 4] {
    [v.x, v.y, v.z, 0.0]
}
fn graph(data: &Collections, class: &str, field: &str) -> Result<PointGraph<8>, String> {
    let words = data
        .words::<16>(class, "default", field)?
        .map(f32::from_bits);
    Ok(PointGraph {
        x: words[..8].try_into().unwrap(),
        y: words[8..].try_into().unwrap(),
    })
}
