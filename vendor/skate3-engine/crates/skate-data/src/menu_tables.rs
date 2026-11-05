//! The front-end menu tables from an unpacked retail executable, written by
//! setup as `assets/private/menu-tables.json` (`skate3rust --extract-menu-tables`).
//!
//! Every table is located by its content or by the code that reads it, never by
//! a fixed address, so any build of the executable works (TU3 addresses in
//! docs/hails-additions are for verification only):
//! - settings rows: (string id, widget type) pairs found through the pointer to
//!   `ID_GAMESETTINGS_SFXVOLUME` next to a `slider` pointer; the settings screens
//!   (16 row indices each, 0-terminated) follow the rows;
//! - crossbar items: 20-byte records (name, label, icon, sub-option kind, help
//!   text) found through the pointer to `ReplayEditor`; the categories (12-byte
//!   records name, label, icon) end right before them;
//! - Apt key table: 20-byte records (input action, name, Apt key code, released
//!   flag, kind) found through the pointer to `AptStart`;
//! - per-mode menus: each crossbar mode class has a vtable whose slots hold a
//!   tab accessor (`tabs[r4]`), a row accessor (`rows[r4 * slots + r5]`), the tab
//!   count (`li r3,n`) and the mode title (a string id). The accessors are tiny
//!   leaf functions; a small evaluator runs them to read the table addresses and
//!   the row stride (8 slots normally, 9 in the skate-park variants).
//!
//! No game data lives in this module: the JSON is produced from the user's disc.
use crate::xex::XexImage;
use serde::{Deserialize, Serialize};

pub const SCHEMA: u32 = 1;
const WIDGETS: [&str; 3] = ["option", "slider", "selector"];
const SCREEN_ROWS: usize = 16;
const ITEM_RECORD: u32 = 20;
const CATEGORY_RECORD: u32 = 12;
const KEY_RECORD: u32 = 20;
/// Instructions the accessor evaluator runs before giving up.
const LEAF_LIMIT: usize = 12;
/// Vtable slots after the tab accessor searched for the tab count and title.
const VTABLE_SPAN: usize = 9;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SettingsRow {
    /// Language string id (`ID_GAMESETTINGS_*`); retail leaves a few rows blank.
    pub id: String,
    /// `option` (a link or action), `slider` or `selector`.
    pub widget: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Category {
    pub name: String,
    pub label: String,
    pub icon: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Item {
    pub name: String,
    pub label: String,
    pub icon: String,
    /// Sub-option list kind (a second column), -1 = none.
    pub sub_option_kind: i32,
    pub help: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AptKey {
    pub name: String,
    /// The game input action polled for this key.
    pub action: u32,
    /// The key code the movies see (AptUp, AptNext, ...).
    pub apt_key: u32,
    pub released: bool,
    pub kind: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModeTab {
    /// Category name (`SinglePlayer`, `Multiplayer`, ...).
    pub category: String,
    /// Item names in display order.
    pub items: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModeMenu {
    /// Stable key (see `mode_keys`): Career, CareerPark, FreePlay, FreePlayPark,
    /// FreeSkateRestricted, or the title without `ID_CROSSBAR_` / `_MODE_TITLE`.
    pub key: String,
    /// The mode's title string id.
    pub title: String,
    /// Row slots per tab (8, or 9 in the skate-park variants).
    pub slots: u32,
    pub tabs: Vec<ModeTab>,
    /// Where the vtable was found (diagnostics only).
    pub vtable: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MenuTables {
    pub schema: u32,
    pub source_sha256: String,
    pub settings_rows: Vec<SettingsRow>,
    /// Row indices per settings screen.
    pub settings_screens: Vec<Vec<u32>>,
    pub categories: Vec<Category>,
    pub items: Vec<Item>,
    pub keys: Vec<AptKey>,
    pub modes: Vec<ModeMenu>,
    /// Table addresses in this build (diagnostics only).
    pub addresses: std::collections::BTreeMap<String, String>,
}

impl MenuTables {
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("menu tables serialize")
    }
    pub fn from_json(text: &str) -> Result<Self, String> {
        let tables: Self = serde_json::from_str(text).map_err(|e| format!("menu tables: {e}"))?;
        if tables.schema != SCHEMA {
            return Err(format!(
                "menu tables: schema {} (expected {SCHEMA})",
                tables.schema
            ));
        }
        Ok(tables)
    }
    pub fn mode(&self, key: &str) -> Option<&ModeMenu> {
        self.modes.iter().find(|m| m.key == key)
    }
}

struct Image<'a> {
    image: &'a XexImage,
}

impl Image<'_> {
    fn word(&self, address: u32) -> Option<u32> {
        self.image
            .at(address, 4)
            .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn end(&self) -> u32 {
        self.image
            .base_address
            .wrapping_add(self.image.image.len() as u32)
    }
    fn inside(&self, address: u32) -> bool {
        address >= self.image.base_address && address < self.end()
    }
    /// A NUL-terminated printable ASCII string (possibly empty).
    fn cstr(&self, address: u32) -> Option<String> {
        if !self.inside(address) {
            return None;
        }
        let start = (address - self.image.base_address) as usize;
        let bytes = &self.image.image[start..(start + 256).min(self.image.image.len())];
        let end = bytes.iter().position(|b| *b == 0)?;
        let text = &bytes[..end];
        text.iter()
            .all(|b| (0x20..0x7f).contains(b))
            .then(|| String::from_utf8_lossy(text).into_owned())
    }
    fn string_at(&self, pointer: u32) -> Option<String> {
        self.word(pointer).and_then(|p| self.cstr(p))
    }
    /// Addresses of `text` as a whole C string.
    fn strings(&self, text: &str) -> Vec<u32> {
        let needle: Vec<u8> = text.bytes().chain([0]).collect();
        let data = &self.image.image;
        let mut found = Vec::new();
        let mut at = 0;
        while let Some(i) = find(&data[at..], &needle) {
            let offset = at + i;
            if offset == 0 || data[offset - 1] == 0 {
                found.push(self.image.base_address + offset as u32);
            }
            at = offset + 1;
        }
        found
    }
    /// Aligned words equal to `value`.
    fn pointers_to(&self, value: u32) -> Vec<u32> {
        let needle = value.to_be_bytes();
        self.image
            .image
            .chunks_exact(4)
            .enumerate()
            .filter(|(_, w)| *w == needle)
            .map(|(i, _)| self.image.base_address + (i * 4) as u32)
            .collect()
    }
    fn pointers_to_string(&self, text: &str) -> Vec<u32> {
        self.strings(text)
            .into_iter()
            .flat_map(|s| self.pointers_to(s))
            .collect()
    }
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn hex(address: u32) -> String {
    format!("0x{address:08X}")
}

// ---------------------------------------------------------------- settings

fn settings_row(img: &Image, at: u32) -> Option<SettingsRow> {
    let widget = img.string_at(at + 4)?;
    if !WIDGETS.contains(&widget.as_str()) {
        return None;
    }
    let pointer = img.word(at)?;
    let id = if pointer == 0 {
        String::new()
    } else {
        img.cstr(pointer)?
    };
    Some(SettingsRow { id, widget })
}

fn settings(img: &Image) -> Result<(u32, Vec<SettingsRow>, Vec<Vec<u32>>), String> {
    let anchors: Vec<u32> = img
        .pointers_to_string("ID_GAMESETTINGS_SFXVOLUME")
        .into_iter()
        .filter(|p| img.string_at(p + 4).as_deref() == Some("slider"))
        .collect();
    let [anchor] = anchors.as_slice() else {
        return Err(format!("settings rows: {} anchors", anchors.len()));
    };
    let mut start = *anchor;
    while settings_row(img, start - 8).is_some() {
        start -= 8;
    }
    let mut rows = Vec::new();
    let mut at = start;
    while let Some(row) = settings_row(img, at) {
        rows.push(row);
        at += 8;
    }
    let mut screens = Vec::new();
    loop {
        let values: Option<Vec<u32>> = (0..SCREEN_ROWS as u32)
            .map(|k| img.word(at + k * 4))
            .collect();
        let Some(values) = values else { break };
        if values.iter().any(|v| *v as usize >= rows.len()) {
            break;
        }
        let list: Vec<u32> = values.iter().copied().take_while(|v| *v != 0).collect();
        if list.is_empty() {
            break;
        }
        screens.push(list);
        at += (SCREEN_ROWS * 4) as u32;
    }
    if rows.len() < 8 || screens.is_empty() {
        return Err(format!(
            "settings tables incomplete: {} rows, {} screens",
            rows.len(),
            screens.len()
        ));
    }
    Ok((start, rows, screens))
}

// ---------------------------------------------------------------- crossbar

fn item(img: &Image, at: u32) -> Option<Item> {
    let kind = img.word(at + 12)? as i32;
    if !(-1..=64).contains(&kind) {
        return None;
    }
    let item = Item {
        name: img.string_at(at)?,
        label: img.string_at(at + 4)?,
        icon: img.string_at(at + 8)?,
        sub_option_kind: kind,
        help: img.string_at(at + 16)?,
    };
    (!item.name.is_empty() && !item.label.is_empty()).then_some(item)
}

fn category(img: &Image, at: u32) -> Option<Category> {
    let category = Category {
        name: img.string_at(at)?,
        label: img.string_at(at + 4)?,
        icon: img.string_at(at + 8)?,
    };
    // Name and icon are identifiers (`SinglePlayer`, `singleplayer`); string ids follow
    // the disc build's categories.
    let identifier = |s: &str| {
        !s.is_empty() && !s.starts_with("ID_") && s.bytes().all(|b| b.is_ascii_alphanumeric())
    };
    (identifier(&category.name)
        && identifier(&category.icon)
        && category.label.starts_with("ID_CROSSBAR_"))
    .then_some(category)
}

fn crossbar(img: &Image) -> Result<(u32, Vec<Item>, u32, Vec<Category>), String> {
    let anchors: Vec<u32> = img
        .pointers_to_string("ReplayEditor")
        .into_iter()
        .filter(|p| item(img, *p).is_some())
        .collect();
    let [anchor] = anchors.as_slice() else {
        return Err(format!("crossbar items: {} anchors", anchors.len()));
    };
    let mut start = *anchor;
    while item(img, start - ITEM_RECORD).is_some() {
        start -= ITEM_RECORD;
    }
    let mut items = Vec::new();
    let mut at = start;
    while let Some(item) = item(img, at) {
        items.push(item);
        at += ITEM_RECORD;
    }
    // The categories sit next to the items: right before them (TU3) or right
    // after them (the disc build).
    let mut first = start;
    while category(img, first - CATEGORY_RECORD).is_some() {
        first -= CATEGORY_RECORD;
    }
    let (first, last) = if first < start {
        (first, start)
    } else {
        let mut end = at;
        while category(img, end).is_some() {
            end += CATEGORY_RECORD;
        }
        (at, end)
    };
    let categories: Vec<Category> = (first..last)
        .step_by(CATEGORY_RECORD as usize)
        .filter_map(|a| category(img, a))
        .collect();
    if items.len() < 8 || categories.is_empty() {
        return Err(format!(
            "crossbar tables incomplete: {} items, {} categories",
            items.len(),
            categories.len()
        ));
    }
    Ok((start, items, first, categories))
}

// ---------------------------------------------------------------- Apt keys

fn key(img: &Image, at: u32) -> Option<AptKey> {
    let name = img.string_at(at + 4)?;
    let released = img.word(at + 12)?;
    let kind = img.word(at + 16)?;
    (name.starts_with("Apt") && released <= 1 && kind <= 16).then(|| AptKey {
        name,
        action: img.word(at).unwrap_or(0),
        apt_key: img.word(at + 8).unwrap_or(0),
        released: released == 1,
        kind,
    })
}

fn keys(img: &Image) -> Result<(u32, Vec<AptKey>), String> {
    let anchors: Vec<u32> = img
        .pointers_to_string("AptStart")
        .into_iter()
        .map(|p| p - 4)
        .filter(|r| key(img, *r).is_some())
        .collect();
    let [anchor] = anchors.as_slice() else {
        return Err(format!("Apt key table: {} anchors", anchors.len()));
    };
    let mut start = *anchor;
    while key(img, start - KEY_RECORD).is_some() {
        start -= KEY_RECORD;
    }
    let mut keys = Vec::new();
    let mut at = start;
    while let Some(key) = key(img, at) {
        keys.push(key);
        at += KEY_RECORD;
    }
    Ok((start, keys))
}

// ---------------------------------------------------------------- modes

#[derive(Debug, PartialEq)]
enum Leaf {
    /// The function returns the word loaded from this address.
    Load(u32),
    /// The function returns this value in r3.
    Value(u32),
}

/// Runs a small leaf function with r4 / r5 set; only the handful of integer
/// instructions table accessors use.
fn leaf(img: &Image, function: u32, r4: u32, r5: u32) -> Option<Leaf> {
    let mut regs: [Option<u32>; 32] = [None; 32];
    regs[4] = Some(r4);
    regs[5] = Some(r5);
    let mut loaded: Option<(usize, u32)> = None;
    for k in 0..LEAF_LIMIT {
        let w = img.word(function.checked_add(k as u32 * 4)?)?;
        if w == 0x4e80_0020 {
            return match loaded {
                Some((3, address)) => Some(Leaf::Load(address)),
                _ => regs[3].map(Leaf::Value),
            };
        }
        let (op, rd, ra, rb) = (
            w >> 26,
            ((w >> 21) & 31) as usize,
            ((w >> 16) & 31) as usize,
            ((w >> 11) & 31) as usize,
        );
        let simm = (w & 0xffff) as u16 as i16 as i32 as u32;
        let base = |regs: &[Option<u32>; 32]| if ra == 0 { Some(0) } else { regs[ra] };
        match op {
            14 => regs[rd] = Some(base(&regs)?.wrapping_add(simm)),
            15 => regs[rd] = Some(base(&regs)?.wrapping_add(simm << 16)),
            7 => regs[rd] = Some(regs[ra]?.wrapping_mul(simm)),
            24 => regs[ra] = Some(regs[rd]? | (w & 0xffff)),
            21 => {
                let (sh, mb, me) = ((w >> 11) & 31, (w >> 6) & 31, (w >> 1) & 31);
                let mask = if mb <= me {
                    (u32::MAX >> mb) & (u32::MAX << (31 - me))
                } else {
                    (u32::MAX >> mb) | (u32::MAX << (31 - me))
                };
                regs[ra] = Some(regs[rd]?.rotate_left(sh) & mask);
            }
            31 => match (w >> 1) & 1023 {
                266 => regs[rd] = Some(regs[ra]?.wrapping_add(regs[rb]?)),
                23 => {
                    let address = base(&regs)?.wrapping_add(regs[rb]?);
                    loaded = Some((rd, address));
                    regs[rd] = img.word(address);
                }
                444 if rd == rb => regs[ra] = regs[rd], // mr
                _ => return None,
            },
            32 => {
                let address = base(&regs)?.wrapping_add(simm);
                loaded = Some((rd, address));
                regs[rd] = img.word(address);
            }
            _ => return None,
        }
    }
    None
}

/// (table address, row stride in words) when `function` reads `table[r4]`
/// (stride 1) or `table[r4 * stride + r5]`.
fn accessor(img: &Image, function: u32, two_d: bool) -> Option<(u32, u32)> {
    let Leaf::Load(origin) = leaf(img, function, 0, 0)? else {
        return None;
    };
    if two_d {
        let Leaf::Load(next_slot) = leaf(img, function, 0, 1)? else {
            return None;
        };
        let Leaf::Load(next_row) = leaf(img, function, 1, 0)? else {
            return None;
        };
        let stride = next_row.wrapping_sub(origin) / 4;
        (next_slot == origin + 4 && next_row > origin && (2..=16).contains(&stride))
            .then_some((origin, stride))
    } else {
        let Leaf::Load(next) = leaf(img, function, 1, 0)? else {
            return None;
        };
        (next == origin + 4).then_some((origin, 1))
    }
}

struct RawMode {
    vtable: u32,
    title: String,
    slots: u32,
    tabs: Vec<(i32, Vec<i32>)>,
}

fn raw_modes(img: &Image) -> Vec<RawMode> {
    let words: Vec<u32> = img
        .image
        .image
        .chunks_exact(4)
        .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
        .collect();
    let base = img.image.base_address;
    let code = |w: u32| w % 4 == 0 && img.inside(w);
    let mut modes = Vec::new();
    for i in 0..words.len().saturating_sub(VTABLE_SPAN + 1) {
        let (tabs_fn, rows_fn) = (words[i], words[i + 1]);
        if !code(tabs_fn) || !code(rows_fn) || tabs_fn == rows_fn {
            continue;
        }
        let Some((tabs_at, _)) = accessor(img, tabs_fn, false) else {
            continue;
        };
        let Some((rows_at, slots)) = accessor(img, rows_fn, true) else {
            continue;
        };
        let (mut count, mut title) = (None, None);
        for &slot in &words[i + 2..i + 1 + VTABLE_SPAN] {
            if !code(slot) {
                continue;
            }
            // Only two-instruction getters (`li r3,n` / `lis+addi r3`; blr).
            let short =
                img.word(slot + 4) == Some(0x4e80_0020) || img.word(slot + 8) == Some(0x4e80_0020);
            match (short, leaf(img, slot, 0, 0)) {
                (true, Some(Leaf::Value(n))) if (1..=16).contains(&n) => {
                    count.get_or_insert(n);
                }
                (true, Some(Leaf::Value(p))) => {
                    if let Some(text) = img.cstr(p).filter(|t| !t.is_empty()) {
                        title.get_or_insert(text);
                    }
                }
                _ => {}
            }
        }
        let (Some(count), Some(title)) = (count, title) else {
            continue;
        };
        let read = |a: u32| img.word(a).map(|w| w as i32);
        let tabs: Option<Vec<(i32, Vec<i32>)>> = (0..count)
            .map(|t| {
                let category = read(tabs_at + t * 4)?;
                let row: Option<Vec<i32>> = (0..slots)
                    .map(|s| read(rows_at + (t * slots + s) * 4))
                    .collect();
                Some((
                    category,
                    row?.into_iter().take_while(|v| *v != -1).collect(),
                ))
            })
            .collect();
        if let Some(tabs) = tabs {
            // The tab accessor is the slot after the destructor.
            modes.push(RawMode {
                vtable: base + (i as u32) * 4 - 4,
                title,
                slots,
                tabs,
            });
        }
    }
    modes
}

/// Stable keys. Retail has no names for the mode classes; Career / Free Play
/// share their titles with their park variants (9-slot rows) and Free Play with
/// the restricted free-skate menu (no ResumeCareer on its Main tab).
fn mode_key(mode: &ModeMenu) -> String {
    let main_has = |name: &str| {
        mode.tabs
            .iter()
            .any(|t| t.category == "SinglePlayer" && t.items.iter().any(|i| i == name))
    };
    match (mode.title.as_str(), mode.slots) {
        ("ID_CROSSBAR_SINGLE_PLAYER_MODE_TITLE", 9) => "CareerPark".into(),
        ("ID_CROSSBAR_SINGLE_PLAYER_MODE_TITLE", _) => "Career".into(),
        ("ID_CROSSBAR_OFFLINE_FREESKATE_MODE_TITLE", 9) => "FreePlayPark".into(),
        ("ID_CROSSBAR_OFFLINE_FREESKATE_MODE_TITLE", _) if main_has("ResumeCareer") => {
            "FreePlay".into()
        }
        ("ID_CROSSBAR_OFFLINE_FREESKATE_MODE_TITLE", _) => "FreeSkateRestricted".into(),
        (title, _) => {
            let core = title
                .trim_start_matches("ID_CROSSBAR_")
                .trim_end_matches("_MODE_TITLE");
            core.chars()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect()
        }
    }
}

fn modes(img: &Image, categories: &[Category], items: &[Item]) -> Vec<ModeMenu> {
    let name = |list: &[String], i: i32| usize::try_from(i).ok().and_then(|i| list.get(i)).cloned();
    let category_names: Vec<String> = categories.iter().map(|c| c.name.clone()).collect();
    let item_names: Vec<String> = items.iter().map(|c| c.name.clone()).collect();
    let mut out: Vec<ModeMenu> = Vec::new();
    for raw in raw_modes(img) {
        let tabs: Option<Vec<ModeTab>> = raw
            .tabs
            .iter()
            .map(|(category, row)| {
                Some(ModeTab {
                    category: name(&category_names, *category)?,
                    items: row
                        .iter()
                        .map(|i| name(&item_names, *i))
                        .collect::<Option<_>>()?,
                })
            })
            .collect();
        let Some(tabs) = tabs else { continue };
        let mut mode = ModeMenu {
            key: String::new(),
            title: raw.title,
            slots: raw.slots,
            tabs,
            vtable: hex(raw.vtable),
        };
        let key = mode_key(&mode);
        let taken = |k: &str| out.iter().any(|m| m.key == k);
        mode.key = if taken(&key) {
            (2..)
                .map(|n| format!("{key}_{n}"))
                .find(|k| !taken(k))
                .unwrap()
        } else {
            key
        };
        out.push(mode);
    }
    out
}

pub fn extract(image: &XexImage) -> Result<MenuTables, String> {
    let img = Image { image };
    let (rows_at, settings_rows, settings_screens) = settings(&img)?;
    let (items_at, items, categories_at, categories) = crossbar(&img)?;
    let (keys_at, keys) = keys(&img)?;
    let modes = modes(&img, &categories, &items);
    if modes.is_empty() {
        return Err("no per-mode crossbar menus found".into());
    }
    let addresses = [
        ("settings_rows", rows_at),
        ("settings_screens", rows_at + settings_rows.len() as u32 * 8),
        ("crossbar_categories", categories_at),
        ("crossbar_items", items_at),
        ("apt_keys", keys_at),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), hex(v)))
    .collect();
    Ok(MenuTables {
        schema: SCHEMA,
        source_sha256: crate::sha256::digest(&image.image),
        settings_rows,
        settings_screens,
        categories,
        items,
        keys,
        modes,
        addresses,
    })
}

/// Unpacks `xex`, extracts the tables and returns the game's JSON.
pub fn json_from_xex(xex: &[u8]) -> Result<String, String> {
    let image = XexImage::parse(xex)?;
    Ok(extract(&image)?.to_json())
}

impl MenuTables {
    /// The engine menu structure with these tables as its defaults: entries keyed
    /// by retail internal names, settings rows by string id, retail links.
    pub fn to_menu_data(&self) -> skate_core::menus::MenuData {
        use skate_core::menus as m;
        let keys: Vec<String> = self
            .settings_rows
            .iter()
            .enumerate()
            .map(|(i, r)| m::setting_key(i, &r.id))
            .collect();
        m::MenuData {
            categories: self
                .categories
                .iter()
                .map(|c| m::Category {
                    id: c.name.clone(),
                    label: c.label.clone(),
                    icon: m::Icon::Retail(c.icon.clone()),
                })
                .collect(),
            items: self
                .items
                .iter()
                .map(|i| m::Item {
                    id: i.name.clone(),
                    label: i.label.clone(),
                    icon: m::Icon::Retail(i.icon.clone()),
                    help: i.help.clone(),
                    sub_option_kind: i.sub_option_kind,
                })
                .collect(),
            settings: self
                .settings_rows
                .iter()
                .zip(&keys)
                .map(|(r, key)| m::Setting {
                    id: key.clone(),
                    label: r.id.clone(),
                    widget: m::Widget::parse(&r.widget).unwrap_or(m::Widget::Option),
                    link: m::retail_link(key),
                })
                .collect(),
            screens: self
                .settings_screens
                .iter()
                .map(|rows| {
                    rows.iter()
                        .filter_map(|r| keys.get(*r as usize).cloned())
                        .collect()
                })
                .collect(),
            modes: self
                .modes
                .iter()
                .map(|mode| m::Mode {
                    key: mode.key.clone(),
                    title: mode.title.clone(),
                    slots: mode.slots as usize,
                    tabs: mode
                        .tabs
                        .iter()
                        .map(|t| m::Tab {
                            category: t.category.clone(),
                            items: t.items.clone(),
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}
