//! FootPlant601, vtable82327400 and manager82D704B0/82D70E40.
use super::{Footplant, GroundStep, math::*};
use crate::skate3_native::physics::{Skate3PhysicsState, Skate3ControllerState, air_phase, plant_skeleton};

pub(in crate::skate3_native::physics) fn enter(
    physics: &mut Skate3PhysicsState,
    skater: &mut Skate3ControllerState,
) -> Result<(), String> {
    physics
        .board
        .hook_mut()
        .drive
        .enable_angular_only(&mut skater.ground_lifecycle.board_animated_290);
    //82D91330 sets the foot/toe volume group and clears their disable counters.
    for part in [15, 16, 19, 20] {
        skater.skeleton_collision.parts[part].volume_group = 4;
        skater.skeleton_collision.parts[part].enabled = false;
        skater.skeleton_collision.disable_count[part] = 0;
    }
    let p = &skater.player_input.processed;
    let frames = skater.footplant.start(
        p.vectors_544_560_592_608[2].map(f32::from_bits),
        p.vectors_544_560_592_608[3].map(f32::from_bits),
    );
    if frames != 0 {
        //The native Start only disables collisions; the first Update sets IK.
        let right = skater.footplant.selected_toe == Some(19);
        for part in if right { [19, 20, 21] } else { [15, 16, 17] } {
            skater.skeleton_collision.pending_reenable = true;
            skater.skeleton_collision.disable_count[part] = frames;
            skater.skeleton_collision.parts[part].enabled = false;
        }
    }
    Ok(())
}

impl Footplant {
    pub(crate) fn start(&mut self, com: [f32; 4], velocity: [f32; 4]) -> u32 {
        self.perform = false;
        self.scalar_600 = 0.0;
        self.flag_627 = true;
        let contact = self.adjusted_contact;
        self.vectors_352_368[1] = contact;
        let offset = sub(com, contact);
        let radial = normalize(offset);
        let multiplier = self
            .settings
            .radial_speed_scale
            .evaluate(length(scale(radial, dot(velocity, radial))));
        let rotation = cross(offset, velocity);
        let angular_speed = length(rotation);
        //830BD300 is initialized by82F82690 from821647E0.
        if !(angular_speed > f32::from_bits(0x3780_0000)) {
            return 0;
        }
        let axis = scale(rotation, reciprocal(angular_speed));
        let angular_rate = angular_speed * multiplier;
        //8286CD88 projects both vectors into the plane;8296EC98 signs acos.
        let normal = scale(axis, -1.0);
        let up = [0.0, 1.0, 0.0, 0.0];
        let projected_up = normalize(sub(up, scale(normal, dot(up, normal))));
        let mut angle = if length(projected_up) == 0.0 {
            0.0
        } else {
            skate3_core::trigonometry::acos(dot(radial, projected_up).clamp(-1.0, 1.0))
        };
        if dot(cross(radial, projected_up), normal) < 0.0 {
            angle = -angle;
        }
        let min_end = angle + angular_rate * self.settings.min_duration;
        let max_end = angle + angular_rate * self.settings.max_duration;
        let end_angle = min_end
            .max(self.settings.end_angle * f32::from_bits(0x3c8e_fa35))
            .min(max_end);
        let delta = end_angle - angle;
        self.scalar_596 = delta / angular_rate;
        //Quaternion(axis,delta) applied in the native two-cross-product order.
        let (sin, cos) = skate3_core::trigonometry::sin_cos(delta * 0.5);
        let q = scale(axis, sin);
        let end_radial = madd(cross(q, madd(radial, cos, cross(q, radial))), 2.0, radial);
        let end = madd(end_radial, self.settings.end_leg_length, contact);
        let middle = normalize(add(end_radial, radial));
        let release = madd(
            sub(velocity, scale(middle, dot(velocity, middle))),
            multiplier,
            scale(middle, self.settings.release_outward_speed),
        );
        self.scalar_612 = length(release);
        self.vectors_352_368[0] = scale(release, reciprocal(self.scalar_612));
        self.curve = [
            com,
            madd(
                scale(velocity, multiplier),
                self.scalar_596 * self.settings.start_handle,
                com,
            ),
            sub(
                end,
                scale(release, self.scalar_596 * self.settings.end_handle),
            ),
            end,
        ];
        (4 - (self.scalar_596 * f32::from_bits(0xc26f_ffff)) as i32) as u32
    }

    ///82D70CE0: authored BodyAdjustX/Z move the release point, retaining
    ///the last handle length.82C1E170 removes only positive up projection.
    fn adjust(&mut self, t: f32, input: [f32; 2]) {
        let amount = (1.0 - t).mul_add(0.02, t * 0.01);
        let mut movement = [input[0] * amount, 0.0, input[1] * amount, 0.0];
        let projection = dot(movement, normalize(self.current_up));
        if projection > 0.0 {
            movement = sub(movement, scale(self.current_up, projection));
        }
        let previous = self.curve[3];
        self.curve[3] = add(previous, movement);
        let direction = normalize(sub(self.curve[3], self.curve[2]));
        self.curve[2] = sub(
            self.curve[3],
            scale(direction, length(sub(self.curve[2], previous))),
        );
        self.vectors_352_368[0] = direction;
    }
}

impl Footplant {
    pub(crate) fn ground_step(&mut self,body_adjust:[f32;2]) -> GroundStep {
        self.scalar_600 += STEP;
        let t = self.scalar_600 / self.scalar_596;
        self.adjust(t, body_adjust);
        let u = 1.0 - t;
        let position = madd(
            self.curve[3],
            t * t * t,
            madd(
                self.curve[2],
                3.0 * t * t * u,
                madd(self.curve[0], u * u * u, scale(self.curve[1], 3.0 * t * u * u)),
            ),
        );
        let launched = self.scalar_600 > self.scalar_596;
        if launched {
            self.flag_627 = false;
        }
        let anchor = self.vectors_352_368[1];
        let velocity = scale(self.vectors_352_368[0], self.scalar_612);
        let right = self.selected_toe != Some(15);
        GroundStep { position,anchor,toe:if right {19} else {15},release_velocity:launched.then_some(velocity) }
    }
    pub(crate) fn plant_wipeout_requests(&self,up:[f32;4],velocity:[f32;4]) -> [Option<(usize,f32)>;2] {
        let down=scale(up,-1.);
        let descending=dot(velocity,down);
        let horizontal=length(sub(velocity,scale(down,descending)));
        [(descending>self.settings.max_descending_speed).then_some((6,descending)),
            (horizontal>self.settings.max_horizontal_speed).then_some((23,0.))]
    }
}

pub(in crate::skate3_native::physics) fn update(
    physics: &mut Skate3PhysicsState,
    skater: &mut Skate3ControllerState,
) -> Result<(), String> {
    let step=skater.footplant.ground_step(skater.animation_input.extra.body_adjust);
    let position=step.position;
    let anchor=step.anchor;
    let right=step.toe!=15;
    plant_skeleton::hold_foot(skater, right, anchor, 0);
    plant_skeleton::advance(physics, skater, position, None)?;
    if let Some(velocity)=step.release_velocity {
        let mut info = air_phase::launch_info(physics, skater)?;
        info.start_velocity = velocity;
        info.start_position_override = add(anchor, [0.0, 0.2, 0.0, 0.0]);
        info.board_position_override = add(info.start_position_override, [0.0, -0.1, 0.0, 0.0]);
        info.use_position_override = true;
        info.player_jumped = true;
        let input = air_phase::selector_input(physics, skater)?;
        skater.trajectory.launch(info, input, &physics.world)?;
        skater.trajectory.update(input, &physics.world, crate::skate3_native::physics::air_trajectory::GrindContext::from_processed(&skater.player_input.processed, crate::skate3_native::physics::solve::deck_frame(&physics.board)[3]))?;
    }
    Ok(())
}

///82D4C6D8, after the shared plant collision check82D8FDC0.
pub(in crate::skate3_native::physics) fn post_physics(skater: &mut Skate3ControllerState) {
    let p = &skater.player_input.processed;
    for (reason,value) in skater.footplant.plant_wipeout_requests(
        p.vectors_544_560_592_608[0].map(f32::from_bits),
        p.vectors_544_560_592_608[3].map(f32::from_bits)).into_iter().flatten() {
        skater.wipeout.state.request(reason,value);
    }
}
