//! One selected original Vault class selection shared by bootstrap and corpus.
use crate::{
    skateboard_attributes::SkateboardAttributeClass, skeleton_attributes::SkeletonAttributeClass,
    skeleton_body_attributes, skeleton_collision_attributes::SkeletonCollisionAttributes,
    skeleton_drive_attributes::ANIMATION_DRIVES_CLASS, skeleton_hat::HatAttributes,
};

/// The three classes prepended by the mounted physics attribute helper.
pub(crate) const BASE_PHYSICS_CLASSES: [u64; 3] = [
    0x0429_3d0f_2adc_fbb4,
    0x46ad_d2ee_a90c_c062,
    0x485a_76b6_8ca6_5d6f,
];

/// Exact additional selection used by the selected mounted physical startup.
/// Order and duplicate behavior are preserved; this is not a full Vault service.
pub(crate) fn selected_additional_classes() -> Vec<u64> {
    vec![
        0x7f47_ca62_f410_0f41, // CAS morph parameters
        0x3822_5df1_cffa_ffaa, // original character CPU-skin position expansion
        0xbb85_2b43_8bf5_78ed, // CAS frontend morph input settings
        0xbd77_229e_4122_863e, // CAS stamp zones
        0x0d71_da87_70e0_9cbc, // CAS stamps
        0xae1f_950c_a0d1_adf2, // CAS colour zones
        0x6d93_4d0e_38c9_5e19, // CAS colours
        0x548b_58b0_c27c_9564, // sponsor: CAS brand filters and their logos
        crate::selected_original_assets_bootstrap::map_catalog::LOCATION_CLASS,
        crate::selected_original_assets_bootstrap::map_catalog::GROUP_CLASS,
        crate::selected_original_assets_bootstrap::map_catalog::ICON_CLASS,
        crate::selected_original_assets_bootstrap::map_catalog::DISTRICT_CLASS,
        crate::camera_attributes::BINDINGS[0].0,
        crate::camera_attributes::BINDINGS[2].0,
        crate::camera_attributes::BINDINGS[3].0,
        crate::camera_attributes::BINDINGS[6].0,
        crate::camera_attributes::BINDINGS[9].0,
        crate::camera_attributes::BINDINGS[11].0,
        crate::camera_attributes::BINDINGS[16].0,
        crate::camera_attributes::BINDINGS[17].0,
        crate::camera_attributes::BINDINGS[18].0,
        crate::camera_shot_manager::attributes::CAMERA_SHOTS_CLASS,
        0x0a12_455a_1506_9596, // SlowMotion constructor
        crate::living_world_rain::LIVING_WORLD_CLASS,
        crate::living_world_rain::DEBUG_CLASS,
        // MoodMan.Init82ce9638 binds categories and builds the authored
        // reaction map before pedestrian initialization. The selected Vault
        // loads only explicitly selected classes; RefSpecs do not load them.
        crate::pedestrian_mood::CATEGORY_CLASS,
        crate::pedestrian_mood::MOOD_CLASS,
        crate::pedestrian_mood::RESULT_CLASS,
        0xda7a_8f0e_f63b_e539, // dmo_models: handprop Model RefSpec -> bank asset
        0xd558_c5d6_3afe_bcad, // livingworld_dynamicobject_safety: WorldPainter/RAIN
        0xec8b_7565_46e2_26f6, // livingworld_handprops: authored channel/layout
        0x40c4_819c_bd5a_014b, // livingworld_handprops_usagecharacteristics
        0x53e3_32f6_7b81_24b2, // OverrideAnimData.Begin826ff038, marquee
        // Pedestrian ctor82e97748 resolves these authored entity RefSpecs.
        // try_ref_spec_instance does not load absent classes on demand.
        0x1477_b0d5_343d_29fd, // livingworld_entities_chase
        0xd521_592a_5af8_ecc1, // livingworld_entities_locomotion
        0x3032_dc45_3378_9895, // livingworld_entity_takedown
        // BigBlack ctor82e98b00..b34 resolves the entity's ProtectionParameters
        // RefSpec. Export existence alone does not construct this class.
        0xda06_e0e2_b89b_86c0, // livingworld_entities_protect
        0xb8b6_0ddf_94b9_6b4a, // entity NavigationParameters RefSpec
        0xf74b_f69f_6a10_6edf, // entity post-adjust RefSpec
        0x2e3b_f6df_8961_4045, // entity head-tracking RefSpec
        0xb5de_32c9_7060_5efa, // patrol-zone default and guard parameters
        0x5cac_3784_5e0c_4722, // contained waypoint Prop characteristics
        0x05ec_9cb0_3a5b_2eee, // physics_bodyspin Air evolution
        0x07d3_9b41_aace_e50f, // Motion skitching
        0x705b_d1b1_2aef_b73a, // Motion SetTurning anim_carving
        0x97b0_272a_2a7e_54f8, // Motion moving-object attributes
        crate::living_world_presentation_opacity::MODELS_CLASS,
        // SHLightingMan::UpdateDefaultLightProbe82827860, used by vehicles.
        skate2_formats::attrib::string_hash64("render_locations"),
        skate2_formats::attrib::string_hash64("rendering"), // LivingWorld presentation material LOD
        crate::living_world_recipe_manager::LIVING_WORLD_CONFIG_CLASS_ID,
        crate::living_world_grind_services::census::targets::CENSUS_CLASS,
        crate::living_world_grind_services::census::targets::GROUPS_CLASS,
        crate::living_world_grind_services::census::targets::CATEGORIES_CLASS,
        crate::living_world_grind_services::vehicle_motion::CHARACTERISTICS_CLASS,
        crate::living_world_grind_services::vehicle_motion::attributes::DRIVERS_CLASS,
        crate::physical_player_grind_air_adjust::GRIND_AIR_CLASS,
        0x74bc_9c93_affe_6aac, // conditioner animation
        0x546c_36b6_5603_8e04, // scoring states
        0x3492_15e2_e817_703c, // scoring general
        crate::physical_player_phys_out::conditioners::scoring_database::TRICK_CLASS,
        0xf4c9_f7ff_78b8_f653, // scoring_entities: actual vehicle/pedestrian/DMO RefSpecs
        0x9e4c_471b_c6ea_b615, // scoring wipeouts
        0xd552_e0b1_0a9a_104f, // visualtreatments_bail: VDWriter HoM speed curves (827fccc8)
        crate::physical_player_handplant::POSITION_CLASS,
        crate::physical_player_handplant::MANAGER_CLASS,
        crate::actor_construction::motion_runtime_session::handplant::ANIMATION_CLASS,
        0x2656_db32_bef0_b68d, // handplant trajectory
        crate::skateboard_foot_attributes::wallride::WallRideTrajectoryAttributes::CLASS,
        0x2377_cef0_ff59_c4d2, // LandingOnDeck
        0x018e_8a2e_5028_ab3f, // Revert
        0x032c_d3de_521b_4ecd, // Skitch
        0x4ab4_b3c7_411f_583a, // Handplant
        0xccb9_5a83_c78b_4ff9, // Boneless
        0x3eda_5b14_0604_613d, // Offboard pushing
        0x118b_9162_e026_2fbf, // Offboard jump
        0x58db_c281_5ca6_66fd, // Skeleton head tracking
        0x1ca5_e1dd_0b2e_a896, // Skeleton landing wobble
        0xcdd9_26b3_5ef0_f6c4, // OffboardTrajectorySelector
        0x172d_5539_37d8_e07c, // FootPlantManager
        SkateboardAttributeClass::Deck.key(),
        SkateboardAttributeClass::Trucks.key(),
        SkateboardAttributeClass::TrucksDrives.key(),
        SkateboardAttributeClass::Wipeout.key(),
        SkateboardAttributeClass::Slide.key(),
        SkeletonAttributeClass::Skeleton.key(),
        SkeletonAttributeClass::Bones.key(),
        SkeletonAttributeClass::Drives.key(),
        SkeletonAttributeClass::Joints.key(),
        skeleton_body_attributes::ANIMATION_CLASS,
        skeleton_body_attributes::DEBUG_PHYSICS_CLASS,
        HatAttributes::CLASS,
        SkeletonCollisionAttributes::CLASS,
        ANIMATION_DRIVES_CLASS,
        crate::skateboard_foot_attributes::SkateboardFootVaultAttributes::CLASS,
        crate::skateboard_brake_attributes::SkateboardBrakeVaultAttributes::CLASS,
        crate::skateboard_steering::STEERING_CLASS,
        0x2574_e307_ae61_a71e,
        0x3a2a_8b50_4536_22bb,
        0x7379_c446_7aa7_5df9,
        0xf59f_e715_b7ae_3426,
        crate::reckoning_attributes::RECKONING_CLASS,
        0x90c3_decf_0105_631e, // anim_motion.jumping
        0x3505_a2ac_5dde_a557, // anim_wipeout.default
        0xc2e0_a8b6_c139_90fd, // physics_jump.default
        crate::skateboard_force_runtime::push::attributes::PushAttributes::CLASS, // physics_push.default
        crate::actor_construction::breadcrumbs::CLASS, // skater_breadcrumbs.default
        0x7f08_6875_d621_c88c,                         // recognizer left/right
        0x7dea_05dc_70a1_ed7d,                         // physics_skatecontroller.default
        0x1477_b0d5_343d_29fd,                         // livingworld_entities_chase.global
        0x86eb_6b56_4cf1_2561,                         // livingworld_entities_perceptions.global
        crate::action_graph_input_listener::INPUT_LISTENER_CLASS,
        0x5be4_6545_de48_5c98, // board SpeedWobble
        0x4920_26e0_f096_f283, // animation offboard
        crate::skeleton_current_animation::PHYSICS_ANIMATION_CLASS,
        0xc8be_3349_f2bd_493e,
        0x91bc_a669_3efc_9aa7,
        0xb6b8_9972_86f1_9549, // selected material_character
        0xa0d7_351e_fa17_6078, // physics_manual
        0x28ab_b872_cb8c_b9ed, // physics_airstates
        0xab3a_006f_3a29_56e2, // physics_pumping (Pumping + pump force; global+f8)
        0xa099_c166_f4be_639f, // physics_speed_conservation (SpeedModel; global+c4)
        crate::physical_player_grind_manager::view::PHYSICS_GRINDS_CLASS, // physics_grinds (global+a4)
        // AISkaterManager, AIController and Skateboard::UpdateAIPathFollowing.
        crate::ai::attributes::AI_SKATER_CLASS,
        crate::ai::profile::AI_SKATER_PROFILES_CLASS,
        crate::ai::tricks::TRICK_CLASS,
        crate::skateboard_ai_path_following::PHYSICS_AI_CLASS,
        // PhysOutConditioner_Audio::Update reads GlobalAttributeCollection's aud_general and
        // aud_contacts collections.
        0xc183_1bdb_6cb1_b1ea, // aud_general
        0xc269_49fc_b638_a2ca, // aud_contacts
        // LivingWorld DMOs: ModelMan::AddDMOModel, the census and SpawnDynamicObject.
        crate::living_world_grind_services::dmo_models::CHARACTERISTICS_CLASS,
        crate::living_world_grind_services::dmo_models::PRIORITY_CLASS,
        crate::living_world_grind_services::dmo_objects::CENSUS_RANGES_CLASS,
        crate::living_world_grind_services::dmo_objects::ENTITIES_CLASS,
    ]
}

pub(crate) fn selected_classes() -> Vec<u64> {
    let additional = selected_additional_classes();
    let mut classes = Vec::with_capacity(BASE_PHYSICS_CLASSES.len() + additional.len());
    classes.extend(BASE_PHYSICS_CLASSES);
    classes.extend(additional);
    classes
}
