//! Stock vault bindings for WipeoutGround300 and its authored control profiles.
use skate3_core::{
    player::wipeout_state::{profiles::Profile, recovery},
    point_graph::PointGraph,
};
use skate_data::collections::Collections;

pub(crate) struct Settings {
    pub recovery: recovery::Settings,
    pub remove_target_time: f32,
    pub remove_drives_time: f32,
    pub controlled_weight_step: f32,
    pub collision_weight_step: f32,
    pub board_restitution: f32,
    pub board_friction: f32,
    pub deck_angular_drag: f32,
    pub push_force: f32,
    pub standard_materials: [skate3_core::physics::contact::RetailContactMaterial; 3],
}

impl Settings {
    pub(crate) fn load(data: &Collections) -> Result<Self, String> {
        let f = |name| data.float("physics_wipeout", "default", name);
        Ok(Self {
            recovery: recovery::Settings {
                minimum_time: f("TeleportMinTimeForAutoReset")?,
                minimum_settled: f("TeleportMinTimeAfterSettlingForAutoReset")?,
                maximum_time: f("TeleportMaxTime")?,
                fade_time: f("TeleportAutoResetFadeOutTime")?,
                over_speed: f("WipeoutOverSpeed")?,
                over_minimum_time: f("WipeoutOverMinTime")?,
            },
            remove_target_time: f("TimeToRemoveHook")?,
            remove_drives_time: f("TimeToRemoveDrives")?,
            controlled_weight_step: f("DynamicDriveWeightVelControlled")?,
            collision_weight_step: f("DynamicDriveWeightVelCollision")?,
            board_restitution: f("SkateboardRestitution")?,
            board_friction: f("SkateboardFriction")?,
            deck_angular_drag: data.float("physicsdeck", "default", "DeckAngularDrag")?
                * f32::from_bits(0x426F_FFFF),
            push_force: f("LivingWorldPushForce")?,
            standard_materials: [
                material(data, "physicswheels", "Wheel")?,
                material(data, "physicstrucks", "Truck")?,
                material(data, "physicsdeck", "Deck")?,
            ],
        })
    }
}

fn material(
    data: &Collections,
    class: &str,
    prefix: &str,
) -> Result<skate3_core::physics::contact::RetailContactMaterial, String> {
    Ok(skate3_core::physics::contact::RetailContactMaterial {
        static_friction: data.float(class, "default", &format!("{prefix}StaticFriction"))?,
        dynamic_friction: data.float(class, "default", &format!("{prefix}DynamicFriction"))?,
        restitution: data.float(class, "default", &format!("{prefix}Restitution"))?,
    })
}

/// The key is supplied by the original constructor's verified handle binding.
/// PointNegGraph metadata occupies four words before the eight X/Y samples.
pub(crate) fn load_profile(data: &Collections, key: &str) -> Result<Profile, String> {
    let class = "physics_wipeout_control";
    let f = |name| data.float(class, key, name);
    let b = |name| data.boolean(class, key, name);
    let v = |name| {
        data.words::<4>(class, key, name)
            .map(|words| words.map(f32::from_bits))
    };
    let graph = data.words::<20>(class, key, "SpinVsTime")?;
    Ok(Profile {
        roll_axis: v("RollOnGroundAxis")?,
        horizontal_axis: v("HorizAlignAxis")?,
        align_euler: v("AlignAxisEuler")?,
        spin_vs_time: PointGraph {
            x: std::array::from_fn(|i| f32::from_bits(graph[4 + i])),
            y: std::array::from_fn(|i| f32::from_bits(graph[12 + i])),
        },
        direction_follow: f("Hash_1BC908CDB3520A8E")?,
        torque_velocity: f("TorqueVelScalar")?,
        torque_distance: f("TorqueDistScalar")?,
        spin_inertia: f("SpinInertia")?,
        roll_on_ground: b("RollOnGround")?,
        ground_roll_torque: f("RollingOnGroundTorqueVelScalar")?,
        sideways_spin: f("Hash_40B2B3C1A87A4D76")?,
        forward_spin: f("Hash_9047531918285FC3")?,
        horizontal_directed: b("Hash_6009FD75F9EDC436")?,
        drift_maximum_speed: f("DriftMaxSpeed")?,
        drift_forward: f("DriftFactorZ")?,
        drift_sideways: f("DriftFactorX")?,
        align_with_velocity: b("AlignWithVel")?,
        align_ground_with_velocity: b("Hash_9626703A9939FE36")?,
        ground_align_torque: f("AlignOnGroundTorqueVelScalar")?,
        tilt_degrees: f("Hash_B65276A8CC7FB760")?,
        drift_drag: f("Hash_7E6B3A99C0A33ABC")?,
    })
}
