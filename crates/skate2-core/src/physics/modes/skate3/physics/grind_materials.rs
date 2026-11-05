//!82D89DC0/82C090C0 material modes, applied to the live board collider slots.
//! Init82C08C28 clears all three grind materials at82C09084..A4; S2 Init
//!82B374E8 confirms their identities. Grind resistance is a separate force.
use super::{player_input::grind::MaterialMode, settings::PhysicsSettings};
use skate3_core::physics::{board_runtime::BoardRuntime, contact::RetailContactMaterial};

pub(crate) struct GrindMaterials {
    standard: [RetailContactMaterial; 3],
}

impl GrindMaterials {
    pub fn new(settings: &PhysicsSettings) -> Self {
        Self {
            standard: [
                settings.standard_wheel_material,
                settings.truck_material,
                settings.deck_material,
            ],
        }
    }

    pub fn apply(
        &self,
        mode: MaterialMode,
        board: &mut BoardRuntime,
        settings: &mut PhysicsSettings,
    ) {
        let materials = match mode {
            MaterialMode::Unchanged => return,
            MaterialMode::Standard => self.standard,
            MaterialMode::Grind => {
                [RetailContactMaterial {
                    static_friction: 0.,
                    dynamic_friction: 0.,
                    restitution: 0.,
                }; 3]
            }
        };
        board.set_collision_group(4);
        [
            settings.wheel_material,
            settings.truck_material,
            settings.deck_material,
        ] = materials;
        //These leaves do not change shape enable flags, angular drag, motion
        //activation or wiping-out state. They are not full SetStandard82C03AF8.
    }
}
