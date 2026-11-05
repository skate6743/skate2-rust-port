//! Original SkateboardController's lifecycle, TU3 82D74DD8/82D75EA0.
//! This is the hand-held/retrieval controller. Normal riding stops it.
use crate::player::lifecycle::{SkateboardControllerActions, SkateboardControllerFields};

pub struct SkateboardController {
    /// The same mutable fields consumed by PhysicalPlayer state changes.
    pub fields: SkateboardControllerFields,
}
impl Default for SkateboardController {
    fn default() -> Self {
        Self::new()
    }
}
impl SkateboardController {
    pub const fn new() -> Self {
        //82D74EFC/4F00/4F04, after the retrieval-progress initializer.
        Self {
            fields: SkateboardControllerFields {
                word_444: 0,
                state_448: 0,
                system_on_452: false,
            },
        }
    }

    ///82D75EA0. PhysicalPlayer invokes this for requested states outside
    ///500..502. The active-held release remains the real LetGoOfSkateboard
    ///action; do not replace it with a transform reset or an empty callback.
    pub fn stop(&mut self, actions: &mut impl SkateboardControllerActions) {
        if !self.fields.system_on_452 {
            return;
        }
        let previous = self.fields.state_448;
        self.fields.word_444 = 0;
        if previous != 0 {
            actions.let_go_of_skateboard();
            self.fields.state_448 = 0;
        }
        self.fields.system_on_452 = false;
    }

    ///The player passes this to Skeleton::UpdatePostPhysics. Native868
    ///is the count of seven board parts in contact, not wheel count869.
    pub fn request_partial_ragdoll(&self, part_contact_count: u8) -> bool {
        self.fields.state_448 == 1 && part_contact_count != 0
    }
}
