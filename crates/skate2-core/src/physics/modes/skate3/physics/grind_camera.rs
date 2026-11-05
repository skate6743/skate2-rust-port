//! ZIP contact conditioner82DF0640, corroborated against S2 82E2D000.
//! Tangent selection82D87460 belongs to the manager, not to this conditioner.
use crate::skate3_native::camera::CameraGrindOutput;
type V = [f32; 4];

#[derive(Clone, Copy, Debug)]
pub(crate) struct Contact {
    pub point: V,
    pub direction: V,
    /// Exact processed primitive endpoints, not the board's two truck hits.
    pub endpoints: [V; 2],
    pub family: u32,
}

#[derive(Default)]
pub(crate) struct GrindCamera {
    current: V,
    previous: V,
    error: V,
    midpoint: V,
    family: u32,
    active: bool,
}

impl GrindCamera {
    /// Schedule once after physical FillOut, before camera/graph readers. Uses
    /// retained physical direction, including tipslide's latched drop-in frame,
    /// rather than rereading a reset or newly selected investigation family.
    pub fn condition_fields(
        &mut self,
        out: &mut skate3_core::player::input_phase::GrindOutputFields,
    ) {
        let vector = |v: [u32; 4]| v.map(f32::from_bits);
        let contact = (out.grinding_316 != 0).then(|| Contact {
            point: vector(out.point_16),
            direction: vector(out.direction_0),
            endpoints: [vector(out.primitive_start_64), vector(out.primitive_end_80)],
            family: out.words_136_140[0],
        });
        let mut camera = CameraGrindOutput {
            direction_0: vector(out.direction_0),
            camera_target_96: vector(out.camera_target_96),
            grinding_316: out.grinding_316,
        };
        self.update(contact, &mut camera);
        out.direction_0 = camera.direction_0.map(f32::to_bits);
        out.camera_target_96 = camera.camera_target_96.map(f32::to_bits);
    }

    /// None invalidates history but leaves the caller's retained output alone.
    /// It must not replace inactive direction/target fields with zero.
    pub fn update(&mut self, contact: Option<Contact>, output: &mut CameraGrindOutput) {
        let Some(contact) = contact else {
            self.active = false;
            return;
        };
        let midpoint: V =
            core::array::from_fn(|i| (contact.endpoints[0][i] + contact.endpoints[1][i]) * 0.5);
        if !self.active {
            self.error = [0.0; 4];
            self.current = contact.point;
            self.previous = contact.point;
        } else if contact.family != self.family || midpoint[..3] != self.midpoint[..3] {
            self.error = core::array::from_fn(|i| {
                self.current[i] + (self.current[i] - self.previous[i]) - contact.point[i]
            });
        }
        // S3 constructor82DF2514:0.95 per physical update, not dt-based smoothing.
        self.error = self.error.map(|v| v * 0.95);
        self.previous = self.current;
        self.current = core::array::from_fn(|i| contact.point[i] + self.error[i]);
        self.family = contact.family;
        self.midpoint = midpoint;
        self.active = true;
        output.direction_0 = contact.direction;
        output.camera_target_96 = self.current;
        // grinding_316 is produced by the physical selector; never inferred here.
    }
}

#[cfg(any())]
#[path = "grind_camera/tests.rs"]
mod tests;
