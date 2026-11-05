use super::unpack;

#[test]
fn native_sign_bit_and_zero_width_do_not_consume_extra_selectors() {
    // Widths3,0,2: selector bits11, packed negative5=1010 and positive2=101.
    let values = unpack(&[2, 3, 0x5a], &[0x0203]).unwrap();
    assert_eq!(&values[0][..3], &[-5.0, 0.0, 2.0]);
    let negative_zero = unpack(&[1, 1, 0], &[1]).unwrap();
    assert_eq!(negative_zero[0][0].to_bits(), 0x8000_0000);
}

#[test]
fn packed_words_continue_across_the_native_vector_boundary() {
    // Nine width15 coefficients consume nine sign+magnitude16-bit words.
    let mut bytes = vec![9, 0xff, 1];
    bytes.extend_from_slice(&[
        0xff, 0xff, 2, 0, 5, 0, 6, 0, 9, 0, 10, 0, 13, 0, 14, 0, 17, 0,
    ]);
    let values = unpack(&bytes, &[0xffff_ffff, 15]).unwrap();
    assert_eq!(values[0], [32767.0, -1.0, 2.0, -3.0, 4.0, -5.0, 6.0, -7.0]);
    assert_eq!(values[1][0], 8.0);
    bytes.pop();
    assert!(unpack(&bytes, &[0xffff_ffff, 15]).is_err());
}
