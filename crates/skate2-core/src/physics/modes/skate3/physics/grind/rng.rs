//! Host-owned orientation stream using recovered82970628 arithmetic.
//! Draw ownership/seed bookkeeping is host policy; do not claim original
//! cross-subsystem interleaving. No random draw occurs until AddNoise asks.
pub(super) struct OrientationRandom {
    words: [u32; 8],
}

impl OrientationRandom {
    pub(super) fn new() -> Self {
        //Use the existing recovered MotionRandom initialization, not an OS
        //seed or a fabricated substitute algorithm. Seed words821642B0.
        Self {
            words: [
                0,
                0,
                0xf22d_0e56,
                0x8831_26e9,
                0xc624_dd2f,
                0x0702_c49c,
                0x9e35_3f7d,
                0x6fdf_3b64,
            ],
        }
    }

    ///82D405C8/E0/F0: exactly three successive unsigned draws, in call order.
    ///The existing noise leaf owns modulo100000 and angle mapping.
    pub(super) fn take_three(&mut self) -> [u32; 3] {
        [self.next_u32(), self.next_u32(), self.next_u32()]
    }

    fn next_u32(&mut self) -> u32 {
        let words = &mut self.words;
        let last = words[7];
        let previous = words[6];
        let mut sum = previous.wrapping_add(last);
        let mut carry = u32::from(sum < last || sum < previous);
        words[6] = sum;
        for index in (2..=5).rev() {
            let previous = words[index];
            sum = previous.wrapping_add(sum).wrapping_add(carry);
            carry = u32::from(sum < previous);
            words[index] = sum;
        }
        words[7] = last.wrapping_add(1);
        if words[7] == 0 {
            for index in (2..=6).rev() {
                words[index] = words[index].wrapping_add(1);
                if words[index] != 0 {
                    break;
                }
            }
        }
        words[1] = words[1].wrapping_add(1);
        words[2]
    }
}
