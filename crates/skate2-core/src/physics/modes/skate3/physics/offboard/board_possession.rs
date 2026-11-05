//! Live board controller adapter. The caller borrows the existing board and
//! state448 owners; this module never advances a second physics island.
pub(crate) mod drives;
pub(crate) mod settings;
use skate3_core::{
    math::{Basis3, Vector3},
    physics::{
        board_runtime::BoardRuntime, contact::RetailContactMaterial,
        drive_frames::RetailAffineTransform,
    },
    player::offboard::board_possession::{
        Frame, Vector, angular,
        lifecycle::{Alignment, Effects as NativeEffects},
    },
};

/// Three shared shapes: deck aggregate A064, truck capsule A87C used by both
/// trucks A8B0/A8FC, wheel sphere AB94 assigned to all four wheels AC44.
/// Initialize from the actual authored shape flags, then collision generation
/// reads these live flags instead of immutable construction settings.
pub(crate) struct VolumeFlags {
    pub deck: bool,
    pub trucks: bool,
    pub wheels: bool,
    pub deck_children: Vec<bool>,
}

/// Borrow the actual three material slots read by colliders.rs. A detached
/// seven-element copy is not a live collision publication in this host.
pub(crate) struct Materials<'a> {
    pub wheels: &'a mut RetailContactMaterial,
    pub trucks: &'a mut RetailContactMaterial,
    pub deck: &'a mut RetailContactMaterial,
}
impl Materials<'_> {
    fn set(&mut self, values: [RetailContactMaterial; 3]) {
        *self.wheels = values[0];
        *self.trucks = values[1];
        *self.deck = values[2];
    }
}
impl VolumeFlags {
    pub(crate) fn set_enabled(&mut self, enabled: bool) {
        self.deck = enabled;
        self.trucks = enabled;
        self.wheels = enabled;
        self.deck_children.fill(enabled);
    }
}

pub(crate) struct Effects<'a> {
    pub board: &'a mut BoardRuntime,
    pub animated_290: &'a mut u8,
    pub wiping_out: &'a mut bool,
    pub alignment_active: &'a mut bool,
    pub alignment: &'a mut Alignment,
    pub volumes: &'a mut VolumeFlags,
    pub materials: Materials<'a>,
    /// Wheels, trucks, deck, from their original standard-material settings.
    pub standard_materials: [RetailContactMaterial; 3],
    pub released_material: RetailContactMaterial,
    ///091F8: actual8D3065CED858A7DF field multiplied by822F860C.
    pub standard_angular_drag: f32,
    pub processed_dt_2604: f32,
}
impl NativeEffects for Effects<'_> {
    fn enable_animation_soft(&mut self) {
        self.board
            .hook_mut()
            .drive
            .enable_animation_soft(self.animated_290);
    }
    fn enable_animation_angular_only(&mut self) {
        self.board
            .hook_mut()
            .drive
            .enable_angular_only(self.animated_290);
    }
    fn disable_animation(&mut self) {
        self.board
            .hook_mut()
            .drive
            .disable_animation(self.animated_290);
    }
    fn disable_linear_drive(&mut self) {
        self.board.hook_mut().drive.disable_linear();
    }
    fn standard_board(&mut self) {
        *self.wiping_out = false;
        self.board.bodies_mut()[6].inertia.angular_drag = self.standard_angular_drag;
        self.board.set_collision_group(4);
        self.materials.set(self.standard_materials);
        self.collision_volumes(true);
    }
    fn released_board(&mut self) {
        *self.wiping_out = true;
        self.board.bodies_mut()[6].inertia.angular_drag = f32::from_bits(0x416fffff);
        self.board.set_collision_group(7);
        self.materials.set([self.released_material; 3]);
        self.collision_volumes(true);
    }
    fn collision_volumes(&mut self, enabled: bool) {
        self.volumes.set_enabled(enabled);
    }
    fn clear_alignment(&mut self) {
        *self.alignment_active = false;
    }
    fn alignment(&mut self, value: Alignment) {
        *self.alignment = value;
        *self.alignment_active = true;
    }
    fn velocity(&mut self, value: Vector) {
        for body in self.board.bodies_mut() {
            body.rates.linear_velocity = xyz(value);
        }
    }
    fn position(&mut self, value: Vector) {
        let mut target = self.board.part_transforms()[6];
        target.translation = xyz(value);
        self.board.set_transform(target);
    }
    fn hook_frame(&mut self, value: Frame) {
        self.board.set_hook_transform(affine(value));
    }
    fn target_position_velocity(&mut self, value: Vector) {
        let center = self.board.bodies()[6].rates.position;
        let delta = [
            value[0] - center.x,
            value[1] - center.y,
            value[2] - center.z,
            0.,
        ];
        self.velocity(delta.map(|x| x * (1. / self.processed_dt_2604)));
    }
    fn torque(&mut self, value: Vector) {
        let deck = &mut self.board.bodies_mut()[6];
        let delta = angular::acceleration_delta(
            value,
            vector(deck.rates.angular_velocity),
            deck.rates
                .world_inverse_inertia
                .columns
                .map(|v| [v[0], v[1], v[2], 0.]),
        );
        let a = deck.rates.torque_acceleration;
        deck.rates.torque_acceleration =
            Vector3::new(a.x + delta[0], a.y + delta[1], a.z + delta[2]);
        deck.rates.cool_down = 0;
    }
}
pub(crate) fn xyz(v: Vector) -> Vector3 {
    Vector3::new(v[0], v[1], v[2])
}
pub(crate) fn vector(v: Vector3) -> Vector {
    [v.x, v.y, v.z, 0.]
}
pub(crate) fn affine(v: Frame) -> RetailAffineTransform {
    RetailAffineTransform {
        basis: Basis3 {
            columns: std::array::from_fn(|i| [v[i][0], v[i][1], v[i][2]]),
        },
        translation: xyz(v[3]),
    }
}
