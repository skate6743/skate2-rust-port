//! Persistent SkeletonIK state and the GeneralUpdate82BDCA38 stage order.
use super::{
    blend,
    contact::{ContactInput, ContactState},
    drive::{self, Geometry},
    external::{self, ExternalTarget},
    settings::Settings,
    status::{self, LimbStatus, Mode},
    transforms::{self, LimbBinding, LimbFrames},
};
use crate::physics::skeleton_animation_record::AnimationPartTransform as Transform;

///82BED688 binds the toe targets to foot/toe volumes and the hand targets to
///their hand volumes. The optional second volume is distinct from chain parents.
pub const LIMBS: [LimbBinding; 4] = [
    LimbBinding {
        part: 15,
        parent_part: Some(16),
    },
    LimbBinding {
        part: 19,
        parent_part: Some(20),
    },
    LimbBinding {
        part: 3,
        parent_part: None,
    },
    LimbBinding {
        part: 7,
        parent_part: None,
    },
];

#[derive(Clone, Debug)]
pub struct FootIkState {
    pub limbs: [LimbStatus; 4],
    pub frames: [LimbFrames; 4],
    pub external_targets: [ExternalTarget; 4],
    pub contacts: ContactState,
    feet_enabled: bool,
}
impl Default for FootIkState {
    fn default() -> Self {
        //82BED780 resets the retained histories and enables feet; it is not
        //the wipeout-entry solve82BF1B90, which disables feet after its update.
        Self {
            limbs: [LimbStatus::default(); 4],
            frames: [LimbFrames::default(); 4],
            external_targets: [ExternalTarget::default(); 4],
            contacts: ContactState::default(),
            feet_enabled: true,
        }
    }
}

pub struct UpdateInput<'a> {
    pub flags_2468: u32,
    pub flags_2472: u32,
    pub flags_2480: u32,
    /// DataIn32/96; both are actual Skeleton root transforms.
    pub animation_to_world: &'a Transform,
    pub world_to_animation: &'a Transform,
    /// DataIn160, from Skeleton12496. This is independently produced from
    ///DataIn352 (Skeleton12304) used by foot contact correction.
    pub physical_board: &'a Transform,
    pub contact_board: &'a Transform,
    pub inverse_contact_board: &'a Transform,
    /// SkeletonData animation record: physical volume frames after board offset.
    pub animation: &'a [Transform; 24],
    /// ProcessData14160: original selected globals, before volume composition.
    pub original_animation: &'a [Transform; 24],
    /// Actual board-parented targets after their type5 volume composition.
    pub targets: &'a [Transform; 4],
    pub toe_contact_normal: [f32; 4],
    pub contact_bone: usize,
    pub foot_bones: [usize; 2],
    pub current_contacts: [Option<[f32; 4]>; 2],
}

impl FootIkState {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Ground::Enter writes (*(Skeleton+16432))+464 directly.
    pub fn enable_feet(&mut self, enabled: bool) {
        self.feet_enabled = enabled;
    }
    pub fn feet_enabled(&self) -> bool {
        self.feet_enabled
    }

    /// Ground collision handling can also set the per-update3185 flag.
    pub fn mark_support_failed_this_update(&mut self) {
        self.contacts.support_failed_this_update = true;
    }

    /// The mutable output is Skeleton12624 consumed by SkeletonDrives. The
    ///original animation and collision-volume record must remain separate.
    pub fn update(
        &mut self,
        input: UpdateInput<'_>,
        geometry: &Geometry,
        settings: &Settings,
        drives: &mut [Transform; 24],
    ) -> [bool; 4] {
        for limb in 0..4 {
            self.frames[limb].target = input.targets[limb];
        }
        status::update_modes(&mut self.limbs, self.feet_enabled, input.flags_2472);
        status::update_blends(
            &mut self.limbs,
            settings.deck_half_width,
            settings.deck_total_half_length,
            input.flags_2480,
            &settings.blend,
        );
        let inverse_animation_board = transforms::inverse_rigid(&input.animation[0]);
        for limb in 0..4 {
            self.frames[limb].within_contact_bounds = false;
            if matches!(self.limbs[limb].mode, Mode::External | Mode::Local) {
                external::update(
                    &mut self.external_targets[limb],
                    &mut self.limbs[limb],
                    &mut self.frames[limb],
                    LIMBS[limb],
                    input.animation,
                    input.animation_to_world,
                    input.world_to_animation,
                );
            }
            transforms::prepare_animation_target(
                &mut self.limbs[limb],
                &mut self.frames[limb],
                LIMBS[limb],
                input.animation,
                input.animation_to_world,
                &inverse_animation_board,
                settings.contact_bounds,
            );
        }
        blend::update(
            &self.limbs,
            &mut self.frames,
            &LIMBS,
            input.physical_board,
            input.animation_to_world,
            &input.animation[0],
        );
        self.contacts.update(
            &self.limbs,
            &mut self.frames,
            ContactInput {
                flags_2468: input.flags_2468,
                contact_bone: input.contact_bone,
                foot_bones: input.foot_bones,
                toe_contact_normal: input.toe_contact_normal,
                inverse_board: input.inverse_contact_board,
                board: input.contact_board,
                current_contacts: input.current_contacts,
            },
        );
        drive::update(
            &self.limbs,
            &mut self.frames,
            &LIMBS,
            geometry,
            input.original_animation,
            drives,
            input.world_to_animation,
            settings.angle_limits,
        )
    }
}
