//! PC port of TU3's shove on a pedestrian (Skate 3 physics): the host's answer to the probe's
//! pedestrian query (`LivingWorld::Manager` vslot 0x44, 0x826bed80) and the game's response to a
//! strike (0x8275f0d8 -> `Pedestrian` 0x82e38cc0), on Skate 2's living world.
use super::PedestrianBodyKind;
use crate::pedestrian_graph::colliding::{CollisionDirection, CollisionForce, PedestrianCollisionInfo};
use crate::physical_player_assembly::PhysicalAssemblyWorld;
use crate::reckoning_numeric::HostReckoningNumeric;
use crate::skate3_native::physics::shove_probe::{ShoveHit, ShovePedestrian};
use crate::skeleton_root_transforms::SkeletonRootVector;
use std::f32::consts::{PI, TAU};

/// 0x826bed80's `GetPedestriansWithinRange`: the folded constants 2.0 and 1.5, at most six.
const QUERY_RADIUS: f32 = 3.5;
const QUERY_MAX: u32 = 6;

fn trace(message: std::fmt::Arguments<'_>) {
    static TRACE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if *TRACE.get_or_init(|| std::env::var_os("SKATE2_TRACE_SHOVE").is_some()) {
        eprintln!("shove: {message}");
    }
}

/// The pedestrians within the query's range of `point` (XZ), in the KD tree's order, with what
/// 0x826bed80 tests of each.
pub(crate) fn pedestrians(world: &PhysicalAssemblyWorld, point: [f32; 3]) -> Result<Vec<ShovePedestrian>, String> {
    let owner = world.grind_services.living_world();
    let living = owner.lock().map_err(|_| "shove: living world lock poisoned")?;
    let Some(roster) = living.pedestrian_entities.as_deref() else { return Ok(Vec::new()) };
    let Some(spatial) = roster.spatial() else { return Ok(Vec::new()) };
    let near = spatial
        .find_near_numeric([point[0], point[2]], QUERY_RADIUS, QUERY_MAX, &mut HostReckoningNumeric)
        .map_err(|e| format!("shove pedestrian query: {e:?}"))?;
    trace(format_args!("query at {point:?}: {} near", near.len()));
    let mut found = Vec::with_capacity(near.len());
    for (id, _) in near {
        let Ok(id) = u32::try_from(id) else { continue };
        let Some(entity) = roster.get(id) else { continue };
        let (Some(core), Some(state), Some(collision)) = (entity.core.as_ref(), entity.state.as_ref(), entity.collision.as_ref())
        else {
            continue;
        };
        let Ok(part) = collision.skater_part_transform(0, world) else { continue };
        found.push(ShovePedestrian {
            id,
            position: core.transform[3],
            velocity: core.velocity,
            wiped_out: state.flags[1] & 4 != 0,
            part: std::array::from_fn(|row| std::array::from_fn(|lane| part[row * 4 + lane])),
            dimensions: collision.model.as_ref().map_or([0.0; 4], |model| model.dimensions),
        });
    }
    Ok(found)
}

/// 0x82e38cc0: the shoved pedestrian goes down from the side the shove came from, its collision
/// takes the skater's knockdown, and both collision bodies leave with the shove's velocity
/// (`Body::SetVelocity`, the skater body first).
pub(crate) fn apply(world: &mut PhysicalAssemblyWorld, hit: ShoveHit) -> Result<(), String> {
    let owner = std::sync::Arc::clone(world.grind_services.living_world());
    let mut living = owner.lock().map_err(|_| "shove: living world lock poisoned")?;
    let Some(roster) = living.pedestrian_entities.as_deref_mut() else { return Ok(()) };
    let Some(entity) = roster.get_mut(hit.id) else { return Ok(()) };
    let Some(core) = entity.core.as_ref() else { return Ok(()) };
    let direction = side(core.transform[2], hit.velocity);
    trace(format_args!("pedestrian {} knocked down from {direction:?} at {:?}", hit.id, hit.velocity));
    let Some(collision) = entity.collision.as_mut() else { return Ok(()) };
    collision.colliding_with_skater = true;
    collision.required_to_begin_colliding = true;
    entity.collision_info = PedestrianCollisionInfo { force: CollisionForce::Knockdown, direction };
    let velocity = [hit.velocity[0], hit.velocity[1], hit.velocity[2]];
    for kind in [PedestrianBodyKind::Skater, PedestrianBodyKind::World] {
        let handle = entity.collision.as_ref().and_then(|collision| collision.attached_handle(kind)).cloned();
        if let Some(handle) = handle {
            world.try_set_pedestrian_collision_part_velocity(&handle, 0, velocity)?;
        }
    }
    Ok(())
}

/// 0x82e38dd8: the shove's angle from the pedestrian's facing about world up, wrapped to a half
/// turn either way; within 55 degrees of the facing it comes from behind, within 55 of the
/// opposite from the front, else from the right (positive) or left.
fn side(forward: [f32; 4], shove: [f32; 4]) -> CollisionDirection {
    let squared = forward[2].mul_add(forward[2], forward[1].mul_add(forward[1], forward[0] * forward[0]));
    let scale = squared.sqrt().recip();
    let forward = SkeletonRootVector::known(forward.map(|lane| lane * scale));
    let up = SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]);
    let angle = crate::physical_player_phys_out::construct::skeleton_fill_suffix::plane_angle(
        forward,
        SkeletonRootVector::known(shove),
        up,
        &mut HostReckoningNumeric,
    )
    .unwrap_or(0.0);
    let turns = angle * TAU.recip();
    let fraction = turns - turns.floor();
    let angle = (fraction - if fraction > 0.5 { 1.0 } else { 0.0 }) * TAU;
    let limit = 55.0_f32 * 0.017_453_292;
    if !(angle.abs() >= limit) {
        CollisionDirection::Back
    } else if !(angle.abs() <= PI - limit) {
        CollisionDirection::Front
    } else if angle > 0.0 {
        CollisionDirection::Right
    } else {
        CollisionDirection::Left
    }
}
