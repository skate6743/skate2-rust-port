use super::*;
#[test]
fn reset_is_selective_not_full_default() {
    let mut words = [u32::MAX; 21];
    let mut gesture = 8;
    let mut bytes = [9; 4];
    motion_output(&mut words, &mut gesture, &mut bytes);
    assert_eq!(words[18], u32::MAX);
    assert_eq!(words[19], u32::MAX);
    assert_eq!(words[11], 0x7fffffff);
    assert_eq!(words[20], 0x03fbffff);
    assert_eq!(gesture, 37);
    assert_eq!(bytes, [0, 0, 9, 9]);
}
#[test]
fn given_stance_retains_lower_flags_and_consumes_request() {
    for (original, request, mirror, upper) in [
        (0, 0, false, 3),
        (0, 1, true, 0),
        (1, 0, false, 0),
        (1, 1, true, 3),
        (2, 0, true, 3),
    ] {
        let mut request = request;
        let mut flags = 0x1234;
        let mut mirrored = false;
        given_stance(original, &mut request, &mut flags, &mut mirrored);
        assert_eq!(mirrored, mirror);
        assert_eq!(flags, (upper << 30) | 0x1234);
        assert_eq!(request, 0);
    }
}

#[test]
fn playback_reset_preserves_unrelated_flags_and_resets_only_meaningful_request_words() {
    let mut flags = u32::MAX;
    let mut mirrored = true;
    let mut word_15320 = 17;
    skater_flags(&mut flags, &mut mirrored, &mut word_15320);
    assert_eq!(flags, 0x0ff7_ffff);
    assert!(!mirrored);
    assert_eq!(word_15320, 0);
    // The local-player bit controls later animation culling and must survive.
    assert_ne!(flags & 0x0800_0000, 0);
    let mut request = [u32::MAX; 4];
    let mut active = true;
    motion_request(&mut request, &mut active);
    assert_eq!(request, [0, 0x3e4c_cccd, 0, 0]);
    assert!(!active);
}
