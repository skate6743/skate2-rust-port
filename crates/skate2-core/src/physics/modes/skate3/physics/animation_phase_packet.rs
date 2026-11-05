//! Owned PlayerInput packet storage; original actor publication825937EC.
use skate3_core::{
    animation::output::{
        actor_packet::ExternalPhysicsInput,
        packet_reset::{AdditionalResetFields, RESET_POSE},
        physics_packet::PhysicsPosePacket,
    },
    input::animation_packet::AnimationPacketFields,
    player::input_phase::AnimationInputPacket,
};
use skate_data::collections::Collections;

/// Host-owned player preferences, distinct from the stock response curves.
pub(crate) struct AnimationProfile {
    pub maximum_ground_angle_degrees: f32,
    pub skitch_transition_time: f32,
    pub truck_tightness: f32,
    pub wheel_hardness: f32,
    pub physics_mode: u32,
    pub prevent_manual_respawn: bool,
    pub ignore_respawn_reset_button: u8,
    pub force_braking: bool,
    /// Host Actor's actual one-shot LivingWorld push, in source packet words.
    pub external_impulse: Option<[u32;4]>,
    pub suppress_transition: bool,
    /// Optional gesture customization and external context are host policy.
    pub gesture_selections: Option<[u32; 4]>,
    pub suppress_up_gesture: bool,
    pub gesture_force_brake_bypass: bool,
}
impl AnimationProfile {
    pub fn load(data: &Collections, mode: &str) -> Result<Self, String> {
        //Original82DB1B08..1B78 initializes these variant slots in this order.
        let physics_mode = ["easy", "normal", "hardcore", "motorized", "test"]
            .iter()
            .position(|name| *name == mode)
            .ok_or_else(|| format!("Undefined player physics profile {mode}"))?
            as u32;
        Ok(Self {
            //82BBBDE0: Globals400/layout96, bound by828A0154.
            skitch_transition_time: data.float(
                "anim_skitching",
                "default",
                "LongSkitchIntoReachTime",
            )?,
            //82BA5150 global316/layout1844;8289FB24 binds pushing.
            maximum_ground_angle_degrees: data.float(
                "anim_motion",
                "pushing",
                "disable_push_brake_at_slope",
            )?,
            //Initial equipment preferences use the fixed settings value from
            //82593988/94. The host allows users to change these independently.
            truck_tightness: f32::from_bits(0x3f333333),
            wheel_hardness: f32::from_bits(0x3f333333),
            physics_mode,
            prevent_manual_respawn: false,
            ignore_respawn_reset_button: 0,
            force_braking: false,
            external_impulse: None,
            suppress_transition: false,
            gesture_selections: Some([0, 1, 2, 3]),
            //82B988B8 returns false when its optional context is absent.
            suppress_up_gesture: false,
            gesture_force_brake_bypass: false,
        })
    }
}

pub(crate) struct AnimationPhaseOutput {
    pub(super) reset: AdditionalResetFields,
    publication: AnimationPacketFields,
    external: ExternalPhysicsInput,
    mirrored: u8,
    weight_forwards: u8,
    flags: u32,
    // Actor825938FC/393C consumes these request bits after publication.
}
impl AnimationPhaseOutput {
    pub(super) fn new() -> Self {
        Self {
            //Reserved storage: native reset writes every field before use.
            reset: AdditionalResetFields {
                compression: 0.0,
                foot_ik_influence: [0.0; 2],
                next_step_position_valid: false,
                actor_flag_1904_bit23: false,
                actor_flag_1908_bit2: false,
                external_impulse_active: false,
                external_physics_input_active: false,
                externally_controlled: false,
                prevent_manual_respawn: false,
                ignore_respawn_reset_button: 0,
                force_braking: false,
                truck_tightness: 0.0,
                wheel_hardness: 0.0,
                auxiliary_vectors: [[0.0; 4]; 6],
                requested_physics_mode: 1,
            },
            publication: AnimationPacketFields {
                stance_byte: 0,
                timestep: 0.0,
                scalar_10388: 0.0,
                flags_10375_10496_10784: [0; 3],
                vector_10480: [0; 4],
                matrix_10704: RESET_POSE.map(|c| c.map(f32::to_bits)),
                byte_10768: 0,
                truck_tightness: 0.0,
                scalar_10792: 0.0,
                flag_10371: 0,
            },
            //No external impulse/controller/reset provider in this game. The
            //inactive payloads are owned storage and are not consumed.
            external: ExternalPhysicsInput {
                vectors: [[0; 4]; 10],
                flags: 0,
            },
            mirrored: 0,
            weight_forwards: 0,
            flags: 0,
        }
    }
    pub(crate) fn publish(
        &mut self,
        pose: &PhysicsPosePacket,
        profile: &AnimationProfile,
        actor_flags: u32,
    ) {
        let r = &mut self.reset;
        // Actor1904 bit23 is distinct from bit30's LivingWorld impulse.
        r.actor_flag_1904_bit23 = false;
        r.external_impulse_active = profile.external_impulse.is_some();
        r.actor_flag_1908_bit2 = profile.suppress_transition || actor_flags & (1 << 2) != 0;
        r.truck_tightness = profile.truck_tightness;
        r.wheel_hardness = profile.wheel_hardness;
        r.requested_physics_mode = profile.physics_mode;
        r.prevent_manual_respawn = profile.prevent_manual_respawn;
        r.ignore_respawn_reset_button = profile.ignore_respawn_reset_button;
        r.force_braking = profile.force_braking;
        //Free skating has no challenge scene19/20 requesting bit22.
        //82593924 reads Actor1904 bit25, not the input-filter word1908.
        self.flags = pose.flags & !((1 << 24) | (1 << 22));
        self.mirrored = u8::from(pose.mirrored);
        self.weight_forwards = u8::from(pose.weight_forwards);
        self.publication = AnimationPacketFields {
            stance_byte: u8::from(pose.board_flipped),
            timestep: pose.timestep,
            scalar_10388: r.compression,
            flags_10375_10496_10784: [
                u8::from(r.actor_flag_1904_bit23),
                u8::from(r.external_impulse_active),
                0, //Actor1904 bit29 is published only by the canonical reset reply.
            ],
            vector_10480: profile.external_impulse.unwrap_or([0;4]),
            matrix_10704: RESET_POSE.map(|c| c.map(f32::to_bits)),
            byte_10768: 0,
            truck_tightness: r.truck_tightness,
            scalar_10792: r.wheel_hardness,
            flag_10371: u8::from(pose.riding_switch),
        };
    }
    ///Actor825926F8 reply ->10704/10768/10784, then82DB5BE0 maps physical input.
    pub(crate) fn publish_external_reset(
        &mut self,
        reply: skate3_core::animation::output::actor_packet::ExternalReset,
    ) {
        self.publication.matrix_10704 = reply.transform;
        self.publication.byte_10768 = reply.byte64;
        self.publication.flags_10375_10496_10784[2] = 1;
    }
    pub fn packet(&self) -> AnimationInputPacket<'_> {
        let r = &self.reset;
        let vectors = r.auxiliary_vectors.map(|v| v.map(f32::to_bits));
        AnimationInputPacket {
            publication: &self.publication,
            external_physics_10512: &self.external,
            flag_10369: u8::from(r.next_step_position_valid),
            flag_10370: self.mirrored,
            flag_10373: self.weight_forwards,
            suppress_transition_10376: u8::from(r.actor_flag_1908_bit2),
            use_external_physics_10688: 0,
            external_physics_flag_10689: 0,
            flag_10786: u8::from(r.prevent_manual_respawn),
            flag_10787: r.ignore_respawn_reset_button,
            force_braking_10796: u8::from(r.force_braking),
            vector_10816: vectors[0],
            vector_10832: vectors[1],
            vector_10848: vectors[2],
            vector_10864: vectors[3],
            vector_10880: vectors[4],
            vector_10896: vectors[5],
            scalar_10912: r.foot_ik_influence[0],
            scalar_10916: r.foot_ik_influence[1],
            state_variant_10928: r.requested_physics_mode,
            flags_10932: self.flags,
        }
    }
}
