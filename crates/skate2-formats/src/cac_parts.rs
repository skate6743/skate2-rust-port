//! CACPartDB::Initialize82b6ca18, string table82b68590, part table82b6c828.
//! Tables retain their authored order; slots index the original contiguous ranges.
use crate::recipe::{CharacterRecipe, CharacterRecipeError};
use std::ops::Range;

pub const DATABASE_PATH: &str = "data\\content\\recipe\\createacharacter\\cas_db_000.recipe";
/// PC port: a linked Skate 3 data folder's clothing database (`crate::skate3` in the core).
pub const SKATE3_DATABASE_PATH: &str = "data\\content\\recipe\\createacharacter\\cas_db_xfd1ca62f_000.recipe";
pub const SLOT_NAMES: [&str; 26] = ["Arm","Leg","Hair","Rostral","Torso","Feet",
    "Accessory","Equipment","Skeleton","Organ","Misc","SkateBoard","SkateTruck",
    "SkateWheel","Beard","Pants","Underwear","Sock","Hat","Belt","Glasses",
    "WristItem","InnerTorso","OuterTorso","Piercings","Jewellery"];

#[derive(Clone, Debug)]
pub struct CacPartDb {
    pub master: CharacterRecipe,
    pub parts: Vec<CacPart>,
    pub strings: Vec<String>,
    ranges: [Range<usize>; 26],
    /// PC port: each slot's parts that [`CacPartDb::with_skate3`] joined, the end of its range.
    skate3: [Range<usize>; 26],
}
#[derive(Clone, Debug)]
pub struct CacPart {
    pub model_id: u64,
    pub material_id: u64,
    pub cost: u32,
    pub slot: u8,
    pub colourizable: u8,
    pub default: u8,
    pub default_wear_style: u8,
    pub layered: u8,
    pub logoable: u8,
    pub skinned: u8,
    pub used_in_game: u8,
    pub attributes: [u16; 43],
}
/// PC port: Skate 3's part record (table version 9): the model, material and cost as Skate 2's,
/// 80 bytes of colour data, the slot and nine flags (Skate 2's seven with `IsNecklaceRemoved`
/// after `IsLogoable` and `IsTattooViewingChoice` after `IsSkinned`), then the string indices,
/// Skate 2's 43 with `TattooCategory` before its last three. Matched field by field against
/// Skate 3's `cas_db_xfd1ca62f_000.xml`.
const SKATE3_RECORD: usize = 208;
/// `Unlock` and `SponsorUnlock` among the 43 string indices.
const UNLOCK_INDEX: usize = 41;
const SPONSOR_UNLOCK_INDEX: usize = 37;

fn skate3_part(b: &[u8]) -> CacPart {
    let index = |i: usize| u16::from_be_bytes([b[110 + i * 2], b[111 + i * 2]]);
    CacPart {
        model_id: u64::from_be_bytes(b[..8].try_into().unwrap()),
        material_id: u64::from_be_bytes(b[8..16].try_into().unwrap()),
        cost: u32::from_be_bytes(b[16..20].try_into().unwrap()),
        slot: b[100],
        colourizable: b[101],
        default: b[102],
        default_wear_style: b[103],
        layered: b[104],
        logoable: b[105],
        skinned: b[107],
        used_in_game: b[109],
        attributes: std::array::from_fn(|i| index(if i < 40 { i } else { i + 1 })),
    }
}

#[derive(Debug)]
pub enum CacPartError { Recipe(CharacterRecipeError), Invalid(&'static str), StringIndex(u16), Attribute(u32) }
impl std::fmt::Display for CacPartError {
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result { write!(f,"CAC part database: {self:?}") }
}
impl std::error::Error for CacPartError {}
impl CacPartDb {
    pub fn parse(bytes: &[u8]) -> Result<Self, CacPartError> {
        use CacPartError as E;
        let (master, consumed) = CharacterRecipe::parse_prefix(bytes).map_err(E::Recipe)?;
        let word = |offset: usize| -> Result<u32,E> {
            let b = bytes.get(offset..offset+4).ok_or(E::Invalid("truncated table"))?;
            Ok(u32::from_be_bytes(b.try_into().unwrap()))
        };
        let start = (consumed+15)&!15;
        let count = word(start)? as usize;
        if count > bytes.len()/4 { return Err(E::Invalid("string count")); }
        let mut strings = Vec::with_capacity(count);
        let mut end = start;
        for i in 0..count {
            let mut offset = word(start+4+i*4)? as usize;
            if offset < start { offset = offset.checked_add(0x10000).ok_or(E::Invalid("string offset"))?; }
            let rest = bytes.get(offset..).ok_or(E::Invalid("string offset"))?;
            let size = rest.iter().position(|&b| b==0).ok_or(E::Invalid("string terminator"))?;
            strings.push(std::str::from_utf8(&rest[..size]).map_err(|_|E::Invalid("string encoding"))?.to_owned());
            end = end.max(offset+size+1);
        }
        let start = (end+15)&!15;
        let count = word(start)? as usize;
        let version = word(start+4)?;
        // PC port: Skate 3's table (version 9) widens each part to 208 bytes.
        let record = match version { 6 | 7 => 128, 9 => SKATE3_RECORD, _ => return Err(E::Invalid("part table version")) };
        let header = start+8;
        let raw = (header+208+15)&!15;
        let raw_end = raw.checked_add(count.checked_mul(record).ok_or(E::Invalid("part count"))?).ok_or(E::Invalid("part count"))?;
        if raw_end.checked_add(4) != Some(bytes.len()) { return Err(E::Invalid("part table extent")); }
        let mut ranges: [Range<usize>;26] = std::array::from_fn(|_|0..0);
        for (slot, range) in ranges.iter_mut().enumerate() {
            let count = word(header+slot*4)? as usize;
            let first = word(header+104+slot*4)? as usize;
            let last = first.checked_add(count).ok_or(E::Invalid("slot range"))?;
            if last > (raw_end-raw)/record { return Err(E::Invalid("slot range")); }
            *range=first..last;
        }
        let mut parts=Vec::with_capacity(count);
        for b in bytes[raw..raw_end].chunks_exact(record) {
            parts.push(if record == SKATE3_RECORD { skate3_part(b) } else {
                CacPart { model_id:u64::from_be_bytes(b[..8].try_into().unwrap()),
                material_id:u64::from_be_bytes(b[8..16].try_into().unwrap()), cost:u32::from_be_bytes(b[16..20].try_into().unwrap()),
                slot:b[20], colourizable:b[21], default:b[22], default_wear_style:b[23],layered:b[24],
                logoable:b[25],skinned:b[26],used_in_game:b[27],
                attributes:std::array::from_fn(|i|u16::from_be_bytes([b[28+i*2],b[29+i*2]])) } });
        }
        let mut result = Self {master, parts, strings, ranges, skate3: std::array::from_fn(|_|0..0)};
        result.apply_native_fixes()?;
        Ok(result)
    }
    pub fn string(&self,index:u16)->Result<&str,CacPartError> {
        if index == u16::MAX { return Ok(""); }
        self.strings.get(usize::from(index)).map(String::as_str).ok_or(CacPartError::StringIndex(index))
    }
    pub fn attribute(&self,part:&CacPart,attribute:u32)->Result<&str,CacPartError> {
        let index = match attribute {0..=2=>attribute,5..=23=>attribute-2,31..=34=>attribute-9,
            36=>26,38..=53=>attribute-11,_=>return Err(CacPartError::Attribute(attribute))};
        self.string(part.attributes[index as usize])
    }
    pub fn parts_in_slot(&self,slot:u8)->impl Iterator<Item=&CacPart> {
        self.parts[self.slot_range(slot)].iter()
    }
    /// The indices of a slot's parts (`CACPartDB::GetParts`): its authored range, which a runtime
    /// correction of a part's ComponentType leaves it in.
    pub fn slot_range(&self,slot:u8)->Range<usize> {
        self.ranges.get(usize::from(slot)).cloned().unwrap_or(0..0)
    }
    pub fn find(&self,slot:u8,model:u64,material:u64)->Option<&CacPart> {
        self.parts_in_slot(slot).find(|p|p.model_id==model&&p.material_id==material)
    }

    /// PC port: this database with `skate3`'s parts of `slots` (a linked Skate 3 data folder's,
    /// read by [`CacPartDb::parse`]) after each slot's own: those this one lacks (by model and
    /// material), their strings joined to this one's table, and with no unlock (Skate 3's unlocks
    /// are not this game's). [`CacPartDb::is_skate3`] tells them apart.
    pub fn with_skate3(&self, skate3: &CacPartDb, slots: &[u8]) -> Result<Self, CacPartError> {
        // The master recipe's components hold each part's model; the joined slots' models join
        // this one's components (a slot it lacks joins whole).
        let mut master = self.master.clone();
        for &slot in slots {
            let Some(name) = SLOT_NAMES.get(usize::from(slot)) else { continue };
            let Some(theirs) = skate3.master.components.iter().find(|component| component.name == *name) else { continue };
            match master.components.iter_mut().find(|component| component.name == *name) {
                Some(ours) => {
                    for model in &theirs.models {
                        if !ours.models.iter().any(|held| held.asset_id == model.asset_id) {
                            ours.models.push(model.clone());
                        }
                    }
                }
                None => master.components.push(theirs.clone()),
            }
        }
        let mut merged = Self { master, parts: Vec::with_capacity(self.parts.len() + skate3.parts.len()),
            strings: self.strings.clone(), ranges: std::array::from_fn(|_|0..0), skate3: std::array::from_fn(|_|0..0) };
        let mut index: std::collections::HashMap<String, u16> = merged.strings.iter().enumerate()
            .map(|(i, s)| (s.clone(), u16::try_from(i).unwrap_or(u16::MAX))).collect();
        for slot in 0..26_u8 {
            let start = merged.parts.len();
            merged.parts.extend(self.parts_in_slot(slot).cloned());
            let own = merged.parts.len();
            let joined = if slots.contains(&slot) { skate3.slot_range(slot) } else { 0..0 };
            for part in &skate3.parts[joined] {
                if self.find(slot, part.model_id, part.material_id).is_some() {
                    continue;
                }
                let mut part = part.clone();
                for (at, attribute) in part.attributes.iter_mut().enumerate() {
                    if *attribute == u16::MAX {
                        continue;
                    }
                    if matches!(at, UNLOCK_INDEX | SPONSOR_UNLOCK_INDEX) {
                        *attribute = u16::MAX;
                        continue;
                    }
                    let text = skate3.string(*attribute)?;
                    *attribute = match index.get(text) {
                        Some(&i) => i,
                        None => {
                            let i = u16::try_from(merged.strings.len()).map_err(|_| CacPartError::Invalid("joined strings capacity"))?;
                            merged.strings.push(text.to_owned());
                            index.insert(text.to_owned(), i);
                            i
                        }
                    };
                }
                merged.parts.push(part);
            }
            merged.ranges[usize::from(slot)] = start..merged.parts.len();
            merged.skate3[usize::from(slot)] = own..merged.parts.len();
        }
        Ok(merged)
    }

    /// PC port: whether part `index` is one [`CacPartDb::with_skate3`] joined.
    #[must_use]
    pub fn is_skate3(&self, index: usize) -> bool {
        self.skate3.iter().any(|range| range.contains(&index))
    }

    /// Original runtime data corrections, CACDBPartContainer::HackFixData82b6c048.
    /// Slot ranges remain unchanged when a correction changes ComponentType.
    fn apply_native_fixes(&mut self)->Result<(),CacPartError> {
        let mut intern = |s:&str| -> Result<u16,CacPartError> {
            if let Some(i)=self.strings.iter().position(|v|v==s) { return Ok(i as u16); }
            let i=u16::try_from(self.strings.len()).map_err(|_|CacPartError::Invalid("extra strings capacity"))?;
            self.strings.push(s.to_owned()); Ok(i)
        };
        let diamond=intern("diamond")?;let anti_hero=intern("anti_hero")?;let style7=intern("style7")?;
        let index=|s:&str|self.strings.iter().position(|v|v==s).map_or(u16::MAX,|i|i as u16);
        let diamondsupply=index("diamondsupply");let antihero=index("antihero");let zero=index("0");
        let female=index("female");let long_loose=index("long loose");let normal=index("normal");
        let pirate=index("Street_Pirate_Hat2");let newera=index("hat_newera_hMat_77");
        let wind_closed=index("windbreaker_closed_male");let wind_open=index("windbreaker_open_male");
        let polo=index("shortsleeve_polo_shirt_male");let diffuse7=index("diffuse_1_tga7");
        let wind_mat=index("windbreaker_openMATbase");let blinn94=index("blinn94");
        let diffuse179=index("txture_diffuse_179");let diffuse34=index("diffuse_34_tga");
        let female_shirts=[index("shortsleeve_tshirt_medium_female"),index("shortsleeve_tshirt_baggy_female"),index("shortsleeve_tshirt_female")];
        let circa=index("circa");
        for slot in 0..26 {
            for part in &mut self.parts[self.ranges[slot].clone()] {
                let a=&mut part.attributes;
                if a[25]==diamondsupply {a[25]=diamond;} else if a[25]==antihero {a[25]=anti_hero;}
                match slot {
                    2=>{if a[17]==long_loose&&a[13]==female{part.slot=19;} if a[30]==u16::MAX{a[30]=zero;}},
                    3=>{if a[9]==u16::MAX{a[9]=style7;}if a[15]==u16::MAX{a[15]=zero;}},
                    11=>{
                        let name=self.strings.get(a[26] as usize).ok_or(CacPartError::StringIndex(a[26]))?;
                        if name.contains("_stampable_"){if !name.contains("dirty"){part.colourizable=1;}}
                        else if name.ends_with(['_', 'd']) {part.used_in_game=1;}
                    },
                    18=>{if a[26]==pirate||a[26]==newera{part.slot=19;part.default_wear_style=0;}
                        else if a[18]==normal{part.default_wear_style=1;}},
                    23=>{
                        if (a[27]==wind_closed&&a[26]==diffuse7)||(a[27]==wind_open&&a[26]==wind_mat)
                            ||(a[27]==polo&&a[26]==blinn94)||(female_shirts.contains(&a[27])&&a[26]==diffuse179){
                            part.slot=19;a[40]=u16::MAX;
                        }else if female_shirts.contains(&a[27])&&a[26]==diffuse34{a[25]=circa;}
                    },
                    _=>{},
                }
            }
        }
        Ok(())
    }
}
