//! Completed PhysOut -> chromosome82DEE508, before filtered82DE56B0.
use super::{
    super::{Skate3PhysicsState, Skate3ControllerState, grind_chromosome::Input},
    Family,
};
use skate3_core::physics::board::BodyId;

/// Run on EVERY completed publication, not merely physical grind frames.
///82DF2130 installs chromosome at slot1 and filtered at slot5;82DF2710
///dispatches in ascending order. Board/Skeleton/state Fill must precede this.
pub(crate) fn condition(physics: &Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    let out = &mut skater.player_input.physical.grinds;
    //Common82DB7948..7968 derives this after state Fill, not from category400.
    out.grinding_316 = u8::from(matches!(out.words_136_140[1], 1 | 2));
    let family = match out.words_136_140[0] {
        0 => Some(Family::FiftyFifty),
        1 => Some(Family::Boardslide),
        2 => Some(Family::Tipslide),
        3 => Some(Family::FiveO),
        4 => Some(Family::Backslash),
        5 => Some(Family::Darkslide),
        //The reset template uses u32::MAX. No family is normal outside grind;
        //history and previous-category bookkeeping must still run this frame.
        _ => None,
    };
    if out.grinding_316 != 0 && family.is_none() {
        return Err(format!(
            "Active grind has invalid native family {}",
            out.words_136_140[0]
        ));
    }
    //Board Fill82C02AD8/B14 copies the unadjusted part transform. Its effective
    //variant82C01BF8 supplies Basic96 and128; neither is the Skeleton forward.
    let deck = physics.board.part_transforms()[BodyId::Deck.index()];
    let axis = |a: [f32; 3]| [a[0], a[1], a[2], 0.0];
    let effective_forward = axis(physics.riding.motion.effective_basis.columns[2]);
    //Basic144 comes from the dynamics COM at82C02D0C, not part translation48.
    let com = physics.board.bodies()[BodyId::Deck.index()].rates.position;
    let raw = |v: [u32; 4]| v.map(f32::from_bits);
    let input = Input {
        category: skater.player_input.physical.state.category_12,
        grinding_316: out.grinding_316 != 0,
        //Common82DB7480 copies Processed2468 bit21 to Air439 on every frame.
        air_event_439: skater.player_input.processed.flags_2468 & 0x0020_0000 != 0,
        family,
        basic_right_0: axis(deck.basis.columns[0]),
        basic_forward_32: axis(deck.basis.columns[2]),
        basic_position_48: [
            deck.translation.x,
            deck.translation.y,
            deck.translation.z,
            0.0,
        ],
        basic_location_axis_96: effective_forward,
        basic_twist_axis_128: effective_forward,
        basic_location_position_144: [com.x, com.y, com.z, 0.0],
        //82BE1D00..1E58: physical pose starts8016;9232=part19,8976=part15.
        //256/272 receive column0, not toe positions or IK/rendered bones.
        feet_256_272: [
            skater.skeleton.record.pose[19][0],
            skater.skeleton.record.pose[15][0],
        ],
        //Common82DB7A90 copies the completed animation packet's byte10372.
        fakie_155: skater.animation.packet.riding_fakie,
        point: raw(out.point_16),
        direction: raw(out.direction_0),
        normal: raw(out.normal_32),
        across: raw(out.across_48),
    };
    skater.grind.condition_outputs(input, out)
}
