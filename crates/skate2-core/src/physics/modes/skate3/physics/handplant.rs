//! TU3 HandPlantManager82D61040 and physical state600 (823273CC).
mod contact;
mod ik;
mod rotation;
mod settings;
mod trajectory;
use super::footplant::math;
use super::{Skate3PhysicsState, Skate3ControllerState, plant_skeleton};
use math::*;
use settings::Settings;
use skate3_core::{air::trajectory::Trajectory, physics::skeleton_animation_record::IDENTITY};
type V = [f32; 4];
const UP: V = [0.0, 1.0, 0.0, 0.0];
const GRAVITY: V = [0.0, f32::from_bits(0xc11ccccd), 0.0, 0.0];
const EMPTY: Trajectory = Trajectory {
    position: [0.0; 4],
    velocity: [0.0; 4],
    acceleration: [0.0; 4],
    duration: -1.0,
};

pub(crate) struct Handplant {
    settings: Settings,
    pub flags: u32,
    pub phase: f32,
    pub anchor: V,
    previous_candidate_point: V, //2192; separate from the published anchor1856.
    pending: Option<(contact::Candidate, V, V, V, V)>,
    candidate: Option<contact::Candidate>,
    initial: Trajectory,
    entry: Trajectory,
    outgoing: [Trajectory; 2],
    curve: [V; 4],
    rotations: [[V; 4]; 4],
    direction: V,
    travel_sign: f32,
    elapsed: f32,
    warped: f32,
    apex: f32,
    estimated_phase: f32, //2944; retained when the authored mean is zero.
    out_duration: f32,
    pub continuation: bool,
    direction_hint: i32,
    direction_count: i32,
    ik_blend: f32,
    ik_distance: f32,
    ik_released: bool,
    ik_latched: bool,
}
impl Handplant {
    pub fn load(data: &skate_data::collections::Collections) -> Result<Self, String> {
        Ok(Self {
            settings: Settings::load(data)?,
            flags: 0,
            phase: f32::MAX,
            anchor: [0.0; 4],
            previous_candidate_point: [0.0; 4],
            pending: None,
            candidate: None,
            initial: EMPTY,
            entry: EMPTY,
            outgoing: [EMPTY; 2],
            curve: [[0.0; 4]; 4],
            rotations: [IDENTITY; 4],
            direction: [0.0; 4],
            travel_sign: 0.0,
            elapsed: -1.0,
            warped: -1.0,
            apex: -1.0,
            estimated_phase: -1.0,
            out_duration: 0.0,
            continuation: true,
            direction_hint: 0,
            direction_count: 0,
            ik_blend: 0.0,
            ik_distance: -1.0,
            ik_released: false,
            ik_latched: false,
        })
    }
    pub fn animation_thresholds(&self) -> [f32; 3] {
        self.settings.animation
    }
    ///82D62F20 preserves the new-position bit and the stored trajectories.
    pub fn reset(&mut self) {
        self.flags &= !0xb000_0000;
        self.phase = f32::MAX;
        self.anchor = [0.0; 4];
        self.pending = None;
        self.candidate = None;
        self.elapsed = -1.0;
        self.warped = -1.0;
        self.apex = -1.0;
        self.estimated_phase = -1.0;
        self.out_duration = 0.0;
        self.direction = [0.0; 4];
        self.continuation = true;
    }
    pub fn full_reset(&mut self) {
        self.reset();
        self.reset_ik();
        self.direction_hint = 0;
        self.direction_count = 0;
    }
    fn reset_ik(&mut self) {
        self.ik_blend = 0.0;
        self.ik_distance = -1.0;
        self.ik_released = false;
        self.ik_latched = false;
    }
    fn estimate_apex(&mut self) {
        //82D63280 averages four authored time-warp samples.
        let offset = self.warped - self.apex;
        let stride = (offset * f32::from_bits(0x3eaaaaab)).abs(); //Globals440
        let mean = (0..4)
            .map(|i| 1.0 + self.settings.time_warp.evaluate(offset + i as f32 * stride))
            .sum::<f32>()
            * 0.25;
        if mean != 0.0 {
            self.estimated_phase = (-1.0 / mean) * offset;
        }
        self.phase = self.estimated_phase;
    }
}
pub(super) fn apex_time(t: Trajectory) -> f32 {
    if t.velocity[1] < 0.0 || t.acceleration[1] >= 0.0 {
        0.0
    } else {
        t.velocity[1] * reciprocal(-t.acceleration[1])
    }
}

///Ground82D37F38 consumes the candidate query before its shared skeleton update.
pub(super) fn ground_update(
    physics: &mut Skate3PhysicsState,
    skater: &mut Skate3ControllerState,
) -> Result<(), String> {
    let p = &skater.player_input.processed;
    if p.flags_2476 & (1 << 22) == 0 {
        skater.handplant.full_reset();
        return Ok(());
    }
    if let Some((candidate, com, velocity, normal, heading)) = skater.handplant.pending.take() {
        use skate3_core::air::trajectory::grind_surface::{self, InvestigationInput, GeometryType};
        let query = InvestigationInput {
            start: candidate.edge.start,
            end: candidate.edge.end,
            reference: candidate.point,
            optional_probe: None,
            deck_center_to_truck: skater.handplant.settings.truck_distance,
        };
        let surface = grind_surface::investigate(query, |index, probe| {
            super::player_input::grind::world::surface_probe(&physics.world,
                [p.actor_query_2948, p.actor_query_2952], index, probe)
        })?;
        crate::skate3_native::trace::info!("HANDPLANT_SURFACE tick={} kind={:?} point={:?}",
            physics.ticks, surface.kind, candidate.point);
        if grind_surface::prepare(query).is_some() && surface.kind != GeometryType::Impossible {
            skater.handplant.launch(
                candidate,
                com,
                velocity,
                normal,
                heading,
                physics.riding.reckoning_frames.heading,
            );
        }
    }
    if skater.handplant.flags & 0x8000_0000 != 0 {
        ik::update(skater, true);
    }
    Ok(())
}

/// Capture the solved pose before the bail check, rather than comparing this
/// tick's targets against last tick's bodies in HANDPLANT_POSE.
pub(super) fn trace_solved(physics: &Skate3PhysicsState, skater: &Skate3ControllerState) {
    if physics.ticks % 6 != 0 && skater.collision_maximum_error.unwrap_or(0.0) < 0.15 {
        return;
    }
    let root = &skater.animated_skeleton.roots.animation_to_world;
    let actual = skater.skeleton.part_transforms();
    let parts = [3, 4, 5, 6, 7, 8, 9, 10];
    let authored = parts.map(|i| point(root, skater.animated_skeleton.record.pose[i][3]));
    let targets = parts.map(|i| point(root, skater.skeleton_input.drive_frames[i][3]));
    let solved = parts.map(|i| actual[i][3]);
    let strengths = parts.map(|i| skater.skeleton_drives.bones[i].as_ref()
        .map(|bone| (bone.active, bone.dynamics.strengths)));
    let ik = &skater.foot_ik.state.limbs;
    let feedback = &skater.collision_feedback;
    crate::skate3_native::trace::info!("HANDPLANT_SOLVED tick={} phase={} parts={parts:?} authored={authored:?} targets={targets:?} solved={solved:?} strengths={strengths:?} ik={ik:?} partial={} collision_weight={} pose_errors={:?} anchor={:?}",
        physics.ticks, skater.handplant.phase, skater.skeleton_collision.partial_ragdoll,
        feedback.drive_weight, skater.pose_errors.parts, skater.handplant.anchor);
}
///Ground82D38430 submits the candidate before82D37F38 consumes its investigation.
pub(super) fn ground_query(physics: &Skate3PhysicsState, skater: &mut Skate3ControllerState) {
    let p = &skater.player_input.processed;
    if p.flags_2476 & (1 << 22) == 0 {
        skater.handplant.full_reset();
        return;
    }
    let h = &mut skater.handplant;
    let com = p.vectors_544_560_592_608[2].map(f32::from_bits);
    let velocity = p.vectors_400_416[0].map(f32::from_bits);
    let normal = p.vectors_464_480_496_512_528[0].map(f32::from_bits);
    let heading = p.vectors_544_560_592_608[1].map(f32::from_bits);
    if p.flags_2480 & (1 << 29) != 0 || p.flags_2480 & (1 << 28) == 0 {
        h.direction_hint = 0;
        h.direction_count = 0;
    } else if normal[1] < 0.95 {
        if h.direction_hint == 0 {
            h.direction_hint = h.candidate.map_or(0, |c| c.side);
        }
        let side = if dot(velocity, normalize(cross(UP, normal))) > 0.0 {
            1
        } else {
            2
        };
        if h.direction_hint == side {
            h.direction_count = 0;
        } else {
            h.direction_count += 1;
            if h.direction_count > h.settings.direction_frames {
                h.direction_hint = 0;
            }
        }
    } else {
        h.direction_hint = 0;
        h.direction_count = 0;
    }
    let candidate = contact::select(
        &h.settings,
        com,
        velocity,
        normal,
        h.direction_hint,
        physics.grind_world.primitives(),
    );
    if candidate.is_some() || physics.ticks % 30 == 0 {
        crate::skate3_native::trace::info!("HANDPLANT_QUERY tick={} speed={} vy={} normal={:?} minimum_speed={} minimum_slope={} edges={} candidate={:?} prior_flags={:08x} phase={} processed={:08x}/{:08x}",
            physics.ticks, length(velocity), velocity[1], normal, h.settings.minimum_speed,
            h.settings.minimum_slope, physics.grind_world.primitives().len(), candidate.map(|c| c.point),
            h.flags, h.phase, p.flags_2476, p.flags_2480);
    }
    //82D61268 clears the active output when submission begins.
    let old_point = h.candidate.map_or([0.0; 4], |c| c.point);
    let hint = h.direction_hint;
    let count = h.direction_count;
    h.reset();
    h.previous_candidate_point = old_point;
    h.direction_hint = hint;
    h.direction_count = count;
    h.candidate = candidate;
    h.pending = candidate.map(|c| (c, com, velocity, normal, heading));
}
pub(super) fn enter(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    physics
        .board
        .hook_mut()
        .drive
        .enable_angular_only(&mut skater.ground_lifecycle.board_animated_290);
    let p = &skater.player_input.processed;
    let h = &mut skater.handplant;
    h.entry = Trajectory {
        position: p.vectors_544_560_592_608[2].map(f32::from_bits),
        velocity: p.vectors_544_560_592_608[3].map(f32::from_bits),
        acceleration: GRAVITY,
        duration: -1.0,
    };
    h.warped = 0.0;
    h.elapsed = 0.0;
    h.continuation = true;
    h.select_outgoing(&physics.world)?;
    Ok(())
}
pub(super) fn update(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    //82D4C3D8 ->82D91298: keep planted hands out of collision response.
    skater.skeleton_collision.disable_handplant_contacts(2);
    let h = &mut skater.handplant;
    h.warped += (1.0 + h.settings.time_warp.evaluate(h.warped - h.apex)) * STEP;
    h.estimate_apex();
    h.elapsed += STEP;
    let pose = skater.animated_skeleton.record.pose.map(|local| {
        skate3_core::physics::skeleton_animation_record::compose_affine(
            &skater.animated_skeleton.roots.animation_to_world,
            &local,
        )
    });
    let (com, up, heading) = h.values(
        &pose,
        skater.player_input.processed.vectors_544_560_592_608[2].map(f32::from_bits),
    );
    skater.air_reckoning.update_plant(
        &mut physics.riding,
        &skater.player_input.processed,
        up,
        heading,
    );
    ik::update(skater, false);
    plant_skeleton::advance(physics, skater, com, None)?;
    if physics.ticks % 6 == 0 || skater.handplant.elapsed <= STEP * 1.5 {
        let root = &skater.animated_skeleton.roots.animation_to_world;
        let parts = [23, 1, 3, 7, 15, 19]; //hips, head, hands, toes
        let targets = parts.map(|i| point(root, skater.skeleton_input.drive_frames[i][3]));
        let actual = skater.skeleton.part_transforms();
        let actual = parts.map(|i| actual[i][3]);
        let hands = [2, 3].map(|i| (skater.foot_ik.state.external_targets[i].world_position,
            skater.foot_ik.state.limbs[i].target_blend));
        crate::skate3_native::trace::info!("HANDPLANT_POSE tick={} elapsed={} phase={} com={com:?} up={up:?} heading={heading:?} root={root:?} targets={targets:?} actual={actual:?} hands={hands:?} force_mode={} flags={:08x}/{:08x}/{:08x}",
            physics.ticks, skater.handplant.elapsed, skater.handplant.phase,
            skater.skeleton_input.force_mode, skater.player_input.processed.flags_2468,
            skater.player_input.processed.flags_2472, skater.player_input.processed.flags_2476);
    }
    Ok(())
}
