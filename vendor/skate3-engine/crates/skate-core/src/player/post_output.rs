//! TU3 `PhysicalPlayerHiLOD::PostOutput` (`0x82DB6CB0`).
//!
//! This is the final player-owned phase before the world finishes output and
//! conditioners. The native body performs exactly two calls in this order.

/// Required services called by the complete `0x82DB6CB0` wrapper.
pub trait PostOutputServices {
    /// `0x82909510(Player+1392, 0, -1)`.
    fn finish_player_output_82909510(&mut self, player_field_1392: &mut PlayerOutputField1392);

    /// `0x82BE2BA8(Player->Skeleton)`.
    fn finish_skeleton_output_82be2ba8(&mut self);
}

/// Opaque Player+1392-owned object passed to `0x82909510`.
///
/// Its internal layout is deliberately absent: PostOutput only passes its
/// address and the fixed native arguments `0` and `-1`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayerOutputField1392;

/// Runs the complete TU3 PostOutput phase.
pub fn run_post_output(
    player_field_1392: &mut PlayerOutputField1392,
    services: &mut impl PostOutputServices,
) {
    services.finish_player_output_82909510(player_field_1392);
    services.finish_skeleton_output_82be2ba8();
}

#[cfg(test)]
#[path = "tests/post_output.rs"]
mod tests;
