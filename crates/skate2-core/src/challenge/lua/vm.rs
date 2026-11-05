//! `lvm.c` `luaV_execute` and `ldo.c` call handling for Lua 5.1.

use super::{Function, Lua, LuaError, LuaResult, NativeCall, Proto, Upvalue, Value, number};
use skate2_formats::lua_chunk::{arg_a, arg_b, arg_bx, arg_c, arg_sbx, opcode};
use std::sync::Arc;

const OP_MOVE: u32 = 0;
const OP_LOADK: u32 = 1;
const OP_LOADBOOL: u32 = 2;
const OP_LOADNIL: u32 = 3;
const OP_GETUPVAL: u32 = 4;
const OP_GETGLOBAL: u32 = 5;
const OP_GETTABLE: u32 = 6;
const OP_SETGLOBAL: u32 = 7;
const OP_SETUPVAL: u32 = 8;
const OP_SETTABLE: u32 = 9;
const OP_NEWTABLE: u32 = 10;
const OP_SELF: u32 = 11;
const OP_ADD: u32 = 12;
const OP_SUB: u32 = 13;
const OP_MUL: u32 = 14;
const OP_DIV: u32 = 15;
const OP_MOD: u32 = 16;
const OP_POW: u32 = 17;
const OP_UNM: u32 = 18;
const OP_NOT: u32 = 19;
const OP_LEN: u32 = 20;
const OP_CONCAT: u32 = 21;
const OP_JMP: u32 = 22;
const OP_EQ: u32 = 23;
const OP_LT: u32 = 24;
const OP_LE: u32 = 25;
const OP_TEST: u32 = 26;
const OP_TESTSET: u32 = 27;
const OP_CALL: u32 = 28;
const OP_TAILCALL: u32 = 29;
const OP_RETURN: u32 = 30;
const OP_FORLOOP: u32 = 31;
const OP_FORPREP: u32 = 32;
const OP_TFORLOOP: u32 = 33;
const OP_SETLIST: u32 = 34;
const OP_CLOSE: u32 = 35;
const OP_CLOSURE: u32 = 36;
const OP_VARARG: u32 = 37;

/// `LFIELDS_PER_FLUSH`.
const FIELDS_PER_FLUSH: usize = 50;
/// `MAXTAGLOOP`.
const MAX_TAG_LOOP: usize = 100;
/// `LUAI_MAXCSTACK`-scale guard for runaway stacks.
const MAX_STACK: usize = 1 << 20;

#[derive(Clone, Copy)]
enum Arith {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Unm,
}

impl Arith {
    const fn event(self) -> &'static str {
        match self {
            Self::Add => "__add",
            Self::Sub => "__sub",
            Self::Mul => "__mul",
            Self::Div => "__div",
            Self::Mod => "__mod",
            Self::Pow => "__pow",
            Self::Unm => "__unm",
        }
    }
    /// `luai_num*` on `float`.
    #[expect(clippy::cast_possible_truncation, reason = "C double results narrowed to lua_Number")]
    fn apply(self, a: f32, b: f32) -> f32 {
        match self {
            Self::Add => a + b,
            Self::Sub => a - b,
            Self::Mul => a * b,
            Self::Div => a / b,
            Self::Mod => (f64::from(a) - f64::from(a / b).floor() * f64::from(b)) as f32,
            Self::Pow => f64::from(a).powf(f64::from(b)) as f32,
            Self::Unm => -a,
        }
    }
}

/// `luaO_fb2int`.
const fn fb2int(x: usize) -> usize {
    let e = (x >> 3) & 31;
    if e == 0 { x } else { ((x & 7) + 8) << (e - 1) }
}

/// `luaV_tonumber`.
pub(crate) fn to_number(value: &Value) -> Option<f32> {
    match value {
        Value::Number(n) => Some(*n),
        Value::String(s) => number::string_to_number(s.as_bytes()),
        _ => None,
    }
}

const fn same_type(a: &Value, b: &Value) -> bool {
    matches!(
        (a, b),
        (Value::Nil, Value::Nil)
            | (Value::Boolean(_), Value::Boolean(_))
            | (Value::Number(_), Value::Number(_))
            | (Value::String(_), Value::String(_))
            | (Value::Table(_), Value::Table(_))
            | (Value::Function(_), Value::Function(_))
            | (Value::UserData(_), Value::UserData(_))
            | (Value::LightUserData(_), Value::LightUserData(_))
    )
}

impl<H> Lua<H> {
    fn ensure(&mut self, size: usize) -> LuaResult<()> {
        if size > MAX_STACK {
            return Err(self.error("stack overflow"));
        }
        if self.stack.len() < size {
            self.stack.resize(size.next_power_of_two(), Value::Nil);
        }
        Ok(())
    }

    /// `lua_call` from the host: push `function(args...)` above the current
    /// top and return every result.
    ///
    /// Host `Value`s are heap handles, not GC roots. Keep values needed across
    /// separate calls (including a handler's lazy attachment) in the registry
    /// with `reference`/`unreference`, just as native callers retain stack slots.
    /// Results are protected through this return; retaining them across another
    /// call or an explicit collection also requires a root.
    pub fn call(&mut self, host: &mut H, function: &Value, args: &[Value]) -> LuaResult<Vec<Value>> {
        let frames = self.frames.len();
        let depth = self.depth;
        let func = self.top;
        let result = self.call_function(host, function.clone(), args, None);
        if result.is_err() {
            self.close_upvalues(func);
            self.frames.truncate(frames);
            self.depth = depth;
            for slot in self.stack.iter_mut().skip(func) {
                if slot.is_nil() {
                    break;
                }
                *slot = Value::Nil;
            }
        }
        self.top = func;
        // luaD_call (0x8299c398) checks GC while returned values are still
        // on the Lua stack. Our host API moved them into a Vec, so retain them
        // as explicit roots until this call hands them back to its caller.
        let roots = result.as_ref().map_or(&[][..], |values| values.as_slice());
        self.maybe_collect_with_roots(roots);
        result
    }

    /// Calls `function` above the current top (`luaD_call`); used by natives
    /// and metamethods. `wanted` is `nresults` (`None` = `LUA_MULTRET`).
    pub fn call_function(
        &mut self,
        host: &mut H,
        function: Value,
        args: &[Value],
        wanted: Option<usize>,
    ) -> LuaResult<Vec<Value>> {
        let func = self.top;
        self.ensure(func + args.len() + 2)?;
        self.stack[func] = function;
        for (i, arg) in args.iter().enumerate() {
            self.stack[func + 1 + i] = arg.clone();
        }
        self.top = func + 1 + args.len();
        let n = self.call_at(host, func, args.len(), wanted)?;
        let out = self.stack[func..func + n].to_vec();
        self.top = func;
        Ok(out)
    }

    fn call_first(&mut self, host: &mut H, function: Value, args: &[Value]) -> LuaResult<Value> {
        Ok(self.call_function(host, function, args, Some(1))?.into_iter().next().unwrap_or_default())
    }

    /// `luaD_precall`/`luaD_poscall`: results end up at `func..func+n`.
    pub(crate) fn call_at(&mut self, host: &mut H, func: usize, nargs: usize, wanted: Option<usize>) -> LuaResult<usize> {
        if self.depth >= Self::max_call_depth() {
            return Err(self.error("C stack overflow"));
        }
        let callee = self.stack[func].clone();
        let Value::Function(id) = callee else {
            let tm = self.metamethod(&callee, "__call");
            if !matches!(tm, Value::Function(_)) {
                return Err(self.error(format!("attempt to call a {} value", callee.type_name())));
            }
            self.ensure(func + nargs + 3)?;
            for i in (func..=func + nargs).rev() {
                self.stack[i + 1] = self.stack[i].clone();
            }
            self.stack[func] = tm;
            self.top = func + nargs + 2;
            return self.call_at(host, func, nargs + 1, wanted);
        };
        let count = match self.function(id) {
            Function::Native { function, upvalues, name } => {
                if trace_natives() {
                    eprintln!("career native: {name}");
                }
                let function = *function;
                let call = NativeCall {
                    args: self.stack[func + 1..func + 1 + nargs].to_vec(),
                    upvalues: upvalues.clone(),
                };
                self.top = func + 1 + nargs;
                self.depth += 1;
                let result = function(self, host, &call);
                self.depth -= 1;
                let results = result?;
                self.ensure(func + results.len() + 1)?;
                let n = results.len();
                for (i, v) in results.into_iter().enumerate() {
                    self.stack[func + i] = v;
                }
                n
            }
            Function::Lua { proto, upvalues } => {
                let proto = Arc::clone(proto);
                let upvalues = upvalues.clone();
                self.depth += 1;
                let result = self.execute(host, func, nargs, &proto, &upvalues);
                self.depth -= 1;
                result?
            }
        };
        let count = match wanted {
            Some(w) => {
                self.ensure(func + w + 1)?;
                for i in count..w {
                    self.stack[func + i] = Value::Nil;
                }
                w
            }
            None => count,
        };
        self.top = func + count;
        Ok(count)
    }

    fn execute(&mut self, host: &mut H, func: usize, nargs: usize, proto: &Arc<Proto>, upvalues: &[usize]) -> LuaResult<usize> {
        let np = proto.parameter_count;
        let base = if proto.vararg {
            // adjust_varargs: fixed parameters move above the actual arguments.
            let first = func + 1;
            let actual = nargs.max(np);
            self.ensure(first + actual + np + proto.max_stack + 1)?;
            for i in nargs..np {
                self.stack[first + i] = Value::Nil;
            }
            let base = first + actual;
            for i in 0..np {
                self.stack[base + i] = std::mem::take(&mut self.stack[first + i]);
            }
            base
        } else {
            func + 1
        };
        let frame_top = base + proto.max_stack;
        self.ensure(frame_top + 1)?;
        let first_clear = if proto.vararg { base + np } else { base + nargs.min(np) };
        for slot in &mut self.stack[first_clear..frame_top] {
            *slot = Value::Nil;
        }
        self.top = frame_top;
        self.frames.push(super::FrameInfo { source: Arc::clone(&proto.source), pc: 0 });
        let frame = self.frames.len() - 1;
        let result = self.run(host, func, base, frame_top, proto, upvalues, frame);
        self.frames.truncate(frame);
        result
    }

    #[expect(clippy::too_many_arguments, reason = "interpreter frame registers")]
    #[expect(clippy::too_many_lines, reason = "single opcode dispatch, as luaV_execute")]
    fn run(
        &mut self,
        host: &mut H,
        func: usize,
        base: usize,
        frame_top: usize,
        proto: &Arc<Proto>,
        upvalues: &[usize],
        frame: usize,
    ) -> LuaResult<usize> {
        let code = &proto.code;
        let k = &proto.constants;
        let mut pc = 0usize;
        macro_rules! rk {
            ($x:expr) => {{
                let x = $x;
                if x >= 256 { k[x - 256].clone() } else { self.stack[base + x].clone() }
            }};
        }
        macro_rules! jump {
            ($offset:expr) => {{
                pc = usize::try_from(pc as i64 + i64::from($offset)).map_err(|_| self.error("bad jump"))?;
            }};
        }
        loop {
            let i = *code.get(pc).ok_or_else(|| self.error("pc out of range"))?;
            self.frames[frame].pc = pc;
            pc += 1;
            let a = arg_a(i);
            match opcode(i) {
                OP_MOVE => self.stack[base + a] = self.stack[base + arg_b(i)].clone(),
                OP_LOADK => self.stack[base + a] = k[arg_bx(i)].clone(),
                OP_LOADBOOL => {
                    self.stack[base + a] = Value::Boolean(arg_b(i) != 0);
                    if arg_c(i) != 0 {
                        pc += 1;
                    }
                }
                OP_LOADNIL => {
                    for r in a..=arg_b(i) {
                        self.stack[base + r] = Value::Nil;
                    }
                }
                OP_GETUPVAL => self.stack[base + a] = self.read_upvalue(upvalues[arg_b(i)]),
                OP_GETGLOBAL => {
                    let key = k[arg_bx(i)].clone();
                    let globals = Value::Table(self.globals());
                    let v = self.index_value(host, &globals, &key, || describe_global(&key))?;
                    self.stack[base + a] = v;
                }
                OP_GETTABLE => {
                    let t = self.stack[base + arg_b(i)].clone();
                    let key = rk!(arg_c(i));
                    let v = self.index_value(host, &t, &key, || describe_register(proto, pc - 1, arg_b(i)))?;
                    self.stack[base + a] = v;
                }
                OP_SETGLOBAL => {
                    let key = k[arg_bx(i)].clone();
                    let globals = Value::Table(self.globals());
                    let v = self.stack[base + a].clone();
                    self.set_index(host, &globals, &key, v)?;
                }
                OP_SETUPVAL => {
                    let v = self.stack[base + a].clone();
                    self.write_upvalue(upvalues[arg_b(i)], v);
                }
                OP_SETTABLE => {
                    let t = self.stack[base + a].clone();
                    let key = rk!(arg_b(i));
                    let v = rk!(arg_c(i));
                    self.set_index(host, &t, &key, v)?;
                }
                OP_NEWTABLE => {
                    let t = self.create_table_with(fb2int(arg_b(i)), fb2int(arg_c(i)));
                    self.stack[base + a] = Value::Table(t);
                }
                OP_SELF => {
                    let object = self.stack[base + arg_b(i)].clone();
                    let key = rk!(arg_c(i));
                    self.stack[base + a + 1] = object.clone();
                    let v = self.index_value(host, &object, &key, || describe_register(proto, pc - 1, arg_b(i)))?;
                    self.stack[base + a] = v;
                }
                op @ (OP_ADD | OP_SUB | OP_MUL | OP_DIV | OP_MOD | OP_POW) => {
                    let kind = match op {
                        OP_ADD => Arith::Add,
                        OP_SUB => Arith::Sub,
                        OP_MUL => Arith::Mul,
                        OP_DIV => Arith::Div,
                        OP_MOD => Arith::Mod,
                        _ => Arith::Pow,
                    };
                    let b = rk!(arg_b(i));
                    let c = rk!(arg_c(i));
                    let v = self.arith(host, kind, &b, &c)?;
                    self.stack[base + a] = v;
                }
                OP_UNM => {
                    let b = self.stack[base + arg_b(i)].clone();
                    let v = self.arith(host, Arith::Unm, &b, &b)?;
                    self.stack[base + a] = v;
                }
                OP_NOT => self.stack[base + a] = Value::Boolean(self.stack[base + arg_b(i)].is_falsy()),
                OP_LEN => {
                    let b = self.stack[base + arg_b(i)].clone();
                    let v = self.length(host, &b)?;
                    self.stack[base + a] = v;
                }
                OP_CONCAT => {
                    let values = self.stack[base + arg_b(i)..=base + arg_c(i)].to_vec();
                    let v = self.concat(host, values)?;
                    self.stack[base + a] = v;
                }
                OP_JMP => jump!(arg_sbx(i)),
                op @ (OP_EQ | OP_LT | OP_LE) => {
                    let b = rk!(arg_b(i));
                    let c = rk!(arg_c(i));
                    let result = match op {
                        OP_EQ => self.equals(host, &b, &c)?,
                        OP_LT => self.less_than(host, &b, &c)?,
                        _ => self.less_equal(host, &b, &c)?,
                    };
                    if result == (a != 0) {
                        jump!(arg_sbx(code[pc]));
                    }
                    pc += 1;
                }
                OP_TEST => {
                    if usize::from(self.stack[base + a].is_falsy()) != arg_c(i) {
                        jump!(arg_sbx(code[pc]));
                    }
                    pc += 1;
                }
                OP_TESTSET => {
                    let b = self.stack[base + arg_b(i)].clone();
                    if usize::from(b.is_falsy()) != arg_c(i) {
                        self.stack[base + a] = b;
                        jump!(arg_sbx(code[pc]));
                    }
                    pc += 1;
                }
                OP_CALL | OP_TAILCALL => {
                    let callee = base + a;
                    let b = arg_b(i);
                    let nargs = if b != 0 { b - 1 } else { self.top - callee - 1 };
                    let wanted = if opcode(i) == OP_CALL && arg_c(i) != 0 { Some(arg_c(i) - 1) } else { None };
                    self.top = callee + 1 + nargs;
                    let callee_value = &self.stack[callee];
                    if !matches!(callee_value, Value::Function(_)) && self.metamethod(callee_value, "__call").is_nil() {
                        return Err(self.type_error(callee_value, "call", &describe_register(proto, pc - 1, a)));
                    }
                    let n = self.call_at(host, callee, nargs, wanted)?;
                    if opcode(i) == OP_TAILCALL {
                        self.close_upvalues(base);
                        for j in 0..n {
                            self.stack[func + j] = self.stack[callee + j].clone();
                        }
                        self.top = func + n;
                        return Ok(n);
                    }
                    self.top = if wanted.is_some() { frame_top } else { callee + n };
                }
                OP_RETURN => {
                    let b = arg_b(i);
                    let n = if b != 0 { b - 1 } else { self.top - (base + a) };
                    self.close_upvalues(base);
                    for j in 0..n {
                        self.stack[func + j] = self.stack[base + a + j].clone();
                    }
                    self.top = func + n;
                    return Ok(n);
                }
                OP_FORLOOP => {
                    let step = self.stack[base + a + 2].as_number().unwrap_or_default();
                    let index = self.stack[base + a].as_number().unwrap_or_default() + step;
                    let limit = self.stack[base + a + 1].as_number().unwrap_or_default();
                    if if 0.0 < step { index <= limit } else { limit <= index } {
                        jump!(arg_sbx(i));
                        self.stack[base + a] = Value::Number(index);
                        self.stack[base + a + 3] = Value::Number(index);
                    }
                }
                OP_FORPREP => {
                    let init = to_number(&self.stack[base + a]).ok_or_else(|| self.error("'for' initial value must be a number"))?;
                    let limit = to_number(&self.stack[base + a + 1]).ok_or_else(|| self.error("'for' limit must be a number"))?;
                    let step = to_number(&self.stack[base + a + 2]).ok_or_else(|| self.error("'for' step must be a number"))?;
                    self.stack[base + a + 1] = Value::Number(limit);
                    self.stack[base + a + 2] = Value::Number(step);
                    self.stack[base + a] = Value::Number(init - step);
                    jump!(arg_sbx(i));
                }
                OP_TFORLOOP => {
                    let cb = base + a + 3;
                    self.stack[cb + 2] = self.stack[base + a + 2].clone();
                    self.stack[cb + 1] = self.stack[base + a + 1].clone();
                    self.stack[cb] = self.stack[base + a].clone();
                    self.top = cb + 3;
                    self.call_at(host, cb, 2, Some(arg_c(i)))?;
                    self.top = frame_top;
                    if self.stack[cb].is_nil() {
                        pc += 1;
                    } else {
                        self.stack[cb - 1] = self.stack[cb].clone();
                        jump!(arg_sbx(code[pc]));
                        pc += 1;
                    }
                }
                OP_SETLIST => {
                    let mut n = arg_b(i);
                    let mut c = arg_c(i);
                    if n == 0 {
                        n = self.top - (base + a) - 1;
                        self.top = frame_top;
                    }
                    if c == 0 {
                        c = code[pc] as usize;
                        pc += 1;
                    }
                    let Value::Table(t) = self.stack[base + a] else {
                        return Err(self.error("SETLIST target is not a table"));
                    };
                    let mut last = (c - 1) * FIELDS_PER_FLUSH + n;
                    for j in (1..=n).rev() {
                        let v = self.stack[base + a + j].clone();
                        self.table_mut(t).set_int(last, v);
                        last -= 1;
                    }
                }
                OP_CLOSE => self.close_upvalues(base + a),
                OP_CLOSURE => {
                    let child = Arc::clone(&proto.protos[arg_bx(i)]);
                    let mut captured = Vec::with_capacity(child.upvalue_count);
                    for _ in 0..child.upvalue_count {
                        let pseudo = code[pc];
                        pc += 1;
                        if opcode(pseudo) == OP_GETUPVAL {
                            captured.push(upvalues[arg_b(pseudo)]);
                        } else {
                            captured.push(self.find_upvalue(base + arg_b(pseudo)));
                        }
                    }
                    let f = self.create_closure(child, captured);
                    self.stack[base + a] = Value::Function(f);
                }
                OP_VARARG => {
                    let available = base - func - 1 - proto.parameter_count;
                    let b = arg_b(i);
                    if b == 0 {
                        self.ensure(base + a + available + 1)?;
                        for j in 0..available {
                            self.stack[base + a + j] = self.stack[base - available + j].clone();
                        }
                        self.top = base + a + available;
                    } else {
                        for j in 0..b - 1 {
                            self.stack[base + a + j] =
                                if j < available { self.stack[base - available + j].clone() } else { Value::Nil };
                        }
                    }
                }
                other => return Err(self.error(format!("invalid opcode {other}"))),
            }
        }
    }

    fn read_upvalue(&self, id: usize) -> Value {
        match self.upvalue(id) {
            Upvalue::Open(slot) => self.stack[*slot].clone(),
            Upvalue::Closed(v) => v.clone(),
        }
    }

    fn write_upvalue(&mut self, id: usize, value: Value) {
        match self.upvalue_mut(id) {
            Upvalue::Open(slot) => {
                let slot = *slot;
                self.stack[slot] = value;
            }
            Upvalue::Closed(v) => *v = value,
        }
    }

    /// `luaF_findupval`.
    fn find_upvalue(&mut self, level: usize) -> usize {
        if let Some(&id) = self.open_upvalues.get(&level) {
            return id;
        }
        let id = self.create_upvalue(Upvalue::Open(level));
        self.open_upvalues.insert(level, id);
        id
    }

    /// `luaF_close`.
    pub(crate) fn close_upvalues(&mut self, level: usize) {
        let open: Vec<(usize, usize)> = self.open_upvalues.range(level..).map(|(&s, &id)| (s, id)).collect();
        for (slot, id) in open {
            let v = self.stack[slot].clone();
            *self.upvalue_mut(id) = Upvalue::Closed(v);
            self.open_upvalues.remove(&slot);
        }
    }

    /// `luaV_gettable`.
    pub fn index(&mut self, host: &mut H, table: &Value, key: &Value) -> LuaResult<Value> {
        self.index_value(host, table, key, String::new)
    }

    fn index_value(&mut self, host: &mut H, table: &Value, key: &Value, describe: impl Fn() -> String) -> LuaResult<Value> {
        let mut t = table.clone();
        for _ in 0..MAX_TAG_LOOP {
            let tm = if let Value::Table(id) = t {
                let v = self.table(id).get(key);
                if !v.is_nil() {
                    return Ok(v);
                }
                let tm = self.table(id).metatable.map_or(Value::Nil, |mt| self.table(mt).get_str("__index"));
                if tm.is_nil() {
                    return Ok(Value::Nil);
                }
                tm
            } else {
                let tm = self.metamethod(&t, "__index");
                if tm.is_nil() {
                    return Err(self.type_error(&t, "index", &describe()));
                }
                tm
            };
            if let Value::Function(_) = tm {
                return self.call_first(host, tm, &[t, key.clone()]);
            }
            t = tm;
        }
        Err(self.error("loop in gettable"))
    }

    /// `luaV_settable`.
    pub fn set_index(&mut self, host: &mut H, table: &Value, key: &Value, value: Value) -> LuaResult<()> {
        let mut t = table.clone();
        for _ in 0..MAX_TAG_LOOP {
            let tm = if let Value::Table(id) = t {
                let existing = self.table(id).get(key);
                let tm = if existing.is_nil() {
                    self.table(id).metatable.map_or(Value::Nil, |mt| self.table(mt).get_str("__newindex"))
                } else {
                    Value::Nil
                };
                if tm.is_nil() {
                    return self.table_mut(id).set(key.clone(), value).map_err(|e| self.error(e.message()));
                }
                tm
            } else {
                let tm = self.metamethod(&t, "__newindex");
                if tm.is_nil() {
                    return Err(self.type_error(&t, "index", ""));
                }
                tm
            };
            if let Value::Function(_) = tm {
                self.call_function(host, tm, &[t, key.clone(), value], Some(0))?;
                return Ok(());
            }
            t = tm;
        }
        Err(self.error("loop in settable"))
    }

    fn type_error(&self, value: &Value, operation: &str, description: &str) -> LuaError {
        if description.is_empty() {
            self.error(format!("attempt to {operation} a {} value", value.type_name()))
        } else {
            self.error(format!("attempt to {operation} {description} (a {} value)", value.type_name()))
        }
    }

    fn arith(&mut self, host: &mut H, kind: Arith, b: &Value, c: &Value) -> LuaResult<Value> {
        if let (Some(x), Some(y)) = (to_number(b), to_number(c)) {
            return Ok(Value::Number(kind.apply(x, y)));
        }
        let mut tm = self.metamethod(b, kind.event());
        if tm.is_nil() {
            tm = self.metamethod(c, kind.event());
        }
        if tm.is_nil() {
            let culprit = if to_number(b).is_none() { b } else { c };
            return Err(self.type_error(culprit, "perform arithmetic on", ""));
        }
        self.call_first(host, tm, &[b.clone(), c.clone()])
    }

    fn length(&mut self, host: &mut H, value: &Value) -> LuaResult<Value> {
        #[expect(clippy::cast_precision_loss, reason = "lua_Number length")]
        match value {
            Value::Table(t) => Ok(Value::Number(self.table(*t).len() as f32)),
            Value::String(s) => Ok(Value::Number(s.as_bytes().len() as f32)),
            _ => {
                let tm = self.metamethod(value, "__len");
                if tm.is_nil() {
                    return Err(self.type_error(value, "get length of", ""));
                }
                self.call_first(host, tm, &[value.clone(), Value::Nil])
            }
        }
    }

    /// `luaV_concat` over `R(B)..R(C)`.
    fn concat(&mut self, host: &mut H, mut values: Vec<Value>) -> LuaResult<Value> {
        let stringish = |v: &Value| matches!(v, Value::String(_) | Value::Number(_));
        while values.len() > 1 {
            let n = values.len();
            if !stringish(&values[n - 2]) || !stringish(&values[n - 1]) {
                let (x, y) = (values[n - 2].clone(), values[n - 1].clone());
                let mut tm = self.metamethod(&x, "__concat");
                if tm.is_nil() {
                    tm = self.metamethod(&y, "__concat");
                }
                if tm.is_nil() {
                    let culprit = if stringish(&x) { &y } else { &x };
                    return Err(self.type_error(culprit, "concatenate", ""));
                }
                let r = self.call_first(host, tm, &[x, y])?;
                values.truncate(n - 2);
                values.push(r);
                continue;
            }
            let mut start = n - 1;
            while start > 0 && stringish(&values[start - 1]) {
                start -= 1;
            }
            let mut joined = Vec::new();
            for v in &values[start..] {
                match v {
                    Value::String(s) => joined.extend_from_slice(s.as_bytes()),
                    Value::Number(x) => joined.extend_from_slice(number::number_to_string(*x).as_bytes()),
                    _ => unreachable!("stringish"),
                }
            }
            values.truncate(start);
            values.push(Value::bytes(&joined));
        }
        Ok(values.pop().unwrap_or_default())
    }

    /// `luaV_equalval` / `equalobj`.
    pub fn equals(&mut self, host: &mut H, a: &Value, b: &Value) -> LuaResult<bool> {
        if !same_type(a, b) {
            return Ok(false);
        }
        let (mt_a, mt_b) = match (a, b) {
            (Value::Table(x), Value::Table(y)) if x != y => (self.table(*x).metatable, self.table(*y).metatable),
            (Value::UserData(x), Value::UserData(y)) if x != y => (self.userdata(*x).metatable, self.userdata(*y).metatable),
            _ => return Ok(a.raw_equal(b)),
        };
        let tm = self.comparison_metamethod(mt_a, mt_b, "__eq");
        if tm.is_nil() {
            return Ok(false);
        }
        Ok(!self.call_first(host, tm, &[a.clone(), b.clone()])?.is_falsy())
    }

    /// `get_compTM`.
    fn comparison_metamethod(&self, mt_a: Option<super::TableId>, mt_b: Option<super::TableId>, event: &str) -> Value {
        let Some(mt_a) = mt_a else { return Value::Nil };
        let tm_a = self.table(mt_a).get_str(event);
        if tm_a.is_nil() {
            return Value::Nil;
        }
        if Some(mt_a) == mt_b {
            return tm_a;
        }
        let Some(mt_b) = mt_b else { return Value::Nil };
        let tm_b = self.table(mt_b).get_str(event);
        if tm_a.raw_equal(&tm_b) { tm_a } else { Value::Nil }
    }

    /// `call_orderTM`: `None` when no shared handler exists.
    fn order_metamethod(&mut self, host: &mut H, a: &Value, b: &Value, event: &str) -> LuaResult<Option<bool>> {
        let tm_a = self.metamethod(a, event);
        if tm_a.is_nil() {
            return Ok(None);
        }
        let tm_b = self.metamethod(b, event);
        if !tm_a.raw_equal(&tm_b) {
            return Ok(None);
        }
        Ok(Some(!self.call_first(host, tm_a, &[a.clone(), b.clone()])?.is_falsy()))
    }

    fn order_error(&self, a: &Value, b: &Value) -> LuaError {
        let (ta, tb) = (a.type_name(), b.type_name());
        if ta == tb {
            self.error(format!("attempt to compare two {ta} values"))
        } else {
            self.error(format!("attempt to compare {ta} with {tb}"))
        }
    }

    /// `luaV_lessthan`.
    pub fn less_than(&mut self, host: &mut H, a: &Value, b: &Value) -> LuaResult<bool> {
        if !same_type(a, b) {
            return Err(self.order_error(a, b));
        }
        match (a, b) {
            (Value::Number(x), Value::Number(y)) => Ok(x < y),
            (Value::String(x), Value::String(y)) => Ok(x.as_bytes() < y.as_bytes()),
            _ => self.order_metamethod(host, a, b, "__lt")?.ok_or_else(|| self.order_error(a, b)),
        }
    }

    /// `lessequal`.
    fn less_equal(&mut self, host: &mut H, a: &Value, b: &Value) -> LuaResult<bool> {
        if !same_type(a, b) {
            return Err(self.order_error(a, b));
        }
        match (a, b) {
            (Value::Number(x), Value::Number(y)) => Ok(x <= y),
            (Value::String(x), Value::String(y)) => Ok(x.as_bytes() <= y.as_bytes()),
            _ => {
                if let Some(r) = self.order_metamethod(host, a, b, "__le")? {
                    return Ok(r);
                }
                if let Some(r) = self.order_metamethod(host, b, a, "__lt")? {
                    return Ok(!r);
                }
                Err(self.order_error(a, b))
            }
        }
    }
}

fn describe_global(key: &Value) -> String {
    match key {
        Value::String(s) => format!("global '{}'", s.to_str_lossy()),
        _ => String::new(),
    }
}

/// Simplified `getobjname`: the instruction that last loaded `register`.
fn describe_register(proto: &Proto, pc: usize, register: usize) -> String {
    for at in (0..pc).rev() {
        let i = proto.code[at];
        let op = opcode(i);
        if op == OP_JMP || op == OP_FORLOOP || op == OP_FORPREP {
            break;
        }
        if arg_a(i) != register || matches!(op, OP_SETTABLE | OP_SETGLOBAL | OP_SETUPVAL | OP_EQ | OP_LT | OP_LE | OP_TEST | OP_SETLIST) {
            continue;
        }
        return match op {
            OP_GETGLOBAL => describe_global(&proto.constants[arg_bx(i)]),
            OP_GETTABLE | OP_SELF if arg_c(i) >= 256 => match &proto.constants[arg_c(i) - 256] {
                Value::String(s) => format!("{} '{}'", if op == OP_SELF { "method" } else { "field" }, s.to_str_lossy()),
                _ => String::new(),
            },
            OP_MOVE if arg_b(i) < register => describe_register(proto, at, arg_b(i)),
            _ => String::new(),
        };
    }
    String::new()
}

/// PC port: `SKATE2_TRACE_CAREER_NATIVES` logs every native a script calls.
fn trace_natives() -> bool {
    static TRACE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *TRACE.get_or_init(|| std::env::var_os("SKATE2_TRACE_CAREER_NATIVES").is_some())
}
