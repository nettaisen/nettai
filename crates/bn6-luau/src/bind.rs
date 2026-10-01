//! The content API as Luau sees it: handles for objects, sprites,
//! collision, content state and navi stats, the `Vec3` value type, and the
//! libraries `battle`, `field`, `dimming`, `navi_chip` and `int` (declared
//! for editors in content/bn6/core.d.luau).
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
    ACTOR_TYPES, ActorField, ApiError, BattleInfo, CollisionField, ContentState, CoreApi, DimmingStep, FieldType,
    HitboxSpec, HookCall, Key, Lifecycle, LinkedChip, NaviStat, NaviState, OVERLAY_STEPPINGS, ObjectField, ObstacleAction,
    AssetKind, SpawnAt,
    ObstacleCrush, ObstacleRequest, PANEL_TYPES, Pad, PanelPos, Pool, Registry, RequestFlag, SpriteField, SpriteId,
    StateId, StatusFlag, StatusTimer, Value, Vec3,
};
use bn6_content_api::ObjectRef;
use bn6_content_api::{CollisionHandle, EffectHandle, RegionHandle, SparkHandle};

use crate::Bound;
// Subtypes 8, 17, 18 (Wind, Anubis, Otenko) and the obstacle framework.
use bn6_content_api::{ObstacleHold, ObstaclePush, WindSource};
use mlua::{AnyUserData, Lua, MetaMethod, UserData, UserDataFields, UserDataMethods, Value as LuaValue};

type Ctx = (NonNull<dyn CoreApi>, NonNull<Bound>);

thread_local! {
    /// The engine, and what the binding reads, of the call running on this
    /// thread.
    static CTX: Cell<Option<Ctx>> = const { Cell::new(None) };
}

/// Makes the engine reachable from script callbacks for one call.
pub struct Enter<'a> {
    prev: Option<Ctx>,
    _borrow: PhantomData<&'a mut ()>,
}

impl<'a> Enter<'a> {
    pub fn new(api: &'a mut dyn CoreApi, bound: &'a Bound) -> Enter<'a> {
        let api: NonNull<dyn CoreApi + 'a> = NonNull::from(api);
        // SAFETY: only the lifetime is erased. The pointer is reachable
        // (through CTX) only while this guard lives, and the guard borrows
        // `api` exclusively for its whole life.
        let api: NonNull<dyn CoreApi + 'static> = unsafe { std::mem::transmute(api) };
        let prev = CTX.with(|c| c.replace(Some((api, NonNull::from(bound)))));
        Enter { prev, _borrow: PhantomData }
    }
}

impl Drop for Enter<'_> {
    fn drop(&mut self) {
        CTX.with(|c| c.set(self.prev));
    }
}

/// Run `f` against the engine of the running call.
fn with<R>(f: impl FnOnce(&mut dyn CoreApi, &Bound) -> mlua::Result<R>) -> mlua::Result<R> {
    let (api, bound) = CTX
        .with(|c| c.get())
        .ok_or_else(|| mlua::Error::runtime("the battle is only reachable while content runs"))?;
    // SAFETY: set by a live `Enter`, which holds the exclusive borrow.
    // Callbacks don't nest `with` calls, so this is the only reference.
    f(unsafe { &mut *api.as_ptr() }, unsafe { &*bound.as_ptr() })
}

/// Read what the binding knows (definitions, assets) during the running
/// call.
fn bound<R>(f: impl FnOnce(&Bound) -> mlua::Result<R>) -> mlua::Result<R> {
    let (_, bound) = CTX
        .with(|c| c.get())
        .ok_or_else(|| mlua::Error::runtime("the battle is only reachable while content runs"))?;
    // SAFETY: set by a live `Enter`; the binding only ever reads it.
    f(unsafe { &*bound.as_ptr() })
}

/// A definition of `registry` (a region, a collision type, an effect, a
/// spark): its handle.
fn def_arg(b: &Bound, v: &LuaValue, registry: Registry, what: &str) -> mlua::Result<u16> {
    match b.def(v) {
        Some((r, h)) if r == registry => Ok(h),
        Some((r, _)) => Err(mlua::Error::runtime(format!("{what}: a {r} is not a {registry}"))),
        None => Err(mlua::Error::runtime(format!("{what}: expected a {registry} definition, got {}", v.type_name()))),
    }
}

/// A record definition's handle (an absorbed obstacle's look, say).
fn record_arg(b: &Bound, v: &LuaValue, what: &str) -> mlua::Result<u16> {
    match b.def(v) {
        Some((Registry::Record, h)) => Ok(h),
        Some((r, _)) => Err(mlua::Error::runtime(format!("{what}: a {r} is not a record"))),
        None => Err(mlua::Error::runtime(format!("{what}: expected a record definition, got {}", v.type_name()))),
    }
}

/// A sound asset, or a sound number (deprecated).
fn sound_arg(v: LuaValue) -> mlua::Result<u16> {
    if let LuaValue::Table(_) = v {
        return bound(|b| match b.asset(&v) {
            Some((AssetKind::Sound, h)) => b.with_names(|n| n.sound(h)).ok_or_else(|| mlua::Error::runtime("no such sound")),
            _ => Err(mlua::Error::runtime("expected a sound (asset.sound)")),
        });
    }
    u16_arg(v, "sound")
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
        Err(mlua::Error::runtime(format!(
            "{what}: {n} is not an integer (engine values are integers; use // or the int library)"
        )))
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

fn u16_arg(v: LuaValue, what: &str) -> mlua::Result<u16> {
    Ok(int(&v, what)? as u16)
}

fn panel(x: LuaValue, y: LuaValue) -> mlua::Result<PanelPos> {
    Ok(PanelPos { x: u8_arg(x, "panel x")?, y: u8_arg(y, "panel y")? })
}

/// A name from a set of names (a flag, a key, a lifecycle state...).
fn named<T>(s: &mlua::LuaString, what: &str, parse: impl Fn(&str) -> Option<T>) -> mlua::Result<T> {
    let s = s.to_str()?;
    parse(&s).ok_or_else(|| mlua::Error::runtime(format!("{:?} is not a {what}", &*s)))
}

// ---- Values -------------------------------------------------------------------------

/// A Luau value going into a field of type `ty`.
fn to_api(v: LuaValue, ty: &FieldType, what: &str) -> mlua::Result<Value> {
    Ok(match v {
        LuaValue::Nil => Value::Nil,
        LuaValue::Table(_) => bound(|b| {
            if let Some((registry, h)) = b.def(&v) {
                if let FieldType::Ref(Registry::Record, Some(want)) = ty
                    && registry == Registry::Record
                    && b.record_type(h) != Some(want.as_str())
                {
                    return Err(mlua::Error::runtime(format!(
                        "{what}: expected a record:{want}, got a record:{}",
                        b.record_type(h).unwrap_or("?")
                    )));
                }
                return Ok(Value::Def(registry, h));
            }
            if let Some((kind, h)) = b.asset(&v) {
                return Ok(Value::Asset(kind, h));
            }
            Err(mlua::Error::runtime(format!("{what}: expected {ty}, got a table that is no definition or asset")))
        })?,
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
                return Err(mlua::Error::runtime(format!(
                    "{what}: expected {ty}, got {}",
                    ud.type_name().map(|s| s.to_string_lossy()).unwrap_or_default()
                )));
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
        Value::Def(r, h) => LuaValue::Table(bound(|b| b.def_value(r, h))?),
        Value::Asset(k, h) => LuaValue::Table(bound(|b| b.asset_value(lua, k, h))?),
    })
}

fn object_value(lua: &Lua, o: Option<ObjectRef>) -> mlua::Result<LuaValue> {
    o.map_or(Ok(LuaValue::Nil), |o| Ok(LuaValue::UserData(lua.create_userdata(Object(o))?)))
}

fn object_arg(v: &LuaValue, what: &str) -> mlua::Result<Option<ObjectRef>> {
    match v {
        LuaValue::Nil => Ok(None),
        LuaValue::UserData(ud) => ud
            .borrow::<Object>()
            .map(|o| Some(o.0))
            .map_err(|_| mlua::Error::runtime(format!("{what}: expected an Object"))),
        v => Err(mlua::Error::runtime(format!("{what}: expected an Object, got {}", v.type_name()))),
    }
}

/// Up to four spawn parameters from a table (missing ones are 0).
fn params(t: Option<mlua::Table>, what: &str) -> mlua::Result<[u8; 4]> {
    let mut p = [0u8; 4];
    if let Some(t) = t {
        for (i, slot) in p.iter_mut().enumerate() {
            let v = t.raw_get::<LuaValue>(i + 1)?;
            if !v.is_nil() {
                *slot = u8_arg(v, what)?;
            }
        }
    }
    Ok(p)
}

fn params_table(lua: &Lua, p: [u8; 4]) -> mlua::Result<mlua::Table> {
    lua.create_sequence_from(p.iter().map(|&b| b as f64))
}

/// A sprite: an asset (`asset.sprite("bomb")`), or (deprecated) `"CC-II"`
/// in hex or its category and index as numbers.
fn sprite_id(a: LuaValue, b: Option<LuaValue>) -> mlua::Result<SpriteId> {
    if let LuaValue::Table(_) = a {
        return bound(|bd| match bd.asset(&a) {
            Some((AssetKind::Sprite, h)) => bd.with_names(|n| n.sprite(h)).ok_or_else(|| mlua::Error::runtime("no such sprite")),
            _ => Err(mlua::Error::runtime("expected a sprite (asset.sprite)")),
        });
    }
    match (a, b) {
        (LuaValue::String(s), None) => s.to_str()?.parse().map_err(mlua::Error::runtime),
        (c, Some(i)) => Ok(SpriteId { category: u8_arg(c, "sprite category")?, index: u8_arg(i, "sprite index")? }),
        (v, None) => Err(mlua::Error::runtime(format!("a sprite is \"CC-II\" or two numbers, got {}", v.type_name()))),
    }
}

// ---- Object -------------------------------------------------------------------------

/// A handle to an object slot (`Object` in core.d.luau). Like the game's
/// pointers, it can outlive the object.
#[derive(Clone, Copy)]
pub struct Object(pub ObjectRef);

impl UserData for Object {
    fn register(registry: &mut mlua::UserDataRegistry<Self>) {
        Self::add_fields(registry);
        Self::add_methods(registry);
        // `obj:method()` resolves through `__namecall`, skipping `__index`.
        registry.enable_namecall();
    }

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
        fields.add_field_method_get("kind", |lua, this| {
            let kind = with(|api, _| Ok(api.object_kind(this.0)))?;
            match kind {
                Some(h) => from_api(lua, Value::Def(Registry::Kind, h), &FieldType::Ref(Registry::Kind, None)),
                None => Ok(LuaValue::Nil),
            }
        });
        fields.add_field_method_get("lifecycle", |_, this| with(|api, _| Ok(api.lifecycle(this.0).name())));
        fields.add_field_method_set("lifecycle", |_, this, name: mlua::LuaString| {
            let l = named(&name, "lifecycle state", Lifecycle::from_name)?;
            with(|api, _| Ok(api.set_lifecycle_only(this.0, l)))
        });
        fields.add_field_method_get("facing", |_, this| with(|api, _| Ok(api.facing(this.0))));
        fields.add_field_method_get("sprite", |_, this| Ok(Sprite(this.0)));
        fields.add_field_method_get("collision", |_, this| Ok(Collision(this.0)));
        fields.add_field_method_get("state", |_, this| Ok(State { owner: this.0, action: false, of_action: None }));
        fields.add_field_method_get("attack_state", |_, this| {
            Ok(State { owner: this.0, action: true, of_action: None })
        });
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::Eq, |_, this, other: AnyUserData| {
            Ok(other.borrow::<Object>().is_ok_and(|o| o.0 == this.0))
        });
        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| {
            Ok(format!("Object({} {})", this.0.pool.name(), this.0.slot))
        });

        // Lifecycle and parameters.
        methods.add_method("param", |_, this, n: LuaValue| {
            let n = param_index(n)?;
            with(|api, _| Ok(api.param(this.0, n)))
        });
        methods.add_method("set_param", |_, this, (n, v): (LuaValue, LuaValue)| {
            let (n, v) = (param_index(n)?, u8_arg(v, "param")?);
            with(|api, _| Ok(api.set_param(this.0, n, v)))
        });
        methods.add_method("set_lifecycle", |_, this, name: mlua::LuaString| {
            let l = named(&name, "lifecycle state", Lifecycle::from_name)?;
            with(|api, _| Ok(api.set_lifecycle(this.0, l)))
        });
        methods.add_method("set_action", |_, this, a: LuaValue| {
            let a = u8_arg(a, "action")?;
            with(|api, _| Ok(api.set_action(this.0, a)))
        });
        methods.add_method("free", |_, this, ()| with(|api, _| Ok(api.free(this.0))));
        methods.add_method("destroy", |_, this, ()| with(|api, _| Ok(api.destroy(this.0))));
        methods.add_method("death_hook", |_, this, name_id: LuaValue| {
            let name_id = u16_arg(name_id, "NameID")?;
            with(|api, _| Ok(api.death_hook(this.0, name_id)))
        });
        methods.add_method(
            "add_navi_parts",
            |_, this, (actor_type, ai, arg): (mlua::LuaString, LuaValue, LuaValue)| {
                let t = named(&actor_type, "actor type", |s| ACTOR_TYPES.iter().position(|n| *n == s))? as u8;
                let (ai, arg) = (u8_arg(ai, "AI index")?, u8_arg(arg, "arg")?);
                with(|api, _| Ok(api.add_navi_parts(this.0, t, ai, arg)))
            },
        );
        methods.add_method("remove_navi_parts", |_, this, (actor_type, ai): (mlua::LuaString, LuaValue)| {
            let t = named(&actor_type, "actor type", |s| ACTOR_TYPES.iter().position(|n| *n == s))? as u8;
            let ai = u8_arg(ai, "AI index")?;
            with(|api, _| Ok(api.remove_navi_parts(this.0, t, ai)))
        });
        methods.add_method(
            "add_parts_of",
            |_, this, (owner, keep, stepping): (mlua::UserDataRef<Object>, Option<bool>, Option<bool>)| {
                let owner = owner.0;
                with(|api, _| Ok(api.add_parts_of(this.0, owner, keep.unwrap_or(false), stepping.unwrap_or(false))))
            },
        );
        methods.add_method("remove_parts_of", |_, this, owner: mlua::UserDataRef<Object>| {
            let owner = owner.0;
            with(|api, _| Ok(api.remove_parts_of(this.0, owner)))
        });

        // Sprite stepping.
        methods.add_method("set_animation", |_, this, anim: LuaValue| {
            let anim = u8_arg(anim, "anim")?;
            with(|api, _| Ok(api.set_animation(this.0, anim)))
        });
        methods.add_method("update_sprite", |_, this, ()| with(|api, _| Ok(api.update_sprite(this.0))));
        methods.add_method("update_sprite_while_dimmed", |_, this, ()| {
            with(|api, _| Ok(api.update_sprite_while_dimmed(this.0)))
        });
        methods.add_method("step_sprite", |_, this, ()| with(|api, _| Ok(api.step_sprite(this.0))));
        methods.add_method("update_sprite_even_paused", |_, this, ()| {
            with(|api, _| Ok(api.update_sprite_even_paused(this.0)))
        });
        methods.add_method("update_sprite_while_paused", |_, this, ()| {
            with(|api, _| Ok(api.update_sprite_while_paused(this.0)))
        });
        methods.add_method("load_or_step_sprite", |_, this, ()| with(|api, _| Ok(api.load_or_step_sprite(this.0))));
        methods.add_method("attach_point", |_, this, n: LuaValue| {
            let n = u8_arg(n, "attach point")?;
            with(|api, _| Ok(api.attach_point(this.0, n)))
        });

        // Position and panels.
        methods.add_method("set_coordinates_from_panel", |_, this, ()| {
            with(|api, _| Ok(api.set_coordinates_from_panel(this.0)))
        });
        methods.add_method("set_panel_from_coordinates", |_, this, ()| {
            with(|api, _| Ok(api.set_panel_from_coordinates(this.0)))
        });
        methods.add_method("update_visibility", |_, this, ()| with(|api, _| Ok(api.update_visibility(this.0))));
        methods.add_method("update_collision_panels", |_, this, ()| {
            with(|api, _| Ok(api.update_collision_panels(this.0)))
        });
        methods.add_method("snap_to_future_panel", |_, this, ()| with(|api, _| Ok(api.snap_to_future_panel(this.0))));
        methods.add_method("set_collision_panel", |_, this, ()| with(|api, _| Ok(api.set_collision_panel(this.0))));
        methods.add_method("highlight_collision_panels", |_, this, ()| {
            with(|api, _| Ok(api.highlight_collision_panels(this.0)))
        });
        methods.add_method("reserve_panel", |_, this, (x, y): (LuaValue, LuaValue)| {
            let p = panel(x, y)?;
            with(|api, _| Ok(api.reserve_panel(this.0, p)))
        });
        methods.add_method("set_header_flags", |_, this, flags: LuaValue| {
            let flags = u8_arg(flags, "header flags")?;
            with(|api, _| Ok(api.set_header_flags(this.0, flags)))
        });
        methods.add_method("unreserve_panel", |_, this, (x, y): (LuaValue, LuaValue)| {
            let p = panel(x, y)?;
            with(|api, _| Ok(api.unreserve_panel(this.0, p)))
        });
        methods.add_method("release_reservations", |_, this, ()| with(|api, _| Ok(api.release_reservations(this.0))));
        methods.add_method("can_step", |_, this, (x, y): (LuaValue, LuaValue)| {
            let p = panel(x, y)?;
            with(|api, _| Ok(api.can_step(this.0, p)))
        });
        methods.add_method("can_stand_any_side", |_, this, (x, y): (LuaValue, LuaValue)| {
            let p = panel(x, y)?;
            with(|api, _| Ok(api.can_stand_any_side(this.0, p)))
        });

        // Status.
        methods.add_method("status", |_, this, name: mlua::LuaString| {
            let flag = named(&name, "status flag", StatusFlag::from_name)?;
            with(|api, _| api.status(this.0, flag).map_err(api_error))
        });
        methods.add_method("set_status", |_, this, (name, on): (mlua::LuaString, bool)| {
            let flag = named(&name, "status flag", StatusFlag::from_name)?;
            with(|api, _| api.set_status(this.0, flag, on).map_err(api_error))
        });
        methods.add_method("clear_statuses", |_, this, ()| with(|api, _| api.clear_statuses(this.0).map_err(api_error)));
        methods.add_method("status_timer", |_, this, name: mlua::LuaString| {
            let t = named(&name, "status timer", StatusTimer::from_name)?;
            with(|api, _| api.status_timer(this.0, t).map_err(api_error))
        });
        methods.add_method("set_status_timer", |_, this, (name, v): (mlua::LuaString, LuaValue)| {
            let t = named(&name, "status timer", StatusTimer::from_name)?;
            let v = u16_arg(v, "status timer")?;
            with(|api, _| api.set_status_timer(this.0, t, v).map_err(api_error))
        });

        // Collision.
        methods.add_method("create_collision", |_, this, ()| with(|api, _| Ok(api.create_collision(this.0))));
        methods.add_method(
            "setup_collision",
            |_, this, (self_type, target_type, hit_mod): (LuaValue, LuaValue, LuaValue)| {
                let h = u8_arg(hit_mod, "hit mod")?;
                with(|api, b| {
                    let s = CollisionHandle(def_arg(b, &self_type, Registry::Collision, "setup_collision")?);
                    let t = CollisionHandle(def_arg(b, &target_type, Registry::Collision, "setup_collision")?);
                    Ok(api.setup_collision(this.0, s, t, h))
                })
            },
        );
        methods.add_method(
            "reset_collision_types",
            |_, this, (self_type, target_type, hit_mod): (LuaValue, LuaValue, LuaValue)| {
                let h = u8_arg(hit_mod, "hit mod")?;
                with(|api, b| {
                    let s = CollisionHandle(def_arg(b, &self_type, Registry::Collision, "reset_collision_types")?);
                    let t = CollisionHandle(def_arg(b, &target_type, Registry::Collision, "reset_collision_types")?);
                    Ok(api.reset_collision_types(this.0, s, t, h))
                })
            },
        );
        methods.add_method("hit_spark", |_, this, ()| with(|api, _| Ok(api.hit_spark(this.0))));
        methods.add_method("take_damage", |_, this, mode: LuaValue| {
            let mode = u8_arg(mode, "damage mode")?;
            with(|api, _| Ok(api.take_damage(this.0, mode)))
        });
        methods.add_method("name_look_is", |_, this, sprite: LuaValue| {
            let sprite = sprite_id(sprite, None)?;
            with(|api, _| api.name_look_is(this.0, sprite).map_err(api_error))
        });
        methods.add_method("raise_barrier", |_, this, t: mlua::Table| {
            let behavior: mlua::LuaString = t.raw_get("behavior")?;
            let behavior = named(&behavior, "barrier behavior", |s| {
                bn6_content_api::api::BARRIER_BEHAVIORS.iter().position(|n| *n == s)
            })? as u8;
            let byte = |k: &str| -> mlua::Result<u8> {
                u8::try_from(table_int(&t, k)?).map_err(|_| mlua::Error::runtime(format!("barrier `{k}` is not a byte")))
            };
            let timer = u16::try_from(table_int(&t, "timer")?)
                .map_err(|_| mlua::Error::runtime("barrier `timer` is not a halfword"))?;
            let spec = bn6_content_api::api::BarrierSpec {
                behavior,
                hp: byte("hp")?,
                threshold: byte("threshold")?,
                timer,
                weak_element: byte("weak_element")?,
            };
            with(|api, _| api.raise_barrier(this.0, spec).map_err(api_error))
        });

        // Navis: the attack, requests, state, buttons.
        methods.add_method("attack_param", |_, this, n: LuaValue| {
            let n = param_index(n)?;
            with(|api, _| api.attack_param(this.0, n).map_err(api_error))
        });
        methods.add_method("set_attack_param", |_, this, (n, v): (LuaValue, LuaValue)| {
            let (n, v) = (param_index(n)?, u8_arg(v, "attack param")?);
            with(|api, _| api.set_attack_param(this.0, n, v).map_err(api_error))
        });
        methods.add_method("request", |_, this, name: mlua::LuaString| {
            let f = named(&name, "request", RequestFlag::from_name)?;
            with(|api, _| api.request(this.0, f).map_err(api_error))
        });
        methods.add_method("set_request", |_, this, (name, on): (mlua::LuaString, bool)| {
            let f = named(&name, "request", RequestFlag::from_name)?;
            with(|api, _| api.set_request(this.0, f, on).map_err(api_error))
        });
        methods.add_method("navi_state", |_, this, name: mlua::LuaString| {
            let f = named(&name, "navi state", NaviState::from_name)?;
            with(|api, _| api.navi_state(this.0, f).map_err(api_error))
        });
        methods.add_method("set_navi_state", |_, this, (name, on): (mlua::LuaString, bool)| {
            let f = named(&name, "navi state", NaviState::from_name)?;
            with(|api, _| api.set_navi_state(this.0, f, on).map_err(api_error))
        });
        for &pad in Pad::ALL {
            methods.add_method(pad.name(), move |_, this, name: mlua::LuaString| {
                let key = named(&name, "button", Key::from_name)?;
                with(|api, _| api.key(this.0, pad, key).map_err(api_error))
            });
        }
        methods.add_method("open_counter_window", |_, this, ()| with(|api, _| Ok(api.open_counter_window(this.0))));
        methods.add_method("check_reactive_abort", |_, this, ()| with(|api, _| Ok(api.check_reactive_abort(this.0))));
        methods.add_method("start_stance_counter", |_, this, ()| with(|api, _| Ok(api.start_stance_counter(this.0))));
        methods.add_method("refresh_form_overlay", |_, this, ()| with(|api, _| Ok(api.refresh_form_overlay(this.0))));
        methods.add_method("exit_attack", |_, this, ()| with(|api, _| Ok(api.exit_attack(this.0))));
        methods.add_method("end_attack", |_, this, ()| with(|api, _| Ok(api.end_attack(this.0))));
        methods.add_method("set_attack", |_, this, (action, kind): (LuaValue, LuaValue)| {
            let kind = u8_arg(kind, "attack kind")?;
            with(|api, b| match b.def(&action) {
                Some((Registry::Action, h)) => api.set_content_attack(this.0, h, kind).map_err(api_error),
                Some((r, _)) => Err(mlua::Error::runtime(format!("set_attack: a {r} is not an action"))),
                None => Ok(api.set_attack(this.0, u8_arg(action, "action")?, kind)),
            })
        });
        methods.add_method("navi_action", |lua, this, ()| {
            let a = with(|api, _| api.navi_action(this.0).map_err(api_error))?;
            match a {
                bn6_content_api::NaviAction::Content(h) => {
                    from_api(lua, Value::Def(Registry::Action, h), &FieldType::Ref(Registry::Action, None))
                }
                bn6_content_api::NaviAction::Engine(name) => Ok(LuaValue::String(lua.create_string(name)?)),
                bn6_content_api::NaviAction::Number(n) => Ok(LuaValue::Number(n as f64)),
            }
        });
        methods.add_method("set_damage_word", |_, this, word: LuaValue| {
            // The damage (with its flag bits) and, above it, the counter
            // byte a hit carries: the object's damage and stamina.
            let word = int(&word, "damage word")? as u32;
            with(|api, _| {
                api.set(this.0, ObjectField::Damage, Value::Int((word & 0xFFFF) as i64)).map_err(api_error)?;
                api.set(this.0, ObjectField::Stamina, Value::Int((word >> 16) as i64)).map_err(api_error)
            })
        });
        methods.add_method("reset_attack_links", |_, this, ()| with(|api, _| Ok(api.reset_attack_links(this.0))));
        methods.add_method("held_direction", |_, this, ()| with(|api, _| Ok(api.held_direction(this.0))));
        methods.add_method("step_target", |_, this, dir: LuaValue| {
            let dir = u8_arg(dir, "direction")?;
            let p = with(|api, _| Ok(api.step_target(this.0, dir)))?;
            Ok(p.map_or((None, None), |p| (Some(p.x), Some(p.y))))
        });
        methods.add_method("start_move", |_, this, dir: LuaValue| {
            let dir = u8_arg(dir, "direction")?;
            with(|api, _| Ok(api.start_move(this.0, dir)))
        });
        methods.add_method("lockon_panel", |_, this, (x, y, mode): (LuaValue, LuaValue, LuaValue)| {
            let p = panel(x, y)?;
            // A lock-on mode (rules/lockon), or nil: the navi's own panel.
            let mode = bound(|b| match (&mode, b.def(&mode)) {
                (LuaValue::Nil, _) => Ok(None),
                (_, Some((Registry::Lockon, h))) => Ok(Some(bn6_content_api::LockonHandle(h))),
                _ => Err(mlua::Error::runtime("lockon_panel: expected a lock-on mode definition or nil")),
            })?;
            let p = with(|api, _| Ok(api.lockon_panel(this.0, p, mode)))?;
            Ok((p.x, p.y))
        });
        methods.add_method("can_move", |_, this, ()| with(|api, _| Ok(api.can_move(this.0))));
        methods.add_method("wear_navi_image", |_, this, user: mlua::UserDataRef<Object>| {
            with(|api, _| api.wear_navi_image(this.0, user.0).map_err(api_error))
        });
        methods.add_method("wear_megaman_image", |_, this, form: LuaValue| {
            let form = u8_arg(form, "form")?;
            with(|api, _| api.wear_megaman_image(this.0, form).map_err(api_error))
        });
        methods.add_method("navi_image_parts", |_, this, on: bool| with(|api, _| Ok(api.navi_image_parts(this.0, on))));
        methods.add_method("wear_junk_look", |_, this, look: LuaValue| {
            let look = u16_arg(look, "junk look")?;
            with(|api, _| api.wear_junk_look(this.0, look).map_err(api_error))
        });
        methods.add_method("heal", |_, this, (amount, anti_recovery): (LuaValue, bool)| {
            let amount = u16_arg(amount, "HP")?;
            with(|api, _| Ok(api.heal(this.0, amount, anti_recovery)))
        });
        methods.add_method("buster_damage", |_, this, ()| with(|api, _| Ok(api.buster_damage(this.0))));
        methods.add_method("prepare_chip", |_, this, ()| with(|api, _| Ok(api.prepare_chip(this.0))));
        methods.add_method("absorbed", |lua, this, ()| {
            let list = with(|api, _| api.absorbed(this.0).map_err(api_error))?;
            let t = lua.create_table_with_capacity(list.len(), 0)?;
            for (i, (look, anim)) in list.into_iter().enumerate() {
                let e = lua.create_table()?;
                e.raw_set("look", bound(|b| b.def_value(Registry::Record, look))?)?;
                e.raw_set("anim", anim)?;
                t.raw_set(i + 1, e)?;
            }
            Ok(t)
        });
        methods.add_method("push_absorbed", |_, this, (look, anim): (LuaValue, LuaValue)| {
            let anim = u8_arg(anim, "anim")?;
            with(|api, b| {
                let look = record_arg(b, &look, "push_absorbed")?;
                api.push_absorbed(this.0, look, anim).map_err(api_error)
            })
        });
        methods.add_method("action_state", |_, this, action: LuaValue| {
            let id = with(|api, bound| {
                let action = match bound.def(&action) {
                    Some((Registry::Action, h)) => Value::Def(Registry::Action, h),
                    Some((r, _)) => return Err(mlua::Error::runtime(format!("action_state: a {r} is not an action"))),
                    None => Value::Int(u8_arg(action, "action")? as i64),
                };
                api.action_schema(action).map_err(api_error)
            })?;
            Ok(State { owner: this.0, action: true, of_action: Some(id) })
        });
        methods.add_method("pop_absorbed", |_, this, ()| {
            let popped = with(|api, _| api.pop_absorbed(this.0).map_err(api_error))?;
            match popped {
                None => Ok((None, None)),
                Some((look, anim)) => Ok((Some(bound(|b| b.def_value(Registry::Record, look))?), Some(anim))),
            }
        });

        // Field objects (obstacles; the rest is the `obstacle` service).
        methods.add_method("obstacle_flag", |_, this, name: mlua::LuaString| {
            let f = named(&name, "obstacle flag", bn6_content_api::api::ObstacleFlag::from_name)?;
            with(|api, _| api.obstacle_flag(this.0, f).map_err(api_error))
        });
    }
}

/// A parameter number 1..=4, as the game's Param1..Param4.
fn param_index(n: LuaValue) -> mlua::Result<usize> {
    let n = int(&n, "param")?;
    if !(1..=4).contains(&n) {
        return Err(mlua::Error::runtime(format!("param {n}: there are params 1 to 4")));
    }
    Ok(n as usize - 1)
}

// ---- Sprite, collision, state, navi --------------------------------------------------------

/// An object's sprite (`Sprite` in core.d.luau).
#[derive(Clone, Copy)]
pub struct Sprite(ObjectRef);

impl UserData for Sprite {
    fn register(registry: &mut mlua::UserDataRegistry<Self>) {
        Self::add_fields(registry);
        Self::add_methods(registry);
        registry.enable_namecall();
    }

    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        for &f in SpriteField::ALL {
            fields.add_field_method_get(f.name(), move |lua, this| {
                let v = with(|api, _| Ok(api.sprite_get(this.0, f)))?;
                from_api(lua, v, &f.ty())
            });
            if f.writable() {
                fields.add_field_method_set(f.name(), move |_, this, v: LuaValue| {
                    let v = to_api(v, &f.ty(), f.name())?;
                    with(|api, _| api.sprite_set(this.0, f, v).map_err(api_error))
                });
            }
        }
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("load", |_, this, (a, b): (LuaValue, Option<LuaValue>)| {
            let id = sprite_id(a, b)?;
            with(|api, _| Ok(api.sprite_load(this.0, id)))
        });
        methods.add_method("load_like", |_, this, like: mlua::UserDataRef<Object>| {
            with(|api, _| api.sprite_load_like(this.0, like.0).map_err(api_error))
        });
        methods.add_method("load_look_of", |_, this, owner: mlua::UserDataRef<Object>| {
            let owner = owner.0;
            with(|api, _| Ok(api.sprite_load_look_of(this.0, owner)))
        });
        methods.add_method("set_animation", |_, this, anim: LuaValue| {
            let anim = u8_arg(anim, "anim")?;
            with(|api, _| Ok(api.sprite_set_animation(this.0, anim)))
        });
        methods.add_method("step", |_, this, ()| with(|api, _| Ok(api.sprite_step(this.0))));
        methods.add_method("part_offset", |_, this, n: LuaValue| {
            let n = u8_arg(n, "n")?;
            with(|api, _| Ok(api.sprite_part_offset(this.0, n)))
        });
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
    fn register(registry: &mut mlua::UserDataRegistry<Self>) {
        Self::add_fields(registry);
        Self::add_methods(registry);
        registry.enable_namecall();
    }

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
        // A region or a hit spark content defines; nil is none (the same
        // as assigning the `region` and `hit_effect` fields).
        for (name, field, registry) in [
            ("set_region", CollisionField::Region, Registry::Region),
            ("set_hit_effect", CollisionField::HitEffect, Registry::Spark),
        ] {
            methods.add_method(name, move |_, this, v: LuaValue| {
                with(|api, b| {
                    let v = match v {
                        LuaValue::Nil => Value::Nil,
                        v => Value::Def(registry, def_arg(b, &v, registry, name)?),
                    };
                    api.collision_set(this.0, field, v).map_err(api_error)
                })
            });
        }
        methods.add_method("remove", |_, this, ()| with(|api, _| Ok(api.remove_collision(this.0))));
        methods.add_method("free", |_, this, ()| with(|api, _| Ok(api.free_collision(this.0))));
        methods.add_method("element_damage", |_, this, element: LuaValue| {
            let element = u8_arg(element, "element")?;
            with(|api, _| api.collision_element_damage(this.0, element).map_err(api_error))
        });
        methods.add_method("hit_by", |lua, this, ()| {
            let hitters = with(|api, _| api.collision_hit_by(this.0).map_err(api_error))?;
            lua.create_sequence_from(hitters.into_iter().map(Object))
        });
    }
}

/// The content-declared state of an object (`me.state`) or of the action
/// it runs (the second argument of an action's `update`). Fields are the
/// ones the kind's `state` table declares, typed as declared.
#[derive(Clone, Copy)]
pub struct State {
    pub owner: ObjectRef,
    pub action: bool,
    /// The attack state as a state of this layout (an action's update's
    /// own, `navi:action_state(action)`), rather than the running action's.
    pub of_action: Option<StateId>,
}

impl State {
    fn with_state<R>(
        self,
        key: &str,
        f: impl FnOnce(&mut ContentState, &bn6_content_api::Schema, usize) -> mlua::Result<R>,
    ) -> mlua::Result<R> {
        with(|api, bound| {
            let s = if let Some(id) = self.of_action {
                api.attack_state_for(self.owner, id).map_err(api_error)?
            } else if self.action {
                api.action_state_mut(self.owner).map_err(api_error)?
            } else {
                api.state_mut(self.owner).ok_or_else(|| api_error(ApiError::NoState(self.owner)))?
            };
            let schema = bound.manifest.schema(s.id());
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
            let got = this.with_state(&key, |s, schema, i| {
                let ty = schema.field(i).ty.clone();
                Ok(match ty {
                    FieldType::Array(..) => None,
                    _ => Some((s.get(schema, i).load(), ty)),
                })
            })?;
            match got {
                Some((v, ty)) => from_api(lua, v, &ty),
                None => Ok(LuaValue::UserData(lua.create_userdata(StateArray { state: *this, field: key.to_string() })?)),
            }
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

/// An array field of a content state: `s.targets[i]`, 1-based like Luau
/// arrays, and `#s.targets`.
pub struct StateArray {
    state: State,
    field: String,
}

impl UserData for StateArray {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::Index, |lua, this, i: LuaValue| {
            let k = int(&i, &this.field)?;
            let (v, ty) = this.state.with_state(&this.field, |s, schema, f| {
                let FieldType::Array(elem, _) = &schema.field(f).ty else { unreachable!("an array field") };
                let v = (k >= 1).then(|| s.get_elem(schema, f, k as usize - 1)).flatten();
                Ok((v, (**elem).clone()))
            })?;
            match v {
                Some(v) => from_api(lua, v.load(), &ty),
                None => Err(mlua::Error::runtime(format!("{}[{k}] is past the array's end", this.field))),
            }
        });
        methods.add_meta_method(MetaMethod::NewIndex, |_, this, (i, v): (LuaValue, LuaValue)| {
            let k = int(&i, &this.field)?;
            this.state.with_state(&this.field, |s, schema, f| {
                let FieldType::Array(elem, _) = &schema.field(f).ty else { unreachable!("an array field") };
                let v = to_api(v, elem, &this.field)?;
                if k < 1 {
                    return Err(mlua::Error::runtime(format!("{}[{k}]: arrays start at 1", this.field)));
                }
                s.set_elem(schema, f, k as usize - 1, v).map_err(mlua::Error::runtime)
            })
        });
        methods.add_meta_method(MetaMethod::Len, |_, this, ()| {
            this.state.with_state(&this.field, |_, schema, f| match &schema.field(f).ty {
                FieldType::Array(_, n) => Ok(*n as i64),
                _ => unreachable!("an array field"),
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
            if f.writable() {
                fields.add_field_method_set(f.name(), move |_, this, v: LuaValue| {
                    let v = to_api(v, &f.ty(), f.name())?;
                    with(|api, _| api.set_navi_stat(this.0, f, v).map_err(api_error))
                });
            }
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
        methods
            .add_meta_function(MetaMethod::Add, |_, (a, b): (mlua::UserDataRef<LVec3>, mlua::UserDataRef<LVec3>)| {
                Ok(LVec3(a.0.wrapping_add(b.0)))
            });
        methods
            .add_meta_function(MetaMethod::Sub, |_, (a, b): (mlua::UserDataRef<LVec3>, mlua::UserDataRef<LVec3>)| {
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

fn vec3_arg(v: Option<mlua::UserDataRef<LVec3>>) -> Vec3 {
    v.map_or(Vec3::default(), |p| p.0)
}

/// An optional `Vec3` argument from a loose value.
fn vec3_value(v: Option<LuaValue>) -> mlua::Result<Vec3> {
    match v {
        None | Some(LuaValue::Nil) => Ok(Vec3::default()),
        Some(LuaValue::UserData(ud)) => ud.borrow::<LVec3>().map(|p| p.0).map_err(|_| mlua::Error::runtime("expected a Vec3")),
        Some(v) => Err(mlua::Error::runtime(format!("expected a Vec3, got {}", v.type_name()))),
    }
}

/// Spawn a kind content defines (or an engine or v1 kind's stand-in).
fn spawn_kind_def(lua: &Lua, kind: LuaValue, pos: Vec3, at: SpawnAt) -> mlua::Result<LuaValue> {
    let o = with(|api, b| match b.def(&kind) {
        Some((Registry::Kind, h)) => api.spawn_def(h, pos, at).map_err(api_error),
        Some((r, _)) => Err(mlua::Error::runtime(format!("battle.spawn: a {r} is not a kind"))),
        None => Err(mlua::Error::runtime("battle.spawn: expected a kind definition")),
    })?;
    object_value(lua, o)
}

// ---- Libraries -------------------------------------------------------------------------------------

/// Add `name = function` to a library table.
macro_rules! lib_fn {
    ($lua:expr, $t:expr, $name:literal, $f:expr) => {
        $t.set($name, $lua.create_function($f)?)?;
    };
}

/// A field of a table argument, as an integer (0 when absent).
fn table_int(t: &mlua::Table, key: &str) -> mlua::Result<i64> {
    let v: LuaValue = t.raw_get(key)?;
    if v.is_nil() { Ok(0) } else { int(&v, key) }
}

/// Install `battle`, `field`, `dimming`, `navi_chip`, `obstacle`, `Vec3`
/// and `int`.
pub fn install(lua: &Lua) -> mlua::Result<()> {
    let g = lua.globals();
    g.set("battle", battle_lib(lua)?)?;
    g.set("field", field_lib(lua)?)?;
    g.set("dimming", dimming_lib(lua)?)?;
    g.set("obstacle", obstacle_lib(lua)?)?;

    let navi_chip = lua.create_table()?;
    lib_fn!(lua, navi_chip, "warp", |_, (user, out): (mlua::UserDataRef<Object>, bool)| {
        with(|api, _| Ok(api.navi_warp(user.0, out)))
    });
    lib_fn!(lua, navi_chip, "navi_left", |_, c: mlua::UserDataRef<Object>| {
        with(|api, _| Ok(api.navi_chip_left(c.0)))
    });
    g.set("navi_chip", navi_chip)?;

    let vec3 = lua.create_table()?;
    let wrap = |v: LuaValue, what| -> mlua::Result<i32> { Ok(int(&v, what)? as i32) };
    lib_fn!(lua, vec3, "new", move |_, (x, y, z): (LuaValue, LuaValue, LuaValue)| {
        Ok(LVec3(Vec3 { x: wrap(x, "x")?, y: wrap(y, "y")?, z: wrap(z, "z")? }))
    });
    lib_fn!(lua, vec3, "px", move |_, (x, y, z): (LuaValue, LuaValue, LuaValue)| {
        Ok(LVec3(Vec3 { x: wrap(x, "x")? << 16, y: wrap(y, "y")? << 16, z: wrap(z, "z")? << 16 }))
    });
    vec3.set("zero", LVec3(Vec3::default()))?;
    g.set("Vec3", vec3)?;

    g.set("int", int_lib(lua)?)?;
    Ok(())
}

fn battle_lib(lua: &Lua) -> mlua::Result<mlua::Table> {
    let t = lua.create_table()?;
    lib_fn!(lua, t, "dimmed", |_, ()| with(|api, _| Ok(api.is_dimmed())));
    lib_fn!(lua, t, "paused", |_, ()| with(|api, _| Ok(api.is_paused())));
    lib_fn!(lua, t, "over", |_, ()| with(|api, _| Ok(api.is_battle_over())));
    lib_fn!(lua, t, "time_up", |_, ()| with(|api, _| Ok(api.is_time_up())));
    lib_fn!(lua, t, "next_chip_damages", |_, user: mlua::UserDataRef<Object>| {
        with(|api, _| Ok(api.next_chip_damages(user.0)))
    });
    lib_fn!(lua, t, "viewer_sees", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| Ok(api.viewer_sees(side)))
    });
    for &f in BattleInfo::ALL {
        t.set(
            f.name(),
            lua.create_function(move |lua, ()| {
                let v = with(|api, _| Ok(api.battle_info(f)))?;
                from_api(lua, v, &f.ty())
            })?,
        )?;
    }
    lib_fn!(lua, t, "play_sound", |_, id: LuaValue| {
        let id = sound_arg(id)?;
        with(|api, _| Ok(api.play_sound(id)))
    });
    lib_fn!(lua, t, "play_sound_for", |_, (side, id): (LuaValue, LuaValue)| {
        let (side, id) = (u8_arg(side, "side")? & 1, sound_arg(id)?);
        with(|api, _| Ok(api.play_sound_for(side, id)))
    });
    lib_fn!(lua, t, "shake_camera", |_, (magnitude, ticks): (LuaValue, LuaValue)| {
        let (magnitude, ticks) = (u16_arg(magnitude, "magnitude")?, u16_arg(ticks, "ticks")?);
        if magnitude > 3 {
            return Err(mlua::Error::runtime(format!("camera shake magnitude {magnitude} reads past byte_8030284")));
        }
        with(|api, _| Ok(api.shake_camera(magnitude, ticks)))
    });
    lib_fn!(lua, t, "navi", |_, side: LuaValue| Ok(Navi(u8_arg(side, "side")? & 1)));
    lib_fn!(lua, t, "emotion", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| Ok(api.emotion(side).name()))
    });
    lib_fn!(lua, t, "set_mood", |_, (side, mood): (LuaValue, LuaValue)| {
        let (side, mood) = (u8_arg(side, "side")? & 1, u8_arg(mood, "mood")?);
        with(|api, _| Ok(api.set_mood(side, mood)))
    });
    lib_fn!(lua, t, "side_special", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| Ok(api.side_special(side).name()))
    });
    lib_fn!(lua, t, "player", |lua, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        let p = with(|api, _| Ok(api.player(side)))?;
        object_value(lua, p)
    });
    lib_fn!(lua, t, "alive_actors", |lua, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        let list = with(|api, _| Ok(api.alive_actors(side)))?;
        lua.create_sequence_from(list.into_iter().map(Object))
    });
    lib_fn!(lua, t, "objects_of", |lua, kind: LuaValue| {
        let list = with(|api, b| match b.def(&kind) {
            Some((Registry::Kind, h)) => Ok(api.objects_of_kind(h)),
            Some((r, _)) => Err(mlua::Error::runtime(format!("battle.objects_of: a {r} is not a kind"))),
            None => Err(mlua::Error::runtime("battle.objects_of: expected a kind definition")),
        })?;
        lua.create_sequence_from(list.into_iter().map(Object))
    });
    lib_fn!(lua, t, "rng", |_, ()| with(|api, _| Ok(api.rng())));
    lib_fn!(lua, t, "rng_positive", |_, ()| with(|api, _| Ok(api.rng_positive())));
    lib_fn!(lua, t, "jitter", |_, (mask, pos): (LuaValue, mlua::UserDataRef<LVec3>)| {
        let mask = int(&mask, "mask")? as u32;
        with(|api, _| Ok(LVec3(api.jitter(mask, pos.0))))
    });
    lib_fn!(lua, t, "hand_chip", |_, (side, i): (LuaValue, LuaValue)| {
        let (side, i) = (u8_arg(side, "side")? & 1, u8_arg(i, "hand index")?);
        with(|api, _| Ok(api.hand_chip(side, i)))
    });
    lib_fn!(lua, t, "hand_cursor", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| Ok(api.hand_cursor(side)))
    });
    lib_fn!(lua, t, "advance_hand", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| Ok(api.advance_hand(side)))
    });
    lib_fn!(lua, t, "linked", |lua, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        let r = with(|api, _| Ok(api.linked(side)))?;
        let t = lua.create_table()?;
        t.raw_set("chip", r.chip)?;
        t.raw_set("bonus", r.bonus)?;
        t.raw_set("damage", r.damage)?;
        t.raw_set("owner", object_value(lua, r.owner)?)?;
        t.raw_set("object", object_value(lua, r.object)?)?;
        Ok(t)
    });
    lib_fn!(lua, t, "set_linked", |_, (side, r): (LuaValue, mlua::Table)| {
        let side = u8_arg(side, "side")? & 1;
        let rec = LinkedChip {
            chip: table_int(&r, "chip")? as u16,
            bonus: table_int(&r, "bonus")? as u16,
            damage: table_int(&r, "damage")? as u32,
            owner: object_arg(&r.raw_get("owner")?, "owner")?,
            object: object_arg(&r.raw_get("object")?, "object")?,
        };
        with(|api, _| Ok(api.set_linked(side, rec)))
    });
    lib_fn!(lua, t, "clear_linked", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| Ok(api.clear_linked(side)))
    });
    lib_fn!(lua, t, "clear_navicust_bugs", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| Ok(api.clear_navicust_bugs(side)))
    });
    lib_fn!(lua, t, "fill_custom_gauge", |_, ()| with(|api, _| Ok(api.fill_custom_gauge())));
    lib_fn!(lua, t, "add_side_gauge", |_, (side, n): (LuaValue, LuaValue)| {
        let (side, n) = (u8_arg(side, "side")? & 1, u16_arg(n, "gauge")?);
        with(|api, _| Ok(api.add_side_gauge(side, n)))
    });
    lib_fn!(lua, t, "add_special_bonus", |_, (side, index, n): (LuaValue, LuaValue, LuaValue)| {
        let (side, index, n) = (u8_arg(side, "side")? & 1, u8_arg(index, "bonus")?, u16_arg(n, "bonus")?);
        with(|api, _| api.add_special_bonus(side, index, n).map_err(api_error))
    });
    lib_fn!(lua, t, "set_gauge_rate", |_, rate: LuaValue| {
        let rate = u16_arg(rate, "gauge rate")?;
        with(|api, _| Ok(api.set_gauge_rate(rate)))
    });
    lib_fn!(lua, t, "set_gauge_speed_ticks", |_, (side, slow, fast): (LuaValue, LuaValue, LuaValue)| {
        let (side, slow, fast) = (u8_arg(side, "side")? & 1, u16_arg(slow, "ticks")?, u16_arg(fast, "ticks")?);
        with(|api, _| Ok(api.set_gauge_speed_ticks(side, slow, fast)))
    });
    lib_fn!(lua, t, "bump_side_stat", |_, (side, i, n): (LuaValue, LuaValue, LuaValue)| {
        let (side, i, n) = (u8_arg(side, "side")? & 1, u8_arg(i, "stat")?, u8_arg(n, "count")?);
        with(|api, _| Ok(api.bump_side_stat(side, i, n)))
    });
    lib_fn!(lua, t, "side_stat", |_, (side, i): (LuaValue, LuaValue)| {
        let (side, i) = (u8_arg(side, "side")? & 1, u8_arg(i, "stat")?);
        with(|api, _| Ok(api.side_stat(side, i)))
    });
    lib_fn!(lua, t, "damage_carry", |lua, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        let r = with(|api, _| Ok(api.damage_carry(side)))?;
        let t = lua.create_table()?;
        t.raw_set("this_tick", r.this_tick)?;
        t.raw_set("previous", r.previous)?;
        t.raw_set("source", object_value(lua, r.source)?)?;
        t.raw_set("target", object_value(lua, r.target)?)?;
        Ok(t)
    });
    lib_fn!(lua, t, "set_damage_carry", |_, (side, r): (LuaValue, mlua::Table)| {
        let side = u8_arg(side, "side")? & 1;
        let rec = bn6_content_api::api::DamageCarryInfo {
            this_tick: table_int(&r, "this_tick")? as u16,
            previous: table_int(&r, "previous")? as u16,
            source: object_arg(&r.raw_get("source")?, "source")?,
            target: object_arg(&r.raw_get("target")?, "target")?,
        };
        with(|api, _| Ok(api.set_damage_carry(side, rec)))
    });
    lib_fn!(lua, t, "loop_register", |_, ()| with(|api, _| Ok(api.loop_register())));
    lib_fn!(lua, t, "navi_record", |lua, name_id: LuaValue| {
        let name_id = u16_arg(name_id, "NameID")?;
        let Some(r) = with(|api, _| Ok(api.navi_record(name_id)))? else { return Ok(LuaValue::Nil) };
        let t = lua.create_table()?;
        t.raw_set("actor_type", ACTOR_TYPES[r.actor_type as usize])?;
        t.raw_set("ai_index", r.ai_index)?;
        Ok(LuaValue::Table(t))
    });
    lib_fn!(
        lua,
        t,
        "spawn",
        |lua, args: mlua::MultiValue| {
            let mut args = args.into_iter();
            let first = args.next().unwrap_or(LuaValue::Nil);
            // `battle.spawn(kind, pos)`, or (deprecated) by pool and index.
            if let LuaValue::Table(_) = first {
                let pos = vec3_value(args.next())?;
                return spawn_kind_def(lua, first, pos, SpawnAt::AfterCurrent);
            }
            let LuaValue::String(pool) = first else {
                return Err(mlua::Error::runtime("battle.spawn: expected a kind"));
            };
            let pool = named(&pool, "pool", Pool::from_name)?;
            let index = u8_arg(args.next().unwrap_or(LuaValue::Nil), "kind index")?;
            let pos = vec3_value(args.next())?;
            let p = match args.next() {
                Some(LuaValue::Table(t)) => params(Some(t), "spawn param")?,
                _ => [0; 4],
            };
            let o = with(|api, _| Ok(api.spawn(pool, index, pos, p)))?;
            object_value(lua, o)
        }
    );
    lib_fn!(lua, t, "spawn_first", |lua, (kind, pos): (LuaValue, Option<mlua::UserDataRef<LVec3>>)| {
        spawn_kind_def(lua, kind, vec3_arg(pos), SpawnAt::First)
    });
    lib_fn!(lua, t, "spawn_at_end", |lua, (kind, pos): (LuaValue, Option<mlua::UserDataRef<LVec3>>)| {
        spawn_kind_def(lua, kind, vec3_arg(pos), SpawnAt::End)
    });
    lib_fn!(
        lua,
        t,
        "spawn_kind",
        |lua, (name, pos, p): (mlua::LuaString, Option<mlua::UserDataRef<LVec3>>, Option<mlua::Table>)| {
            let name = name.to_str()?.to_string();
            let (pos, p) = (vec3_arg(pos), params(p, "spawn param")?);
            let o = with(|api, _| api.spawn_kind(&name, pos, p).map_err(api_error))?;
            object_value(lua, o)
        }
    );
    lib_fn!(
        lua,
        t,
        "spawn_kind_first",
        |lua, (name, pos, p): (mlua::LuaString, Option<mlua::UserDataRef<LVec3>>, Option<mlua::Table>)| {
            let name = name.to_str()?.to_string();
            let (pos, p) = (vec3_arg(pos), params(p, "spawn param")?);
            let o = with(|api, _| api.spawn_kind_first(&name, pos, p).map_err(api_error))?;
            object_value(lua, o)
        }
    );
    lib_fn!(
        lua,
        t,
        "spawn_kind_at_end",
        |lua, (name, pos, p): (mlua::LuaString, Option<mlua::UserDataRef<LVec3>>, Option<mlua::Table>)| {
            let name = name.to_str()?.to_string();
            let (pos, p) = (vec3_arg(pos), params(p, "spawn param")?);
            let o = with(|api, _| api.spawn_kind_at_end(&name, pos, p).map_err(api_error))?;
            object_value(lua, o)
        }
    );
    lib_fn!(
        lua,
        t,
        "effect",
        |lua, (pos, id, flip, palette_add, priority): (
            mlua::UserDataRef<LVec3>,
            LuaValue,
            Option<LuaValue>,
            Option<LuaValue>,
            Option<LuaValue>
        )| {
            let opt = |v: Option<LuaValue>, what| v.map_or(Ok(0), |v| u8_arg(v, what));
            let (flip, add, prio) = (opt(flip, "flip")?, opt(palette_add, "palette")?, opt(priority, "priority")?);
            let o = with(|api, b| {
                let look = EffectHandle(def_arg(b, &id, Registry::Effect, "battle.effect")?);
                Ok(api.spawn_effect(pos.0, look, flip, add, prio))
            })?;
            object_value(lua, o)
        }
    );
    lib_fn!(
        lua,
        t,
        "palette_flash",
        |lua, (variant, ticks, while_dimmed, while_paused): (LuaValue, LuaValue, Option<bool>, Option<bool>)| {
            let (variant, ticks) = (u8_arg(variant, "palette flash variant")?, u8_arg(ticks, "palette flash ticks")?);
            let (dimmed, paused) = (while_dimmed.unwrap_or(false), while_paused.unwrap_or(false));
            let o = with(|api, _| Ok(api.spawn_palette_flash(variant, ticks, dimmed, paused)))?;
            object_value(lua, o)
        }
    );
    lib_fn!(
        lua,
        t,
        "region_effects",
        |_, (x, y, region, side, id, z): (LuaValue, LuaValue, LuaValue, LuaValue, LuaValue, Option<LuaValue>)| {
            let (x, y) = (int(&x, "x")? as i32, int(&y, "y")? as i32);
            let side = u8_arg(side, "side")? & 1;
            let z = match z {
                Some(z) => int(&z, "z")? as i32,
                None => 0,
            };
            with(|api, b| {
                let region = RegionHandle(def_arg(b, &region, Registry::Region, "battle.region_effects")?);
                let look = EffectHandle(def_arg(b, &id, Registry::Effect, "battle.region_effects")?);
                Ok(api.spawn_region_effects(x, y, region, side, look, z))
            })
        }
    );
    lib_fn!(lua, t, "hitbox", |lua, (owner, spec): (mlua::UserDataRef<Object>, mlua::Table)| {
        // A definition; a region or a hit spark may be absent (none).
        let def = |key: &str, registry: Registry| -> mlua::Result<Option<u16>> {
            let v: LuaValue = spec.raw_get(key)?;
            if v.is_nil() {
                return Ok(None);
            }
            bound(|b| def_arg(b, &v, registry, key)).map(Some)
        };
        let collision = |key: &str| -> mlua::Result<CollisionHandle> {
            let h = def(key, Registry::Collision)?;
            h.map(CollisionHandle).ok_or_else(|| mlua::Error::runtime(format!("battle.hitbox: `{key}` is a collision type (define.collision)")))
        };
        let s = HitboxSpec {
            panel: PanelPos { x: table_int(&spec, "panel_x")? as u8, y: table_int(&spec, "panel_y")? as u8 },
            element: table_int(&spec, "element")? as u8,
            z: table_int(&spec, "z")? as i32,
            region: def("region", Registry::Region)?.map(RegionHandle),
            hit_effect: def("hit_effect", Registry::Spark)?.map(SparkHandle),
            target: collision("target")?,
            self_type: collision("self_type")?,
            damage: table_int(&spec, "damage")? as u16,
            stamina: table_int(&spec, "stamina")? as u16,
            hit_mod: table_int(&spec, "hit_mod")? as u8,
            // A status definition, or nothing.
            status: {
                let v: LuaValue = spec.raw_get("status")?;
                bound(|b| match (&v, b.def(&v)) {
                    (LuaValue::Nil, _) => Ok(None),
                    (_, Some((Registry::Status, h))) => Ok(Some(bn6_content_api::StatusHandle(h))),
                    _ => Err(mlua::Error::runtime("battle.hitbox: `status` is a status definition (rules/status) or nil")),
                })?
            },
            bug: table_int(&spec, "bug")? as u8,
            bug_arg: table_int(&spec, "bug_arg")? as u8,
        };
        let o = with(|api, _| Ok(api.spawn_hitbox(owner.0, &s)))?;
        object_value(lua, o)
    });
    lib_fn!(lua, t, "spark", |lua, (owner, pos, id): (mlua::UserDataRef<Object>, mlua::UserDataRef<LVec3>, LuaValue)| {
        let o = with(|api, b| {
            let look = SparkHandle(def_arg(b, &id, Registry::Spark, "battle.spark")?);
            Ok(api.spawn_spark(owner.0, pos.0, look))
        })?;
        object_value(lua, o)
    });
    lib_fn!(lua, t, "form_overlay", |lua, (owner, spec): (mlua::UserDataRef<Object>, mlua::Table)| {
        let sprite = sprite_id(spec.raw_get("sprite")?, None)?;
        let stepping = match spec.raw_get::<Option<mlua::LuaString>>("stepping")? {
            Some(s) => named(&s, "overlay stepping", |n| OVERLAY_STEPPINGS.iter().position(|x| *x == n))? as u8,
            None => 0,
        };
        let anim_offset = table_int(&spec, "anim_offset")? as u8;
        let nudged = spec.raw_get::<Option<bool>>("nudged")?.unwrap_or(false);
        let owner_palette = spec.raw_get::<Option<bool>>("owner_palette")?.unwrap_or(false);
        let o = with(|api, _| Ok(api.spawn_form_overlay(owner.0, sprite, stepping, anim_offset, nudged, owner_palette)))?;
        object_value(lua, o)
    });
    // Subtype 18 (Otenko).
    lib_fn!(lua, t, "hand_turn", |_, (side, i): (LuaValue, LuaValue)| {
        let (side, i) = (u8_arg(side, "side")? & 1, u8_arg(i, "hand index")?);
        with(|api, _| Ok(api.hand_turn(side, i)))
    });
    lib_fn!(lua, t, "add_hand_attack_bonus", |_, (side, i, n): (LuaValue, LuaValue, LuaValue)| {
        let (side, i, n) = (u8_arg(side, "side")? & 1, u8_arg(i, "hand index")?, u16_arg(n, "bonus")?);
        with(|api, _| Ok(api.add_hand_attack_bonus(side, i, n)))
    });
    lib_fn!(lua, t, "hand_chip_damages", |_, (side, i): (LuaValue, LuaValue)| {
        let (side, i) = (u8_arg(side, "side")? & 1, u8_arg(i, "hand index")?);
        with(|api, _| Ok(api.hand_chip_damages(side, i)))
    });
    // Subtype 8 (Wind and Fan).
    lib_fn!(lua, t, "wind", |lua, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        let (o, source) = with(|api, _| Ok(api.wind(side)))?;
        Ok((object_value(lua, o)?, source.name()))
    });
    lib_fn!(lua, t, "set_wind", |_, (o, side, source): (mlua::UserDataRef<Object>, LuaValue, mlua::LuaString)| {
        let (side, source) = (u8_arg(side, "side")? & 1, named(&source, "wind source", WindSource::from_name)?);
        with(|api, _| Ok(api.set_wind(o.0, side, source)))
    });
    lib_fn!(lua, t, "clear_wind", |_, o: mlua::UserDataRef<Object>| with(|api, _| Ok(api.clear_wind(o.0))));
    lib_fn!(
        lua,
        t,
        "afterimage",
        |lua, (owner, pos, spec): (mlua::UserDataRef<Object>, mlua::UserDataRef<LVec3>, mlua::Table)| {
            let sprite: LuaValue = spec.raw_get("sprite")?;
            let shadow: Option<mlua::LuaString> = spec.raw_get("shadow")?;
            let shadow = match shadow {
                Some(s) => named(&s, "shadow", |n| bn6_content_api::Shadow::NAMES.iter().position(|&m| m == n).map(|i| bn6_content_api::Shadow::ALL[i]))?,
                None => bn6_content_api::Shadow::WithSprite,
            };
            let tether: Option<mlua::LuaString> = spec.raw_get("tether")?;
            let tether = match tether {
                Some(s) => named(&s, "afterimage tether", |n| match n {
                    "beast_form" => Some(1),
                    "attack" => Some(2),
                    _ => None,
                })?,
                None => 0,
            };
            let s = bn6_content_api::api::AfterimageSpec {
                sprite: if sprite.is_nil() { None } else { Some(sprite_id(sprite, None)?) },
                anim: table_int(&spec, "anim")? as u8,
                flip: table_int(&spec, "flip")? as u8,
                lifetime: table_int(&spec, "lifetime")? as u16,
                color_shader: table_int(&spec, "color_shader")? as u16,
                palette: table_int(&spec, "palette")? as u8,
                shadow,
                steady: spec.raw_get::<Option<bool>>("steady")?.unwrap_or(false),
                tether,
            };
            let o = with(|api, _| Ok(api.spawn_afterimage(owner.0, pos.0, &s)))?;
            object_value(lua, o)
        }
    );
    lib_fn!(lua, t, "attach_point", |_, (name_id, point, alliance, flip): (LuaValue, LuaValue, LuaValue, LuaValue)| {
        let (name_id, point) = (u16_arg(name_id, "NameID")?, u8_arg(point, "attach point")?);
        let (alliance, flip) = (u8_arg(alliance, "side")?, u8_arg(flip, "flip")?);
        with(|api, _| Ok(api.name_attach_point(name_id, point, alliance, flip)))
    });
    Ok(t)
}

fn field_lib(lua: &Lua) -> mlua::Result<mlua::Table> {
    let t = lua.create_table()?;
    lib_fn!(lua, t, "valid", |_, (x, y): (LuaValue, LuaValue)| {
        let p = panel(x, y)?;
        with(|api, _| Ok(api.panel_valid(p)))
    });
    lib_fn!(lua, t, "center", |_, (x, y): (LuaValue, LuaValue)| {
        let p = panel(x, y)?;
        with(|api, _| Ok(api.panel_center(p)))
    });
    lib_fn!(lua, t, "flags", |_, (x, y): (LuaValue, LuaValue)| {
        let p = panel(x, y)?;
        with(|api, _| Ok(api.panel_flags(p)))
    });
    lib_fn!(lua, t, "check", |_, (x, y, require, forbid): (LuaValue, LuaValue, LuaValue, LuaValue)| {
        let p = panel(x, y)?;
        let (require, forbid) = (int(&require, "require")? as u32, int(&forbid, "forbid")? as u32);
        with(|api, _| Ok(api.panel_check(p, require, forbid)))
    });
    lib_fn!(lua, t, "panel", |lua, (x, y): (LuaValue, LuaValue)| {
        let p = panel(x, y)?;
        let Some(info) = with(|api, _| Ok(api.panel_info(p)))? else { return Ok(LuaValue::Nil) };
        let t = lua.create_table()?;
        t.raw_set("kind", PANEL_TYPES[info.kind as usize])?;
        t.raw_set("alliance", info.alliance)?;
        t.raw_set("home", info.home)?;
        Ok(LuaValue::Table(t))
    });
    lib_fn!(lua, t, "all_objects", |lua, ()| {
        let list = with(|api, _| Ok(api.all_field_objects()))?;
        lua.create_sequence_from(list.into_iter().map(Object))
    });
    lib_fn!(lua, t, "objects", |lua, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        let list = with(|api, _| Ok(api.side_field_objects(side)))?;
        lua.create_sequence_from(list.into_iter().map(Object))
    });
    lib_fn!(lua, t, "column", |lua, x: LuaValue| {
        let x = u8_arg(x, "column")?;
        let c = with(|api, _| Ok(api.column_info(x)))?;
        let t = lua.create_table()?;
        t.raw_set("home", c.home)?;
        t.raw_set("timer", c.timer)?;
        Ok(t)
    });
    lib_fn!(lua, t, "set_column_timer", |_, (x, ticks): (LuaValue, LuaValue)| {
        let (x, ticks) = (u8_arg(x, "column")?, u16_arg(ticks, "ticks")?);
        with(|api, _| Ok(api.set_column_timer(x, ticks)))
    });
    lib_fn!(lua, t, "set_alliance", |_, (x, y, side): (LuaValue, LuaValue, LuaValue)| {
        let (p, side) = (panel(x, y)?, u8_arg(side, "side")?);
        with(|api, _| Ok(api.set_panel_alliance(p, side)))
    });
    lib_fn!(lua, t, "set_type", |_, (x, y, kind): (LuaValue, LuaValue, mlua::LuaString)| {
        let p = panel(x, y)?;
        let kind = named(&kind, "panel type", |s| PANEL_TYPES.iter().position(|&n| n == s))? as u8;
        with(|api, _| Ok(api.set_panel_type(p, kind)))
    });
    lib_fn!(lua, t, "crack", |_, (x, y): (LuaValue, LuaValue)| {
        let p = panel(x, y)?;
        with(|api, _| Ok(api.crack_panel(p)))
    });
    lib_fn!(lua, t, "break_panel", |_, (x, y): (LuaValue, LuaValue)| {
        let p = panel(x, y)?;
        with(|api, _| Ok(api.break_panel(p)))
    });
    lib_fn!(lua, t, "shatter", |_, (x, y): (LuaValue, LuaValue)| {
        let p = panel(x, y)?;
        with(|api, _| Ok(api.shatter_panel(p)))
    });
    lib_fn!(lua, t, "break_empty", |_, (x, y): (LuaValue, LuaValue)| {
        let p = panel(x, y)?;
        with(|api, _| Ok(api.break_empty_panel(p)))
    });
    lib_fn!(lua, t, "solid", |_, (x, y): (LuaValue, LuaValue)| {
        let p = panel(x, y)?;
        with(|api, _| Ok(api.panel_solid(p)))
    });
    lib_fn!(lua, t, "highlight", |_, (x, y): (LuaValue, LuaValue)| {
        let p = panel(x, y)?;
        with(|api, _| Ok(api.highlight_panel(p)))
    });
    // Panel changes (dimming chip subtypes 2, 3, 5, 15 and 27).
    lib_fn!(lua, t, "poison", |_, (x, y): (LuaValue, LuaValue)| {
        let p = panel(x, y)?;
        with(|api, _| Ok(api.poison_panel(p)))
    });
    lib_fn!(lua, t, "blink", |_, (x, y, kind, side): (LuaValue, LuaValue, mlua::LuaString, LuaValue)| {
        let (p, side) = (panel(x, y)?, u8_arg(side, "side")?);
        let kind = named(&kind, "panel type", |s| PANEL_TYPES.iter().position(|&n| n == s))? as u8;
        with(|api, _| Ok(api.blink_panel(p, kind, side)))
    });
    Ok(t)
}

fn dimming_lib(lua: &Lua) -> mlua::Result<mlua::Table> {
    let t = lua.create_table()?;
    for &step in DimmingStep::ALL {
        let needs_chip = matches!(step, DimmingStep::CheckAntiNavi | DimmingStep::ShowNaviTelop);
        t.set(
            step.name(),
            lua.create_function(move |_, (me, chip): (mlua::UserDataRef<Object>, Option<LuaValue>)| {
                let chip = match chip {
                    Some(c) => u16_arg(c, "chip")?,
                    None if needs_chip => {
                        return Err(mlua::Error::runtime(format!("dimming.{} needs the chip", step.name())));
                    }
                    None => 0,
                };
                with(|api, _| Ok(api.dimming(me.0, step, chip)))
            })?,
        )?;
    }
    lib_fn!(
        lua,
        t,
        "start",
        |_, (side, no_cut_in, controller, user): (LuaValue, bool, LuaValue, mlua::UserDataRef<Object>)| {
            let side = u8_arg(side, "side")? & 1;
            let controller = object_arg(&controller, "controller")?;
            with(|api, _| Ok(api.start_dimming(side, no_cut_in, controller, user.0)))
        }
    );
    lib_fn!(lua, t, "hide_user", |_, user: mlua::UserDataRef<Object>| with(|api, _| Ok(api.hide_user(user.0))));
    lib_fn!(lua, t, "show_user", |_, user: mlua::UserDataRef<Object>| with(|api, _| Ok(api.show_user(user.0))));
    lib_fn!(lua, t, "hide_user_sparing", |_, user: mlua::UserDataRef<Object>| {
        with(|api, _| Ok(api.hide_user_sparing(user.0)))
    });
    Ok(t)
}

fn obstacle_lib(lua: &Lua) -> mlua::Result<mlua::Table> {
    type Me = mlua::UserDataRef<Object>;
    let t = lua.create_table()?;
    lib_fn!(lua, t, "register", |_, (me, side, class): (Me, LuaValue, LuaValue)| {
        let (side, class) = (u8_arg(side, "side")? & 1, u8_arg(class, "class")?);
        if class > 1 {
            return Err(mlua::Error::runtime(format!("obstacle.register: class {class} (0 or 1)")));
        }
        with(|api, _| Ok(api.obstacle_register(me.0, side, class)))
    });
    lib_fn!(lua, t, "unregister", |_, me: Me| with(|api, _| Ok(api.obstacle_unregister(me.0))));
    lib_fn!(lua, t, "take_hits", |_, (me, push): (Me, Option<mlua::LuaString>)| {
        let push = match push {
            Some(p) => named(&p, "push", ObstaclePush::from_name)?,
            None => ObstaclePush::ForgetsDamage,
        };
        with(|api, _| api.obstacle_take_hits(me.0, push).map_err(api_error))
    });
    lib_fn!(lua, t, "tick_lifetime", |_, me: Me| with(|api, _| api.obstacle_tick_lifetime(me.0).map_err(api_error)));
    lib_fn!(lua, t, "react", |_, (me, crush, hold): (Me, Option<mlua::LuaString>, Option<mlua::LuaString>)| {
        let crush = match crush {
            Some(c) => named(&c, "crush", ObstacleCrush::from_name)?,
            None => ObstacleCrush::Breaks,
        };
        let hold = match hold {
            Some(h) => named(&h, "dimming hold", ObstacleHold::from_name)?,
            None => ObstacleHold::AfterAppearing,
        };
        with(|api, _| api.obstacle_react(me.0, crush, hold).map_err(api_error))
    });
    for &a in ObstacleAction::ALL {
        t.set(
            a.name(),
            lua.create_function(move |_, me: Me| with(|api, _| api.obstacle_action(me.0, a).map_err(api_error)))?,
        )?;
    }
    lib_fn!(lua, t, "removal", |_, me: Me| {
        with(|api, _| api.obstacle_removal(me.0).map(|r| r.name()).map_err(api_error))
    });
    lib_fn!(lua, t, "blink_out", |_, me: Me| {
        with(|api, _| api.obstacle_blink_out(me.0).map(|r| r.name()).map_err(api_error))
    });
    lib_fn!(lua, t, "fly_to_absorber", |_, (me, look): (Me, LuaValue)| {
        with(|api, b| {
            let look = record_arg(b, &look, "obstacle.fly_to_absorber")?;
            api.obstacle_fly_to_absorber(me.0, look).map_err(api_error)
        })
    });
    lib_fn!(lua, t, "release_tracking", |_, me: Me| with(|api, _| Ok(api.obstacle_release_tracking(me.0))));
    lib_fn!(lua, t, "stage_slot_free", |_, ()| with(|api, _| Ok(api.obstacle_stage_slot_free())));
    lib_fn!(lua, t, "enter_stage", |_, me: Me| with(|api, _| api.obstacle_enter_stage(me.0).map_err(api_error)));
    lib_fn!(lua, t, "leave_stage", |_, me: Me| with(|api, _| Ok(api.obstacle_leave_stage(me.0))));
    lib_fn!(lua, t, "absorb_all", |_, absorber: Me| with(|api, _| Ok(api.obstacle_absorb_all(absorber.0))));
    lib_fn!(lua, t, "present", |_, o: Me| with(|api, _| Ok(api.obstacle_present(o.0))));
    lib_fn!(lua, t, "junk_look", |_, o: Me| with(|api, _| Ok(api.junk_look(o.0))));
    lib_fn!(lua, t, "swallowable", |_, o: Me| with(|api, _| Ok(api.obstacle_swallowable(o.0))));
    for &r in ObstacleRequest::ALL {
        t.set(
            r.name(),
            lua.create_function(move |_, (o, by): (Me, Option<Me>)| {
                let by = match (r, by) {
                    (_, Some(by)) => by.0,
                    (ObstacleRequest::Absorb, None) => {
                        return Err(mlua::Error::runtime("obstacle.absorb needs the absorber"));
                    }
                    (_, None) => o.0,
                };
                with(|api, _| Ok(api.obstacle_request(o.0, r, by)))
            })?,
        )?;
    }
    Ok(t)
}

fn int_lib(lua: &Lua) -> mlua::Result<mlua::Table> {
    let lib = lua.create_table()?;
    macro_rules! wrapper {
        ($($name:literal => $t:ty),*) => {$(
            lib.set($name, lua.create_function(|_, v: LuaValue| Ok(int(&v, $name)? as $t))?)?;
        )*};
    }
    wrapper!("u8" => u8, "u16" => u16, "u32" => u32, "i8" => i8, "i16" => i16, "i32" => i32);
    lib_fn!(lua, lib, "asr", |_, (x, n): (LuaValue, LuaValue)| {
        let (x, n) = (int(&x, "asr")?, int(&n, "asr")?);
        Ok((x as i32) >> (n & 31))
    });
    lib_fn!(lua, lib, "shl", |_, (x, n): (LuaValue, LuaValue)| {
        let (x, n) = (int(&x, "shl")?, int(&n, "shl")?);
        Ok((x as i32).wrapping_shl(n as u32 & 31))
    });
    lib_fn!(lua, lib, "lsr", |_, (x, n): (LuaValue, LuaValue)| {
        let (x, n) = (int(&x, "lsr")?, int(&n, "lsr")?);
        Ok((x as u32) >> (n & 31))
    });
    lib_fn!(lua, lib, "tdiv", |_, (a, b): (LuaValue, LuaValue)| {
        let (a, b) = (int(&a, "tdiv")?, int(&b, "tdiv")?);
        if b == 0 {
            return Err(mlua::Error::runtime("int.tdiv: division by zero"));
        }
        Ok(a / b)
    });
    lib_fn!(lua, lib, "tmod", |_, (a, b): (LuaValue, LuaValue)| {
        let (a, b) = (int(&a, "tmod")?, int(&b, "tmod")?);
        if b == 0 {
            return Err(mlua::Error::runtime("int.tmod: division by zero"));
        }
        Ok(a % b)
    });
    lib_fn!(lua, lib, "mul32", |_, (a, b): (LuaValue, LuaValue)| {
        let (a, b) = (int(&a, "mul32")?, int(&b, "mul32")?);
        Ok((a as i32).wrapping_mul(b as i32))
    });
    Ok(lib)
}

/// Wrap the object being updated as a script value.
pub fn object(lua: &Lua, o: ObjectRef) -> mlua::Result<AnyUserData> {
    lua.create_userdata(Object(o))
}

/// Wrap the attack state of `o`, as a state of layout `state` (the action
/// it runs), as a script value.
pub fn action_state(lua: &Lua, o: ObjectRef, state: StateId) -> mlua::Result<AnyUserData> {
    lua.create_userdata(State { owner: o, action: true, of_action: Some(state) })
}

/// A hook call's arguments.
pub fn hook_args(lua: &Lua, call: HookCall, bound: &Bound) -> mlua::Result<mlua::MultiValue> {
    let obj = |o: ObjectRef| -> mlua::Result<LuaValue> { Ok(LuaValue::UserData(lua.create_userdata(Object(o))?)) };
    let values = match call {
        HookCall::Weapon { navi } => vec![obj(navi)?],
        HookCall::DimmingChip { user, spec } => {
            let t = lua.create_table()?;
            t.raw_set("element", spec.element)?;
            t.raw_set("params", params_table(lua, spec.params)?)?;
            t.raw_set("damage", spec.damage)?;
            t.raw_set("chip", spec.chip)?;
            t.raw_set("bonus", spec.bonus)?;
            vec![obj(user)?, LuaValue::Table(t)]
        }
        HookCall::NaviChip { user, controller, spec } => {
            let t = lua.create_table()?;
            t.raw_set("panel_x", spec.panel.x)?;
            t.raw_set("panel_y", spec.panel.y)?;
            t.raw_set("element", spec.element)?;
            t.raw_set("params", params_table(lua, spec.params)?)?;
            t.raw_set("damage", spec.damage)?;
            vec![obj(user)?, obj(controller)?, LuaValue::Table(t)]
        }
        HookCall::InstantChip { user, spec } => {
            let t = lua.create_table()?;
            t.raw_set("panel_x", spec.panel.x)?;
            t.raw_set("panel_y", spec.panel.y)?;
            t.raw_set("element", spec.element)?;
            t.raw_set("z", spec.z)?;
            t.raw_set("params", params_table(lua, spec.params)?)?;
            t.raw_set("damage", spec.damage)?;
            vec![obj(user)?, LuaValue::Table(t)]
        }
        HookCall::Place { spec } => {
            let t = lua.create_table()?;
            t.raw_set("panel_x", spec.panel.x)?;
            t.raw_set("panel_y", spec.panel.y)?;
            t.raw_set("side", spec.side)?;
            if let Some(v) = spec.variant {
                t.raw_set("variant", bound.def_value(Registry::Record, v.0)?)?;
            }
            t.raw_set("argument", spec.argument)?;
            vec![LuaValue::Table(t)]
        }
        HookCall::RoleNavi { navi } => vec![obj(navi)?],
        HookCall::RoleEncased { obstacle, ice, class } => {
            let class = class.map_or(LuaValue::Nil, |c| LuaValue::Integer(c as i64));
            vec![obj(obstacle)?, LuaValue::Boolean(ice), class]
        }
    };
    Ok(mlua::MultiValue::from_iter(values))
}

/// A hook's result as the engine takes it.
pub fn hook_result(v: LuaValue, call: HookCall, bound: &Bound) -> mlua::Result<Value> {
    match call {
        // An action: its number (registration by number) or its definition;
        // nothing for a weapon whose own instant effect the engine runs.
        HookCall::Weapon { .. } if v.is_nil() => Ok(Value::Nil),
        HookCall::Weapon { .. } => match bound.def(&v) {
            Some((Registry::Action, h)) => Ok(Value::Def(Registry::Action, h)),
            Some((r, _)) => Err(mlua::Error::runtime(format!("a weapon routine returns an action, not a {r}"))),
            None => Ok(Value::Int(int(&v, "the action a weapon routine returns")? as u8 as i64)),
        },
        HookCall::DimmingChip { .. } | HookCall::NaviChip { .. } | HookCall::Place { .. } => {
            Ok(object_arg(&v, "the object a spawner returns")?.map_or(Value::Nil, Value::Object))
        }
        HookCall::InstantChip { .. } | HookCall::RoleNavi { .. } | HookCall::RoleEncased { .. } => Ok(Value::Nil),
    }
}
