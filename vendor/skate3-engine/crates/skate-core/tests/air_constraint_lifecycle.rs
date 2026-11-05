//! Source-derived lifecycle checks, not a replay or hardware parity fixture.
use skate_core::{
    animation::foot_ik::status::{self, BlendSettings, LimbStatus, Mode},
    physics::hook_drive::HookDriveState,
};

#[test]
fn landing_releases_both_deck_drives_and_second_air_entry_is_angular_only() {
    let mut drive = HookDriveState::initial();
    let mut animated = 0;
    // Start with both channels armed, so stale linear strength is observable.
    drive.enable_animation_soft(&mut animated);
    assert_ne!(drive.dynamics[2], 0);
    assert_ne!(drive.dynamics[6], 0);
    let frames = drive.frames;

    // Ground Enter82D375D0..5FC clears both channels, retaining the anchors.
    drive.disable_animation(&mut animated);
    assert_eq!(animated, 0);
    assert_eq!(drive.dynamics, [0, 0, 0, 2, 0, 0, 0, 2]);
    assert_eq!(drive.frames, frames);

    // PhysicsAir Enter82D343C8 ->82C05658 must not revive linear following.
    drive.enable_angular_only(&mut animated);
    assert_eq!(animated, 1);
    assert_eq!(&drive.dynamics[..4], &[0, 0, 0, 2]);
    assert_ne!(drive.dynamics[6], 0);
    assert_eq!(drive.dynamics[7], 1);
    assert_eq!(drive.frames, frames);
}

#[test]
fn released_grab_hand_waits_for_geometric_blend_before_disabling() {
    let mut limbs = [LimbStatus::default(); 4];
    limbs[2].mode = Mode::OnDeck;
    limbs[2].board_blend = 0.5;
    limbs[2].part_position = [4.0, 0.0, 0.0, 0.0];
    // Synthetic dimensions/steps expose82BEDCF8 and82BEEC00's ordering;
    // these values are not proposed stock settings or a release-time tuning.
    let settings = BlendSettings {
        hand_inner_padding: [0.25; 4],
        hand_outer_padding: [0.25; 4],
        external_blend_step: 0.25,
        board_blend_step: 0.25,
    };
    for remaining in [0.25, 0.0] {
        status::update_modes(&mut limbs, true, 0);
        assert_eq!(limbs[2].mode, Mode::OnDeck);
        status::update_blends(&mut limbs, 0.5, 1.0, 0, &settings);
        assert_eq!(limbs[2].board_blend, remaining);
    }
    status::update_modes(&mut limbs, true, 0);
    assert_eq!(limbs[2].mode, Mode::Disabled);
    assert_eq!(limbs[2].external_blend, 0.0);
    assert_eq!(limbs[0].mode, Mode::OnDeck);
    assert_eq!(limbs[1].mode, Mode::OnDeck);
}
