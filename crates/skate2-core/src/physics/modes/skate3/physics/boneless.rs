//! Physical state602, TU3 vtable82327434. Stock no-comply/boneless/fastplant
//! animations select this state with AnimSkateboard and launch with FootJump.
use super::footplant::math::*;
use super::{Skate3PhysicsState, Skate3ControllerState, air_phase, plant_skeleton};
use skate3_core::point_graph::PointGraph;
use skate_data::collections::Collections;

#[derive(Clone)]
pub(crate) struct Boneless {
    pub toe: usize,       //52
    pub anchor: [f32; 4], //64
    pub right: bool,      //80
    curves: [PointGraph<8>; 4],
}
pub(crate) struct LaunchInput {
    pub up:[f32;4],
    pub current_velocity:[f32;4],
    pub prepared_velocity:[f32;4],
    pub forward:[f32;4],
    pub centre_of_mass:[f32;4],
}
pub(crate) struct LaunchOutput {
    pub velocity:[f32;4],
    pub position:[f32;4],
}
impl Boneless {
    pub fn load(data: &Collections) -> Result<Self, String> {
        let graph = |name| -> Result<_, String> {
            let words = data
                .words::<20>("Hash_CCB95A83C78B4FF9", "default", name)?
                .map(f32::from_bits);
            Ok(PointGraph {
                x: words[4..12].try_into().unwrap(),
                y: words[12..20].try_into().unwrap(),
            })
        };
        Ok(Self {
            toe: 15,
            anchor: [0.0; 4],
            right: false,
            curves: [
                graph("Hash_6D781EFAF01E707D")?,
                graph("Hash_C9112CCD0BCB1850")?,
                graph("Hash_88D0CDEFA38A36D6")?,
                graph("Hash_9228B7F18C223F9C")?,
            ],
        })
    }
}

///82D4C900: anchor the last completed physical toe position.
pub(super) fn enter(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    skater.ground_lifecycle.board_animated_290 = 1;
    let drive = &mut physics.board.hook_mut().drive;
    //Globals12/912 at822F860C/822F8990; linear HardDrive, angular SoftDrive.
    Boneless::configure_drive(drive);
    let right = skater.player_input.processed.flags_2468 & (1 << 26) != 0;
    let toe=if right {19} else {15};
    skater.boneless.begin(right,skater.skeleton.record.pose[toe][3]);
    Ok(())
}

///82D4C9B8. Authored FootJump controls every launch request; no host timer.
pub(super) fn update(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    plant_skeleton::advance(
        physics,
        skater,
        skater.boneless.anchor,
        Some(skater.boneless.toe),
    )?;
    plant_skeleton::hold_foot(skater, skater.boneless.right, skater.boneless.anchor, 3);
    if skater.player_input.processed.flags_2480 & (1 << 13) != 0 {
        launch(physics, skater)?;
    }
    Ok(())
}

///82D4CB18/82D4CC18, four PointNegGraphData8 values from the stock schema.
fn launch(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    let p = &skater.player_input.processed;
    let up = p.vectors_544_560_592_608[0].map(f32::from_bits);
    let current = p.vectors_544_560_592_608[3].map(f32::from_bits);
    let prepared = p.prepared_jump_704.map(f32::from_bits);
    let output=skater.boneless.calculate_launch(LaunchInput {
        up,current_velocity:current,prepared_velocity:prepared,
        forward:p.effective_anim_transform_192[2].map(f32::from_bits),
        centre_of_mass:p.vectors_544_560_592_608[2].map(f32::from_bits),
    });
    let velocity=output.velocity;
    let position=output.position;
    let mut info = air_phase::launch_info(physics, skater)?;
    info.start_velocity = velocity;
    info.start_position_override = position;
    info.board_position_override = position;
    info.use_position_override = true;
    info.player_jumped = true;
    let input = air_phase::selector_input(physics, skater)?;
    skater.trajectory.launch(info, input, &physics.world)?;
    skater.trajectory.update(input, &physics.world, crate::skate3_native::physics::air_trajectory::GrindContext::from_processed(&skater.player_input.processed, crate::skate3_native::physics::solve::deck_frame(&physics.board)[3]))?;
    Ok(())
}

impl Boneless {
    ///82D4CB18/82D4CC18, independent of either player's body/solver ownership.
    pub(crate) fn calculate_launch(&self,input:LaunchInput)->LaunchOutput {
        let up=input.up;
        let current=input.current_velocity;
        let prepared=input.prepared_velocity;
        let forward=input.forward;
        let com=input.centre_of_mass;
    let horizontal = sub(prepared, scale(up, dot(prepared, up)));
    let speed = length(horizontal);
    let g = &self.curves;
    let vertical = g[0].evaluate(up[1]) * g[1].evaluate(speed);
    let scaled_speed = g[2].evaluate(speed) * speed;
    let a = madd(up, vertical - dot(current, up), current);
    let b = madd(normalize(horizontal), scaled_speed, scale(up, vertical));
    let blend = g[3].evaluate(up[1]);
    let mut velocity = madd(b, blend, scale(a, 1.0 - blend));
    if (length(velocity) - length(current)).abs() > 10.0 {
        velocity = current;
    }
    let component = dot(velocity, forward).max(0.0);
    if component < 1.92 {
        velocity = madd(forward, 1.92 - component, velocity);
    }
    let position = madd(up, dot(sub(self.anchor, com), up) + 0.25, com);
        LaunchOutput {velocity,position}
    }
    pub(crate) fn configure_drive(drive:&mut skate3_core::physics::hook_drive::HookDriveState) {
        drive.dynamics[..4].copy_from_slice(&[0x426f_ffff,0,0x4560_fffe,2]);
        drive.enable_angular_soft();
    }
    pub(crate) fn begin(&mut self,right:bool,anchor:[f32;4]) {
        self.toe=if right {19} else {15};
        self.anchor=anchor;
        self.right=right;
    }
}
