//! Attribute copying and ordered packet merge from TU3 8258EED0/8258F210.
//! Binary names stay at this typed boundary. This module does not interpret
//! Skeleton attributes or invent a physics effect from their names.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttributeName(pub [u32; 5]);

/// The six union lanes retain initialization separately from value. Copying a
/// scalar must not fabricate values for the other five lanes. Words preserve
/// all floating-point payload bits across non-arithmetic publication.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttributePayload(pub [Option<u32>; 6]);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimationAttribute {
    pub payload: AttributePayload,
    pub begin_time: f32,
    pub end_time: f32,
    pub name: AttributeName,
    pub status: u8,
    /// Native+53:0/2 scalar,1 vector,3 six-word payload; other kinds copy none.
    pub kind: u8,
    pub sequence_id: i32,
}

impl AnimationAttribute {
    /// Complete observable field assignment8258EED0. Native padding at54/55
    /// and60..63 is not copied and has no represented domain field.
    pub fn copy_from(&mut self, source: &Self) {
        self.kind = source.kind;
        self.name = source.name;
        self.begin_time = source.begin_time;
        self.end_time = source.end_time;
        self.status = source.status;
        self.sequence_id = source.sequence_id;
        let lanes = match self.kind {
            0 | 2 => 1,
            1 => 4,
            3 => 6,
            _ => 0,
        };
        self.payload.0[..lanes].copy_from_slice(&source.payload.0[..lanes]);
    }

    /// Copy-construction8258EE68 initializes name storage then performs the
    /// assignment above. Its inactive union lanes are unspecified, not zero.
    fn copy_construct(source: &Self) -> Self {
        let mut result = Self {
            payload: AttributePayload([None; 6]),
            ..*source
        };
        result.copy_from(source);
        result
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotionGraphAttribute {
    /// Native24-byte MG record:20-byte name followed by scalar.
    pub name: AttributeName,
    pub value: f32,
}
impl MotionGraphAttribute {
    ///8258F210 makes an untimed scalar attribute with status6 and sequence-1.
    pub fn to_animation(self) -> AnimationAttribute {
        let mut payload = AttributePayload([None; 6]);
        payload.0[0] = Some(self.value.to_bits());
        AnimationAttribute {
            payload,
            begin_time: -1.0,
            end_time: -1.0,
            name: self.name,
            status: 6,
            kind: 0,
            sequence_id: -1,
        }
    }
}

/// Active entries plus retained slots model native end=begin list reset.
/// Capacity allocation policy is host-owned; active ordering and union-lane
/// preservation follow native append/copy semantics. No sorting/deduplication.
#[derive(Default)]
pub struct PacketAttributes {
    slots: Vec<AnimationAttribute>,
    active_len: usize,
}
impl PacketAttributes {
    pub fn entries(&self) -> &[AnimationAttribute] {
        &self.slots[..self.active_len]
    }
    pub fn clear(&mut self) {
        self.active_len = 0;
    }
    pub fn append(&mut self, source: &AnimationAttribute) {
        if let Some(slot) = self.slots.get_mut(self.active_len) {
            slot.copy_from(source);
        } else {
            self.slots.push(AnimationAttribute::copy_construct(source));
        }
        self.active_len += 1;
    }

    /// Exact list operation order at82593750..825937E4: discard prior active
    /// list, append converted MG records, then tree records in stored order.
    /// Duplicate names remain distinct records for the later Skeleton dispatch.
    pub fn replace_from(
        &mut self,
        motion_graph: &[MotionGraphAttribute],
        tree: &[AnimationAttribute],
    ) {
        self.clear();
        for attribute in motion_graph {
            self.append(&attribute.to_animation());
        }
        for attribute in tree {
            self.append(attribute);
        }
    }
}
