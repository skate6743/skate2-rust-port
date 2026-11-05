//! CameraPresentation manual cam (`SetManualCamMode` / `UpdateManualCam`).
use super::manager_frame::CameraFrame;
use crate::input::gameplay_map::GameplayActions;
use crate::math::Basis3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ManualCamSettings {
    /// Settings-object +100 deadzone for cubed analog input.
    pub deadzone: f32,
    /// Settings-object +76 move scale (m/s).
    pub move_speed: f32,
    /// Settings-object +124 look scale (rad/s).
    pub look_speed: f32,
    /// Trigger vertical speed (m/s).
    pub vertical_speed: f32,
    /// L3 / stick-click fast multiplier.
    pub fast_multiplier: f32,
}

impl Default for ManualCamSettings {
    fn default() -> Self {
        Self {
            deadzone: 0.01,
            move_speed: 8.0,
            look_speed: 1.75,
            vertical_speed: 6.0,
            fast_multiplier: 3.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ManualCamMatrix {
    pub right: [f32; 3],
    pub up: [f32; 3],
    pub at: [f32; 3],
    pub position: [f32; 3],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ManualCam {
    pub active: bool,
    pub matrix: ManualCamMatrix,
    speed_multiplier: f32,
}

impl Default for ManualCam {
    fn default() -> Self {
        Self {
            active: false,
            matrix: ManualCamMatrix {
                right: [1.0, 0.0, 0.0],
                up: [0.0, 1.0, 0.0],
                at: [0.0, 0.0, 1.0],
                position: [0.0, 0.0, 0.0],
            },
            speed_multiplier: 1.0,
        }
    }
}

impl ManualCam {
    /// `CameraPresentation+0xF50` — manual cam active flag.
    pub fn manual_cam_active(&self) -> bool {
        self.active
    }

    /// `SetManualCamMode`: enabling snapshots `GetMatrix` into `+3696`.
    pub fn set_manual_cam_mode(&mut self, enable: bool, normal: Option<&CameraFrame>) {
        if enable {
            if !self.active {
                if let Some(frame) = normal {
                    self.matrix = matrix_from_frame(frame);
                    self.speed_multiplier = 1.0;
                    self.active = true;
                }
            }
        } else if self.active {
            self.active = false;
        }
    }

    /// `GetMatrix`: manual matrix at `+3696` when the flag is set, else the normal frame.
    pub fn get_matrix(&self, normal: &CameraFrame) -> CameraFrame {
        if self.active {
            self.to_camera_frame(normal)
        } else {
            *normal
        }
    }

    pub fn spawn_matrix(&self) -> [[f32; 4]; 4] {
        let m = self.matrix;
        [
            [m.right[0], m.right[1], m.right[2], 0.0],
            [m.up[0], m.up[1], m.up[2], 0.0],
            [m.at[0], m.at[1], m.at[2], 0.0],
            [m.position[0], m.position[1], m.position[2], 1.0],
        ]
    }

    pub fn to_camera_frame(&self, source: &CameraFrame) -> CameraFrame {
        let [right, up, at] = [self.matrix.right, self.matrix.up, self.matrix.at];
        CameraFrame {
            basis: Basis3 {
                columns: [right, up, at],
            },
            position: [
                self.matrix.position[0],
                self.matrix.position[1],
                self.matrix.position[2],
                0.0,
            ],
            previous_basis: source.previous_basis,
            previous_position: source.previous_position,
            linear_velocity: [0.0; 4],
            angular_velocity: [0.0; 4],
            shake_translation: [0.0; 4],
            discontinuity: false,
            field_of_view_degrees: source.field_of_view_degrees,
            opacity: source.opacity,
            blur: source.blur,
        }
    }

    /// `UpdateManualCam` using stock gameplay analog getters (actions 64+).
    pub fn update_manual_cam(
        &mut self,
        actions: &GameplayActions,
        settings: ManualCamSettings,
        dt: f32,
    ) {
        if !self.active || dt <= 0.0 {
            return;
        }
        let values = actions.values();
        let strafe = cubic(values[0], settings.deadzone);
        let forward = cubic(values[1], settings.deadzone);
        let fast = values[2] > 0.5;
        let look_x = cubic(-values[3], settings.deadzone);
        let look_y = cubic(-values[4], settings.deadzone);
        let vertical = values[7] - values[6];

        let target_speed = if fast { settings.fast_multiplier } else { 1.0 };
        self.speed_multiplier = smooth_speed(self.speed_multiplier, target_speed, dt);
        let speed = settings.move_speed * self.speed_multiplier * dt;

        let mut right = self.matrix.right;
        let mut at = self.matrix.at;
        let mut pos = self.matrix.position;

        if forward.abs() > 0.0 {
            pos = add(pos, scale(at, forward * speed));
        }
        if strafe.abs() > 0.0 {
            let side = normalize(cross(at, [0.0, 1.0, 0.0]));
            pos = add(pos, scale(side, strafe * speed));
        }
        if vertical.abs() > 0.0 {
            pos[1] += vertical * settings.vertical_speed * dt;
        }

        let look = settings.look_speed * dt;
        if look_x.abs() > 0.0 {
            let yaw = rotate_axis([0.0, 1.0, 0.0], look_x * look);
            right = mul_mat_vec(yaw, right);
            at = mul_mat_vec(yaw, at);
        }
        if look_y.abs() > 0.0 {
            let pitch = rotate_axis(right, look_y * look);
            at = mul_mat_vec(pitch, at);
        }

        at = normalize(at);
        right = normalize(cross([0.0, 1.0, 0.0], at));
        if length(right) < 1.0e-4 {
            right = normalize(self.matrix.right);
        }
        let up = normalize(cross(at, right));
        right = normalize(cross(up, at));

        self.matrix = ManualCamMatrix {
            right,
            up,
            at,
            position: pos,
        };
    }
}

fn matrix_from_frame(frame: &CameraFrame) -> ManualCamMatrix {
    let [right, up, at] = frame.basis.columns;
    ManualCamMatrix {
        right,
        up,
        at,
        position: [frame.position[0], frame.position[1], frame.position[2]],
    }
}

fn cubic(value: f32, deadzone: f32) -> f32 {
    let cubed = value * value * value;
    if cubed.abs() > deadzone { cubed } else { 0.0 }
}

fn smooth_speed(current: f32, target: f32, dt: f32) -> f32 {
    // Native +3888 smoothing uses ~0.1 blend per frame at 60 Hz.
    let blend = (0.1 * dt * 60.0).clamp(0.0, 1.0);
    current + (target - current) * blend
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn scale(v: [f32; 3], s: f32) -> [f32; 3] {
    [v[0] * s, v[1] * s, v[2] * s]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn length(v: [f32; 3]) -> f32 {
    dot(v, v).sqrt()
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = length(v);
    if len > 1.0e-6 { scale(v, 1.0 / len) } else { v }
}

fn rotate_axis(axis: [f32; 3], angle: f32) -> [[f32; 3]; 3] {
    let axis = normalize(axis);
    let (x, y, z) = (axis[0], axis[1], axis[2]);
    let (s, c) = angle.sin_cos();
    let t = 1.0 - c;
    [
        [t * x * x + c, t * x * y - s * z, t * x * z + s * y],
        [t * x * y + s * z, t * y * y + c, t * y * z - s * x],
        [t * x * z - s * y, t * y * z + s * x, t * z * z + c],
    ]
}

fn mul_mat_vec(m: [[f32; 3]; 3], v: [f32; 3]) -> [f32; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cubic_respects_deadzone() {
        assert_eq!(cubic(0.5, 0.01), 0.125);
        assert_eq!(cubic(0.05, 0.01), 0.0);
    }

    #[test]
    fn enter_snapshots_frame_basis() {
        let frame = CameraFrame {
            basis: Basis3 {
                columns: [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]],
            },
            position: [3.0, 4.0, 5.0, 0.0],
            previous_basis: Basis3 {
                columns: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            },
            previous_position: [0.0; 4],
            linear_velocity: [0.0; 4],
            angular_velocity: [0.0; 4],
            shake_translation: [0.0; 4],
            discontinuity: false,
            field_of_view_degrees: 55.0,
            opacity: 1.0,
            blur: 1.0,
        };
        let mut manual = ManualCam::default();
        manual.set_manual_cam_mode(true, Some(&frame));
        assert!(manual.active);
        assert_eq!(manual.matrix.position, [3.0, 4.0, 5.0]);
        assert!((manual.matrix.at[0] - 1.0).abs() < 0.001);
    }

    #[test]
    fn forward_moves_along_at() {
        let mut manual = ManualCam::default();
        manual.active = true;
        manual.matrix.at = [0.0, 0.0, 1.0];
        let actions = GameplayActions::from_values([
            0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            0.0,
        ]);
        manual.update_manual_cam(&actions, ManualCamSettings::default(), 1.0);
        assert!(manual.matrix.position[2] > 0.0);
    }
}
