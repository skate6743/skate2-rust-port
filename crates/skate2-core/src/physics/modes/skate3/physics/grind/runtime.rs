//! Host lifetime for the six native physical-state instances and chromosome.
use super::super::grind_chromosome;
use super::{Family, ManagerObservation, output, settings::Settings, state::State};
use skate3_core::{physics::filtered_state::GrindState, player::input_phase::GrindOutputFields};
use skate_data::collections::Collections;

pub(crate) struct Runtime {
    pub(super) states: [State; 6],
    pub(super) active: Option<Family>,
    pub(super) nonspecific_active: bool,
    pub(super) nonspecific_jumped: bool,
    pub(super) nonspecific_jump_velocity: [f32; 4],
    orientation_random: super::rng::OrientationRandom,
    pub(super) settings: Settings,
    pub(super) manager: Option<ManagerObservation>,
    /// Exact optional Wipeout output16/flag32 write for main's shared output
    /// owner. CURRENT does not yet expose that block in PhysicalPlayerInput.
    pub pending_wipeout_impulse: Option<[f32; 4]>,
    chromosome: grind_chromosome::Chromosome,
}
impl Runtime {
    pub(crate) fn adopt_mode_settings(&mut self, other: Self) { self.settings = other.settings; }
    pub fn load(data: &Collections) -> Result<Self, String> {
        Ok(Self {
            states: [State::new(); 6],
            active: None,
            nonspecific_active: false,
            nonspecific_jumped: false,
            nonspecific_jump_velocity: [0.; 4],
            orientation_random: super::rng::OrientationRandom::new(),
            settings: Settings::load(data)?,
            manager: None,
            pending_wipeout_impulse: None,
            chromosome: grind_chromosome::Chromosome::uninitialized(),
        })
    }
    pub fn active_name(&self) -> Option<&'static str> {
        Some(match self.active? {
            Family::FiftyFifty => "50-50",
            Family::Boardslide => "boardslide",
            Family::Tipslide => "tipslide",
            Family::FiveO => "5-0",
            Family::Backslash => "backslash",
            Family::Darkslide => "darkslide",
        })
    }
    pub(super) fn take_orientation_noise(&mut self) -> [u32; 3] {
        self.orientation_random.take_three()
    }
    /// Call immediately after the real input manager finishes its Post phase.
    pub fn observe(&mut self, manager: ManagerObservation) {
        self.manager = Some(manager);
    }

    /// Physical FillOut only. Selector owns grinding316; output reset owns all
    /// other templates. Active family comes from lifecycle, not manager selection.
    pub fn fill_physical(
        &self,
        manager: &ManagerObservation,
        out: &mut GrindOutputFields,
    ) -> Option<output::SideEffects> {
        self.active.map(|family| {
            output::fill_fields(family, self.states[family as usize].output, manager, out)
        })
    }

    /// Run once per completed output publication, including nongrind frames.
    /// Ground/Air history is not restricted to the six physical grind states.
    pub fn condition_outputs(
        &mut self,
        input: grind_chromosome::Input,
        out: &mut GrindOutputFields,
    ) -> Result<(), String> {
        let chromosome = &mut self.chromosome;
        //Native leaves saved-fakie144 unwritten. Host lifecycle explicitly seeds
        //only that byte from the first completed packet; later history wins.
        chromosome.initialize_host_fakie(input.fakie_155);
        if let Some(publication) = chromosome.update(input) {
            grind_chromosome::publish(publication, out);
        }
        Ok(())
    }

    ///82DE5588/82DE5608 initializes196 to0;82DE5BA0/82DE5E10 copies it.
    ///82DE5858 measures velocity/acceleration, not grind distance. No distance
    ///accumulator is present in the reviewed82DE56B0 update chain.
    pub fn last_grind_distance(&self) -> f32 {
        0.0
    }

    pub fn filtered_output(out: &GrindOutputFields) -> Result<GrindState, String> {
        Ok(GrindState {
            kind: out.words_136_140[0] as i32,
            scorable_id: out.scorable_id_152 as i32,
            name: out
                .animation_name_156
                .ok_or("Missing native grind name reset/publication")?,
            scoring_name: out
                .scoring_name_176
                .ok_or("Missing native scoring grind name reset/publication")?,
            on_front: out.flag_318 != 0,
            crouch: out.crouch_132,
            pathed_guid: out.spline_guids_224_232[0],
            local_guid: out.spline_guids_224_232[1],
        })
    }
}
