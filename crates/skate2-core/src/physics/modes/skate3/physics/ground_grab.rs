//! PhysicsGround::CheckForGrabbableObject (`0x82D39BB8`) and ConsiderGrabSplineQueryResults
//! (`0x82D39D98`): while the world-grab intent (ProcessedPhysIn+2476 bit 22) is held on board,
//! the SAME PlayerGrabSpline owner as the off-board grab queries every grabbable spline around
//! the centre of mass (type 0xFF: vehicles, then dynamic objects), and the first validated one
//! the skater reaches in time becomes the skitch candidate: Ground +2729/+2560/+2564/+2664/+2668,
//! the primary spline request and two frames without hand collision.
use super::Skate3ControllerState;
use skate3_core::player::offboard::{
    grab_scene::{self, Query, Record},
    ground_query::QueryContext,
    ground_sync::{BoardLimits, Bounds},
};
use skate_data::collections::Collections;

type Vector = [f32; 4];
type Frame = [Vector; 4];

/// `kInclineAngle` (`0x8206D110`), the degree-to-radian factor of both functions.
const DEG: f32 = f32::from_bits(0x3c8e_fa35);
/// `GrabbableObjectType` of a vehicle spline (`eGRABBABLE_VEHICLE`) and of a dynamic object's.
const VEHICLE: u32 = 1;
const DYNAMIC_OBJECT: u32 = 2;

/// The `physics_state_skitching` default collection's values the two functions read.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Settings {
    /// 0x8E3C1298E8072745 and 0x978B2A7533B6675D: the grab box's offset and half extents.
    offset: Vector,
    extents: Vector,
    /// 0xBF433C58D0F12390 and 0x424A1C69E0076DBB: the reach subtracted from the distance to the
    /// spline, and the time to it under which the spline is taken.
    entry_offset: f32,
    entry_time: f32,
}

impl Settings {
    pub(crate) fn load(data: &Collections) -> Result<Self, String> {
        let vector = |name: &str| -> Result<Vector, String> {
            Ok(data.words::<4>("physics_state_skitching", "default", name)?.map(f32::from_bits))
        };
        let float = |name: &str| data.float("physics_state_skitching", "default", name);
        Ok(Self {
            offset: vector("0x8E3C1298E8072745")?,
            extents: vector("0x978B2A7533B6675D")?,
            entry_offset: float("0xBF433C58D0F12390")?,
            entry_time: float("0x424A1C69E0076DBB")?,
        })
    }
}

fn sub(a: Vector, b: Vector) -> Vector {
    std::array::from_fn(|i| a[i] - b[i])
}
fn dot3(a: Vector, b: Vector) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn madd(a: Vector, s: f32, b: Vector) -> Vector {
    std::array::from_fn(|i| a[i].mul_add(s, b[i]))
}
fn cross(a: Vector, b: Vector) -> Vector {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0], 0.0]
}
fn raw(v: [u32; 4]) -> Vector {
    v.map(f32::from_bits)
}

/// `0x82D2E250` with argument7 true: the frame's own axes, unlike the off-board board box (the
/// right axis flattened and normalized with the X axis as its fallback, up the forward axis
/// crossed with it).
fn oriented_bounds(frame: Frame, offset: Vector, extents: Vector) -> Bounds {
    // 0x82D2E328..330: the stacked right axis takes a zero height before the normalize.
    let flat = [frame[0][0], 0.0, frame[0][2], frame[0][3]];
    let length = dot3(flat, flat).sqrt();
    let right = if length > 0.0 && length.is_finite() {
        flat.map(|v| v / length)
    } else {
        [1.0, 0.0, 0.0, 0.0]
    };
    let forward = frame[2];
    let up = cross(forward, right);
    let position = madd(up, offset[1], madd(forward, offset[2] + extents[2], frame[3]));
    Bounds { frame: [right, up, forward, position], extents }
}

fn transform_point(frame: Frame, p: Vector) -> Vector {
    let mut out = frame[3];
    for (axis, &value) in frame.iter().zip(&p).take(3) {
        out = madd(*axis, value, out);
    }
    out[3] = 1.0;
    out
}

/// The rigid inverse of the box frame applied to a point.
fn inverse_point(frame: Frame, p: Vector) -> Vector {
    let d = sub(p, frame[3]);
    [dot3(d, frame[0]), dot3(d, frame[1]), dot3(d, frame[2]), 1.0]
}

/// CheckForGrabbableObject, the tail of `PhysicsGround::Update` (`0x82D37F04`), or the
/// owner's InvalidateQuery (`0x82D37F10`) without the intent. Without the skitching
/// collection in the game data nothing is grabbed.
pub(crate) fn check(skater: &mut Skate3ControllerState) {
    let p = &skater.player_input.processed;
    let Some(settings) = skater.ground_grab.filter(|_| p.flags_2476 & 0x0040_0000 != 0) else {
        skater.offboard_grab.invalidate();
        return;
    };
    let settings = &settings;
    let frame: Frame = p.effective_anim_transform_192.map(raw);
    let centre_of_mass = raw(p.vectors_544_560_592_608[2]);
    let bounds = oriented_bounds(frame, settings.offset, settings.extents);
    if skater.offboard_grab.flags_12836 & 0x40 != 0 {
        consider(skater, settings, frame, bounds);
    }
    let p = &skater.player_input.processed;
    let context = QueryContext {
        selection_flags_2948: p.actor_query_2948,
        matching_id_2952: p.actor_query_2952 as i32,
    };
    // 0x82D39D8C: AddDeferredGrabSplineQuery(0xFF, 5, centre of mass, box, 0.25, 60, 30 degrees).
    skater.offboard_grab.query(Query {
        position: centre_of_mass,
        sort_position: centre_of_mass,
        bounds,
        limits: BoardLimits {
            margin: f32::from_bits(0x3e80_0000),
            angle_a: 60.0 * DEG,
            angle_b: 30.0 * DEG,
        },
        mode: 0xFF,
        capacity: 5,
        context,
    });
}

/// ConsiderGrabSplineQueryResults over the owner's validated results.
fn consider(skater: &mut Skate3ControllerState, settings: &Settings, frame: Frame, bounds: Bounds) {
    let state = &mut skater.ground.state;
    state.flag_2729 = false;
    state.word_2560 = 0;
    state.word_2564 = 0;
    state.scalar_2668 = 0.0;
    let p = &skater.player_input.processed;
    let centre_of_mass = raw(p.vectors_544_560_592_608[2]);
    let velocity = raw(p.vectors_400_416[0]);
    let deck_height = skater.player_input.toolkit.as_ref().map_or(frame[3][1], |toolkit| toolkit.deck[3][1]);
    let reskitch_delay = p.ground_timer_2848;
    let last_skitched = p.skitch_value_2592;
    // A dynamic object's spline takes a box three quarters as wide and as long, its back face
    // where the full box's is (0x82D39E68..EC4).
    let mut narrow = bounds;
    narrow.extents[0] *= f32::from_bits(0x3f40_0000);
    narrow.extents[2] *= f32::from_bits(0x3f40_0000);
    narrow.frame[3] = madd(bounds.frame[2], narrow.extents[2] - bounds.extents[2], bounds.frame[3]);
    let records: Vec<Record> = skater.offboard_grab.validated().to_vec();
    if super::skitching::trace_enabled() {
        eprintln!(
            "skate3 skitch consider: {} validated {:?}",
            records.len(),
            records.iter().map(|r| (r.descriptor().kind, r.descriptor().id)).collect::<Vec<_>>()
        );
    }
    for record in records {
        let descriptor = record.descriptor();
        let [mut p1, mut p2] = record.endpoints();
        if dot3(sub(p2, p1), bounds.frame[0]) < 0.0 {
            std::mem::swap(&mut p1, &mut p2);
        }
        let box_ = if descriptor.kind == DYNAMIC_OBJECT { narrow } else { bounds };
        // CanGrabSpline from the centre of mass: no end exclusion, kSkitchMaxEntryAngle 60 and
        // kSkitchMaxHorizontalEntryAngle 40 (their cvar defaults, constants here).
        let limits = BoardLimits { margin: 0.0, angle_a: 60.0 * DEG, angle_b: 40.0 * DEG };
        if !grab_scene::qualify(&record, centre_of_mass, box_, limits) {
            if super::skitching::trace_enabled() {
                eprintln!("skate3 skitch candidate: kind {} id {} does not qualify", descriptor.kind, descriptor.id);
            }
            continue;
        }
        // 0x82D3A04C..070: while the reskitch delay runs, only another vehicle may be taken.
        if !(reskitch_delay <= 0.0) && (descriptor.kind != VEHICLE || last_skitched == descriptor.id) {
            continue;
        }
        // Where the spline crosses the box's centre plane (box-local X = 0).
        let a = inverse_point(box_.frame, p1);
        let b = inverse_point(box_.frame, p2);
        let weight = a[0] / (a[0] - b[0]);
        let point = transform_point(box_.frame, madd(sub(b, a), weight, a));
        let state = &mut skater.ground.state;
        state.scalar_2668 = point[1] - deck_height;
        let distance = dot3(sub(point, frame[3]), frame[2]);
        let closing = dot3(sub(velocity, record.velocity()), frame[2]);
        // kSkitchMinApproachSpeed 0.5 (`0x82099194`).
        let time = (distance - settings.entry_offset) / closing.max(0.5);
        state.scalar_2664 = if time >= 0.0 { time } else { 0.0 };
        if super::skitching::trace_enabled() {
            eprintln!(
                "skate3 skitch candidate: kind {} id {} distance {distance} closing {closing} time {time} (entry {}) delay {reskitch_delay}",
                descriptor.kind, descriptor.id, settings.entry_time,
            );
        }
        skater.offboard_grab.request_primary(descriptor);
        if skater.ground.state.scalar_2664 < settings.entry_time {
            let state = &mut skater.ground.state;
            state.flag_2729 = true;
            state.word_2560 = descriptor.kind;
            state.word_2564 = descriptor.id;
            skater.offboard_grab.request_primary(descriptor);
            skater.skeleton_collision.disable_handplant_contacts(2);
            return;
        }
    }
}
