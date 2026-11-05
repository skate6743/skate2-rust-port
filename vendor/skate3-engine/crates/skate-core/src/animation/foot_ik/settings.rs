//! Authored IK parameters, resolved against the stock class layout.
use super::status::BlendSettings;
use super::two_bone::AngleLimits;

#[derive(Clone, Copy, Debug)]
pub struct Settings {
    /// physics_animation/default layout876/880, consumed by82BF0FA8.
    pub angle_limits: AngleLimits,
    pub blend: BlendSettings,
    /// physics_skeletonik layout0/48/64/80.
    pub post_ik_padding: [f32; 4],
    pub contact_bounds: [f32; 4],
    pub foot_on_deck_padding: [f32; 4],
    pub wipeout_feet_offset: f32,
    ///82BED418 retrieves physicsdeck hashes EE1FD008/1DCB828B/932190E7.
    pub deck_half_width: f32,
    pub deck_half_length: f32,
    pub deck_total_half_length: f32,
    /// physicsdeck.DeckFrontEndAngle, hashF9B732CA; retained authored degrees.
    pub deck_front_angle_degrees: f32,
}
