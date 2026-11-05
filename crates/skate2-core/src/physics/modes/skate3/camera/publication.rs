//! Game owners -> normal camera subject82DF69C0. Call after physical output.
//! Missing publications are explicit inputs, never inferred from render pose.
use super::{graph_subject::CameraGraphSubject, subject::CameraSubjectSnapshot};
use crate::skate3_native::physics::{Skate3PhysicsState, Skate3ControllerState};
use skate3_core::{
    camera::{
        AnchorInputs, Anchors, CompassPoseInputs, ManagerSubject, ReferencePointInputs, Subject,
        SubjectPoseInputs,
    },
    physics::{board::BodyId, skeleton_animation_record::AnimationPartTransform as Transform},
};
use crate::skate3_native::trace::debug;

/// State record fields absent from the input phase's smaller CurrentStateFields.
/// The same output/reset owner must publish these; there is no camera default.
pub(crate) struct CameraStateOutput {
    pub height_32: f32,
    pub physically_pushing_55: u8,
    pub wiping_out_59: u8,
    pub manual_60: u8,
    pub reset_62: u8,
    pub use_skeleton_root_75: u8,
    pub flag_79: u8,
    pub flag_81: u8,
}

pub(crate) struct CameraAnimationOutput {
    /// PhysOutAnimation32..60 from the existing turn conditioner, once per tick.
    pub conditioned_turn: [f32; 8],
    pub input_turn_64: f32,
    pub input_kickturn_68: f32,
    pub time_since_input_128: f32,
    pub wipeout_tweak_148: u32,
    pub stance_155: u8,
    pub running_out_160: u8,
    /// Actor's actual SkaterAnim component virtual28, copied at82DF6EA4.
    pub skater_animation_stance: u8,
}

/// Air output2. The ordinary Ground cancellation/reset owns its retained
/// values and validity; a false validity flag does not erase these vectors.
pub(crate) struct CameraAirOutput {
    pub apex_0: [f32; 4],
    pub landing_position_16: [f32; 4],
    pub landing_normal_32: [f32; 4],
    pub launch_position_48: [f32; 4],
    pub heading_80: [f32; 4],
    pub time_176: f32,
    pub duration_180: f32,
    pub apex_time_196: f32,
    pub flag_440: u8,
}

/// The alternate trajectory and event fields read from OffBoard output18.
pub(crate) struct CameraOffboardOutput {
    pub duration_92: f32,
    pub time_152: f32,
    pub apex_time_156: f32,
    pub launch_normal_160: [f32; 4],
    pub launch_position_176: [f32; 4],
    pub landing_normal_192: [f32; 4],
    pub landing_position_208: [f32; 4],
    pub heading_224: [f32; 4],
    pub apex_240: [f32; 4],
    pub object_held_304: u8,
    pub hurdle_317: u8,
    pub use_trajectory_331: u8,
    pub dropping_in_334: u8,
}

/// Grinds output4, distinct from Ground output8 (whose316 is wall-ride exit).
pub(crate) struct CameraGrindOutput {
    pub direction_0: [f32; 4],
    pub camera_target_96: [f32; 4],
    pub grinding_316: u8,
}

pub(crate) struct CameraEventsOutput {
    pub intent_51: u8,
    pub preparing_52: u8,
    pub dropping_in_63: u8,
    pub trick_125: u8,
    pub hippy_jump_322: u8,
    pub broken_bone_duration_200: f32,
    pub capabilities_204: u32,
}

/// Engine-owned preference values copied by the original subject publisher.
pub(crate) struct CameraPreferences {
    pub invert_look: [bool; 2],
    pub shake_variant: u8,
    pub value_32: f32,
}

/// Remaining real output producers needed alongside the existing game owners.
/// This structure deliberately has no Default: its callers must publish the
/// actual native reset/state values even when air/offboard/grinds are inactive.
pub(crate) struct CameraPublicationInputs {
    /// Completed physics tick represented by these publication inputs.
    pub tick: u64,
    pub state: CameraStateOutput,
    pub animation: CameraAnimationOutput,
    pub air: CameraAirOutput,
    pub offboard: CameraOffboardOutput,
    pub grinds: CameraGrindOutput,
    pub events: CameraEventsOutput,
    /// PhysOutSystemReckoning80, after its own physical output conditioner.
    pub damped_com_80: [f32; 4],
    /// PhysOutGround80 and288.80 differs from dynamic up64 and wheel normal96.
    pub ground_up_80: [f32; 4],
    pub ground_scalar_288: f32,
    /// PhysOutSkeleton552/556 and PhysOutCollision64, respectively.
    pub look_552_556: [f32; 2],
    pub collision_look_target_64: [f32; 4],
    pub preferences: CameraPreferences,
    /// Our stable player/query identity replaces the native actor address.
    pub context: u32,
}

/// Publish from the very same physical bodies and completed output records
/// used by the animation phase. This neither advances physics nor conditions
/// the steering twice. SubjectPublisher owns the subsequent camera histories.
pub(crate) fn snapshot(
    physics: &Skate3PhysicsState,
    skater: &Skate3ControllerState,
    input: &CameraPublicationInputs,
) -> Result<CameraSubjectSnapshot, String> {
    let p = &skater.player_input.physical;
    let processed = &skater.player_input.processed;
    let toolkit = skater
        .player_input
        .toolkit
        .as_ref()
        .ok_or("Camera publication requires the completed player input toolkit")?;
    let ground = skater
        .ground
        .output(processed, &skater.animation_input, toolkit);
    let deck = physics.board.part_transforms()[BodyId::Deck.index()];
    let mut physical_transform = [[0.0; 4]; 4];
    for (column, axis) in physics
        .riding
        .motion
        .effective_basis
        .columns
        .iter()
        .enumerate()
    {
        physical_transform[column][..3].copy_from_slice(axis);
    }
    physical_transform[3] = [
        deck.translation.x,
        deck.translation.y,
        deck.translation.z,
        0.0,
    ];
    //82BE3650 -> PhysOutSkeleton0/432 at82BE20F4/2100. Its stance flag is
    //Processed2476 bit2, distinct from the board's effective-frame flag2468.
    let skeleton_root = effective_skeleton_root(
        skater.animated_skeleton.roots.animation_to_world,
        processed.flags_2476,
    );
    //82DF80D8 selects the subject pose separately.82DF70CC/70E8 still
    //publish the board transform/position, including while off-board.
    let record = &skater.skeleton.record;
    let com = p.reckoning.vector_64.map(f32::from_bits);
    let up = p.reckoning.vector_96.map(f32::from_bits);
    let velocity = p.skateboard.vector_80.map(f32::from_bits);
    //The source getter396 is bound to Motion64, despite its camera name
    //"acceleration". Motion64 is the published deck angular velocity.
    let acceleration = p.skateboard.vector_64.map(f32::from_bits);
    // Native vector storage retains four lanes, but camera positions,
    // directions and magnitudes use only XYZ. W is not a homogeneous
    // coordinate here and can be non-finite without corrupting the pose.
    let finite = |name: &str, values: &[f32; 4]| -> Result<(), String> {
        if values[..3].iter().all(|value| value.is_finite()) {
            Ok(())
        } else {
            Err(format!(
                "Camera subject owner published non-finite {name}: {values:?}; state={:?}; category={:?}",
                processed.state_2508,
                processed.category_2512,
            ))
        }
    };
    finite("centre_of_mass", &com)?;
    finite("reckoning_up", &up)?;
    finite("damped_centre_of_mass", &input.damped_com_80)?;
    finite("ground_up", &input.ground_up_80)?;
    for column in &skeleton_root {
        finite("skeleton_root", column)?;
    }
    for (index, pose) in [1usize, 15, 19, 23].into_iter().map(|index| (index, record.pose[index][3])) {
        finite(match index { 1 => "head", 15 => "left_foot", 19 => "right_foot", _ => "hips" }, &pose)?;
    }
    if velocity[..3].iter().chain(acceleration[..3].iter()).any(|v| !v.is_finite()) {
        return Err(format!(
            "Camera received non-finite board motion publication: velocity={velocity:?}; acceleration={acceleration:?}; raw_velocity={:?}; raw_acceleration={:?}",
            p.skateboard.vector_80,
            p.skateboard.vector_64,
        ));
    }
    debug!(state = p.state.state_16, category = p.state.category_12,
        board_position = ?physical_transform[3], board_velocity = ?velocity,
        board_angular_velocity = ?acceleration, skeleton_root = ?skeleton_root[3],
        "camera subject publication");
    let last_ground_up = p.ground.vector_96.map(f32::from_bits);
    let category = p.state.category_12;
    let state = p.state.state_16;
    let off = &input.offboard;
    let air = &input.air;
    let alternate = off.use_trajectory_331 != 0;
    let (
        launch_position,
        launch_normal,
        landing_position,
        landing_normal,
        apex_position,
        heading,
        trajectory_time,
        trajectory_duration,
        apex_time,
    ) = if alternate {
        (
            off.launch_position_176,
            off.launch_normal_160,
            off.landing_position_208,
            off.landing_normal_192,
            off.apex_240,
            off.heading_224,
            off.time_152,
            off.duration_92,
            off.apex_time_156,
        )
    } else {
        (
            air.launch_position_48,
            last_ground_up,
            air.landing_position_16,
            air.landing_normal_32,
            air.apex_0,
            air.heading_80,
            air.time_176,
            air.duration_180,
            air.apex_time_196,
        )
    };
    let grinding = input.grinds.grinding_316;
    let look = std::array::from_fn(|i| {
        if input.preferences.invert_look[i] {
            -input.look_552_556[i]
        } else {
            input.look_552_556[i]
        }
    });
    let subject = ManagerSubject {
        rig: Subject {
            //Overwritten by the stock subject's one-frame publication history.
            transform: physical_transform,
            skeleton_root,
            hips_position: record.pose[23][3],
            last_valid_ground_up: last_ground_up,
            reference_positions: [[0.0; 4]; 10],
            context: input.context,
            pumping_acceleration: skater.ground.pumping.pump_acceleration,
            state_height_32: input.state.height_32,
            in_ground_physics: u8::from(category == 100),
            grinding,
            // KnownAir Fill publishes validity at437. Byte441 is the body-flip
            // flag; using it hides ordinary ollie trajectories from the camera.
            trajectory_valid: if alternate { 1 } else { p.air.known_air_valid_437 },
            wiping_out: input.state.wiping_out_59,
            physically_pushing: input.state.physically_pushing_55,
            at_pushable_speed: u8::from(ground.skateboard_motion_4.is_at_pushable_speed),
            off_board: u8::from(category == 500),
            air_flag_452: p.air.use_air_reckoning_452,
            state_flag_81: input.state.flag_81,
            broken_bone_slowmo: u8::from(
                input.events.broken_bone_duration_200 > 0.0
                    && input.events.capabilities_204 & 4 != 0,
            ),
            subject_flag_328: 0, //82DF726C clears this every publication.
        },
        anchors: Anchors::new().entries, //filled by the real anchor histories.
        compass: [0.0; 9],               //filled by the real Compass owner.
        board_offset_direction: up,
        ground_normal: input.ground_up_80,
        launch_position,
        launch_normal,
        landing_position,
        landing_normal,
        apex_position,
        direction_424: input.grinds.direction_0,
        look,
        steering: [
            input.animation.conditioned_turn[4],
            input.animation.conditioned_turn[3],
            input.animation.input_turn_64,
            input.animation.input_kickturn_68,
        ],
        trajectory_time,
        trajectory_duration,
        apex_time,
        value_512: input.preferences.value_32,
        value_516: input.ground_scalar_288,
        reset: input.state.reset_62,
        flag_556: air.flag_440,
        stance_560: input.animation.stance_155,
        stance_592: input.animation.skater_animation_stance,
        flag_652: input.state.flag_79,
        shake_variant: input.preferences.shake_variant,
        special_effect: 0, //82DF6A44..6A60 clears observer mode for this subject.
        flag_684: off.object_held_304,
    };
    Ok(CameraSubjectSnapshot {
        tick: input.tick,
        subject,
        pose: SubjectPoseInputs {
            physical_transform,
            skeleton_root,
            center_of_mass: com,
            reckoned_center_of_mass: input.damped_com_80,
            wiping_out: input.state.wiping_out_59 != 0,
            state_flag_75: input.state.use_skeleton_root_75 != 0,
        },
        anchors: AnchorInputs {
            board_position: physical_transform[3],
            board_velocity: velocity,
            board_acceleration: acceleration,
            board_offset_direction: up,
            center_of_mass: com,
            damped_center_of_mass: input.damped_com_80,
            skeleton_root_up: skeleton_root[1],
            grind_point: input.grinds.camera_target_96,
            grinding: grinding != 0,
            reset: input.state.reset_62 != 0,
        },
        reference_points: ReferencePointInputs {
            //82BE1E64..1ED4 copies these actual physical volume positions.
            head: record.pose[1][3],
            hips: record.pose[23][3],
            left_foot: record.pose[15][3],
            right_foot: record.pose[19][3],
            board: skeleton_root[3],
            centre_of_mass: com,
            damped_centre_of_mass: input.damped_com_80,
            grind_position: input.grinds.camera_target_96,
            tracked_anchor: [0.0; 4],
            incline_normal: [0.0; 4],
            grinding,
        },
        compass: CompassPoseInputs {
            skeleton_direction: skeleton_root[2],
            look_target: input.collision_look_target_64,
            board_velocity: velocity,
            trajectory_direction: heading,
            state_103: state == 103,
        },
        graph: CameraGraphSubject {
            time_since_player_input: input.animation.time_since_input_128,
            wipeout_tweak: input.animation.wipeout_tweak_148,
            onboard_air: category == 200,
            preparing_to_jump: input.events.preparing_52 != 0,
            manual: input.state.manual_60 != 0 && input.events.intent_51 == 0,
            offboard_air: state == 501,
            running_out: input.animation.running_out_160 != 0,
            footplant: state == 601,
            handplant: state == 600,
            hippy_jump: input.events.hippy_jump_322 != 0,
            hippy_hurdle: off.hurdle_317 != 0,
            boneless: input.events.trick_125 != 0,
            moving_object: state == 502,
            skitching: state == 104,
            dropping_in: input.events.dropping_in_63 != 0 && off.dropping_in_334 != 0,
            slow_motion_air_duration: p.air.scalar_184,
        },
    })
}

fn effective_skeleton_root(mut root: Transform, flags_2476: u32) -> Transform {
    if flags_2476 & 4 != 0 {
        root[0] = root[0].map(|v| -v);
        root[2] = root[2].map(|v| -v);
    }
    root
}
