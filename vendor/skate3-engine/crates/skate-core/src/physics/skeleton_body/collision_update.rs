//! Complete contact traversal and classification of TU3 82BD4A30.
use super::collision_feedback::*;
use super::collision_vector::{
    add, clamp_length, cross, dot, length, normalize, scale, sub, transform,
};

// Original read-only table820CFCB0. Part0 is not a region participant.
const REGIONS: [usize; 24] = [
    usize::MAX,
    0,
    1,
    2,
    2,
    2,
    1,
    3,
    3,
    3,
    1,
    1,
    1,
    1,
    1,
    6,
    6,
    4,
    4,
    7,
    7,
    5,
    5,
    1,
];

impl SkeletonCollisionFeedback {
    pub fn update(
        &mut self,
        input: &SkeletonCollisionInput<'_>,
        reports: &[SkeletonContactReport],
    ) {
        self.begin_frame(input);
        let reference_speed = length(input.reference_velocity);
        let direction = normalize(input.reference_velocity);
        for report in reports {
            // Original constructor enables reports for the24 animated parts.
            // A different owner must not silently index this state as part24/25.
            assert!(
                report.part < 24,
                "SkeletonCollision report part exceeds native24-part array"
            );
            let part = report.part;
            let material = (report.tag >> 7) & 31;
            self.flags.any = true;
            if report.other_group == 4 && [15, 16, 19, 20].contains(&part) {
                self.flags.foot_board = true;
            }
            if material == 6 {
                self.flags.material_6 = true;
            }
            if material == 12 {
                self.flags.material_12 = true;
                self.material_12_height = report.point[1];
            }
            if self.ignore_ground(input, report) {
                continue;
            }
            self.observe_normal(input.ragdoll, report, material);
            let mut specific = false;
            for point in &mut self.specific {
                let delta = sub(report.point, point.world_point);
                if part == point.part && dot(delta, delta) < point.radius_squared {
                    point.current = true;
                    point.recent = true;
                    specific = true;
                }
            }
            // This gate precedes priority replacement and all force recording.
            if self.planes.len() >= 20 {
                continue;
            }
            self.contact_age[part] = 0.0;
            self.current[part] = true;
            if self.priority[part] > self.maximum_priority {
                self.maximum_priority = self.priority[part];
                self.planes.clear();
            }
            if self.priority[part] == self.maximum_priority {
                self.planes.push(ContactPlane {
                    normal: report.normal,
                    part,
                });
            }

            let mass_a = mass(report.body_a);
            let mass_b = mass(report.body_b);
            let other_mass = if report.side_a { mass_b } else { mass_a };
            let small = other_mass > 0.0001 && other_mass < self.settings.small_object_mass;
            let factor = ((mass_b + mass_a) / (mass_b * mass_a)) * f32::from_bits(0x3991_A2B5);
            let weighted_force =
                length(scale(report.solved_vector, factor)) * input.part_weights[part];
            let part_velocity = input.physical.velocities[part];
            let own_velocity = if input.ragdoll {
                scale(add(input.com_velocity, part_velocity), 0.5)
            } else if input.offboard || input.entering_offboard {
                let mut velocity = input.com_velocity;
                velocity[1] = 0.0;
                velocity
            } else {
                let projected = scale(direction, dot(part_velocity, direction));
                clamp_length(projected, reference_speed)
            };
            let other = if report.side_a {
                report.body_b
            } else {
                report.body_a
            };
            let relative = sub(other.linear_velocity, own_velocity);
            let mut force = dot(relative, report.normal).abs();
            if small {
                force *= 0.5;
            }
            match report.other_group {
                5 => {
                    force *= if input.ai_collision_scalar {
                        self.settings.ai_scalar
                    } else {
                        self.settings.skater_scalar
                    };
                    self.maximum_skater_force = maximum(self.maximum_skater_force, force);
                    if let Some(entity) = report.other_entity {
                        self.other_skater = entity;
                    }
                    self.flags.nonboard = true;
                }
                8 => {
                    self.flags.group_8 = true;
                    self.maximum_group_8_force = maximum(self.maximum_group_8_force, force);
                    self.flags.nonboard = true;
                }
                11 => {
                    self.maximum_group_11_force = maximum(self.maximum_group_11_force, force);
                    self.flags.nonboard = true;
                }
                4 => (),
                _ => self.flags.nonboard = true,
            }
            if part > 0 {
                self.record_force(report, specific, relative, force, weighted_force);
                self.timer = self.settings.body.effect_time;
            }
            if !self.compliant[part] || small {
                self.flags.noncompliant = true;
            } else {
                self.flags.compliant = true;
            }
            self.flags.has_impulse = self.flags.compliant || input.offboard;
        }
        if input.ragdoll {
            self.flags.impaled = self.check_impaled();
        } else {
            self.flags.conflicting = self.check_conflicting(input);
        }
        let duration = self.settings.body.effect_time;
        self.drive_weight = if duration == 0.0 {
            1.0
        } else {
            let ratio = self.timer / duration;
            let low = if -ratio >= 0.0 { 0.0 } else { ratio };
            1.0 - if 1.0 - low >= 0.0 { low } else { 1.0 }
        };
        self.flags.recovering = self.drive_weight < 1.0;
    }

    fn begin_frame(&mut self, input: &SkeletonCollisionInput<'_>) {
        self.timer -= input.dt;
        self.wipeout_times[2] += input.dt;
        self.flags = SkeletonContactFlags {
            ragdoll: input.ragdoll,
            ..Default::default()
        };
        self.planes.clear();
        self.maximum_priority = 0.0;
        self.maximum_skater_force = 0.0;
        self.other_skater = -1;
        self.maximum_group_8_force = 0.0;
        self.maximum_group_11_force = 0.0;
        self.material_12_height = 0.0;
        self.highest_normal = [0.0, -1.0, 0.0, 0.0];
        self.foot_normal = [0.0; 4];
        self.material_normals = [[0.0, -1.0, 0.0, 0.0]; 2];
        for part in 0..24 {
            self.contact_age[part] -= input.dt;
            self.current[part] = false;
            self.bones[part] = BoneContact::default();
        }
        for region in &mut self.regions {
            // Unlike Reset, Update leaves the material flag untouched.
            *region = ContactRegion {
                material_flags: region.material_flags,
                ..Default::default()
            };
        }
        if !input.ragdoll {
            self.wipeout_times = [0.0; 3];
        }
        for point in &mut self.specific {
            point.recent &= input.ragdoll;
            point.current = false;
            point.world_point = transform(input.physical.pose[point.part], point.local_point);
        }
        if input.request_partial_ragdoll {
            self.timer = self.settings.body.effect_time;
        }
    }

    fn ignore_ground(
        &self,
        input: &SkeletonCollisionInput<'_>,
        report: &SkeletonContactReport,
    ) -> bool {
        if input.ragdoll || (input.disable_ground_filter && !input.category_600) {
            return false;
        }
        let distance = dot(input.plane_normal, sub(report.point, input.plane_point));
        let threshold = if input.offboard {
            0.3
        } else {
            self.settings.ground_plane_max_distance
        };
        distance < threshold
            && (input.offboard
                || dot(report.normal, input.plane_normal).abs()
                    > self.settings.ground_plane_max_angle)
    }
    fn observe_normal(&mut self, ragdoll: bool, report: &SkeletonContactReport, material: u32) {
        let normal = report.normal;
        if normal[1] > self.highest_normal[1] {
            self.highest_normal = normal;
        }
        if ragdoll {
            if material == 10 && normal[1] > self.material_normals[0][1] {
                self.material_normals[0] = normal;
                self.flags.material_10 = true;
            } else if material == 11 && normal[1] > self.material_normals[1][1] {
                self.material_normals[1] = normal;
                self.flags.material_11 = true;
            }
        } else if (report.part == 15 || report.part == 19) && normal[1] > self.foot_normal[1] {
            self.foot_normal = normal;
        }
    }
    fn record_force(
        &mut self,
        report: &SkeletonContactReport,
        specific: bool,
        relative: V,
        force: f32,
        weighted_force: f32,
    ) {
        let bone = &mut self.bones[report.part];
        let region = &mut self.regions[REGIONS[report.part]];
        let previous = if specific {
            bone.specific_force
        } else {
            bone.force
        };
        let tangent = if force > previous || force > region.force {
            cross(relative, report.normal)
        } else {
            [0.0; 4]
        };
        if force > previous {
            if specific {
                bone.specific_force = force;
                bone.specific_normal = report.normal;
                bone.specific_tangent = tangent;
                bone.specific_tag = report.tag;
            } else {
                bone.force = force;
                bone.normal = report.normal;
                bone.tangent = tangent;
                bone.point = report.point;
                bone.tag = report.tag;
            }
            for (i, group) in [8, 11, 5].into_iter().enumerate() {
                bone.groups[i] |= report.other_group == group;
            }
            bone.groups[3] |= ![8, 11, 5].contains(&report.other_group);
        }
        if force > region.force {
            *region = ContactRegion {
                force,
                weighted_force,
                tangent_speed: length(tangent),
                material_flags: report.tag & 0x7f,
                normal: report.normal,
                part: Some(report.part),
            };
        }
    }
    /// Original82BD5770 calls GetPartTransform, not animation-pose lookup.
    fn check_conflicting(&self, input: &SkeletonCollisionInput<'_>) -> bool {
        for (i, a) in self.regions.iter().enumerate() {
            let Some(part_a) = a.part else {
                continue;
            };
            if !(a.force > 0.0) || !self.compliant[part_a] {
                continue;
            }
            for (j, b) in self.regions.iter().enumerate() {
                let Some(part_b) = b.part else {
                    continue;
                };
                if i == j || !(b.force > 0.0) || !self.compliant[part_b] {
                    continue;
                }
                if dot(a.normal, b.normal) < -0.7
                    && dot(
                        sub(input.body_frames[part_b][3], input.body_frames[part_a][3]),
                        a.normal,
                    ) < 0.0
                {
                    return true;
                }
            }
        }
        false
    }
    fn check_impaled(&self) -> bool {
        for i in 1..23 {
            let a = self.bones[i];
            if !(a.force > 0.0) {
                continue;
            }
            for b in &self.bones[i + 1..24] {
                if b.force > 0.0
                    && dot(a.normal, b.normal) < -0.99
                    && dot(a.normal, sub(a.point, b.point)).abs() < 0.03
                {
                    return true;
                }
            }
        }
        false
    }
}
fn mass(body: SkeletonContactBody) -> f32 {
    1.0 / if body.state_flags == 1 {
        0.001
    } else {
        body.inverse_mass
    }
}
fn maximum(a: f32, b: f32) -> f32 {
    if a - b >= 0.0 { a } else { b }
}
