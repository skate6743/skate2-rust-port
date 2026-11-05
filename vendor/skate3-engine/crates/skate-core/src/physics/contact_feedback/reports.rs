/// Registered target for a rigid-body pointer (Island+80 lookup).
#[derive(Clone, Copy, Debug)]
pub struct ContactTarget {
    pub body: u32,
    pub owner: u32,
    pub tag: u32,
}

/// Supplied storage preserves every unwritten byte when records are reused.
pub struct ContactReportBuffer {
    pub count: u32,
    pub records: [[u8; 96]; 16],
}

/// Container/locking boundary of 827682B0. The map operation inserts an absent
/// Body key; clearing resets existing vector ends without erasing keys/storage.
/// Allocated report slots and their padding are supplied by the backend.
pub trait ContactReportBackend {
    fn lock(&mut self);
    fn clear_report_counts(&mut self);
    fn lookup_target(&mut self, rigid_body: u32) -> Option<ContactTarget>;
    fn get_or_insert_reports(&mut self, body: u32) -> usize;
    fn reports(&mut self, handle: usize) -> &mut ContactReportBuffer;
    fn linear_velocity(&mut self, rigid_body: u32) -> [u32; 4];
    fn unlock(&mut self);
}

/// Complete 82768250 lookup lifetime: lock, operator[] (possibly inserts),
/// unlock, return the live vector handle. No detached report copy is produced.
pub fn get_contact_reports(body: u32, backend: &mut impl ContactReportBackend) -> usize {
    backend.lock();
    let handle = backend.get_or_insert_reports(body);
    backend.unlock();
    handle
}

/// Live tree traversal and Body virtual+8 dispatch boundary of 827685C8.
/// The backend retains native map order and resolves next after each callback.
pub trait ContactReportDispatchBackend {
    fn first_report_entry(&mut self) -> Option<usize>;
    fn next_report_entry(&mut self, handle: usize) -> Option<usize>;
    fn report_count(&mut self, handle: usize) -> u32;
    fn dispatch_reports(&mut self, handle: usize);
}

/// Complete 827685C8. Visits every map entry, invokes only nonempty live vectors,
/// and performs no list clearing or local locking around callbacks.
pub fn resolve_contact_reports(backend: &mut impl ContactReportDispatchBackend) {
    let mut entry = backend.first_report_entry();
    while let Some(handle) = entry {
        if backend.report_count(handle) != 0 {
            backend.dispatch_reports(handle);
        }
        entry = backend.next_report_entry(handle);
    }
}

/// Complete 827682B0 report construction/filter/order, with its native map and
/// lock operations supplied by the backend. `spy_base` retains native pointer
/// identity; reports borrow that simulation storage through the consuming phase.
pub fn create_contact_reports(
    spies: &[[u32; 28]],
    spy_base: u32,
    backend: &mut impl ContactReportBackend,
) {
    backend.lock();
    backend.clear_report_counts();
    for (index, spy) in spies.iter().enumerate() {
        let a = backend.lookup_target(spy[24]);
        let b = backend.lookup_target(spy[25]);
        let a_body = a.map_or(0, |target| target.body);
        let b_body = b.map_or(0, |target| target.body);
        let a_owner = a.map_or(0, |target| target.owner);
        let b_owner = b.map_or(0, |target| target.owner);
        let same_body = a.is_some() && b.is_some() && a_body == b_body;
        let excluded = same_body || (a_owner != 0 && a_owner == b_owner);
        for (target, is_a) in [(a, true), (b, false)] {
            let Some(target) = target else {
                continue;
            };
            // Native operator[] runs even for a subsequently excluded pair.
            let handle = backend.get_or_insert_reports(target.body);
            let count = backend.reports(handle).count;
            if count >= 16 || excluded {
                continue;
            }
            let other = if is_a { spy[25] } else { spy[24] };
            let this = if is_a { spy[24] } else { spy[25] };
            let other_velocity = backend.linear_velocity(other).map(f32::from_bits);
            let this_velocity = backend.linear_velocity(this).map(f32::from_bits);
            let normal: [u32; 4] = std::array::from_fn(|i| {
                (f32::from_bits(spy[i]) * if is_a { 1.0 } else { -1.0 }).to_bits()
            });
            let relative: [u32; 4] =
                std::array::from_fn(|i| (this_velocity[i] - other_velocity[i]).to_bits());
            let buffer = backend.reports(handle);
            let record = &mut buffer.records[count as usize];
            put(
                record,
                0,
                spy_base.wrapping_add((index as u32).wrapping_mul(112)),
            );
            for i in 0..4 {
                put(record, 16 + i * 4, normal[i]);
                put(record, 32 + i * 4, spy[12 + i]);
                put(record, 48 + i * 4, relative[i]);
            }
            put(record, 64, target.tag);
            put(
                record,
                68,
                if is_a {
                    spy[26] & 0xffff
                } else {
                    spy[26] >> 16
                },
            );
            put(record, 72, target.body);
            put(record, 76, if is_a { b_body } else { a_body });
            record[80] = 0;
            record[81] = u8::from(is_a);
            buffer.count += 1;
        }
    }
    backend.unlock();
}

fn put(record: &mut [u8; 96], offset: usize, value: u32) {
    record[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}
