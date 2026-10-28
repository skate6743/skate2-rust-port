//! The teleporter challenges (`eChallengeTypes` 45): `HUD2.SetupTeleportHUD` and
//! `ShowTeleportHUD`, a sign-up HUD group of `eSignUpType_Teleport` whose D-pad up sends
//! `cMsgTeleporterSignUp`; the `FrontEnd` popups that confirm the teleport; and
//! `Teleport.ToWorldFileLocation`.

use super::{
    attrib_lua::{AttribArray, ObjectKind, Pointee, activity_of, attrib_location},
    binding::{get_bool, get_int, get_int_or, get_string_or, int},
    keys::{class, hash_value32},
    lua::{Lua, LuaResult, NativeCall, NativeFn, Value},
    signup_hud,
    system::{Event, Field, Message, Output, World},
};

type R = LuaResult<Vec<Value>>;

/// `fe_locations` `World`: the world file name `CommonBackEnd::GetWorldFilename` (`0x82562480`)
/// formats.
const FE_LOCATION_WORLD: u64 = 0x116c_a244_49c4_ce44;
/// The attribute type `ShowGenericAttributePopup` takes its options from
/// (`0x4657a686d9f1a95e`).
const TEXT_TYPE: &str = "EA::Reflection::Text";
/// `ShowGenericAttributePopup`'s option count when the attribute holds fewer.
const GENERIC_POPUP_OPTIONS: usize = 4;

/// A `SignUpData` of `eSignUpType_Teleport` (`SetupTeleportHUD`): its group, named after the
/// activity, and the one challenge `AddChallenge` gave it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TeleportHud {
    pub group: String,
    pub activity: usize,
    /// `IHUDData::mDirty`.
    pub dirty: bool,
}

/// The teleporters' sign-up data and the ids of the popups the scripts open.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Teleporters {
    pub huds: Vec<TeleportHud>,
    /// `StateManagerFrontEndBase::OpenPopup` (`0x82d6aaa0`) numbers every popup from the static
    /// `test` (`0x830a7c50`, 0 at boot); the port numbers the popups the scripts open.
    next_popup_id: i32,
}

impl Teleporters {
    /// The teleporter data of a HUD group.
    #[must_use]
    pub fn hud(&self, group: &str) -> Option<&TeleportHud> {
        self.huds.iter().find(|hud| hud.group == group)
    }

    pub fn hud_mut(&mut self, group: &str) -> Option<&mut TeleportHud> {
        self.huds.iter_mut().find(|hud| hud.group == group)
    }
}

/// `HUD2.SetupTeleportHUD(activity)` (`0x8289c148`): `HUDManager2::CreateGroup` named after the
/// activity with the `signup` component, whose `SignUpData` holds the activity alone
/// (`ClearChallenges`, `AddChallenge`, `MakeDirty`) as `eSignUpType_Teleport`; the HUD tracker
/// owns the group. Returns the group.
fn setup_teleport_hud(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let activity = activity_of(lua, call.arg(0)).ok_or_else(|| lua.error("HUD2.SetupTeleportHUD: expected an activity"))?;
    let name = world.activities.activities[activity].name.clone();
    world.hud.create_group(&name, &[]);
    world.hud.add_component(&name, signup_hud::COMPONENT);
    let hud = &world.hud;
    world.teleporters.huds.retain(|teleporter| teleporter.group != name && hud.group_exists(&teleporter.group));
    world.teleporters.huds.push(TeleportHud { group: name.clone(), activity, dirty: true });
    let hash = hash_value32(&name);
    world.hud_owners.push((activity, hash));
    let handle = world.pointers.handle(Pointee::Object { kind: ObjectKind::HudGroup, id: hash });
    Ok(vec![Value::LightUserData(handle)])
}

/// `HUD2.ShowTeleportHUD(group, show)` (`0x82896910`): the group made active and its `signup`
/// instance shown, or the instance hidden and the group made inactive.
fn show_teleport_hud(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let Some(hash) = world.pointers.object(call.arg(0), ObjectKind::HudGroup) else { return Ok(Vec::new()) };
    let Some(name) = world.hud.groups.iter().find(|group| group.hash == hash).map(|group| group.name.clone()) else {
        return Ok(Vec::new());
    };
    if get_bool(call, 1) {
        world.hud.add_group_to_active(&name);
        world.hud.show(&name, signup_hud::COMPONENT, true);
    } else {
        world.hud.show(&name, signup_hud::COMPONENT, false);
        world.hud.remove_group_from_active(&name);
    }
    Ok(Vec::new())
}

/// `SignUpHUD::UpdateDPadUp` (`0x826bd450`) of a teleporter's data: `cMsgTeleporterSignUp` with
/// the challenge's unique ID.
pub fn sign_up(world: &mut World, group: &str) {
    let Some(activity) = world.teleporters.hud(group).map(|hud| hud.activity) else { return };
    let key = world.activities.activities[activity].key;
    world.send(Event::Challenge(Message::new("cMsgTeleporterSignUp", vec![("ActivityKey", Field::Key(key))])));
}

/// `GenericPopupCallback` (`0x82897910`): `cMsgHUDPopupOption` with the chosen option.
#[must_use]
pub fn popup_option(option: usize) -> Event {
    let index = f32::from(u16::try_from(option).unwrap_or(u16::MAX));
    Event::Challenge(Message::new("cMsgHUDPopupOption", vec![("OptionIndex", Field::Number(index))]))
}

/// `FrontEndManager::OpenPopup` (`0x826147b8`): the popup and its id.
fn open_popup(world: &mut World, title: String, description: String, options: Option<Vec<String>>) -> i32 {
    let id = world.teleporters.next_popup_id;
    world.teleporters.next_popup_id += 1;
    world.output(Output::ShowPopup { id, title, description, options });
    id
}

/// `FrontEnd.ShowYesNoPopup(title, description)` (`0x82897840`): a popup with the options
/// `ID_COMMON_NO` and `ID_COMMON_YES` (default No) that `OpenProgressionPopupCallback`
/// (`0x82897720`) answers with `cMsgHUDNo` or `cMsgHUDYes`. Returns its id.
fn show_yes_no_popup(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let title = get_string_or(call, 0, "");
    let description = get_string_or(call, 1, "");
    let id = open_popup(world, title, description, None);
    Ok(vec![int(id)])
}

/// `FrontEnd.ShowGenericAttributePopup(title, description, options)` (`0x828979c0`): a popup
/// whose options are the texts of an `EA::Reflection::Text` attribute, at least four (the missing
/// ones empty), default the first, that `GenericPopupCallback` (`0x82897910`) answers with
/// `cMsgHUDPopupOption`. Returns its id. Any other argument sends `cMsgHUDPopupOption` with
/// option 0 at once and returns nil.
fn show_generic_attribute_popup(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let title = get_string_or(call, 0, "");
    let description = get_string_or(call, 1, "");
    let options = lua
        .userdata_payload::<AttribArray>(call.arg(2))
        .copied()
        .filter(|array| {
            world.db.definition(array.class, array.key).and_then(|definition| world.db.type_name(definition.type_id)) == Some(TEXT_TYPE)
        })
        .map(|array| {
            let attribute = world.db.attribute(array.class, array.collection, array.key);
            let count = attribute.as_ref().map_or(0, |attribute| attribute.len()).max(GENERIC_POPUP_OPTIONS);
            (0..count).map(|i| attribute.as_ref().and_then(|attribute| attribute.text(i)).unwrap_or_default().to_owned()).collect::<Vec<_>>()
        });
    let Some(options) = options else {
        world.send(popup_option(0));
        return Ok(vec![Value::Nil]);
    };
    let id = open_popup(world, title, description, Some(options));
    Ok(vec![int(id)])
}

/// `FrontEnd.ClosePopup(id = -1)` (`0x82897be0`): `FrontEndManager::ClosePopup` (`0x82614758`)
/// of a popup other than -1, without its callback.
fn close_popup(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let id = get_int_or(call, 0, -1);
    if id != -1 {
        world.output(Output::ClosePopup { id });
    }
    Ok(Vec::new())
}

/// `Teleport.ToWorldFileLocation(player, worldLocation)` (`0x82da85f0`): the
/// `tWorldLocationID`'s world (`fe_locations`, +8) and location (+24) to
/// `cChallengeTeleportManager::TeleportPlayer` (`0x82da6b80`), which for the local player alone
/// runs `cTeleportManager::TeleportLocalToWorldLocation` (`0x828a91f8`): the location's spawn
/// point for the local player, off the board. `IsMainWorldLocation` (`0x828a95b8`) is true for
/// the invalid key and a missing collection, else when the world's file is the loaded one.
fn to_world_file_location(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let player = get_int(call, 0);
    let Some(at) = attrib_location(&world.pointers, call.arg(1)) else { return Ok(Vec::new()) };
    if player != world.local_player {
        return Ok(Vec::new());
    }
    let key = world.db.bytes(at.offset_by(8), 8).and_then(|bytes| bytes.try_into().ok()).map_or(u64::MAX, u64::from_be_bytes);
    let location = world.db.text_at(at.offset_by(24)).unwrap_or_default().to_owned();
    let world_file = (key != u64::MAX && world.db.has_collection(class::FE_LOCATIONS, key)).then(|| {
        world.db.attribute(class::FE_LOCATIONS, key, FE_LOCATION_WORLD).and_then(|attribute| attribute.text(0)).unwrap_or_default().to_owned()
    });
    world.output(Output::TeleportLocalToWorldLocation { world_location: key, world_file, location });
    Ok(Vec::new())
}

pub(crate) fn natives() -> Vec<(&'static str, &'static str, NativeFn<World>)> {
    vec![
        ("HUD2", "SetupTeleportHUD", setup_teleport_hud),
        ("HUD2", "ShowTeleportHUD", show_teleport_hud),
        ("FrontEnd", "ShowYesNoPopup", show_yes_no_popup),
        ("FrontEnd", "ShowGenericAttributePopup", show_generic_attribute_popup),
        ("FrontEnd", "ClosePopup", close_popup),
        ("Teleport", "ToWorldFileLocation", to_world_file_location),
    ]
}
