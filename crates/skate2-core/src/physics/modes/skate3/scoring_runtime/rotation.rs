//! Skate 3 TU3 82DA8BE0, 82DAC880 and 82DAC9D0.
//! Track the skater root while a trick animates; otherwise track the board
//! projected against reckoning up. Deck flicks are not skater rotations.
pub(crate) type Basis = [[f32; 3]; 3];
const IDENTITY: Basis = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn normalized(v: [f32; 3]) -> Option<[f32; 3]> {
    let length = dot(v, v).sqrt();
    (length > 1e-6 && length.is_finite()).then(|| v.map(|x| x / length))
}
#[derive(Clone, Copy)]
struct Tracker {
    previous: Basis,
    angle: f32,
}
impl Tracker {
    fn reset(&mut self, basis: Basis) {
        self.previous = basis;
        self.angle = 0.;
    }
    fn update(&mut self, basis: Basis) {
        let up = self.previous[1];
        // Native double cross projects the new Z onto the previous Y plane.
        if let Some(forward) = normalized(cross(cross(up, basis[2]), up)) {
            let cosine = dot(forward, self.previous[2]);
            // 82DACB20 rejects a discontinuity beyond a quarter turn per tick.
            if cosine >= 0. {
                self.angle += dot(cross(self.previous[2], forward), up).atan2(cosine);
            }
        }
        self.previous = basis;
    }
}
pub(crate) struct Rotation {
    player: Tracker,
    board: Tracker,
    banked: f32,
    both: bool,
}
impl Default for Rotation {
    fn default() -> Self {
        Self {
            player: Tracker {
                previous: IDENTITY,
                angle: 0.,
            },
            board: Tracker {
                previous: IDENTITY,
                angle: 0.,
            },
            banked: 0.,
            both: false,
        }
    }
}
impl Rotation {
    pub fn reset(&mut self, player: Basis, board: Basis) {
        self.player.reset(player);
        self.board.reset(board);
        self.banked = 0.;
        self.both = false;
    }
    pub fn update(&mut self, player: Basis, board: Basis, up: [f32; 3], trick: bool) -> f32 {
        self.player.update(player);
        if trick {
            if self.both {
                self.banked += self.player.angle;
                self.player.reset(player);
                self.both = false;
            }
            self.board.reset(board);
        } else if self.both {
            // 82D2CDC0 builds an orthonormal frame from reckoning up/deck Z.
            if let Some(right) = normalized(cross(up, board[2])) {
                if let Some(forward) = normalized(cross(right, up)) {
                    self.board.update([right, up, forward]);
                }
            }
        } else {
            self.banked += self.player.angle;
            self.player.reset(player);
            self.board.reset(board);
            self.both = true;
        }
        self.banked + self.board.angle + if self.both { 0. } else { self.player.angle }
    }
}

#[cfg(any())]
mod tests {
    use super::*;
    pub fn yaw(degrees: f32) -> Basis {
        let (s, c) = degrees.to_radians().sin_cos();
        [[c, 0., -s], [0., 1., 0.], [s, 0., c]]
    }
    #[test]
    fn flick_rotation_is_excluded_and_root_spin_survives_heading_wrap() {
        let mut r = Rotation::default();
        r.reset(yaw(170.), yaw(170.));
        let mut angle = 0.;
        for step in 1..=72 {
            angle = r.update(
                yaw(170. + step as f32 * 5.),
                yaw(170. + step as f32 * 10.),
                [0., 1., 0.],
                true,
            );
        }
        assert!((angle.to_degrees() - 360.).abs() < 0.01);
        r.reset(IDENTITY, IDENTITY);
        for step in 1..=72 {
            angle = r.update(IDENTITY, yaw(step as f32 * 5.), [0., 1., 0.], true);
        }
        assert!(angle.abs() < 1e-6);
    }
    #[test]
    fn standalone_board_spin_is_not_double_counted_with_player() {
        let mut r = Rotation::default();
        r.reset(IDENTITY, IDENTITY);
        r.update(IDENTITY, IDENTITY, [0., 1., 0.], false);
        let mut angle = 0.;
        for step in 1..=36 {
            angle = r.update(
                yaw(step as f32 * 5.),
                yaw(step as f32 * 5.),
                [0., 1., 0.],
                false,
            );
        }
        assert!((angle.to_degrees() - 180.).abs() < 0.01);
    }
}
