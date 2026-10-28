//! PC first-use name prompt. The mandatory startup timing is a PC addition;
//! DoRequestProfileName82759248 supplies the native title and ten-character limit.
use super::extras::TextInput;
use skate2_net::protocol::{nickname_character, valid_nickname};

/// What the nickname prompt and the Online options say of a nickname the server does not take.
pub const NICKNAME_NOT_PERMITTED: &str = "Nickname not permitted.";
/// What the nickname prompt says of a saved nickname longer than the prompt types.
pub const NICKNAME_TOO_LONG: &str = "Nicknames hold at most 16 characters.";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Prompt {
    pub text: String,
    pub cursor: usize,
    pub error: Option<String>,
    accepted: bool,
    limit: usize,
    cancellable: bool,
    cancelled: bool,
    /// The online nickname's rules (`valid_nickname`) instead of the profile name's printable
    /// characters.
    nickname: bool,
}
impl Default for Prompt {
    fn default() -> Self {
        Self { text: String::new(), cursor: 0, error: None, accepted: false, limit: 10, cancellable: false, cancelled: false, nickname: false }
    }
}
impl Prompt {
    /// PC online nickname for `online-config.json`; Escape/B closes it unsaved.
    pub fn nickname() -> Self {
        Self { limit: super::online_config::NICKNAME_LIMIT, cancellable: true, nickname: true, ..Self::default() }
    }
    /// The nickname prompt holding `nickname` to edit, as much of it as the prompt can type.
    pub fn nickname_with(nickname: &str) -> Self {
        let mut prompt = Self::nickname();
        prompt.input(TextInput::Insert(nickname.trim().to_owned()));
        prompt
    }
    /// The nickname prompt for a saved nickname the game cannot sign in with: what the prompt can
    /// type of it under [`NICKNAME_TOO_LONG`] when it is longer than a nickname, else under
    /// [`NICKNAME_NOT_PERMITTED`]; with none saved, the first nickname's prompt.
    pub fn nickname_replacing(saved: &str) -> Self {
        let mut prompt = Self::nickname_with(saved);
        let saved = saved.trim();
        if saved.chars().count() > super::online_config::NICKNAME_LIMIT {
            prompt.error = Some(NICKNAME_TOO_LONG.into());
        } else if !saved.is_empty() {
            prompt.error = Some(NICKNAME_NOT_PERMITTED.into());
        }
        prompt
    }
    pub fn cancellable(&self) -> bool {
        self.cancellable
    }
    pub fn input(&mut self, input: TextInput) {
        match input {
            // Same printable host keyboard alphabet as the Extras textbox; a nickname takes
            // only the characters the server permits.
            TextInput::Insert(value) => {
                let nickname = self.nickname;
                for c in value.chars().filter(|&c| if nickname { nickname_character(c) } else { c.is_ascii_graphic() || c == ' ' }) {
                    if self.text.len() == self.limit { break; }
                    self.text.insert(self.cursor, c);
                    self.cursor += 1;
                }
                self.error = None;
            }
            TextInput::Backspace if self.cursor > 0 => {
                self.cursor -= 1; self.text.remove(self.cursor);
            }
            TextInput::Delete if self.cursor < self.text.len() => { self.text.remove(self.cursor); }
            TextInput::Left => self.cursor = self.cursor.saturating_sub(1),
            TextInput::Right => self.cursor = (self.cursor + 1).min(self.text.len()),
            TextInput::Home => self.cursor = 0,
            TextInput::End => self.cursor = self.text.len(),
            TextInput::Accept => {
                if self.text.trim().is_empty() { self.error = Some("Enter a name to continue.".into()); }
                else if self.nickname && !valid_nickname(self.text.trim()) { self.error = Some(NICKNAME_NOT_PERMITTED.into()); }
                else { self.accepted = true; }
            }
            TextInput::Cancel if self.cancellable => self.cancelled = true,
            // PC addition: a mandatory prompt, which Escape/B cannot dismiss.
            _ => (),
        }
    }
    pub fn take_cancelled(&mut self) -> bool {
        std::mem::take(&mut self.cancelled)
    }
    pub fn take_accepted(&mut self) -> Option<String> {
        std::mem::take(&mut self.accepted).then(|| self.text.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::{NICKNAME_NOT_PERMITTED, NICKNAME_TOO_LONG, Prompt, TextInput};
    use skate2_net::protocol::{NICKNAME_CHARACTERS, valid_nickname};

    #[test]
    fn the_nickname_prompt_makes_only_what_the_server_takes() {
        let mut prompt = Prompt::nickname();
        prompt.input(TextInput::Insert("a".repeat(40)));
        assert_eq!(prompt.text.len(), NICKNAME_CHARACTERS);
        assert!(valid_nickname(&prompt.text));
        let edit = Prompt::nickname_with("  Caf\u{e9} Racer \t");
        assert_eq!(edit.text, "Caf Racer");
        assert_eq!(edit.cursor, edit.text.len());
        assert!(valid_nickname(&edit.text));
        let marks = Prompt::nickname_with("Rob!.Mc'Gill-99_ (x)");
        assert_eq!(marks.text, "RobMc'Gill-99_ x");
        assert!(valid_nickname(&marks.text));
    }

    #[test]
    fn a_nickname_of_marks_alone_is_not_permitted() {
        let mut prompt = Prompt::nickname_with("- _ '");
        prompt.input(TextInput::Accept);
        assert_eq!(prompt.take_accepted(), None);
        assert_eq!(prompt.error.as_deref(), Some(NICKNAME_NOT_PERMITTED));
        assert_eq!(Prompt::nickname_replacing("Rob!").error.as_deref(), Some(NICKNAME_NOT_PERMITTED));
        assert_eq!(Prompt::nickname_replacing("").error, None);
        let long = Prompt::nickname_replacing("TwentyFourCharacterName1");
        assert_eq!((long.text.as_str(), long.error.as_deref()), ("TwentyFourCharac", Some(NICKNAME_TOO_LONG)), "a saved name past 16 asks again");
        assert!(valid_nickname(&long.text));
        let mut profile = Prompt::default();
        profile.input(TextInput::Insert("A.B!".into()));
        assert_eq!(profile.text, "A.B!", "the profile name keeps every printable character");
    }
}
