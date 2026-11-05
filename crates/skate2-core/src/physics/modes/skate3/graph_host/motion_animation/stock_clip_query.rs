//! Direct named-bank query82D16680, not the currently playing tree/channel.
use super::MotionAnimation;

impl MotionAnimation {
    pub(crate) fn stock_clip_translation_z(&self, database: &str, animation: &str) -> Result<f32, String> {
        //82BA7FA4/7FDC: missing bank/clip/attribute leaves the scalar zero.
        let Some(source) = self.metadata.source_for(animation) else { return Ok(0.0); };
        let bank = std::path::Path::new(&source.source_bank)
            .file_stem().and_then(|name| name.to_str()).unwrap_or("");
        if !bank.eq_ignore_ascii_case(database) { return Ok(0.0); }
        let clip = self.metadata.clip(animation)?;
        let Some(attribute) = clip.attributes.iter().find(|a| a.name.eq_ignore_ascii_case("AnimTransZ"))
            else { return Ok(0.0); };
        //82D16680 requests Attribute::Init82D164F8 at time0, irrespective of
        //the event's authored time range. Curve frame is therefore zero.
        match attribute.type_id {
            0 | 1 | 3 => attribute.payload_words.first().copied().map(f32::from_bits)
                .ok_or_else(|| format!("{database}/{animation}: truncated AnimTransZ")),
            2 => skate3_core::animation::playback_clip::sample_curve(&attribute.payload_words, 0.0),
            _ => Ok(0.0), //82D1659C: no payload write for other kinds.
        }
    }
}
