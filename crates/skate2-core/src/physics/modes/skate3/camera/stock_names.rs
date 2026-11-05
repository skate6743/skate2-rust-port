//! Host decoding of the collection-name hash stored in stock RefSpec values.
//! Same lookup8 byte format as the existing private XML extraction tools.
pub(super) fn lookup8(bytes: &[u8]) -> u64 {
    if bytes.is_empty() {
        return 0;
    }
    let mut a = 0xabcdef0011223344_u64;
    let mut b = a;
    let mut c = 0x9e3779b97f4a7c13_u64;
    let mut blocks = bytes.chunks_exact(24);
    for block in &mut blocks {
        a = a.wrapping_add(u64::from_le_bytes(block[..8].try_into().unwrap()));
        b = b.wrapping_add(u64::from_le_bytes(block[8..16].try_into().unwrap()));
        c = c.wrapping_add(u64::from_le_bytes(block[16..24].try_into().unwrap()));
        mix(&mut a, &mut b, &mut c);
    }
    c = c.wrapping_add(bytes.len() as u64);
    for (i, byte) in blocks.remainder().iter().enumerate() {
        match i {
            0..=7 => a = a.wrapping_add(u64::from(*byte) << (8 * i)),
            8..=15 => b = b.wrapping_add(u64::from(*byte) << (8 * (i - 8))),
            _ => c = c.wrapping_add(u64::from(*byte) << (8 * (i - 15))),
        }
    }
    mix(&mut a, &mut b, &mut c);
    c
}

fn mix(a: &mut u64, b: &mut u64, c: &mut u64) {
    *a = a.wrapping_sub(*b).wrapping_sub(*c) ^ (*c >> 43);
    *b = b.wrapping_sub(*c).wrapping_sub(*a) ^ (*a << 9);
    *c = c.wrapping_sub(*a).wrapping_sub(*b) ^ (*b >> 8);
    *a = a.wrapping_sub(*b).wrapping_sub(*c) ^ (*c >> 38);
    *b = b.wrapping_sub(*c).wrapping_sub(*a) ^ (*a << 23);
    *c = c.wrapping_sub(*a).wrapping_sub(*b) ^ (*b >> 5);
    *a = a.wrapping_sub(*b).wrapping_sub(*c) ^ (*c >> 35);
    *b = b.wrapping_sub(*c).wrapping_sub(*a) ^ (*a << 49);
    *c = c.wrapping_sub(*a).wrapping_sub(*b) ^ (*b >> 11);
    *a = a.wrapping_sub(*b).wrapping_sub(*c) ^ (*c >> 12);
    *b = b.wrapping_sub(*c).wrapping_sub(*a) ^ (*a << 18);
    *c = c.wrapping_sub(*a).wrapping_sub(*b) ^ (*b >> 22);
}
