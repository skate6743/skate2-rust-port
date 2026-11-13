//! `SKATE2_TRACE_SKATE3_SPIN`: one stderr line per physics tick with everything that sets the
//! body spin, after the solve. Fields:
//! - `lx`, `rx`, `ry`: the derived controller's left X, right X and right Y (what the intents read);
//! - `pbs`, `bs`: the PhysBodySpin and BodySpin attributes (Processed +2812, +2640);
//! - BodySpin (TU3 0x82D8BF58) state: `in` (+120), `f` (+128 flick this tick), `sp` (+140 speed),
//!   `pk` (+148 credit), `pt` (+156 credit time), `tm` (+152 air time), `au` (+164 auto target);
//! - `rs`: the reckoning spin speed (+1572); `hy`, `dy`: the reckoning heading's and the deck's yaw
//!   in degrees, `dp` the deck's pitch; `ba`: the BoardAdjust attribute (Processed +2492: 1 left,
//!   2 right, 3 up, 4 down); `js`: the jump strength; `sf`: frames since the jump (Processed +2572);
//! - `k`: Processed 2468 bit 10 (a landing prediction: KnownAir on the jump);
//! - `kv`, `rem`: KnownAir's landing heading flag and its remaining time to the landing.
//!
//! `SKATE2_TRACE_SKATE3_LAND` adds a `land3` line per tick for touchdown comparisons: the deck
//! body's solved linear velocity, angular velocity and position (`bv`, `bw`, `bp`); Processed
//! +400, +720 and +464 (`pv`, `pw`, `pn`), +2652, +2612 (`s52`, `s12`), flags +2468, +2472, +2476
//! (`f68`, `f72`, `f76`), wheel count +2556 (`wc`) and time on ground +2752 (`tg`); the balance
//! attribute (`bal`); the manual controller's board fields +264..+280 (`mf`, `mt`, `mm`, `mc`, `me`).
//!
//! With it, `SKATE2_TRACE_SKATE3_PARTS` adds a `parts3` line per tick: each board body's solved linear and
//! angular velocity (`v0`..`v6`, `w0`..`w6`, native part order, the deck is 6) and the board force
//! queue this tick (`f0`.. as tag, world force xyz, body point xyz).
use super::{PlayerControls, Skate3ControllerState, Skate3PhysicsState};

pub(super) fn tick(
    tick: u64,
    physics: &Skate3PhysicsState,
    skater: &Skate3ControllerState,
    controls: &PlayerControls,
) {
    static TRACE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if !*TRACE.get_or_init(|| std::env::var_os("SKATE2_TRACE_SKATE3_SPIN").is_some()) {
        return;
    }
    let spin = physics.riding.body_spin.words();
    let state = |offset: usize| f32::from_bits(spin[offset / 4]);
    let axes = controls.controller.words();
    let axis = |index: usize| f32::from_bits(axes[index]);
    let yaw = |v: [f32; 4]| v[0].atan2(v[2]).to_degrees();
    let deck = physics.board.part_transforms()
        [skate3_core::physics::board::BodyId::Deck.index()]
    .basis
    .columns[2];
    let p = &skater.player_input.processed;
    let known = &skater.known_air.state;
    eprintln!(
        "spin3 t={tick} st={:?} lx={:.3} rx={:.3} ry={:.3} pbs={:.3} bs={:.3} in={:.3} f={:.3} sp={:.3} pk={:.3} pt={:.3} tm={:.3} au={:.3} rs={:.3} hy={:.2} dy={:.2} dp={:.2} ba={} js={:.3} sf={} k={} kv={} rem={:.3}",
        skater.player_state.current(),
        axis(7),
        axis(9),
        axis(10),
        skater.animation_input.extra.physical_body_spin,
        skater.animation_input.fields.body_spin,
        state(120),
        state(128),
        state(140),
        state(148),
        state(156),
        state(152),
        state(164),
        skater.air_reckoning.state.spin_speed,
        yaw(physics.riding.reckoning_frames.heading),
        yaw([deck[0], deck[1], deck[2], 0.0]),
        deck[1].clamp(-1.0, 1.0).asin().to_degrees(),
        skater.animation_input.fields.board_adjust,
        skater.animation_input.extra.jump_strength,
        skater.player_state.post.state_frames,
        u8::from(p.flags_2468 & 0x400 != 0),
        u8::from(known.landing_heading_valid_209),
        known.collision_time_196 - known.time_in_state_180,
    );
    static LAND: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if !*LAND.get_or_init(|| std::env::var_os("SKATE2_TRACE_SKATE3_LAND").is_some()) {
        return;
    }
    let body = &physics.board.bodies()[skate3_core::physics::board::BodyId::Deck.index()].rates;
    let v3 = |v: skate3_core::math::Vector3| format!("{:.5},{:.5},{:.5}", v.x, v.y, v.z);
    let raw3 = |v: [u32; 4]| {
        format!("{:.5},{:.5},{:.5}", f32::from_bits(v[0]), f32::from_bits(v[1]), f32::from_bits(v[2]))
    };
    let manual = &skater.ground.manual;
    eprintln!(
        "land3 t={tick} bv={} bw={} bp={} pv={} pw={} pn={} s52={:.5} s12={:.5} f68={:08x} f72={:08x} f76={:08x} wc={} tg={:.4} bal={:.4} mf={:.6} mt={:.6} mm={:.6} mc={:.6} me={:.4}",
        v3(body.linear_velocity),
        v3(body.angular_velocity),
        v3(body.position),
        raw3(p.vectors_400_416[0]),
        raw3(p.vectors_720_784_800_816_832_864[0]),
        raw3(p.vectors_464_480_496_512_528[0]),
        p.scalar_2652,
        p.scalar_2612,
        p.flags_2468,
        p.flags_2472,
        p.flags_2476,
        p.wheel_count_2556,
        p.time_on_ground_2752,
        skater.animation_input.fields.balance,
        manual.filtered_angle_error,
        manual.target_angle,
        manual.measured_angle,
        manual.angular_correction,
        manual.elapsed,
    );
    static PARTS: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if !*PARTS.get_or_init(|| std::env::var_os("SKATE2_TRACE_SKATE3_PARTS").is_some()) {
        return;
    }
    use std::fmt::Write;
    let mut line = format!("parts3 t={tick}");
    for (i, body) in physics.board.bodies().iter().enumerate() {
        let r = &body.rates;
        let _ = write!(line, " v{i}={} w{i}={}", v3(r.linear_velocity), v3(r.angular_velocity));
    }
    for (k, force) in physics.board.forces().entries().iter().enumerate() {
        let _ = write!(line, " f{k}={},{},{}", force.tag, v3(force.force_world), v3(force.point_body));
    }
    eprintln!("{line}");
}
