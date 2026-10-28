//! Lobby chat (PC port addition): T opens a prompt, Enter sends its line to the server, which
//! relays it to the game's members, and every member's event log shows it as
//! `[name]: line`, wrapped into lines of at most 50 characters.

use crate::ui::extras::TextInput;
use skate2_net::protocol::{CHAT_CHARACTERS, valid_chat};

/// The widest line the event log shows a chat line in.
pub const LINE_CHARACTERS: usize = 50;

/// The open chat prompt: the line typed so far, the cursor's place in it, and whether Ctrl+A
/// selected the whole line (a key typed then replaces it, Backspace or Delete clears it, Esc only
/// lets it go, and an arrow, Home or End puts the cursor at that end of it).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChatPrompt {
    pub text: String,
    pub cursor: usize,
    pub selected: bool,
}

/// What a key did to the prompt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChatOutcome {
    Open,
    Closed,
    /// Enter on a line worth sending: the line, trimmed.
    Send(String),
}

impl ChatPrompt {
    /// A key: printable ASCII and spaces up to the line's limit, editing, Enter and Esc, and
    /// Ctrl+A's selection of the whole line.
    pub fn input(&mut self, input: TextInput) -> ChatOutcome {
        if self.selected {
            self.selected = false;
            match input {
                TextInput::Insert(ref typed) if typed.chars().any(|character| character.is_ascii_graphic() || character == ' ') => {
                    self.text.clear();
                    self.cursor = 0;
                }
                TextInput::Insert(_) => {
                    self.selected = true;
                    return ChatOutcome::Open;
                }
                TextInput::Backspace | TextInput::Delete => {
                    self.text.clear();
                    self.cursor = 0;
                    return ChatOutcome::Open;
                }
                TextInput::Cancel => return ChatOutcome::Open,
                TextInput::Left | TextInput::Home => {
                    self.cursor = 0;
                    return ChatOutcome::Open;
                }
                TextInput::Right | TextInput::End => {
                    self.cursor = self.text.len();
                    return ChatOutcome::Open;
                }
                TextInput::SelectAll => {
                    self.selected = true;
                    return ChatOutcome::Open;
                }
                TextInput::Accept => {}
            }
        }
        match input {
            TextInput::SelectAll => self.selected = !self.text.is_empty(),
            TextInput::Insert(typed) => {
                for character in typed.chars().filter(|character| character.is_ascii_graphic() || *character == ' ') {
                    if self.text.len() == CHAT_CHARACTERS {
                        break;
                    }
                    self.text.insert(self.cursor, character);
                    self.cursor += 1;
                }
            }
            TextInput::Backspace if self.cursor > 0 => {
                self.cursor -= 1;
                self.text.remove(self.cursor);
            }
            TextInput::Delete if self.cursor < self.text.len() => {
                self.text.remove(self.cursor);
            }
            TextInput::Left => self.cursor = self.cursor.saturating_sub(1),
            TextInput::Right => self.cursor = (self.cursor + 1).min(self.text.len()),
            TextInput::Home => self.cursor = 0,
            TextInput::End => self.cursor = self.text.len(),
            TextInput::Accept => {
                let line = self.text.trim();
                return if valid_chat(line) { ChatOutcome::Send(line.to_owned()) } else { ChatOutcome::Closed };
            }
            TextInput::Cancel => return ChatOutcome::Closed,
            TextInput::Backspace | TextInput::Delete => {}
        }
        ChatOutcome::Open
    }
}

/// The event log's lines for a member's chat line: `[name]: line` (both ASCII, as the
/// nickname and chat checks make them), broken at spaces into lines of at most
/// [`LINE_CHARACTERS`] characters; a word wider than a line fills the line it starts on and
/// goes on in the next.
#[must_use]
pub fn lines(name: &str, text: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for mut word in format!("[{name}]: {text}").split(' ').filter(|word| !word.is_empty()) {
        let gap = usize::from(!current.is_empty());
        let append = |current: &mut String, part: &str| {
            if gap == 1 {
                current.push(' ');
            }
            current.push_str(part);
        };
        if current.len() + gap + word.len() <= LINE_CHARACTERS {
            append(&mut current, word);
            continue;
        }
        if word.len() > LINE_CHARACTERS {
            let room = LINE_CHARACTERS.saturating_sub(current.len() + gap);
            if room > 0 {
                append(&mut current, &word[..room]);
                word = &word[room..];
            }
        }
        if !current.is_empty() {
            lines.push(std::mem::take(&mut current));
        }
        while word.len() > LINE_CHARACTERS {
            let (head, rest) = word.split_at(LINE_CHARACTERS);
            lines.push(head.to_owned());
            word = rest;
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typed(text: &str) -> ChatPrompt {
        let mut prompt = ChatPrompt::default();
        prompt.input(TextInput::Insert(text.to_owned()));
        prompt
    }

    #[test]
    fn the_prompt_takes_printable_ascii_up_to_the_limit_and_sends_it_trimmed() {
        let mut prompt = typed("  hey\tthere\u{e9}!  ");
        assert_eq!(prompt.text, "  heythere!  ");
        assert_eq!(prompt.input(TextInput::Accept), ChatOutcome::Send("heythere!".into()));
        let mut prompt = typed(&"a".repeat(CHAT_CHARACTERS + 20));
        assert_eq!(prompt.text.len(), CHAT_CHARACTERS);
        prompt.input(TextInput::Home);
        prompt.input(TextInput::Delete);
        prompt.input(TextInput::Insert("bc".into()));
        assert!(prompt.text.starts_with('b') && prompt.text.len() == CHAT_CHARACTERS);
        assert_eq!(typed("   ").input(TextInput::Accept), ChatOutcome::Closed);
        assert_eq!(typed("hi").input(TextInput::Cancel), ChatOutcome::Closed);
    }

    #[test]
    fn ctrl_a_selects_the_line_for_the_next_key_to_replace_or_clear() {
        let mut prompt = typed("hello there");
        prompt.input(TextInput::SelectAll);
        assert!(prompt.selected);
        assert_eq!(prompt.input(TextInput::Insert("x".into())), ChatOutcome::Open);
        assert_eq!((prompt.text.as_str(), prompt.cursor, prompt.selected), ("x", 1, false), "a key replaces the line");
        prompt.input(TextInput::SelectAll);
        prompt.input(TextInput::Backspace);
        assert_eq!((prompt.text.as_str(), prompt.cursor, prompt.selected), ("", 0, false), "Backspace clears it");
        let mut prompt = typed("keep me");
        prompt.input(TextInput::SelectAll);
        assert_eq!(prompt.input(TextInput::Cancel), ChatOutcome::Open, "Esc lets the selection go and keeps the prompt");
        assert_eq!((prompt.text.as_str(), prompt.selected), ("keep me", false));
        assert_eq!(prompt.input(TextInput::Cancel), ChatOutcome::Closed);
        let mut prompt = typed("abc");
        prompt.input(TextInput::SelectAll);
        prompt.input(TextInput::Left);
        assert_eq!((prompt.cursor, prompt.selected), (0, false));
        prompt.input(TextInput::SelectAll);
        prompt.input(TextInput::Delete);
        assert!(prompt.text.is_empty());
        let mut prompt = typed("send me");
        prompt.input(TextInput::SelectAll);
        assert_eq!(prompt.input(TextInput::Accept), ChatOutcome::Send("send me".into()), "Enter sends the selected line");
        let mut empty = ChatPrompt::default();
        empty.input(TextInput::SelectAll);
        assert!(!empty.selected, "nothing to select");
        let mut prompt = typed("x");
        prompt.input(TextInput::SelectAll);
        prompt.input(TextInput::Insert("\u{e9}".into()));
        assert_eq!((prompt.text.as_str(), prompt.selected), ("x", true), "a key the line cannot take replaces nothing");
    }

    #[test]
    fn a_chat_line_wraps_at_fifty_characters_behind_its_name() {
        assert_eq!(lines("Rob", "hi all"), ["[Rob]: hi all"]);
        let wrapped = lines("Rob", &"word ".repeat(40).trim_end().to_owned());
        assert!(wrapped.iter().all(|line| line.len() <= LINE_CHARACTERS));
        assert!(wrapped[0].starts_with("[Rob]: word word"));
        assert_eq!(wrapped.join(" "), format!("[Rob]: {}", "word ".repeat(40).trim_end()));
        let long = lines("Rob", &"x".repeat(120));
        assert_eq!(long, [format!("[Rob]: {}", "x".repeat(43)), "x".repeat(50), "x".repeat(27)]);
    }
}
