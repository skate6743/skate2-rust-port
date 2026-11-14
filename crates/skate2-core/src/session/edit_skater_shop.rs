//! The shop of `FrontEndState_CaC` and its `FrontEndState_Receipt` (`festate_cac.cpp`,
//! `festate_receipt.cpp`): a catalog part's status, the parts tried on that cost money (the
//! receipt list), the receipt screen and the purchase.
use skate2_formats::{
    cac_parts::{CacPart, CacPartDb, CacPartError},
    skater_profile::{CacPartId, CacRecord, ProfileError},
};

/// `kUnlockCategories` 15 and 16: the unlockables `LockedFilter` reads a part's unlock and
/// sponsor unlock in.
pub(crate) const CLOTHING_UNLOCK: u32 = 15;
pub(crate) const CLOTHING_SPONSOR: u32 = 16;
// RawPartDataV6's u16 fields from byte 28 follow CACPart's pointers from
// pAlternateMaterialName (+0x2C): pDlcID +0x48, pSponsorUnlock +0xC0, pUnlock +0xD0.
const DLC: usize = 7;
const SPONSOR_UNLOCK: usize = 37;
const UNLOCK: usize = 41;

/// `*pCost` as the receipt sums it (`GetReceiptBalance` `0x82649ec8`): -1 reads as 0.
#[must_use]
pub(crate) fn cost(part: &CacPart) -> i32 {
    match part.cost.cast_signed() {
        -1 => 0,
        cost => cost,
    }
}

fn id(part: &CacPart) -> CacPartId {
    CacPartId { model: part.model_id, material: part.material_id }
}

/// `rw::core::stdc::ConvertAToI`: leading spaces, a sign, then the digits.
fn atoi(text: &str) -> i32 {
    let text = text.trim_start();
    let (negative, digits) = match text.as_bytes().first() {
        Some(b'-') => (true, &text[1..]),
        Some(b'+') => (false, &text[1..]),
        _ => (false, text),
    };
    let value = digits
        .bytes()
        .take_while(u8::is_ascii_digit)
        .fold(0i32, |value, digit| value.wrapping_mul(10).wrapping_add(i32::from(digit - b'0')));
    if negative { value.wrapping_neg() } else { value }
}

/// `LockedFilter::AcceptImpl` (`0x82649010`): a part with an unlock (`pUnlock`, category 15)
/// or a sponsor unlock (`pSponsorUnlock`, category 16) stays locked until one of them is
/// unlocked.
pub(crate) fn is_locked(
    db: &CacPartDb,
    part: &CacPart,
    unlocked: &impl Fn(u32, &str) -> bool,
) -> Result<bool, CacPartError> {
    let (unlock, sponsor) = (part.attributes[UNLOCK], part.attributes[SPONSOR_UNLOCK]);
    if unlock == u16::MAX && sponsor == u16::MAX {
        return Ok(false);
    }
    if unlock != u16::MAX && unlocked(CLOTHING_UNLOCK, db.string(unlock)?) {
        return Ok(false);
    }
    if sponsor == u16::MAX {
        return Ok(true);
    }
    Ok(!unlocked(CLOTHING_SPONSOR, db.string(sponsor)?))
}

/// PC port addition (Max Out Save): every unlock (category 15) and sponsor unlock (category 16) a
/// part Edit Skater offers waits for, each once, in catalog order.
pub(crate) fn clothing_unlocks(db: &CacPartDb) -> Result<(Vec<String>, Vec<String>), CacPartError> {
    let mut unlocks: Vec<String> = Vec::new();
    let mut sponsors: Vec<String> = Vec::new();
    for part in db.parts.iter().filter(|part| offered(part)) {
        for (attribute, names) in [(UNLOCK, &mut unlocks), (SPONSOR_UNLOCK, &mut sponsors)] {
            let value = part.attributes[attribute];
            if value == u16::MAX {
                continue;
            }
            let name = db.string(value)?;
            if !names.iter().any(|known| known == name) {
                names.push(name.to_owned());
            }
        }
    }
    Ok((unlocks, sponsors))
}

/// The inventory's capacity: a record holds fewer owned parts than this (`CACRecord`'s 6000).
const INVENTORY: usize = 6000;

/// PC port addition (Max Out Save): every part the shop sells (one Edit Skater offers, with a
/// cost, not a part used in game) and the record does not own joins its inventory, as a purchase
/// adds a part (`AddPartToInventory`: component types from 4), while the inventory stays within
/// its capacity: the skater's gender's parts and the ungendered ones first, then the other
/// gender's (the editor can switch it). How many joined, and how many did not fit.
/// `SKATE2_TRACE_MAX_OUT` logs each gender's parts.
pub(crate) fn own_everything(db: &CacPartDb, record: &mut CacRecord) -> Result<(usize, usize), ProfileError> {
    const GENDER: usize = 13;
    let other_name = if record.data.is_male { "female" } else { "male" };
    let other_gender = db.strings.iter().position(|s| s == other_name).and_then(|i| u16::try_from(i).ok());
    let invalid = CacPartId::invalid();
    let before: usize = record.owned_parts.iter().map(Vec::len).sum();
    let mut owned = before;
    let (mut added, mut missed) = (0, 0);
    for other in [false, true] {
        // (Skate 2's, Skate 3's) parts sold: already owned, joined, did not fit.
        let mut counts = [[0usize; 3]; 2];
        for (index, part) in db.parts.iter().enumerate() {
            let slot = usize::from(part.slot);
            if (other_gender == Some(part.attributes[GENDER])) != other
                || !offered(part)
                || part.slot < 4
                || slot >= record.owned_parts.len()
                || cost(part) <= 0
                || matches!(part.used_in_game, 1..=254)
                || id(part) == invalid
            {
                continue;
            }
            let game = usize::from(db.is_skate3(index));
            if record_owns(record, id(part)) {
                counts[game][0] += 1;
            } else if owned + 1 >= INVENTORY {
                counts[game][2] += 1;
                missed += 1;
            } else {
                record.owned_parts[slot].push(id(part));
                counts[game][1] += 1;
                owned += 1;
                added += 1;
            }
        }
        if std::env::var_os("SKATE2_TRACE_MAX_OUT").is_some() {
            let gender = if other { "other gender" } else { "skater's gender and ungendered" };
            eprintln!("max out: {gender} parts sold (owned, joined, did not fit): Skate 2 {:?}, Skate 3 {:?}", counts[0], counts[1]);
        }
    }
    if std::env::var_os("SKATE2_TRACE_MAX_OUT").is_some() {
        eprintln!("max out: inventory {before} -> {owned} of {}", INVENTORY - 1);
    }
    record.rebuild_unused_owned_slots(invalid)?;
    Ok((added, missed))
}

/// Whether Edit Skater offers the part: one without a DLC id (`DLCFilter::AcceptImpl`
/// `0x82670190`, `EditSkaterFrontend`'s `available_parts`).
fn offered(part: &CacPart) -> bool {
    const DLC_IDS: usize = 7;
    part.attributes[DLC_IDS] == u16::MAX
}

/// `NewFilter::AcceptImpl` (`0x82648e00`): the part's unlock (category 15) or sponsor unlock
/// (category 16) is a new unlock.
pub(crate) fn is_new(
    db: &CacPartDb,
    part: &CacPart,
    new_unlock: &impl Fn(u32, &str) -> bool,
) -> Result<bool, CacPartError> {
    let (unlock, sponsor) = (part.attributes[UNLOCK], part.attributes[SPONSOR_UNLOCK]);
    let new = unlock != u16::MAX && new_unlock(CLOTHING_UNLOCK, db.string(unlock)?);
    Ok(sponsor != u16::MAX && new_unlock(CLOTHING_SPONSOR, db.string(sponsor)?) || new)
}

/// `DLCFilter::AcceptImpl` (`0x82670190`): a part with a DLC id list whose every
/// comma-separated id reads non-zero.
pub(crate) fn is_dlc(db: &CacPartDb, part: &CacPart) -> Result<bool, CacPartError> {
    let dlc = part.attributes[DLC];
    if dlc == u16::MAX {
        return Ok(false);
    }
    Ok(db.string(dlc)?.split(',').all(|id| atoi(id) != 0))
}

/// `CACRecord::IsOwnedPartID(partid)` (`0x82b67d48`): installed in, or owned under, any slot.
#[must_use]
pub(crate) fn record_owns(record: &CacRecord, part: CacPartId) -> bool {
    record.installed_parts.contains(&part) || record.owned_parts.iter().any(|owned| owned.contains(&part))
}

/// `CASSimManager::GetWearStyleAlternates(alts, pref, false)` (`0x8259d710`): the part first,
/// then, for a part with a style subcategory or a hat, the parts of its slot that `GetParts`
/// (`0x8259d510`) keeps under `PARTCONDITION_SHINYNEW`: of a pack on disk, accepted by
/// `WearStyleFilter` (`SetPart` `0x82589318`, `AcceptImpl` `0x82589390`: its gender, another
/// part, its wear style group) and not a used one.
pub(crate) fn wear_style_alternates(db: &CacPartDb, available: &[bool], index: usize) -> Vec<usize> {
    const STYLE_SUBCATEGORY: usize = 38;
    const MATERIAL_VARIANT_NAME: usize = 26;
    const GENDER: usize = 13;
    let part = &db.parts[index];
    let mut alternates = vec![index];
    let hat = part.slot == 18;
    if part.attributes[STYLE_SUBCATEGORY] != u16::MAX || hat {
        // A hat without a style subcategory groups on its material variant, for itself and for
        // the hats it is offered with.
        let group = crate::ui::edit_skater::wear_style_group(part);
        alternates.extend(db.slot_range(part.slot).filter(|&i| {
            let q = &db.parts[i];
            let q_group = if hat && q.attributes[STYLE_SUBCATEGORY] == u16::MAX { q.attributes[MATERIAL_VARIANT_NAME] } else { q.attributes[STYLE_SUBCATEGORY] };
            available.get(i).copied().unwrap_or(false)
                && q.attributes[GENDER] == part.attributes[GENDER]
                && (q.model_id, q.material_id) != (part.model_id, part.material_id)
                && q_group == group
                && matches!(q.used_in_game, 0 | u8::MAX)
        }));
    }
    alternates
}

/// What `OnThumbnailSelect` (`0x8268aee0`) does with a part the shop sells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ShopSelection {
    /// Locked: the denied sound, nothing worn.
    Locked,
    /// Owned or already on the receipt: worn.
    Wear,
    /// Added to the receipt (screen state 52) and worn.
    AddedToReceipt,
    /// The balance does not cover it: the denied sound and `ID_CAC_NOT_ENOUGH_CASH`.
    NotEnoughCash,
}

/// `FrontEndState_Receipt::mState`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReceiptState {
    /// `eReceiptState_ReceiptList`: the lines, toggled with X.
    ReceiptList,
    /// `eReceiptState_MenuPicker`: Purchase, Edit Cart, Back to Store.
    MenuPicker,
}

/// The pushed `FrontEndState_Receipt` (screen 122, `cas/receipt`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReceiptScreen {
    /// `mMode` (`SetState`): 0 checkout, 1 edit the cart.
    pub mode: u8,
    pub state: ReceiptState,
    /// `mNewReceiptList`: the lines kept, as catalog indices.
    pub kept: Vec<usize>,
    /// The menu picker's and the list's highlighted rows.
    pub option: usize,
    pub row: usize,
}

/// The shop of an open editor.
pub(crate) struct EditSkaterShop {
    /// `cSignPostManager::GetCash`, less what this editor bought.
    cash: i32,
    /// Each line this editor bought, for the career's `SubtractCash` when it closes.
    purchases: Vec<i32>,
    /// `LockedFilter`, `DLCFilter` and `NewFilter` of every catalog part, as the editor opened.
    locked: Vec<bool>,
    dlc: Vec<bool>,
    new: Vec<bool>,
    /// `FrontEndState_Receipt::mReceiptList`.
    receipt: Vec<usize>,
    /// The receipt screen while pushed.
    pub(crate) screen: Option<ReceiptScreen>,
    /// `mLastOptionSelected` of the receipt screen last popped (-1 none).
    pub(crate) last_option: i32,
    /// The `ID_CAC_NOT_ENOUGH_CASH` popup is up.
    pub(crate) not_enough_cash: bool,
}

impl EditSkaterShop {
    pub(crate) fn new(
        db: &CacPartDb,
        cash: i32,
        unlocked: &impl Fn(u32, &str) -> bool,
        new_unlock: &impl Fn(u32, &str) -> bool,
    ) -> Result<Self, CacPartError> {
        let mut locked = Vec::with_capacity(db.parts.len());
        let mut dlc = Vec::with_capacity(db.parts.len());
        let mut new = Vec::with_capacity(db.parts.len());
        for part in &db.parts {
            locked.push(is_locked(db, part, unlocked)?);
            dlc.push(is_dlc(db, part)?);
            new.push(is_new(db, part, new_unlock)?);
        }
        Ok(Self { cash, purchases: Vec::new(), locked, dlc, new, receipt: Vec::new(), screen: None, last_option: -1, not_enough_cash: false })
    }

    /// `FrontEndState_CaC::IsPartNew` (`0x82649590`) of a part thumbnail.
    #[must_use]
    pub(crate) fn is_new(&self, index: usize) -> bool {
        self.new[index]
    }

    /// The cost of each line the editor bought.
    #[must_use]
    pub(crate) fn purchases(&self) -> &[i32] {
        &self.purchases
    }

    /// `cSignPostManager::GetCash` as the editor sees it.
    #[must_use]
    pub(crate) const fn cash(&self) -> i32 {
        self.cash
    }

    #[must_use]
    pub(crate) fn receipt(&self) -> &[usize] {
        &self.receipt
    }

    /// `GetReceiptBalance` (`0x82649ec8`): the cash less every receipt line.
    #[must_use]
    pub(crate) fn balance(&self, db: &CacPartDb) -> i32 {
        self.cash - self.receipt.iter().map(|&i| cost(&db.parts[i])).sum::<i32>()
    }

    /// `OwnedFilter::AcceptImpl` (`0x82652c90`): a free part, else one off the receipt that
    /// the record holds.
    #[must_use]
    pub(crate) fn owned(&self, db: &CacPartDb, record: &CacRecord, index: usize) -> bool {
        let part = &db.parts[index];
        let price = part.cost.cast_signed();
        price == -1 || price == 0 || !self.receipt.contains(&index) && record_owns(record, id(part))
    }

    /// `FrontEndState_CaC::GetPartStatus` (`0x82659be8`).
    #[must_use]
    pub(crate) fn status(&self, db: &CacPartDb, record: &CacRecord, index: usize) -> &'static str {
        if self.locked[index] {
            "locked"
        } else if self.receipt.contains(&index) {
            "purchased"
        } else if self.owned(db, record, index) {
            "owned"
        } else if self.dlc[index] {
            "dlc"
        } else {
            "none"
        }
    }

    /// `OnThumbnailSelect`'s part branch (`0x8268afc4..0x8268b154`).
    pub(crate) fn select(&mut self, db: &CacPartDb, record: &CacRecord, index: usize) -> ShopSelection {
        if self.locked[index] {
            return ShopSelection::Locked;
        }
        if self.receipt.contains(&index) || self.owned(db, record, index) {
            return ShopSelection::Wear;
        }
        if self.balance(db) < cost(&db.parts[index]) {
            self.not_enough_cash = true;
            return ShopSelection::NotEnoughCash;
        }
        // AddPartToList 0x82678718.
        self.receipt.push(index);
        ShopSelection::AddedToReceipt
    }

    /// `FrontEndState_Receipt::OnEnter` (`0x826784f0`) and `OnLoaded` (`0x82669688`): every
    /// line kept, mode 0 on the menu picker, mode 1 on the list.
    pub(crate) fn open_receipt(&mut self, mode: u8) {
        self.last_option = -1;
        let state = if mode == 1 { ReceiptState::ReceiptList } else { ReceiptState::MenuPicker };
        self.screen = Some(ReceiptScreen { mode, state, kept: self.receipt.clone(), option: 0, row: 0 });
    }

    /// `UpdateReceiptList` (`0x826785a0`): the lines no longer kept (with their wear style
    /// alternates) leave the record (`CASSimManager::RemoveParts(.., replace)`
    /// `0x82599010`: `RemovePartID`, then each slot left empty takes the undo point's part),
    /// and the kept lines become the receipt.
    pub(crate) fn update_receipt_list(
        &mut self,
        db: &CacPartDb,
        available: &[bool],
        record: &mut CacRecord,
        undo: &CacRecord,
        kept: &[usize],
    ) -> Result<(), ProfileError> {
        let mut removed = Vec::new();
        for &line in &self.receipt {
            if !kept.contains(&line) {
                removed.extend(wear_style_alternates(db, available, line));
            }
        }
        let invalid = CacPartId::invalid();
        let mut emptied = [false; 26];
        for index in removed {
            let part = &db.parts[index];
            let slot = usize::from(part.slot);
            if slot >= 26 {
                continue;
            }
            // CACRecord::RemovePartID 0x82b68690: the installed part leaves its slot, else
            // the owned list loses it.
            if record.installed_parts[slot] == id(part) {
                record.installed_parts[slot] = invalid;
            } else if let Some(at) = record.owned_parts[slot].iter().position(|&owned| owned == id(part)) {
                record.owned_parts[slot].remove(at);
            }
            emptied[slot] = record.installed_parts[slot] == invalid;
        }
        for (slot, emptied) in emptied.into_iter().enumerate() {
            if emptied {
                // ExchangePart 0x82b6bfd8 with the undo point's installed part.
                record.installed_parts[slot] = undo.installed_parts[slot];
                if let Some(at) = record.owned_parts[slot].iter().position(|&owned| owned == undo.installed_parts[slot]) {
                    record.owned_parts[slot].remove(at);
                }
            }
        }
        record.rebuild_unused_owned_slots(invalid)?;
        self.receipt = kept.to_vec();
        Ok(())
    }

    /// `OnMenuSelect`'s purchase (`0x82680fe4..0x82681170`): each receipt line's cost leaves
    /// the cash and its wear style alternates join the record's inventory
    /// (`AddPartToInventory` `0x825857c0`: component types from 4, `SetOwnedPartID`
    /// `0x82b6bbd0`), then both lists empty.
    pub(crate) fn purchase(&mut self, db: &CacPartDb, available: &[bool], record: &mut CacRecord) -> Result<(), ProfileError> {
        let invalid = CacPartId::invalid();
        for line in std::mem::take(&mut self.receipt) {
            let price = cost(&db.parts[line]);
            self.cash -= price;
            self.purchases.push(price);
            for alternate in wear_style_alternates(db, available, line) {
                let part = &db.parts[alternate];
                let slot = usize::from(part.slot);
                if part.slot >= 4 && slot < 26 && id(part) != invalid && !record_owns(record, id(part)) {
                    record.owned_parts[slot].push(id(part));
                }
            }
        }
        record.rebuild_unused_owned_slots(invalid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atoi_reads_like_the_c_library() {
        assert_eq!((atoi(" 12,"), atoi("-3"), atoi(""), atoi("x1")), (12, -3, 0, 0));
    }
}
