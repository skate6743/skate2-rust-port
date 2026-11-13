//! Drop-in diagnostics, after the solve and before the outputs reset:
//! - `SKATE2_TRACE_SKATE3_EDGES=<tick>:<radius>`: at that physics tick, one `edge3` line per grind
//!   primitive whose chord passes within the radius (metres, horizontally) of the deck, nearest
//!   first, with its chord ends and that distance; for finding a coping to drop in from.
//! - `SKATE2_TRACE_SKATE3_DROP_IN_TICKS`: one `dropin3` line per tick: the physical state, the
//!   off-board edge distance (OffBoard +116), Processed flags +2476, +2480, +2488 (OB_Traj is
//!   2476 bit 15, OB_DropIn 2488 bit 28), state +2508 and category +2512, the grind family and
//!   Grinds bytes 324, 325, 326 (IsDroppingIn, IsDropInWithoutTransition), the deck position and
//!   Processed +2652; then the board controller's state (+448: 1 held, 2 let go, 3 hidden,
//!   4 retrieving) and system flag (+452), the board flags +872 (bit 26 lets a held board go), the
//!   hand alignment's two directions and factor when set, the hook position, and each board
//!   part in contact with its normal.
use super::{Skate3ControllerState, Skate3PhysicsState};
use skate3_core::physics::board::BodyId;

pub(super) fn tick(tick: u64, physics: &Skate3PhysicsState, skater: &Skate3ControllerState) {
    static EDGES: std::sync::OnceLock<Option<(u64, f32)>> = std::sync::OnceLock::new();
    let deck = physics.board.bodies()[BodyId::Deck.index()].rates.position;
    if let Some((at, radius)) = *EDGES.get_or_init(|| {
        let spec = std::env::var("SKATE2_TRACE_SKATE3_EDGES").ok()?;
        let (at, radius) = spec.split_once(':')?;
        Some((at.trim().parse().ok()?, radius.trim().parse().ok()?))
    }) && at == tick
    {
        let mut near: Vec<(f32, usize)> = physics
            .grind_world
            .primitives()
            .iter()
            .enumerate()
            .map(|(index, primitive)| {
                let (a, b) = (primitive.start, primitive.end);
                let (dx, dz) = (b[0] - a[0], b[2] - a[2]);
                let square = dx.mul_add(dx, dz * dz);
                let t = if square > 1e-6 {
                    ((deck.x - a[0]).mul_add(dx, (deck.z - a[2]) * dz) / square).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let (px, pz) = (t.mul_add(dx, a[0]), t.mul_add(dz, a[2]));
                ((deck.x - px).hypot(deck.z - pz), index)
            })
            .filter(|(distance, _)| *distance <= radius)
            .collect();
        near.sort_by(|a, b| a.0.total_cmp(&b.0));
        eprintln!("edge3 t={tick} deck={:.3},{:.3},{:.3} count={}", deck.x, deck.y, deck.z, near.len());
        for (distance, index) in near.into_iter().take(200) {
            let primitive = &physics.grind_world.primitives()[index];
            let (a, b) = (primitive.start, primitive.end);
            eprintln!(
                "edge3 i={index} d={distance:.2} a={:.3},{:.3},{:.3} b={:.3},{:.3},{:.3} owner={}",
                a[0], a[1], a[2], b[0], b[1], b[2], primitive.owner
            );
        }
    }
    static TICKS: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if !*TICKS.get_or_init(|| std::env::var_os("SKATE2_TRACE_SKATE3_DROP_IN_TICKS").is_some()) {
        return;
    }
    let p = &skater.player_input.processed;
    let physical = &skater.player_input.physical;
    eprintln!(
        "dropin3 t={tick} st={:?} edge={:.3} f76={:08x} f80={:08x} f88={:08x} s08={} c12={} fam={} g324={} g325={} g326={} pos={:.3},{:.3},{:.3} s52={:.3}",
        skater.player_state.current(),
        physical.off_board.distance_116,
        p.flags_2476,
        p.flags_2480,
        p.flags_2488,
        p.state_2508,
        p.category_2512,
        skater.grind.active_name().unwrap_or("-"),
        physical.grinds.dropping_in_324,
        physical.grinds.tipslide_325,
        physical.grinds.tipslide_326,
        deck.x,
        deck.y,
        deck.z,
        p.scalar_2652,
    );
    use std::fmt::Write;
    let fields = &skater.skateboard_controller.fields;
    let hook = physics.board.hook_transform().translation;
    let mut line = format!(
        "dropin3b t={tick} bs={} on={} cf={:08x} hook={:.3},{:.3},{:.3}",
        fields.state_448,
        u8::from(fields.system_on_452),
        physics.riding.ground.collision_flags,
        hook.x,
        hook.y,
        hook.z,
    );
    if let Some(a) = skater.board_possession_live.alignment() {
        let _ = write!(
            line,
            " al={:.2},{:.2},{:.2}/{:.2},{:.2},{:.2}/{:.2}",
            a.first_1008[0], a.first_1008[1], a.first_1008[2],
            a.second_1024[0], a.second_1024[1], a.second_1024[2],
            a.factor_1040,
        );
    }
    for (index, part) in physics.riding.ground.parts.iter().enumerate() {
        if part.in_contact {
            let n = part.normal;
            let _ = write!(line, " c{index}={:.2},{:.2},{:.2}", n.x, n.y, n.z);
        }
    }
    eprintln!("{line}");
}
