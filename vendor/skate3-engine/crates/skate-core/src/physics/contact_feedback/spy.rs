/// Complete TU3 82AE1608. Compacts eligible256-byte compiled contact rows into
///112-byte spies at the same buffer start. `spy_count` increments from its input
///value; it does not choose the write offset and is not reset by this routine.
/// `body_center` resolves the exact body pointer to its packed COM/ID vector.
pub fn spy_contact_jacobians(
    storage: &mut [u32],
    contact_count: u32,
    spy_count: &mut u32,
    frequency: f32,
    mut body_center: impl FnMut(u32) -> [u32; 4],
) {
    assert!(storage.len() >= contact_count as usize * 64);
    let squared_frequency = frequency * frequency;
    let mut write = 0;
    for index in 0..contact_count as usize {
        let start = index * 64;
        if storage[start + 11] & 8 == 0 {
            continue;
        }
        let impulse = vector(storage, start + 20);
        if !(impulse[0] > 0.0) {
            continue;
        }
        let a = storage[start + 31];
        let b = storage[start + 43];
        let b_center = body_center(b).map(f32::from_bits);
        let a_center = body_center(a).map(f32::from_bits);
        let a_position = vector(storage, start);
        let b_position = vector(storage, start + 4);
        let a_weight = f32::from_bits(storage[start + 35]);
        let b_weight = f32::from_bits(storage[start + 39]);
        let normal = vector(storage, start + 28);
        let tangent0 = vector(storage, start + 40);
        let tangent1 = vector(storage, start + 52);
        let tangent: [f32; 4] =
            std::array::from_fn(|i| tangent0[i].mul_add(impulse[1], tangent1[i] * impulse[2]));
        let inverse = 1.0 / (b_weight + a_weight);
        let position: [f32; 4] = std::array::from_fn(|i| {
            (a_center[i] + a_position[i])
                .mul_add(a_weight, (b_center[i] + b_position[i]) * b_weight)
                * inverse
        });
        let tag = storage[start + 55];
        // All source vectors/scalars are loaded before the in-place stores.
        storage[write + 24] = a;
        storage[write + 25] = b;
        storage[write + 26] = tag;
        for i in 0..4 {
            storage[write + 16 + i] = ((normal[i] * impulse[0]) * squared_frequency).to_bits();
            storage[write + 20 + i] = (tangent[i] * squared_frequency).to_bits();
            storage[write + i] = normal[i].to_bits();
            storage[write + 4 + i] = tangent0[i].to_bits();
            storage[write + 8 + i] = tangent1[i].to_bits();
            storage[write + 12 + i] = position[i].to_bits();
        }
        *spy_count = spy_count.wrapping_add(1);
        write += 28;
    }
}

fn vector(words: &[u32], offset: usize) -> [f32; 4] {
    std::array::from_fn(|i| f32::from_bits(words[offset + i]))
}
