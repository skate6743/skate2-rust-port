//! `ChallengeRuntime` (`packages/sk8/challenge`): every activity's state
//! graph, its Lua message handlers, and the post office that routes
//! Challenge-port messages to the graphs subscribed to them. See
//! `docs/career/runtime.md`.

use super::{
    attrib_db::AttribDb,
    attrib_lua::push_data_providers,
    keys::{class, hash32, hash64},
    lua::{Lua, LuaResult, NativeCall, Value, stdlib::check_string},
    system::{Event, Field, Message, Output, World},
};
use std::collections::BTreeMap;

pub mod field {
    pub const NAME: u64 = 0x942a_b8ae_e4b4_14ed;
    /// `challenges`: the state graph's `challenge_states` RefSpecs.
    pub const STATES: u64 = 0x3cb7_ba2a_fc9b_de42;
    pub const GLOBAL_DATA: u64 = 0x2449_1bf0_0316_aa45;
    pub const LOCAL_DATA: u64 = 0xd7bb_2b16_0caf_8ed6;
    /// `challenge_states`: `challenge_messagehandlers` RefSpecs.
    pub const STATE_HANDLERS: u64 = 0x8103_94ec_efe7_e48b;
    /// `challenge_messagehandlers`: `Attrib::Blob` chunk and message kind.
    pub const HANDLER_BLOB: u64 = 0xc3cd_4b41_8327_4819;
    pub const HANDLER_KIND: u64 = 0x6931_0573_18d2_132e;
}

/// `cMessageHandler`: a compiled handler for one message kind.
#[derive(Clone, Copy, Debug)]
pub struct Handler {
    pub key: u64,
    pub kind: u32,
}

/// `cState`.
#[derive(Clone, Debug)]
pub struct State {
    pub key: u64,
    pub name: String,
    pub handlers: Vec<Handler>,
}

/// `cStateGraph` (`0x778` bytes).
#[derive(Debug, Default)]
pub struct StateGraph {
    pub states: Vec<State>,
    pub global: Option<usize>,
    pub current: Option<usize>,
    registered: Option<usize>,
    pub running: bool,
    vars_in_vm: bool,
}

/// `cLuaPostOffice`: message kind → subscribed graphs in registration order.
#[derive(Debug, Default)]
pub struct PostOffice {
    subscribers: BTreeMap<u32, Vec<usize>>,
}

impl PostOffice {
    /// `RegisterHandler` (`0x82ce5c68`).
    fn register(&mut self, kind: u32, graph: usize) {
        let list = self.subscribers.entry(kind).or_default();
        if !list.contains(&graph) {
            list.push(graph);
        }
    }
    fn unregister(&mut self, kind: u32, graph: usize) {
        if let Some(list) = self.subscribers.get_mut(&kind) {
            list.retain(|&g| g != graph);
        }
    }
    #[must_use]
    pub fn subscribers(&self, kind: u32) -> &[usize] {
        self.subscribers.get(&kind).map_or(&[], Vec::as_slice)
    }
}

fn text(db: &AttribDb, class: u64, collection: u64, key: u64) -> Option<String> {
    db.attribute(class, collection, key)?.text(0).map(str::to_owned)
}

fn refs(db: &AttribDb, class: u64, collection: u64, key: u64) -> Vec<(u64, u64)> {
    db.attribute(class, collection, key)
        .map(|a| (0..a.len()).filter_map(|i| a.ref_spec(i)).collect())
        .unwrap_or_default()
}

impl StateGraph {
    /// `GatherStatesAndHandlers` (`0x82ce5d10`) for a `challenges` collection.
    #[must_use]
    pub fn gather(db: &AttribDb, activity: u64) -> Self {
        let mut graph = Self::default();
        let global_hash = hash32("global");
        for (_, state_key) in refs(db, class::CHALLENGES, activity, field::STATES) {
            let name = text(db, class::CHALLENGE_STATES, state_key, field::NAME).unwrap_or_default();
            let handlers = refs(db, class::CHALLENGE_STATES, state_key, field::STATE_HANDLERS)
                .into_iter()
                .map(|(_, key)| Handler {
                    key,
                    kind: db
                        .attribute(class::CHALLENGE_MESSAGE_HANDLERS, key, field::HANDLER_KIND)
                        .and_then(|a| a.u32(0))
                        .unwrap_or(0),
                })
                .collect();
            let index = match graph.states.iter().position(|s| s.key == state_key) {
                Some(i) => {
                    graph.states[i].handlers = handlers;
                    i
                }
                None => {
                    graph.states.push(State { key: state_key, name, handlers });
                    graph.states.len() - 1
                }
            };
            if hash32(&graph.states[index].name) == global_hash {
                graph.global = Some(index);
            }
        }
        graph
    }

    /// `GetStateByName` (`0x82ce3f98`).
    #[must_use]
    pub fn state_by_hash(&self, hash: u32) -> Option<usize> {
        self.states.iter().position(|s| hash32(&s.name) == hash)
    }

    #[must_use]
    pub fn current_name(&self) -> Option<&str> {
        self.current.map(|i| self.states[i].name.as_str())
    }
}

/// `RegisterMessageHandlers`/`UnregisterMessageHandlers` for one state.
fn subscribe(world: &mut World, graph: usize, state: usize, register: bool) {
    let kinds: Vec<u32> = world.activities.graph(graph).states[state].handlers.iter().map(|h| h.kind).collect();
    for kind in kinds {
        if register {
            world.post_office.register(kind, graph);
        } else {
            world.post_office.unregister(kind, graph);
        }
    }
}

/// `UnregisterState` (`0x82ce6488`).
fn unregister_state(world: &mut World, graph: usize) {
    let g = world.activities.graph(graph);
    let (registered, global) = (g.registered, g.global);
    if let Some(state) = registered {
        subscribe(world, graph, state, false);
        if let Some(global) = global {
            subscribe(world, graph, global, false);
        }
        world.activities.graph_mut(graph).registered = None;
    }
}

/// `RegisterState` (`0x82ce66e8`).
fn register_state(world: &mut World, graph: usize, state: usize) {
    if world.activities.graph(graph).registered == Some(state) {
        return;
    }
    unregister_state(world, graph);
    subscribe(world, graph, state, true);
    if let Some(global) = world.activities.graph(graph).global {
        subscribe(world, graph, global, true);
    }
    world.activities.graph_mut(graph).registered = Some(state);
}

/// `cStateGraph::Run` (`0x82ce68a8`): resume the current state, or queue
/// `ChangeState("initial")`.
pub fn run(world: &mut World, graph: usize) {
    let g = world.activities.graph_mut(graph);
    if g.running || g.states.is_empty() {
        return;
    }
    g.running = true;
    if let Some(current) = g.current {
        register_state(world, graph, current);
    } else {
        world.send(Event::ChangeState { graph, state: hash32("initial") });
    }
}

/// `cStateGraph::Suspend` (`0x82ce64e0`).
pub fn suspend(world: &mut World, graph: usize) {
    world.activities.graph_mut(graph).running = false;
    unregister_state(world, graph);
}

/// `cStateGraph::Reset` (`0x82ce6568`): suspend, drop the activity's Lua
/// vars, and forget the current state.
pub fn reset(lua: &mut Lua<World>, world: &mut World, graph: usize) {
    suspend(world, graph);
    clear_vars(lua, world, graph);
    world.activities.graph_mut(graph).current = None;
}

/// `ClearActivityVars` (`0x82ce3788`): `registry[Name] = nil`.
fn clear_vars(lua: &mut Lua<World>, world: &mut World, graph: usize) {
    let name = world.activities.activities[graph].name.clone();
    let registry = lua.registry();
    lua.table_mut(registry).set_str(&name, Value::Nil);
    world.activities.graph_mut(graph).vars_in_vm = false;
}

/// `cEventChangeState` delivery (its destructor, `0x828b5a50`).
pub fn change_state_event(lua: &mut Lua<World>, world: &mut World, graph: usize, state_hash: u32) {
    if state_hash == 0 || graph >= world.activities.activities.len() {
        return;
    }
    let state = world.activities.graph(graph).state_by_hash(state_hash);
    enter_state(lua, world, graph, state);
}

/// `cStateGraph::EnterState` (`0x82ce67a8`).
pub fn enter_state(lua: &mut Lua<World>, world: &mut World, graph: usize, new_state: Option<usize>) {
    let g = world.activities.graph(graph);
    if !g.running || new_state == g.current {
        return;
    }
    if let Some(current) = g.current {
        let name = g.states[current].name.clone();
        let exit = Message::new("cMsgStateExit", vec![("StateName", Field::Text(name))]);
        handle_local_message(lua, world, graph, &exit);
    }
    unregister_state(world, graph);
    world.activities.graph_mut(graph).current = new_state;
    if let Some(state) = new_state {
        register_state(world, graph, state);
        let name = world.activities.graph(graph).states[state].name.clone();
        trace(world, graph, &format!("enter {name}"));
        let enter = Message::new("cMsgStateEnter", vec![("StateName", Field::Text(name))]);
        handle_local_message(lua, world, graph, &enter);
    }
}

fn trace(world: &World, graph: usize, what: &str) {
    if std::env::var_os("SKATE2_TRACE_CAREER").is_some() {
        match world.online.as_ref() {
            Some(online) => eprintln!("career graph {} @{}: {what}", world.activities.activities[graph].name, online.tick),
            None => eprintln!("career graph {}: {what}", world.activities.activities[graph].name),
        }
    }
}

/// `cLuaMessageDeliveryInfo`: the message table is built once per delivery
/// and shared by every graph and handler that receives it.
struct Delivery<'a> {
    message: &'a Message,
    table: Option<Value>,
    table_reference: Option<usize>,
}

impl Delivery<'_> {
    /// `BuildMessageTable` (`0x82ce36d0`).
    fn table(&mut self, lua: &mut Lua<World>) -> Value {
        if let Some(table) = &self.table {
            return table.clone();
        }
        let value = match &self.message.fields {
            None => Value::Nil,
            Some(fields) => {
                let t = lua.create_table();
                for (name, field) in fields {
                    let v = match field {
                        Field::Number(n) => Value::Number(*n),
                        Field::Bool(b) => Value::Boolean(*b),
                        Field::Text(s) => Value::string(s),
                        Field::Pointer(p) => Value::LightUserData(*p),
                        Field::Key(k) => Value::Number(crate::skate3::id_float(*k)),
                        Field::Nil => Value::Nil,
                    };
                    lua.table_mut(t).set_str(name, v);
                }
                Value::Table(t)
            }
        };
        // Native BuildMessageTable leaves this on the Lua stack until the
        // delivery destructor (0x82ce3670), including across subscriber graphs.
        // A Rust Value is only a heap handle: retain the table in the registry
        // while Attach/handler calls can collect garbage. Nil needs no root.
        if !value.is_nil() {
            self.table_reference = Some(lua.reference(value.clone()));
        }
        self.table = Some(value.clone());
        value
    }

    /// The native delivery destructor pops its message after all recipients.
    fn finish(self, lua: &mut Lua<World>) {
        if let Some(reference) = self.table_reference {
            lua.unreference(reference);
        }
    }
}

/// `cStateGraph::HandleLocalMessage` (`0x82ce4e38`): deliver to one graph
/// with the activity as closure data.
pub fn handle_local_message(lua: &mut Lua<World>, world: &mut World, graph: usize, message: &Message) {
    let mut delivery = Delivery { message, table: None, table_reference: None };
    world.closure = Some(graph);
    handle_message(lua, world, graph, &mut delivery);
    world.closure = None;
    delivery.finish(lua);
}

/// `cLuaPostOffice::RouteMessage` (`0x82ce5a98`).
pub fn route_message(lua: &mut Lua<World>, world: &mut World, message: &Message) {
    let graphs = world.post_office.subscribers(message.kind).to_vec();
    if graphs.is_empty() {
        return;
    }
    let mut delivery = Delivery { message, table: None, table_reference: None };
    for graph in graphs {
        world.closure = Some(graph);
        handle_message(lua, world, graph, &mut delivery);
    }
    world.closure = None;
    delivery.finish(lua);
}

/// `cStateGraph::HandleMessage` (`0x82ce4c78`): the current state's handlers
/// for the kind, then the global state's.
fn handle_message(lua: &mut Lua<World>, world: &mut World, graph: usize, delivery: &mut Delivery<'_>) {
    let g = world.activities.graph(graph);
    let kind = delivery.message.kind;
    let mut handlers = Vec::new();
    for state in [g.current, g.global].into_iter().flatten() {
        handlers.extend(g.states[state].handlers.iter().filter(|h| h.kind == kind).map(|h| h.key));
    }
    if handlers.is_empty() {
        return;
    }
    let message = delivery.table(lua);
    let (data, vars) = build_activity_tables(lua, world, graph);
    // HandleMessage (0x82ce4c78) keeps BOTH activity arguments below every
    // handler call, including the chunk executed by Attach (0x82ce38c8).
    // Rooting just the handler's arguments in Lua::call is too late for Attach
    // and would not keep them alive between the current/global-state handlers.
    let references = [lua.reference(data.clone()), lua.reference(vars.clone())];
    for handler in handlers {
        execute_scripted_handler(lua, world, graph, handler, &[data.clone(), message.clone(), vars.clone()]);
    }
    for reference in references {
        lua.unreference(reference);
    }
}

/// `BuildActivityTables` (`0x82ce4598`): the `data` userdata, the vars table
/// and the global `ChangeState` bound to this graph.
fn build_activity_tables(lua: &mut Lua<World>, world: &mut World, graph: usize) -> (Value, Value) {
    let data = push_data_providers(lua, world, graph);
    let vars = push_activity_vars(lua, world, graph);
    #[expect(clippy::cast_precision_loss, reason = "graph index")]
    let change_state = lua.create_native("ChangeStateFromScript", change_state_from_script, vec![Value::Number(graph as f32)]);
    lua.set_global("ChangeState", change_state);
    (data, vars)
}

/// `PushActivityVars` (`0x82ce4470`): `registry[Name]`, created with
/// `kActivityID` (the collection key as a float).
fn push_activity_vars(lua: &mut Lua<World>, world: &mut World, graph: usize) -> Value {
    let activity = &world.activities.activities[graph];
    let (name, key) = (activity.name.clone(), activity.key);
    let registry = lua.registry();
    let existing = lua.table(registry).get_str(&name);
    let vars = if existing.is_nil() {
        let t = lua.create_table();
        #[expect(clippy::cast_precision_loss, reason = "(float)_u64tod")]
        lua.table_mut(t).set_str("kActivityID", Value::Number(key as f64 as f32));
        lua.table_mut(registry).set_str(&name, Value::Table(t));
        Value::Table(t)
    } else {
        existing
    };
    world.activities.graph_mut(graph).vars_in_vm = true;
    vars
}

/// `cMessageHandler::ExecuteScriptedHandler` (`0x82ce3ed8`): attach the
/// handler chunk once per Lua state, then call `h_<key>(data, message, vars)`.
fn execute_scripted_handler(lua: &mut Lua<World>, world: &mut World, graph: usize, handler: u64, args: &[Value]) {
    if !world.attached.contains(&handler) {
        attach(lua, world, handler);
    }
    if spot_battle_join(lua, world, graph, handler, args) {
        return;
    }
    let name = if skate3_handler(lua, world, graph, handler) {
        format!("h_skate3_{handler:016X}")
    } else if fixed_handler(lua, world, handler) {
        format!("h_fix_{handler:016X}")
    } else {
        format!("h_{handler:016X}")
    };
    let function = lua.get_global(&name);
    if let Err(e) = lua.call(world, &function, args) {
        let activity = &world.activities.activities[graph];
        let state = activity.graph.current_name().unwrap_or("?").to_owned();
        let context = format!("{}/{state}/{}", activity.name, handler_name(&world.db, handler));
        eprintln!("career script error in {context}: {e}");
        world.output(Output::ScriptError { context, error: e.to_string() });
    }
}

/// PC port (user request): a Spot Battle takes in a player who joins it in play, by starting
/// again from its state between two turns (`docs/evidence/spot-battle-joining-2026-11-22.md`).
/// True where the handler's call was made, or left out, here.
///
/// While a player waits (`OnlineChallenge::carry_wanted`), the `countdown` `stateenter` that
/// would start the next turn starts none: the battle's state goes to `OnlineChallenge::carry`
/// and the battle holds in `countdown` (`parked`) until the server loads it again. A battle its
/// host has ended goes on to its end instead.
///
/// A battle that starts from a carried state (`OnlineChallenge::restore`) runs `setup` without
/// its flythrough and ends `reset` with that state in place (`script_fixes::JOIN_SOURCE`).
fn spot_battle_join(lua: &mut Lua<World>, world: &mut World, graph: usize, handler: u64, args: &[Value]) -> bool {
    use super::script_fixes::{JOIN_SOURCE, SPOT_BATTLE_COUNTDOWN_ENTER, SPOT_BATTLE_RESET_ENTER, SPOT_BATTLE_SETUP_ENTER};
    let Some(online) = world.online.as_ref() else { return false };
    let [data, message, vars] = args else { return false };
    match handler {
        SPOT_BATTLE_COUNTDOWN_ENTER if online.parked => true,
        SPOT_BATTLE_COUNTDOWN_ENTER if online.carry_wanted && !online.host_ended => {
            let Some(carry) = capture_spot_battle(lua, vars) else { return false };
            if let Some(online) = world.online.as_mut() {
                online.carry = Some(carry);
                online.carry_wanted = false;
                online.parked = true;
            }
            true
        }
        SPOT_BATTLE_SETUP_ENTER | SPOT_BATTLE_RESET_ENTER if online.restore.is_some() => {
            let chunk = hash64("PC port: spot_battle_join");
            if world.attached.insert(chunk) {
                let loaded = lua.load_source(JOIN_SOURCE.as_bytes(), "spot_battle_join").and_then(|main| lua.call(world, &main, &[]).map(|_| ()));
                if let Err(e) = loaded {
                    world.output(Output::ScriptError { context: "attach spot_battle_join".to_owned(), error: e.to_string() });
                }
            }
            let shipped = lua.get_global(&format!("h_{handler:016X}"));
            let called = if handler == SPOT_BATTLE_SETUP_ENTER {
                let setup = lua.get_global("pc_spot_battle_setup");
                lua.call(world, &setup, &[shipped, data.clone(), message.clone(), vars.clone()]).map(|_| ())
            } else {
                lua.call(world, &shipped, args).and_then(|_| {
                    let Some(carry) = world.online.as_mut().and_then(|online| online.restore.take()) else { return Ok(()) };
                    let table = carry_table(lua, &carry);
                    let reference = lua.reference(table.clone());
                    let restore = lua.get_global("pc_spot_battle_restore");
                    let restored = lua.call(world, &restore, &[data.clone(), vars.clone(), table]).map(|_| ());
                    lua.unreference(reference);
                    restored
                })
            };
            if let Err(e) = called {
                let context = format!("{}/{}", world.activities.activities[graph].name, handler_name(&world.db, handler));
                eprintln!("career script error in {context}: {e}");
                world.output(Output::ScriptError { context, error: e.to_string() });
            }
            true
        }
        _ => false,
    }
}

/// The battle's state as its script's vars hold it at a turn's end: the players still in it, in
/// turn order and by rank. `None` where the vars are not a battle's between two turns.
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "the script's numbers are small whole ones")]
fn capture_spot_battle(lua: &Lua<World>, vars: &Value) -> Option<super::players::SpotBattleCarry> {
    let Value::Table(vars) = vars else { return None };
    let vars = lua.table(*vars);
    let number = |value: Value| if let Value::Number(n) = value { Some(n) } else { None };
    let set = |value: Value| matches!(value, Value::Boolean(true));
    if set(vars.get_str("LockOut")) {
        return None;
    }
    let count = number(vars.get_str("NumPlayers"))? as usize;
    let round = number(vars.get_str("CurrentRound"))? as i32;
    let index = number(vars.get_str("CurrentPlayerIndex"))? as usize;
    let (Value::Table(data), Value::Table(order), Value::Table(ranks)) = (vars.get_str("PlayerData"), vars.get_str("PlayerOrderMap"), vars.get_str("PlayerRankMap")) else {
        return None;
    };
    // A player's score, bonus objective and whether it left.
    let player = |id: usize| -> Option<(i32, bool, bool)> {
        let Value::Table(row) = lua.table(data).get_int(id) else { return None };
        let row = lua.table(row);
        Some((number(row.get_str("Score"))? as i32, set(row.get_str("IsBonusObjectiveComplete")), set(row.get_str("IsDisconnected"))))
    };
    let mut carry = super::players::SpotBattleCarry { round, ..Default::default() };
    for position in 1..=count {
        let id = number(lua.table(order).get_int(position))? as usize;
        if player(id)?.2 {
            continue;
        }
        if position <= index {
            carry.played += 1;
        }
        carry.order.push(id as i32 - 1);
    }
    for rank in 1..=count {
        let id = number(lua.table(ranks).get_int(rank))? as usize;
        let (score, bonus, gone) = player(id)?;
        if !gone {
            carry.ranking.push((id as i32 - 1, score, bonus));
        }
    }
    // `votingcomplete` keeps the voted player as its index plus one.
    if let Value::Table(vote) = vars.get_str("HighestVote") {
        let vote = lua.table(vote);
        if let (Some(score), Some(voted)) = (number(vote.get_str("Score")), number(vote.get_str("PlayerID")))
            && voted >= 1.0
        {
            carry.vote = Some((score as i32, voted as i32 - 1));
        }
    }
    Some(carry)
}

/// A carried state as `pc_spot_battle_restore` reads it: the script's own one-based players.
#[expect(clippy::cast_precision_loss, reason = "players, rounds and scores as the script's numbers")]
fn carry_table(lua: &mut Lua<World>, carry: &super::players::SpotBattleCarry) -> Value {
    let root = lua.create_table();
    let reference = lua.reference(Value::Table(root));
    let player = |index: i32| Value::Number((index + 1) as f32);
    let key = |index: i32| usize::try_from(index + 1).unwrap_or(0);
    lua.table_mut(root).set_str("Round", Value::Number(carry.round as f32));
    lua.table_mut(root).set_str("Played", Value::Number(carry.played as f32));
    let order = lua.create_table();
    lua.table_mut(root).set_str("Order", Value::Table(order));
    for (position, &index) in carry.order.iter().enumerate() {
        lua.table_mut(order).set_int(position + 1, player(index));
    }
    let ranking = lua.create_table();
    lua.table_mut(root).set_str("Ranking", Value::Table(ranking));
    let score = lua.create_table();
    lua.table_mut(root).set_str("Score", Value::Table(score));
    let bonus = lua.create_table();
    lua.table_mut(root).set_str("Bonus", Value::Table(bonus));
    for (rank, &(index, points, done)) in carry.ranking.iter().enumerate() {
        lua.table_mut(ranking).set_int(rank + 1, player(index));
        lua.table_mut(score).set_int(key(index), Value::Number(points as f32));
        if done {
            lua.table_mut(bonus).set_int(key(index), Value::Boolean(true));
        }
    }
    if let Some((points, index)) = carry.vote {
        lua.table_mut(root).set_str("VoteScore", Value::Number(points as f32));
        lua.table_mut(root).set_str("VotePlayer", player(index));
    }
    lua.unreference(reference);
    Value::Table(root)
}

/// PC port: runs `script_fixes::END_SOURCE`'s `pc_end_spot_battle(data, state, vars)` for the
/// graph, with the tables and the `ChangeState` a handler of it runs with
/// (`cStateGraph::HandleLocalMessage`); the source loads once. What it returns: whether the
/// battle took the end (true as well where the script cannot say, so no one asks again).
pub fn end_spot_battle(lua: &mut Lua<World>, world: &mut World, graph: usize) -> bool {
    use super::script_fixes::END_SOURCE;
    let Some(state) = world.activities.graph(graph).current_name().map(str::to_owned) else { return true };
    let chunk = hash64("PC port: end_spot_battle");
    if world.attached.insert(chunk) {
        let loaded = lua.load_source(END_SOURCE.as_bytes(), "end_spot_battle").and_then(|main| lua.call(world, &main, &[]).map(|_| ()));
        if let Err(e) = loaded {
            world.output(Output::ScriptError { context: "attach end_spot_battle".to_owned(), error: e.to_string() });
        }
    }
    world.closure = Some(graph);
    let (data, vars) = build_activity_tables(lua, world, graph);
    let references = [lua.reference(data.clone()), lua.reference(vars.clone())];
    let function = lua.get_global("pc_end_spot_battle");
    let taken = match lua.call(world, &function, &[data, Value::string(&state), vars]) {
        Ok(returned) => !matches!(returned.first(), Some(Value::Boolean(false))),
        Err(e) => {
            let context = format!("{}/{state}/end_spot_battle", world.activities.activities[graph].name);
            eprintln!("career script error in {context}: {e}");
            world.output(Output::ScriptError { context, error: e.to_string() });
            true
        }
    };
    for reference in references {
        lua.unreference(reference);
    }
    world.closure = None;
    taken
}

/// PC port: whether the handler is one a linked Skate 3 data folder's game of its type runs
/// through Skate 3's own rules (`script_fixes::SKATE3_HANDLERS`, whose source loads once, after
/// the shipped handlers it calls); without its wrapper defined the shipped handler runs.
fn skate3_handler(lua: &mut Lua<World>, world: &mut World, graph: usize, handler: u64) -> bool {
    use super::script_fixes::{SKATE3_HANDLERS, SKATE3_SOURCE};
    let Some(&(_, net_type)) = SKATE3_HANDLERS.iter().find(|&&(key, _)| key == handler) else { return false };
    let key = world.activities.activities[graph].key;
    if !crate::skate3::linked().is_some_and(|data| data.online_spot(key).is_some_and(|spot| spot.net_type == net_type)) {
        return false;
    }
    let chunk = hash64("PC port: skate3_handlers");
    if world.attached.insert(chunk) {
        for &(shipped, _) in &SKATE3_HANDLERS {
            if !world.attached.contains(&shipped) {
                attach(lua, world, shipped);
            }
        }
        let loaded = lua.load_source(SKATE3_SOURCE.as_bytes(), "skate3_handlers")
            .and_then(|main| lua.call(world, &main, &[]).map(|_| ()));
        if let Err(e) = loaded {
            world.output(Output::ScriptError { context: "attach skate3_handlers".to_owned(), error: e.to_string() });
        }
    }
    !lua.get_global(&format!("h_skate3_{handler:016X}")).is_nil()
}

/// PC port: whether the handler runs through the port's wrapper of it
/// (`script_fixes::FIXED_HANDLERS`, whose source loads once; the shipped handlers a wrapper
/// calls attach first); without its wrapper defined the shipped handler runs.
fn fixed_handler(lua: &mut Lua<World>, world: &mut World, handler: u64) -> bool {
    use super::script_fixes::{FIXED_HANDLERS, FIXES_SOURCE};
    let Some(&(_, other)) = FIXED_HANDLERS.iter().find(|&&(key, _)| key == handler) else { return false };
    if !world.attached.contains(&other) {
        attach(lua, world, other);
    }
    let chunk = hash64("PC port: script_fixes");
    if world.attached.insert(chunk) {
        let loaded = lua.load_source(FIXES_SOURCE.as_bytes(), "script_fixes")
            .and_then(|main| lua.call(world, &main, &[]).map(|_| ()));
        if let Err(e) = loaded {
            world.output(Output::ScriptError { context: "attach script_fixes".to_owned(), error: e.to_string() });
        }
    }
    !lua.get_global(&format!("h_fix_{handler:016X}")).is_nil()
}

fn handler_name(db: &AttribDb, handler: u64) -> String {
    text(db, class::CHALLENGE_MESSAGE_HANDLERS, handler, field::NAME).unwrap_or_else(|| format!("{handler:016x}"))
}

/// `cMessageHandler::Attach` (`0x82ce38c8`): `luaL_loadbuffer` the blob and
/// run it; the handler counts as attached whatever the outcome.
fn attach(lua: &mut Lua<World>, world: &mut World, handler: u64) {
    let db = std::sync::Arc::clone(&world.db);
    let Some(blob) = db.attribute(class::CHALLENGE_MESSAGE_HANDLERS, handler, field::HANDLER_BLOB).and_then(|a| a.blob()) else {
        return;
    };
    if blob.is_empty() {
        return;
    }
    world.attached.insert(handler);
    let name = handler_name(&db, handler);
    let chunk = super::script_fixes::corrected(handler, blob);
    match lua.load(&chunk, &name) {
        Ok(main) => {
            if let Err(e) = lua.call(world, &main, &[]) {
                world.output(Output::ScriptError { context: format!("attach {name}"), error: e.to_string() });
            }
        }
        Err(e) => world.output(Output::ScriptError { context: format!("attach {name}"), error: e.to_string() }),
    }
}

/// `cStateGraph::ChangeStateFromScript` (`0x82ce4028`): queue a
/// `cEventChangeState`; the state is entered when the queue is flushed.
fn change_state_from_script(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> LuaResult<Vec<Value>> {
    let name = check_string(lua, call, 1, "ChangeState")?;
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "graph index upvalue")]
    let graph = match call.upvalues.first() {
        Some(Value::Number(n)) => *n as usize,
        _ => return Err(lua.error("ChangeState: missing state graph")),
    };
    let hash = hash32(&String::from_utf8_lossy(&name));
    world.send(Event::ChangeState { graph, state: hash });
    Ok(Vec::new())
}
