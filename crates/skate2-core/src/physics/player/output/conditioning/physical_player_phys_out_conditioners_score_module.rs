//! Complete nested `ScoreModule` constructor82dfb508 and Reset82df26f0.
use super::{
    super::{PhysOutComponent, PhysicalPlayerPhysOut, SkeletonRootVector, ZERO},
    AllocationSite, ConstructionError, Retained,
    air_collector::AirCollector,
    collector_base::{CollectorBase, MomentumMeter, ScoringMeter},
    collectors::{GrindCollector, GroundCollector, HandplantCollector, OffboardCollector},
    score_holder::{MissingSequence, ScoreHolder},
    sequence::GlobalMetrics,
    wipeout_score::{DmoScorer, EntityScorer, WipeoutScorer},
};
use crate::attribute_runtime::{AttributeRuntimeVault};
use super::scoring_attributes::ScoringAttributes;
use std::sync::{Arc, Mutex};

#[expect(
    clippy::struct_excessive_bools,
    reason = "independent ScoreModule reset flags"
)]
pub(super) struct ResetSuffix {
    pub(super) raw_scorable_id: i32,
    pub(super) collector_type: i32,
    pub(super) position_last_frame: SkeletonRootVector,
    pub(super) num_frames_ground_sequence_maintains_line: u32,
    pub(super) current_gesture: i32,
    pub(super) num_frames_since_last_gesture: u32,
    pub(super) num_frames_left_to_remember_requested_runout: u32,
    pub(super) just_entered_wipeout: bool,
    pub(super) reset_sequence: bool,
    pub(super) is_multiplier_active: bool,
    pub(super) skater_has_been_on_ground: bool,
    pub(super) just_performed_gesture: bool,
    pub(super) was_offboard_and_not_hippy_jumping: bool,
    pub(super) was_onboard_or_hippy_jumping: bool,
    pub(super) was_hippy_jumping: bool,
}
pub(super) enum CurrentCollector {
    Ground,
    Air,
    Grind,
    Handplant,
    Offboard,
    Other(Arc<Mutex<CollectorBase>>),
}
pub(super) struct ScoreModule {
    pub(super) output: Arc<()>,
    pub(super) current_collector: Option<CurrentCollector>,
    pub(super) air: Retained<Box<AirCollector>>,
    pub(super) grind: Retained<Box<GrindCollector>>,
    pub(super) ground: Retained<Box<GroundCollector>>,
    pub(super) handplant: Retained<Box<HandplantCollector>>,
    pub(super) offboard: Retained<Box<OffboardCollector>>,
    pub(super) other: Retained<Arc<Mutex<CollectorBase>>>,
    pub(super) metrics: Retained<Arc<Mutex<GlobalMetrics>>>,
    pub(super) holder: Retained<Arc<Mutex<ScoreHolder>>>,
    pub(super) line_timer: Retained<Box<ScoringMeter>>,
    pub(super) momentum: Retained<Box<MomentumMeter>>,
    pub(super) wipeout: Retained<Box<WipeoutScorer>>,
    pub(super) states: ScoringAttributes,
    pub(super) general: ScoringAttributes,
    // PhysicsScoring allocation is not cleared. The original Reset is the first
    // writer of this group, after every nested constructor has returned.
    pub(super) reset_suffix: Option<ResetSuffix>,
}
impl ScoreModule {
    #[cfg(feature = "skate3-runtime")]
    pub(super) fn select_skate3(&self, enabled: bool) -> Result<(), super::scoring_attributes::ScoreError> {
        use super::scoring_attributes::ScoreError as E;
        self.states.select_skate3(enabled);
        self.general.select_skate3(enabled);
        let select = |base: &CollectorBase| {
            base.states.select_skate3(enabled); base.general.select_skate3(enabled);
        };
        if let Some(c) = self.air.published_value() { select(&c.base); }
        if let Some(c) = self.grind.published_value() { select(&c.base); }
        if let Some(c) = self.ground.published_value() { select(&c.base); }
        if let Some(c) = self.handplant.published_value() { select(&c.base); }
        if let Some(c) = self.offboard.published_value() { select(&c.base); }
        if let Some(c) = self.other.published_value() { select(&*c.lock().map_err(|_| E::Poisoned)?); }
        if let Some(c) = self.holder.published_value() { c.lock().map_err(|_| E::Poisoned)?.general.select_skate3(enabled); }
        if let Some(c) = self.line_timer.published_value() { c.general.select_skate3(enabled); }
        if let Some(c) = self.momentum.published_value() { c.base.general.select_skate3(enabled); }
        if let Some(c) = self.wipeout.published_value() { c.attributes.select_skate3(enabled); }
        Ok(())
    }

    pub(super) fn before_children(output: Arc<()>, vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self {
            output,
            current_collector: None,
            air: Retained::new(),
            grind: Retained::new(),
            ground: Retained::new(),
            handplant: Retained::new(),
            offboard: Retained::new(),
            other: Retained::new(),
            metrics: Retained::new(),
            holder: Retained::new(),
            line_timer: Retained::new(),
            momentum: Retained::new(),
            wipeout: Retained::new(),
            states: ScoringAttributes::new(vault.instance(0x546c_36b6_5603_8e04, 0xd7ed_bd36_2d7d_2152), 0x546c_36b6_5603_8e04),
            general: ScoringAttributes::new(vault.instance(0x3492_15e2_e817_703c, 0xd7ed_bd36_2d7d_2152), 0x3492_15e2_e817_703c),
            reset_suffix: None,
        }
    }
    fn base(&self, vault: &Arc<AttributeRuntimeVault>) -> CollectorBase {
        CollectorBase::new(
            self.output.clone(),
            self.metrics.published_value().cloned(),
            self.holder.published_value().cloned(),
            vault,
        )
    }
    pub(super) fn construct<E>(
        &mut self,
        output: &mut PhysicalPlayerPhysOut,
        vault: &Arc<AttributeRuntimeVault>,
        allocate: &mut impl FnMut(AllocationSite) -> Result<bool, E>,
    ) -> Result<(), ConstructionError<E>> {
        if allocate(AllocationSite::GlobalMetrics).map_err(ConstructionError::Allocation)? {
            self.metrics.value = Some(Arc::new(Mutex::new(GlobalMetrics::new(
                self.output.clone(),
            ))));
        }
        self.metrics.published = true;
        if allocate(AllocationSite::ScoreHolder).map_err(ConstructionError::Allocation)? {
            self.holder.value = Some(Arc::new(Mutex::new(ScoreHolder::before_sequence(
                self.output.clone(),
                vault,
            ))));
            let mut holder = self
                .holder
                .value
                .as_ref()
                .expect("retained holder")
                .lock()
                .map_err(|_| ConstructionError::Lock)?;
            if allocate(AllocationSite::Sequence).map_err(ConstructionError::Allocation)? {
                holder.construct_sequence(self.metrics.published_value().cloned());
            }
            holder
                .finish_constructor()
                .map_err(|MissingSequence| ConstructionError::Missing(AllocationSite::Sequence))?;
        }
        self.holder.published = true;
        if allocate(AllocationSite::AirCollector).map_err(ConstructionError::Allocation)? {
            self.air.value = Some(Box::new(AirCollector::before_reset_sim(self.base(vault))));
            self.air
                .value
                .as_mut()
                .expect("retained air")
                .reset(output)
                .map_err(ConstructionError::Air)?;
        }
        self.air.published = true;
        if allocate(AllocationSite::GrindCollector).map_err(ConstructionError::Allocation)? {
            self.grind.value = Some(Box::new(GrindCollector::new(self.base(vault))));
        }
        self.grind.published = true;
        if allocate(AllocationSite::GroundCollector).map_err(ConstructionError::Allocation)? {
            self.ground.value = Some(Box::new(GroundCollector::new(self.base(vault))));
        }
        self.ground.published = true;
        if allocate(AllocationSite::HandplantCollector).map_err(ConstructionError::Allocation)? {
            self.handplant.value = Some(Box::new(HandplantCollector::new(self.base(vault))));
        }
        self.handplant.published = true;
        if allocate(AllocationSite::OffboardCollector).map_err(ConstructionError::Allocation)? {
            self.offboard.value = Some(Box::new(OffboardCollector::new(self.base(vault))));
        }
        self.offboard.published = true;
        if allocate(AllocationSite::OtherCollector).map_err(ConstructionError::Allocation)? {
            self.other.value = Some(Arc::new(Mutex::new(self.base(vault))));
            self.other
                .value
                .as_mut()
                .expect("retained other")
                .lock()
                .map_err(|_| ConstructionError::Lock)?
                .reset_analysis();
        }
        self.other.published = true;
        if allocate(AllocationSite::LineTimer).map_err(ConstructionError::Allocation)? {
            self.line_timer.value = Some(Box::new(ScoringMeter::new(self.output.clone(), vault)));
        }
        self.line_timer.published = true;
        if allocate(AllocationSite::MomentumMeter).map_err(ConstructionError::Allocation)? {
            self.momentum.value = Some(Box::new(MomentumMeter::new(self.output.clone(), vault)));
        }
        self.momentum.published = true;
        if allocate(AllocationSite::WipeoutScorer).map_err(ConstructionError::Allocation)? {
            self.wipeout.value = Some(Box::new(WipeoutScorer::before_children(
                self.output.clone(),
                vault,
            )));
            let wipeout = self.wipeout.value.as_mut().expect("retained wipeout");
            if allocate(AllocationSite::PedestrianScorer).map_err(ConstructionError::Allocation)? {
                wipeout.pedestrian = Some(Box::new(EntityScorer::new(vault)));
            }
            if allocate(AllocationSite::VehicleScorer).map_err(ConstructionError::Allocation)? {
                wipeout.vehicle = Some(Box::new(EntityScorer::new(vault)));
            }
            if allocate(AllocationSite::DmoScorer).map_err(ConstructionError::Allocation)? {
                wipeout.dmo.value = Some(Box::new(DmoScorer::before_attribute(vault)));
                wipeout
                    .dmo
                    .value
                    .as_mut()
                    .expect("retained DMO scorer")
                    .finish_constructor()
                    .map_err(ConstructionError::Dmo)?;
            }
            wipeout.dmo.published = true;
        }
        self.wipeout.published = true;
        self.reset(output)
    }
    pub(super) fn reset<E>(
        &mut self,
        output: &mut PhysicalPlayerPhysOut,
    ) -> Result<(), ConstructionError<E>> {
        if !Arc::ptr_eq(&self.output, output.occurrence()) {
            return Err(ConstructionError::ForeignOutput);
        }
        self.holder
            .published_value()
            .ok_or(ConstructionError::Missing(AllocationSite::ScoreHolder))?
            .lock()
            .map_err(|_| ConstructionError::Lock)?
            .reset()
            .map_err(|MissingSequence| ConstructionError::Missing(AllocationSite::Sequence))?;
        self.metrics
            .published_value()
            .ok_or(ConstructionError::Missing(AllocationSite::GlobalMetrics))?
            .lock()
            .map_err(|_| ConstructionError::Lock)?
            .reset_contexts();
        self.air
            .published_value_mut()
            .ok_or(ConstructionError::Missing(AllocationSite::AirCollector))?
            .reset(output)
            .map_err(ConstructionError::Air)?;
        self.grind
            .published_value_mut()
            .ok_or(ConstructionError::Missing(AllocationSite::GrindCollector))?
            .reset();
        self.ground
            .published_value_mut()
            .ok_or(ConstructionError::Missing(AllocationSite::GroundCollector))?
            .reset();
        self.handplant
            .published_value_mut()
            .ok_or(ConstructionError::Missing(
                AllocationSite::HandplantCollector,
            ))?
            .reset();
        self.offboard
            .published_value_mut()
            .ok_or(ConstructionError::Missing(
                AllocationSite::OffboardCollector,
            ))?
            .reset();
        self.other
            .published_value_mut()
            .ok_or(ConstructionError::Missing(AllocationSite::OtherCollector))?
            .lock()
            .map_err(|_| ConstructionError::Lock)?
            .reset_analysis();
        self.line_timer
            .published_value_mut()
            .ok_or(ConstructionError::Missing(AllocationSite::LineTimer))?
            .reset();
        self.momentum
            .published_value_mut()
            .ok_or(ConstructionError::Missing(AllocationSite::MomentumMeter))?
            .reset();
        self.current_collector = Some(CurrentCollector::Other(
            self.other
                .published_value()
                .expect("previously checked OtherCollector")
                .clone(),
        ));
        let CurrentCollector::Other(current) = self
            .current_collector
            .as_ref()
            .expect("published current collector") else {
                unreachable!("Reset binds the existing OtherCollector")
            };
        current
            .lock()
            .map_err(|_| ConstructionError::Lock)?
            .reset_analysis();
        self.reset_suffix = Some(ResetSuffix {
            raw_scorable_id: -1,
            collector_type: 4,
            position_last_frame: ZERO,
            num_frames_ground_sequence_maintains_line: 0,
            current_gesture: 35,
            num_frames_since_last_gesture: 0,
            num_frames_left_to_remember_requested_runout: 0,
            just_entered_wipeout: false,
            reset_sequence: false,
            is_multiplier_active: false,
            skater_has_been_on_ground: false,
            just_performed_gesture: false,
            was_offboard_and_not_hippy_jumping: false,
            was_onboard_or_hippy_jumping: false,
            was_hippy_jumping: false,
        });
        output
            .check_current_guard()
            .map_err(ConstructionError::Access)?;
        output
            .concrete
            .scoring
            .as_mut()
            .ok_or(ConstructionError::MissingOutput(PhysOutComponent::Scoring2))?
            .has_been_reset = true;
        Ok(())
    }
}
