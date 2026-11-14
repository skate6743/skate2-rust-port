//! PC port addition: the game's update over the main menu. The app looks for a newer release at
//! startup (`skate2-update`); once the main menu is idle a newer one is offered, Yes downloads it
//! under a progress popup and the game then closes and starts again on it, No asks again only at
//! the next start or after the server refuses this version (`OnlineNotice::UpdateRequired`).
//! The first start after an update lists the release's changes ([`Changelog`]) before anything
//! else is offered.

/// What the app's updater reports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateStatus {
    /// No newer release is known.
    None,
    Available { version: String },
    Downloading { version: String, percent: u8 },
    /// The release is in place; the game closes and starts again.
    Installed { version: String },
    /// The update failed and the game is as it was: why, for the player.
    Failed { message: String },
}

/// What the main menu asks of the app's updater.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateRequest {
    /// Look for a newer release again.
    Check,
    /// Download and install the release offered.
    Install,
}

/// The update's popup over the main menu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdatePrompt {
    /// A newer release: Yes (0) or No (1).
    Offer { version: String, current: String, selected: usize },
    Downloading { version: String, percent: u8 },
    Installed { version: String },
    Failed { message: String },
    /// The release's changes at the first start after an update: OK.
    Changelog(Changelog),
}

impl UpdatePrompt {
    /// The popup as sk8popup shows it: title, text, options, the highlighted option and whether
    /// B closes it.
    #[must_use]
    pub fn dialog(&self) -> (String, String, Vec<String>, usize, bool) {
        match self {
            Self::Offer { version, current, selected } => (
                "#Update Available".into(),
                format!("#Version {version} is out (you have {current}). Online play needs the newest version. Update now?"),
                vec!["ID_COMMON_YES".into(), "ID_COMMON_NO".into()],
                *selected,
                true,
            ),
            Self::Downloading { version, percent } => ("#Updating".into(), format!("#Downloading version {version}: {percent}%"), Vec::new(), 0, false),
            Self::Installed { version } => ("#Updating".into(), format!("#Version {version} is installed. The game starts again now."), Vec::new(), 0, false),
            Self::Failed { message } => (
                "ID_COMMON_WARNING_TITLE".into(),
                format!("#The update could not be installed, and the game is unchanged. {message}"),
                vec!["ID_COMMON_OK".into()],
                0,
                false,
            ),
            Self::Changelog(changelog) => {
                (format!("#Version {}", changelog.version), changelog.text(), vec!["ID_COMMON_OK".into()], 0, false)
            }
        }
    }
}

/// The release's changes, shown once over the main menu at a player's first launch of the release
/// (the app's `config.json` records it).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Changelog {
    pub version: String,
    pub entries: Vec<String>,
    /// The first body line the popup shows: a changelog longer than the popup scrolls.
    pub scroll: usize,
}

impl Changelog {
    /// A changelog file's entries, a change a line. Blank lines and lines starting with `#` are
    /// left out, a dash or star with a space after it that starts a line is dropped (the popup
    /// adds its own dash), and typographic quotes, dashes and ellipses become plain ones.
    #[must_use]
    pub fn parse(version: &str, text: &str) -> Self {
        let entries = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.starts_with('#'))
            .map(|line| match line.strip_prefix(['-', '*']) {
                Some(rest) if rest.is_empty() || rest.starts_with(char::is_whitespace) => rest.trim_start(),
                _ => line,
            })
            .filter(|line| !line.is_empty())
            .map(plain)
            .collect();
        Self { version: version.to_owned(), entries, scroll: 0 }
    }

    /// "Update Changelogs:" and each entry under a dash, a blank line before each. The popup
    /// shows the lines that fit and scrolls through the rest.
    fn text(&self) -> String {
        let mut text = String::from("#Update Changelogs:");
        for entry in &self.entries {
            text.push_str("\n\n- ");
            text.push_str(entry);
        }
        text
    }
}

/// PC: how a long changelog scrolls. Up and down move it a line each; the right stick moves it a
/// line as soon as it leaves its dead zone, then keeps it going, faster the further it is pushed,
/// up to [`ChangelogScroll::LINES_PER_SECOND`].
#[derive(Clone, Debug, Default)]
pub struct ChangelogScroll {
    /// The last line the body can start at, from the popup's layout; `None` until it is laid out.
    pub limit: Option<usize>,
    direction: i8,
    owed: f32,
}

impl ChangelogScroll {
    const DEAD_ZONE: f32 = 0.3;
    pub const LINES_PER_SECOND: f32 = 18.0;

    /// Moves `changelog` by a frame's input: `stick` is the right stick's up (-1 to 1) and
    /// `step` an up (-1) or down (1) press.
    pub fn update(&mut self, changelog: &mut Changelog, stick: f32, step: i8, dt: f32) {
        let travel = ((stick.abs() - Self::DEAD_ZONE) / (1.0 - Self::DEAD_ZONE)).clamp(0.0, 1.0);
        let direction: i8 = if travel <= 0.0 {
            0
        } else if stick > 0.0 {
            -1
        } else {
            1
        };
        let mut lines = isize::from(step);
        if direction == 0 {
            self.owed = 0.0;
        } else if direction == self.direction {
            self.owed = travel.mul_add(Self::LINES_PER_SECOND * dt, self.owed);
            let whole = self.owed.floor();
            self.owed -= whole;
            lines += isize::from(direction) * whole as isize;
        } else {
            self.owed = 0.0;
            lines += isize::from(direction);
        }
        self.direction = direction;
        changelog.scroll = changelog.scroll.saturating_add_signed(lines).min(self.limit.unwrap_or(0));
    }

    /// The layout's last starting line; the changelog's scroll is kept within it.
    pub fn set_limit(&mut self, changelog: Option<&mut Changelog>, limit: Option<usize>) {
        self.limit = limit;
        if let Some(changelog) = changelog {
            changelog.scroll = changelog.scroll.min(limit.unwrap_or(0));
        }
    }
}

/// `text` with typographic quotes, dashes and ellipses in their plain forms, which the popup's
/// font has.
fn plain(text: &str) -> String {
    let mut plain = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '\u{2018}' | '\u{2019}' => plain.push('\''),
            '\u{201C}' | '\u{201D}' => plain.push('"'),
            '\u{2013}' | '\u{2014}' => plain.push('-'),
            '\u{2026}' => plain.push_str("..."),
            character => plain.push(character),
        }
    }
    plain
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_changelog_file_lists_each_change_under_a_dash_a_blank_line_apart() {
        let file = "# Shown after an update\r\n\r\nLeave Challenge in the pause menu\r\n- Lobby ghosts\r\n* \u{201C}Alt-tab\u{201D} kicks \u{2014} fixed\u{2026}\r\n-flag stays\r\n - \r\n";
        let changelog = Changelog::parse("0.2.5", file);
        assert_eq!(changelog.entries, ["Leave Challenge in the pause menu", "Lobby ghosts", "\"Alt-tab\" kicks - fixed...", "-flag stays"]);
        let (title, text, options, selected, cancellable) = UpdatePrompt::Changelog(changelog).dialog();
        assert_eq!(title, "#Version 0.2.5");
        assert_eq!(
            text,
            "#Update Changelogs:\n\n- Leave Challenge in the pause menu\n\n- Lobby ghosts\n\n- \"Alt-tab\" kicks - fixed...\n\n- -flag stays"
        );
        assert_eq!((options, selected, cancellable), (vec!["ID_COMMON_OK".to_owned()], 0, false));
        assert!(Changelog::parse("0.3.0", "# nothing yet\n\n").entries.is_empty());
    }

    #[test]
    fn a_long_changelog_lists_every_entry_and_scrolls_within_its_layout() {
        let file: String = (1..=40).map(|n| format!("Change {n}\n")).collect();
        let mut changelog = Changelog::parse("0.3.0", &file);
        let (_, text, ..) = UpdatePrompt::Changelog(changelog.clone()).dialog();
        assert_eq!(text.matches("\n\n- Change ").count(), 40, "every entry, none left out");
        assert!(text.ends_with("\n\n- Change 40"));
        let frame = 1.0 / 60.0;
        let mut scroll = ChangelogScroll::default();
        scroll.update(&mut changelog, 0.0, 1, frame);
        assert_eq!(changelog.scroll, 0, "nothing moves before the popup is laid out");
        scroll.set_limit(Some(&mut changelog), Some(50));
        scroll.update(&mut changelog, -1.0, 0, frame);
        assert_eq!(changelog.scroll, 1, "the stick moves a line at once");
        for _ in 0..60 {
            scroll.update(&mut changelog, -1.0, 0, frame);
        }
        assert!((18..=19).contains(&changelog.scroll), "then about 18 lines a second: {}", changelog.scroll);
        let held = changelog.scroll;
        scroll.update(&mut changelog, 0.0, -1, frame);
        assert_eq!(changelog.scroll, held - 1, "up moves a line back");
        scroll.update(&mut changelog, 0.2, 0, frame);
        assert_eq!(changelog.scroll, held - 1, "inside the dead zone nothing moves");
        scroll.update(&mut changelog, 1.0, 0, frame);
        assert_eq!(changelog.scroll, held - 2, "the stick up moves back a line at once");
        for _ in 0..600 {
            scroll.update(&mut changelog, -1.0, 0, frame);
        }
        assert_eq!(changelog.scroll, 50, "never past the layout's last line");
        scroll.set_limit(Some(&mut changelog), Some(10));
        assert_eq!(changelog.scroll, 10, "a shorter layout pulls it back");
    }
}
