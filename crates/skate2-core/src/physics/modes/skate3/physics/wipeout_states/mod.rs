//! Original physical WipeoutGround300. Off-board recovery destinations are
//! separate states; this owner retains the native request and timing fields.
mod contact;
mod drive_settings;
mod lifecycle;
mod output;
mod prediction;
pub(crate) mod ragdoll;
mod settings;
mod skeleton;
mod update;
use skate3_core::player::wipeout_state::{
    State, contact_response::ContactResponse, drives, profiles::Profile,
};
use skate_data::collections::Collections;
use std::path::Path;

pub(crate) struct WipeoutState {
    pub(crate) ragdoll: ragdoll::RagdollSetup,
    pub state: State,
    settings: settings::Settings,
    drives: drives::Settings,
    profiles: [Profile; 5],
    contact: ContactResponse,
    prediction: prediction::Prediction,
}
impl WipeoutState {
    pub fn load(data: &Collections, assets: &Path, primary_bank_sha: &str) -> Result<Self, String> {
        //Ctor82D3B1EC..B25C full64 handle hashes, verified against the stock keys.
        let names = [
            "free_fall",
            "cannon_ball",
            "judo_kick",
            "swan_dive",
            "torpedo",
        ];
        let profiles = names.map(|name| settings::load_profile(data, name));
        let [a, b, c, d, e] = profiles;
        Ok(Self {
            ragdoll: ragdoll::RagdollSetup::load(data, assets, primary_bank_sha)?,
            state: State::default(),
            settings: settings::Settings::load(data)?,
            drives: drive_settings::load(data, assets, primary_bank_sha)?,
            profiles: [a?, b?, c?, d?, e?],
            contact: ContactResponse::new(),
            prediction: prediction::Prediction::new(),
        })
    }
}
pub(crate) use lifecycle::{enter, exit, post_physics};
pub(crate) use output::fill;
pub(crate) use update::advance;
#[cfg(any())]
mod tests {
    use super::*;
    use skate3_core::{
        math::Vector3,
        physics::skeleton_animation_record::IDENTITY,
        player::{
            input_phase::ProcessedPhysicsInput,
            wipeout_state::{control_ground, profiles},
        },
    };

    #[test]
    #[ignore = "requires private stock animation banks and collections"]
    fn bail_gesture_profiles_and_ground_roll_preserve_signed_controls() {
        let root = std::path::PathBuf::from(std::env::var_os("SKATE3_ASSET_ROOT").unwrap());
        let data = Collections::load(&root).unwrap();
        let names = [
            "free_fall",
            "cannon_ball",
            "judo_kick",
            "swan_dive",
            "torpedo",
        ];
        let profiles = names.map(|key| settings::load_profile(&data, key).unwrap());
        for (gesture, expected) in [
            ([0., 0.], 0),
            ([0., 1.], 1),
            ([1., 0.], 2),
            ([0., -1.], 3),
            ([-1., 0.], 4),
        ] {
            let mut state = State::default();
            state.velocity = [0., 0., 5., 0.];
            profiles::prepare(&mut state, &profiles, gesture, [0.75, -0.5]);
            assert_eq!(state.profile, expected);
            assert!((state.sideways_input - 0.75).abs() < 0.001);
            assert!((state.forward_input + 0.5).abs() < 0.001);
        }
        let assets = skate_data::GameAssets::load(&root).unwrap();
        let graphs = crate::skate3_native::graph_runtime::StockGraphs::load(&root, &assets).unwrap();
        let physics = crate::skate3_native::physics::Skate3PhysicsState::load(&root).unwrap();
        let mut skater =
            crate::skate3_native::physics::Skate3ControllerState::load(&root, &graphs, &physics, "normal").unwrap();
        let mut p = ProcessedPhysicsInput::default();
        p.vectors_464_480_496_512_528[0] = [0_f32, 1., 0., 0.].map(f32::to_bits);
        let mut profile = profiles[1].clone();
        profile.roll_axis = [1., 0., 0., 0.];
        profile.roll_on_ground = true;
        profile.align_ground_with_velocity = true;
        profile.ground_roll_torque = 1.;
        // Synthetic stationary skeleton isolates the real torque application.
        skater.skeleton.record.pose = [IDENTITY; 26];
        skater.skeleton.record.pose[1][3] = [0., 1., 0., 0.];
        skater.skeleton.record.velocities = [[0.; 4]; 26];
        skater.skeleton.record.centre_of_mass_velocity = [0.; 4];
        let mut positive: Option<f32> = None;
        for location in [[100., -20., 50., 0.], [-100., 20., -50., 0.]] {
            for sign in [-1., 0., 1.] {
                for part in skater.skeleton.bodies_mut() {
                    part.rates.force_acceleration = Vector3::new(0., 0., 0.);
                }
                let mut state = State::default();
                state.velocity = [0., 0., 5., 0.];
                state.forward_input = sign;
                state.predicted_position = location;
                control_ground::update(
                    &mut state,
                    &mut skater.skeleton,
                    [0.; 4],
                    &IDENTITY,
                    &p,
                    &profile,
                    &IDENTITY,
                    &[IDENTITY; 24],
                );
                let force = skater.skeleton.bodies()[1].rates.force_acceleration;
                assert!(force.x.abs() < 0.001 && force.y.abs() < 0.001);
                if sign == 0. {
                    assert!(force.z.abs() < 0.001);
                } else {
                    assert!(force.z * sign > 0.);
                }
                if sign > 0. {
                    if let Some(previous) = positive {
                        assert!((force.z - previous).abs() < 0.001);
                    }
                    positive = Some(force.z);
                }
            }
        }
    }
}
