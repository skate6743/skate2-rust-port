//! Handplant Launch82D613B0, trajectory selector82D66318/66948 and COM82D625C0.
use super::*;
use skate3_core::{air::trajectory::QueryRequest, physics::board_world::BoardWorld};

impl Handplant {
    pub(super) fn launch(
        &mut self,
        c: contact::Candidate,
        com: V,
        velocity: V,
        normal: V,
        body_heading: V,
        heading: V,
    ) {
        let mut side = normalize(cross(sub(c.edge.end, c.edge.start), UP));
        if dot(side, normal) > 0.0 {
            side = scale(side, -1.0);
        }
        self.direction = scale(side, -1.0);
        let apex = apex_position(c.point, side, self.settings.apex_radius, self.settings.apex_angle);
        self.initial = to_apex(com, apex);
        self.apex = apex_time(self.initial);
        self.warped = 0.0;
        self.outgoing = [self.initial; 2];
        self.travel_sign = if dot(heading, velocity) > 0.0 {
            1.0
        } else {
            -1.0
        };
        //830BD4A0=-1 and830BD320=1; signs follow native vsel order.
        self.rotations[0] = rotation::frame(heading, normal);
        self.rotations[0][3] = com;
        let mut along = normalize(sub(c.edge.start, c.edge.end));
        let direction = if dot([along[0], 0.0, along[2], 0.0], sub(apex, com)) > 0.0 {
            1.0
        } else {
            -1.0
        };
        along = scale(along, direction * self.travel_sign);
        self.rotations[1] = rotation::frame(along, normalize(sub(c.point, apex)));
        self.rotations[1][3] = apex;
        self.rotations[2] = rotation::frame(scale(UP, -self.travel_sign), normal);
        self.rotations[3] = self.rotations[2];
        self.estimate_apex();
        let changed = length(sub(self.previous_candidate_point, c.point)) > 0.15;
        let front = dot(sub(c.point, com), body_heading) < 0.0;
        self.flags = (self.flags & 0x0fff_ffff)
            | 0x8000_0000
            | if changed { 0x4000_0000 } else { 0 }
            | if front { 0x2000_0000 } else { 0 }
            | if self.phase < self.settings.committed_time {
                0x1000_0000
            } else {
                0
            };
        self.anchor = c.point;
        self.candidate = Some(c);
    }
    pub(super) fn select_outgoing(&mut self, world: &BoardWorld) -> Result<(), String> {
        let candidate = self
            .candidate
            .ok_or("Handplant entry requires the selected coping")?;
        let apex = self.initial.position_at(self.apex);
        let incoming = self.initial.velocity_at(self.apex);
        let across = scale(self.direction, dot(incoming, self.direction));
        let tangent = sub(incoming, across);
        let along = normalize(tangent);
        let speed = length(add(across, clamp_length(tangent, 2.0)));
        if candidate.side != 0 {
            let expected = scale(
                cross(UP, self.direction),
                if candidate.side == 1 { 1.0 } else { -1.0 },
            );
            self.continuation = dot(expected, along) >= 0.0;
        }
        let mut position = madd(self.direction, 0.1, self.anchor);
        position[1] = apex[1];
        let mut selected = None;
        let mut best_up = 1.1;
        let mut final_trajectory = EMPTY;
        for angle in [
            0.0,
            0.24,
            0.48,
            f32::from_bits(0x3f3851eb),
            0.96,
            f32::from_bits(0x3f999999),
        ] {
            let (sin, cos) = skate3_core::trigonometry::sin_cos(angle);
            let velocity = scale(madd(self.direction, sin, scale(along, cos)), speed);
            let trajectory = Trajectory {
                position,
                velocity,
                acceleration: GRAVITY,
                duration: 3.0,
            };
            final_trajectory = trajectory;
            let hit = super::super::air_trajectory::AirTrajectoryRuntime::query(
                world,
                QueryRequest {
                    trajectory,
                    radius: 0.2,
                    start_error: 1.0,
                    end_error: 1.0,
                },
            )?;
            if hit.contact_time < 0.0 {
                continue;
            }
            let contact = hit.contact_position; //Admission uses GetPosition82D2D988.
            if self.anchor[1] - contact[1] <= 0.5 {
                continue;
            }
            let normal = hit.landing_normal;
            //82D66B64..98/82D66BC0..BF8 evaluate the query trajectory at
            //contact time for the landing anchor, not the surface contact.
            let point = trajectory.position_at(hit.contact_time);
            //The host sphere query can return a nearby ledge whose lifted
            //landing target lies above the apex. It cannot define a descending
            //arc (82D608A0 takes sqrt(2*g*drop)). Keep searching; if none is
            //feasible, use the existing native three-second fallback below.
            let destination = madd(normal, 0.56, point);
            if !destination.iter().all(|v| v.is_finite()) || !(destination[1] < apex[1]) {
                continue;
            }
            if normal[1] < 0.71 {
                selected = Some((point, normal));
                break;
            }
            if normal[1] < best_up {
                best_up = normal[1];
                selected = Some((point, normal));
            }
        }
        let (landing, normal) = selected.unwrap_or((final_trajectory.position_at(3.0), UP));
        let destination = madd(normal, 0.56, landing);
        let mut outgoing = from_apex(apex, destination);
        let fall_time = outgoing.duration;
        //82D66D70 rebase the outgoing parabola to the incoming time origin.
        outgoing.position = outgoing.position_at(-self.apex);
        outgoing.velocity = outgoing.velocity_at(-self.apex);
        outgoing.duration = -1.0;
        self.outgoing = [outgoing; 2];
        let speed = dot(self.direction, self.outgoing[1].velocity);
        if speed < self.settings.minimum_out_speed {
            self.outgoing[1].velocity = madd(
                self.direction,
                self.settings.minimum_out_speed - speed,
                self.outgoing[1].velocity,
            );
        }
        self.rotations[2][3] = outgoing.position_at(2.0 * self.apex);
        let time = self.apex + fall_time;
        self.rotations[3] = rotation::frame(
            scale(normalize(outgoing.velocity_at(time)), self.travel_sign),
            normal,
        );
        self.rotations[3][3] = outgoing.position_at(time);
        self.out_duration = (time - self.phase).max(0.5);
        self.build_curve();
        Ok(())
    }
    fn build_curve(&mut self) {
        //82D62408: secant handles one fixed tick apart, scaled by6.
        let start = self.apex - self.settings.curve_half_time;
        let end = self.apex + self.settings.curve_half_time;
        self.curve[0] = self.initial.position_at(start);
        self.curve[1] = madd(
            sub(self.initial.position_at(start + STEP), self.curve[0]),
            6.0,
            self.curve[0],
        );
        self.curve[3] = self.outgoing[1].position_at(end);
        self.curve[2] = madd(
            sub(self.outgoing[1].position_at(end - STEP), self.curve[3]),
            6.0,
            self.curve[3],
        );
    }
    pub(super) fn values(&mut self, pose: &[[V; 4]], physical_com: V) -> (V, V, V) {
        let relative = self.warped - self.apex;
        let fraction = (1.0 + relative * reciprocal(self.settings.curve_half_time)) * 0.5;
        let mut com = if fraction < 0.0 {
            self.initial.position_at(self.warped)
        } else if fraction > 1.0 {
            let blend =
                clamp01((relative - self.settings.curve_half_time) * reciprocal(self.out_duration));
            madd(
                self.outgoing[0].position_at(self.warped),
                blend,
                scale(self.outgoing[1].position_at(self.warped), 1.0 - blend),
            )
        } else {
            bezier(self.curve, fraction)
        };
        let foot_y = pose[15][3][1].max(pose[19][3][1]);
        let (divisor, offset) = if self.flags & 0x2000_0000 != 0 {
            (5.0, 0.0)
        } else {
            (3.2, -0.05)
        };
        com[1] += ((foot_y - physical_com[1]) / divisor + offset).max(0.0);
        let blend = (1.0 - self.elapsed * reciprocal(self.settings.entry_blend)).max(0.0);
        let blended = madd(
            com,
            1.0 - blend,
            scale(self.entry.position_at(self.elapsed), blend),
        );
        if blend > 0.0 && fraction < 0.0 {
            let velocity = self.initial.velocity_at(self.warped);
            let correction =
                dot(sub(blended, com), normalize(velocity)) * reciprocal(length(velocity));
            self.warped += correction.clamp(f32::from_bits(0xbc088889), f32::from_bits(0x3c088889));
        }
        let frame = if self.warped < self.apex {
            let fraction = (1.0
                - (self.apex - self.warped)
                    * reciprocal(self.apex.min(self.settings.rotation_time)))
            .max(0.0);
            rotation::blend(
                self.rotations[0],
                self.rotations[1],
                self.settings.into_rotation.evaluate(fraction),
            )
        } else {
            let fraction = self.warped - self.apex;
            let out = self
                .settings
                .out_heading
                .evaluate(clamp01(fraction * reciprocal(self.out_duration)));
            let into = self
                .settings
                .out_rotation
                .evaluate(clamp01(fraction * reciprocal(self.settings.rotation_time)));
            rotation::blend(
                self.rotations[1],
                rotation::blend(self.rotations[2], self.rotations[3], out),
                into,
            )
        };
        (blended, frame[1], frame[2])
    }
}
///82D61458/64 call standalone Cos/Sin. Permute mask822FBAC0 followed
///by vsldoi8 produces[sin,cos,0,0]: sine goes sideways, cosine goes up.
fn apex_position(coping: V, side: V, radius: f32, angle: f32) -> V {
    let sin = skate3_core::trigonometry::sin(angle);
    let cos = skate3_core::trigonometry::cos(angle);
    madd(side, radius * sin, madd(UP, radius * cos, coping))
}
fn to_apex(start: V, end: V) -> Trajectory {
    let t = ((-2.0 * GRAVITY[1]) * (end[1] - start[1])).sqrt() * reciprocal(-GRAVITY[1]);
    arc_between(start, end, t)
}

#[cfg(any())]
mod tests {
    use super::*;
    #[test]
    fn stock_handplant_apex_is_above_coping_and_back_towards_ramp() {
        let coping = [-7.0, 3.8850045, 5.5987973, 0.0];
        let apex = apex_position(coping, [0.0,0.0,1.0,0.0], 0.8, -0.3);
        assert!((apex[1]-4.649274).abs()<0.00001);
        assert!((apex[2]-5.362381).abs()<0.00001);
        //Observed late attempt COM previously exceeded the incorrectly lowered
        //apex. The original geometry admits a finite ascending trajectory.
        let start = [-6.9291053,3.925457,4.9562364,0.0];
        let trajectory = to_apex(start,apex);
        assert!(trajectory.velocity.iter().all(|v|v.is_finite()));
        assert!(trajectory.duration>0.0);
        let end=trajectory.position_at(trajectory.duration);
        for i in 0..3 { assert!((end[i]-apex[i]).abs()<0.00001); }
        assert!(trajectory.velocity_at(trajectory.duration)[1].abs()<0.00001);
    }
    #[test]
    fn apex_angle_zero_places_com_directly_above_lip_for_either_edge_direction() {
        for direction in [-1.0,1.0] {
            let apex=apex_position([2.0,3.0,4.0,0.0],[direction,0.0,0.0,0.0],0.8,0.0);
            assert!((apex[1]-3.8).abs()<0.00001);
            assert!((apex[0]-2.0).abs()<0.00001);
            assert!((apex[2]-4.0).abs()<0.00001);
        }
    }
}
fn from_apex(start: V, end: V) -> Trajectory {
    let t = ((-2.0 * GRAVITY[1]) * (start[1] - end[1])).sqrt() * reciprocal(-GRAVITY[1]);
    arc_between(start, end, t)
}
fn arc_between(start: V, end: V, time: f32) -> Trajectory {
    let velocity = scale(
        sub(sub(end, start), scale(GRAVITY, 0.5 * time * time)),
        reciprocal(time),
    );
    Trajectory {
        position: start,
        velocity,
        acceleration: GRAVITY,
        duration: time,
    }
}
fn bezier(p: [V; 4], t: f32) -> V {
    let s = 1.0 - t;
    madd(
        p[3],
        t * t * t,
        madd(
            p[2],
            3.0 * t * t * s,
            madd(p[0], s * s * s, scale(p[1], 3.0 * t * s * s)),
        ),
    )
}
