//! PC port addition (user request): a linked Skate 3 district's water spots. Skate 3 bails the
//! skater into a swim on water (`ProcessOutput`'s special-surface writer 0x82db8120: surface type
//! 12 sets the wipeout flag 65) and resets the skater at its sea (surface type 6, the teleport
//! flag 69). Without the swim, a district's water resets the skater as its sea does: the local
//! skater's last breadcrumb (`TeleportLocalToLastBreadcrumb` 0x828a93a8, as the NIS's), once the
//! ground under or over the skater is water.
use super::{SelectedLiveError, SelectedLiveSession, SelectedLiveStage as Stage};
use crate::skeleton_root_transforms::SkeletonRootVector;

/// The packed surface's type bits (`(surface >> 7) & 31`): water.
const WATER: u16 = 12;
/// Ticks after a reset before water resets the skater again.
const COOLDOWN_TICKS: u32 = 120;

fn error(e: impl std::fmt::Display) -> SelectedLiveError {
    SelectedLiveError::failed(Stage::SessionMarker, format!("district water: {e}"))
}

impl<T, C> SelectedLiveSession<T, C> {
    /// One completed tick's check; nothing runs outside a district.
    pub(super) fn try_district_water_reset(&mut self) -> Result<(), SelectedLiveError> {
        if !crate::skate3::district_active() || self.map_travel.is_some() {
            return Ok(());
        }
        static SCAN: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        if *SCAN.get_or_init(|| std::env::var_os("SKATE2_DIAGNOSTIC_SKATE3_WATER_SCAN").is_some()) {
            self.diagnostic_water_scan()?;
        }
        if self.district_water_cooldown > 0 {
            self.district_water_cooldown -= 1;
            return Ok(());
        }
        let Some(selected) = self.inner.player.as_ref().and_then(|player| player.selected.actor.as_deref()) else { return Ok(()) };
        let world = self.inner.world.as_ref().map(|owner| &owner.world);
        let Ok(position) = super::local_skater_position(world, selected) else { return Ok(()) };
        if !self.water_at(position)? {
            return Ok(());
        }
        let Some(crumb) = self.local_suggested_breadcrumb()? else { return Ok(()) };
        // A crumb left on the water would only send the skater back into it.
        if self.water_at(crumb.transform.columns()[3].xyz)? {
            return Ok(());
        }
        if std::env::var_os("SKATE2_TRACE_SKATE3_WATER").is_some() {
            eprintln!("district water at {position:?}: reset to {:?}", crumb.transform.columns()[3].xyz);
        }
        self.player_ui_queues()?.send_teleport(crumb.transform, !crumb.offboard);
        self.district_water_cooldown = COOLDOWN_TICKS;
        Ok(())
    }

    /// Diagnostics (`SKATE2_DIAGNOSTIC_SKATE3_WATER_SCAN`): 300 ticks into the first district, the
    /// water within 120 m of the skater (first hits of vertical lines on a 2 m grid), then a
    /// teleport onto the nearest water, so the reset above can be seen.
    fn diagnostic_water_scan(&mut self) -> Result<(), SelectedLiveError> {
        use std::sync::atomic::{AtomicU32, Ordering};
        static TICKS: AtomicU32 = AtomicU32::new(0);
        if TICKS.fetch_add(1, Ordering::Relaxed) != 300 {
            return Ok(());
        }
        let Some(selected) = self.inner.player.as_ref().and_then(|player| player.selected.actor.as_deref()) else { return Ok(()) };
        let world_owner = self.inner.world.as_ref().map(|owner| &owner.world);
        let Ok(origin) = super::local_skater_position(world_owner, selected) else { return Ok(()) };
        let hits = {
            let world = &self.inner.world.as_ref().ok_or_else(|| error("world absent"))?.world;
            let terrain = self.inner.terrain.as_ref().ok_or_else(|| error("terrain absent"))?;
            let ground = terrain.ground_pool_for(world).map_err(|e| error(format!("{e:?}")))?;
            let mut physical = world.lock().map_err(|_| error("world lock poisoned"))?;
            let mut numeric = crate::collision_line_numeric::HostCollisionLineNumeric;
            let mut hits = Vec::new();
            for i in -60_i16..=60 {
                for j in -60_i16..=60 {
                    let (x, z) = (origin[0] + f32::from(i) * 2.0, origin[2] + f32::from(j) * 2.0);
                    let start = SkeletonRootVector::known([x, origin[1] + 40.0, z, 1.0]);
                    let end = SkeletonRootVector::known([x, origin[1] - 40.0, z, 1.0]);
                    let input = crate::collision_line_collection::CollisionLineInput { start, end, fatness: 0.0 };
                    if let Some(hit) = physical
                        .try_world_line_test(ground, input, "PC district water scan", &mut numeric)
                        .map_err(|e| error(format!("{e:?}")))?
                        && (hit.surface_id >> 7) & 0x1f == WATER
                    {
                        hits.push(hit.position.xyz);
                    }
                }
            }
            hits
        };
        let distance = |p: &[f32; 3]| (p[0] - origin[0]).powi(2) + (p[2] - origin[2]).powi(2);
        let nearest = hits.iter().copied().min_by(|a, b| distance(a).total_cmp(&distance(b)));
        eprintln!("district water scan around {origin:?}: {} water points, nearest {nearest:?}", hits.len());
        let (Some(point), Some(crumb)) = (nearest, self.local_suggested_breadcrumb()?) else { return Ok(()) };
        let mut transform = crumb.transform;
        transform.set_column(3, SkeletonRootVector::known([point[0], point[1] + 0.5, point[2], 1.0]));
        self.player_ui_queues()?.send_teleport(transform, true);
        Ok(())
    }

    /// Whether the first surface a line from 2 m over `position` to 0.6 m under it meets is
    /// water: the skater rides on it or stands in it.
    fn water_at(&self, position: [f32; 3]) -> Result<bool, SelectedLiveError> {
        let world = &self.inner.world.as_ref().ok_or_else(|| error("world absent"))?.world;
        let terrain = self.inner.terrain.as_ref().ok_or_else(|| error("terrain absent"))?;
        let ground = terrain.ground_pool_for(world).map_err(|e| error(format!("{e:?}")))?;
        let mut physical = world.lock().map_err(|_| error("world lock poisoned"))?;
        let start = SkeletonRootVector::known([position[0], position[1] + 2.0, position[2], 1.0]);
        let end = SkeletonRootVector::known([position[0], position[1] - 0.6, position[2], 1.0]);
        let input = crate::collision_line_collection::CollisionLineInput { start, end, fatness: 0.0 };
        let mut numeric = crate::collision_line_numeric::HostCollisionLineNumeric;
        let hit = physical
            .try_world_line_test(ground, input, "PC district water", &mut numeric)
            .map_err(|e| error(format!("{e:?}")))?;
        Ok(hit.is_some_and(|hit| (hit.surface_id >> 7) & 0x1f == WATER))
    }
}
