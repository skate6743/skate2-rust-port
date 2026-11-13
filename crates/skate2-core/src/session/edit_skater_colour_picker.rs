//! PC port addition: Edit Skater's colour picker (`crate::ui::colour_picker`) on a colour row whose
//! colours are tints, and the custom colour swatch a picked colour leaves after the row's own. See
//! `docs/evidence/edit-skater-colour-picker-2026-11-13.md`.
use super::*;
use crate::ui::colour_picker::{self as pick, Hsv, PickerInput, Primary, Tint};

/// A colour row's colour as a skater's customizations hold it: its shader base and highlight, and
/// the key of the row's own colour it stands for (`colour_ids`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ZoneColour {
    pub(super) key: u64,
    pub(super) base: [u32; 3],
    pub(super) highlight: [u32; 3],
}

impl ZoneColour {
    fn tint(&self) -> Tint {
        Tint { base: self.base.map(f32::from_bits), highlight: self.highlight.map(f32::from_bits) }
    }
}

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
    /// The row's swatch, converted as the row's view converts it.
    fn swatch_bytes(&self) -> [u8; 3] {
        self.swatch.map(|c| (c * 255.0) as u8)
    }
}

/// The tint a row's swatches show, which the picker sets.
fn row_primary(own: &[RowColour]) -> Primary {
    Primary::of_row(&own.iter().map(|c| (c.tint(), c.swatch)).collect::<Vec<_>>())
}

/// The colour a row's zone holds.
enum WornColour {
    /// The row's own colour at this index.
    Own(usize),
    /// A picked colour.
    Custom(ZoneColour),
}

/// What Left or Right does on a colour row (`adjust_colour`).
pub(super) enum ColourStep {
    /// The row's own rules: it is not a row of tints.
    Native,
    /// The row's own colour at this index.
    Own(usize),
    /// The custom colour, already on.
    Custom,
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
    option: EditSkaterOption,
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
    /// The custom colour `slot`'s row last held.
    fn remembered_colour(&self, slot: usize) -> Option<ZoneColour> {
        self.custom_colours.get(slot).copied().flatten()
    }
    fn remember_colour(&mut self, slot: usize, colour: ZoneColour) {
        if let Some(remembered) = self.custom_colours.get_mut(slot) {
            *remembered = Some(colour);
        }
    }

    /// A colour row's own colours.
    fn row_colours(&self, o: EditSkaterOption) -> RecordResult<Vec<RowColour>> {
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

    /// The colour `slot`'s zone holds: one of the row's own when its tints are that colour's.
    fn worn_colour(&self, slot: usize, own: &[RowColour]) -> RecordResult<WornColour> {
        let (custom, _) = CharacterCustomizations::parse_prefix(self.record.customizations_bytes()?)?;
        // The bottom zone's colour is added the first time one is picked; until then the row
        // stands on its first colour, as `colour_position` has it.
        let Some(worn) = custom.colours.iter().find(|c| c.key == slot as i32) else {
            return Ok(WornColour::Own(0));
        };
        if let Some(index) = own.iter().position(|c| c.base == worn.base_rgb_bits && c.highlight == worn.highlight_rgb_bits) {
            return Ok(WornColour::Own(index));
        }
        let key = self
            .record
            .colour_ids
            .get(slot)
            .copied()
            .or_else(|| own.first().map(|c| c.key))
            .ok_or("CAC colour list empty")?;
        Ok(WornColour::Custom(ZoneColour { key, base: worn.base_rgb_bits, highlight: worn.highlight_rgb_bits }))
    }

    /// A colour row of tints: its own colours' swatches, then the custom colour's (the picker's
    /// while it is open, else the one worn or last picked), and the swatch worn. False for another
    /// colour row, which keeps the row's own rules.
    pub(super) fn colour_row(&self, o: EditSkaterOption, row: &mut EditSkaterRow) -> RecordResult<bool> {
        if !self.colour_pickable(o)? {
            return Ok(false);
        }
        let slot = self.colour_slot(o)?;
        let own = self.row_colours(o)?;
        row.colours = own.iter().map(RowColour::swatch_bytes).collect();
        if let Some(picker) = self.colour_picker.as_ref().filter(|picker| picker.option == o) {
            row.colours.push(picker.colour.display_bytes());
            row.value = own.len() as i32;
            return Ok(true);
        }
        let primary = row_primary(&own);
        let swatch = |colour: &ZoneColour| Hsv::from_rgb(primary.of(colour.tint())).display_bytes();
        match self.worn_colour(slot, &own)? {
            WornColour::Own(index) => {
                row.value = index as i32;
                if let Some(custom) = self.remembered_colour(slot) {
                    row.colours.push(swatch(&custom));
                }
            }
            WornColour::Custom(custom) => {
                row.value = own.len() as i32;
                row.colours.push(swatch(&custom));
            }
        }
        Ok(true)
    }

    /// Left or Right on a colour row of tints, whose last swatch is the custom colour when there is
    /// one. A custom colour stepped off stays as that swatch.
    pub(super) fn colour_step(&mut self, o: EditSkaterOption, slot: usize, forward: bool) -> RecordResult<ColourStep> {
        if !self.colour_pickable(o)? {
            return Ok(ColourStep::Native);
        }
        let own = self.row_colours(o)?;
        let (at, custom) = match self.worn_colour(slot, &own)? {
            WornColour::Own(index) => (index, self.remembered_colour(slot)),
            WornColour::Custom(custom) => (own.len(), Some(custom)),
        };
        // FrontEndState_CaC::OnKey's clamp at either end of the strip, the custom swatch its last.
        let last = (own.len() + usize::from(custom.is_some())).saturating_sub(1);
        let next = if forward { (at + 1).min(last) } else { at.saturating_sub(1) };
        if let Some(custom) = custom {
            self.remember_colour(slot, custom);
            if next == own.len() {
                self.set_zone_colour(slot, custom.key, custom.base, custom.highlight)?;
                return Ok(ColourStep::Custom);
            }
        }
        Ok(ColourStep::Own(next))
    }

    /// A on a colour row of tints: the picker opens on the colour worn.
    pub(super) fn open_colour_picker(&mut self, o: EditSkaterOption) -> RecordResult<()> {
        let slot = self.colour_slot(o)?;
        let own = self.row_colours(o)?;
        let primary = row_primary(&own);
        let worn = match self.worn_colour(slot, &own)? {
            WornColour::Own(index) => own.get(index).ok_or("CAC colour list empty")?.tint(),
            WornColour::Custom(custom) => custom.tint(),
        };
        let colour = Hsv::from_rgb(primary.of(worn));
        let own = own
            .iter()
            .map(|c| {
                let tint = c.tint();
                OwnColour { key: c.key, primary: primary.of(tint), ratio: pick::ratio(primary.of(tint), primary.other(tint)) }
            })
            .collect();
        self.colour_picker = Some(ColourPicker {
            option: o,
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
                // A picked colour that is none of the row's own stays as the row's last swatch.
                if picker.changed {
                    let own = self.row_colours(picker.option)?;
                    if let WornColour::Custom(custom) = self.worn_colour(picker.slot, &own)? {
                        self.remember_colour(picker.slot, custom);
                    }
                }
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
    /// of the row's own colours has it, standing for that colour.
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
