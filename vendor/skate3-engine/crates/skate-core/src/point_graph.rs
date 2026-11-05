//! TU3 `PointGraph` scalar evaluator, `0x82481E10`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointGraph<const N: usize> {
    pub x: [f32; N],
    pub y: [f32; N],
}

impl<const N: usize> PointGraph<N> {
    /// Native endpoint handling and slope-first binary32 interpolation.
    pub fn evaluate(&self, input: f32) -> f32 {
        assert!(N > 0);
        if input < self.x[0] {
            return self.y[0];
        }
        // The native branch enters the scan only on an ordered less-than.
        // Unordered input or a NaN final key therefore selects the last value.
        if !(input < self.x[N - 1]) {
            return self.y[N - 1];
        }
        for upper in 1..N {
            if !(input < self.x[upper]) {
                continue;
            }
            let lower = upper - 1;
            let width = self.x[upper] - self.x[lower];
            if width <= 0.0 {
                return self.y[upper];
            }
            let slope = (self.y[upper] - self.y[lower]) / width;
            return slope.mul_add(input - self.x[lower], self.y[lower]);
        }
        self.y[0]
    }
}
