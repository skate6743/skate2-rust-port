//! Host manager82D8A828/82D8AB08, using CURRENT player-input ownership.
//! Static authored-world scope. Gameplay leaves live in skate-core; world
//! filtering, primitive provenance and board material application stay explicit.
mod history;
mod post;
mod pre;
mod publication;
mod settings;
#[cfg(any())]
mod tests;
pub(crate) mod world;

use crate::skate3_native::{grind_world::StaticProvider, physics::grind::ManagerObservation};
use skate3_core::{
    air::trajectory::grind_surface::{self, GrindSurface, Probe, ProbeHit},
    physics::{
        board_world::BoardWorld,
        grind_contact::{balance, control, entry, manager},
    },
    player::input_phase::{GrindInvestigationFields, ProcessedPhysicsInput},
};
use skate_data::collections::Collections;
type V = [f32; 4];

/// These are real physical/animation producers omitted from the old shared
/// input subset. There are deliberately no guessed defaults.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PreContext {
    /// Physical Processed64..127, not effective animation transform192.
    pub board: [V; 4],
    pub air_counter: i32,
    pub tip_state: u32,
    pub air_targeting_grind_9653: bool,
    pub balance_2720: f32,
    pub translation_2796: f32,
    pub stability_nudge_2800: f32,
    #[cfg(any())]
    pub up_down_2804: f32,
    #[cfg(any())]
    pub grab_min_height_2808: f32,
}

///Fresh physical/animation values read by82D8AB08 after intervening producers.
///Candidate selection and submitted geometry hits remain cached in Pending.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PostContext {
    ///Current physical Processed64..127, not animation transform192.
    pub board: [V; 4],
    pub balance_2720: f32,
    pub translation_2796: f32,
    pub stability_nudge_2800: f32,
    pub up_down_2804: f32,
    pub grab_min_height_2808: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MaterialMode {
    ///82D89DC0 leaves materials and its air-target history untouched.
    Unchanged,
    ///Restore the initialized standard material triplet (82D89DC0).
    ///Not the broader SetStandard routine's unrelated body/volume writes.
    Standard,
    ///All parts group4; wheels board8316, deck8328, trucks8340.
    Grind,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct PrimitiveMetadata {
    pub spline_guids: Option<[u64; 2]>,
}

/// Callbacks execute the original grind line-batch contract against current
/// world metadata (see world), not the contact-fatness BoardWorld line query.
/// Query errors propagate, never become misses. The host owns actual body and
/// material tables; a material mode must be applied before this call returns.
pub(crate) trait Host {
    fn apply_material_mode(&mut self, mode: MaterialMode) -> Result<(), String>;
    fn surface_probe(
        &mut self,
        world: &BoardWorld,
        actor: [u32; 2],
        index: usize,
        probe: Probe,
    ) -> Result<Option<ProbeHit>, String>;
    fn force_exit_line(
        &mut self,
        world: &BoardWorld,
        actor: [u32; 2],
        probe: balance::ForceExitProbe,
    ) -> Result<Option<balance::ForceExitHit>, String>;
}

/// Not Clone: the caller consumes one pre result in one post phase.
pub(crate) struct Pending {
    fields: GrindInvestigationFields,
    geometry: Option<GeometryWork>,
    metadata: Option<PrimitiveMetadata>,
}
struct GeometryWork {
    input: grind_surface::InvestigationInput,
    plan: Option<grind_surface::Investigation>,
    hits: [Option<ProbeHit>; 7],
}

pub(crate) struct PostResult {
    /// One entry per native Request call, including two simultaneous impacts.
    /// Main delivers these through its actual wipeout manager (which updates
    /// reason flags, timers and request count); processed bit18 is set here.
    pub wipeout_reasons: Vec<usize>,
    pub observation: ManagerObservation,
}

pub(crate) struct GrindInputState {
    previous_state: u32,
    engagement_counter: u32,
    cooldown: u32,
    pub disabled: bool,
    pub suppressed: bool,
    pub elapsed: f32,
    pub friction_vs_time: f32,
    pub previous_velocity: [u32; 4],
    pub grind_history: u32,
    pub secondary_history: u32,
    pub grounded_frames: u32,
    pub air_frames: u32,
    pub low_wheel_frames: u32,
    previous_proximity: bool,
    previous_air_target: bool,
    previous_direction: V,
    gravity_timer: f32,
    pub balance: balance::BalanceState,
    pub engagement: entry::Engagement,
    pub control: control::Control,
    /// Physical pop consumes this same cache/energy/cooldown, not a copy.
    pub jumper: manager::Jumper,
    pub investigation: GrindInvestigationFields,
    settings: settings::Settings,
}

impl GrindInputState {
    pub fn load(data: &Collections) -> Result<Self, String> {
        Ok(Self {
            previous_state: 0,
            engagement_counter: 0,
            cooldown: 0,
            disabled: false,
            suppressed: false,
            elapsed: 0.,
            friction_vs_time: 0.,
            previous_velocity: [0; 4],
            grind_history: 0,
            secondary_history: 0,
            grounded_frames: 0,
            air_frames: 0,
            low_wheel_frames: 0,
            previous_proximity: false,
            previous_air_target: false,
            previous_direction: [0.; 4],
            gravity_timer: 0.,
            balance: balance::BalanceState::default(),
            engagement: entry::Engagement::default(),
            control: control::Control::default(),
            jumper: manager::Jumper::default(),
            investigation: GrindInvestigationFields::default(),
            settings: settings::Settings::load(data)?,
        })
    }

    ///82D8A638 resets the manager, not the separately constructed child objects.
    #[cfg(any())]
    pub fn reset(&mut self) {
        self.previous_state = 0;
        self.engagement_counter = 0;
        self.cooldown = 0;
        self.disabled = false;
        self.suppressed = false;
        self.elapsed = 0.;
        self.gravity_timer = 0.;
        self.friction_vs_time = 0.;
        self.previous_direction = [0.; 4];
        self.investigation = GrindInvestigationFields::default();
    }
}

fn float(v: [u32; 4]) -> V {
    v.map(f32::from_bits)
}
fn raw(v: V) -> [u32; 4] {
    v.map(f32::to_bits)
}

impl core::fmt::Debug for GrindInputState {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("GrindInputState")
            .field("disabled", &self.disabled)
            .field("suppressed", &self.suppressed)
            .field("investigation", &self.investigation)
            .field("balance", &self.balance)
            .field("engagement", &self.engagement)
            .field("jumper", &self.jumper)
            .finish_non_exhaustive()
    }
}
