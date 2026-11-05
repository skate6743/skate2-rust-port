//! Actual Ground entry82D37538. Numerical operations touch live board bodies;
//! the physical skeleton/air modifier remain calls to their separate owners.
use super::{
    super::{animation_input::AnimationInput, foot_ik::FootIk},
    GroundState,
};
use skate3_core::{
    math::Vector3,
    physics::{board_runtime::BoardRuntime, board_toolkit::BoardToolkit},
    player::input_phase::ProcessedPhysicsInput,
    riding::{
        ground_correction_math::dot_product,
        grounded::{
            manual_entry::{self, ManualGroundBodies, ManualGroundInput, ManualGroundProjection},
            state::motion,
        },
    },
};
use skate_data::collections::Collections;
use std::convert::Infallible;

pub(super) struct EntrySettings {
    deck_angular_drag: f32,
    powerslide_exit: f32,
    landing_strength: f32,
    landing_offset: f32,
}
impl EntrySettings {
    pub fn load(data: &Collections) -> Result<Self, String> {
        Ok(Self {
            //82C091F8 restores the standard angular drag with literal822F860C.
            deck_angular_drag: data.float("physicsdeck", "default", "DeckAngularDrag")?
                * f32::from_bits(0x426f_ffff),
            powerslide_exit: data.float("physics_manual", "default", "PowerslideExitScalar")?,
            landing_strength: data.float("physics_feet", "default", "LandingOnDeckEffectScalar")?,
            landing_offset: data.float("physics_feet", "default", "LandingOnDeckOffset")?,
        })
    }
}
///References to the actual lifecycle owners. These fields are not derived from
///controller input or cached separately in Ground. Collision and air-landing
///calls must apply the recovered physical owner behavior before returning.
pub(crate) struct GroundEntryTargets<'a> {
    pub foot_ik: &'a mut FootIk,
    pub skeleton_elapsed_16505: &'a mut bool,
    pub reckoning_spin_angle: &'a mut f32,
    pub reckoning_spin_speed: &'a mut f32,
    pub board_flags_8384: &'a mut u8,
    pub board_animated_290: &'a mut u8,
    pub wipeout_mode: &'a mut u32,
    pub wipeout_timer: &'a mut f32,
    pub set_skeleton_collision_state: &'a mut dyn FnMut(u32) -> Result<(), String>,
    pub enter_air_landing_modifier: &'a mut dyn FnMut(bool) -> Result<(), String>,
}
impl GroundState {
    pub fn enter(
        &mut self,
        board: &mut BoardRuntime,
        processed: &ProcessedPhysicsInput,
        animation: &AnimationInput,
        toolkit: &BoardToolkit,
        targets: GroundEntryTargets<'_>,
    ) -> Result<(), String> {
        restore_standard_board_fields(
            board,
            targets.board_flags_8384,
            self.entry_settings.deck_angular_drag,
        );
        targets.foot_ik.enable_feet(true);
        *targets.skeleton_elapsed_16505 = false;
        (targets.set_skeleton_collision_state)(6)?;
        self.state.begin_entry();
        *targets.reckoning_spin_angle = 0.0;
        *targets.reckoning_spin_speed = 0.0;
        board
            .hook_mut()
            .drive
            .disable_animation(targets.board_animated_290);
        let normal = processed.vectors_464_480_496_512_528[0].map(f32::from_bits);
        let angular = processed.vectors_720_784_800_816_832_864[0].map(f32::from_bits);
        board.bodies_mut()[6].rates.angular_velocity =
            motion::entry_angular_velocity(normal, angular);
        self.speed.target_speed = motion::entry_target_speed(
            board.bodies()[6].rates.linear_velocity,
            normal,
            toolkit.forward,
        );
        self.wobble.reset();
        manual_entry::enter_ground(
            &mut self.manual,
            processed.category_2516,
            self.entry_settings.powerslide_exit,
            ManualGroundInput {
                balance: animation.fields.balance,
                ground_normal: normal,
                flags_2468: processed.flags_2468,
                flags_2472: processed.flags_2472,
            },
            &mut ManualBodies(board),
            &mut Projection,
        )
        .unwrap_or_else(|never| match never {});
        if processed.category_2516 == 200 {
            let stance = ((processed.flags_2468 >> 20) ^ (processed.flags_2476 >> 2)) & 1 != 0;
            (targets.enter_air_landing_modifier)(stance)?;
        }
        if processed.state_2504 == 503 {
            let force = motion::landing_on_deck_force(
                processed.vectors_400_416[0].map(f32::from_bits),
                processed.vectors_544_560_592_608[3].map(f32::from_bits),
                processed.vectors_544_560_592_608[0].map(f32::from_bits),
                toolkit.total_mass,
                self.entry_settings.landing_strength,
                self.entry_settings.landing_offset,
            );
            board.forces_mut().append(force);
        }
        //The scalar assignment precedes pumping reset in source; no called
        //operation between those writes reads it.
        self.state.straighten_scale_2672 = if processed.state_2504 == 101 {
            f32::from_bits(0x3e23_d70a)
        } else {
            1.0
        };
        self.pumping.reset();
        if *targets.wipeout_mode != 1 {
            *targets.wipeout_timer = 0.0;
            *targets.wipeout_mode = 1;
        }
        self.state.finish_entry(processed.state_2504);
        self.entered = true;
        Ok(())
    }
}
/// Board-owned portion of SetStandard82C03AF8. 82C090D4/90EC write
/// assembly/part COLLISION groups, not rigid-body activation flags. Material
/// bindings and volume enables still require the live settings/volume owner.
fn restore_standard_board_fields(board: &mut BoardRuntime, flags: &mut u8, drag: f32) {
    *flags &= 0x7f;
    board.bodies_mut()[6].inertia.angular_drag = drag;
    board.set_collision_group(4);
}

#[cfg(any())]
#[path = "entry_restoration_tests.rs"]
mod restoration_tests;

struct ManualBodies<'a>(&'a mut BoardRuntime);
impl ManualGroundBodies for ManualBodies<'_> {
    fn linear_velocity(&mut self, part: usize) -> [f32; 4] {
        let v = self.0.bodies()[part].rates.linear_velocity;
        [v.x, v.y, v.z, 0.0]
    }
    fn set_linear_velocity(&mut self, part: usize, v: [f32; 4]) {
        self.0.bodies_mut()[part].rates.linear_velocity = Vector3::new(v[0], v[1], v[2]);
    }
}
struct Projection;
impl ManualGroundProjection for Projection {
    type Error = Infallible;
    fn normal_speed(&mut self, n: [f32; 4], v: [f32; 4]) -> Result<f32, Infallible> {
        Ok(dot_product(n, v))
    }
}
