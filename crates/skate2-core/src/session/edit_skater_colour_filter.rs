//! PC port addition: Apply Color, a colour filter on a shirt or pants with no colour of its own. Y on
//! such a part in its list wears it as A does and opens its wear styles menu on a row of the
//! clothing colours (`WearStylesFilter`), whose colour the garment's cloth shader multiplies its own
//! colours by. A part with a colour of its own opens its own colour row instead. The filter stays
//! with the garment it was picked for (`record_ops::TOP_FILTER`, `record_ops::BOTTOM_FILTER`). See
//! `docs/evidence/edit-skater-colour-filter-2026-11-14.md`.
use super::*;
use crate::ui::colour_picker::{self as pick, MAX_TINT};
use skate2_formats::skater_profile::CacPartId;

/// The colour on screen of a stored filter tint, and the other way round: the cloth shader squares
/// the diffuse it multiplies by the tint (`cas_colour`), and its tone curve takes the square root
/// again, so a tint makes the garment's own colours on screen its square root times as bright.
fn shown_by(tint: [f32; 3]) -> [f32; 3] {
    tint.map(|c| if c.is_finite() { c.max(0.0).sqrt() } else { 0.0 })
}
fn tint_for(colour: [f32; 3]) -> [f32; 3] {
    colour.map(|c| if c.is_finite() { (c.max(0.0) * c.max(0.0)).min(MAX_TINT) } else { 0.0 })
}

fn bound_to(colour: &skate2_formats::recipe::customizations::Colour, part: CacPartId) -> bool {
    CacPartId { model: colour.model_id, material: colour.material_id } == part
}

fn trace(detail: std::fmt::Arguments<'_>) {
    // SKATE2_DIAGNOSTIC_COLOUR_FILTER: what each change does to the colour filters.
    if std::env::var_os("SKATE2_DIAGNOSTIC_COLOUR_FILTER").is_some() {
        eprintln!("colour filter: {detail}");
    }
}

impl EditSkaterFrontend {
    /// The clothing colours (the tops' row) as filters, in the row's order: each colour's key and
    /// the colour on screen it makes a white garment, its swatch over the row's white's (the swatch
    /// whose least channel is largest), so white leaves a garment's own colours as they are.
    pub(super) fn filter_colours(&self) -> RecordResult<Vec<(u64, [f32; 3])>> {
        let mut out = Vec::new();
        for key in self.zone_colours(TOP_COLOUR)? {
            let colour = self.vault.instance(record_ops::COLOUR_CLASS, key);
            out.push((key, [word(&colour, 16)?, word(&colour, 20)?, word(&colour, 24)?].map(f32::from_bits)));
        }
        let least = |c: &[f32; 3]| c[0].min(c[1]).min(c[2]);
        let white = out
            .iter()
            .map(|(_, swatch)| *swatch)
            .max_by(|a, b| least(a).total_cmp(&least(b)))
            .ok_or("CAC colour list empty")?;
        for (_, swatch) in &mut out {
            *swatch = std::array::from_fn(|i| if white[i] > 0.0 { swatch[i] / white[i] } else { swatch[i] });
        }
        Ok(out)
    }

    /// The colour on screen filter `key` gives its garment's whites: white, its own colours as they
    /// are, when it has none.
    pub(super) fn worn_filter(&self, key: usize) -> RecordResult<[f32; 3]> {
        let (custom, _) = CharacterCustomizations::parse_prefix(self.record.customizations_bytes()?)?;
        Ok(record_ops::filter(&custom, key).map_or([1.0; 3], |colour| shown_by(colour.base_rgb_bits.map(f32::from_bits))))
    }

    /// The row of `filter_colours` filter `key` stands on: the colour nearest its own.
    pub(super) fn filter_position(&self, key: usize) -> RecordResult<usize> {
        let worn = self.worn_filter(key)?;
        let colours: Vec<[f32; 3]> = self.filter_colours()?.into_iter().map(|(_, colour)| colour).collect();
        pick::nearest(&colours, worn).ok_or_else(|| "CAC colour list empty".into())
    }

    /// Puts filter `key` on the garment its slot wears, giving its whites `colour` on screen; white
    /// takes it off.
    pub(super) fn set_filter_colour(&mut self, key: usize, colour: [f32; 3]) -> RecordResult<()> {
        let slot = record_ops::filter_part_slot(key).ok_or("CAC colour filter slot absent")?;
        let target = self.record.installed_parts[slot];
        let (mut custom, _) = CharacterCustomizations::parse_prefix(self.record.customizations_bytes()?)?;
        let tint = tint_for(colour);
        let base = (tint != [1.0; 3]).then(|| tint.map(f32::to_bits));
        record_ops::set_filter(&mut custom, key, base, target);
        self.record.replace_customizations(&custom.encode()?)?;
        match base {
            Some(_) => trace(format_args!("{key} on {:016x}/{:016x}: colour {colour:?}, tint {tint:?}", target.model, target.material)),
            None => trace(format_args!("{key} off {:016x}/{:016x}", target.model, target.material)),
        }
        Ok(())
    }

    /// A garment worn in another's place leaves the other's filter behind.
    pub(super) fn drop_unworn_filters(&mut self) -> RecordResult<()> {
        let (mut custom, _) = CharacterCustomizations::parse_prefix(self.record.customizations_bytes()?)?;
        if record_ops::retain_worn_filters(&mut custom, &self.record.installed_parts) {
            self.record.replace_customizations(&custom.encode()?)?;
            trace(format_args!("a garment no longer worn left its filter"));
        }
        Ok(())
    }

    /// The wear style worn in place of `previous` in `slot` (the Style row's): the filter of the
    /// garment goes on the style.
    pub(super) fn move_filter(&mut self, slot: u8, previous: CacPartId) -> RecordResult<()> {
        let Some(key) = record_ops::part_filter(slot) else { return Ok(()) };
        let (mut custom, _) = CharacterCustomizations::parse_prefix(self.record.customizations_bytes()?)?;
        let Some(base) = record_ops::filter(&custom, key).filter(|colour| bound_to(colour, previous)).map(|colour| colour.base_rgb_bits) else {
            return Ok(());
        };
        let worn = self.record.installed_parts[usize::from(slot)];
        record_ops::set_filter(&mut custom, key, Some(base), worn);
        self.record.replace_customizations(&custom.encode()?)?;
        trace(format_args!("{key} moved to the style {:016x}/{:016x}", worn.model, worn.material));
        Ok(())
    }

    /// The cart giving a slot back the undo point's garment (`update_receipt_list`) gives it back
    /// the undo point's filter as well.
    pub(super) fn restore_undo_filters(&mut self) -> RecordResult<()> {
        let (undo, _) = CharacterCustomizations::parse_prefix(self.undo.customizations_bytes()?)?;
        let (mut custom, _) = CharacterCustomizations::parse_prefix(self.record.customizations_bytes()?)?;
        let mut restored = false;
        for key in [record_ops::TOP_FILTER, record_ops::BOTTOM_FILTER] {
            let slot = record_ops::filter_part_slot(key).ok_or("CAC colour filter slot absent")?;
            let worn = self.record.installed_parts[slot];
            if worn != self.undo.installed_parts[slot] || record_ops::filter(&custom, key).is_some_and(|colour| bound_to(colour, worn)) {
                continue;
            }
            if let Some(base) = record_ops::filter(&undo, key).filter(|colour| bound_to(colour, worn)).map(|colour| colour.base_rgb_bits) {
                record_ops::set_filter(&mut custom, key, Some(base), worn);
                restored = true;
                trace(format_args!("{key} back on {:016x}/{:016x} with the cart's undo", worn.model, worn.material));
            }
        }
        if restored {
            self.record.replace_customizations(&custom.encode()?)?;
        }
        Ok(())
    }
}
