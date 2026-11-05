//! TU3 actor checkpoint manager, bound to the single-player static scene.
use super::{Skate3PhysicsState, Skate3ControllerState, offboard::contact_queries, teleport_state::Checkpoint};
use skate3_core::{
    math::Vector3,
    physics::{
        board_world::{BoardWorld, query_metadata::Bounds},
        skeleton_animation_record::AnimationPartTransform as Matrix,
    },
    player::{
        respawn::{Candidate, Ground, History, Observation, Validation, surface_allowed},
    },
};
use skate_data::collections::Collections;
use super::offboard::contact_queries::Probe;

pub(super) struct Runtime {
    history: History,
    settings: Settings,
    measurements: i32,
    /// Actor82592518's request from this tick's Teleporting update.
    requested: bool,
    /// This tick's completed output for the Actor's PostUpdate (82BFB1E0).
    observation: Option<Observation>,
}
struct Settings {
    height: f32,
    radius: f32,
    drop: f32,
    normal_y: f32,
    minimum_frames: u32,
    pedestrian_radius: f32,
    vehicle_radius: f32,
}

/// The Actor's services the history asks about a position: the network game's collision query
/// (82BFBB48) and the living world's DMO, vehicle and pedestrian tests (82BFB928). A standalone
/// source run has neither, and neither rejects there.
pub(crate) trait RespawnHost {
    /// `INetworkGameQuery` slot 0 (IsNetworkGame); breadcrumb Update82BFB1E0 skips its ground
    /// test in a network game.
    fn network_game(&mut self) -> Result<bool, String>;
    /// `INetworkGameCollisionQuery` slot 4 with radius -1, asked only in a network game.
    fn network_position_ok(&mut self, position: [f32; 4]) -> Result<bool, String>;
    /// `TestForLWRespawnContraints` 82BFB928, which accepts every position in a network game.
    fn living_world_clear(&mut self, position: [f32; 4], radii: LivingWorldRadii) -> Result<bool, String>;
}

/// skater_breadcrumbs.default's radii and capsule for the living-world test.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LivingWorldRadii {
    pub pedestrian: f32,
    pub vehicle: f32,
    pub capsule_height: f32,
    pub capsule_radius: f32,
}

impl Runtime {
    pub fn load(data: &Collections, transform: Matrix, stance: u32) -> Result<Self, String> {
        let class = "Hash_12B64C0E804B0853";
        let value = |hash| data.float(class, "default", hash);
        // The generated accessors return DefaultDataArea's zero for an absent field.
        let optional = |hash| match data.float(class, "default", hash) {
            Err(error) if error.starts_with("Missing stock field") => Ok(0.),
            other => other,
        };
        Ok(Self {
            history: History::new(Candidate {
                transform,
                stance,
                offboard: false,
                score: 0.,
            }),
            measurements: 0,
            requested: false,
            observation: None,
            settings: Settings {
                height: value("Hash_C0526C883AF0ECCA")?,
                radius: value("Hash_CEB092E418A5B001")?,
                drop: value("Hash_8ABE098D3806D273")?,
                normal_y: value("Hash_ADD032CACF6A1C15")?,
                minimum_frames: data.integer(class, "default", "Hash_10B7C3A9CC8D3721")? as u32,
                pedestrian_radius: optional("Hash_E64C980EE2114070")?,
                vehicle_radius: optional("Hash_59E1F2B3CDC4E658")?,
            },
        })
    }
    pub fn reset_measurements(&mut self) {
        self.measurements = 0;
    }
    /// Teleporting82D431F0 calls the Actor; it answers after the source tick.
    pub fn request(&mut self) {
        self.requested = true;
    }
    /// The conditioner's sample count since the last reset (EmpiricalMeasurements82DE5858).
    pub fn measurements(&self) -> i32 {
        self.measurements
    }
    /// SkaterManager::OnCommand case 3 replaces the Actor; its new manager starts at the spawn.
    pub fn respawn(&mut self, initial: Candidate) {
        self.history.respawn(initial);
    }
    pub fn import(&mut self, entries: &[Candidate]) {
        self.history.import(entries);
    }
    /// The same Actor manager's riding orientation (+208) and position (+272) at the takeover.
    pub fn continue_from(&mut self, orientation: Matrix, position: [f32; 4]) {
        self.history.current_orientation = orientation;
        self.history.current_position = position;
    }
    pub fn history(&self) -> &History {
        &self.history
    }
}

///Actor82592518 saves stance then queues the ordinary deferred reply825926F8.
fn answer(physics: &Skate3PhysicsState, skater: &mut Skate3ControllerState, host: Option<&mut (dyn RespawnHost + '_)>) -> Result<(), String> {
    let stance = skater.animation.checkpoint_stance();
    let Runtime { history, settings, .. } = &mut skater.respawn;
    let candidate = history.automatic(
        stance,
        &mut Scene {
            world: &physics.world,
            settings,
            host,
        },
    )?;
    // 825926C0..E8: SpawnFootForwardStance, then the same Actor teleport (vtable +0x70,
    // 0x825926f8, no adjust) a marker target takes; 82593640 and 82DB5BE0 copy it raw to the
    // reset. The port converts Actor targets once, after the stance restore (host_teleport).
    skater.animation.restore_foot_forward(candidate.stance != 0);
    let mut transform = candidate.transform;
    if !candidate.offboard && skater.animation.stance().1 {
        flip(&mut transform);
    }
    skater.teleport_state.reply(Checkpoint {
        transform,
        on_board: !candidate.offboard,
    });
    crate::skate3_native::trace::info!(
        "BAIL_CHECKPOINT position={:?} heading={:?} stance={} offboard={} score={} entries={}",
        candidate.transform[3],
        candidate.transform[2],
        candidate.stance,
        candidate.offboard,
        candidate.score,
        skater.respawn.history.entries().len()
    );
    Ok(())
}

/// The Actor's side of the tick, in its order: the Teleporting callback's reply (82592518), then
/// PostUpdate's breadcrumb Update (82BFB1E0) over this tick's output.
pub(super) fn post_update(physics: &Skate3PhysicsState, skater: &mut Skate3ControllerState, mut host: Option<&mut (dyn RespawnHost + '_)>) -> Result<(), String> {
    if std::mem::take(&mut skater.respawn.requested) {
        answer(physics, skater, host.as_deref_mut())?;
    }
    if let Some(mut observation) = skater.respawn.observation.take() {
        observation.alternate_world = match host.as_deref_mut() {
            Some(host) => host.network_game()?,
            None => false,
        };
        let Runtime { history, settings, .. } = &mut skater.respawn;
        let before = (history.entries().len(), history.entries().last().copied());
        history.observe(
            &observation,
            settings.minimum_frames,
            &mut Scene {
                world: &physics.world,
                settings,
                host,
            },
        )?;
        // SKATE2_TRACE_SKATE3_BREADCRUMBS: each entry the PostUpdate records.
        if (history.entries().len(), history.entries().last().copied()) != before
            && std::env::var_os("SKATE2_TRACE_SKATE3_BREADCRUMBS").is_some()
        {
            let entry = history.entries().last().expect("recorded");
            eprintln!(
                "skate3 breadcrumb: dropped position={:?} heading={:?} stance={} offboard={} score={} entries={}",
                entry.transform[3], entry.transform[2], entry.stance, entry.offboard, entry.score, history.entries().len()
            );
        }
    }
    Ok(())
}

/// A tick the host did not follow with its Actor update (a standalone run): answer with no host.
pub(super) fn drain_unanswered(physics: &Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    if skater.respawn.requested || skater.respawn.observation.is_some() {
        post_update(physics, skater, None)?;
    }
    Ok(())
}

/// GetSuggestedRespawnPosition82BFC828, the history unchanged.
pub(super) fn suggested(physics: &Skate3PhysicsState, skater: &Skate3ControllerState, host: &mut dyn RespawnHost) -> Result<Candidate, String> {
    let stance = skater.animation.checkpoint_stance();
    let runtime = &skater.respawn;
    runtime.history.suggested(
        stance,
        &mut Scene {
            world: &physics.world,
            settings: &runtime.settings,
            host: Some(host),
        },
    )
}

/// PopSuggestedRespawnPosition82BFC9B0 for a reset the source did not request (security capture's
/// TeleportSomewhereAppropriate).
pub(super) fn pop_suggested(physics: &Skate3PhysicsState, skater: &mut Skate3ControllerState, host: &mut dyn RespawnHost) -> Result<Candidate, String> {
    let stance = skater.animation.checkpoint_stance();
    let Runtime { history, settings, .. } = &mut skater.respawn;
    history.automatic(
        stance,
        &mut Scene {
            world: &physics.world,
            settings,
            host: Some(host),
        },
    )
}

/// IsPositionOk82BFAFF0: the ground test (its category8 off-board answer is discarded), then the
/// network game's collision query.
pub(super) fn position_ok(physics: &Skate3PhysicsState, skater: &Skate3ControllerState, transform: &Matrix, host: &mut dyn RespawnHost) -> Result<bool, String> {
    let mut scene = Scene {
        world: &physics.world,
        settings: &skater.respawn.settings,
        host: Some(host),
    };
    Ok(scene.ground(transform)?.is_some() && scene.location(transform)?)
}

///One completed physical output, including the empirical conditioner sample. The Actor's
///PostUpdate records it (post_update).
pub(super) fn observe(physics: &Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    let physical = &skater.player_input.physical;
    let processed = &skater.player_input.processed;
    //82592A00 -> SkateboardReckoning64 ->82C01BF8: solved deck, with bit20 flip.
    let mut deck = super::solve::deck_frame(&physics.board);
    if processed.flags_2468 & 0x0010_0000 != 0 {
        flip(&mut deck);
    }
    deck[3][1] += 0.2;
    let mut offboard = skater.animated_skeleton.roots.animation_to_world;
    if processed.flags_2476 & 4 != 0 {
        flip(&mut offboard);
    }
    let velocity = physical.skateboard.vector_80.map(f32::from_bits);
    let stance = skater.animation.checkpoint_stance();
    let runtime = &mut skater.respawn;
    runtime.measurements = runtime.measurements.wrapping_add(1);
    runtime.observation = Some(Observation {
        measurements: runtime.measurements,
        root_position: skater.animated_skeleton.roots.animation_to_world[3],
        com_position: physical.reckoning.vector_64.map(f32::from_bits),
        teleport_requested: physical.state.flag_69 != 0,
        physical_state: physical.state.state_16,
        state_frames: skater.player_state.state_count,
        ground_suppressed: processed.grind.flags_1516 & 0x0800_0000 != 0,
        offboard_correction: skater.biped_ground.controller.state.contact.active,
        ground_category: super::ground_runtime::active_surface(&physics.riding, &physics.board)
            & 0xffff,
        //82DB6EC0 publishes the same processed foot record to both fields.
        foot_categories: [(processed.left_surface_2596 >> 7) & 31; 2],
        riding_transform: heading(deck, velocity),
        //82591E30 returns immediately for offboard, before the velocity branch.
        offboard_transform: offboard,
        stance,
        //The Actor's IsNetworkGame, set by post_update.
        alternate_world: false,
    });
    Ok(())
}

fn flip(matrix: &mut Matrix) {
    for i in [0, 2] {
        matrix[i] = matrix[i].map(|v| -v);
    }
}
///82591E30 retains the source axes at low speed or near vertical travel.
fn heading(mut matrix: Matrix, velocity: [f32; 4]) -> Matrix {
    let square = velocity[0] * velocity[0] + velocity[1] * velocity[1] + velocity[2] * velocity[2];
    if square > 0.25 {
        let inv = square.sqrt().recip();
        let forward = [velocity[0] * inv, 0., velocity[2] * inv, 0.];
        if forward[0] * forward[0] + forward[2] * forward[2] > 0.9 {
            matrix[0] = [forward[2], 0., -forward[0], 0.];
            matrix[1] = [0., 1., 0., 0.];
            matrix[2] = forward;
        }
    }
    matrix
}

struct Scene<'a, 'h> {
    world: &'a BoardWorld,
    settings: &'a Settings,
    host: Option<&'a mut (dyn RespawnHost + 'h)>,
}
impl Validation for Scene<'_, '_> {
    type Error = String;
    fn ground(&mut self, transform: &Matrix) -> Result<Option<Ground>, String> {
        let mut start = transform[3];
        start[1] += 0.1;
        let mut end = start;
        end[1] -= 10.;
        let probe = |start, end, radius| Probe {
            start,
            end,
            radius,
        };
        //The only actor has matching identity0; canonical map groups remain active.
        let Some(hit) = contact_queries::query(self.world, probe(start, end, 0.), 0)? else {
            return Ok(None);
        };
        let category = (u32::from(hit.packed_surface) >> 7) & 31;
        if hit.geometry.normal.y < self.settings.normal_y
            || start[1] - hit.geometry.position.y > self.settings.drop
            || !surface_allowed(category)
        {
            return Ok(None);
        }
        let mut bottom = start;
        bottom[1] += self.settings.radius;
        let mut top = bottom;
        top[1] += self.settings.height;
        if contact_queries::query(self.world, probe(bottom, top, self.settings.radius), 0)?
            .is_some()
        {
            return Ok(None);
        }
        let p = hit.geometry.position;
        Ok(Some(Ground {
            position: [p.x, p.y, p.z, 0.],
            offboard: category == 8,
        }))
    }
    fn location(&mut self, transform: &Matrix) -> Result<bool, String> {
        //82BFBB48 asks the collision query only in a network game, 0.1m above the position.
        let Some(host) = self.host.as_deref_mut() else { return Ok(true) };
        let mut position = transform[3];
        position[1] += 0.1;
        host.network_position_ok(position)
    }
    fn occupants(&mut self, transform: &Matrix) -> Result<bool, String> {
        //82BFB928 queries the LivingWorldManager's DMOs, vehicles and pedestrians. The player and
        //board are not living-world entities.
        self.world.query_metadata().map_err(str::to_owned)?;
        let Some(host) = self.host.as_deref_mut() else { return Ok(true) };
        host.living_world_clear(
            transform[3],
            LivingWorldRadii {
                pedestrian: self.settings.pedestrian_radius,
                vehicle: self.settings.vehicle_radius,
                capsule_height: self.settings.height,
                capsule_radius: self.settings.radius,
            },
        )
    }
    fn edges(&mut self, transform: &Matrix) -> Result<bool, String> {
        let p = transform[3];
        let bounds = Bounds {
            min: Vector3::new(p[0] - 0.3, p[1] - 0.6, p[2] - 0.3),
            max: Vector3::new(p[0] + 0.3, p[1] + 0.6, p[2] + 0.3),
        };
        let metadata = self.world.query_metadata().map_err(str::to_owned)?;
        //82C1EAD8: authored static order, shared capacity40; no triangle diagonals.
        for edge in metadata
            .static_edges
            .iter()
            .filter(|e| e.local_bounds.overlaps(bounds))
            .take(40)
        {
            let start = [edge.start.x, edge.start.y, edge.start.z];
            let end = [edge.end.x, edge.end.y, edge.end.z];
            let delta: [f32; 3] = std::array::from_fn(|i| end[i] - start[i]);
            let square: f32 = delta.iter().map(|v| v * v).sum();
            let along: f32 = (0..3).map(|i| (p[i] - start[i]) * delta[i]).sum();
            let fraction = if square > 0. {
                (along / square).clamp(0., 1.)
            } else {
                0.
            };
            let distance: f32 = (0..3)
                .map(|i| (p[i] - start[i] - delta[i] * fraction).powi(2))
                .sum();
            if distance < 0.09 {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

#[cfg(any())]
mod tests {
    use super::*;
    use skate3_core::physics::{
        contact::RetailContactMaterial, skeleton_animation_record::IDENTITY,
    };
    #[test]
    fn real_scene_records_a_checkpoint_and_recovers_it_from_an_unsupported_position() {
        let world = super::super::ground::world(RetailContactMaterial {
            static_friction: 0.,
            dynamic_friction: 0.,
            restitution: 0.,
        });
        let settings = Settings {
            height: 0.4,
            radius: 0.5,
            drop: 1.,
            normal_y: 0.75,
            minimum_frames: 15,
            pedestrian_radius: 1.,
            vehicle_radius: 2.,
        };
        let mut scene = Scene {
            world: &world,
            settings: &settings,
            host: None,
        };
        let mut history = History::new(Candidate {
            transform: IDENTITY,
            stance: 0,
            offboard: false,
            score: 0.,
        });
        //The first conditioning samples clear the construction cooldown.
        assert!(!history.recording_due(119, [2., 0., 0., 0.]));
        let mut transform = IDENTITY;
        transform[3] = [2., 0.2, 0., 0.];
        history
            .observe(
                &Observation {
                    measurements: 120,
                    root_position: transform[3],
                    com_position: [1000., 0., 0., 0.],
                    teleport_requested: false,
                    physical_state: 100,
                    state_frames: 16,
                    ground_suppressed: false,
                    offboard_correction: false,
                    ground_category: 1,
                    foot_categories: [1; 2],
                    riding_transform: transform,
                    offboard_transform: IDENTITY,
                    stance: 1,
                    alternate_world: false,
                },
                15,
                &mut scene,
            )
            .unwrap();
        let selected = history.automatic(0, &mut scene).unwrap();
        assert_eq!(selected.transform, transform);
        assert_eq!(selected.stance, 1);
        //Successful historical entries are consumed: a second failure uses spawn.
        assert_eq!(
            history.automatic(0, &mut scene).unwrap().transform,
            IDENTITY
        );
    }
    #[test]
    fn native_heading_keeps_low_speed_and_vertical_axes() {
        let mut source = IDENTITY;
        source[0] = [0., 0., -1., 0.];
        source[2] = [1., 0., 0., 0.];
        assert_eq!(heading(source, [0., 0., 0.5, 0.]), source);
        assert_eq!(heading(source, [0., 5., 0.1, 0.]), source);
        let result = heading(source, [0., 0., 2., 0.]);
        assert_eq!(result[2], [0., 0., 1., 0.]);
        assert_eq!(result[3], source[3]);
    }
}
