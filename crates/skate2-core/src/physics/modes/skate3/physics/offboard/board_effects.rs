//! Board-controller effects applied to the existing board and hook bodies.
use skate3_core::{
    math::{Basis3, Vector3},
    physics::{board::BodyId, board_runtime::BoardRuntime, drive_frames::RetailAffineTransform},
    player::offboard::board_possession::{
        Frame, Vector,
        lifecycle::{Alignment, Effects},
    },
};

pub(crate) struct Policy {
    pub volumes_enabled: bool,
    pub released: bool,
    pub alignment: Option<Alignment>,
}
impl Default for Policy {
    fn default() -> Self {
        Self {
            volumes_enabled: true,
            released: false,
            alignment: None,
        }
    }
}
pub(crate) struct BoardEffects<'a> {
    pub board: &'a mut BoardRuntime,
    pub animated: &'a mut u8,
    pub policy: &'a mut Policy,
    pub standard_deck_drag: f32,
    pub timestep: f32,
}
fn xyz(v: Vector) -> Vector3 {
    Vector3::new(v[0], v[1], v[2])
}
fn transform(f: Frame) -> RetailAffineTransform {
    RetailAffineTransform {
        basis: Basis3 {
            columns: std::array::from_fn(|i| [f[i][0], f[i][1], f[i][2]]),
        },
        translation: xyz(f[3]),
    }
}
impl Effects for BoardEffects<'_> {
    fn enable_animation_soft(&mut self) {
        self.board
            .hook_mut()
            .drive
            .enable_animation_soft(self.animated);
    }
    fn enable_animation_angular_only(&mut self) {
        self.board
            .hook_mut()
            .drive
            .enable_angular_only(self.animated);
    }
    fn enable_animation_on_ground(&mut self) {
        self.board
            .hook_mut()
            .drive
            .enable_animation_on_ground(self.animated);
    }
    fn disable_animation(&mut self) {
        self.board.hook_mut().drive.disable_animation(self.animated);
    }
    fn disable_linear_drive(&mut self) {
        self.board.hook_mut().drive.disable_linear();
    }
    fn standard_board(&mut self) {
        self.policy.released = false;
        self.board.set_collision_group(4);
        self.board.bodies_mut()[BodyId::Deck.index()]
            .inertia
            .angular_drag = self.standard_deck_drag;
        self.policy.volumes_enabled = true;
    }
    fn released_board(&mut self) {
        self.policy.released = true;
        self.board.bodies_mut()[BodyId::Deck.index()]
            .inertia
            .angular_drag = f32::from_bits(0x416f_ffff);
        self.board.set_collision_group(7);
        self.policy.volumes_enabled = true;
    }
    fn collision_volumes(&mut self, enabled: bool) {
        self.policy.volumes_enabled = enabled;
    }
    fn clear_alignment(&mut self) {
        self.policy.alignment = None;
    }
    fn alignment(&mut self, value: Alignment) {
        self.policy.alignment = Some(value);
    }
    fn velocity(&mut self, value: Vector) {
        for body in self.board.bodies_mut() {
            body.rates.linear_velocity = xyz(value);
        }
    }
    fn position(&mut self, value: Vector) {
        let mut target = self.board.part_transforms()[BodyId::Deck.index()];
        target.translation = xyz(value);
        self.board.set_transform(target);
    }
    fn hook_frame(&mut self, value: Frame) {
        self.board.set_hook_transform(transform(value));
    }
    fn target_position_velocity(&mut self, value: Vector) {
        //82C04270 reads Body+16 (mass-frame position), not GetPartTransform.
        let origin = self.board.bodies()[BodyId::Deck.index()].rates.position;
        let frequency = 1.0 / self.timestep;
        self.velocity([
            (value[0] - origin.x) * frequency,
            (value[1] - origin.y) * frequency,
            (value[2] - origin.z) * frequency,
            0.,
        ]);
    }
    fn torque(&mut self, value: Vector) {
        //750F0 passes all three767D0 outputs to07328. They are requested
        //angular displacements, not direct torques or inverse-inertia products.
        skate3_core::physics::deck_angular_correction::apply_axis_displacement(
            &mut self.board.bodies_mut()[BodyId::Deck.index()].rates,
            xyz(value),
        );
    }
}

///82C08370..8428 classifies retained per-part normals against the controller's
///two directions. This is a release observation, not an extra force or joint.
pub(crate) fn classify_alignment(
    policy: &Policy,
    ground: &mut skate3_core::physics::board_ground::BoardGroundState,
) {
    let Some(a) = policy.alignment else {
        return;
    };
    for part in &ground.parts {
        if !part.in_contact {
            continue;
        }
        let normal = [part.normal.x, part.normal.y, part.normal.z, 0.];
        if [a.first_1008, a.second_1024].into_iter().any(|direction| {
            (normal[0] * -direction[0] + normal[1] * -direction[1]) + normal[2] * -direction[2]
                > a.factor_1040
        }) {
            ground.collision_flags |= 0x0400_0000;
        }
    }
}
