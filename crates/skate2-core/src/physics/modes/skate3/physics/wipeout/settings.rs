//! Globals264 -> original8289D270 class EB844CFDBF928EFD/default.
//! Fields are independently mapped through the stock physics_wipeout schema.
use skate3_core::{
    player::wipeout::{AirSettings, GroundSettings, Mode, Settings},
    point_graph::PointGraph,
};
use skate_data::collections::Collections;
pub(super) fn load(data: &Collections) -> Result<(Settings, [Mode; 5]), String> {
    let f = |name| data.float("physics_wipeout", "default", name);
    let graph = data
        .words::<20>("physics_wipeout", "default", "Hash_4F08D9BAE6831524")?
        .map(f32::from_bits);
    let mut modes = Vec::new();
    for name in ["easy", "normal", "hardcore", "motorized", "test"] {
        modes.push(Mode {
            check_squash: data.boolean("physics_mode", name, "Hash_CC890A3BDCF6290A")?,
            check_bad_landing: data.boolean("physics_mode", name, "WipeoutCheckForBadLanding")?,
            ground_xz: data.float("physics_mode", name, "Wipeout_GroundXZAcceleration")?,
            //82D90D40..94 constructs full64 D978550D6DB4E6F7.
            bad_landing_scale: data.float("physics_mode", name, "Hash_D978550D6DB4E6F7")?,
        });
    }
    Ok((
        Settings {
            ground: GroundSettings {
                vehicle_scalar: f("Wipeout_GroundVehicleScalar")?,
                vehicle_contact: f("Wipeout_GroundVehicleContact")?,
                skitch_contact: f("Wipeout_GroundSkitchingContact")?,
                skitch_scalar: f("Wipeout_GroundSkitchingScalar")?,
                skitch_arms_scalar: f("Wipeout_GroundSkitchingScalarArms")?,
                skater_scalar: f("SkaterSkaterThresholdScalar")?,
                max_squash: f("Wipeout_GroundMaxSquash")?,
                max_squash_coffin: f("Wipeout_GroundMaxSquashCoffin")?,
                max_displacement: f("Wipeout_GroundSkeletonMaxDisp")?,
                max_contact: f("Wipeout_GroundSkeletonMaxContact")?,
                max_arm_contact: f("Wipeout_GroundSkeletonMaxContactArms")?,
                max_deck_error: f("Wipeout_GroundMaxAngularDeckError")?,
                opposing_contact: f("Wipeout_GroundOpposingContact")?,
                y_acceleration: f("Wipeout_GroundYAcceleration")?,
                light_dmo_scalar: f("WipeoutGroundLightDMOScalar")?,
                player_scalar: f("DeckAccPlayerScalar")?,
                ai_scalar: f("DeckAccAIScalar")?,
                skitch_acc_scalar: f("DeckAccSkitchScalar")?,
                balance_total: f("Wipeout_GroundBalanceTotal")?,
                balance_min_speed: f("Wipeout_GroundBalanceMinSpeed")?,
                balance_base: f("Wipeout_GroundBalanceBase")?,
            },
            air: AirSettings {
                xz_trick: f("Wipeout_AirXZTrick")?,
                y_trick: f("Wipeout_AirYTrick")?,
                xz_acceleration: f("Wipeout_AirXZAcceleration")?,
                y_acceleration: f("Wipeout_AirYAcceleration")?,
                max_squash: f("Wipeout_AirMaxSquash")?,
                max_displacement: f("Wipeout_AirSkeletonMaxDisp")?,
                max_contact: f("Wipeout_AirSkeletonMaxContact")?,
                max_arm_contact: f("Wipeout_AirSkeletonMaxContactArms")?,
                body_flip_scalar: f("Wipeout_AirBodyFlipScalar")?,
                body_flip_acc_scalar: f("DeckAccBodyFlipScalar")?,
                light_dmo_scalar: f("WipeoutAirLightDMOScalar")?,
                ignore_danger_frames: data.integer(
                    "physics_wipeout",
                    "default",
                    "FramesAtStartOfAirToIgnoreDangerZone",
                )? as i32,
                max_landing_speed: f("Wipeout_AirMaxSpeedIntoGround")?,
                max_stairs_speed: f("Wipeout_AirMaxSpeedIntoStairs")?,
                max_grind_speed: f("Wipeout_AirMaxSpeedIntoCollisionNearGrind")?,
                max_landing_angle: PointGraph {
                    x: graph[4..12].try_into().unwrap(),
                    y: graph[12..20].try_into().unwrap(),
                },
            },
            lean_contact_y: f("Wipeout_GroundLeanContactYThresh")?,
        },
        modes
            .try_into()
            .map_err(|_| "Expected five wipeout physics modes".to_owned())?,
    ))
}
