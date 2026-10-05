//! The VM content runs in: which libraries it gets, and freezing.
//!
//! Scripts get the parts of the standard library whose results are the
//! same on every machine and don't depend on the VM's history:
//!
//! - base functions, minus `collectgarbage`/`gcinfo` (they observe the
//!   GC), `getfenv`/`setfenv`, `loadstring`, `newproxy`; `print` goes to
//!   a debug log only when asked for; `setmetatable` refuses weak tables
//!   (`__mode`), whose contents depend on when the GC ran;
//! - `string`, `table`, `bit32`;
//! - a `math` with only exact integer operations (`abs`, `ceil`, `floor`,
//!   `max`, `min`, `clamp`, `sign`, `round`): no `random` (the RNG is the
//!   engine's), no `noise`, no transcendental functions (their results can
//!   differ between C libraries);
//! - no `os`, `io`, `debug`, `coroutine` (a suspended coroutine is hidden
//!   state), `buffer` (mutable memory), `utf8`, `vector`.

use std::collections::HashSet;
use std::ffi::c_void;

use mlua::chunk::Compiler;
use mlua::{Lua, LuaOptions, StdLib, Table, Value, ffi};

/// The `math` functions content keeps: exact on integers.
const EXACT_MATH: [&str; 8] = ["abs", "ceil", "floor", "max", "min", "clamp", "sign", "round"];

/// Builtins the compiler must not treat as known. Luau folds calls to known
/// builtins with constant arguments at compile time, and under `safeenv`
/// calls them directly (`FASTCALL`) without looking them up, so removing
/// them from the environment isn't enough: `math.sin(1)` would still run.
/// Listing them here makes every call go through the environment, where
/// they don't exist. `setmetatable` is listed so calls reach the guarded
/// version below.
const NOT_BUILTINS: [&str; 31] = [
    "math.random",
    "math.randomseed",
    "math.noise",
    "math.sin",
    "math.cos",
    "math.tan",
    "math.asin",
    "math.acos",
    "math.atan",
    "math.atan2",
    "math.sinh",
    "math.cosh",
    "math.tanh",
    "math.exp",
    "math.log",
    "math.log10",
    "math.pow",
    "math.sqrt",
    "math.frexp",
    "math.ldexp",
    "math.deg",
    "math.rad",
    "math.fmod",
    "math.modf",
    "math.lerp",
    "math.map",
    "math.isnan",
    "math.isinf",
    "math.isfinite",
    "setmetatable",
    "vector",
];

/// The compiler content is compiled with.
pub fn compiler() -> Compiler {
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    crate::wasm::initialize();
    Compiler::new().set_optimization_level(1).set_debug_level(2).set_disabled_builtins(NOT_BUILTINS)
}

/// A VM with the content standard library.
pub fn new_vm(debug_print: bool) -> mlua::Result<Lua> {
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    crate::wasm::initialize();
    let lua = Lua::new_with(StdLib::STRING | StdLib::TABLE | StdLib::BIT | StdLib::MATH, LuaOptions::default())?;
    let g = lua.globals();
    for name in
        ["collectgarbage", "gcinfo", "getfenv", "setfenv", "loadstring", "newproxy", "load", "dofile", "loadfile"]
    {
        g.raw_set(name, Value::Nil)?;
    }
    let math: Table = g.get("math")?;
    let exact = lua.create_table()?;
    for name in EXACT_MATH {
        exact.raw_set(name, math.raw_get::<Value>(name)?)?;
    }
    g.raw_set("math", exact)?;
    let print = lua.create_function(move |_, args: mlua::Variadic<Value>| {
        if debug_print {
            let parts: Vec<String> = args.iter().map(|v| format!("{v:?}")).collect();
            eprintln!("[content] {}", parts.join("\t"));
        }
        Ok(())
    })?;
    g.raw_set("print", print)?;
    let setmetatable: mlua::Function = g.get("setmetatable")?;
    let guarded = lua.create_function(move |_, (t, mt): (Table, Option<Table>)| {
        if let Some(mt) = &mt
            && !mt.raw_get::<Value>("__mode")?.is_nil()
        {
            return Err(mlua::Error::runtime(
                "weak tables (__mode) are not allowed in content: their contents depend on when the GC runs",
            ));
        }
        setmetatable.call::<Table>((t, mt))
    })?;
    g.raw_set("setmetatable", guarded)?;
    Ok(lua)
}

/// Make `v` and everything reachable from it read-only: tables (and their
/// metatables) are frozen, and functions' upvalues are followed.
pub fn deep_freeze(lua: &Lua, v: &Value) -> mlua::Result<()> {
    // SAFETY: only Luau stack operations on the value mlua pushed; the
    // stack is balanced by the end (exec_raw restores it anyway).
    unsafe {
        lua.exec_raw::<()>(v.clone(), |state| {
            let mut seen = HashSet::new();
            freeze_at(state, 1, &mut seen);
            ffi::lua_settop(state, 0);
        })
    }
}

unsafe fn freeze_at(state: *mut ffi::lua_State, idx: i32, seen: &mut HashSet<*const c_void>) {
    unsafe {
        if ffi::lua_checkstack(state, 4) == 0 {
            return;
        }
        let idx = ffi::lua_absindex(state, idx);
        match ffi::lua_type(state, idx) {
            ffi::LUA_TTABLE => {
                if !seen.insert(ffi::lua_topointer(state, idx)) {
                    return;
                }
                ffi::lua_setreadonly(state, idx, 1);
                ffi::lua_pushnil(state);
                while ffi::lua_next(state, idx) != 0 {
                    let top = ffi::lua_gettop(state);
                    freeze_at(state, top - 1, seen);
                    freeze_at(state, top, seen);
                    ffi::lua_pop(state, 1);
                }
                if ffi::lua_getmetatable(state, idx) != 0 {
                    freeze_at(state, -1, seen);
                    ffi::lua_pop(state, 1);
                }
            }
            ffi::LUA_TFUNCTION => {
                if ffi::lua_iscfunction(state, idx) != 0 || !seen.insert(ffi::lua_topointer(state, idx)) {
                    return;
                }
                let mut n = 1;
                while !ffi::lua_getupvalue(state, idx, n).is_null() {
                    freeze_at(state, -1, seen);
                    ffi::lua_pop(state, 1);
                    n += 1;
                }
            }
            _ => {}
        }
    }
}
