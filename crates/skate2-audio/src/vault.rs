//! Attribute vault access for the audio code. The game reads `Attrib::Instance`s of the `aud_*`
//! classes in `data\db\skater.vlt`; the host supplies them through [`AttribSource`] (the app
//! implements it over the port's `AttribDb`), so inheritance and layout defaults follow the
//! native `Instance::GetAttributePointer`.

/// `Attrib::Instance::GetAttributePointer` and the layout pointer of one collection.
pub trait AttribSource: Send + Sync {
    /// Element `index` of attribute `key` (its definition-sized bytes), `None` for a null
    /// pointer (missing collection or attribute, or an index past the array).
    fn attribute(&self, class: u64, collection: u64, key: u64, index: u32) -> Option<Vec<u8>>;
    /// Word `index` of the collection's layout (`mLayoutPtr`), 0 when missing
    /// (`DefaultDataArea`).
    fn layout_word(&self, class: u64, collection: u64, index: u32) -> u32;
    /// `Attrib::FindCollection`.
    fn has_collection(&self, class: u64, collection: u64) -> bool;
    /// The string a `char*` word of the collection's layout points at.
    fn layout_text(&self, _class: u64, _collection: u64, _index: u32) -> Option<String> {
        None
    }
    /// The string element `index` of a `char*` attribute points at.
    fn text(&self, _class: u64, _collection: u64, _key: u64, _index: u32) -> Option<String> {
        None
    }
    /// `Attrib::Instance::Get(key).GetLength()`: 1 for a scalar, the element count of an array,
    /// 0 when missing.
    fn length(&self, _class: u64, _collection: u64, _key: u64) -> u32 {
        0
    }
    /// The class's leaf collections in key order (the order `Sk8::Score::TrickDatabase` lists
    /// `scoring_trick`'s in).
    fn leaf_collections(&self, _class: u64) -> Vec<u64> {
        Vec::new()
    }
}

/// One attribute collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Collection {
    pub class: u64,
    pub key: u64,
}

impl Collection {
    #[must_use]
    pub const fn new(class: u64, key: u64) -> Self {
        Self { class, key }
    }
}

/// `Attrib::StringToKey` class keys of the `aud_*` classes.
pub mod class {
    pub const AUD_GENERAL: u64 = 0xC183_1BDB_6CB1_B1EA;
    pub const AUD_CONTACTS: u64 = 0xC269_49FC_B638_A2CA;
    pub const AUD_COLLISIONS: u64 = 0x6EBA_5BCD_3E38_A98A;
    pub const AUD_SKATEBOARD: u64 = 0x7AB2_3C11_B6AD_A2DE;
    pub const AUD_RAILS: u64 = 0x0498_61E8_F9A8_D16B;
    pub const AUD_SPLICER: u64 = 0x923C_CB46_EF5B_F5BA;
    pub const AUD_SPEECH: u64 = 0xB29C_3B2C_13D9_6482;
    pub const AUD_CRACKS: u64 = 0x7242_F328_31ED_3332;
    pub const AUD_SENSEOFSPEED: u64 = 0x6E87_8344_774A_7999;
}

/// Vault access with the native `DefaultDataArea` fallbacks.
pub struct Vault {
    source: Option<Box<dyn AttribSource>>,
}

impl Vault {
    #[must_use]
    pub fn new(source: Option<Box<dyn AttribSource>>) -> Self {
        Self { source }
    }

    fn word(&self, collection: Collection, key: u64, index: u32) -> u32 {
        self.source
            .as_ref()
            .and_then(|source| source.attribute(collection.class, collection.key, key, index))
            .filter(|bytes| bytes.len() >= 4)
            .map_or(0, |bytes| u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// A 4-byte attribute read as `f32` (0.0 from `DefaultDataArea`).
    #[must_use]
    pub fn f32(&self, collection: Collection, key: u64) -> f32 {
        f32::from_bits(self.word(collection, key, 0))
    }

    #[must_use]
    pub fn f32_at(&self, collection: Collection, key: u64, index: u32) -> f32 {
        f32::from_bits(self.word(collection, key, index))
    }

    #[must_use]
    pub fn i32(&self, collection: Collection, key: u64) -> i32 {
        self.word(collection, key, 0) as i32
    }

    #[must_use]
    pub fn i32_at(&self, collection: Collection, key: u64, index: u32) -> i32 {
        self.word(collection, key, index) as i32
    }

    /// A `bool` attribute (first byte; false from `DefaultDataArea`).
    #[must_use]
    pub fn bool(&self, collection: Collection, key: u64) -> bool {
        self.source
            .as_ref()
            .and_then(|source| source.attribute(collection.class, collection.key, key, 0))
            .and_then(|bytes| bytes.first().copied())
            .is_some_and(|byte| byte != 0)
    }

    /// `Attrib::Instance::Lookup` of element `index`: its bytes, `None` when the collection or the
    /// attribute is missing.
    #[must_use]
    pub fn find(&self, collection: Collection, key: u64, index: u32) -> Option<Vec<u8>> {
        self.source.as_ref()?.attribute(collection.class, collection.key, key, index)
    }

    /// A found attribute's element 0 as `f32` (0.0 when shorter than a word).
    #[must_use]
    pub fn find_f32(&self, collection: Collection, key: u64) -> Option<f32> {
        let bytes = self.find(collection, key, 0)?;
        Some(bytes.get(..4).map_or(0.0, |word| f32::from_be_bytes([word[0], word[1], word[2], word[3]])))
    }

    /// Element `index` of a structure array (zeros from `DefaultDataArea`).
    #[must_use]
    pub fn bytes(&self, collection: Collection, key: u64, index: u32, size: usize) -> Vec<u8> {
        let mut out = self.source.as_ref().and_then(|source| source.attribute(collection.class, collection.key, key, index)).unwrap_or_default();
        out.resize(size, 0);
        out
    }

    /// Word `index` of a structure array element.
    #[must_use]
    pub fn struct_word(&self, collection: Collection, key: u64, index: u32, word: usize) -> u32 {
        let bytes = self.bytes(collection, key, index, 4 * (word + 1));
        u32::from_be_bytes([bytes[4 * word], bytes[4 * word + 1], bytes[4 * word + 2], bytes[4 * word + 3]])
    }

    #[must_use]
    pub fn layout_word(&self, collection: Collection, index: u32) -> u32 {
        self.source.as_ref().map_or(0, |source| source.layout_word(collection.class, collection.key, index))
    }

    #[must_use]
    pub fn layout_f32(&self, collection: Collection, index: u32) -> f32 {
        f32::from_bits(self.layout_word(collection, index))
    }

    /// A `char*` layout word's string (`None` for a missing collection or a null pointer).
    #[must_use]
    pub fn layout_text(&self, collection: Collection, index: u32) -> Option<String> {
        self.source.as_ref().and_then(|source| source.layout_text(collection.class, collection.key, index))
    }

    /// A `char*` attribute's string (`None` for a missing attribute or a null pointer).
    #[must_use]
    pub fn text(&self, collection: Collection, key: u64) -> Option<String> {
        self.source.as_ref().and_then(|source| source.text(collection.class, collection.key, key, 0))
    }

    #[must_use]
    pub fn has_collection(&self, collection: Collection) -> bool {
        self.source.as_ref().is_some_and(|source| source.has_collection(collection.class, collection.key))
    }

    /// `Attrib::Attribute::GetLength` of attribute `key`.
    #[must_use]
    pub fn length(&self, collection: Collection, key: u64) -> u32 {
        self.source.as_ref().map_or(0, |source| source.length(collection.class, collection.key, key))
    }

    /// The class's leaf collections in key order.
    #[must_use]
    pub fn leaf_collections(&self, class: u64) -> Vec<u64> {
        self.source.as_ref().map_or_else(Vec::new, |source| source.leaf_collections(class))
    }
}

impl Default for Vault {
    fn default() -> Self {
        Self::new(None)
    }
}
