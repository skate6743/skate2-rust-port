//! Diagnostic observations only: never repair, reset or clamp simulation values.
#[path = "dev_trace_buffer.rs"]
mod buffer;
use super::{Skate3PhysicsState, Skate3ControllerState};
use std::sync::{Mutex, OnceLock};
use skate3_core::{physics::board::BodyId};
static TRACE: OnceLock<Mutex<buffer::Buffer>> = OnceLock::new();
fn trace() -> &'static Mutex<buffer::Buffer> { TRACE.get_or_init(Default::default) }
pub struct TickGuard;
impl Drop for TickGuard {
    fn drop(&mut self) { if std::thread::panicking() { dump("panic during physics tick"); } }
}
pub fn begin(p: &Skate3PhysicsState, s: &Skate3ControllerState, inputs: [f32;18]) -> TickGuard {
    let packet = &s.player_input.processed;
    let deck = &p.board.bodies()[BodyId::Deck.index()].rates;
    let line = format!("TICK tick={} dt={} state={:?} input64_81={inputs:?} flags={:08x}/{:08x}/{:08x}/{:08x}/{:08x} previous_processed={} previous_balance={} force_mode={} wheels={} contacts={} pos={:?} vel={:?} angular={:?} previous_grab={:?} previous_trick={:?}",
        p.ticks, p.settings.step.simulation.time_step, s.player_state.current(),
        packet.flags_2468,packet.flags_2472,packet.flags_2476,packet.flags_2480,packet.flags_2484,
        packet.state_2508,s.animation_input.fields.balance,s.skeleton_input.force_mode,
        packet.wheel_count_2556,p.contact_count,deck.position,deck.linear_velocity,deck.angular_velocity,
        s.animation.motion.score_packet.grab,s.animation.motion.score_packet.trick_names.first);
    if let Ok(mut b)=trace().lock() { b.tick_id=p.ticks; b.tick(line); }
    TickGuard
}
/// Leaf producers supply their own inputs BEFORE they overwrite retained outputs.
pub fn event(stage: &str, details: String) {
    if let Ok(mut b)=trace().lock() { let tick=b.tick_id; b.stage(format!("PROBE tick={tick} {stage} {details}")); }
}
pub fn dump(reason: &str) {
    // Never deadlock or panic a second time while reporting an unwind.
    let lines = trace().try_lock().ok().and_then(|mut b| b.freeze(reason));
    if let Some(lines)=lines {
        use std::io::Write;
        let mut stderr=std::io::stderr().lock();
        for line in lines { let _=writeln!(stderr,"REPORT_PHYSICS {line}"); }
        let _=stderr.flush();
    }
}
pub fn checkpoint(stage: &str, p: &Skate3PhysicsState, s: &Skate3ControllerState) {
    let roots=&s.animated_skeleton.roots;
    let frames=&s.animated_skeleton.board_frames;
    let r=&p.riding.reckoning;
    event(stage,format!("tick={} state={:?} processed={} balance={} up={:?} heading={:?} ground_normal={:?} root={:?} com={:?} lifted_com={:?}",
        p.ticks,s.player_state.current(),s.player_input.processed.state_2508,s.animation_input.fields.balance,
        r.up,p.riding.reckoning_frames.heading,r.ground_normal,roots.animation_to_world,frames.com_frame,frames.lifted_com_frame));
    for (name,matrix) in [
        ("reckoning",&p.riding.reckoning_frames.system),
        ("animation_board",&s.animated_skeleton.animation_board),
        ("animation_root",&roots.animation_to_world),
        ("com_frame",&frames.com_frame),("lifted_com_frame",&frames.lifted_com_frame),
    ] {
        if matrix.iter().flatten().any(|v| !v.is_finite()) {
            dump(&format!("tick={} stage={stage} field={name}",p.ticks)); return;
        }
    }
    for (i,body) in p.board.bodies().iter().chain(s.skeleton.bodies()).enumerate() {
        let r=&body.rates;
        let invalid=[r.position,r.linear_velocity,r.angular_velocity,r.force_acceleration,r.torque_acceleration]
            .iter().any(|v| !v.x.is_finite() || !v.y.is_finite() || !v.z.is_finite());
        if invalid { event("invalid_body",format!("board_then_skeleton_index={i} body={body:?}")); dump(&format!("tick={} stage={stage} field=body_rates index={i}",p.ticks)); return; }
    }
}
