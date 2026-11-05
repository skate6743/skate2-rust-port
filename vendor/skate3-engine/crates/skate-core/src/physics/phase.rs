//! Engine-independent transport records for one authoritative simulation tick.
//!
//! These records deliberately contain requests and published values rather
//! than references to a game/world runtime. Game code owns when they are
//! consumed; core code owns their shape and deterministic validation.

use crate::{math::Vector3, player::state::PhysicalStateId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhysicsBody {
    Board,
    Rider,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PhysicsCommand {
    SetVelocity {
        body: PhysicsBody,
        linear: Vector3,
        angular: Vector3,
    },
    ApplyImpulse {
        body: PhysicsBody,
        impulse: Vector3,
        point: Vector3,
    },
    RequestState(PhysicalStateId),
    SetContactMode {
        body: PhysicsBody,
        enabled: bool,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhysicsCommandBuffer {
    tick: u64,
    commands: Vec<PhysicsCommand>,
}

impl PhysicsCommandBuffer {
    pub fn new(tick: u64) -> Self {
        Self {
            tick,
            commands: Vec::new(),
        }
    }
    pub fn tick(&self) -> u64 {
        self.tick
    }
    pub fn commands(&self) -> &[PhysicsCommand] {
        &self.commands
    }
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    pub fn push(&mut self, tick: u64, command: PhysicsCommand) -> Result<(), String> {
        if tick != self.tick {
            return Err(format!(
                "Physics command belongs to tick {tick}, buffer owns tick {}",
                self.tick
            ));
        }
        self.commands.push(command);
        Ok(())
    }

    pub fn clear(&mut self, tick: u64) -> Result<(), String> {
        if tick != self.tick {
            return Err(format!(
                "Cannot clear physics commands for tick {tick}; buffer owns tick {}",
                self.tick
            ));
        }
        self.commands.clear();
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhysicsEvent {
    StateChanged {
        from: PhysicalStateId,
        to: PhysicalStateId,
    },
    Landing,
    Wipeout,
    Contact {
        body: PhysicsBody,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhysicsEventBuffer {
    tick: u64,
    events: Vec<PhysicsEvent>,
}

impl PhysicsEventBuffer {
    pub fn new(tick: u64) -> Self {
        Self {
            tick,
            events: Vec::new(),
        }
    }
    pub fn tick(&self) -> u64 {
        self.tick
    }
    pub fn events(&self) -> &[PhysicsEvent] {
        &self.events
    }

    pub fn emit(&mut self, tick: u64, event: PhysicsEvent) -> Result<(), String> {
        if tick != self.tick {
            return Err(format!(
                "Physics event belongs to tick {tick}, buffer owns tick {}",
                self.tick
            ));
        }
        self.events.push(event);
        Ok(())
    }
}

/// Compact immutable publication consumed by animation, camera, rendering,
/// feedback and UI. Detailed native records remain owned by their subsystems;
/// this is the stable cross-phase summary.
#[derive(Clone, Debug, PartialEq)]
pub struct PhysicalOutputSnapshot {
    pub tick: u64,
    pub state: PhysicalStateId,
    pub board_position: Vector3,
    pub board_linear_velocity: Vector3,
    pub rider_root_position: Vector3,
    pub rider_linear_velocity: Vector3,
    /// Ground/contact results are published once by the solver and must not
    /// be reconstructed by camera, animation, or feedback consumers.
    pub ground_normal: Vector3,
    pub contact_count: u32,
    pub predicted_position: Vector3,
    pub grounded: bool,
    pub wiping_out: bool,
    pub landed: bool,
    pub events: Vec<PhysicsEvent>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_and_event_buffers_reject_stale_ticks() {
        let mut commands = PhysicsCommandBuffer::new(4);
        assert!(
            commands
                .push(
                    3,
                    PhysicsCommand::RequestState(PhysicalStateId::PhysicsGround)
                )
                .is_err()
        );
        assert!(
            commands
                .push(
                    4,
                    PhysicsCommand::RequestState(PhysicalStateId::PhysicsGround)
                )
                .is_ok()
        );
        let mut events = PhysicsEventBuffer::new(4);
        assert!(events.emit(5, PhysicsEvent::Landing).is_err());
        assert!(events.emit(4, PhysicsEvent::Landing).is_ok());
    }
}
