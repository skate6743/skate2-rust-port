//! Concrete Ground grab decision82D324B0. Query readiness belongs to ONE owner.
use super::grab_runtime::Owner;
use skate3_core::player::offboard::{
    grab_scene::{self, Descriptor, Query},
    ground_entry::{Frame, State, Vector},
    ground_query::QueryContext,
    ground_sync::{BoardLimits, BoardSettings, board_bounds},
};
pub(crate) struct Input {
    pub frame: Frame,
    pub position: Vector,
    pub bone: Vector,
    pub flags_2476: u32,
    pub flags_2480: u32,
    pub flags_2484: u32,
    pub elapsed: f32,
    pub timer_2852: f32,
    pub context: QueryContext,
}
pub(crate) fn sync(state: &mut State, owner: &mut Owner, settings: BoardSettings, p: Input) {
    if p.flags_2484 & 0x20000000 != 0 || p.flags_2480 & 0x40000 != 0 || p.timer_2852 > 0. {
        return;
    }
    if p.flags_2476 & 0x400000 == 0 {
        owner.invalidate();
        return;
    }
    if state.flags_144_to_150[4] {
        if !(p.elapsed <= 0.5) {
            state.flags_144_to_150[4] = false;
        }
        return;
    }
    let bounds = board_bounds(p.frame, settings.offset_32, settings.extent_16);
    let ready = owner.flags_12836 & 0x40 != 0;
    let mut traced = (ready, None, false);
    if ready {
        if let Some(candidate) = owner.best(p.position) {
            let inner = board_bounds(
                p.frame,
                settings.offset_32,
                settings.extent_0.map(|v| v * f32::from_bits(0x3f733333)),
            );
            state.flags_144_to_150[0] = grab_scene::qualify(
                &candidate,
                p.bone,
                inner,
                limits(settings.margin_444, settings.angle_452, settings.angle_436),
            );
            traced = (ready, Some(candidate.descriptor().id), state.flags_144_to_150[0]);
            if state.flags_144_to_150[0] {
                let key = candidate.descriptor();
                state.counter_152 = key.kind;
                state.counter_156 = key.id;
            }
        }
    }
    // SKATE2_TRACE_SKATE3_GRAB: each change of the grab decision (validated results ready, the
    // best candidate's spline, whether it qualified).
    {
        static LAST: std::sync::Mutex<Option<(bool, Option<u32>, bool)>> = std::sync::Mutex::new(None);
        if std::env::var_os("SKATE2_TRACE_SKATE3_GRAB").is_some()
            && let Ok(mut last) = LAST.lock()
            && *last != Some(traced)
        {
            *last = Some(traced);
            eprintln!("skate3 grab decision: ready={} candidate={:?} qualified={} position={:?} bone={:?}",
                traced.0, traced.1, traced.2, p.position, p.bone);
        }
    }
    //Query always submits, even when the previous result was rejected.
    owner.query(Query {
        position: p.position,
        sort_position: p.position,
        bounds,
        limits: limits(settings.margin_444, settings.angle_456, settings.angle_440),
        mode: 4,
        capacity: 5,
        context: p.context,
    });
    if state.flags_144_to_150[0] {
        //82585DD8 descriptor initialization followed by827602E0 actual data request.
        owner.request_primary(Descriptor {
            kind: state.counter_152,
            id: state.counter_156,
        });
    } else {
        owner.request_interactable(p.frame, p.context);
    }
}
fn limits(margin: f32, a: f32, b: f32) -> BoardLimits {
    let radians = f32::from_bits(0x3c8efa35);
    BoardLimits {
        margin,
        angle_a: a * radians,
        angle_b: b * radians,
    }
}
