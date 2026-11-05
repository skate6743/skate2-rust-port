//! Board contact normals and wheel-line state used by riding output.
//!
//! TU3 CollisionInfo constructor82C00D68/reset82C00CA0, wheel result
//! publication82C079E0, and world-contact normal selection82C07D20. The
//! postphysics stage consumes actual solver reports, never speculative query
//! contacts. Dynamic-object classification is outside the host's static world.
use super::{
    board::{BODY_COUNT, BodyId},
    board_motion_output::{add, dot, inverse_length_squared, length, scale, subtract},
    board_runtime::BoardRuntime,
    board_step::CollisionBody,
    contact_feedback::BoardContactReport,
    drive_frames::RetailAffineTransform,
    native_arithmetic,
};
use crate::{math::Vector3, trigonometry};

const UP: Vector3 = Vector3::new(0.0, 1.0, 0.0);

/// 82C07F74..8018: the contact point's offset from the deck, flattened onto the deck plane and
/// normalized (zero below the 830BD350 threshold), lies within acos(0.92) of the deck's length.
fn end_on(deck: RetailAffineTransform, point: Vector3) -> bool {
    let column = |i: usize| {
        let c = deck.basis.columns[i];
        Vector3::new(c[0], c[1], c[2])
    };
    let (up, forward) = (column(1), column(2));
    let offset = subtract(point, deck.translation);
    let flat = subtract(offset, scale(up, dot(up, offset)));
    let squared = dot(flat, flat);
    let inverse = inverse_length_squared(squared, 2);
    let length = if squared == 0.0 { 0.0 } else { squared * inverse };
    let direction = if length > f32::from_bits(0x3586_37bd) {
        scale(flat, inverse)
    } else {
        Vector3::ZERO
    };
    dot(direction, forward).abs() > f32::from_bits(0x3f6b_851f)
}
pub const WHEEL_LINE_LENGTH: f32 = f32::from_bits(0x3E4C_CCCD);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WheelLine {
    pub start: Vector3,
    pub end: Vector3,
}

/// The four zero-radius queries in82C07788. Up is Reckoning+1152, supplied
/// by PhysicalPlayerHiLOD::StartSkateboardLineTests82DB6310.
pub fn wheel_lines(board: &BoardRuntime, reckoning_up: Vector3) -> [WheelLine; 4] {
    let poses = board.part_transforms();
    let delta = scale(reckoning_up, WHEEL_LINE_LENGTH);
    core::array::from_fn(|i| WheelLine {
        start: poses[i].translation,
        end: subtract(poses[i].translation, delta),
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WheelLineHit {
    /// Query result+96, before82C079E0 multiplies by the authored line length.
    pub fraction: f32,
    pub normal: Vector3,
    pub surface_tag: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WheelLineState {
    pub normals: [Vector3; 4],
    pub distances: [f32; 4],
    pub physics_surfaces: [u32; 4],
    /// Audio material (surface tag bits 0..6, `& 0x7F`) per wheel, 0 on a miss. Not
    /// part of 82C079E0; kept for the engine's rolling sound.
    pub audio_surfaces: [u32; 4],
    /// Seam pattern (surface tag bits 12..15, `>> 12 & 0xF`) per wheel, 0 on a miss: the
    /// audio state's `+636..+648` (82C079E0 stores it next to the material). Engine-side input
    /// for the rolling bed's seam-pattern gain envelope and `Class_Seams`.
    pub seam_patterns: [u32; 4],
    pub minimum_distance: f32,
}
impl Default for WheelLineState {
    fn default() -> Self {
        //82C00D68 initializes all four normal/distance/surface records.
        Self {
            normals: [UP; 4],
            distances: [0.0; 4],
            physics_surfaces: [0; 4],
            audio_surfaces: [0; 4],
            seam_patterns: [0; 4],
            minimum_distance: 0.0,
        }
    }
}
impl WheelLineState {
    /// Complete four-wheel branch82C079F8..82C07A9C. A miss clears the
    /// surface, while the previous distance and normal deliberately survive.
    pub fn publish(&mut self, hits: [Option<WheelLineHit>; 4]) {
        self.minimum_distance = WHEEL_LINE_LENGTH;
        for (i, hit) in hits.into_iter().enumerate() {
            self.physics_surfaces[i] = 0;
            self.audio_surfaces[i] = 0;
            self.seam_patterns[i] = 0;
            if let Some(hit) = hit {
                let distance = hit.fraction * WHEEL_LINE_LENGTH;
                self.minimum_distance = if distance - self.minimum_distance >= -0.0 {
                    self.minimum_distance
                } else {
                    distance
                };
                self.normals[i] = hit.normal;
                self.distances[i] = distance;
                self.physics_surfaces[i] = (hit.surface_tag >> 7) & 31;
                self.audio_surfaces[i] = hit.surface_tag & 0x7F;
                self.seam_patterns[i] = (hit.surface_tag >> 12) & 0xF;
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PartGroundContact {
    pub in_contact: bool,
    pub normal: Vector3,
    pub point: Vector3,
    pub relative_velocity: Vector3,
}
const EMPTY_PART: PartGroundContact = PartGroundContact {
    in_contact: false,
    normal: UP,
    point: Vector3::ZERO,
    relative_velocity: Vector3::ZERO,
};

#[derive(Clone, Debug, PartialEq)]
pub struct BoardGroundState {
    pub parts: [PartGroundContact; BODY_COUNT],
    /// BoardBody320..416, retained between postphysics updates.
    pub previous_velocities: [Vector3; BODY_COUNT],
    /// BoardBody432..528, observed accelerations in actual part order.
    pub accelerations: [Vector3; BODY_COUNT],
    ///Body656/852: strongest closing velocity against this frame's contacts,
    ///computed from retained part velocities before acceleration sampling.
    pub closing_velocity: Vector3,
    pub maximum_closing_speed: f32,
    ///Body860: range of DECK contact normal projections onto Reckoning1152.
    pub opposing_contact: f32,
    ///Body864/872: static-world contact classifications and surface12 height.
    ///Other-assembly group flags require actual dynamic-object reports.
    pub surface_twelve_height: f32,
    pub collision_flags: u32,
    /// CollisionInfo+0. Can use truck/deck normals when the wheel sum fails.
    pub overall_normal: Vector3,
    /// CollisionInfo+16. Retained until a valid contacting-wheel sum replaces it.
    /// FillPhysOut publishes this to Ground+80 and Ground+96.
    pub wheel_normal: Vector3,
    pub valid_wheel_normals: [bool; 4],
    pub part_contact_count: u8,
    pub wheel_contact_count: u8,
    ///Body7692, accumulated only while all four physical wheels lack contact.
    pub time_without_wheel_contact: f32,
    /// Angular drag: S3 postphysics82C08634 writes wheel inertia+36;
    /// paired S2 postphysics82B372E0 names this mAngularDrag.
    pub wheel_angular_drag: [f32; 4],
    /// CollisionInfo+4/+8/+12: the trucks' and deck's last contact audio surface (tag & 0x7F,
    /// 0 without contact this frame), read by the audio state as materials +652..+660.
    pub part_audio_surfaces: [u32; 3],
}
impl Default for BoardGroundState {
    fn default() -> Self {
        Self {
            parts: [EMPTY_PART; BODY_COUNT],
            overall_normal: UP,
            wheel_normal: UP,
            // CollisionInfo ctor82C00DC8/CC zeros both arrays. Board reset
            //82C0D758 calls that constructor again; per-frame reset retains velocities.
            previous_velocities: [Vector3::ZERO; BODY_COUNT],
            accelerations: [Vector3::ZERO; BODY_COUNT],
            closing_velocity: Vector3::ZERO,
            maximum_closing_speed: 0.0,
            opposing_contact: 0.0,
            surface_twelve_height: 0.0,
            collision_flags: 0,
            valid_wheel_normals: [false; 4],
            part_contact_count: 0,
            wheel_contact_count: 0,
            time_without_wheel_contact: 0.0,
            wheel_angular_drag: [0.0; 4],
            part_audio_surfaces: [0; 3],
        }
    }
}
impl BoardGroundState {
    /// Original PostPhysics82C082B8..8444. The scalar reciprocal timestep is
    /// shared by all seven parts, then each previous velocity is replaced.
    pub fn sample_accelerations(&mut self, velocities: [Vector3; BODY_COUNT], time_step: f32) {
        let inverse_dt = 1.0 / time_step;
        for (i, current) in velocities.into_iter().enumerate() {
            self.accelerations[i] =
                scale(subtract(current, self.previous_velocities[i]), inverse_dt);
            self.previous_velocities[i] = current;
        }
    }

    ///82C087D8..87F4 after contact publication. The incoming f1 is the
    ///actual physics timestep; reset82C0D680 deliberately preserves this field.
    pub fn advance_contact_time(&mut self, time_step: f32) {
        self.time_without_wheel_contact = if self.wheel_contact_count == 0 {
            self.time_without_wheel_contact + time_step
        } else {
            0.0
        };
    }
    /// Complete contact-normal/count/drag subpipeline of82C07D20 for the
    /// static world. Reports must already satisfy82AE1608/827682B0 eligibility.
    /// `maximum_ground_angle_degrees` is physics_reckoning.
    /// MaxAllowedGroundNormalFromUp (full64 0E508708CC7F5249).
    pub fn update(
        &mut self,
        reports: &[BoardContactReport],
        lines: &WheelLineState,
        reckoning_up: Vector3,
        maximum_ground_angle_degrees: f32,
        board_wiping_out: bool,
    ) {
        self.update_with_groups(
            reports,
            lines,
            reckoning_up,
            maximum_ground_angle_degrees,
            board_wiping_out,
            RetailAffineTransform::IDENTITY,
            |_| None,
        );
    }

    /// [`Self::update`] with the other assembly of each attached-body report: its collision
    /// group (other Body+24 -> +24, 82C07F48) classifies the contact (82C07F54..80CC), and the
    /// deck's live body transform places it (82C07F68).
    #[allow(clippy::too_many_arguments)]
    pub fn update_with_groups(
        &mut self,
        reports: &[BoardContactReport],
        lines: &WheelLineState,
        reckoning_up: Vector3,
        maximum_ground_angle_degrees: f32,
        board_wiping_out: bool,
        deck: RetailAffineTransform,
        group: impl Fn(usize) -> Option<u32>,
    ) {
        self.parts.fill(EMPTY_PART);
        //CollisionInfo Reset82C00CA0 clears per-frame impacts and high flags.
        self.closing_velocity = Vector3::ZERO;
        self.maximum_closing_speed = 0.0;
        self.surface_twelve_height = 0.0;
        self.collision_flags &= 0x01ff_ffff;
        self.overall_normal = UP;
        self.valid_wheel_normals.fill(true);
        self.part_audio_surfaces = [0; 3];
        let mut highest_y = -2.0;
        let mut highest_part = None;
        //82C07E24..30 seeds min/max; only deck reports enter82C07EAC..CC.
        let mut minimum_projection = 1.0;
        let mut maximum_projection = -1.0;
        let mut surfaces = [0; BODY_COUNT];
        surfaces[..4].copy_from_slice(&lines.physics_surfaces);
        for report in reports {
            let i = report.part.index();
            if report.part == BodyId::Deck {
                let projection = dot(report.normal, reckoning_up);
                minimum_projection = if minimum_projection - projection >= 0.0 {
                    projection
                } else {
                    minimum_projection
                };
                maximum_projection = if maximum_projection - projection >= 0.0 {
                    maximum_projection
                } else {
                    projection
                };
            }
            //82C07ED0..7F2C: this is the OLD part velocity, not the report's
            //post-solve relative velocity or the finite-difference acceleration.
            let closing = -dot(report.normal, self.previous_velocities[i]);
            if !(closing <= self.maximum_closing_speed) {
                self.maximum_closing_speed = closing;
                self.closing_velocity = scale(report.normal, -closing);
            }
            if let CollisionBody::Attached(index) = report.other
                && let Some(group) = group(index)
            {
                match group {
                    // Another skater's board, met along this deck's length.
                    4 if end_on(deck, report.position) => self.collision_flags |= 1 << 30,
                    // Light dynamic objects: their normal never supports a wheel.
                    13 | 14 => {
                        self.collision_flags |= 1 << 29;
                        if i < 4 {
                            self.valid_wheel_normals[i] = false;
                        }
                    }
                    // 82C08044..80A0 also sets bit 26 for an owner flag (SkateboardBody+0x20C0
                    // bit 6) that no port consumer reads; bit 28 is the contact itself.
                    12 => self.collision_flags |= 1 << 28,
                    11 => self.collision_flags |= 1 << 27,
                    _ => {}
                }
            }
            let surface = (u32::from(report.other_surface) >> 7) & 31;
            if surface == 12 {
                self.collision_flags |= 1 << 25; //82C080D4..80F0.
                self.surface_twelve_height = report.position.y;
            }
            if i >= 4 {
                surfaces[i] = surface; //82C081B4..81E0, last report wins.
                self.part_audio_surfaces[i - 4] = u32::from(report.other_surface) & 0x7F;
            }
            let contact = &mut self.parts[i];
            if !contact.in_contact || report.normal.y > contact.normal.y {
                contact.normal = report.normal;
                contact.point = report.position;
                contact.relative_velocity = report.relative_linear_velocity;
            }
            // Selection occurs after the per-part maximum, in report order.
            if contact.normal.y > highest_y {
                highest_y = contact.normal.y;
                highest_part = Some(i);
            }
            contact.in_contact = true;
        }
        //82C082AC/E8/F4/F8; no deck reports produce max(-2,0)=0.
        let range = maximum_projection - minimum_projection;
        self.opposing_contact = if -range >= 0.0 { 0.0 } else { range };
        //82C0834C..836C combines physical contact with the wheel query's
        //surface (wheels), or the last actual contact tag (trucks/deck).
        for (part, surface) in self.parts.iter().zip(surfaces) {
            if part.in_contact && surface == 8 {
                self.collision_flags |= 1 << 31;
            }
        }
        // The source's -1 best-part index addresses the retained wheel normal.
        let reference_normal = highest_part.map_or(self.wheel_normal, |i| self.parts[i].normal);
        for i in 0..4 {
            if !self.parts[i].in_contact {
                if lines.distances[i] < f32::from_bits(0x3D8F_5C29) {
                    self.parts[i].normal = lines.normals[i];
                } else {
                    self.valid_wheel_normals[i] = false;
                }
            }
        }
        self.part_contact_count = self.parts.iter().filter(|part| part.in_contact).count() as u8;
        self.wheel_contact_count = self.parts[..4]
            .iter()
            .filter(|part| part.in_contact)
            .count() as u8;
        let angle_radians = maximum_ground_angle_degrees * f32::from_bits(0x3C8E_FA35);
        let minimum_up_dot = trigonometry::cos(angle_radians);
        let mut sum = Vector3::ZERO;
        for i in 0..4 {
            let contact = self.parts[i];
            if self.valid_wheel_normals[i]
                && dot(contact.normal, reckoning_up) > minimum_up_dot
                && angle_between(reference_normal, contact.normal) < f32::from_bits(0x3F49_0FDB)
            {
                sum = add(sum, contact.normal);
            }
            let drag = if contact.in_contact {
                if board_wiping_out {
                    f32::from_bits(0x3D23_D70A)
                } else {
                    0.0
                }
            } else {
                f32::from_bits(0x3BC4_9BA6)
            };
            self.wheel_angular_drag[i] = drag * f32::from_bits(0x426F_FFFF);
        }
        let squared = dot(sum, sum);
        if self.wheel_contact_count > 0 && squared > f32::from_bits(0x3780_0000) {
            self.overall_normal = scale(sum, inverse_length_squared(squared, 2));
            self.wheel_normal = self.overall_normal;
        } else {
            let mut support = Vector3::ZERO;
            // Native sum order: deck, front truck, back truck.
            for i in [
                BodyId::Deck.index(),
                BodyId::FrontTruck.index(),
                BodyId::BackTruck.index(),
            ] {
                if self.parts[i].in_contact {
                    support = add(support, self.parts[i].normal);
                }
            }
            let magnitude = length(support);
            if magnitude > f32::from_bits(0x3C23_D70A) {
                let mut inverse = native_arithmetic::reciprocal_estimate(magnitude);
                for _ in 0..2 {
                    let error = (-inverse).mul_add(magnitude, 1.0);
                    inverse = inverse.mul_add(error, inverse);
                }
                self.overall_normal = scale(support, inverse);
            }
        }
    }
}

/// Complete8296EBB0, including its small-vector gate and ONE reciprocal-square-
/// root refinement per vector. Native82C07D20 compares this angle with pi/4.
pub fn angle_between(a: Vector3, b: Vector3) -> f32 {
    let a_squared = dot(a, a);
    let b_squared = dot(b, b);
    let epsilon = f32::from_bits(0x38D1_B717);
    if !(a_squared > epsilon && b_squared > epsilon) {
        return 0.0;
    }
    let a = scale(a, inverse_length_squared(a_squared, 1));
    let b = scale(b, inverse_length_squared(b_squared, 1));
    let cosine = native_arithmetic::vector_min(native_arithmetic::vector_max(dot(a, b), -1.0), 1.0);
    trigonometry::acos(cosine)
}

#[cfg(test)]
#[path = "tests/board_ground.rs"]
mod tests;
