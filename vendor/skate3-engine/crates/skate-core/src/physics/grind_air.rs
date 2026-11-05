//! Original S3 82D71138/82D34FE8/82D712E0, with persistent native histories.
//! Independent PC arithmetic; not a bit-exact Xenon emulation claim.
mod angles;
mod pose;
mod prediction;
mod selection;
mod targets;
use super::skeleton_animation_record::AnimationPartTransform as Frame;
use super::{grind_contact::Primitive, native_arithmetic as arithmetic};
use crate::air::trajectory::grind_surface::LandingOrientation;
pub use pose::PoseInput;
pub use targets::{DeckDimensions, Settings};
pub type V = [f32; 4];
const ZERO: V = [0.; 4];
const UP: V = [0., 1., 0., 0.];
const DEG: f32 = f32::from_bits(0x3c8e_fa35);
const RAD: f32 = f32::from_bits(0x4265_2ee1);
const PI: f32 = f32::from_bits(0x4049_0fdb);
const TAU: f32 = f32::from_bits(0x40c9_0fdb);

#[derive(Clone, Copy, Debug)]
pub struct Target {
    pub edge: Primitive,
    pub primitive_flags: u32,
    pub orientation: LandingOrientation,
}
#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub active: bool,
    pub board: Frame,
    pub velocity: V,
    pub angular_velocity: V,
    pub up: V,
    pub timestep: f32,
    pub flags_2468: u32,
    pub flags_2472: u32,
    pub flags_2480: u32,
    pub flags_2484: u32,
}
#[derive(Clone, Copy, Debug)]
pub struct Adjustment {
    pub axis: V,
    pub offset: V,
    pub angles: V,
}
#[derive(Clone, Debug)]
pub struct GrindAir {
    pub target: Option<Target>,
    offset_delta: V,
    offset: V,
    angle_delta: V,
    angles: V,
    selected_kind: Option<usize>,
    // Native array is initialized once, not cleared on each prediction.
    headings: [f32; 12],
}
impl Default for GrindAir {
    fn default() -> Self {
        Self {
            target: None,
            offset_delta: ZERO,
            offset: ZERO,
            angle_delta: ZERO,
            angles: ZERO,
            selected_kind: None,
            headings: [0.; 12],
        }
    }
}
impl GrindAir {
    ///82D34FE8. Started/activated latches are owned by the existing KnownAir caller.
    pub fn start(&mut self, target: Target) {
        self.target = Some(target);
        self.offset_delta = ZERO;
        self.offset = ZERO;
        self.angle_delta = ZERO;
        self.angles = ZERO;
        self.selected_kind = None;
    }
    pub fn update(
        &mut self,
        input: Input,
        settings: &Settings,
    ) -> Result<Option<Adjustment>, String> {
        if !input.active {
            return Ok(None);
        }
        if input.flags_2480 & 0x0400_0000 != 0
            || input.flags_2472 & 0x8000 != 0
            || input.flags_2468 & 8 != 0
        {
            self.offset = ZERO;
            self.angles = ZERO;
            return Ok(None);
        }
        let target = self
            .target
            .ok_or("Active GrindAirAdjust has no selected primitive")?;
        settings.validate()?;
        let rail = unit(sub(target.edge.end, target.edge.start));
        let normal = unit(cross(rail, cross(input.up, rail)));
        let samples = prediction::project(input, settings, rail, normal, &mut self.headings);
        let contacts =
            selection::contacts(&samples, settings.frames, target.edge.start, rail, normal);
        let Some(kind) = selection::choose(
            &contacts,
            &self.headings,
            settings,
            input.flags_2484 & 0x0020_0000 != 0,
            self.selected_kind,
        ) else {
            return Ok(None);
        };
        self.selected_kind = Some(kind);
        let probe = targets::CONTACTS[kind];
        let time = contacts.times[probe];
        //82D72610, per-frame and total displacement are separately bounded.
        self.offset_delta = limited(
            add(
                self.offset_delta,
                scale(contacts.corrections[probe], reciprocal(time)),
            ),
            settings.max_delta,
        );
        self.offset = limited(add(self.offset, self.offset_delta), settings.max_offset);
        angles::update(
            self, input, settings, target, &samples, rail, normal, kind, time,
        );
        Ok(Some(Adjustment {
            axis: rail,
            offset: self.offset,
            angles: self.angles,
        }))
    }
}
fn dot(a: V, b: V) -> f32 {
    arithmetic::dot3(a, b)
}
fn sub(a: V, b: V) -> V {
    core::array::from_fn(|i| a[i] - b[i])
}
fn add(a: V, b: V) -> V {
    core::array::from_fn(|i| a[i] + b[i])
}
fn scale(v: V, s: f32) -> V {
    v.map(|x| x * s)
}
fn madd(v: V, s: f32, a: V) -> V {
    core::array::from_fn(|i| v[i].mul_add(s, a[i]))
}
fn cross(a: V, b: V) -> V {
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
        0.,
    ]
}
fn inverse_length(square: f32) -> f32 {
    let mut r = arithmetic::reciprocal_square_root_estimate(square);
    for _ in 0..2 {
        r = (r * 0.5).mul_add((-square).mul_add(r * r, 1.), r);
    }
    r
}
fn reciprocal(value: f32) -> f32 {
    let mut r = arithmetic::reciprocal_estimate(value);
    for _ in 0..2 {
        r = r.mul_add((-r).mul_add(value, 1.), r);
    }
    r
}
fn length(v: V) -> f32 {
    let s = dot(v, v);
    if s == 0. { 0. } else { s * inverse_length(s) }
}
fn unit(v: V) -> V {
    let s = dot(v, v);
    let r = inverse_length(s);
    let magnitude = if s == 0. { 0. } else { s * r };
    if magnitude > f32::from_bits(0x3586_37bd) {
        scale(v, r)
    } else {
        ZERO
    }
}
fn limited(v: V, maximum: f32) -> V {
    let magnitude = length(v);
    if magnitude > maximum {
        scale(v, maximum * reciprocal(magnitude))
    } else {
        v
    }
}
fn wrap(angle: f32) -> f32 {
    let turns = angle * f32::from_bits(0x3e22_f983);
    let fraction = turns - turns.floor();
    (fraction - if fraction > 0.5 { 1. } else { 0. }) * TAU
}
///8296EC98: one refinement, positive full-turn signed angle then8258DB98 wrap.
fn signed_angle(a: V, b: V, axis: V) -> f32 {
    let aa = dot(a, a);
    let bb = dot(b, b);
    if !(aa > f32::from_bits(0x38d1_b717) && bb > f32::from_bits(0x38d1_b717)) {
        return 0.;
    }
    let normalize = |v, square| {
        let r = arithmetic::reciprocal_square_root_estimate(square);
        scale(v, (r * 0.5).mul_add((-square).mul_add(r * r, 1.), r))
    };
    let a = normalize(a, aa);
    let b = normalize(b, bb);
    let angle = crate::trigonometry::acos(dot(a, b).max(-1.).min(1.));
    wrap(if dot(cross(a, b), axis) < 0. {
        TAU - angle
    } else {
        angle
    })
}
