use super::DerivedControllerInput;

#[test]
fn initialize_preserves_only_native_low_twenty_flag_bits() {
    let mut words = [0u32; 26];
    words[6] = 0xfff0_1234;
    words[13] = 0xabcd_5678;
    words[0] = 1;
    words[24] = 2;
    let mut input = DerivedControllerInput::from_words(words);
    input.initialize();
    assert_eq!(input.words()[6], 0x0000_1234);
    assert_eq!(input.words()[13], 0x000d_5678);
    for (index, &word) in input.words().iter().enumerate() {
        if !(index == 6 || index == 13 || (16..20).contains(&index)) {
            assert_eq!(word, 0, "word {index}");
        }
    }
    assert_eq!(&input.words()[16..20], &[0x7eff_ffff; 4]);
}
