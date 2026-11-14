//! PC port addition: Edit Skater's colour picker (`crate::ui::colour_picker`) on a colour row whose
//! colours are tints. The colour it picks goes on the skater alone: the row keeps its own colours,
//! its lit swatch the one nearest the colour worn. See
//! `docs/evidence/edit-skater-colour-picker-2026-11-13.md`.
use super::*;
use crate::ui::colour_picker::{self as pick, Hsv, PickerInput, Primary, Tint};

/// One of a colour row's own colours (`cac_colour`: its highlight at +0, its swatch at +16 and
/// its base at +32).
struct RowColour {
    key: u64,
    base: [u32; 3],
    highlight: [u32; 3],
    swatch: [f32; 3],
}

impl RowColour {
    fn tint(&self) -> Tint {
        Tint { base: self.base.map(f32::from_bits), highlight: self.highlight.map(f32::from_bits) }
    }
}

/// The tint a row's swatches show, which the picker sets.
fn row_primary(own: &[RowColour]) -> Primary {
    Primary::of_row(&own.iter().map(|c| (c.tint(), c.swatch)).collect::<Vec<_>>())
}

/// One of a row's own colours as the picker reads it: its key, its primary tint and its other
/// tint's ratio to that (`colour_picker::ratio`).
struct OwnColour {
    key: u64,
    primary: [f32; 3],
    ratio: [f32; 3],
}

/// The open colour picker.
pub(super) struct ColourPicker {
    slot: usize,
    colour: Hsv,
    /// The colour it opened on, which X goes back to.
    start: Hsv,
    primary: Primary,
    own: Vec<OwnColour>,
    /// The record as the picker found it, which B and X put back.
    original: CacRecord,
    /// The record holds a colour the picker set.
    changed: bool,
}

impl EditSkaterFrontend {
    /// A colour row's own colours. A colour filter's row holds its colours on screen as its tints
    /// (`edit_skater_colour_filter.rs`).
    fn row_colours(&self, o: EditSkaterOption) -> RecordResult<Vec<RowColour>> {
        if record_ops::filter_part_slot(self.colour_slot(o)?).is_some() {
            return Ok(self
                .filter_colours()?
                .into_iter()
                .map(|(key, colour)| RowColour { key, base: colour.map(f32::to_bits), highlight: [0; 3], swatch: colour })
                .collect());
        }
        let mut out = Vec::new();
        for key in self.colours(o)? {
            let colour = self.vault.instance(record_ops::COLOUR_CLASS, key);
            let words = |offset: usize| -> RecordResult<[u32; 3]> {
                Ok([word(&colour, offset)?, word(&colour, offset + 4)?, word(&colour, offset + 8)?])
            };
            out.push(RowColour { key, base: words(32)?, highlight: words(0)?, swatch: words(16)?.map(f32::from_bits) });
        }
        Ok(out)
    }

    /// Whether `o` is a colour row whose own colours tint their parts, which A opens the colour
    /// picker on. An eye or deck colour is a part of its own, with no tint.
    pub(super) fn colour_pickable(&self, o: EditSkaterOption) -> RecordResult<bool> {
        if o.definition().kind != "color" {
            return Ok(false);
        }
        Ok(self
            .row_colours(o)?
            .iter()
            .any(|c| c.base.iter().chain(&c.highlight).any(|&bits| f32::from_bits(bits) != 0.0)))
    }

    /// The tints `slot`'s zone wears: one of the row's own colours, or one the picker set.
    fn worn_tint(&self, slot: usize, own: &[RowColour]) -> RecordResult<Tint> {
        if record_ops::filter_part_slot(slot).is_some() {
            return Ok(Tint { base: self.worn_filter(slot)?, highlight: [0.0; 3] });
        }
        let (custom, _) = CharacterCustomizations::parse_prefix(self.record.customizations_bytes()?)?;
        // The bottom zone's colour is added the first time one is picked; until then the row
        // stands on its first colour, as `colour_position` has it.
        match custom.colours.iter().find(|c| c.key == slot as i32) {
            Some(worn) => Ok(Tint {
                base: worn.base_rgb_bits.map(f32::from_bits),
                highlight: worn.highlight_rgb_bits.map(f32::from_bits),
            }),
            None => Ok(own.first().ok_or("CAC colour list empty")?.tint()),
        }
    }

    /// A on a colour row of tints: the picker opens on the colour worn.
    pub(super) fn open_colour_picker(&mut self, o: EditSkaterOption) -> RecordResult<()> {
        let slot = self.colour_slot(o)?;
        let own = self.row_colours(o)?;
        let primary = row_primary(&own);
        let colour = Hsv::from_rgb(primary.of(self.worn_tint(slot, &own)?));
        let own = own
            .iter()
            .map(|c| {
                let tint = c.tint();
                OwnColour { key: c.key, primary: primary.of(tint), ratio: pick::ratio(primary.of(tint), primary.other(tint)) }
            })
            .collect();
        self.colour_picker = Some(ColourPicker {
            slot,
            colour,
            start: colour,
            primary,
            own,
            original: self.record.clone(),
            changed: false,
        });
        Ok(())
    }

    /// The open picker's update. A keeps the colour and closes it; B puts the colour it opened on
    /// back and closes it; X puts that colour back. The left stick and the D-pad move the cursor,
    /// the right stick and the triggers the intensity, and the skater wears the colour at once.
    pub(super) fn colour_picker_input(&mut self, input: EditSkaterInput, elapsed_ms: u32, e: &mut EditSkaterEffects) -> RecordResult<()> {
        if input.select || input.back {
            let picker = self.colour_picker.take().ok_or("CAC colour picker absent")?;
            self.stop_slider();
            if input.select {
                self.sounds.push(FeSoundRequest::Play("FE_SOUND_CORE_A_BUTTON"));
            } else {
                self.sounds.push(FeSoundRequest::Play("FE_SOUND_CORE_B_BUTTON"));
                if picker.changed {
                    self.record = picker.original;
                    e.recipe_changed = true;
                }
            }
            return Ok(());
        }
        if input.misc {
            self.stop_slider();
            self.sounds.push(FeSoundRequest::Play("FE_SOUND_CORE_X_BUTTON"));
            let picker = self.colour_picker.as_mut().ok_or("CAC colour picker absent")?;
            picker.colour = picker.start;
            if std::mem::take(&mut picker.changed) {
                self.record = picker.original.clone();
                e.recipe_changed = true;
            }
            return Ok(());
        }
        let [up, down, left, right] = input.dpad;
        let controls = PickerInput {
            stick: [input.morph_axis, input.left_stick_y],
            right_stick: input.right_stick,
            triggers: input.triggers,
            nudge: [i8::from(right) - i8::from(left), i8::from(down) - i8::from(up)],
        };
        for (pressed, sound) in input.dpad.into_iter().zip([
            "FE_SOUND_CORE_NAV_UP",
            "FE_SOUND_CORE_NAV_DOWN",
            "FE_SOUND_CORE_NAV_LEFT",
            "FE_SOUND_CORE_NAV_RIGHT",
        ]) {
            if pressed {
                self.sounds.push(FeSoundRequest::Play(sound));
            }
        }
        let picker = self.colour_picker.as_mut().ok_or("CAC colour picker absent")?;
        let colour = pick::step(picker.colour, &controls, elapsed_ms as f32 / 1000.0);
        let moved = colour != picker.colour;
        picker.colour = colour;
        if moved {
            self.wear_picked_colour()?;
            e.recipe_changed = true;
        }
        // As a morph's stick (OnUpdate 0x82670400): cas_slider loops while a stick or trigger
        // moves the colour, and stops at a bound or once they rest.
        if moved && controls.analog() {
            if !self.slider {
                self.slider = true;
                self.sounds.push(FeSoundRequest::Loop("FE_SOUND_CAS_SLIDER"));
            }
        } else {
            self.stop_slider();
        }
        Ok(())
    }

    /// The picker's colour on the skater: its primary tint, the other following it as the nearest
    /// of the row's own colours has it, whose key the zone's `colour_ids` takes (the row's lit
    /// swatch, and the colour name hair and skin parts match by).
    fn wear_picked_colour(&mut self) -> RecordResult<()> {
        let picker = self.colour_picker.as_mut().ok_or("CAC colour picker absent")?;
        let primary = picker.colour.rgb();
        let primaries: Vec<_> = picker.own.iter().map(|own| own.primary).collect();
        let nearest = &picker.own[pick::nearest(&primaries, primary).ok_or("CAC colour list empty")?];
        let tint = picker.primary.tint(primary, nearest.ratio);
        let (slot, key) = (picker.slot, nearest.key);
        picker.changed = true;
        self.set_zone_colour(slot, key, tint.base.map(f32::to_bits), tint.highlight.map(f32::to_bits))
    }

    /// The online game closing the editor for the player puts back the colour an open picker
    /// opened on, as B would.
    pub(crate) fn cancel_colour_picker(&mut self) {
        if let Some(picker) = self.colour_picker.take() {
            self.stop_slider();
            if picker.changed {
                self.record = picker.original;
            }
        }
    }

    /// What the open picker shows.
    pub(super) fn colour_picker_view(&self) -> Option<ColourPickerView> {
        self.colour_picker.as_ref().map(|picker| ColourPickerView { colour: picker.colour })
    }
}
