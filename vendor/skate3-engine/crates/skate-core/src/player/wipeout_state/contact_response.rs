//! Wipeout82D3EC08's two persistent material responses, 82D91708..82D92440.
//! Each correction writes the actual 23 skater bodies' linear velocities.
mod material10;
mod material11;
use super::math::{V, dot, normalize_or, scale, sub};
use crate::physics::skeleton_body::SkeletonBody;

#[derive(Clone, Copy)]
pub struct ContactResponseInput {
    /// Original Processed608 snapshot, reused by both helpers before physics.
    pub com_velocity_608: V,
    /// SkeletonCollision4079/1168; absence preserves the retained normal.
    pub material10_normal: Option<V>,
    /// SkeletonCollision4080/1184; the helper owns its separate contact latch.
    pub material11_normal: Option<V>,
    /// Original Processed224 (effective animation transform's third axis).
    pub effective_axis_224: V,
    /// Original Processed2824/2828, the stock Wipeout control attributes.
    pub wipeout_control_2824_2828: [f32; 2],
}
#[derive(Clone, Copy, Debug)]
pub struct ContactResponseOutput {
    pub material10_phase: u32,
    /// One-update pulse at first helper2->3, copied to Wipeout572.
    pub material10_finished_77: bool,
    /// Second helper's retained byte73; selects caller's collision mode9.
    pub material11_active_73: bool,
}
pub struct ContactResponse {
    material10: material10::Response,
    material11: material11::Response,
}
impl ContactResponse {
    pub fn new() -> Self {
        Self {
            material10: material10::Response::new(),
            material11: material11::Response::new(),
        }
    }
    /// Wipeout Reset82D3B3A8 invokes both resets; byte73 survives the second.
    pub fn reset(&mut self) {
        self.material10.reset();
        self.material11.reset();
    }
    pub fn update(
        &mut self,
        body: &mut SkeletonBody,
        input: ContactResponseInput,
    ) -> ContactResponseOutput {
        //82D3EC58..5C: state selection precedes the first response's update.
        self.material10
            .update(body, input.com_velocity_608, input.material10_normal);
        self.material11.update(body, input);
        ContactResponseOutput {
            material10_phase: self.material10.phase,
            material10_finished_77: self.material10.finished,
            material11_active_73: self.material11.active,
        }
    }
}

/// 82E0A4E8: reject the component along the safely normalized direction.
fn reject(value: V, direction: V) -> V {
    let normal = normalize_or(direction, [0.0; 4]);
    sub(value, scale(normal, dot(value, normal)))
}
/// 82C1E170 differs: positive component test uses the normalized direction,
/// but its subtraction multiplies the ORIGINAL direction, even if nonunit.
fn reject_positive(value: V, direction: V) -> V {
    let projection = dot(value, normalize_or(direction, [0.0; 4]));
    if projection > 0.0 {
        sub(value, scale(direction, projection))
    } else {
        value
    }
}
fn apply_velocity_delta(body: &mut SkeletonBody, delta: V) {
    //All original loops start at part1 and execute23 stores; root0/extras24/25
    //and angular velocities remain untouched. The host stores XYZ explicitly.
    for part in &mut body.bodies_mut()[1..24] {
        part.rates.linear_velocity.x += delta[0];
        part.rates.linear_velocity.y += delta[1];
        part.rates.linear_velocity.z += delta[2];
    }
}
