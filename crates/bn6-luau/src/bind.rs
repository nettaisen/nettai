//! The content API as Luau sees it: handles for objects, sprites,
//! collision, content state and navis, the `Vec3` value type, and the
//! `battle` and `int` libraries (declared for editors in
//! content/bn6/core.d.luau).
//!
//! Every call goes straight to the engine through [`CoreApi`]; nothing is
//! cached or deferred, so later objects in the same tick see a change at
//! once, as in the game.
//!
//! Numbers: Luau numbers are doubles. Integer arithmetic on doubles is exact
//! up to 2^53, far beyond the engine's 32-bit values, so scripts compute
//! with plain numbers and the *engine* applies the game's integer rules when
//! a value is stored: every number crossing into the engine must be an
//! integer (a fraction, NaN or infinity is an error), and a field store
//! wraps it to the field's width (`u8`, `u16`, `i32`, ...). Positions are
//! `Vec3` values held as three `i32` on the Rust side, with wrapping `+`
//! and `-`. The `int` library covers what plain arithmetic can't express
//! exactly (truncating division, arithmetic shifts, 32-bit products).

use std::cell::Cell;
use std::marker::PhantomData;
use std::ptr::NonNull;

use bn6_content_api::{
    ActorField, ApiError, CollisionField, ContentState, CoreApi, FieldType, Lifecycle, Manifest, NaviStat,
    ObjectField, ObjectRef, PanelPos, Pool, SpriteField, SpriteId, StatusFlag, Value, Vec3,
};
use mlua::{AnyUserData, Lua, MetaMethod, UserData, UserDataFields, UserDataMethods, Value as LuaValue};

type Ctx = (NonNull<dyn CoreApi>, NonNull<Manifest>);

thread_local! {
    /// The engine and manifest of the update running on this thread.
    static CTX: Cell<Option<Ctx>> = const { Cell::new(None) };
}

/// Makes the engine reachable from script callbacks for one update call.
pub struct Enter<'a> {
    prev: Option<Ctx>,
    _borrow: PhantomData<&'a mut ()>,
}

impl<'a> Enter<'a> {
    pub fn new(api: &'a mut dyn CoreApi, manifest: &'a Manifest) -> Enter<'a> {
        let api: NonNull<dyn CoreApi + 'a> = NonNull::from(api);
        // SAFETY: only the lifetime is erased. The pointer is reachable
        // (through CTX) only while this guard lives, and the guard borrows
        // `api` exclusively for its whole life.
        let api: NonNull<dyn CoreApi + 'static> = unsafe { std::mem::transmute(api) };
        let prev = CTX.with(|c| c.replace(Some((api, NonNull::from(manifest)))));
        Enter { prev, _borrow: PhantomData }
    }
}

impl Drop for Enter<'_> {
    fn drop(&mut self) {
        CTX.with(|c| c.set(self.prev));
    }
}

/// Run `f` against the engine of the running update.
fn with<R>(f: impl FnOnce(&mut dyn CoreApi, &Manifest) -> mlua::Result<R>) -> mlua::Result<R> {
    let (api, manifest) = CTX
        .with(|c| c.get())
        .ok_or_else(|| mlua::Error::runtime("the battle is only reachable while content updates"))?;
    // SAFETY: set by a live `Enter`, which holds the exclusive borrow.
    // Callbacks don't nest `with` calls, so this is the only reference.
    f(unsafe { &mut *api.as_ptr() }, unsafe { &*manifest.as_ptr() })
}

fn api_error(e: ApiError) -> mlua::Error {
    mlua::Error::runtime(e.to_string())
}

// ---- Numbers ----------------------------------------------------------------------

/// Largest integer a double holds exactly.
const EXACT: f64 = 9_007_199_254_740_992.0;

fn exact(n: f64, what: &str) -> mlua::Result<i64> {
    if n.fract() == 0.0 && n.abs() <= EXACT {
        Ok(n as i64)
    } else {
        Err(mlua::Error::runtime(format!("{what}: {n} is not an integer (engine values are integers; use // or the int library)")))
    }
}

fn int(v: &LuaValue, what: &str) -> mlua::Result<i64> {
    match v {
        LuaValue::Number(n) => exact(*n, what),
        LuaValue::Integer(i) => Ok(*i),
        v => Err(mlua::Error::runtime(format!("{what}: expected an integer, got {}", v.type_name()))),
    }
}

fn u8_arg(v: LuaValue, what: &str) -> mlua::Result<u8> {
    Ok(int(&v, what)? as u8)
}

// ---- Values -------------------------------------------------------------------------

/// A Luau value going into a field of type `ty`.
fn to_api(v: LuaValue, ty: &FieldType, what: &str) -> mlua::Result<Value> {
    Ok(match v {
        LuaValue::Nil => Value::Nil,
        LuaValue::Boolean(b) => Value::Bool(b),
        LuaValue::String(s) => match ty {
            FieldType::Enum(names) => {
                let s = s.to_str()?;
                let i = names.iter().position(|n| *n == *s).ok_or_else(|| {
                    mlua::Error::runtime(format!("{what}: {:?} is not one of {}", &*s, names.join(", ")))
                })?;
                Value::Int(i as i64)
            }
            _ => return Err(mlua::Error::runtime(format!("{what}: expected {ty}, got a string"))),
        },
        LuaValue::UserData(ud) => {
            if let Ok(o) = ud.borrow::<Object>() {
                Value::Object(o.0)
            } else if let Ok(p) = ud.borrow::<LVec3>() {
                Value::Vec3(p.0)
            } else {
                return Err(mlua::Error::runtime(format!("{what}: expected {ty}, got {}", ud.type_name().map(|s| s.to_string_lossy()).unwrap_or_default())));
            }
        }
        v @ (LuaValue::Number(_) | LuaValue::Integer(_)) => Value::Int(int(&v, what)?),
        v => return Err(mlua::Error::runtime(format!("{what}: expected {ty}, got {}", v.type_name()))),
    })
}

/// An engine value coming out of a field of type `ty`.
fn from_api(lua: &Lua, v: Value, ty: &FieldType) -> mlua::Result<LuaValue> {
    Ok(match v {
        Value::Nil => LuaValue::Nil,
        Value::Bool(b) => LuaValue::Boolean(b),
        Value::Int(i) => match ty {
            FieldType::Enum(names) => LuaValue::String(lua.create_string(&names[i as usize])?),
            _ => LuaValue::Number(i as f64),
        },
        Value::Object(o) => LuaValue::UserData(lua.create_userdata(Object(o))?),
        Value::Vec3(p) => LuaValue::UserData(lua.create_userdata(LVec3(p))?),
    })
}

fn object_value(lua: &Lua, o: Option<ObjectRef>) -> mlua::Result<LuaValue> {
    o.map_or(Ok(LuaValue::Nil), |o| Ok(LuaValue::UserData(lua.create_userdata(Object(o))?)))
}

// ---- Object -------------------------------------------------------------------------

/// A handle to an object slot (`Object` in core.d.luau). Like the game's
/// pointers, it can outlive the object.
#[derive(Clone, Copy)]
pub struct Object(pub ObjectRef);

impl UserData for Object {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        for &f in ObjectField::ALL {
            fields.add_field_method_get(f.name(), move |lua, this| {
                let v = with(|api, _| Ok(api.get(this.0, f)))?;
                from_api(lua, v, &f.ty())
            });
            if f.writable() {
                fields.add_field_method_set(f.name(), move |_, this, v: LuaValue| {
                    let v = to_api(v, &f.ty(), f.name())?;
                    with(|api, _| api.set(this.0, f, v).map_err(api_error))
                });
            }
        }
        for &f in ActorField::ALL {
            fields.add_field_method_get(f.name(), move |lua, this| {
                let v = with(|api, _| api.actor_get(this.0, f).map_err(api_error))?;
                from_api(lua, v, &f.ty())
            });
            if f.writable() {
                fields.add_field_method_set(f.name(), move |_, this, v: LuaValue| {
                    let v = to_api(v, &f.ty(), f.name())?;
                    with(|api, _| api.actor_set(this.0, f, v).map_err(api_error))
                });
            }
        }
        fields.add_field_method_get("pool", |_, this| Ok(this.0.pool.name()));
        fields.add_field_method_get("lifecycle", |_, this| with(|api, _| Ok(api.lifecycle(this.0).name())));
        fields.add_field_method_get("facing", |_, this| with(|api, _| Ok(api.facing(this.0))));
        fields.add_field_method_get("sprite", |_, this| Ok(Sprite(this.0)));
        fields.add_field_method_get("collision", |_, this| Ok(Collision(this.0)));
        fields.add_field_method_get("state", |_, this| Ok(State { owner: this.0, action: false }));
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::Eq, |_, this, other: AnyUserData| {
            Ok(other.borrow::<Object>().is_ok_and(|o| o.0 == this.0))
        });
        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| {
            Ok(format!("Object({} {})", this.0.pool.name(), this.0.slot))
        });
        methods.add_method("param", |_, this, n: LuaValue| {
            let n = int(&n, "param")?;
            if !(1..=4).contains(&n) {
                return Err(mlua::Error::runtime(format!("param {n}: objects have params 1 to 4")));
            }
            with(|api, _| Ok(api.param(this.0, n as usize - 1)))
        });
        methods.add_method("set_lifecycle", |_, this, name: mlua::LuaString| {
            let name = name.to_str()?;
            let l = Lifecycle::from_name(&name)
                .ok_or_else(|| mlua::Error::runtime(format!("{:?} is not a lifecycle state", &*name)))?;
            with(|api, _| Ok(api.set_lifecycle(this.0, l)))
        });
        methods.add_method("free", |_, this, ()| with(|api, _| Ok(api.free(this.0))));
        methods.add_method("destroy", |_, this, ()| with(|api, _| Ok(api.destroy(this.0))));
        methods.add_method("set_animation", |_, this, anim: LuaValue| {
            let anim = u8_arg(anim, "anim")?;
            with(|api, _| Ok(api.set_animation(this.0, anim)))
        });
        methods.add_method("update_sprite", |_, this, ()| with(|api, _| Ok(api.update_sprite(this.0))));
        methods.add_method("attach_point", |_, this, n: LuaValue| {
            let n = u8_arg(n, "attach point")?;
            with(|api, _| Ok(api.attach_point(this.0, n)))
        });
        methods.add_method("status", |_, this, name: mlua::LuaString| {
            let flag = status_flag(&name)?;
            with(|api, _| api.status(this.0, flag).map_err(api_error))
        });
        methods.add_method("set_status", |_, this, (name, on): (mlua::LuaString, bool)| {
            let flag = status_flag(&name)?;
            with(|api, _| api.set_status(this.0, flag, on).map_err(api_error))
        });
        methods.add_method("open_counter_window", |_, this, ()| with(|api, _| Ok(api.open_counter_window(this.0))));
        methods.add_method("check_reactive_abort", |_, this, ()| {
            with(|api, _| Ok(api.check_reactive_abort(this.0)))
        });
        methods.add_method("exit_attack", |_, this, ()| with(|api, _| Ok(api.exit_attack(this.0))));
        methods.add_method("create_collision", |_, this, ()| with(|api, _| Ok(api.create_collision(this.0))));
        methods.add_method(
            "setup_collision",
            |_, this, (self_type, target_type, hit_mod): (LuaValue, LuaValue, LuaValue)| {
                let (s, t, h) = (u8_arg(self_type, "self type")?, u8_arg(target_type, "target type")?, u8_arg(hit_mod, "hit mod")?);
                with(|api, _| Ok(api.setup_collision(this.0, s, t, h)))
            },
        );
        methods.add_method("hit_spark", |_, this, ()| with(|api, _| Ok(api.hit_spark(this.0))));
    }
}

fn status_flag(name: &mlua::LuaString) -> mlua::Result<StatusFlag> {
    let name = name.to_str()?;
    StatusFlag::from_name(&name).ok_or_else(|| mlua::Error::runtime(format!("{:?} is not a status flag", &*name)))
}

// ---- Sprite, collision, state, navi --------------------------------------------------------

/// An object's sprite (`Sprite` in core.d.luau).
#[derive(Clone, Copy)]
pub struct Sprite(ObjectRef);

impl UserData for Sprite {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        for &f in SpriteField::ALL {
            fields.add_field_method_get(f.name(), move |lua, this| {
                let v = with(|api, _| Ok(api.sprite_get(this.0, f)))?;
                from_api(lua, v, &f.ty())
            });
            fields.add_field_method_set(f.name(), move |_, this, v: LuaValue| {
                let v = to_api(v, &f.ty(), f.name())?;
                with(|api, _| api.sprite_set(this.0, f, v).map_err(api_error))
            });
        }
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("load", |_, this, (category, index): (LuaValue, LuaValue)| {
            let id = SpriteId { category: u8_arg(category, "sprite category")?, index: u8_arg(index, "sprite index")? };
            with(|api, _| Ok(api.sprite_load(this.0, id)))
        });
        methods.add_method("set_animation", |_, this, anim: LuaValue| {
            let anim = u8_arg(anim, "anim")?;
            with(|api, _| Ok(api.sprite_set_animation(this.0, anim)))
        });
        methods.add_method("step", |_, this, ()| with(|api, _| Ok(api.sprite_step(this.0))));
        methods.add_method("set_flip", |_, this, flip: LuaValue| {
            let flip = u8_arg(flip, "flip")?;
            with(|api, _| {
                api.sprite_set(this.0, SpriteField::HFlip, Value::Bool(flip & 1 != 0)).map_err(api_error)?;
                api.sprite_set(this.0, SpriteField::VFlip, Value::Bool(flip & 2 != 0)).map_err(api_error)
            })
        });
    }
}

/// An object's collision registration (`Collision` in core.d.luau).
#[derive(Clone, Copy)]
pub struct Collision(ObjectRef);

impl UserData for Collision {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        for &f in CollisionField::ALL {
            fields.add_field_method_get(f.name(), move |lua, this| {
                let v = with(|api, _| api.collision_get(this.0, f).map_err(api_error))?;
                from_api(lua, v, &f.ty())
            });
            if f.writable() {
                fields.add_field_method_set(f.name(), move |_, this, v: LuaValue| {
                    let v = to_api(v, &f.ty(), f.name())?;
                    with(|api, _| api.collision_set(this.0, f, v).map_err(api_error))
                });
            }
        }
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("present", |_, this, ()| with(|api, _| Ok(api.present_collision(this.0))));
        methods.add_method("remove", |_, this, ()| with(|api, _| Ok(api.remove_collision(this.0))));
        methods.add_method("free", |_, this, ()| with(|api, _| Ok(api.free_collision(this.0))));
    }
}

/// The content-declared state of an object (`me.state`) or of the action
/// it runs (the second argument of an action's `update`). Fields are the
/// ones the kind's `state` table declares, typed as declared.
#[derive(Clone, Copy)]
pub struct State {
    pub owner: ObjectRef,
    pub action: bool,
}

impl State {
    fn with_state<R>(
        self,
        key: &str,
        f: impl FnOnce(&mut ContentState, &bn6_content_api::Schema, usize) -> mlua::Result<R>,
    ) -> mlua::Result<R> {
        with(|api, manifest| {
            let s = if self.action {
                api.action_state_mut(self.owner).map_err(api_error)?
            } else {
                api.state_mut(self.owner).ok_or_else(|| api_error(ApiError::NoState(self.owner)))?
            };
            let schema = manifest.schema(s.id());
            let i = schema.index_of(key).ok_or_else(|| {
                let names: Vec<&str> = schema.fields().iter().map(|f| f.name.as_str()).collect();
                mlua::Error::runtime(format!("state has no field `{key}` (it declares {})", names.join(", ")))
            })?;
            f(s, schema, i)
        })
    }
}

impl UserData for State {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::Index, |lua, this, key: mlua::LuaString| {
            let key = key.to_str()?;
            let (v, ty) = this.with_state(&key, |s, schema, i| Ok((s.get(i).load(), schema.field(i).ty.clone())))?;
            from_api(lua, v, &ty)
        });
        methods.add_meta_method(MetaMethod::NewIndex, |_, this, (key, v): (mlua::LuaString, LuaValue)| {
            let key = key.to_str()?;
            this.with_state(&key, |s, schema, i| {
                let v = to_api(v, &schema.field(i).ty, &key)?;
                s.set(schema, i, v).map_err(|e| mlua::Error::runtime(format!("state field `{}`: {e}", &*key)))
            })
        });
    }
}

/// A side's navi stats (`battle.navi(side)`).
#[derive(Clone, Copy)]
pub struct Navi(u8);

impl UserData for Navi {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        for &f in NaviStat::ALL {
            fields.add_field_method_get(f.name(), move |lua, this| {
                let v = with(|api, _| Ok(api.navi_stat(this.0, f)))?;
                from_api(lua, v, &f.ty())
            });
        }
    }
}

// ---- Vec3 ------------------------------------------------------------------------------------

/// A 16.16 fixed-point vector: three `i32`, wrapping on `+` and `-`.
/// Immutable, like a number.
#[derive(Clone, Copy)]
pub struct LVec3(pub Vec3);

impl UserData for LVec3 {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("x", |_, this| Ok(this.0.x));
        fields.add_field_method_get("y", |_, this| Ok(this.0.y));
        fields.add_field_method_get("z", |_, this| Ok(this.0.z));
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_function(MetaMethod::Add, |_, (a, b): (mlua::UserDataRef<LVec3>, mlua::UserDataRef<LVec3>)| {
            Ok(LVec3(a.0.wrapping_add(b.0)))
        });
        methods.add_meta_function(MetaMethod::Sub, |_, (a, b): (mlua::UserDataRef<LVec3>, mlua::UserDataRef<LVec3>)| {
            Ok(LVec3(a.0.wrapping_sub(b.0)))
        });
        methods.add_meta_method(MetaMethod::Eq, |_, this, other: AnyUserData| {
            Ok(other.borrow::<LVec3>().is_ok_and(|o| o.0 == this.0))
        });
        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| {
            Ok(format!("Vec3({:#x}, {:#x}, {:#x})", this.0.x, this.0.y, this.0.z))
        });
    }
}

// ---- Libraries -------------------------------------------------------------------------------------

/// Install `battle`, `Vec3` and `int`.
pub fn install(lua: &Lua) -> mlua::Result<()> {
    let g = lua.globals();

    let battle = lua.create_table()?;
    battle.set("time_stop", lua.create_function(|_, ()| with(|api, _| Ok(api.is_time_stop())))?)?;
    battle.set("paused", lua.create_function(|_, ()| with(|api, _| Ok(api.is_paused())))?)?;
    battle.set(
        "play_sound",
        lua.create_function(|_, id: LuaValue| {
            let id = int(&id, "sound")? as u16;
            with(|api, _| Ok(api.play_sound(id)))
        })?,
    )?;
    battle.set("navi", lua.create_function(|_, side: LuaValue| Ok(Navi(u8_arg(side, "side")? & 1)))?)?;
    battle.set(
        "spawn",
        lua.create_function(|lua, (pool, index, pos, params): (mlua::LuaString, LuaValue, Option<mlua::UserDataRef<LVec3>>, Option<mlua::Table>)| {
            let pool = pool.to_str()?;
            let pool = Pool::from_name(&pool).ok_or_else(|| mlua::Error::runtime(format!("{:?} is not a pool", &*pool)))?;
            let index = u8_arg(index, "kind index")?;
            let pos = pos.map_or(Vec3::default(), |p| p.0);
            let mut p = [0u8; 4];
            if let Some(t) = params {
                for (i, slot) in p.iter_mut().enumerate() {
                    *slot = u8_arg(t.raw_get::<LuaValue>(i + 1)?, "spawn param").or_else(|e| {
                        if t.raw_get::<LuaValue>(i + 1)?.is_nil() { Ok(0) } else { Err(e) }
                    })?;
                }
            }
            let o = with(|api, _| Ok(api.spawn(pool, index, pos, p)))?;
            object_value(lua, o)
        })?,
    )?;
    battle.set(
        "panel_valid",
        lua.create_function(|_, (x, y): (LuaValue, LuaValue)| {
            let p = PanelPos { x: u8_arg(x, "x")?, y: u8_arg(y, "y")? };
            with(|api, _| Ok(api.panel_valid(p)))
        })?,
    )?;
    battle.set(
        "panel_center",
        lua.create_function(|_, (x, y): (LuaValue, LuaValue)| {
            let p = PanelPos { x: u8_arg(x, "x")?, y: u8_arg(y, "y")? };
            with(|api, _| Ok(api.panel_center(p)))
        })?,
    )?;
    g.set("battle", battle)?;

    let vec3 = lua.create_table()?;
    let wrap = |v: LuaValue, what| -> mlua::Result<i32> { Ok(int(&v, what)? as i32) };
    vec3.set(
        "new",
        lua.create_function(move |_, (x, y, z): (LuaValue, LuaValue, LuaValue)| {
            Ok(LVec3(Vec3 { x: wrap(x, "x")?, y: wrap(y, "y")?, z: wrap(z, "z")? }))
        })?,
    )?;
    vec3.set(
        "px",
        lua.create_function(move |_, (x, y, z): (LuaValue, LuaValue, LuaValue)| {
            Ok(LVec3(Vec3 { x: wrap(x, "x")? << 16, y: wrap(y, "y")? << 16, z: wrap(z, "z")? << 16 }))
        })?,
    )?;
    vec3.set("zero", LVec3(Vec3::default()))?;
    g.set("Vec3", vec3)?;

    let lib = lua.create_table()?;
    macro_rules! wrapper {
        ($($name:literal => $t:ty),*) => {$(
            lib.set($name, lua.create_function(|_, v: LuaValue| Ok(int(&v, $name)? as $t))?)?;
        )*};
    }
    wrapper!("u8" => u8, "u16" => u16, "u32" => u32, "i8" => i8, "i16" => i16, "i32" => i32);
    lib.set(
        "asr",
        lua.create_function(|_, (x, n): (LuaValue, LuaValue)| {
            let (x, n) = (int(&x, "asr")?, int(&n, "asr")?);
            Ok((x as i32) >> (n & 31))
        })?,
    )?;
    lib.set(
        "shl",
        lua.create_function(|_, (x, n): (LuaValue, LuaValue)| {
            let (x, n) = (int(&x, "shl")?, int(&n, "shl")?);
            Ok((x as i32).wrapping_shl(n as u32 & 31))
        })?,
    )?;
    lib.set(
        "lsr",
        lua.create_function(|_, (x, n): (LuaValue, LuaValue)| {
            let (x, n) = (int(&x, "lsr")?, int(&n, "lsr")?);
            Ok((x as u32) >> (n & 31))
        })?,
    )?;
    lib.set(
        "tdiv",
        lua.create_function(|_, (a, b): (LuaValue, LuaValue)| {
            let (a, b) = (int(&a, "tdiv")?, int(&b, "tdiv")?);
            if b == 0 {
                return Err(mlua::Error::runtime("int.tdiv: division by zero"));
            }
            Ok(a / b)
        })?,
    )?;
    lib.set(
        "tmod",
        lua.create_function(|_, (a, b): (LuaValue, LuaValue)| {
            let (a, b) = (int(&a, "tmod")?, int(&b, "tmod")?);
            if b == 0 {
                return Err(mlua::Error::runtime("int.tmod: division by zero"));
            }
            Ok(a % b)
        })?,
    )?;
    lib.set(
        "mul32",
        lua.create_function(|_, (a, b): (LuaValue, LuaValue)| {
            let (a, b) = (int(&a, "mul32")?, int(&b, "mul32")?);
            Ok((a as i32).wrapping_mul(b as i32))
        })?,
    )?;
    g.set("int", lib)?;
    Ok(())
}

/// Wrap the object being updated as a script value.
pub fn object(lua: &Lua, o: ObjectRef) -> mlua::Result<AnyUserData> {
    lua.create_userdata(Object(o))
}

/// Wrap an action's state as a script value.
pub fn action_state(lua: &Lua, o: ObjectRef) -> mlua::Result<AnyUserData> {
    lua.create_userdata(State { owner: o, action: true })
}
