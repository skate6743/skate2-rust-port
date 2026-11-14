//! FE state 24: Extras (82653150 / 82669830), native cheat table 8208b438.
//! PC replaces state 75's console keyboard with an editable in-game text field.
use super::pause_menu::PauseMenuInput;

pub const LABELS: [&str; 7] = [
    "ID_EXTRA_PROMO_CODE",
    "ID_EXTRA_CREDITS",
    "ID_EXTRA_VIDEO_1",
    "ID_EXTRA_VIDEO_2",
    "ID_EXTRA_VIDEO_4",
    "ID_EXTRA_VIDEO_5",
    "ID_EXTRA_VIDEO_3",
];
/// OnMenuSelect's original 64-bit keys, checked in PPC instructions (8266989c..978).
pub const MOVIES: [u64; 5] = [
    0x4435_4e4f_d31d_ae34,
    0x4ff5_292a_7dfb_6081,
    0x1970_dd9e_472a_7126,
    0x3404_9abc_e23c_10ac,
    0xfce8_a97a_f24d_f012,
];
pub const CREDITS_MOVIE: u64 = 0x0bdb_6ab1_cb80_3a1b;
/// PC port addition: Max Out Save, the menu's last row (`selected_session_extras.rs`).
pub const MAX_OUT: &str = "#Max Out Save";
/// Its confirmation, the progression popup's (default No).
pub const MAX_OUT_QUESTION: &str = "#Are you sure you want to max out this savegame? This action unlocks everything in storyline and can't be undone unless you have a backup save.";
/// What shows once the save holds it.
pub const MAX_OUT_DONE: &str = "#Save maxed out!";

/// The menu's rows (`state.count` of them): the original's, the advanced replay tutorial only with
/// the film pack, then Max Out Save (PC port addition).
#[must_use]
pub fn labels(count: usize) -> Vec<&'static str> {
    LABELS[..count.saturating_sub(1).min(LABELS.len())].iter().copied().chain([MAX_OUT]).collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cheat {
    BigBlack,
    Stereoscopy,
    StreetSweeper,
}
impl Cheat {
    /// ReceiveFEExtrasEnterCode82681650 uses case-sensitive StringCompare.
    /// The other two table rows have null codes (invalid and server promo response).
    pub fn parse(code: &str) -> Option<Self> {
        match code {
            "letsdowork" => Some(Self::BigBlack),
            "strangeloops" => Some(Self::Stereoscopy),
            "streetsweeper" => Some(Self::StreetSweeper),
            _ => None,
        }
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::BigBlack => "ID_VALID_CODE_CHEAT_BIGBLACK",
            Self::Stereoscopy => "ID_VALID_CODE_CHEAT_STEREOSCOPY",
            Self::StreetSweeper => "ID_VALID_CODE_CHEAT_STREETSWEEPER",
        }
    }
}

/// Host keyboard events. They never enter the skating input history.
#[derive(Clone, Debug)]
pub enum TextInput {
    Insert(String),
    Backspace,
    Delete,
    Left,
    Right,
    Home,
    End,
    Accept,
    Cancel,
    /// Ctrl+A (PC): the whole line selected, where a prompt takes a selection.
    SelectAll,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Page {
    Menu,
    Code { text: String, cursor: usize },
    Result(Cheat),
    SaveError(String),
    Movie,
    Credits,
    /// PC port addition: Max Out Save's confirmation, Yes or No highlighted.
    ConfirmMaxOut { yes: bool },
    /// PC port addition: the save holds the maxed-out career.
    MaxedOut,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub page: Page,
    pub selected: usize,
    pub count: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    None,
    Back,
    Movie(u64),
    Credits,
    Cheat(Cheat),
    /// PC port addition: Max Out Save, confirmed.
    MaxOut,
}

pub struct Controller {
    pub state: Snapshot,
    pub intro_complete: bool,
    pending: Action,
    sounds: Vec<&'static str>,
}
impl Controller {
    /// GetNumOptions82643090: the advanced replay tutorial requires REPLAY/filmpackmovie. PC
    /// port: Max Out Save after the original's rows.
    pub fn new(film_pack: bool) -> Self {
        Self {
            state: Snapshot {
                page: Page::Menu,
                selected: 0,
                count: 7 + usize::from(film_pack),
            },
            intro_complete: false,
            pending: Action::None,
            sounds: Vec::new(),
        }
    }
    pub fn snapshot(&self) -> Snapshot {
        self.state.clone()
    }
    pub fn take_sounds(&mut self) -> Vec<&'static str> {
        std::mem::take(&mut self.sounds)
    }
    pub fn text_input(&mut self, input: TextInput) {
        if matches!(self.state.page, Page::Result(_) | Page::SaveError(_)) {
            if matches!(input, TextInput::Accept | TextInput::Cancel) {
                self.sounds.push("FE_SOUND_CORE_A_BUTTON");
                self.state.page = Page::Menu;
            }
            return;
        }
        let Page::Code { text, cursor } = &mut self.state.page else {
            return;
        };
        match input {
            TextInput::Insert(s) => {
                for c in s.chars().filter(char::is_ascii_alphanumeric) {
                    // KeyboardAcceptCallback8265a4d8 truncates its result to 32 bytes.
                    if text.len() == 32 {
                        break;
                    }
                    text.insert(*cursor, c);
                    *cursor += 1;
                }
            }
            TextInput::Backspace if *cursor > 0 => {
                *cursor -= 1;
                text.remove(*cursor);
            }
            TextInput::Delete if *cursor < text.len() => {
                text.remove(*cursor);
            }
            TextInput::Left => *cursor = cursor.saturating_sub(1),
            TextInput::Right => *cursor = (*cursor + 1).min(text.len()),
            TextInput::Home => *cursor = 0,
            TextInput::End => *cursor = text.len(),
            TextInput::Accept => {
                // PC behavior: an unknown code has no action, popup or network request.
                if let Some(cheat) = Cheat::parse(text) {
                    self.pending = Action::Cheat(cheat);
                }
                self.state.page = Page::Menu;
            }
            TextInput::Cancel => self.state.page = Page::Menu,
            _ => {}
        }
    }
    pub fn update(&mut self, input: PauseMenuInput, focused: bool) -> Action {
        if !focused {
            return Action::None;
        }
        let pending = std::mem::replace(&mut self.pending, Action::None);
        if pending != Action::None {
            return pending;
        }
        if !self.intro_complete {
            return Action::None;
        }
        match self.state.page {
            Page::Code { .. } => {
                if input.cancel {
                    self.text_input(TextInput::Cancel);
                } else if input.select {
                    self.text_input(TextInput::Accept);
                }
            }
            Page::Result(_) | Page::SaveError(_) | Page::MaxedOut => {
                if input.select {
                    self.sounds.push("FE_SOUND_CORE_A_BUTTON");
                    self.state.page = Page::Menu;
                }
            }
            // The progression popup's picker: Up and Down move between Yes and No, A answers, and
            // B answers No.
            Page::ConfirmMaxOut { yes } => {
                if input.up || input.down {
                    self.sounds.push(if input.up { "FE_SOUND_CORE_NAV_UP" } else { "FE_SOUND_CORE_NAV_DOWN" });
                    self.state.page = Page::ConfirmMaxOut { yes: !yes };
                } else if input.cancel {
                    self.sounds.push("FE_SOUND_CORE_B_BUTTON");
                    self.state.page = Page::Menu;
                } else if input.select {
                    self.sounds.push("FE_SOUND_CORE_A_BUTTON");
                    self.state.page = Page::Menu;
                    if yes {
                        return Action::MaxOut;
                    }
                }
            }
            Page::Movie | Page::Credits => {}
            Page::Menu => {
                if input.cancel {
                    self.sounds.push("FE_SOUND_CORE_B_BUTTON");
                    return Action::Back;
                }
                if input.up {
                    self.state.selected =
                        (self.state.selected + self.state.count - 1) % self.state.count;
                    self.sounds.push("FE_SOUND_CORE_NAV_UP");
                }
                if input.down {
                    self.state.selected = (self.state.selected + 1) % self.state.count;
                    self.sounds.push("FE_SOUND_CORE_NAV_DOWN");
                }
                if input.select {
                    self.sounds.push("FE_SOUND_CORE_A_BUTTON");
                    match self.state.selected {
                        0 => {
                            self.state.page = Page::Code {
                                text: String::new(),
                                cursor: 0,
                            }
                        }
                        1 => {
                            self.state.page = Page::Credits;
                            return Action::Credits;
                        }
                        i if i + 1 == self.state.count => {
                            self.sounds.push("FE_SOUND_CORE_POPUP");
                            self.state.page = Page::ConfirmMaxOut { yes: false };
                        }
                        i => {
                            self.state.page = Page::Movie;
                            return Action::Movie(MOVIES[i - 2]);
                        }
                    }
                }
            }
        }
        Action::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(set: impl FnOnce(&mut PauseMenuInput)) -> PauseMenuInput {
        let mut input = PauseMenuInput::default();
        set(&mut input);
        input
    }

    /// PC port addition: Max Out Save is the last row, with or without the film pack's row; it asks
    /// first, No highlighted, and only Yes answers it.
    #[test]
    fn max_out_save_asks_first_with_no_highlighted() {
        assert_eq!(labels(7), [LABELS[0], LABELS[1], LABELS[2], LABELS[3], LABELS[4], LABELS[5], MAX_OUT]);
        assert_eq!(labels(8).last(), Some(&MAX_OUT));
        assert_eq!(labels(8)[6], LABELS[6]);
        let mut extras = Controller::new(false);
        extras.intro_complete = true;
        extras.state.selected = extras.state.count - 1;
        assert_eq!(extras.update(press(|input| input.select = true), true), Action::None);
        assert_eq!(extras.state.page, Page::ConfirmMaxOut { yes: false });
        assert_eq!(extras.update(press(|input| input.select = true), true), Action::None, "No leaves the save");
        assert_eq!(extras.state.page, Page::Menu);
        extras.update(press(|input| input.select = true), true);
        extras.update(press(|input| input.cancel = true), true);
        assert_eq!(extras.state.page, Page::Menu, "B answers No");
        extras.update(press(|input| input.select = true), true);
        extras.update(press(|input| input.up = true), true);
        assert_eq!(extras.state.page, Page::ConfirmMaxOut { yes: true });
        assert_eq!(extras.update(press(|input| input.select = true), true), Action::MaxOut);
        assert_eq!(extras.state.page, Page::Menu);
        extras.state.selected = extras.state.count - 2;
        assert!(matches!(extras.update(press(|input| input.select = true), true), Action::Movie(_)), "the row before is a video");
    }
}
