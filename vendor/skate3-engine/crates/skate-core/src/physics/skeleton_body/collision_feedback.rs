//! SkeletonCollision observations from original TU3 82BD4A30/82BD5DA0.
//! Reports must come from the completed solver, in native report order.
use super::{SkeletonCollisionSettings, SkeletonPhysicalRecord};
use crate::physics::skeleton_animation_record::AnimationPartTransform;

pub(super) type V = [f32; 4];

#[derive(Clone, Copy, Debug)]
pub struct SkeletonFeedbackSettings {
    pub body: SkeletonCollisionSettings,
    /// Fixed physics_collision layout120/124/128/164 consumed by82BD60C8.
    pub response: CollisionResponseSettings,
    pub small_object_mass: f32,
    pub ground_plane_max_distance: f32,
    pub ground_plane_max_angle: f32,
    pub skater_scalar: f32,
    pub ai_scalar: f32,
    pub groin_offset: V,
    pub face_offset: V,
    pub groin_radius: f32,
    pub face_radius: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct SkeletonContactBody {
    pub state_flags: u32,
    pub inverse_mass: f32,
    pub linear_velocity: V,
}

/// The useful fields of original 96-byte ContactInfo and its solved spy.
/// `normal` already has the report-side sign; `solved_vector` is spy+80.
#[derive(Clone, Copy, Debug)]
pub struct SkeletonContactReport {
    pub part: usize,
    pub normal: V,
    pub point: V,
    pub tag: u32,
    pub other_group: u32,
    pub other_entity: Option<i32>,
    pub body_a: SkeletonContactBody,
    pub body_b: SkeletonContactBody,
    pub side_a: bool,
    pub solved_vector: V,
}

/// Original UpdatePostPhysics82BD80D8 builds these values before Update.
pub struct SkeletonCollisionInput<'a> {
    pub dt: f32,
    pub plane_point: V,
    pub plane_normal: V,
    pub reference_velocity: V,
    pub com_velocity: V,
    pub ragdoll: bool,
    /// Processed2472 bit0x8000, which suppresses normal ground filtering.
    pub disable_ground_filter: bool,
    /// Processed2472 bit0x10000000 selects AISkeletonScalar.
    pub ai_collision_scalar: bool,
    /// Category500, excluding states501 and503.
    pub offboard: bool,
    pub entering_offboard: bool,
    pub category_600: bool,
    pub request_partial_ragdoll: bool,
    pub physical: &'a SkeletonPhysicalRecord,
    pub part_weights: &'a [f32; 24],
    /// Actual GetPartTransform frames used by CheckConflictingContacts.
    pub body_frames: &'a [AnimationPartTransform; 26],
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BoneContact {
    pub normal: V,
    pub specific_normal: V,
    pub tangent: V,
    pub specific_tangent: V,
    pub point: V,
    pub force: f32,
    pub specific_force: f32,
    pub tag: u32,
    pub specific_tag: u32,
    pub groups: [bool; 4],
}
#[derive(Clone, Copy, Debug)]
pub struct ContactRegion {
    pub force: f32,
    pub weighted_force: f32,
    pub tangent_speed: f32,
    pub material_flags: u32,
    pub normal: V,
    pub part: Option<usize>,
}
impl Default for ContactRegion {
    fn default() -> Self {
        Self {
            force: 0.0,
            weighted_force: 0.0,
            tangent_speed: 0.0,
            material_flags: 0,
            normal: [0.0; 4],
            part: None,
        }
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct ContactPlane {
    pub normal: V,
    pub part: usize,
}
#[derive(Clone, Copy, Debug)]
pub struct SpecificContact {
    pub part: usize,
    pub local_point: V,
    pub world_point: V,
    pub radius_squared: f32,
    pub current: bool,
    pub recent: bool,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct SkeletonContactFlags {
    pub material_6: bool,
    pub noncompliant: bool,
    /// Native4070.
    pub compliant: bool,
    pub recovering: bool,
    pub group_8: bool,
    pub nonboard: bool,
    pub ragdoll: bool,
    pub conflicting: bool,
    pub impaled: bool,
    /// Native4077. This does not itself contain the pose-error vector16272.
    pub has_impulse: bool,
    pub foot_board: bool,
    pub material_10: bool,
    pub material_11: bool,
    pub material_12: bool,
    pub any: bool,
}

pub struct SkeletonCollisionFeedback {
    pub settings: SkeletonFeedbackSettings,
    pub contact_age: [f32; 24],
    pub priority: [f32; 24],
    pub compliant: [bool; 24],
    pub current: [bool; 24],
    pub bones: [BoneContact; 24],
    pub regions: [ContactRegion; 8],
    pub planes: Vec<ContactPlane>,
    pub specific: [SpecificContact; 2],
    pub highest_normal: V,
    pub foot_normal: V,
    pub material_normals: [V; 2],
    pub timer: f32,
    /// Native4028. Reset is zero; Update recovers toward one.
    pub drive_weight: f32,
    pub maximum_priority: f32,
    pub wipeout_times: [f32; 3],
    pub maximum_skater_force: f32,
    pub other_skater: i32,
    pub maximum_group_8_force: f32,
    pub maximum_group_11_force: f32,
    pub material_12_height: f32,
    pub flags: SkeletonContactFlags,
}

impl SkeletonCollisionFeedback {
    /// Constructor82BD5F48/6038 plus the final Body constructor Reset. Static
    /// compliant settings survive Reset, while the priority array is cleared.
    pub fn new(settings: SkeletonFeedbackSettings) -> Self {
        let specific = [
            (23, settings.groin_offset, settings.groin_radius),
            (1, settings.face_offset, settings.face_radius),
        ]
        .map(|(part, local_point, radius)| SpecificContact {
            part,
            local_point,
            world_point: [0.0; 4],
            radius_squared: radius * radius,
            current: false,
            recent: false,
        });
        Self {
            settings,
            contact_age: [0.0; 24],
            priority: [0.0; 24],
            compliant: settings.body.compliant,
            current: [false; 24],
            bones: [BoneContact::default(); 24],
            regions: [ContactRegion::default(); 8],
            planes: Vec::with_capacity(20),
            specific,
            highest_normal: [0.0, 1.0, 0.0, 0.0],
            foot_normal: [0.0, 1.0, 0.0, 0.0],
            material_normals: [[0.0; 4]; 2],
            timer: 0.0,
            drive_weight: 0.0,
            maximum_priority: 0.0,
            wipeout_times: [0.0; 3],
            maximum_skater_force: 0.0,
            other_skater: -1,
            maximum_group_8_force: 0.0,
            maximum_group_11_force: 0.0,
            material_12_height: 0.0,
            flags: SkeletonContactFlags::default(),
        }
    }

    ///82BD5DA0 preserves compliant settings, specific-point histories and4082.
    pub fn reset(&mut self) {
        let specific = self.specific;
        let compliant = self.compliant;
        let any = self.flags.any;
        *self = Self::new(self.settings);
        self.specific = specific;
        self.compliant = compliant;
        self.flags.any = any;
    }

    /// SetUpNormal82BE7280 calls Reset before reinstalling authored priority
    /// and compliant settings. A subsequent constructor Reset clears priority.
    pub fn set_up_normal(&mut self) {
        self.reset();
        self.priority = self.settings.body.priority;
        self.compliant = self.settings.body.compliant;
    }
}

/// Fixed-layout physics_collision tuning for SkeletonCollision::Fill82BD60C8.
#[derive(Clone, Copy, Debug)]
pub struct CollisionResponseSettings {
    pub force_scale: f32,
    pub velocity_scale: f32,
    pub divisor: f32,
    pub region_scale: f32,
}

impl SkeletonCollisionFeedback {
    /// Fill82BD60C8 accumulates these responses after contact traversal, then
    /// publishes them to Collision196/200 and the eight normalized region words.
    pub fn fill_response(
        &mut self,
        physical: &SkeletonPhysicalRecord,
        weights: &[f32; 24],
    ) -> [f32; 8] {
        let velocities: &[[f32; 4]; 24] = physical.velocities[..24]
            .try_into()
            .expect("native24 velocities");
        collision_response(
            self.settings.response,
            &mut self.wipeout_times,
            &self.regions,
            velocities,
            weights,
        )
    }
}

/// The arithmetic block of82BD60C8. The fourth vector lane is not part of either dot.
pub fn collision_response(
    t: CollisionResponseSettings,
    accumulated: &mut [f32; 3],
    regions: &[ContactRegion; 8],
    velocities: &[[f32; 4]; 24],
    weights: &[f32; 24],
) -> [f32; 8] {
    // vmsum3fp128 pairs guest x/y, then adds guest z/zero. Each multiplication
    // rounds before addition; fmadds below is independently fused.
    let dot = |a: V, b: V| (a[0] * b[0] + a[1] * b[1]) + a[2] * b[2];
    let positive = |v: f32| if -v >= 0.0 { 0.0 } else { v };
    let clamp = |v: f32| {
        let v = positive(v);
        if 1.0 - v >= 0.0 { v } else { 1.0 }
    };
    let output = std::array::from_fn(|i| {
        let r = &regions[i];
        let Some(part) = r.part else { return 0.0 };
        let v = velocities[part];
        let speed = if dot(v, v) <= f32::from_bits(0x3dcc_cccd) {
            0.0
        } else {
            1.0
        };
        let force = if r.force <= 0.5 { 0.0 } else { 1.0 };
        let inverse = 1.0 / t.divisor;
        let normal_speed = dot(v, r.normal).abs();
        accumulated[1] = ((inverse * t.force_scale) * force).mul_add(r.force, accumulated[1]);
        accumulated[0] =
            ((inverse * t.velocity_scale) * normal_speed).mul_add(speed, accumulated[0]);
        let value = (normal_speed * weights[part]) * t.region_scale;
        let floor = f32::from_bits(0x3a83_126f);
        clamp(if floor - value >= 0.0 { floor } else { value })
    });
    accumulated[0] = clamp(accumulated[0]);
    accumulated[1] = clamp(accumulated[1]);
    output
}
