//! Effective animation root82BE3650, called by Ground Enter at82D30848.
type Frame = [[f32; 4]; 4];

///The source multiplies every lane of X/Z by -1, not the position or up.
pub(super) fn effective_root(mut frame: Frame, flags_2476: u32) -> Frame {
    if flags_2476 & 4 != 0 {
        for axis in [0, 2] {
            frame[axis] = frame[axis].map(|v| v * -1.);
        }
    }
    frame
}

#[cfg(any())]
mod tests;
