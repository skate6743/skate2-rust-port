//! Ordered native animation records, Skeleton.ProcessAnimAttributes82b59de8.
//! These fields belong to the same Processed input used by Update1.
use super::SkeletonProcessedContact;
use crate::{
    actor_construction::anim_out_phys_in::PhysicsAnimationAttribute,
    physical_player_anim_out::PhysicalAnimOut, play_animation::FastString30,
};

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ProcessedAnimAttributeFields {
    pub(crate) push_speed: Option<f32>,
    pub(crate) body_spin: Option<f32>,
    pub(crate) turn: Option<f32>,
    pub(crate) offboard_turn: Option<f32>,
    pub(crate) look_x: Option<f32>,
    pub(crate) look_y: Option<f32>,
    pub(crate) raw_turn: Option<f32>,
    pub(crate) hard_turn: Option<f32>,
    pub(crate) foot_brake: Option<f32>,
    pub(crate) move_object_z: Option<f32>,
    pub(crate) move_object_x: Option<f32>,
    pub(crate) move_object_rotation: Option<f32>,
    pub(crate) grind_translation: Option<f32>,
    pub(crate) grind_nudge: Option<f32>,
    pub(crate) grind_up_down: Option<f32>,
    pub(crate) grind_loose_end_height: Option<f32>,
    pub(crate) physics_body_spin: Option<f32>,
    pub(crate) world_grab_y: Option<f32>,
    pub(crate) world_grab_z: Option<f32>,
    pub(crate) wipeout_x: Option<f32>,
    pub(crate) wipeout_y: Option<f32>,
    pub(crate) offboard_jump_x: Option<f32>,
    pub(crate) offboard_jump_z: Option<f32>,
}
impl ProcessedAnimAttributeFields {
    pub(crate) const fn new() -> Self {
        Self {
            push_speed: None,
            body_spin: None,
            turn: None,
            offboard_turn: None,
            look_x: None,
            look_y: None,
            raw_turn: None,
            hard_turn: None,
            foot_brake: None,
            move_object_z: None,
            move_object_x: None,
            move_object_rotation: None,
            grind_translation: None,
            grind_nudge: None,
            grind_up_down: None,
            grind_loose_end_height: None,
            physics_body_spin: None,
            world_grab_y: None,
            world_grab_z: None,
            wipeout_x: None,
            wipeout_y: None,
            offboard_jump_x: None,
            offboard_jump_z: None,
        }
    }
    pub(crate) fn reset(&mut self) {
        // Reset39468..3958c writes these actual scalar lanes with f31=+0.
        // Reset39408 also writes native push speed940 with the same f31.
        *self = Self {
            push_speed: Some(0.0),
            body_spin: Some(0.0),
            turn: Some(0.0),
            offboard_turn: Some(0.0),
            look_x: Some(0.0),
            look_y: Some(0.0),
            raw_turn: Some(0.0),
            hard_turn: Some(0.0),
            foot_brake: Some(0.0),
            move_object_z: Some(0.0),
            move_object_x: Some(0.0),
            move_object_rotation: Some(0.0),
            grind_translation: Some(0.0),
            grind_nudge: Some(0.0),
            grind_up_down: Some(0.0),
            grind_loose_end_height: Some(0.0),
            physics_body_spin: Some(0.0),
            world_grab_y: Some(0.0),
            world_grab_z: Some(0.0),
            wipeout_x: Some(0.0),
            wipeout_y: Some(0.0),
            offboard_jump_x: Some(0.0),
            offboard_jump_z: Some(0.0),
        };
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AttributeReadError {
    UnknownPayload(usize),
    UnknownFlags(u32),
}
impl std::fmt::Display for AttributeReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownPayload(offset) => {
                write!(f, "animation attribute payload byte {offset} unknown")
            }
            Self::UnknownFlags(offset) => write!(f, "processed flags at {offset:x} unknown"),
        }
    }
}
pub(crate) fn payload_word(
    attribute: &PhysicsAnimationAttribute,
    offset: usize,
) -> Result<u32, AttributeReadError> {
    let mut bytes = [0; 4];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = attribute.payload[offset + index]
            .ok_or(AttributeReadError::UnknownPayload(offset + index))?;
    }
    Ok(u32::from_be_bytes(bytes))
}
pub(crate) fn payload_scalar(
    attribute: &PhysicsAnimationAttribute,
) -> Result<f32, AttributeReadError> {
    payload_word(attribute, 0).map(f32::from_bits)
}
pub(crate) fn named(attribute: &PhysicsAnimationAttribute, name: &str) -> bool {
    attribute.name == FastString30::encode(name).expect("source attribute literal")
}

impl SkeletonProcessedContact {
    /// Accepted type-three contact prefix. A push result requests the source's
    /// unconditional trajectory length calculation before its final speed store.
    #[expect(
        clippy::float_cmp,
        reason = "native contact transitions compare exactly against plus/minus one"
    )]
    pub(crate) fn try_apply_animation_contact_prefix(
        &mut self,
        attribute: &PhysicsAnimationAttribute,
        data: &crate::skeleton_data::SkeletonData,
    ) -> Result<Option<f32>, AttributeReadError> {
        if attribute.kind != 3 || attribute.flags & 0x0c == 0 {
            return Ok(None);
        }
        let mut words = [0; 5];
        for (index, word) in words.iter_mut().enumerate() {
            *word = payload_word(attribute, index * 4)?;
        }
        self.contact_bone_id = data
            .hierarchy()
            .packed_bone_names
            .iter()
            .position(|name| name.words == words);
        if named(attribute, "push_contact") {
            self.animation_attribute_flags(0x8c4, 0x0800_0000, 0x0800_0000)?;
            let right = self.contact_bone_id == Some(data.bone_ids()[19]);
            self.animation_attribute_flags(
                0x8c4,
                0x0400_0000,
                if right { 0x0400_0000 } else { 0 },
            )?;
            // Native loads may carry unknown bytes; require the scalar only at
            // the first transition-dependent store, after the hierarchy/side.
            let transition = f32::from_bits(payload_word(attribute, 20)?);
            self.animation_attribute_flags(
                0x8c4,
                0x0200_0000,
                if transition == 1.0 { 0x0200_0000 } else { 0 },
            )?;
            self.animation_attribute_flags(
                0x8c4,
                0x0100_0000,
                if transition == -1.0 { 0x0100_0000 } else { 0 },
            )?;
            return Ok(Some(transition));
        }
        if named(attribute, "brake_contact") {
            self.animation_attribute_flags(0x8c4, 0x1000_0000, 0x1000_0000)?;
            let transition = f32::from_bits(payload_word(attribute, 20)?);
            self.animation_attribute_flags(
                0x8c4,
                0x4000_0000,
                if transition == 1.0 { 0x4000_0000 } else { 0 },
            )?;
            self.animation_attribute_flags(
                0x8c4,
                0x0080_0000,
                if transition == -1.0 { 0x0080_0000 } else { 0 },
            )?;
        } else if named(attribute, "right_hand_grab") || named(attribute, "left_hand_grab") {
            let right = words
                == FastString30::encode("righthand")
                    .expect("source literal")
                    .words;
            let mask = if right { 0x400 } else { 0x200 };
            self.animation_attribute_flags(0x8c8, mask, mask)?;
        }
        Ok(None)
    }
    pub(crate) fn store_animation_push_speed(&mut self, speed: f32) {
        self.anim_attributes.push_speed = Some(speed);
    }
    /// Final original input flag samples and retrieve/drop override, b9c0..ba3c.
    pub(crate) fn try_finish_animation_attribute_flags(
        &mut self,
        input: &crate::actor_construction::anim_out_phys_in::AnimOutPhysInOwner,
    ) -> Result<(), AttributeReadError> {
        if input.flags.living_world_impulse_active {
            self.animation_attribute_flags(0x8c4, 0x0004_0000, 0x0004_0000)?;
        }
        let controls = self
            .physics_state
            .flags_8cc
            .ok_or(AttributeReadError::UnknownFlags(0x8cc))?;
        self.animation_attribute_flags(0x8d0, 0x40, (input.locomotion_flags >> 21) & 0x40)?;
        self.animation_attribute_flags(0x8d0, 4, (input.locomotion_flags >> 23) & 4)?;
        self.animation_attribute_flags(
            0x8d4,
            0x0100_0000,
            (input.locomotion_flags >> 4) & 0x0100_0000,
        )?;
        if controls & 0x0c00 != 0 {
            self.animation_attribute_flags(0x8cc, 0x0100_0000, 0)?;
        }
        Ok(())
    }
    pub(crate) fn animation_attribute_fields(&self) -> &ProcessedAnimAttributeFields {
        &self.anim_attributes
    }
    pub(crate) fn animation_attribute_flags(
        &mut self,
        offset: u32,
        mask: u32,
        value: u32,
    ) -> Result<(), AttributeReadError> {
        match offset {
            0x8c4 => {
                // The existing jump owner retains the complete native word.
                self.jump_events.process_animation_bits(mask, value);
            }
            0x8c8 => {
                let word = self
                    .flags_8c8
                    .as_mut()
                    .ok_or(AttributeReadError::UnknownFlags(offset))?;
                *word = (*word & !mask) | (value & mask);
            }
            0x8cc | 0x8d0 => {
                let word = if offset == 0x8cc {
                    &mut self.physics_state.flags_8cc
                } else {
                    &mut self.physics_state.flags_8d0
                };
                let word = word
                    .as_mut()
                    .ok_or(AttributeReadError::UnknownFlags(offset))?;
                *word = (*word & !mask) | (value & mask);
            }
            0x8d4 => {
                let word = &mut self.physics_state.flags_8d4;
                word.0 = (word.0 & !mask) | (value & mask);
                word.1 |= mask;
            }
            _ => unreachable!("selected original flag word"),
        }
        Ok(())
    }

    /// Pure stores in the type-zero chain. Special callbacks are dispatched by
    /// the enclosing ordered pass at their actual position in the record loop.
    pub(crate) fn try_apply_animation_attribute_store(
        &mut self,
        attribute: &PhysicsAnimationAttribute,
        output: &mut PhysicalAnimOut,
    ) -> Result<bool, AttributeReadError> {
        if attribute.kind != 0 {
            return Ok(false);
        }
        for name in [
            "SlideBoard",
            "SlideNose",
            "SlideTail",
            "Grind5050",
            "Grind5_O",
            "GrindNose",
        ] {
            if named(attribute, name) {
                output.grind_name = attribute.name;
                return Ok(true);
            }
        }
        for (name, mask, value) in [
            ("GrindFacingForwards", 0x8000_0000, 0x8000_0000),
            ("GrindFacingBackwards", 0x8000_0000, 0),
            ("NoInput", 0x4000_0000, 0x4000_0000),
            ("NewAutoPump", 0x2000_0000, 0x2000_0000),
            ("KickTurn", 0x1000_0000, 0x1000_0000),
            ("Anticipating", 0x0800_0000, 0x0800_0000),
        ] {
            if named(attribute, name) {
                output.flags_bits = (output.flags_bits & !mask) | value;
                output.flags_known_mask |= mask;
                if name == "NoInput" {
                    self.animation_attribute_flags(0x8c8, 0x0100_0000, 0x0100_0000)?;
                }
                return Ok(true);
            }
        }
        for (name, offset, mask) in FLAG_ATTRIBUTES {
            if named(attribute, name) {
                self.animation_attribute_flags(*offset, *mask, *mask)?;
                return Ok(true);
            }
        }
        if named(attribute, "OB_ReleaseBoard") {
            self.animation_attribute_flags(0x8cc, 0x6000, 0x4000)?;
            return Ok(true);
        }
        if named(attribute, "OBL_SteerMagnitude") {
            let active = payload_scalar(attribute)? > f32::from_bits(0x3e4c_cccd);
            self.animation_attribute_flags(
                0x8d4,
                0x8000_0000,
                if active { 0x8000_0000 } else { 0 },
            )?;
            return Ok(true);
        }
        for (name, scalar, negate) in SCALAR_ATTRIBUTES {
            if named(attribute, name) {
                let value = payload_scalar(attribute)?;
                let value = if *negate { -value } else { value };
                match scalar {
                    Scalar::Balance => self.physics_state.balance = Some(value),
                    Scalar::Slide => self.physics_state.slide = Some(value),
                    Scalar::BodySpin => self.anim_attributes.body_spin = Some(value),
                    Scalar::Turn => self.anim_attributes.turn = Some(value),
                    Scalar::OffboardTurn => self.anim_attributes.offboard_turn = Some(value),
                    Scalar::LookX => self.anim_attributes.look_x = Some(value),
                    Scalar::LookY => self.anim_attributes.look_y = Some(value),
                    Scalar::RawTurn => self.anim_attributes.raw_turn = Some(value),
                    Scalar::HardTurn => self.anim_attributes.hard_turn = Some(value),
                    Scalar::FootBrake => self.anim_attributes.foot_brake = Some(value),
                    Scalar::MoveZ => self.anim_attributes.move_object_z = Some(value),
                    Scalar::MoveX => self.anim_attributes.move_object_x = Some(value),
                    Scalar::MoveRotation => self.anim_attributes.move_object_rotation = Some(value),
                    Scalar::GrindTranslation => {
                        self.anim_attributes.grind_translation = Some(value);
                    }
                    Scalar::GrindNudge => self.anim_attributes.grind_nudge = Some(value),
                    Scalar::GrindUpDown => self.anim_attributes.grind_up_down = Some(value),
                    Scalar::GrindHeight => {
                        self.anim_attributes.grind_loose_end_height = Some(value);
                    }
                    Scalar::PhysicsBodySpin => self.anim_attributes.physics_body_spin = Some(value),
                    Scalar::WorldGrabY => self.anim_attributes.world_grab_y = Some(value),
                    Scalar::WorldGrabZ => self.anim_attributes.world_grab_z = Some(value),
                    Scalar::WipeoutX => self.anim_attributes.wipeout_x = Some(value),
                    Scalar::WipeoutY => self.anim_attributes.wipeout_y = Some(value),
                    Scalar::JumpX => self.anim_attributes.offboard_jump_x = Some(value),
                    Scalar::JumpZ => self.anim_attributes.offboard_jump_z = Some(value),
                }
                return Ok(true);
            }
        }
        Ok(false)
    }
}

const FLAG_ATTRIBUTES: &[(&str, u32, u32)] = &[
    ("GrabWorld", 0x8cc, 0x0100_0000),
    ("ExitGrind", 0x8cc, 0x8000_0000),
    ("EnteringCoffin", 0x8c8, 9),
    ("InCoffin", 0x8c8, 5),
    ("LeavingCoffin", 0x8c8, 3),
    ("PlayingLanding", 0x8cc, 0x4000_0000),
    ("ManualBrake", 0x8c4, 0x2000_0000),
    ("OB_Traj", 0x8cc, 0x0002_0000),
    ("OB_Com", 0x8cc, 0x0001_0000),
    ("OB_Air", 0x8cc, 0x100),
    ("Carving", 0x8c4, 0x8000_0000),
    ("FrontFlip", 0x8c4, 0x80),
    ("BackFlip", 0x8c4, 0x40),
    ("Grabbing", 0x8c4, 0x20),
    ("WantsLeftAirGrab", 0x8c4, 0x10),
    ("WantsRightAirGrab", 0x8c4, 0x10),
    ("FlipTrick", 0x8c8, 0x0080_0000),
    ("WipeOutRecover", 0x8c8, 0x0020_0000),
    ("ControlledWipeout", 0x8c8, 0x0010_0000),
    ("Wipeout", 0x8c4, 0x0004_0000),
    ("LateTrick", 0x8c8, 0x80),
    ("NoDangerZone", 0x8c8, 0x40),
    ("ChristAir", 0x8c8, 0x100),
    ("NewHandPlant", 0x8d0, 0x4000_0000),
    ("AnimCommittedHandPlant", 0x8d0, 0x2000_0000),
    ("DisableFeetTargetIK", 0x8d0, 0x1000_0000),
    ("OB_Jump", 0x8cc, 0x0020_0000),
    ("OB_StandingJump", 0x8cc, 0x0010_0000),
    ("PlayerControlledPump", 0x8cc, 4),
    ("OneFootAir", 0x8d0, 0x0800_0000),
    ("OB_Mounting", 0x8d0, 0x0020_0000),
    ("OB_Unmounting", 0x8d0, 0x0010_0000),
    ("OB_Dismount", 0x8d0, 0x0008_0000),
    ("OB_HoldBoard", 0x8cc, 0x8000),
    ("OB_ThrowBoard", 0x8cc, 0x6000),
    ("OB_RetrieveBoard", 0x8cc, 0x1000),
    ("OB_RetrievingBoard", 0x8cc, 0x800),
    ("OB_DroppingBoard", 0x8cc, 0x400),
    ("FootJump", 0x8d0, 0x0001_0000),
    ("AnimSkateboard", 0x8d0, 0x0002_0000),
    ("PushTricking", 0x8d0, 0x4000),
    ("KickoutDismount", 0x8d0, 0x800),
    ("RunoutDismount", 0x8d0, 0x400),
    ("FixLeftFoot", 0x8d0, 0x200),
    ("FixRightFoot", 0x8d0, 0x100),
    ("DoLandOnBoard", 0x8d0, 0x80),
    ("TransitioningOnOffBoard", 0x8d0, 0x20),
    ("JumpAboutToTakeOff", 0x8d0, 2),
    ("JumpInProgress", 0x8d0, 1),
    ("SkaterOffBoard", 0x8d4, 0x4000_0000),
    ("RunMount", 0x8d4, 0x0400_0000),
    ("SprintMount", 0x8d4, 0x0200_0000),
];
enum Scalar {
    Balance,
    Slide,
    BodySpin,
    Turn,
    OffboardTurn,
    LookX,
    LookY,
    RawTurn,
    HardTurn,
    FootBrake,
    MoveZ,
    MoveX,
    MoveRotation,
    GrindTranslation,
    GrindNudge,
    GrindUpDown,
    GrindHeight,
    PhysicsBodySpin,
    WorldGrabY,
    WorldGrabZ,
    WipeoutX,
    WipeoutY,
    JumpX,
    JumpZ,
}
const SCALAR_ATTRIBUTES: &[(&str, Scalar, bool)] = &[
    ("Balance", Scalar::Balance, true),
    ("Slide", Scalar::Slide, true),
    ("BodySpin", Scalar::BodySpin, true),
    ("Turn", Scalar::Turn, true),
    ("OB_Turn", Scalar::OffboardTurn, true),
    ("OB_LookAtX", Scalar::LookX, false),
    ("OB_LookAtY", Scalar::LookY, false),
    ("RawTurn", Scalar::RawTurn, true),
    ("HardTurn", Scalar::HardTurn, true),
    ("Brake", Scalar::FootBrake, false),
    ("OB_ObjectMvZ", Scalar::MoveZ, false),
    ("OB_ObjectMvX", Scalar::MoveX, false),
    ("OB_ObjectMvRot", Scalar::MoveRotation, false),
    ("PhysGrindTranslation", Scalar::GrindTranslation, false),
    ("PhysGrindStabilityNudge", Scalar::GrindNudge, false),
    ("PhysGrindUpDown", Scalar::GrindUpDown, false),
    ("PhysGrindGrabMinHeight", Scalar::GrindHeight, false),
    ("PhysBodySpin", Scalar::PhysicsBodySpin, false),
    ("world_grab_y", Scalar::WorldGrabY, false),
    ("world_grab_z", Scalar::WorldGrabZ, false),
    ("WipeoutControlX", Scalar::WipeoutX, false),
    ("WipeoutControlY", Scalar::WipeoutY, false),
    // 1.02 8299525c negates it: the PlayerUI stick reaches it in the skater's frame.
    ("OB_JumpX", Scalar::JumpX, true),
    ("OB_JumpZ", Scalar::JumpZ, false),
];

#[cfg(test)]
mod tests {
    use super::*;
    fn attribute(name: &str, value: Option<f32>) -> PhysicsAnimationAttribute {
        let mut payload = [None; 24];
        if let Some(value) = value {
            for (target, byte) in payload.iter_mut().zip(value.to_be_bytes()) {
                *target = Some(byte);
            }
        }
        PhysicsAnimationAttribute {
            payload,
            begin: 0.0,
            end: 0.0,
            name: FastString30::encode(name).unwrap(),
            flags: 0,
            kind: 0,
            sequence: None,
        }
    }
    #[test]
    fn flag_records_ignore_payload_and_status_and_preserve_last_assignment_order() {
        let mut processed = SkeletonProcessedContact::new();
        processed.reset();
        let mut output = PhysicalAnimOut::new();
        for name in [
            "GrindFacingForwards",
            "GrindFacingBackwards",
            "InCoffin",
            "LeavingCoffin",
            "OB_ThrowBoard",
            "OB_ReleaseBoard",
        ] {
            assert!(
                processed
                    .try_apply_animation_attribute_store(&attribute(name, None), &mut output)
                    .unwrap()
            );
        }
        assert_eq!(output.flags_bits & 0x8000_0000, 0);
        assert_eq!(processed.known_flags_8c8().unwrap() & 15, 7);
        assert_eq!(processed.physics_state.flags_8cc.unwrap() & 0x6000, 0x4000);
        let mut non_scalar = attribute("GrindFacingForwards", None);
        non_scalar.kind = 1;
        assert!(
            !processed
                .try_apply_animation_attribute_store(&non_scalar, &mut output)
                .unwrap()
        );
        assert_eq!(output.flags_bits & 0x8000_0000, 0);
    }
    #[test]
    fn scalar_unknown_refuses_only_matching_payload_and_noinput_retains_first_store() {
        let mut processed = SkeletonProcessedContact::new();
        let mut output = PhysicalAnimOut::new();
        assert!(
            !processed
                .try_apply_animation_attribute_store(&attribute("Unrecognized", None), &mut output)
                .unwrap()
        );
        assert_eq!(
            processed.try_apply_animation_attribute_store(&attribute("NoInput", None), &mut output),
            Err(AttributeReadError::UnknownFlags(0x8c8))
        );
        assert_eq!(output.flags_bits & 0x4000_0000, 0x4000_0000);
        processed.reset();
        processed
            .try_apply_animation_attribute_store(&attribute("Balance", Some(0.0)), &mut output)
            .unwrap();
        assert_eq!(
            processed.physics_state.balance.unwrap().to_bits(),
            0x8000_0000
        );
        assert_eq!(
            processed.try_apply_animation_attribute_store(&attribute("Balance", None), &mut output),
            Err(AttributeReadError::UnknownPayload(0))
        );
        assert_eq!(
            processed.physics_state.balance.unwrap().to_bits(),
            0x8000_0000
        );
    }
    #[test]
    fn scalar_threshold_nan_clears_known_bit_and_reset_is_actual_field_reset() {
        let mut processed = SkeletonProcessedContact::new();
        processed.reset();
        let mut output = PhysicalAnimOut::new();
        processed
            .try_apply_animation_attribute_store(
                &attribute("OBL_SteerMagnitude", Some(1.0)),
                &mut output,
            )
            .unwrap();
        assert_ne!(processed.physics_state.flags_8d4.0 & 0x8000_0000, 0);
        processed
            .try_apply_animation_attribute_store(
                &attribute("OBL_SteerMagnitude", Some(f32::NAN)),
                &mut output,
            )
            .unwrap();
        assert_eq!(processed.physics_state.flags_8d4.0 & 0x8000_0000, 0);
        processed
            .try_apply_animation_attribute_store(&attribute("Brake", Some(0.75)), &mut output)
            .unwrap();
        assert_eq!(
            processed
                .animation_attribute_fields()
                .foot_brake
                .unwrap()
                .to_bits(),
            0x3f40_0000
        );
        processed.reset();
        assert_eq!(
            processed
                .animation_attribute_fields()
                .foot_brake
                .unwrap()
                .to_bits(),
            0
        );
    }
}
