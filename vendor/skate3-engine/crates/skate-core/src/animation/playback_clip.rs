//! Stateful direct Andale Clip clock and attribute evaluation.
//! Init827B8AB0, AddTime82D25A00, GetAttributes82D25E30,
//! Attribute::Init82D164F8 and scalar curve lookup82D193E0.
use super::{
    clip_clock::{AdvanceResult, ClipClock},
    output::attributes::{AnimationAttribute, AttributeName, AttributePayload},
};

#[derive(Clone, Debug)]
pub struct ClipAttribute {
    pub name: AttributeName,
    pub kind: u8,
    pub begin: f32,
    pub end: f32,
    pub payload: Vec<u32>,
}

#[derive(Clone, Debug)]
pub struct PlaybackClip {
    pub clock: ClipClock,
    pub attributes: Vec<ClipAttribute>,
}
impl PlaybackClip {
    pub fn new(
        frames: f32,
        fps: f32,
        base_speed: f32,
        flags: u32,
        attributes: Vec<ClipAttribute>,
    ) -> Self {
        Self {
            clock: ClipClock {
                frames,
                fps,
                base_speed,
                speed: 1.0,
                length: (frames - 1.0) / ((1.0 * base_speed) * fps),
                time: 0.0,
                previous_time: 0.0,
                loops_since_evaluation: 0,
                looping: flags & 0x1000_0000 != 0,
                phase_controlled: flags & 0x4000_0000 != 0,
            },
            attributes,
        }
    }
    /// Animatable825310F0 resets the property before every AddTime call.
    pub fn advance(&mut self, dt: f32, phase: f32) -> AdvanceResult {
        let mut result = AdvanceResult {
            crossed_end: false,
            overshoot: -1.0,
            remaining_before_wrap: -1.0,
        };
        self.clock.advance(dt, phase, &mut result);
        result
    }
    /// Ordered GetAttributes. The untimed bulk path passes status4, unlike
    /// GetAttributeStatus's untimed status6; its times are multiplied by length.
    pub fn attributes(&self, mask: u32) -> Result<Vec<AnimationAttribute>, String> {
        let mut output = Vec::new();
        for attribute in &self.attributes {
            let status = if attribute.begin == -1.0 {
                4
            } else {
                self.clock.attribute_status(attribute.begin, attribute.end)
            };
            if attribute.begin != -1.0
                && (mask & 0x1c & u32::from(status) == 0 || mask & 3 & u32::from(status) == 0)
            {
                continue;
            }
            output.push(self.materialize(attribute, status)?);
        }
        Ok(output)
    }
    ///82D25CA0 uses status6 for untimed records and returns first valid match.
    pub fn attribute(
        &self,
        name: AttributeName,
        mask: u32,
    ) -> Result<Option<AnimationAttribute>, String> {
        for attribute in &self.attributes {
            if attribute.name != name {
                continue;
            }
            let status = self.clock.attribute_status(attribute.begin, attribute.end);
            if mask & 0x1c & u32::from(status) == 0 || mask & 3 & u32::from(status) == 0 {
                continue;
            }
            return self.materialize(attribute, status).map(Some);
        }
        Ok(None)
    }
    fn materialize(
        &self,
        attribute: &ClipAttribute,
        status: u8,
    ) -> Result<AnimationAttribute, String> {
        let mut payload = [None; 6];
        let lanes = match attribute.kind {
            0 => 1,
            1 => 4,
            3 => 6,
            _ => 0,
        };
        if attribute.payload.len() < lanes {
            return Err("Truncated clip attribute payload".into());
        }
        for i in 0..lanes {
            payload[i] = Some(attribute.payload[i]);
        }
        if attribute.kind == 2 {
            // The source deliberately multiplies time*fps*playback_speed;
            // base_speed is not an additional factor in Attribute::Init.
            let frame = (self.clock.time * self.clock.fps) * self.clock.speed;
            payload[0] = Some(sample_curve(&attribute.payload, frame)?.to_bits());
        }
        Ok(AnimationAttribute {
            name: attribute.name,
            kind: attribute.kind,
            payload: AttributePayload(payload),
            status,
            sequence_id: -1,
            begin_time: if status & 2 != 0 {
                -1.0
            } else {
                attribute.begin * self.clock.length
            },
            end_time: if status & 2 != 0 {
                -1.0
            } else {
                attribute.end * self.clock.length
            },
        })
    }
}

/// The curve record is an opaque word, point count, then(x,y) pairs. Native
/// binary search interpolates the final crossed bounds with a fused multiply.
pub fn sample_curve(words: &[u32], time: f32) -> Result<f32, String> {
    let count = *words.get(1).ok_or("Missing animation curve count")? as usize;
    if count == 0 || count > words.len().saturating_sub(2) / 2 {
        return Err("Truncated/empty animation curve".into());
    }
    let point = |index: usize| {
        (
            f32::from_bits(words[2 + 2 * index]),
            f32::from_bits(words[3 + 2 * index]),
        )
    };
    let mut low = 0;
    let mut high = count - 1;
    let (mut low_x, mut low_y) = point(low);
    let (mut high_x, mut high_y) = point(high);
    if !(time > low_x) {
        return Ok(low_y);
    }
    if !(time < high_x) {
        return Ok(high_y);
    }
    while !(low_x > high_x) {
        let middle = (low + high) / 2;
        let (x, y) = point(middle);
        if time > x {
            low = middle + 1;
            if low >= count {
                return Err("Malformed animation curve search bounds".into());
            }
            (low_x, low_y) = point(low);
        } else if time < x {
            high = middle
                .checked_sub(1)
                .ok_or("Malformed animation curve search bounds")?;
            (high_x, high_y) = point(high);
        } else {
            return Ok(y);
        }
    }
    let slope = (low_y - high_y) / (low_x - high_x);
    Ok(slope.mul_add(time - high_x, high_y))
}
