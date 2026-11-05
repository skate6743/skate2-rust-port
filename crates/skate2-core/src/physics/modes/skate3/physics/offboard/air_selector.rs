//! Concrete BipedAir selector host over the canonical BoardWorld.
//! Parent owns the Air facade and phase schedule. Native completion is explicit,
//! so a synchronous fixed-step backend never adds an automatic frame delay.
mod settings;
#[cfg(any())]
mod tests;
use super::{
    contact_toolkit::StaticScene,
    ground_query::with_world_scene,
};
pub(crate) use settings::Settings;
use skate3_core::{
    air::trajectory::{Prediction, QueryResult},
    physics::board_world::BoardWorld,
    player::offboard::{
        air_launch::Packet,
        air_selector::{self as core, Context, Frame, Vector, ledge},
        ground_query::GroundQueryScene,
    },
};
pub(crate) struct AirSelector {
    pub core: core::Selector,
    pub settings: Settings,
    completed_launch: Option<Vec<QueryResult>>,
    completed_requery: Option<Prediction>,
}
impl AirSelector {
    pub(crate) fn reset(&mut self) {
        self.core = core::Selector::default();
        self.completed_launch = None;
        self.completed_requery = None;
    }

    pub(crate) fn new(settings: Settings) -> Self {
        Self {
            core: core::Selector::default(),
            settings,
            completed_launch: None,
            completed_requery: None,
        }
    }
    ///82D6CA58 submits; results stay staged until consume_launch at native Sync.
    pub(crate) fn launch(
        &mut self,
        world: &BoardWorld,
        packet: Packet,
        gravity: Vector,
        context: Context,
    ) -> Result<(), &'static str> {
        let mut next = self.core.clone();
        let requests = next
            .begin_launch(packet, gravity, self.settings.query)
            .inspect_err(|error| {
                crate::skate3_native::trace::error!(?packet, ?gravity, settings = ?self.settings.query, %error,
                "BipedAir launch producer rejected; no packet fields substituted");
            })?;
        let scene = StaticScene::new(world)?;
        let results = requests
            .into_iter()
            .map(|request| {
                scene.trajectory(request, context.matching_group_2952, core::MESH_REJECT_MASK)
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.core = next;
        self.completed_launch = Some(results);
        self.completed_requery = None;
        Ok(())
    }
    ///82D6CC50/D020/D4D0. The current authored level has no dynamic edge
    ///providers; static metadata remains mandatory and is never fabricated.
    pub(crate) fn consume_launch(
        &mut self,
        world: &BoardWorld,
        vehicles: &[(usize, skate_dynamics::SolidBody)],
        context: Context,
        deck_half_wheelbase: f32,
    ) -> Result<Option<usize>, &'static str> {
        let Some(results) = self.completed_launch.as_ref() else {
            return Ok(None);
        };
        if !self.core.sampling.pending_8492 {
            return Ok(None);
        }
        let mut next = self.core.clone();
        next.observe_launch(results)?;
        let first = next.candidates[0];
        let prediction = next.predictions[0];
        let adjustment = if let Some(search) = ledge::search(first, prediction, context) {
            let cache = super::mod_solid_ground::VehicleEdgeCache::build(vehicles);
            let edges = cache.with_primary_edges(|vehicle_edges| {
                with_world_scene(
                    world,
                    vehicle_edges,
                    &[],
                    |scene| scene.edge_candidates(&search),
                )
            })?;
            let filtered = ledge::filter_edges(&edges, first.trajectory.position);
            if let Some(adjustment) = ledge::choose(
                first,
                prediction,
                context,
                self.settings.query.sphere_radius,
                &filtered,
            ) {
                if !deck_half_wheelbase.is_finite() || deck_half_wheelbase <= 0. {
                    return Err("BipedAir ledge requires actual positive board half-wheelbase");
                }
                if let Some(lines) = ledge::lines(adjustment, deck_half_wheelbase) {
                    let hits =
                        StaticScene::new(world)?.lines(&lines, context.matching_group_2952)?;
                    ledge::consume_lines(&hits)?;
                    Some(adjustment)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        let index = next.select_launch(context, first, adjustment)?;
        self.core = next;
        self.completed_launch = None;
        Ok(Some(index))
    }
    pub(crate) fn adjust_animation(&mut self, frame: i32, animation: Vector, axes: Frame) {
        self.core
            .adjust_animation(frame, animation, axes, self.settings.query.sphere_radius);
    }
    pub(crate) fn sample(&mut self, frame: i32, dt: f32, out: &mut core::TrajectoryResult) {
        self.core.sample(frame, dt, &self.settings.blend, out);
    }
    ///82D6E798 permits both pending8492 and8499. Its destination is the same
    ///query slot0, so the later requery replaces that slot before consumption.
    pub(crate) fn requery(
        &mut self,
        world: &BoardWorld,
        context: Context,
        flags_2472: u32,
        flags_2488: u32,
    ) -> Result<bool, &'static str> {
        let mut next = self.core.clone();
        let Some(request) =
            next.begin_requery(flags_2472, flags_2488, self.settings.query.sphere_radius)?
        else {
            return Ok(false);
        };
        let result = StaticScene::new(world)?.trajectory(
            request,
            context.matching_group_2952,
            core::MESH_REJECT_MASK,
        )?;
        self.core = next;
        self.completed_requery = Some(Prediction { request, result });
        if let Some(results) = self.completed_launch.as_mut() {
            if let Some(first) = results.first_mut() {
                *first = result;
            }
        }
        if let Some(first) = self.core.predictions.first_mut() {
            *first = Prediction { request, result };
        }
        Ok(true)
    }
    pub(crate) fn consume_requery(&mut self) -> Result<bool, &'static str> {
        let Some(prediction) = self.completed_requery else {
            return Ok(false);
        };
        //CC50 precedesE8A0 and may shift slot0's trajectory/contact time.
        let prediction = self.core.predictions.first().copied().unwrap_or(prediction);
        self.core.complete_requery(prediction)?;
        self.completed_requery = None;
        Ok(true)
    }
    ///82DB3C78: clear ready before launch/requery consumption, then rearm.
    ///Call at the original selector consume phase; never use elapsed frames as
    ///a synthetic readiness condition for the synchronous world backend.
    pub(crate) fn consume(
        &mut self,
        world: &BoardWorld,
        vehicles: &[(usize, skate_dynamics::SolidBody)],
        context: Context,
        deck_half_wheelbase: f32,
    ) -> Result<Option<usize>, &'static str> {
        self.core.sampling.preinitialized_8494 = false;
        let selected = if self.core.sampling.pending_8492 {
            self.consume_launch(world, vehicles, context, deck_half_wheelbase)?
        } else {
            None
        };
        if self.core.requery_pending_8499 {
            self.consume_requery()?;
        }
        self.core.sampling.restart_allowed_8493 = true;
        Ok(selected)
    }
    pub(crate) fn exit(&mut self) {
        self.core.exit();
    }
}
