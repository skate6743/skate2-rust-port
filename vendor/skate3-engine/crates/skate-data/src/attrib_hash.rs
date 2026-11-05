//! AttribSys lookup8 identity. Numeric names preserve fields without debug strings.
fn mix(mut a: u64, mut b: u64, mut c: u64) -> (u64, u64, u64) {
    for (x, y, z) in [(43, 9, 8), (38, 23, 5), (35, 49, 11), (12, 18, 22)] {
        a = a.wrapping_sub(b).wrapping_sub(c) ^ (c >> x);
        b = b.wrapping_sub(c).wrapping_sub(a) ^ (a << y);
        c = c.wrapping_sub(a).wrapping_sub(b) ^ (b >> z);
    }
    (a, b, c)
}
pub fn hash(text: &str) -> u64 {
    if text.is_empty() {
        return 0;
    }
    let mut a = 0xabcdef0011223344u64;
    let mut b = a;
    let mut c = 0x9e3779b97f4a7c13u64;
    let mut chunks = text.as_bytes().chunks_exact(24);
    for chunk in &mut chunks {
        let x = u64::from_le_bytes(chunk[..8].try_into().unwrap());
        let y = u64::from_le_bytes(chunk[8..16].try_into().unwrap());
        let z = u64::from_le_bytes(chunk[16..].try_into().unwrap());
        (a, b, c) = mix(a.wrapping_add(x), b.wrapping_add(y), c.wrapping_add(z));
    }
    c = c.wrapping_add(text.len() as u64);
    for (i, byte) in chunks.remainder().iter().enumerate() {
        match i {
            0..=7 => a = a.wrapping_add(u64::from(*byte) << (i * 8)),
            8..=15 => b = b.wrapping_add(u64::from(*byte) << ((i - 8) * 8)),
            _ => c = c.wrapping_add(u64::from(*byte) << ((i - 15) * 8)),
        }
    }
    mix(a, b, c).2
}
pub fn numeric_name(text: &str) -> String {
    let key = text
        .strip_prefix("Hash_")
        .or_else(|| text.strip_prefix("0x"))
        .and_then(|hex| u64::from_str_radix(hex, 16).ok())
        .unwrap_or_else(|| hash(text));
    format!("Hash_{key:016X}")
}
