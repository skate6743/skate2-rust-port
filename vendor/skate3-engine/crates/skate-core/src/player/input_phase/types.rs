use super::air_output::AirOutputFields;
use super::pose_output::{AnimationOutputFields, ScoringOutputFields, SkeletonOutputFields};
use crate::animation::output::actor_packet::ExternalPhysicsInput;
use crate::input::animation_packet::AnimationPacketFields;

pub type RawVector = [u32; 4];
pub type RawMatrix = [[u32; 4]; 4];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProbeFields {
    pub vectors_16_32_48: [RawVector; 3],
    pub words_64_68: [u32; 2],
    pub bytes_72_73: [u8; 2],
    pub vector_80: RawVector,
    pub words_96_100: [u32; 2],
    pub byte_104: u8,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LineTestFields {
    pub position: RawVector,
    pub normal: RawVector,
    pub surface: u32,
    /// The native byte is copied in full; conditionals test it for nonzero.
    pub valid: u8,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StateVariantFields {
    /// Byte +60 of the object referenced by the variant block's +4 pointer.
    pub surface_override_enabled_60: u8,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
/// Reset82DE53F0 copies the template initialized by82DE3BE8:
/// every represented word and byte in this subset initially equals zero.
pub struct CurrentStateFields {
    pub identifier_8: u32,
    pub category_12: u32,
    pub state_16: u32,
    pub surface_height_32: f32,
    ///BipedGround Fill82D32D38, independent of State40.
    pub counter_36: u32,
    pub skitch_value_40: u32,
    pub flag_61: u8,
    pub flag_66: u8,
    pub flag_69: u8,
    pub flag_74: u8,
    pub signed_ground_step_84: u8,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SkateboardMotionFields {
    pub vector_64: RawVector,
    pub vector_80: RawVector,
    pub vector_96: RawVector,
    pub vector_128: RawVector,
    pub vector_144: RawVector,
    pub scalar_160: f32,
    pub scalar_164: f32,
    pub scalar_168: f32,
    pub scalar_172: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SystemReckoningFields {
    /// Physical COM velocity from Skeleton16176 (82BE1D84/94).
    /// Restored r28 at82DB49B0 is16, not128.
    pub vector_16: RawVector,
    pub vector_64: RawVector,
    pub vector_96: RawVector,
    pub vector_144: RawVector,
    pub flag_164: u8,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GroundOutputFields {
    ///State503 Fill82D4E090..E0C8.
    pub landing_half_turns_312: i32,
    pub hippy_jumping_322: u8,
    pub hippy_takeoff_323: u8,
    pub vector_64: RawVector,
    /// Board Fill82C03318: wheel-contact normal (not Motion80 velocity).
    pub vector_80: RawVector,
    /// Host storage for Motion273, NOT native Ground273. Common82DB7598
    /// publishes Processed2468 bit20; keep this distinct from the normal.
    pub flag_273: u8,
    pub vector_96: RawVector,
    pub vector_128: RawVector,
    pub scalar_276: f32,
    pub scalar_292: f32,
    pub scalar_296: f32,
    pub flag_317: u8,
    pub flag_318: u8,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CollisionOutputFields {
    pub wheel_count_0: u32,
    /// Board Fill surface vote, consumed by 82DB8120 in riding states.
    pub surface_type_16: u32,
    /// SkeletonCollision Fill82BD60C8: +4064/+4068/+4081.
    pub surface_height_208: f32,
    /// SkeletonCollision::Fill82BD60C8, consumed by bail scoring/audio packets.
    pub body_response_196: f32,
    pub force_response_200: f32,
    pub contact_region_response_80: [f32; 8],
    pub flag_214: u8,
    pub flag_217: u8,
    pub scalar_28: f32,
    ///Offboard landing manager82D79498; reset82DE3290 clears this position.
    pub vector_48: RawVector,
    pub predicted_position_64: RawVector,
    pub flag_215: u8,
    pub flag_216: u8,
    pub wheel_contact_3296_3299: [u8; 4],
    pub flag_3472: u8,
    pub flag_3475: u8,
    pub flag_3477: u8,
    pub flag_3478: u8,
    pub flag_3479: u8,
    pub flag_3480: u8,
    pub flag_3481: u8,
    ///Offboard landing manager82D7948C; reset82DE32E4 clears availability.
    pub flag_3482: u8,
    pub flag_3483: u8,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PhysicsOutputFields {
    pub flag_32: u8,
    /// Physics+16. This native record is only48bytes long.
    pub vector_16: RawVector,
}

use super::GrindOutputFields;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct OffBoardOutputFields {
    ///BipedGround Fill82D32D38 and shared Feet Fill82D785F8.
    pub kind_88: u32,
    pub scalar_112: f32,
    pub distance_116: f32,
    pub flags_306_307: [u8; 2],
    pub flag_329: u8,
    pub flag_330: u8,
    pub flag_334: u8,
    ///Common ProcessOutput82DB724C..7258 from the shared Biped720/716.
    pub cadence_phase_80: f32,
    pub locomotion_state_84: u32,
    /// SkateboardController FillPhysOut82D76D20 orientation outputs +36/+40.
    pub angle_36: f32,
    pub angle_40: f32,
    /// Reset82DE40C0; Biped trajectory publication writes the duration here.
    pub trajectory_time_120: f32,
    /// Reset82DE4170; BipedAir Fill82D30324 publishes trajectory availability.
    pub trajectory_valid_331: u8,
    pub scalar_32: f32,
    pub flag_304: u8,
    ///Common ProcessOutput82DB76F4: Processed2480 bit19, not grab-object304.
    pub flag_308: u8,
    pub flag_311: u8,
    ///SkateboardController82D76D88: state448 is FREE/HIDING/RETURNING.
    pub free_board_312: u8,
    ///82D76DA0: physical RETURNING state, distinct from animation activity323.
    pub returning_board_313: u8,
    pub flag_314: u8,
    pub flag_315: u8,
    pub flag_316: u8,
    /// OffBoard reset82DE4138 clears this. LandingOnDeckManager Fill82D79420
    /// publishes manager260 && !manager258; only Biped/LandingOnDeck state
    /// outputs call that producer. Ordinary Ground retains the reset value.
    pub hippy_hurdling_317: u8,
    pub flag_318: u8,
    pub dropping_board_322: u8,
    ///82D76ED4: state448 is HIDING.
    pub hiding_board_321: u8,
    pub flag_319: u8,
    pub retrieving_board_323: u8,
    ///82D76F68: retrieving animation OR retrieve pulse while RETURNING.
    pub flag_324: u8,
    pub flag_328: u8,
    pub vector_64: RawVector,
    ///BipedAir Fill82D30300: distinct from the on-board Air output block.
    pub scalar_92: f32,
    pub vector_96: RawVector,
    pub word_144: u32,
    pub scalar_148: f32,
    pub scalar_152: f32,
    pub scalar_156: f32,
    pub vector_160: RawVector,
    pub vector_176: RawVector,
    pub vector_192: RawVector,
    pub vector_208: RawVector,
    pub vector_224: RawVector,
    pub vector_240: RawVector,
    pub flag_320: u8,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PhysicalPlayerInput {
    /// SkateboardReckoning+176; restored r29 at82DB4994 is176.
    pub board_reckoning_side_176: RawVector,
    pub skateboard: SkateboardMotionFields,
    pub air: AirOutputFields,
    pub physics: PhysicsOutputFields,
    pub grinds: GrindOutputFields,
    pub skeleton: SkeletonOutputFields,
    pub animation: AnimationOutputFields,
    pub scoring: ScoringOutputFields,
    /// Present only when Teleporting702 publishes a ready reset target.
    pub teleport_output: Option<crate::player::teleport_state::Output>,
    pub collision: CollisionOutputFields,
    pub state: CurrentStateFields,
    pub ground: GroundOutputFields,
    pub reckoning: SystemReckoningFields,
    pub filtered_state_0: u32,
    pub off_board: OffBoardOutputFields,
    /// `player+1896 -> +0 -> +24 -> +16`, before the variant override.
    pub surface_default_mode: u32,
    /// Player component +1832, word +1876; source bit 28 is consumed.
    pub component_1832_word_1876: u32,
}

pub struct AnimationInputPacket<'a> {
    pub publication: &'a AnimationPacketFields,
    pub external_physics_10512: &'a ExternalPhysicsInput,
    pub flag_10369: u8,
    pub flag_10370: u8,
    pub flag_10373: u8,
    pub suppress_transition_10376: u8,
    pub use_external_physics_10688: u8,
    pub external_physics_flag_10689: u8,
    pub flag_10786: u8,
    pub flag_10787: u8,
    pub force_braking_10796: u8,
    pub vector_10816: RawVector,
    pub vector_10832: RawVector,
    pub vector_10848: RawVector,
    pub vector_10864: RawVector,
    pub vector_10880: RawVector,
    pub vector_10896: RawVector,
    pub scalar_10912: f32,
    pub scalar_10916: f32,
    pub state_variant_10928: u32,
    pub flags_10932: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeReferenceBase {
    Player,
    SurfaceSelector,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeRelativeReference {
    pub base: NativeReferenceBase,
    pub offset: u16,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerInputState {
    pub manager_1856_counter_320: u32,
    pub manager_1852_flag_256: u8,
    pub manager_1852_vector_176: RawVector,
    pub flags_1296: u32,
    pub grounded_frames_1300: u32,
    pub state_count_1312: u32,
    pub update_count_1316: u32,
    pub spin_same_direction_frames_1324: u32,
    pub frames_since_teleport_1328: u32,
    /// ProcessOutput82DB78A8 keeps a dismount request alive for three outputs.
    pub dismount_request_frames_1332: u32,
    pub state_value_1336: u32,
    pub state_timer_1344: f32,
    pub time_since_last_input_1348: f32,
    pub time_on_ground_1352: f32,
    pub signed_ground_time_1356: f32,
    pub previous_spin_input_1360: f32,
    pub previous_crouch_1364: f32,
    pub time_off_board_1368: f32,
    pub skitch_value_1372: u32,
    pub skitch_timer_1376: f32,
    pub ground_timer_1380: f32,
    pub secondary_ground_timer_1384: f32,
    pub probe: ProbeFields,
    pub queued_vector_1280: RawVector,
    pub external_physics_cache_1008: ExternalPhysicsInput,
    pub prepared_jump_velocity_1184: RawVector,
    pub previous_ground_position_1200: RawVector,
    pub current_ground_position_1216: RawVector,
    pub ground_delta_1232: RawVector,
    pub damped_ground_delta_1248: RawVector,
    pub state_variants_1408: [StateVariantFields; 5],
    pub hips_line_test_1488: LineTestFields,
    pub left_line_test_1536: LineTestFields,
    pub right_line_test_1584: LineTestFields,
    /// Number at player +1304 used to gate the ground-history filter.
    pub ground_history_frames_1304: i32,
}

impl Default for PlayerInputState {
    fn default() -> Self {
        Self {
            manager_1856_counter_320: 0,
            manager_1852_flag_256: 0,
            manager_1852_vector_176: [0; 4],
            flags_1296: 0,
            grounded_frames_1300: 0,
            state_count_1312: 0,
            update_count_1316: 0,
            spin_same_direction_frames_1324: 0,
            frames_since_teleport_1328: 0,
            dismount_request_frames_1332: 0,
            state_value_1336: 0,
            state_timer_1344: 0.0,
            time_since_last_input_1348: 0.0,
            time_on_ground_1352: 0.0,
            signed_ground_time_1356: 0.0,
            previous_spin_input_1360: 0.0,
            previous_crouch_1364: 0.0,
            time_off_board_1368: 0.0,
            skitch_value_1372: 0,
            skitch_timer_1376: 0.0,
            ground_timer_1380: 0.0,
            secondary_ground_timer_1384: 0.0,
            probe: ProbeFields::default(),
            queued_vector_1280: [0; 4],
            external_physics_cache_1008: empty_external_physics(),
            prepared_jump_velocity_1184: [0; 4],
            previous_ground_position_1200: [0; 4],
            current_ground_position_1216: [0; 4],
            ground_delta_1232: [0; 4],
            damped_ground_delta_1248: [0; 4],
            state_variants_1408: [StateVariantFields::default(); 5],
            hips_line_test_1488: LineTestFields::default(),
            left_line_test_1536: LineTestFields::default(),
            right_line_test_1584: LineTestFields::default(),
            ground_history_frames_1304: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProcessedPhysicsInput {
    pub effective_anim_transform_192: RawMatrix,
    pub vectors_400_416: [RawVector; 2],
    pub vectors_464_480_496_512_528: [RawVector; 5],
    pub vectors_544_560_592_608: [RawVector; 4],
    pub vectors_624_640_656_672: [RawVector; 4],
    pub prepared_jump_704: RawVector,
    ///Skeleton::ProcessData82BD8E70..8EA8, after ordered animation attributes.
    pub collision_pose_error_736: RawVector,
    pub animation_com_to_deck_752: RawVector,
    pub animation_com_to_deck_delta_768: RawVector,
    pub vectors_720_784_800_816_832_864: [RawVector; 6],
    pub vectors_880_896_912_928_944: [RawVector; 5],
    pub line_tests_960_1008_1056: [LineTestFields; 3],
    pub grind: super::GrindInvestigationFields,
    pub vector_1520: RawVector,
    pub matrix_1536: RawMatrix,
    pub byte_1600: u8,
    pub external_physics_1616: ExternalPhysicsInput,
    pub probe_1792: ProbeFields,
    /// Actual grab-spline publication payloads; flags2480 bits22/21 carry readiness.
    pub grab_records_1888_2176: [[u32; 72]; 2],
    /// Interactable identity published by82D740F8; bit20 carries validity.
    pub object_2464: u32,
    pub flags_2468: u32,
    pub flags_2472: u32,
    pub flags_2476: u32,
    pub flags_2480: u32,
    pub flags_2484: u32,
    pub flags_2488: u32,
    pub state_identifier_2496: u32,
    pub state_2504: u32,
    pub state_2508: u32,
    pub category_2512: u32,
    pub category_2516: u32,
    pub player_state_value_2520: u32,
    pub filtered_state_2524: u32,
    pub state_variant_index_2528: u32,
    pub grind_words_2532_2536: [u32; 2],
    pub surface_mode_2540: u32,
    pub surface_primary_ref_2544: Option<NativeRelativeReference>,
    pub state_variant_ref_2548: Option<NativeRelativeReference>,
    pub surface_secondary_ref_2552: Option<NativeRelativeReference>,
    pub wheel_count_2556: u32,
    pub state_count_2564: u32,
    pub update_count_2568: u32,
    pub spin_same_direction_frames_2580: u32,
    pub frames_since_teleport_2584: u32,
    pub skitch_value_2592: u32,
    pub left_surface_2596: u32,
    pub right_surface_2600: u32,
    pub timestep_2604: f32,
    pub scalar_2612: f32,
    ///PrepareBoardToolkit82C01744: signed speed times its ordered travel sign.
    pub scalar_2616: f32,
    pub transition_2636: f32,
    ///Skeleton82BDD008: BodySpin plus the actual grind-air adjustment result.
    ///Read only with flags2484bit15; Reset82BFA394 clears it each input tick.
    pub grind_adjusted_body_spin_2644: f32,
    ///Reset82BFA2D0 writes original literal822F8B40, -9.8.
    pub gravity_2648: f32,
    pub scalar_2652: f32,
    pub scalar_2656: f32,
    pub state_timer_2664: f32,
    pub scalar_2668: f32,
    /// Written by Skeleton::ProcessData before the post-skeleton suffix.
    pub spin_input_2672: f32,
    pub scalar_2696: f32,
    pub scalar_2700: f32,
    pub scalar_2736: f32,
    pub time_since_last_input_2748: f32,
    pub time_on_ground_2752: f32,
    pub signed_ground_time_2756: f32,
    pub truck_tightness_2760: f32,
    pub scalar_2764: f32,
    ///Current minus preceding authored board forward-axis Y,82BD8E20.
    pub board_at_y_delta_2768: f32,
    pub air_scalar_2772: f32,
    /// Written by Skeleton::ProcessData before crouch-delta publication.
    pub crouch_2776: f32,
    pub crouch_delta_2780: f32,
    pub off_board_scalar_2832: f32,
    pub time_off_board_2836: f32,
    pub ground_timer_2848: f32,
    pub secondary_ground_timer_2852: f32,
    pub collision_scalar_2924: f32,
    pub actor_query_2948: u32,
    pub actor_query_2952: u32,
}

/// Immutable publication of the completed physical-input phase for one tick.
/// The mutable record remains private to the input producer; selectors and
/// later phases consume this value object so they cannot observe a half-written
/// packet.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProcessedPhysicsSnapshot {
    pub tick: u64,
    pub input: ProcessedPhysicsInput,
}

impl ProcessedPhysicsSnapshot {
    pub const fn new(tick: u64, input: ProcessedPhysicsInput) -> Self {
        Self { tick, input }
    }
}

impl Default for ProcessedPhysicsInput {
    fn default() -> Self {
        Self {
            effective_anim_transform_192: [[0; 4]; 4],
            vectors_400_416: [[0; 4]; 2],
            vectors_464_480_496_512_528: [[0; 4]; 5],
            vectors_544_560_592_608: [[0; 4]; 4],
            vectors_624_640_656_672: [[0; 4]; 4],
            prepared_jump_704: [0; 4],
            collision_pose_error_736: [0; 4],
            animation_com_to_deck_752: [0; 4],
            animation_com_to_deck_delta_768: [0; 4],
            vectors_720_784_800_816_832_864: [[0; 4]; 6],
            vectors_880_896_912_928_944: [[0; 4]; 5],
            line_tests_960_1008_1056: [LineTestFields::default(); 3],
            grind: super::GrindInvestigationFields::default(),
            vector_1520: [0; 4],
            matrix_1536: [[0; 4]; 4],
            byte_1600: 0,
            external_physics_1616: empty_external_physics(),
            probe_1792: ProbeFields::default(),
            grab_records_1888_2176: [[0; 72]; 2],
            object_2464: 0,
            flags_2468: 0,
            flags_2472: 0,
            flags_2476: 0,
            flags_2480: 0,
            flags_2484: 0,
            flags_2488: 0,
            state_identifier_2496: 0,
            state_2504: 0,
            state_2508: 0,
            category_2512: 0,
            category_2516: 0,
            player_state_value_2520: 0,
            filtered_state_2524: 0,
            state_variant_index_2528: 0,
            grind_words_2532_2536: [0; 2],
            surface_mode_2540: 0,
            surface_primary_ref_2544: None,
            state_variant_ref_2548: None,
            surface_secondary_ref_2552: None,
            wheel_count_2556: 0,
            state_count_2564: 0,
            update_count_2568: 0,
            spin_same_direction_frames_2580: 0,
            frames_since_teleport_2584: 0,
            skitch_value_2592: 0,
            left_surface_2596: 0,
            right_surface_2600: 0,
            timestep_2604: 0.0,
            scalar_2612: 0.0,
            scalar_2616: 0.0,
            transition_2636: 0.0,
            grind_adjusted_body_spin_2644: 0.0,
            gravity_2648: 0.0,
            scalar_2652: 0.0,
            scalar_2656: 0.0,
            state_timer_2664: 0.0,
            scalar_2668: 0.0,
            spin_input_2672: 0.0,
            scalar_2696: 0.0,
            scalar_2700: 0.0,
            scalar_2736: 0.0,
            time_since_last_input_2748: 0.0,
            time_on_ground_2752: 0.0,
            signed_ground_time_2756: 0.0,
            truck_tightness_2760: 0.0,
            scalar_2764: 0.0,
            board_at_y_delta_2768: 0.0,
            air_scalar_2772: 0.0,
            crouch_2776: 0.0,
            crouch_delta_2780: 0.0,
            off_board_scalar_2832: 0.0,
            time_off_board_2836: 0.0,
            ground_timer_2848: 0.0,
            secondary_ground_timer_2852: 0.0,
            collision_scalar_2924: 0.0,
            actor_query_2948: 0,
            actor_query_2952: 0,
        }
    }
}

#[cfg(test)]
mod snapshot_tests {
    use super::*;

    #[test]
    fn processed_snapshot_is_immutable_and_tick_owned() {
        let mut input = ProcessedPhysicsInput::default();
        input.flags_2468 = 0x10;
        let snapshot = ProcessedPhysicsSnapshot::new(23, input);
        input.flags_2468 = 0x20;

        assert_eq!(snapshot.tick, 23);
        assert_eq!(snapshot.input.flags_2468, 0x10);
        assert_eq!(input.flags_2468, 0x20);
    }
}

pub(crate) fn empty_external_physics() -> ExternalPhysicsInput {
    ExternalPhysicsInput {
        vectors: [[0; 4]; 10],
        flags: 0,
    }
}
