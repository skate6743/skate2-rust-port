//! Original TU3 reset field writes. Borrow the canonical animation/MG owners.
///82B97308 ->82B972A8. This is the original stance enum comparison, not XOR
///of two guessed booleans; all original integer cases remain distinguishable.
pub fn given_stance(
    original_stance: u32,
    requested: &mut u32,
    flags: &mut u32,
    mirrored: &mut bool,
) {
    if (original_stance != 1 || *requested != 0) && (original_stance != 0 || *requested != 1) {
        *flags |= 0xc000_0000;
        *mirrored = original_stance != 0;
    } else {
        *flags &= 0x3fff_ffff;
        *mirrored = original_stance != 1;
    }
    *requested = 0;
}
///82B98050's two raw rlwinm masks; unrelated stance/request fields survive.
pub fn skater_flags(flags: &mut u32, mirrored: &mut bool, word_15320: &mut u32) {
    *flags &= 0x0ff7_ffff;
    *mirrored = false;
    *word_15320 = 0;
}
///FullMG5840..5920 inclusive, borrowed as its21 published scalar/flag words.
///Only the recovered writes occur. In particular5912/5916 survive.
pub fn motion_output(words: &mut [u32; 21], gesture: &mut u32, gesture_flags: &mut [u8; 4]) {
    for index in [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 14, 17] {
        words[index] = 0;
    }
    words[11] &= 0x7fff_ffff;
    words[13] &= 0x1fff_ffff;
    words[15] &= 0x1fff_ffff;
    words[16] &= 0x3fff_ffff;
    words[20] = (words[20] & 0x01fb_ffff) | 0x0200_0000;
    *gesture = 37;
    gesture_flags[0] = 0;
    gesture_flags[1] = 0;
}
#[cfg(test)]
mod tests;
///8258F360 writes four meaningful request words and clears last word bit31.
///The native last word's lower31 bits originate in uninitialized stack data;
///they are not assigned a fabricated deterministic payload by this API.
pub fn motion_request(words_2664_2676: &mut [u32; 4], active_2680: &mut bool) {
    *words_2664_2676 = [0, 0x3e4c_cccd, 0, 0];
    *active_2680 = false;
}
