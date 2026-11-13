//! Live bindings for TU3 SkateboardController, 82DB6150 and 82D76D20.
use super::super::board_possession::{Effects, Materials, VolumeFlags, settings, vector};
use super::native;
use crate::skate3_native::physics::{Skate3PhysicsState, Skate3ControllerState};
use skate3_core::{
    physics::{
        board::BodyId, board_step::CollisionBody, contact::RetailContactMaterial,
        skeleton_animation_record::compose_affine,
    },
    player::offboard::board_possession::{
        Fill,
        lifecycle::{Alignment, Observation},
    },
};
use skate_data::collections::Collections;

pub(crate) struct LiveState {
    pub volumes: VolumeFlags,
    alignment: Alignment,
    alignment_active: bool,
    standard_materials: [RetailContactMaterial; 3],
    released_material: RetailContactMaterial,
    standard_drag: f32,
    pub output: Option<Fill>,
}
impl LiveState {
    pub(crate) fn load(data: &Collections, physics: &Skate3PhysicsState) -> Result<Self, String> {
        let s = &physics.settings;
        let friction = data.float("physics_wipeout", "default", "SkateboardFriction")?;
        Ok(Self {
            volumes: VolumeFlags {
                deck: true,
                trucks: s.truck_collisions,
                wheels: true,
                deck_children: s
                    .deck_geometry
                    .children
                    .iter()
                    .map(|c| c.collision_enabled)
                    .collect(),
            },
            alignment: Alignment {
                first_1008: [0.; 4],
                second_1024: [0.; 4],
                factor_1040: 0.,
                flag_1044: false,
            },
            alignment_active: false,
            standard_materials: [s.standard_wheel_material, s.truck_material, s.deck_material],
            released_material: RetailContactMaterial {
                static_friction: friction,
                dynamic_friction: friction,
                restitution: data.float("physics_wipeout", "default", "SkateboardRestitution")?,
            },
            standard_drag: settings::standard_angular_drag(data)?,
            output: None,
        })
    }

    /// The hand alignment the held board keeps (Skateboard +1008..+1044), when one is set.
    pub(crate) fn alignment(&self) -> Option<&Alignment> {
        self.alignment_active.then_some(&self.alignment)
    }

    pub(crate) fn effects<'a>(
        &'a mut self,
        physics: &'a mut Skate3PhysicsState,
        animated: &'a mut u8,
        dt: f32,
    ) -> Effects<'a> {
        Effects {
            board: &mut physics.board,
            animated_290: animated,
            wiping_out: &mut physics.board_wiping_out,
            alignment: &mut self.alignment,
            alignment_active: &mut self.alignment_active,
            volumes: &mut self.volumes,
            materials: Materials {
                wheels: &mut physics.settings.wheel_material,
                trucks: &mut physics.settings.truck_material,
                deck: &mut physics.settings.deck_material,
            },
            standard_materials: self.standard_materials,
            released_material: self.released_material,
            standard_angular_drag: self.standard_drag,
            processed_dt_2604: dt,
        }
    }

    /// The compound children and shared truck shape are the actual fields
    /// colliders::world_volumes consumes. Wheel/aggregate gates are applied in
    /// solve.rs before either world queries or board/skater pairing.
    pub(crate) fn publish_volumes(&self, physics: &mut Skate3PhysicsState) {
        physics.settings.truck_collisions = self.volumes.trucks;
        for (child, enabled) in physics
            .settings
            .deck_geometry
            .children
            .iter_mut()
            .zip(&self.volumes.deck_children)
        {
            child.collision_enabled = *enabled;
        }
    }
    pub(crate) fn volume_enabled(&self, body: CollisionBody) -> bool {
        match body {
            CollisionBody::Board(BodyId::Deck) => self.volumes.deck,
            CollisionBody::Board(BodyId::FrontTruck | BodyId::BackTruck) => self.volumes.trucks,
            CollisionBody::Board(_) => self.volumes.wheels,
            _ => true,
        }
    }
}

pub(crate) fn observe(physics: &Skate3PhysicsState, skater: &Skate3ControllerState) -> Observation {
    let p = &skater.player_input.processed;
    let frames = &skater.skeleton_input.drive_frames;
    let mut board = crate::skate3_native::physics::solve::deck_frame(&physics.board);
    //82C01440..60: Processed64 is the unmodified deck part transform.
    //The effective axes are written separately at320/352.
    if let Some(toolkit) = &skater.player_input.toolkit {
        board = toolkit.deck;
    }
    Observation {
        processed: native::Processed {
            board_frame_64: board,
            player_frame_192: p
                .effective_anim_transform_192
                .map(|v| v.map(f32::from_bits)),
            position_592: p.vectors_544_560_592_608[2].map(f32::from_bits),
            velocity_912: p.vectors_880_896_912_928_944[2].map(f32::from_bits),
            direction_400: p.vectors_400_416[0].map(f32::from_bits),
            hide_direction_464: p.vectors_464_480_496_512_528[0].map(f32::from_bits),
            flags_2476: p.flags_2476,
            flags_2480: p.flags_2480,
            flags_2488: p.flags_2488,
        },
        board_collision_flags_872: physics.riding.ground.collision_flags,
        board_state_840: surface(physics),
        //82D761F0 reads Collision1232 + part*112 +99 (group byte1331).
        hand_contacts: [
            skater.collision_feedback.bones[3].groups[3],
            skater.collision_feedback.bones[7].groups[3],
        ],
        physical_hand_positions: [
            skater.skeleton.record.pose[3][3],
            skater.skeleton.record.pose[7][3],
        ],
        animation_board_frame_12624: frames[0],
        animation_hand_frames: [frames[3], frames[7]],
        //82BE3170 composes Skeleton11920 with current12624; not physical pose.
        attachment_frame_0: compose_affine(
            &skater.animated_skeleton.roots.animation_to_world,
            &frames[0],
        ),
    }
}

///82C08818 votes only four wheel surfaces; physical contact counts four,
/// line-only support counts one. Ties retain the lowest nonzero surface.
fn surface(physics: &Skate3PhysicsState) -> u32 {
    if physics.riding.ground.collision_flags & 0x0200_0000 != 0 {
        return 12;
    }
    let mut counts = [0; 32];
    for i in 0..4 {
        let surface = physics.riding.wheel_lines.physics_surfaces[i] as usize;
        if surface != 0 {
            counts[surface] += if physics.riding.ground.parts[i].in_contact {
                4
            } else {
                1
            };
        }
    }
    let mut best = 1;
    let mut count = 0;
    for i in 1..=13 {
        if counts[i] > count {
            best = i;
            count = counts[i];
        }
    }
    best as u32
}

/// Insert immediately after frame.rs's selected physical-state Update match,
/// before state_timer_1344 and solve. TU3 82DB6150..61B0.
pub(crate) fn update(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) {
    if !skater.skateboard_controller.fields.system_on_452 {
        return;
    }
    let observation = observe(physics, skater);
    let mut effects = skater.board_possession_live.effects(
        physics,
        &mut skater.ground_lifecycle.board_animated_290,
        skater.player_input.processed.timestep_2604,
    );
    skater.board_possession.update(
        &mut skater.skateboard_controller.fields,
        &observation,
        &mut effects,
    );
    skater.board_possession_live.publish_volumes(physics);
}

/// PhysState_OffBoard_Pushing::Enter 0x82d44300..44364: a board in hand (SkateboardController
/// state448 0, 1 or 5) is let go, and a returning one (4) starts hiding; free (2) and hiding (3)
/// boards stay as they are.
pub(crate) fn pushing_enter(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) {
    let state = skater.skateboard_controller.fields.state_448;
    if matches!(state, 2 | 3) {
        return;
    }
    let observation = observe(physics, skater);
    let mut effects = skater.board_possession_live.effects(
        physics,
        &mut skater.ground_lifecycle.board_animated_290,
        skater.player_input.processed.timestep_2604,
    );
    let fields = &mut skater.skateboard_controller.fields;
    fields.word_444 = 0;
    if state == 4 {
        // InitSkateboardHiding.
        skater.board_possession.state.hide(&observation, &mut effects);
        fields.state_448 = 3;
    } else {
        skater.board_possession.let_go(fields, &observation, &mut effects);
        fields.state_448 = 2;
    }
    skater.board_possession_live.publish_volumes(physics);
}

///82DB8998 brackets board/skeleton teleport with this operation. Call before
///board.reset_physical, then again after Skeleton update_teleport.
pub(crate) fn reset_for_teleport(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) {
    let observation = observe(physics, skater);
    let mut effects = skater.board_possession_live.effects(
        physics,
        &mut skater.ground_lifecycle.board_animated_290,
        skater.player_input.processed.timestep_2604,
    );
    skater.board_possession.reset_for_teleport(
        &mut skater.skateboard_controller.fields,
        &observation,
        &mut effects,
    );
    skater.board_possession_live.output = None;
    skater.board_possession_live.publish_volumes(physics);
}

///82DB8E10..8E24, AFTER teleport's SetPhysicsState(Ground). Standard-board
///restoration must follow Stop's released-material writes, not precede them.
pub(crate) fn finish_teleport(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) {
    use native::lifecycle::Effects as _;
    skater
        .board_possession_live
        .effects(
            physics,
            &mut skater.ground_lifecycle.board_animated_290,
            skater.player_input.processed.timestep_2604,
        )
        .standard_board();
    skater.board_possession_live.publish_volumes(physics);
}

/// Insert after finish_skater (completed contacts), before next input. The
/// alignment test is82C0837C..8428 and publishes the actual release bit872.
pub(crate) fn publish(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Fill {
    let live = &skater.board_possession_live;
    if live.alignment_active {
        use native::math::dot;
        for part in &physics.riding.ground.parts {
            if part.in_contact
                && (dot(vector(part.normal), live.alignment.first_1008.map(|x| -x))
                    > live.alignment.factor_1040
                    || dot(vector(part.normal), live.alignment.second_1024.map(|x| -x))
                        > live.alignment.factor_1040)
            {
                physics.riding.ground.collision_flags |= 0x0400_0000;
            }
        }
    }
    let observation = observe(physics, skater);
    let bone = compose_affine(
        &skater.animated_skeleton.roots.animation_to_world,
        &skater.skeleton_input.drive_frames[11],
    );
    let out = skater.board_possession.fill(
        &skater.skateboard_controller.fields,
        &observation.processed,
        bone,
    );
    let physical = &mut skater.player_input.physical.off_board;
    physical.angle_36 = out.angle_36;
    physical.angle_40 = out.angle_40;
    physical.flag_311 = u8::from(out.held_311);
    physical.free_board_312 = u8::from(out.free_312);
    physical.returning_board_313 = u8::from(out.returning_313);
    physical.hiding_board_321 = u8::from(out.hiding_321);
    physical.dropping_board_322 = u8::from(out.flag_322);
    physical.retrieving_board_323 = u8::from(out.flag_323);
    physical.flag_324 = u8::from(out.flag_324);
    skater.board_possession_live.output = Some(out);
    out
}
