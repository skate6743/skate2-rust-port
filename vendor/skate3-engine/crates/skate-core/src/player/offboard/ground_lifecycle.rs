//! BipedGround Exit82D30CC0, PostPhysics82D32AA0 and FillPhysOut82D32D38.
//! Borrows canonical Ground state; source writes only the returned packet fields.
mod collision;
mod exit;
mod post;
mod publication;
#[cfg(test)]
mod tests;
use super::ground_entry::Vector;
pub use collision::{CollisionInput, CollisionSettings, check_collision};
pub use exit::{ExitServices, exit};
pub use post::{PostInput, post_physics};
pub use publication::{Publication, PublicationInput, publish};
fn dot(a: Vector, b: Vector) -> f32 {
    (a[0] * b[0] + a[1] * b[1]) + a[2] * b[2]
}
fn length(v: Vector) -> f32 {
    let q = dot(v, v);
    let mut r = crate::physics::reciprocal_sqrt::estimate(q);
    for _ in 0..2 {
        r = (r * 0.5).mul_add((-q).mul_add(r * r, 1.0), r);
    }
    if q == 0.0 { 0.0 } else { q * r }
}
