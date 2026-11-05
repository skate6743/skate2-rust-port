//! Solver observations consumed by the board's post-physics update.
//!
//! TU3 82AE1608 observes completed contact rows after integration; 827682B0
//! then constructs at most sixteen reports per board, in contact order.
//! Rust owns the report storage and part identifiers. No guest pointers or
//! copied native map/locking infrastructure are needed for this single board.
use crate::math::Vector3;

use super::super::{
    assembly::BodySnapshot,
    board::{BODY_COUNT, BodyId},
    board_step::CollisionBody,
    contact_solver::RetailContactJacobian,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoardContactReport {
    pub part: BodyId,
    pub other: CollisionBody,
    pub is_body_a: bool,
    /// Oriented toward this board, as ContactInfo+16.
    pub normal: Vector3,
    pub position: Vector3,
    /// Linear velocities after integration. Native does not add omega cross r.
    pub relative_linear_velocity: Vector3,
    pub other_surface: u16,
    /// The underlying spy retains A's orientation even in a B-side report.
    pub normal_force_on_a: Vector3,
    pub friction_force_on_a: Vector3,
    pub tangents: [Vector3; 2],
}

/// Replace this frame's reports from the actual solved rows. All seven board
/// parts share one report budget and one owner; their self contacts are excluded
/// from reporting, independently of whether they participated in the solver.
pub(crate) fn collect(
    output: &mut Vec<BoardContactReport>,
    contacts: &[RetailContactJacobian],
    bodies: &[BodySnapshot; BODY_COUNT],
    attached: &[&BodySnapshot],
    owned_attached: usize,
    frequency: f32,
) {
    output.clear();
    let frequency_squared = frequency * frequency;
    for contact in contacts {
        let words = contact.words();
        let impulse = contact.accumulated_impulse();
        // 82AE1668..168C: enabled spy and strictly positive normal response.
        if words[11] & 8 == 0 || !(impulse[0] > 0.0) {
            continue;
        }
        let a = CollisionBody::from_contact_id(words[31]);
        let b = CollisionBody::from_contact_id(words[43]);
        let (part, other, is_body_a) = match (a, b) {
            (CollisionBody::Board(part), CollisionBody::StaticWorld) => (part, b, true),
            (CollisionBody::StaticWorld, CollisionBody::Board(part)) => (part, a, false),
            // Dynamic props/vehicles share the solve; board ground consumes them
            // through the recovered attached-body owner (827682B0 routing).
            // 82768418 skips a pair whose two Bodies share an owner: the board never reports
            // its own skater's skeleton.
            (CollisionBody::Board(part), CollisionBody::Attached(index)) if index >= owned_attached => {
                (part, b, true)
            }
            (CollisionBody::Attached(index), CollisionBody::Board(part)) if index >= owned_attached => {
                (part, a, false)
            }
            _ => continue,
        };
        if output.len() == 16 {
            break;
        }
        let normal = vector(words, 28);
        let tangents = [vector(words, 40), vector(words, 52)];
        let a_position = add(center(a, bodies, attached), vector(words, 0));
        let b_position = add(center(b, bodies, attached), vector(words, 4));
        let a_weight = f32::from_bits(words[35]);
        let b_weight = f32::from_bits(words[39]);
        let inverse_weight = 1.0 / (b_weight + a_weight);
        let position = combine(a_position, a_weight, scale(b_position, b_weight));
        let friction = combine(tangents[0], impulse[1], scale(tangents[1], impulse[2]));
        let sign = if is_body_a { 1.0 } else { -1.0 };
        let this = CollisionBody::Board(part);
        output.push(BoardContactReport {
            part,
            other,
            is_body_a,
            normal: scale(normal, sign),
            position: scale(position, inverse_weight),
            relative_linear_velocity: subtract(
                velocity(this, bodies, attached),
                velocity(other, bodies, attached),
            ),
            other_surface: if is_body_a {
                words[55] as u16
            } else {
                (words[55] >> 16) as u16
            },
            normal_force_on_a: scale(scale(normal, impulse[0]), frequency_squared),
            friction_force_on_a: scale(friction, frequency_squared),
            tangents,
        });
    }
}

fn center(
    body: CollisionBody,
    bodies: &[BodySnapshot; BODY_COUNT],
    attached: &[&BodySnapshot],
) -> Vector3 {
    match body {
        CollisionBody::Board(id) => bodies[id.index()].rates.position,
        CollisionBody::StaticWorld => Vector3::ZERO,
        CollisionBody::Attached(index) => attached
            .get(index)
            .map(|body| body.rates.position)
            .unwrap_or(Vector3::ZERO),
    }
}

fn velocity(
    body: CollisionBody,
    bodies: &[BodySnapshot; BODY_COUNT],
    attached: &[&BodySnapshot],
) -> Vector3 {
    match body {
        CollisionBody::Board(id) => bodies[id.index()].rates.linear_velocity,
        CollisionBody::StaticWorld => Vector3::ZERO,
        CollisionBody::Attached(index) => attached
            .get(index)
            .map(|body| body.rates.linear_velocity)
            .unwrap_or(Vector3::ZERO),
    }
}

fn vector(words: &[u32; 64], offset: usize) -> Vector3 {
    Vector3::new(
        f32::from_bits(words[offset]),
        f32::from_bits(words[offset + 1]),
        f32::from_bits(words[offset + 2]),
    )
}
fn scale(v: Vector3, scale: f32) -> Vector3 {
    Vector3::new(v.x * scale, v.y * scale, v.z * scale)
}
fn add(a: Vector3, b: Vector3) -> Vector3 {
    Vector3::new(a.x + b.x, a.y + b.y, a.z + b.z)
}
fn subtract(a: Vector3, b: Vector3) -> Vector3 {
    Vector3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}
fn combine(a: Vector3, weight: f32, b: Vector3) -> Vector3 {
    Vector3::new(
        a.x.mul_add(weight, b.x),
        a.y.mul_add(weight, b.y),
        a.z.mul_add(weight, b.z),
    )
}
