//! `ChallengeRuntime::cLuaAttributes` and the `cLuaStateManager` metatables:
//! how Vault data reaches challenge scripts (`data.Name`, `data.Objectives[1]`,
//! `data.GlobalData.Location`).

use super::{
    attrib_db::{AttribDb, Location},
    keys::hash64,
    lua::{Lua, LuaResult, NativeCall, TableId, Value},
};
use std::collections::HashMap;

pub const DATA_PROVIDER_METATABLE: &str = "MultiDataProviderMetaTable";
pub const ARRAY_METATABLE: &str = "LuaAttribArrayInfoMetaTable";
pub const REF_SPEC_METATABLE: &str = "RefSpecMetaTable";

/// `MultiDataProviderMetaTable` userdata: `[SelfContainerObject,
/// &DataProviders]`. Natives read the activity from the first word
/// (`Lua::Binding::GetUserData<cActivity>`).
#[derive(Clone, Debug)]
pub struct DataProviders {
    pub activity: usize,
    /// `(class, collection)` instances in `AddDataContainer` order.
    pub providers: Vec<(u64, u64)>,
}

/// `LuaAttribArrayInfoMetaTable` userdata: a copied `Attrib::Attribute`.
#[derive(Clone, Copy, Debug)]
pub struct AttribArray {
    pub class: u64,
    pub collection: u64,
    pub key: u64,
}

/// `RefSpecMetaTable` userdata (`Attrib::RefSpec`, 24 bytes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RefSpec {
    pub class: u64,
    pub collection: u64,
}

/// What a `lightuserdata` points at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Pointee {
    /// Vault data pushed by `PushAttribValue<struct>`: the value's address.
    Attrib(Location),
    /// `Attrib::DefaultDataArea`: the zeroed area a missing value reads.
    DefaultData,
    /// A native object owned by a challenge subsystem.
    Object { kind: ObjectKind, id: u32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ObjectKind {
    ObjectiveGroup,
    PresentationGroup,
    VisualIndicatorGroup,
    HighlightGroup,
    /// A `MapIcon` of `MapBackEndManager::AddChallengeObjectIcon`.
    MapIcon,
    /// A `HUDGroup` of `HUD2.SetupTeleportHUD`, by its name hash.
    HudGroup,
    /// A `cActivity` a message carries (`cMsgEnterOTSChallengeBoundary`, `cMsgExitDiscoveryVolume`),
    /// by its index.
    Activity,
}

/// `lightuserdata` values: pointer identity is preserved, so the same Vault
/// value or object always yields the same handle.
#[derive(Default)]
pub struct Pointers {
    handles: HashMap<Pointee, usize>,
    targets: Vec<Pointee>,
}

impl Pointers {
    pub fn handle(&mut self, pointee: Pointee) -> usize {
        if let Some(&handle) = self.handles.get(&pointee) {
            return handle;
        }
        self.targets.push(pointee);
        let handle = self.targets.len();
        self.handles.insert(pointee, handle);
        handle
    }

    #[must_use]
    pub fn get(&self, handle: usize) -> Option<Pointee> {
        self.targets.get(handle.checked_sub(1)?).copied()
    }

    #[must_use]
    pub fn object(&self, value: &Value, kind: ObjectKind) -> Option<u32> {
        let Value::LightUserData(handle) = value else { return None };
        match self.get(*handle)? {
            Pointee::Object { kind: k, id } if k == kind => Some(id),
            _ => None,
        }
    }
}

/// Host access needed by the metatable natives.
pub trait AttribHost {
    fn attrib_db(&self) -> &AttribDb;
    fn pointers(&mut self) -> &mut Pointers;
    /// `GetFieldProviderFromDataProviders` input for an activity.
    fn data_providers(&self, activity: usize) -> Vec<(u64, u64)>;
}

/// `LuaBindUtil::LoadMetatable` (`0x82aab490`) followed by
/// `cLuaStateManager::BindMetaTableAccessors` (`0x82aadad8`).
pub fn bind_metatables<H: AttribHost>(lua: &mut Lua<H>) {
    let registry = lua.registry();
    let invalid = lua.create_native("InvalidMetaTableOperation", invalid_operation::<H>, Vec::new());
    let tables: [(&str, NativeFnPtr<H>, bool); 3] = [
        (DATA_PROVIDER_METATABLE, data_provider_index::<H>, true),
        (ARRAY_METATABLE, array_index::<H>, false),
        (REF_SPEC_METATABLE, ref_spec_index::<H>, true),
    ];
    for (name, index, has_len) in tables {
        let table = lua.create_table();
        lua.table_mut(table).set_str("__index", Value::Table(table));
        lua.table_mut(table).set_str("metatablename", Value::string(name));
        lua.table_mut(registry).set_str(name, Value::Table(table));
        let index = lua.create_native("__index", index, Vec::new());
        lua.table_mut(table).set_str("__index", index);
        if has_len {
            lua.table_mut(table).set_str("__len", invalid.clone());
        }
        lua.table_mut(table).set_str("__newindex", invalid.clone());
        lua.table_mut(table).set_str("__eq", invalid.clone());
    }
}

type NativeFnPtr<H> = fn(&mut Lua<H>, &mut H, &NativeCall) -> LuaResult<Vec<Value>>;

fn metatable<H>(lua: &Lua<H>, name: &str) -> Option<TableId> {
    match lua.table(lua.registry()).get_str(name) {
        Value::Table(t) => Some(t),
        _ => None,
    }
}

/// `InvalidMetaTableOperation` (`0x82aab3e0`): returns nothing.
fn invalid_operation<H>(_: &mut Lua<H>, _: &mut H, _: &NativeCall) -> LuaResult<Vec<Value>> {
    Ok(Vec::new())
}

/// Attaches `MultiDataProviderMetaTable` data for an activity
/// (`cStateGraph::BuildActivityTables` `0x82ce4598`).
pub fn push_data_providers<H: AttribHost>(lua: &mut Lua<H>, host: &H, activity: usize) -> Value {
    let providers = DataProviders { activity, providers: host.data_providers(activity) };
    let mt = metatable(lua, DATA_PROVIDER_METATABLE);
    Value::UserData(lua.create_userdata(Box::new(providers), mt))
}

/// The activity a `MultiDataProviderMetaTable` argument belongs to.
#[must_use]
pub fn activity_of<H>(lua: &Lua<H>, value: &Value) -> Option<usize> {
    lua.userdata_payload::<DataProviders>(value).map(|d| d.activity)
}

fn field_name<H>(lua: &Lua<H>, call: &NativeCall) -> Vec<u8> {
    match call.arg(1) {
        Value::String(s) => s.as_bytes().to_vec(),
        Value::Number(n) => super::lua::number::number_to_string(*n).into_bytes(),
        _ => {
            let _ = lua;
            Vec::new()
        }
    }
}

/// `Attrib::StringToKey` on the field name.
fn field_key(name: &[u8]) -> u64 {
    hash64(&String::from_utf8_lossy(name))
}

/// `MultiDataProviderMetaTable__index` (`0x82aad808`).
fn data_provider_index<H: AttribHost>(lua: &mut Lua<H>, host: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let Some(providers) = lua.userdata_payload::<DataProviders>(call.arg(0)).map(|d| d.providers.clone()) else {
        return Err(lua.error("bad argument #1 (cLuaStateManager::GetMultiDataProviderMetaTableName expected)"));
    };
    let name = field_name(lua, call);
    let key = field_key(&name);
    // GetFieldProviderFromDataProviders: the first provider whose class
    // defines the field.
    let provider = providers.iter().copied().find(|&(class, _)| host.attrib_db().definition(class, key).is_some());
    Ok(vec![match provider {
        Some((class, collection)) => push_field(lua, host, class, collection, key, call.arg(1)),
        None => Value::Nil,
    }])
}

/// `RefSpecMetaTable__index` (`0x82aad8d0`).
fn ref_spec_index<H: AttribHost>(lua: &mut Lua<H>, host: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let Some(spec) = lua.userdata_payload::<RefSpec>(call.arg(0)).copied() else {
        return Err(lua.error("bad argument #1 (cLuaStateManager::GetRefSpecMetaTableName expected)"));
    };
    let name = field_name(lua, call);
    let key = field_key(&name);
    if !host.attrib_db().has_collection(spec.class, spec.collection) {
        return Ok(vec![Value::Nil]);
    }
    Ok(vec![push_field(lua, host, spec.class, spec.collection, key, call.arg(1))])
}

/// `LuaAttribArrayInfoMetaTable__index` (`0x82aad350`): `[i]` (1-based) and
/// `.Count`.
fn array_index<H: AttribHost>(lua: &mut Lua<H>, host: &mut H, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let Some(array) = lua.userdata_payload::<AttribArray>(call.arg(0)).copied() else {
        return Err(lua.error("bad argument #1 (cLuaStateManager::GetLuaAttribArrayInfoMetaTableName() expected)"));
    };
    match call.arg(1) {
        Value::Number(n) => {
            #[expect(clippy::cast_possible_truncation, reason = "(int)lua_tonumber")]
            let index = (*n as i32) - 1;
            let Ok(index) = usize::try_from(index) else { return Ok(vec![Value::Nil]) };
            Ok(vec![push_value(lua, host, array.class, array.collection, array.key, index)])
        }
        Value::String(s) if s.as_bytes() == b"Count" => {
            let len = host.attrib_db().attribute(array.class, array.collection, array.key).map_or(0, |a| a.len());
            #[expect(clippy::cast_precision_loss, reason = "lua_Number")]
            Ok(vec![Value::Number(len as f32)])
        }
        // Any other key: the function returns 1 with nothing pushed, so the
        // key itself comes back.
        other => Ok(vec![other.clone()]),
    }
}

/// `cLuaAttributes::PushAttributeValue(provider, fieldKey)` (`0x82aad4d8`):
/// arrays become `LuaAttribArrayInfoMetaTable` userdata, other values are
/// pushed as element 0. `key_arg` is what a type with no accessor leaves on
/// top of the stack (the `__index` key).
fn push_field<H: AttribHost>(lua: &mut Lua<H>, host: &mut H, class: u64, collection: u64, key: u64, key_arg: &Value) -> Value {
    let db = host.attrib_db();
    let Some(definition) = db.definition(class, key) else { return Value::Nil };
    if definition.flags & 1 != 0 {
        let mt = metatable(lua, ARRAY_METATABLE);
        return Value::UserData(lua.create_userdata(Box::new(AttribArray { class, collection, key }), mt));
    }
    let type_name = db.type_name(definition.type_id).unwrap_or_default().to_owned();
    if !has_accessor(&type_name) {
        return key_arg.clone();
    }
    push_value(lua, host, class, collection, key, 0)
}

/// Types with a `PushAttrib` accessor in `BuildAttributeTypeTable`
/// (`0x82aae658`) or `Lua::Manager::GetLuaAccessorBindingTable`
/// (`0x82892078`). Every other indexed type is registered with a null
/// accessor.
fn has_accessor(type_name: &str) -> bool {
    BASE_TYPES.contains(&type_name)
        || INT_ENUMS.contains(&type_name)
        || REF_SPEC_STRUCTS.contains(&type_name)
        || POINTER_STRUCTS.contains(&type_name)
        || type_name == TRIGGER_VOLUME_INSTANCE
}

const BASE_TYPES: [&str; 14] = [
    "EA::Reflection::Float",
    "EA::Reflection::Double",
    "EA::Reflection::Int64",
    "EA::Reflection::Int32",
    "EA::Reflection::Int16",
    "EA::Reflection::Int8",
    "EA::Reflection::UInt64",
    "EA::Reflection::UInt32",
    "EA::Reflection::UInt16",
    "EA::Reflection::UInt8",
    "EA::Reflection::Char",
    "EA::Reflection::Bool",
    "EA::Reflection::Text",
    "Attrib::RefSpec",
];

/// Enums bound to `PushAttribValue<eChallengeTypes>` (an `int`).
const INT_ENUMS: [&str; 20] = [
    "ChallengePrereq::eChallengeLevel",
    "Sk8::Challenge::eFilmChallengeType",
    "Sk8::Challenge::eSk8ChallengeIndicators",
    "Sk8::Challenge::eContestType",
    "Sk8::Challenge::eFreeskateType",
    "Sk8::Challenge::eFreeskateAccumulationType",
    "Sk8::Challenge::eFreeskateSurvivalType",
    "Sk8::Score::ETrickType",
    "Sk8::Challenge::eSkateChallengeType",
    "Sk8::Scene::eSceneType",
    "Sk8::Challenge::eChallengeTypes",
    "Sk8::Challenge::ePhotoChallengePhotoType",
    "Sk8::Scene::eNISChallengeNotification",
    "Sk8::BE::ObjectiveType",
    "WorldPainter::eLayerID",
    "Sk8::Challenge::eRaceType",
    "Sk8::Challenge::eRaceGateAudio",
    "Sk8::Challenge::eRaceMusicProgress",
    "Sk8::Service::eServiceType",
    "Sk8::Challenge::eRunType",
];

/// Types whose accessor is the RefSpec one (`AttachRefSpecLookup`,
/// identical-code-folded as `PushAttribValue<tRaceHeatDefinition>`).
const REF_SPEC_STRUCTS: [&str; 9] = [
    "Attrib::RefSpec",
    "Sk8::Challenge::tContestRoundDefinition",
    "Sk8::Challenge::tOneUpObjectSelectorDefinition",
    "Sk8::Challenge::tRaceHeatDefinition",
    "Sk8::Challenge::tRaceLegDefinition",
    "Sk8::Challenge::tRaceBranchDefinition",
    "Sk8::Challenge::tRaceGroupDefinition",
    "Sk8::Challenge::tRaceGateDefinition",
    "Sk8::Challenge::tRaceMusicPaceDefinition",
];

/// Types whose accessor pushes `lightuserdata` of the value.
const POINTER_STRUCTS: [&str; 27] = [
    "Sk8::Challenge::tCameraPanInfo",
    "Sk8::Challenge::tLocationID",
    "Sk8::Challenge::tWorldLocationID",
    "Sk8::Audio::eSk8Spch_Announcer_ID",
    "Sk8::Challenge::tTrickSequenceInfo",
    "Sk8::Challenge::tTrickTypeBonus",
    "Sk8::PointNegGraphData4",
    "Sk8::PointNegGraphData8",
    "Sk8::PointNegGraphData16",
    "Sk8::Challenge::tFollowTheLeaderWarningInfo",
    "Sk8::Challenge::tFilmMeasurementObjective",
    "Sk8::Challenge::tRoundPaths",
    "Sk8::Challenge::tSpeechInfo",
    "Sk8::Challenge::tFollowMeSpeechInfo",
    "Sk8::Challenge::tOwnSpotSpeechInfo",
    "Sk8::Challenge::tContextSpeechInfo",
    "Sk8::Challenge::tSpectatorSpawnInfo",
    "Sk8::Challenge::tContestCalloutDefinition",
    "Sk8::Challenge::tVisualEditorData",
    "Sk8::Challenge::tSplineBankObject",
    "Sk8::Service::tServiceInstanceWorldPainterSelector",
    "Sk8::Challenge::tHighlightDefinition",
    "Sk8::tNISPlaybackDefinition",
    "Sk8::Challenge::cChallengePresentationGroup",
    "Sk8::Challenge::tChallengePresentationEvent",
    "Sk8::Challenge::tLargeCrowd",
    "Sk8::Challenge::tRequiredChallengeHull",
];

/// `PushAttribValue<tTriggerVolumeInstanceID>`: the 64-bit volume id.
const TRIGGER_VOLUME_INSTANCE: &str = "Sk8::Challenge::tTriggerVolumeInstanceID";

fn be_u32(b: &[u8]) -> u32 {
    u32::from_be_bytes([b[0], b[1], b[2], b[3]])
}

fn be_u64(b: &[u8]) -> u64 {
    u64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
}

/// `cLuaAttributes::PushAttributeValue(attribute, index)` (`0x82aad188`) and
/// the per-type `PushAttribValue<T>` accessors. A missing value reads
/// `Attrib::DefaultDataArea` (zeros).
#[expect(clippy::cast_precision_loss, reason = "lua_Number is float")]
#[expect(clippy::cast_possible_truncation, reason = "double to float")]
fn push_value<H: AttribHost>(lua: &mut Lua<H>, host: &mut H, class: u64, collection: u64, key: u64, index: usize) -> Value {
    let db = host.attrib_db();
    let Some(attribute) = db.attribute(class, collection, key) else { return Value::Nil };
    let type_name = attribute.type_name().unwrap_or_default().to_owned();
    let size = usize::from(attribute.definition.size);
    let location = attribute.data(index);
    let bytes: Vec<u8> = location.and_then(|l| db.bytes(l, size).map(<[u8]>::to_vec)).unwrap_or_else(|| vec![0; size.max(8)]);
    let text = if type_name == "EA::Reflection::Text" { location.and_then(|l| db.text_at(l)).map(str::to_owned) } else { None };
    let ref_spec = (bytes.len() >= 16).then(|| RefSpec { class: be_u64(&bytes[..8]), collection: be_u64(&bytes[8..16]) });
    match type_name.as_str() {
        "EA::Reflection::Float" => Value::Number(f32::from_bits(be_u32(&bytes))),
        "EA::Reflection::Double" => Value::Number(f64::from_bits(be_u64(&bytes)) as f32),
        "EA::Reflection::Int64" => Value::Number(be_u64(&bytes).cast_signed() as f32),
        "EA::Reflection::UInt64" => Value::Number(crate::skate3::id_float(be_u64(&bytes))),
        "EA::Reflection::Int16" => Value::Number(f32::from(i16::from_be_bytes([bytes[0], bytes[1]]))),
        "EA::Reflection::UInt16" => Value::Number(f32::from(u16::from_be_bytes([bytes[0], bytes[1]]))),
        "EA::Reflection::Int8" => Value::Number(f32::from(bytes[0].cast_signed())),
        "EA::Reflection::UInt8" => Value::Number(f32::from(bytes[0])),
        "EA::Reflection::UInt32" => Value::Number(be_u32(&bytes) as f32),
        "EA::Reflection::Char" => Value::bytes(&bytes[..1]),
        "EA::Reflection::Bool" => Value::Boolean(bytes[0] != 0),
        "EA::Reflection::Text" => text.map_or(Value::Nil, |t| Value::string(&t)),
        TRIGGER_VOLUME_INSTANCE => Value::Number(crate::skate3::id_float(be_u64(&bytes[8..16]))),
        name if REF_SPEC_STRUCTS.contains(&name) => {
            let mt = metatable(lua, REF_SPEC_METATABLE);
            Value::UserData(lua.create_userdata(Box::new(ref_spec.unwrap_or(RefSpec { class: 0, collection: 0 })), mt))
        }
        name if name == "EA::Reflection::Int32" || INT_ENUMS.contains(&name) => Value::Number(be_u32(&bytes).cast_signed() as f32),
        name if POINTER_STRUCTS.contains(&name) => {
            let pointee = location.map_or(Pointee::DefaultData, Pointee::Attrib);
            Value::LightUserData(host.pointers().handle(pointee))
        }
        _ => Value::Nil,
    }
}

/// Reads a `lightuserdata` that points at Vault data back to its location.
#[must_use]
pub fn attrib_location(pointers: &Pointers, value: &Value) -> Option<Location> {
    let Value::LightUserData(handle) = value else { return None };
    match pointers.get(*handle)? {
        Pointee::Attrib(location) => Some(location),
        _ => None,
    }
}
