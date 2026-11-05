//! Host ownership for the original 82DB4048 input phase. Every numerical
//! preparation is performed here; the two callbacks address the shared
//! skeleton/whole-player owners that cannot be borrowed twice by the host.
use super::{
    grind::{GrindInputState, Pending},
    pre_input::PreInputManager,
};
use crate::skate3_native::physics::ground_runtime::GroundRuntime;
use skate3_core::{
    math::Vector3,
    physics::{
        board_runtime::BoardRuntime, board_toolkit::BoardToolkit,
        skeleton_animation_record::AnimationPartTransform,
    },
    player::input_phase::{
        AnimationInputPacket, InputPhaseServices, PhysicalPlayerInput, PlayerInputState,
        ProcessedPhysicsInput, RawVector,
    },
};

#[derive(Clone, Copy)]
pub(crate) struct InputHostFrame {
    pub actor_query_56: u32,
    pub actor_query_44: u32,
    pub input_available: bool,
    /// Actual gameplay action 71 from ControllerInput::player_actions().
    pub transition_action: f32,
    /// Motion output's published +0 frame, read by the native reset request.
    pub published_board_transform: AnimationPartTransform,
    /// Native air-state manager +40, only read by the category200 grind gate.
    pub air_counter_40: i32,
}

pub(crate) trait PlayerInputCallbacks {
    fn prepare_grind(
        &mut self,
        board: &mut BoardRuntime,
        grind: &mut GrindInputState,
        processed: &ProcessedPhysicsInput,
        world: &skate3_core::physics::board_world::BoardWorld,
        provider: &crate::skate3_native::grind_world::StaticProvider,
        air_counter: i32,
    ) -> Result<Pending, String>;
    fn process_skeleton(
        &mut self,
        board: &mut BoardRuntime,
        toolkit: &BoardToolkit,
        packet: &AnimationInputPacket<'_>,
        physical: &mut PhysicalPlayerInput,
        processed: &mut ProcessedPhysicsInput,
    ) -> Result<(), String>;

    /// Complete 82DB8998 reset including board, skeleton, Reckoning, current
    /// physical outputs and Ground entry. Returning an error retains the request.
    fn teleport(
        &mut self,
        board: &mut BoardRuntime,
        ground: &mut GroundRuntime,
        target: AnimationPartTransform,
        player: &mut PlayerInputState,
        physical: &mut PhysicalPlayerInput,
        processed: &mut ProcessedPhysicsInput,
    ) -> Result<(), String>;
}

pub(super) struct Services<'a, C> {
    pub board: &'a mut BoardRuntime,
    pub ground: &'a mut GroundRuntime,
    pub ground_frame: AnimationPartTransform,
    pub host: InputHostFrame,
    pub callbacks: &'a mut C,
    pub pre_input: &'a mut PreInputManager,
    pub grind: &'a mut GrindInputState,
    pub world_edges: &'a crate::skate3_native::grind_world::StaticProvider,
    pub world: &'a skate3_core::physics::board_world::BoardWorld,
    pub pending_grind: &'a mut Option<Pending>,
    pub toolkit: &'a mut Option<BoardToolkit>,
    pub pending_teleport: &'a mut Option<AnimationPartTransform>,
    pub pending_velocity: &'a mut Option<[f32; 3]>,
}
impl<C: PlayerInputCallbacks> InputPhaseServices for Services<'_, C> {
    type Error = String;
    fn update_pre_input_manager_82d81610(
        &mut self,
        player: &mut PlayerInputState,
        _physical: &mut PhysicalPlayerInput,
    ) -> Result<(), String> {
        self.pre_input.prepare(&mut player.manager_1856_counter_320)
    }
    fn reset_processed_input_82bf9ef0(
        &mut self,
        output: &mut ProcessedPhysicsInput,
    ) -> Result<(), String> {
        super::reset::reset_processed(output);
        Ok(())
    }
    fn actor_query_slot_56(&mut self) -> Result<u32, String> {
        Ok(self.host.actor_query_56)
    }
    fn actor_query_slot_44(&mut self) -> Result<u32, String> {
        Ok(self.host.actor_query_44)
    }
    fn reset_player_probe_82d7a330(
        &mut self,
        player: &mut PlayerInputState,
        _physical: &mut PhysicalPlayerInput,
    ) -> Result<(), String> {
        // Native clears every represented field after copying it to Processed.
        player.probe = Default::default();
        Ok(())
    }
    fn check_teleport_82db88c8(
        &mut self,
        player: &mut PlayerInputState,
        physical: &mut PhysicalPlayerInput,
        output: &mut ProcessedPhysicsInput,
    ) -> Result<(), String> {
        // Native handles its state request before its explicit reset bit.
        // Host requests remain owned until the entire reset succeeds.
        if physical.state.flag_61 != 0 {
            let target = self.host.published_board_transform;
            self.callbacks
                .teleport(self.board, self.ground, target, player, physical, output)?;
        }
        if player.flags_1296 & (1 << 19) != 0 {
            let deck = self.board.part_transforms()[6];
            let mut target = [[0.0; 4]; 4];
            for (dst, src) in target[..3].iter_mut().zip(deck.basis.columns) {
                dst[..3].copy_from_slice(&src);
            }
            let p = deck.translation;
            target[3] = [p.x, p.y, p.z, 0.0];
            self.callbacks
                .teleport(self.board, self.ground, target, player, physical, output)?;
            player.flags_1296 &= !(1 << 19);
        }
        if let Some(target) = *self.pending_teleport {
            self.callbacks
                .teleport(self.board, self.ground, target, player, physical, output)?;
            *self.pending_teleport = None;
            if let Some(v) = self.pending_velocity.take() {
                apply_board_linvel(self.board, v);
            }
        }
        Ok(())
    }
    fn actor_input_available_slot_4(&mut self) -> Result<bool, String> {
        Ok(self.host.input_available)
    }
    fn transition_action_825903c8(&mut self) -> Result<f32, String> {
        Ok(self.host.transition_action)
    }
    fn calculate_ground_position_82c02840(
        &mut self,
        _physical: &PhysicalPlayerInput,
    ) -> Result<RawVector, String> {
        Ok(super::ground_position::ground_position(
            self.board,
            &self.ground_frame,
        ))
    }
    fn prepare_board_toolkit_82c013f0(
        &mut self,
        _player: &mut PlayerInputState,
        _physical: &mut PhysicalPlayerInput,
        output: &mut ProcessedPhysicsInput,
    ) -> Result<(), String> {
        let toolkit = self.ground.prepare_toolkit(self.board, output);
        //82C01744 publishes this during toolkit preparation, not PhysOut fill.
        output.scalar_2616 = toolkit.absolute_speed;
        *self.toolkit = Some(toolkit);
        Ok(())
    }
    fn process_skeleton_82bd8918(
        &mut self,
        packet: &AnimationInputPacket<'_>,
        physical: &mut PhysicalPlayerInput,
        output: &mut ProcessedPhysicsInput,
    ) -> Result<(), String> {
        let toolkit = self
            .toolkit
            .as_ref()
            .ok_or("Input toolkit must precede Skeleton::ProcessData")?;
        self.callbacks
            .process_skeleton(self.board, toolkit, packet, physical, output)
    }
    fn update_grind_manager_82d8a828(
        &mut self,
        _player: &mut PlayerInputState,
        _physical: &mut PhysicalPlayerInput,
        output: &mut ProcessedPhysicsInput,
    ) -> Result<(), String> {
        if self.pending_grind.is_some() {
            return Err("Previous grind input was not consumed by PostInput".into());
        }
        *self.pending_grind = Some(self.callbacks.prepare_grind(
            self.board,
            self.grind,
            output,
            self.world,
            self.world_edges,
            self.host.air_counter_40,
        )?);
        Ok(())
    }
}

fn apply_board_linvel(board: &mut BoardRuntime, v: [f32; 3]) {
    let vel = Vector3::new(v[0], v[1], v[2]);
    for body in board.bodies_mut() {
        body.rates.linear_velocity = vel;
    }
}
