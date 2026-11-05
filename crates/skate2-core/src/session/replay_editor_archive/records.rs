//! Lossless presentation payloads for the PC replay archive (no SIM state).
use super::codec::{Store,Writer,Reader,Result,record};
record!(skate2_formats::andale::AndaleSqt {scale,rotation,translation});
record!(crate::SkaterAudioCollision {
    contact_force, slide_speed, audio_surface_type, is_groin_contact, is_face_contact, vehicle_impact, skater_impact, pedestrian_impact
});
record!(crate::SkaterAudioPhysOut {
    centre_of_mass, centre_of_mass_velocity, hips_mat_y, foot_l_vel_deck_space, foot_r_vel_deck_space, foot_left_vel, foot_right_vel, torso_angular_vel, right_foot_angular_vel, left_foot_angular_vel, absorption_compression, average_limb_speed, is_wipeout_over, is_left_foot_on_deck, is_right_foot_on_deck, is_grabbing_left, is_grabbing_right, effective_transform_y, deck_nose_dir, deck_position, wheel_pos, deck_velocity, skateboard_surface_speed, deck_tilt, wheel_hardness, time_in_known_air, traj_collision_time, jump_height, foot_plant_surface_id, hand_plant_surface_id, is_skateboard_airborne, is_starting_to_jump, is_foot_planting, is_left_foot_planting, is_right_foot_planting, hand_plant_grabbing, num_wheels_in_contact, front_truck_audio_surface_type, back_truck_audio_surface_type, deck_audio_surface_type, deck_slide_speed, wheel_audio_surface_type, wheel_surface_pattern, is_front_truck_in_contact, is_back_truck_in_contact, is_deck_in_contact, physics_state, is_foot_braking, is_manual_braking, is_physically_pushing, is_generally_pushing, is_generally_pushing_right_foot, is_wiping_out, is_manual, is_reverting, turn, ground_up_vector, deck_local_xyz_angular_speed, wheel_slide, jump_fraction, grind_type, grind_impact_speed, averaged_deck_contact_force, grind_audio_surface_type, off_board_jump_landing, hippy_jump_strength, wheel_impact_strength, wheel_landed, averaged_skeleton_collision_info, is_grinding, is_on_stairs, is_walking_up, is_walking_down, push_mongo, line_score, wipedout_trick_score, banked_point_change_this_frame, audio_banked_point_change, raw_scorable_id, left_foot_surface_id, right_foot_surface_id, is_grabbing_object, left_foot_down, right_foot_down, is_unmounting, is_air_dismount, is_holding_skateboard, is_jumping, is_starting_standing_jump, momentum_meter_multiplier, overall_wipeout_duration, contact_force_total_during_wipeout, hom_bail_in_progress, hom_effect_enabled, hom_score
} ..default);
record!(crate::AudioPedestrian {
    id, state, length, topic, target_greet_character, flags, audio_character, distance, local_to_world
});
record!(crate::AudioSkater {
    player_index, local, phys_out, can_see_player, ai_audio
});
record!(crate::AudioVehicle {
    id, acceleration, speed, distance, horn_state, skidding, engine_key, matrix
});
record!(crate::AudioDynamicObject {
    id, velocity, grabbed, characteristic_key, angular_velocity, contact, on_side, bounds_center
});
record!(crate::AudioFrame {
    frame, camera_at, camera_position, skaters, reverb_key, ambience_key, emitter_key, out_of_boundary_amount, fade_amount, pedestrians, vehicles, dynamic_objects, local_race_ranking, challenge, challenge_names, player_flags, security
});
record!(crate::SecurityPresentation {
    chasers, flags, tazer, bodyguard
});
record!(crate::SelectedDynamicObjectVisual {
    entity_id, asset_id, render_model_entry, part_transform, opacity_parameters, opacity_settings, simulation_center_velocity
});
record!(crate::LivingWorldOpacityDistance {
    opaque, transparent, speed
});
record!(crate::LivingWorldOpacityParameters {
    far_min_speed, far_max_speed, near_min_speed, near_max_speed
});
record!(crate::LivingWorldOpacitySettings {
    enabled, fade_in_frames
});
record!(crate::animation::AnimationSkeleton {
    parents, trajectory_bone, parts
});
record!(crate::animation::SkeletonPart {
    name, bones
});
record!(crate::character::CharacterHierarchy {
    bone_names
});
record!(crate::character::CharacterSkinningPart {
    component_index, model_index, lod_index, bone_remap, fallback_bone_count, packed_matrix_count
});
record!(crate::character::CharacterSkinningState {
    lod_level, hierarchy_inverse_bind_pose, parts
});
record!(crate::presentation_restore::camera::CameraSubmission {
    frames, lens
});
record!(crate::presentation_restore::camera::CameraLens {
    near_plane, far_plane, override_fov, update_counter
});
record!(crate::camera_frame::CameraFrame {
    matrix, previous_matrix, velocity, angular_velocity, anchor, reference, field_of_view, framing_x, framing_y, near_plane, motion_blur, index_override, hard_cut
});
record!(crate::skeleton_root_transforms::SkeletonRootVector {
    xyz, w
});
record!(crate::presentation_restore::living_world::TransformReplay {
    rotation, translation
});
record!(crate::presentation_restore::living_world::EntityReplay {
    id, transform
});
record!(crate::presentation_restore::living_world::VehicleReplay {
    entity, wheels
});
record!(crate::presentation_restore::living_world::ObjectReplay {
    entity, part
});
record!(crate::presentation_restore::living_world::LivingWorldSubmission {
    pedestrians, vehicles, objects
});
record!(crate::presentation_restore::living_world::VehicleWheels {
    spin, front, lights
});
impl Store for crate::skeleton_root_transforms::SkeletonRootMatrix {
    fn put(&self,w:&mut Writer){self.columns().put(w);}
    fn get(r:&mut Reader<'_>)->Result<Self>{Ok(Self::from_columns(Store::get(r)?))}
}
