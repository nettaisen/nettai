//! The content API as Luau sees it: handles for objects, sprites,
//! collision, content state and navi stats, the `Vec3` value type, and the
//! libraries `battle`, `field`, `dimming`, `navi_chip` and `int` (declared
//! for editors in content/exe6/core.d.luau).
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

use nettai_content_api::{
    ActorField, ApiError, BattleInfo, CollisionField, CoreApi, DimmingStep, FieldType,
    HitboxSpec, HookCall, HudPart, Key, Lifecycle, LinkedChip, NaviStat, NaviState, OVERLAY_STEPPINGS, ObjectField, ObstacleAction,
    AssetKind, SpawnAt,
    ObstacleCrush, ObstacleRequest, PANEL_TYPES, Pad, PanelPos, Registry, RequestFlag, ScreenFade, SpriteField, SpriteId,
    StateId, StatusFlag, StatusTimer, Value, Vec3,
};
use nettai_content_api::{ObjectRef, RulesHook};
use nettai_content_api::{ChipHandle, CollisionHandle, EffectHandle, RegionHandle, SparkHandle};

use crate::Bound;
// Subtypes 8, 17, 18 (Wind, Anubis, Otenko) and the obstacle framework.
use nettai_content_api::{ObstacleHold, ObstaclePush, WindSource};
use mlua::{AnyUserData, Lua, MetaMethod, UserData, UserDataFields, UserDataMethods, Value as LuaValue};

type Ctx = (NonNull<dyn CoreApi>, NonNull<Bound>);

/// The side a call of the game's rules runs for (docs/design/
/// rules-in-luau.md §5.3): whose rules' state `rules.state()` is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RulesCtx {
    pub side: u8,
}

thread_local! {
    /// The engine, and what the binding reads, of the call running on this
    /// thread.
    static CTX: Cell<Option<Ctx>> = const { Cell::new(None) };
    /// The side the running call is the rules' for, if it is the rules':
    /// what `rules.state()` reaches. Every call sets it (to none for
    /// content's own calls), so the rules' state is out of reach of
    /// anything else a hook of theirs leads to.
    static RULES: Cell<Option<RulesCtx>> = const { Cell::new(None) };
}

/// Makes the engine reachable from script callbacks for one call.
pub struct Enter<'a> {
    prev: Option<Ctx>,
    prev_rules: Option<RulesCtx>,
    _borrow: PhantomData<&'a mut ()>,
}

impl<'a> Enter<'a> {
    /// For a call of the rules' (for a side), or of content's own (none).
    pub fn new(api: &'a mut dyn CoreApi, bound: &'a Bound, rules: Option<RulesCtx>) -> Enter<'a> {
        let api: NonNull<dyn CoreApi + 'a> = NonNull::from(api);
        // SAFETY: only the lifetime is erased. The pointer is reachable
        // (through CTX) only while this guard lives, and the guard borrows
        // `api` exclusively for its whole life.
        let api: NonNull<dyn CoreApi + 'static> = unsafe { std::mem::transmute(api) };
        let prev = CTX.with(|c| c.replace(Some((api, NonNull::from(bound)))));
        let prev_rules = RULES.with(|c| c.replace(rules));
        Enter { prev, prev_rules, _borrow: PhantomData }
    }
}

impl Drop for Enter<'_> {
    fn drop(&mut self) {
        CTX.with(|c| c.set(self.prev));
        RULES.with(|c| c.set(self.prev_rules));
    }
}

/// The side the running call of the rules' is for.
fn rules_ctx(what: &str) -> mlua::Result<RulesCtx> {
    RULES.with(|c| c.get()).ok_or_else(|| {
        mlua::Error::runtime(format!("{what}: only the rules' own calls reach their state (a hook the framework calls)"))
    })
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

/// A sound asset: its handle.
fn sound_arg(v: LuaValue) -> mlua::Result<u16> {
    bound(|b| match b.asset(&v) {
        Some((AssetKind::Sound, h)) => Ok(h),
        _ => Err(mlua::Error::runtime(format!("expected a sound (asset.sound), got {}", v.type_name()))),
    })
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
        LuaValue::Integer(i) => Ok(i64::from(*i)),
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
                    && b.record_type(registry, h) != Some(want.as_str())
                {
                    return Err(mlua::Error::runtime(format!(
                        "{what}: expected a record:{want}, got a record:{}",
                        b.record_type(registry, h).unwrap_or("?")
                    )));
                }
                if let FieldType::Ref(Registry::Entry, Some(want)) = ty
                    && registry == Registry::Entry
                    && b.record_type(registry, h) != Some(want.as_str())
                {
                    return Err(mlua::Error::runtime(format!(
                        "{what}: expected an entry of {want}, got one of {}",
                        b.record_type(registry, h).unwrap_or("?")
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
            FieldType::Code => {
                let s = s.to_str()?;
                match s.as_bytes() {
                    [c] if nettai_content_api::is_code(*c) => Value::Code(*c),
                    _ => return Err(mlua::Error::runtime(format!("{what}: {:?} is no chip code (a letter A to Z, or *)", &*s))),
                }
            }
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
            // (An enum nothing has stated is nil: a player's setup's, which
            // no round starts with.)
            FieldType::Enum(names) => match names.get(i as usize) {
                Some(name) => LuaValue::String(lua.create_string(name)?),
                None => LuaValue::Nil,
            },
            _ => LuaValue::Number(i as f64),
        },
        Value::Object(o) => LuaValue::UserData(lua.create_userdata(Object(o))?),
        Value::Vec3(p) => LuaValue::UserData(lua.create_userdata(LVec3(p))?),
        Value::Def(r, h) => LuaValue::Table(bound(|b| b.def_value(r, h))?),
        Value::Asset(k, h) => LuaValue::Table(bound(|b| b.asset_value(lua, k, h))?),
        Value::Code(c) => LuaValue::String(lua.create_string([c])?),
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

/// A sprite: an asset (`asset.sprite("bomb")`).
fn sprite_id(a: LuaValue) -> mlua::Result<SpriteId> {
    bound(|bd| match bd.asset(&a) {
        Some((AssetKind::Sprite, h)) => Ok(SpriteId(h)),
        _ => Err(mlua::Error::runtime(format!("expected a sprite (asset.sprite), got {}", a.type_name()))),
    })
}

/// A chip definition, or nil: none.
fn chip_arg(b: &Bound, v: &LuaValue, what: &str) -> mlua::Result<Option<nettai_content_api::ChipHandle>> {
    match (v, b.def(v)) {
        (LuaValue::Nil, _) => Ok(None),
        (_, Some((Registry::Chip, h))) => Ok(Some(nettai_content_api::ChipHandle(h))),
        (_, Some((r, _))) => Err(mlua::Error::runtime(format!("{what}: a {r} is not a chip"))),
        _ => Err(mlua::Error::runtime(format!("{what}: expected a chip definition or nil, got {}", v.type_name()))),
    }
}

/// A chip as a script value: its definition, or nil for none.
fn chip_value(b: &Bound, chip: Option<nettai_content_api::ChipHandle>) -> mlua::Result<LuaValue> {
    match chip {
        Some(h) => Ok(LuaValue::Table(b.def_value(Registry::Chip, h.0)?)),
        None => Ok(LuaValue::Nil),
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
                let v = with(|api, _| api.get(this.0, f).map_err(api_error))?;
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
        fields.add_field_method_get("state", |_, this| Ok(State(StateOf::Object(this.0))));
        fields.add_field_method_get("attack_state", |_, this| Ok(State(StateOf::Action(this.0))));
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::Eq, |_, this, other: AnyUserData| {
            Ok(other.borrow::<Object>().is_ok_and(|o| o.0 == this.0))
        });
        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| {
            Ok(format!("Object({} {})", this.0.pool.name(), this.0.slot))
        });

        // Lifecycle.
        methods.add_method("set_lifecycle", |_, this, name: mlua::LuaString| {
            let l = named(&name, "lifecycle state", Lifecycle::from_name)?;
            with(|api, _| Ok(api.set_lifecycle(this.0, l)))
        });
        methods.add_method("set_action", |_, this, a: LuaValue| {
            let a = u8_arg(a, "action")?;
            with(|api, _| api.set_action(this.0, a).map_err(api_error))
        });
        methods.add_method("free", |_, this, ()| with(|api, _| Ok(api.free(this.0))));
        methods.add_method("destroy", |_, this, ()| with(|api, _| Ok(api.destroy(this.0))));
        methods.add_method("add_parts", |_, this, (identity, arg): (LuaValue, LuaValue)| {
            let identity = nettai_content_api::IdentityHandle(bound(|b| def_arg(b, &identity, Registry::Identity, "add_parts"))?);
            let arg = u8_arg(arg, "arg")?;
            with(|api, _| Ok(api.add_parts(this.0, identity, arg)))
        });
        methods.add_method("remove_parts", |_, this, identity: LuaValue| {
            let identity = nettai_content_api::IdentityHandle(bound(|b| def_arg(b, &identity, Registry::Identity, "remove_parts"))?);
            with(|api, _| Ok(api.remove_parts(this.0, identity)))
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
        methods.add_method("hide_from_blind", |_, this, ()| with(|api, _| Ok(api.hide_from_blind(this.0))));
        methods.add_method("copy_visibility", |_, this, from: mlua::UserDataRef<Object>| {
            with(|api, _| Ok(api.copy_visibility(from.0, this.0)))
        });
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
        methods.add_method("chips_enabled", |_, this, ()| with(|api, _| api.chips_enabled(this.0).map_err(api_error)));
        methods.add_method("move_lag", |_, this, ()| with(|api, _| api.move_lag(this.0).map_err(api_error)));
        methods.add_method("drop_links", |_, this, ()| with(|api, _| api.drop_links(this.0).map_err(api_error)));
        methods.add_method("drop_status_visuals", |_, this, ()| with(|api, _| api.drop_status_visuals(this.0).map_err(api_error)));
        methods.add_method("drop_chip", |_, this, ()| with(|api, _| api.drop_chip(this.0).map_err(api_error)));
        methods.add_method("drop_alive_count", |_, this, ()| with(|api, _| api.drop_alive_count(this.0).map_err(api_error)));
        methods.add_method("drop_barrier", |_, this, ()| with(|api, _| api.drop_barrier(this.0).map_err(api_error)));
        methods.add_method("leave", |_, this, ()| with(|api, _| api.leave(this.0).map_err(api_error)));
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
            let sprite = sprite_id(sprite)?;
            with(|api, _| api.name_look_is(this.0, sprite).map_err(api_error))
        });
        methods.add_method("raise_barrier", |_, this, t: mlua::Table| {
            let behavior: mlua::LuaString = t.raw_get("behavior")?;
            let behavior = named(&behavior, "barrier behavior", |s| {
                nettai_content_api::api::BARRIER_BEHAVIORS.iter().position(|n| *n == s)
            })? as u8;
            let byte = |k: &str| -> mlua::Result<u8> {
                u8::try_from(table_int(&t, k)?).map_err(|_| mlua::Error::runtime(format!("barrier `{k}` is not a byte")))
            };
            let timer = u16::try_from(table_int(&t, "timer")?)
                .map_err(|_| mlua::Error::runtime("barrier `timer` is not a halfword"))?;
            let spec = nettai_content_api::api::BarrierSpec {
                behavior,
                hp: byte("hp")?,
                threshold: byte("threshold")?,
                timer,
                weak_element: byte("weak_element")?,
            };
            with(|api, _| api.raise_barrier(this.0, spec).map_err(api_error))
        });

        // Navis: the attack, requests, state, buttons.
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
        methods.add_method("open_counter_window", |_, this, ticks: Option<u8>| {
            with(|api, _| Ok(api.open_counter_window(this.0, ticks.unwrap_or(0x10))))
        });
        methods.add_method("check_reactive_abort", |_, this, ()| with(|api, _| Ok(api.check_reactive_abort(this.0))));
        methods.add_method("spawn_mode9_objects", |_, this, ()| with(|api, _| Ok(api.spawn_mode9_objects(this.0))));
        methods.add_method("start_stance_counter", |_, this, ()| with(|api, _| Ok(api.start_stance_counter(this.0))));
        methods.add_method("refresh_form_overlay", |_, this, ()| with(|api, _| Ok(api.refresh_form_overlay(this.0))));
        methods.add_method("exit_attack", |_, this, ()| with(|api, _| Ok(api.exit_attack(this.0))));
        // What a game's rules do to a navi (docs/design/rules-in-luau.md §4.5).
        methods.add_method("clear_invulnerable", |_, this, ()| with(|api, _| api.clear_invulnerable(this.0).map_err(api_error)));
        methods.add_method("face_default", |_, this, ()| with(|api, _| api.face_default(this.0).map_err(api_error)));
        methods.add_method("reset_charge", |_, this, ()| with(|api, _| api.reset_charge(this.0).map_err(api_error)));
        methods.add_method("end_full_synchro_aura", |_, this, ()| {
            with(|api, _| api.end_full_synchro_aura(this.0).map_err(api_error))
        });
        methods.add_method("drop_statuses", |_, this, ()| with(|api, _| api.drop_statuses(this.0).map_err(api_error)));
        methods.add_method("end_statuses", |_, this, ()| with(|api, _| api.end_statuses(this.0).map_err(api_error)));
        methods.add_method("set_overlay_anim_offset", |_, this, offset: u8| {
            with(|api, _| Ok(api.set_overlay_anim_offset(this.0, offset)))
        });
        methods.add_method("overlay_stepping", |_, this, mode: mlua::LuaString| {
            let keep = match &*mode.to_str()? {
                "keep" => true,
                "normal" => false,
                other => return Err(mlua::Error::runtime(format!("overlay_stepping: {other:?} is neither \"keep\" nor \"normal\""))),
            };
            with(|api, _| Ok(api.overlay_stepping(this.0, keep)))
        });
        methods.add_method("take_off_form_overlay", |_, this, form: LuaValue| {
            let form = nettai_content_api::FormHandle(bound(|b| def_arg(b, &form, Registry::Form, "take_off_form_overlay"))?);
            with(|api, _| Ok(api.take_off_form_overlay(this.0, form)))
        });
        methods.add_method("put_on_form_overlay", |_, this, form: LuaValue| {
            let form = nettai_content_api::FormHandle(bound(|b| def_arg(b, &form, Registry::Form, "put_on_form_overlay"))?);
            with(|api, _| Ok(api.put_on_form_overlay(this.0, form)))
        });
        methods.add_method("take_off_form_parts", |_, this, form: LuaValue| {
            let form = nettai_content_api::FormHandle(bound(|b| def_arg(b, &form, Registry::Form, "take_off_form_parts"))?);
            with(|api, _| Ok(api.take_off_form_parts(this.0, form)))
        });
        methods.add_method("put_on_form_parts", |_, this, form: LuaValue| {
            let form = nettai_content_api::FormHandle(bound(|b| def_arg(b, &form, Registry::Form, "put_on_form_parts"))?);
            with(|api, _| Ok(api.put_on_form_parts(this.0, form)))
        });
        methods.add_method("load_form_sprite", |_, this, form: LuaValue| {
            let form = nettai_content_api::FormHandle(bound(|b| def_arg(b, &form, Registry::Form, "load_form_sprite"))?);
            with(|api, _| api.load_form_sprite(this.0, form).map_err(api_error))
        });
        methods.add_method("reset_status", |_, this, ()| with(|api, _| api.reset_status(this.0).map_err(api_error)));
        methods.add_method("strip_body_programs", |_, this, undershirt: bool| {
            with(|api, _| api.strip_body_programs(this.0, undershirt).map_err(api_error))
        });
        methods.add_method("refresh_form_flags", |_, this, ()| with(|api, _| api.refresh_form_flags(this.0).map_err(api_error)));
        methods.add_method("take_status", |_, this, status: LuaValue| {
            let status = nettai_content_api::StatusHandle(bound(|b| def_arg(b, &status, Registry::Status, "take_status"))?);
            with(|api, _| api.take_status(this.0, status).map_err(api_error))
        });
        methods.add_method("end_anger", |_, this, ()| with(|api, _| api.end_anger(this.0).map_err(api_error)));
        methods.add_method("form_change_target", |_, this, ()| {
            let form = with(|api, _| Ok(api.form_change_target(this.0)))?;
            match form {
                Some(h) => Ok(LuaValue::Table(bound(|b| b.def_value(Registry::Form, h.0))?)),
                None => Ok(LuaValue::Nil),
            }
        });
        methods.add_method("form_change_terms", |_, this, ()| with(|api, _| Ok(api.form_change_terms(this.0))));
        methods.add_method("stop_moving", |_, this, ()| with(|api, _| api.stop_moving(this.0).map_err(api_error)));
        methods.add_method("pin_overlay", |_, this, ()| with(|api, _| Ok(api.pin_overlay(this.0))));
        methods.add_method("end_attack", |_, this, ()| with(|api, _| Ok(api.end_attack(this.0))));
        methods.add_method("set_attack", |_, this, (action, kind): (LuaValue, LuaValue)| {
            let kind = u8_arg(kind, "attack kind")?;
            with(|api, b| match b.def(&action) {
                Some((Registry::Action, h)) => api.set_content_attack(this.0, h, kind).map_err(api_error),
                Some((r, _)) => Err(mlua::Error::runtime(format!("set_attack: a {r} is not an action"))),
                None => Err(mlua::Error::runtime(format!(
                    "set_attack: expected an action definition, got {}",
                    action.type_name()
                ))),
            })
        });
        methods.add_method("navi_action", |lua, this, ()| {
            let a = with(|api, _| api.navi_action(this.0).map_err(api_error))?;
            match a {
                nettai_content_api::NaviAction::Content(h) => {
                    from_api(lua, Value::Def(Registry::Action, h), &FieldType::Ref(Registry::Action, None))
                }
                nettai_content_api::NaviAction::Engine(name) => Ok(LuaValue::String(lua.create_string(name)?)),
            }
        });
        methods.add_method("rush_cancels", |_, this, chip: LuaValue| {
            let chip = bound(|b| def_arg(b, &chip, Registry::Chip, "rush_cancels"))?;
            with(|api, _| api.rush_cancels(this.0, chip).map_err(api_error))
        });
        methods.add_method("load_chip_attack", |_, this, chip: LuaValue| {
            let chip = bound(|b| def_arg(b, &chip, Registry::Chip, "load_chip_attack"))?;
            with(|api, _| api.load_chip_attack(this.0, chip).map_err(api_error))
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
        methods.add_method("can_move", |_, this, ()| with(|api, _| Ok(api.can_move(this.0))));
        // A controller's (the rules' `controller` hook).
        methods.add_method("next_chip", |_, this, ()| {
            let chip = with(|api, _| api.next_chip(this.0).map_err(api_error))?;
            bound(|b| chip.map_or(Ok(LuaValue::Nil), |c| Ok(LuaValue::Table(b.def_value(Registry::Chip, c.0)?))))
        });
        methods.add_method("use_chip", |_, this, ()| with(|api, _| api.use_chip(this.0).map_err(api_error)));
        methods.add_method("start_chip_attack", |_, this, (chip, kind): (LuaValue, LuaValue)| {
            let chip = bound(|b| def_arg(b, &chip, Registry::Chip, "start_chip_attack"))?;
            let kind = int(&kind, "kind")? as u8;
            with(|api, _| api.start_chip_attack(this.0, nettai_content_api::ChipHandle(chip), kind).map_err(api_error))
        });
        methods.add_method("start_move_to", |_, this, (x, y, end_lag, face): (LuaValue, LuaValue, LuaValue, LuaValue)| {
            let p = panel(x, y)?;
            let end_lag = int(&end_lag, "end_lag")? as u16;
            let face = object_arg(&face, "face")?;
            with(|api, _| api.start_move_to(this.0, p, end_lag, face).map_err(api_error))
        });
        methods.add_method("start_weapon", |_, this, (weapon, kind): (LuaValue, LuaValue)| {
            let weapon = bound(|b| def_arg(b, &weapon, Registry::Weapon, "start_weapon"))?;
            let kind = int(&kind, "kind")? as u8;
            with(|api, _| api.start_weapon(this.0, nettai_content_api::WeaponHandle(weapon), kind).map_err(api_error))
        });
        // The wrapper's (the role `actions.wrapper`).
        methods.add_method("run_wrapped", |_, this, ()| with(|api, _| api.run_wrapped(this.0).map_err(api_error)));
        methods.add_method("chain_next_chip", |_, this, ()| with(|api, _| api.chain_next_chip(this.0).map_err(api_error)));
        methods.add_method("panel_trail", |_, this, (x, y): (LuaValue, LuaValue)| {
            let p = panel(x, y)?;
            with(|api, _| api.panel_trail(this.0, p).map_err(api_error))
        });
        methods.add_method("freeze_target_marker", |_, this, on: bool| {
            with(|api, _| api.freeze_target_marker(this.0, on).map_err(api_error))
        });
        methods.add_method("face_toward", |_, this, target: LuaValue| {
            let target = object_arg(&target, "face_toward")?.ok_or_else(|| mlua::Error::runtime("face_toward: expected an Object"))?;
            with(|api, _| api.face_toward(this.0, target).map_err(api_error))
        });
        methods.add_method("wear_navi_image", |_, this, (user, megaman): (mlua::UserDataRef<Object>, LuaValue)| {
            let megaman = nettai_content_api::NaviHandle(bound(|b| def_arg(b, &megaman, Registry::Navi, "wear_navi_image"))?);
            with(|api, _| api.wear_navi_image(this.0, user.0, megaman).map_err(api_error))
        });
        methods.add_method("wear_form_image", |_, this, (navi, form): (LuaValue, LuaValue)| {
            let navi = nettai_content_api::NaviHandle(bound(|b| def_arg(b, &navi, Registry::Navi, "wear_form_image"))?);
            let form = nettai_content_api::FormHandle(bound(|b| def_arg(b, &form, Registry::Form, "wear_form_image"))?);
            with(|api, _| api.wear_form_image(this.0, navi, form).map_err(api_error))
        });
        methods.add_method("navi_image_parts", |_, this, on: bool| with(|api, _| Ok(api.navi_image_parts(this.0, on))));
        methods.add_method("wear_absorbed_look", |_, this, look: LuaValue| {
            let look = bound(|b| match b.def(&look) {
                Some((Registry::Identity, h)) => Ok(nettai_content_api::IdentityHandle(h)),
                Some((r, _)) => Err(mlua::Error::runtime(format!("wear_absorbed_look: a {r} is not an identity"))),
                None => Err(mlua::Error::runtime(format!("wear_absorbed_look: expected an identity, got {}", look.type_name()))),
            })?;
            with(|api, _| api.wear_absorbed_look(this.0, look).map_err(api_error))
        });
        methods.add_method("subtract_hp", |_, this, amount: LuaValue| {
            let amount = u16_arg(amount, "HP")?;
            with(|api, _| Ok(api.subtract_hp(this.0, amount)))
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
                    Some((Registry::Action, h)) => h,
                    Some((r, _)) => return Err(mlua::Error::runtime(format!("action_state: a {r} is not an action"))),
                    None => {
                        return Err(mlua::Error::runtime(format!(
                            "action_state: expected an action definition, got {}",
                            action.type_name()
                        )));
                    }
                };
                api.action_schema(action).map_err(api_error)
            })?;
            Ok(State(StateOf::ActionAs(this.0, id)))
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
            let f = named(&name, "obstacle flag", nettai_content_api::api::ObstacleFlag::from_name)?;
            with(|api, _| api.obstacle_flag(this.0, f).map_err(api_error))
        });
    }
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
        methods.add_method("load", |_, this, sprite: LuaValue| {
            let id = sprite_id(sprite)?;
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
pub struct State(pub StateOf);

/// Whose content state a [`State`] is.
#[derive(Clone, Copy, Debug)]
pub enum StateOf {
    /// An object's (`me.state`).
    Object(ObjectRef),
    /// The running action's (`me.attack_state`, an action update's second
    /// argument).
    Action(ObjectRef),
    /// The attack state as a state of this layout (an action's update's
    /// own, `navi:action_state(action)`), rather than the running action's.
    ActionAs(ObjectRef, StateId),
    /// The rules' state of a side (`rules.state()`).
    Rules(RulesCtx),
    /// A side's player's setup of the rules (`rules.setup()`): read-only.
    Setup(RulesCtx),
    /// A side's navi's stats of its game's own (the rules' `stats`: the
    /// names `battle.navi(side)` has beside the engine's).
    Stats(u8),
}

impl State {
    fn with_state<R>(
        self,
        key: &str,
        write: bool,
        f: impl FnOnce(&mut dyn nettai_content_api::Fields, &nettai_content_api::Schema, usize) -> mlua::Result<R>,
    ) -> mlua::Result<R> {
        with(|api, bound| {
            // (A setup is read through a copy: it can't be written. An
            // object's or an action's state is a view of its block in an
            // arena.)
            let mut setup;
            let mut view;
            let s: &mut dyn nettai_content_api::Fields = match self.0 {
                StateOf::Object(o) => {
                    view = api.state_mut(o).ok_or_else(|| api_error(ApiError::NoState(o)))?;
                    &mut view
                }
                StateOf::Action(o) => {
                    view = api.action_state_mut(o).map_err(api_error)?;
                    &mut view
                }
                StateOf::ActionAs(o, id) => {
                    view = api.attack_state_for(o, id).map_err(api_error)?;
                    &mut view
                }
                StateOf::Rules(c) => api.rules_state_mut(c.side).map_err(api_error)?,
                StateOf::Stats(side) => api.navi_game_stats_mut(side).map_err(api_error)?,
                StateOf::Setup(_) if write => {
                    return Err(mlua::Error::runtime(format!(
                        "setup field `{key}`: a player's setup is read-only in battle"
                    )));
                }
                StateOf::Setup(c) => {
                    setup = api.rules_setup(c.side).map_err(api_error)?.clone();
                    &mut setup
                }
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
            let got = this.with_state(&key, false, |s, schema, i| {
                let ty = schema.field(i).ty.clone();
                Ok(ty.is_scalar().then(|| (s.get(schema, i).load(), ty)))
            })?;
            match got {
                Some((v, ty)) => from_api(lua, v, &ty),
                None => Ok(LuaValue::UserData(lua.create_userdata(StatePart {
                    state: State(this.0),
                    field: key.to_string(),
                    path: Vec::new(),
                })?)),
            }
        });
        methods.add_meta_method(MetaMethod::NewIndex, |_, this, (key, v): (mlua::LuaString, LuaValue)| {
            let key = key.to_str()?;
            this.with_state(&key, true, |s, schema, i| assign(s, schema.place(i), v, &format!("state field `{}`", &*key)))
        });
    }
}

/// Store `v` at `place` of state `s`, as the place's type takes it: a
/// value; a record from a table of its fields (those it leaves out zero);
/// an array or a list from a table of its elements, from the first (a
/// list's length the table's, an array's rest zero). `name` says the place
/// in a message.
fn assign(s: &mut dyn nettai_content_api::Fields, place: nettai_content_api::Place, v: LuaValue, name: &str) -> mlua::Result<()> {
    let ty = place.ty();
    if ty.is_scalar() {
        let v = to_api(v, ty, name)?;
        return s.set_at(place, v).map_err(|e| mlua::Error::runtime(format!("{name}: {e}")));
    }
    let LuaValue::Table(t) = v else {
        return Err(mlua::Error::runtime(format!("{name} is a {ty}: give it a table, not a {}", v.type_name())));
    };
    match ty {
        FieldType::Record(fields) => {
            for pair in t.clone().pairs::<LuaValue, LuaValue>() {
                let (k, _) = pair?;
                let known = match &k {
                    LuaValue::String(k) => fields.index_of(&k.to_str()?).is_some(),
                    _ => false,
                };
                if !known {
                    let names: Vec<&str> = fields.fields().iter().map(|f| f.name.as_str()).collect();
                    return Err(mlua::Error::runtime(format!("{name} has no field {k:?} ({})", names.join(", "))));
                }
            }
            s.clear_at(place);
            for (i, f) in fields.fields().iter().enumerate() {
                let x: LuaValue = t.raw_get(f.name.as_str())?;
                if !x.is_nil() {
                    assign(s, place.field_at(i), x, &format!("{name}.{}", f.name))?;
                }
            }
            Ok(())
        }
        FieldType::Array(..) | FieldType::List(..) => {
            let n = t.raw_len();
            let room = place.capacity().expect("an array or a list");
            if n > room {
                return Err(mlua::Error::runtime(format!("{name} holds {room}, not {n}")));
            }
            s.clear_at(place);
            if matches!(ty, FieldType::List(..)) {
                s.set_len_at(place, n).map_err(mlua::Error::runtime)?;
            }
            for k in 0..n {
                let x: LuaValue = t.raw_get(k + 1)?;
                assign(s, place.elem(k).expect("within its room"), x, &format!("{name}[{}]", k + 1))?;
            }
            Ok(())
        }
        _ => unreachable!("a scalar"),
    }
}

/// A step into a part of a state field: a record's field, an element.
#[derive(Clone, Debug)]
enum Step {
    Field(String),
    Elem(usize),
}

/// A part of a content state field that holds parts: an array's elements
/// (`s.targets[i]`), a list's (`s.folder[i]`, `#s.folder`), a record's
/// fields (`s.folder[i].code`), as deep as the schema nests them. Indexes
/// are 1-based like Luau arrays. A list grows by its next element
/// (`s.list[#s.list + 1] = v`); a record, a list or an array takes a table
/// whole (`s.folder[2] = { chip = c, code = "A" }`, [`assign`]).
pub struct StatePart {
    state: State,
    field: String,
    path: Vec<Step>,
}

impl StatePart {
    /// What it is called in a message: `folder[2].code`.
    fn name(&self, next: Option<&Step>) -> String {
        let mut out = self.field.clone();
        for step in self.path.iter().chain(next) {
            match step {
                Step::Field(f) => out.push_str(&format!(".{f}")),
                Step::Elem(k) => out.push_str(&format!("[{}]", k + 1)),
            }
        }
        out
    }

    /// Run `f` on the state with this part's place.
    fn with_place<R>(
        &self,
        write: bool,
        f: impl FnOnce(&mut dyn nettai_content_api::Fields, nettai_content_api::Place) -> mlua::Result<R>,
    ) -> mlua::Result<R> {
        self.state.with_state(&self.field, write, |s, schema, i| {
            let mut place = schema.place(i);
            for (n, step) in self.path.iter().enumerate() {
                place = match step {
                    Step::Field(name) => place.field(name),
                    Step::Elem(k) => place.elem(*k),
                }
                .ok_or_else(|| mlua::Error::runtime(format!("{}: no such part", self.name(self.path.get(n)))))?;
            }
            f(s, place)
        })
    }

    /// The step `key` names into this part (a record's field by name, an
    /// element by its 1-based index), and whether the element is one the
    /// list may take next (one past its end, for a write).
    fn step(&self, key: &LuaValue, s: &dyn nettai_content_api::Fields, place: nettai_content_api::Place, write: bool) -> mlua::Result<Step> {
        match place.ty() {
            FieldType::Record(fields) => {
                let LuaValue::String(name) = key else {
                    return Err(mlua::Error::runtime(format!("{}: a record's fields have names", self.name(None))));
                };
                let name = name.to_str()?.to_string();
                if fields.index_of(&name).is_none() {
                    let names: Vec<&str> = fields.fields().iter().map(|f| f.name.as_str()).collect();
                    return Err(mlua::Error::runtime(format!("{} has no field `{name}` ({})", self.name(None), names.join(", "))));
                }
                Ok(Step::Field(name))
            }
            FieldType::Array(..) | FieldType::List(..) => {
                let k = int(key, &self.name(None))?;
                let len = s.len_at(place).expect("an array or a list") as i64;
                let room = place.capacity().expect("an array or a list") as i64;
                let last = if write && matches!(place.ty(), FieldType::List(..)) { (len + 1).min(room) } else { len };
                if k < 1 || k > last {
                    let what = if matches!(place.ty(), FieldType::List(..)) { "list" } else { "array" };
                    return Err(mlua::Error::runtime(format!("{}[{k}] is past the {what}'s end ({len})", self.name(None))));
                }
                Ok(Step::Elem(k as usize - 1))
            }
            ty => Err(mlua::Error::runtime(format!("{}: a {ty} has no parts", self.name(None)))),
        }
    }
}

impl StatePart {
    /// `this[key]`: a scalar's value, or the part a record's field or a
    /// list's element is.
    fn index(lua: &Lua, this: &StatePart, key: LuaValue) -> mlua::Result<LuaValue> {
        let got = this.with_place(false, |s, place| {
            let step = this.step(&key, s, place, false)?;
            let at = match &step {
                Step::Field(name) => place.field(name),
                Step::Elem(k) => place.elem(*k),
            }
            .expect("a step into the part");
            let ty = at.ty().clone();
            Ok((step, ty.is_scalar().then(|| (s.get_at(at).load(), ty))))
        })?;
        match got {
            (_, Some((v, ty))) => from_api(lua, v, &ty),
            (step, None) => {
                let mut path = this.path.clone();
                path.push(step);
                Ok(LuaValue::UserData(lua.create_userdata(StatePart { state: State(this.state.0), field: this.field.clone(), path })?))
            }
        }
    }

    /// How many elements the list or array this part is holds.
    fn len(&self) -> mlua::Result<usize> {
        self.with_place(false, |s, place| s.len_at(place).ok_or_else(|| mlua::Error::runtime(format!("{}: a record has no length", self.name(None)))))
    }
}

impl UserData for StatePart {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::Index, |lua, this, key: LuaValue| StatePart::index(lua, this, key));
        // `for i, x in list`: a list's or an array's elements from the
        // first, as a table's.
        methods.add_meta_method(MetaMethod::Iter, |lua, this, ()| {
            this.len()?;
            let next = lua.create_function(|lua, (part, i): (AnyUserData, mlua::Integer)| {
                let this = part.borrow::<StatePart>()?;
                if i < 0 || i as usize >= this.len()? {
                    return Ok((LuaValue::Nil, LuaValue::Nil));
                }
                Ok((LuaValue::Integer(i + 1), StatePart::index(lua, &this, LuaValue::Integer(i + 1))?))
            })?;
            let me = lua.create_userdata(StatePart { state: State(this.state.0), field: this.field.clone(), path: this.path.clone() })?;
            Ok((next, me, 0i64))
        });
        methods.add_meta_method(MetaMethod::NewIndex, |_, this, (key, v): (LuaValue, LuaValue)| {
            this.with_place(true, |s, place| {
                let step = this.step(&key, s, place, true)?;
                let name = this.name(Some(&step));
                if let (Step::Elem(k), FieldType::List(..)) = (&step, place.ty())
                    && Some(*k) == s.len_at(place)
                {
                    s.set_len_at(place, k + 1).map_err(mlua::Error::runtime)?;
                }
                let at = match &step {
                    Step::Field(f) => place.field(f),
                    Step::Elem(k) => place.elem(*k),
                }
                .expect("a step into the part");
                assign(s, at, v, &name)
            })
        });
        methods.add_meta_method(MetaMethod::Len, |_, this, ()| this.len().map(|n| n as i64));
    }
}

/// A side's navi stats (`battle.navi(side)`): the engine's by their names,
/// and its game's own (the rules' `stats`) by theirs, as a state's fields.
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

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // (A name the engine's stats haven't: the game's own.)
        methods.add_meta_method(MetaMethod::Index, |lua, this, key: mlua::LuaString| {
            let key = key.to_str()?;
            let got = State(StateOf::Stats(this.0)).with_state(&key, false, |s, schema, i| {
                let ty = schema.field(i).ty.clone();
                Ok((s.get(schema, i).load(), ty))
            })?;
            from_api(lua, got.0, &got.1)
        });
        methods.add_meta_method(MetaMethod::NewIndex, |_, this, (key, v): (mlua::LuaString, LuaValue)| {
            let key = key.to_str()?;
            State(StateOf::Stats(this.0)).with_state(&key, true, |s, schema, i| assign(s, schema.place(i), v, &format!("stat `{}`", &*key)))
        });
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

/// Spawn a kind (a definition, or the stand-in of one of the engine's).
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
    lib_fn!(lua, navi_chip, "last", |lua, ()| {
        let Some((chip, element, damage)) = with(|api, _| Ok(api.last_navi_chip()))? else {
            return Ok(LuaValue::Nil);
        };
        let t = lua.create_table()?;
        t.raw_set("chip", bound(|b| chip_value(b, Some(chip)))?)?;
        t.raw_set("element", element)?;
        t.raw_set("damage", damage)?;
        Ok(LuaValue::Table(t))
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
    g.set("schema", schema_lib(lua)?)?;
    g.set("rules", rules_lib(lua)?)?;
    g.set("custom", custom_lib(lua)?)?;
    Ok(())
}

/// `schema`: what a state's or a setup's declaration takes beside type
/// names, variants and records (docs/design/rust-and-luau.md, step c1):
/// `schema.list(T, n)`, a list of up to `n` elements of type `T` (a type
/// name, a list of variants, a record's table or another list), and
/// `schema.role(name, T)`, a field of type `T` the engine knows by the
/// role `name` (a player's setup field: `PlayerFact`).
fn schema_lib(lua: &Lua) -> mlua::Result<mlua::Table> {
    let t = lua.create_table()?;
    lib_fn!(lua, t, "list", |lua, (of, max): (LuaValue, LuaValue)| {
        let max = int(&max, "schema.list's length")?;
        if !(1..=nettai_content_api::MAX_LIST as i64).contains(&max) {
            return Err(mlua::Error::runtime(format!("schema.list holds 1 to {} elements, not {max}", nettai_content_api::MAX_LIST)));
        }
        if !matches!(of, LuaValue::String(_) | LuaValue::Table(_)) {
            return Err(mlua::Error::runtime(format!("schema.list's elements are a type, not a {}", of.type_name())));
        }
        let list = lua.create_table()?;
        list.raw_set(nettai_content_api::LIST_MARK, true)?;
        list.raw_set("of", of)?;
        list.raw_set("max", max)?;
        Ok(list)
    });
    lib_fn!(lua, t, "role", |lua, (role, of): (LuaValue, LuaValue)| {
        let LuaValue::String(role) = role else {
            return Err(mlua::Error::runtime(format!("schema.role's role is a name, not a {}", role.type_name())));
        };
        if !matches!(of, LuaValue::String(_) | LuaValue::Table(_)) {
            return Err(mlua::Error::runtime(format!("schema.role's field is a type, not a {}", of.type_name())));
        }
        let field = lua.create_table()?;
        field.raw_set(nettai_content_api::ROLE_MARK, true)?;
        field.raw_set("role", role)?;
        field.raw_set("of", of)?;
        Ok(field)
    });
    Ok(t)
}

/// `custom`: a side's custom screen, in the rules' custom hooks
/// (docs/design/rules-in-luau.md §4.4).
fn custom_lib(lua: &Lua) -> mlua::Result<mlua::Table> {
    let t = lua.create_table()?;
    lib_fn!(lua, t, "refuse", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| api.custom_refuse(side).map_err(api_error))
    });
    lib_fn!(lua, t, "sacrifice", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| api.custom_sacrifice(side).map_err(api_error))
    });
    lib_fn!(lua, t, "folder", |lua, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        let chips = with(|api, _| api.custom_folder(side).map_err(api_error))?;
        let list = lua.create_table()?;
        for (i, chip) in chips.into_iter().enumerate() {
            let v = match chip {
                Some(_) => bound(|b| chip_value(b, chip))?,
                None => LuaValue::Boolean(false),
            };
            list.raw_set(i + 1, v)?;
        }
        Ok(list)
    });
    lib_fn!(lua, t, "swap_folder", |_, (side, a, b): (LuaValue, LuaValue, LuaValue)| {
        let side = u8_arg(side, "side")? & 1;
        let place = |v: &LuaValue, what: &str| -> mlua::Result<u8> {
            let n = int(v, what)?;
            if !(1..=30).contains(&n) {
                return Err(mlua::Error::runtime(format!("custom.swap_folder: {what} is a place of the folder, 1 to 30, not {n}")));
            }
            Ok((n - 1) as u8)
        };
        let (a, b) = (place(&a, "a")?, place(&b, "b")?);
        with(|api, _| api.custom_swap_folder(side, a, b).map_err(api_error))
    });
    lib_fn!(lua, t, "offer", |_, (side, slot, chip, code): (LuaValue, LuaValue, LuaValue, LuaValue)| {
        let side = u8_arg(side, "side")? & 1;
        let slot = u8_arg(slot, "slot")?;
        let chip = bound(|b| chip_arg(b, &chip, "chip"))?.ok_or_else(|| mlua::Error::runtime("custom.offer: no chip"))?;
        let code = match &code {
            LuaValue::Nil => None,
            LuaValue::String(s) => {
                let s = s.to_str()?.to_string();
                match s.as_bytes() {
                    [b'*'] => Some(26),
                    [c] if c.is_ascii_uppercase() => Some(c - b'A'),
                    _ => return Err(mlua::Error::runtime(format!("custom.offer: {s:?} is not a chip code (A-Z or *)"))),
                }
            }
            other => return Err(mlua::Error::runtime(format!("custom.offer: a code is a letter or nil, not {}", other.type_name()))),
        };
        with(|api, _| api.custom_offer(side, slot, chip, code).map_err(api_error))
    });
    lib_fn!(lua, t, "hand_size", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| api.custom_hand_size(side).map_err(api_error))
    });
    lib_fn!(lua, t, "redeal", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| api.custom_redeal(side).map_err(api_error))
    });
    lib_fn!(lua, t, "last_pick_is_chip", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| api.custom_last_pick_is_chip(side).map_err(api_error))
    });
    lib_fn!(lua, t, "cursor_state", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| api.custom_cursor_state(side).map_err(api_error))
    });
    // (A button, a window or a form a call names is the rules': only their
    // calls make it.)
    let own = |what: &str| -> mlua::Result<()> { rules_ctx(what).map(|_| ()) };
    let chip_or_nil = |v: &LuaValue, what: &str| -> mlua::Result<Option<nettai_content_api::ChipHandle>> {
        if v.is_nil() { Ok(None) } else { Ok(Some(nettai_content_api::ChipHandle(bound(|b| def_arg(b, v, Registry::Chip, what))?))) }
    };
    let form_or_nil = |v: &LuaValue, what: &str| -> mlua::Result<Option<nettai_content_api::FormHandle>> {
        if v.is_nil() { Ok(None) } else { Ok(Some(nettai_content_api::FormHandle(bound(|b| def_arg(b, v, Registry::Form, what))?))) }
    };
    lib_fn!(lua, t, "pick", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| api.custom_pick(side).map_err(api_error))
    });
    lib_fn!(lua, t, "play", |_, (side, sound): (LuaValue, String)| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| api.custom_play(side, &sound).map_err(api_error))
    });
    lib_fn!(lua, t, "play_sound", |_, (side, id): (LuaValue, LuaValue)| {
        let (side, id) = (u8_arg(side, "side")? & 1, sound_arg(id)?);
        with(|api, _| api.custom_play_sound(side, id).map_err(api_error))
    });
    lib_fn!(lua, t, "set_column_icon", move |_, (side, chip): (LuaValue, LuaValue)| {
        let side = u8_arg(side, "side")? & 1;
        let chip = chip_or_nil(&chip, "custom.set_column_icon")?;
        with(|api, _| api.custom_set_column_icon(side, chip).map_err(api_error))
    });
    lib_fn!(lua, t, "open_window", move |_, (side, window, ticks): (LuaValue, String, LuaValue)| {
        let side = u8_arg(side, "side")? & 1;
        own("custom.open_window")?;
        let ticks = if ticks.is_nil() { 0 } else { int(&ticks, "ticks")? as u16 };
        with(|api, _| api.custom_open_window(side, &window, ticks).map_err(api_error))
    });
    lib_fn!(lua, t, "window_tick", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| api.custom_window_tick(side).map_err(api_error))
    });
    lib_fn!(lua, t, "shake", |_, (side, magnitude, ticks): (LuaValue, LuaValue, LuaValue)| {
        let side = u8_arg(side, "side")? & 1;
        let (magnitude, ticks) = (int(&magnitude, "magnitude")? as u16, int(&ticks, "ticks")? as u16);
        with(|api, _| api.custom_shake(side, magnitude, ticks).map_err(api_error))
    });
    lib_fn!(lua, t, "frame", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| api.custom_frame(side).map_err(api_error))
    });
    lib_fn!(lua, t, "set_frame", |_, (side, frame): (LuaValue, LuaValue)| {
        let side = u8_arg(side, "side")? & 1;
        let frame = int(&frame, "frame")? as u32;
        with(|api, _| api.custom_set_frame(side, frame).map_err(api_error))
    });
    lib_fn!(lua, t, "spin", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| api.custom_spin(side).map_err(api_error))
    });
    lib_fn!(lua, t, "fade", |_, (side, mode, speed): (LuaValue, String, LuaValue)| {
        let side = u8_arg(side, "side")? & 1;
        let speed = int(&speed, "speed")? as u8;
        with(|api, _| api.custom_fade(side, &mode, speed).map_err(api_error))
    });
    lib_fn!(lua, t, "set_face", move |_, (side, form): (LuaValue, LuaValue)| {
        let side = u8_arg(side, "side")? & 1;
        let form = form_or_nil(&form, "custom.set_face")?;
        with(|api, _| api.custom_set_face(side, form).map_err(api_error))
    });
    lib_fn!(lua, t, "pick_first", move |_, (side, icon): (LuaValue, LuaValue)| {
        let side = u8_arg(side, "side")? & 1;
        let icon = chip_or_nil(&icon, "custom.pick_first")?;
        with(|api, _| api.custom_pick_first(side, icon).map_err(api_error))
    });
    lib_fn!(lua, t, "set_button_state", move |_, (side, button, state): (LuaValue, String, String)| {
        let side = u8_arg(side, "side")? & 1;
        own("custom.set_button_state")?;
        with(|api, _| api.custom_set_button_state(side, &button, &state).map_err(api_error))
    });
    lib_fn!(lua, t, "update_availability", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| api.custom_update_availability(side).map_err(api_error))
    });
    lib_fn!(lua, t, "draw_emblem", |_, (side, x): (LuaValue, LuaValue)| {
        let side = u8_arg(side, "side")? & 1;
        let x = int(&x, "x")? as u32;
        with(|api, _| api.custom_draw_emblem(side, x).map_err(api_error))
    });
    // `by`: the rules' button or window whose pick holds the form.
    lib_fn!(lua, t, "set_form", move |_, (side, by, form, turns, alternate): (LuaValue, String, LuaValue, Option<LuaValue>, Option<bool>)| {
        let side = u8_arg(side, "side")? & 1;
        own("custom.set_form")?;
        let form = form_or_nil(&form, "custom.set_form")?;
        let turns = match turns {
            Some(v) if !v.is_nil() => int(&v, "turns")? as u8,
            _ => 0,
        };
        with(|api, _| api.custom_set_form(side, &by, form, turns, alternate.unwrap_or(false)).map_err(api_error))
    });
    lib_fn!(lua, t, "last_pick", |lua, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        let Some(p) = with(|api, _| api.custom_last_pick(side).map_err(api_error))? else { return Ok(LuaValue::Nil) };
        let t = lua.create_table()?;
        t.raw_set("slot", p.slot)?;
        t.raw_set("chip", bound(|b| chip_value(b, Some(p.chip)))?)?;
        t.raw_set("regular", p.regular)?;
        t.raw_set("navi_chip", p.navi_chip)?;
        t.raw_set("attached", p.attached)?;
        Ok(LuaValue::Table(t))
    });
    lib_fn!(lua, t, "attach_to_last_pick", move |_, (side, button, modifiers): (LuaValue, String, LuaValue)| {
        let side = u8_arg(side, "side")? & 1;
        own("custom.attach_to_last_pick")?;
        let modifiers = u8_arg(modifiers, "modifiers")?;
        with(|api, _| api.custom_attach_to_last_pick(side, &button, modifiers).map_err(api_error))
    });
    lib_fn!(lua, t, "hold_last_pick", move |_, (side, button): (LuaValue, String)| {
        let side = u8_arg(side, "side")? & 1;
        own("custom.hold_last_pick")?;
        with(|api, _| api.custom_hold_last_pick(side, &button).map_err(api_error))
    });
    lib_fn!(lua, t, "held_pick", move |_, (side, button): (LuaValue, String)| {
        let side = u8_arg(side, "side")? & 1;
        own("custom.held_pick")?;
        let chip = with(|api, _| api.custom_held_pick(side, &button).map_err(api_error))?;
        bound(|b| chip_value(b, chip))
    });
    lib_fn!(lua, t, "set_held_icon", move |_, (side, button, shown): (LuaValue, String, bool)| {
        let side = u8_arg(side, "side")? & 1;
        own("custom.set_held_icon")?;
        with(|api, _| api.custom_set_held_icon(side, &button, shown).map_err(api_error))
    });
    lib_fn!(lua, t, "trade_last_pick", move |_, (side, button): (LuaValue, String)| {
        let side = u8_arg(side, "side")? & 1;
        own("custom.trade_last_pick")?;
        with(|api, _| api.custom_trade_last_pick(side, &button).map_err(api_error))
    });
    lib_fn!(lua, t, "fading", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| api.custom_fading(side).map_err(api_error))
    });
    lib_fn!(lua, t, "form_taken", move |_, (side, by): (LuaValue, String)| {
        let side = u8_arg(side, "side")? & 1;
        own("custom.form_taken")?;
        with(|api, _| api.custom_form_taken(side, &by).map_err(api_error))
    });
    lib_fn!(lua, t, "full", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| api.custom_full(side).map_err(api_error))
    });
    lib_fn!(lua, t, "button_picked", move |_, (side, button): (LuaValue, String)| {
        let side = u8_arg(side, "side")? & 1;
        own("custom.button_picked")?;
        with(|api, _| api.custom_button_picked(side, &button).map_err(api_error))
    });
    lib_fn!(lua, t, "cursor", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| api.custom_cursor(side).map_err(api_error))
    });
    lib_fn!(lua, t, "set_cursor", |_, (side, slot): (LuaValue, LuaValue)| {
        let side = u8_arg(side, "side")? & 1;
        let slot = u8_arg(slot, "slot")?;
        with(|api, _| api.custom_set_cursor(side, slot).map_err(api_error))
    });
    lib_fn!(lua, t, "pressed", |_, (side, key): (LuaValue, String)| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| api.custom_pressed(side, &key).map_err(api_error))
    });
    lib_fn!(lua, t, "repeated", |_, (side, key): (LuaValue, String)| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| api.custom_repeated(side, &key).map_err(api_error))
    });
    lib_fn!(lua, t, "draw_window", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| api.custom_draw_window(side).map_err(api_error))
    });
    lib_fn!(lua, t, "draw_regular", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| api.custom_draw_regular(side).map_err(api_error))
    });
    lib_fn!(lua, t, "draw_held", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| api.custom_draw_held(side).map_err(api_error))
    });
    lib_fn!(lua, t, "draw_form_list_cursor", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| api.custom_draw_form_list_cursor(side).map_err(api_error))
    });
    lib_fn!(lua, t, "show_chip_window", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| api.custom_show_chip_window(side).map_err(api_error))
    });
    lib_fn!(lua, t, "set_form_list_tab", |_, (side, on): (LuaValue, bool)| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| api.custom_set_form_list_tab(side, on).map_err(api_error))
    });
    lib_fn!(lua, t, "describe", move |_, (side, form): (LuaValue, LuaValue)| {
        let side = u8_arg(side, "side")? & 1;
        let form = form_or_nil(&form, "custom.describe")?;
        with(|api, _| api.custom_describe(side, form).map_err(api_error))
    });
    lib_fn!(lua, t, "refresh_buttons", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| api.custom_refresh_buttons(side).map_err(api_error))
    });
    lib_fn!(lua, t, "player", |lua, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        let p = with(|api, _| api.custom_player(side).map_err(api_error))?;
        let t = lua.create_table()?;
        t.raw_set("emotion", p.emotion.as_str())?;
        t.raw_set("random_battle", p.random_battle)?;
        Ok(t)
    });
    Ok(t)
}

/// `rules`: what the game's rules' own calls reach (docs/design/
/// rules-in-luau.md §4.5, §5.3): their state and the player's setup, of
/// the side they were called for, and that side.
fn rules_lib(lua: &Lua) -> mlua::Result<mlua::Table> {
    let t = lua.create_table()?;
    lib_fn!(lua, t, "state", |_, ()| Ok(State(StateOf::Rules(rules_ctx("rules.state")?))));
    lib_fn!(lua, t, "setup", |_, ()| Ok(State(StateOf::Setup(rules_ctx("rules.setup")?))));
    lib_fn!(lua, t, "side", |_, ()| Ok(rules_ctx("rules.side")?.side));
    // The rules' state of side `side`, for a game's rules alone (its API
    // module, docs/design/rules-in-luau.md, As built S8): the game's API
    // reaches what its rules keep (EXE6's bug frags) for its chips, which
    // never call it themselves.
    lib_fn!(lua, t, "state_of", |lua, side: LuaValue| {
        rules_only(lua, "rules.state_of", "state")?;
        Ok(State(StateOf::Rules(RulesCtx { side: u8_arg(side, "side")? & 1 })))
    });
    // Side `side`'s player's setup of the rules, read-only, likewise a
    // game's rules' alone (its API module: EXE6's and EXE5's navi level,
    // the rules' `level`).
    lib_fn!(lua, t, "setup_of", |lua, side: LuaValue| {
        rules_only(lua, "rules.setup_of", "setup")?;
        Ok(State(StateOf::Setup(RulesCtx { side: u8_arg(side, "side")? & 1 })))
    });
    Ok(t)
}

/// A call only a game's rules make (a module under rules/: its API module).
fn rules_only(lua: &Lua, what: &str, reaches: &str) -> mlua::Result<()> {
    let caller = crate::define::caller(lua);
    if nettai_content_api::keys::local(&caller).starts_with("rules/") {
        return Ok(());
    }
    Err(mlua::Error::runtime(format!(
        "{caller}: {what} is a game's rules' (a module under rules/, its API module): content reaches the rules' {reaches} through the game's API"
    )))
}

fn battle_lib(lua: &Lua) -> mlua::Result<mlua::Table> {
    let t = lua.create_table()?;
    lib_fn!(lua, t, "dimmed", |_, ()| with(|api, _| Ok(api.is_dimmed())));
    lib_fn!(lua, t, "battle_time", |_, ()| with(|api, _| Ok(api.battle_time())));
    lib_fn!(lua, t, "set_dimmed", |_, on: bool| with(|api, _| Ok(api.set_dimmed(on))));
    lib_fn!(
        lua,
        t,
        "spawn_navi",
        |lua, (identity, x, y, side, summoner): (LuaValue, LuaValue, LuaValue, LuaValue, LuaValue)| {
            let p = panel(x, y)?;
            let side = u8_arg(side, "side")?;
            let summoner = object_arg(&summoner, "summoner")?;
            let o = with(|api, b| {
                let identity = nettai_content_api::IdentityHandle(def_arg(b, &identity, Registry::Identity, "battle.spawn_navi")?);
                api.spawn_navi(identity, p, side, summoner).map_err(api_error)
            })?;
            object_value(lua, o)
        }
    );
    lib_fn!(lua, t, "paused", |_, ()| with(|api, _| Ok(api.is_paused())));
    lib_fn!(lua, t, "over", |_, ()| with(|api, _| Ok(api.is_battle_over())));
    lib_fn!(lua, t, "time_up", |_, ()| with(|api, _| Ok(api.is_time_up())));
    lib_fn!(lua, t, "next_chip_damages", |_, user: mlua::UserDataRef<Object>| {
        with(|api, _| Ok(api.next_chip_damages(user.0)))
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
    lib_fn!(lua, t, "warn", |_, (id, at, side): (LuaValue, Option<mlua::UserDataRef<LVec3>>, Option<LuaValue>)| {
        let id = sound_arg(id)?;
        let side = side.map(|s| u8_arg(s, "side")).transpose()?.map(|s| s & 1);
        with(|api, _| Ok(api.warn(id, at.map(|p| p.0), side)))
    });
    lib_fn!(lua, t, "show_hp", |_, (o, spec): (mlua::UserDataRef<Object>, mlua::Table)| {
        let offset = |key: &str| -> mlua::Result<i8> {
            match spec.get::<LuaValue>(key)? {
                LuaValue::Nil => Ok(0),
                v => {
                    let n = int(&v, key)?;
                    i8::try_from(n).map_err(|_| mlua::Error::runtime(format!("battle.show_hp: {key} {n} is past a signed byte")))
                }
            }
        };
        let (dx, dy) = (offset("dx")?, offset("dy")?);
        let damage = spec.get::<Option<bool>>("damage")?.unwrap_or(false);
        with(|api, _| Ok(api.show_hp(o.0, dx, dy, damage)))
    });
    lib_fn!(lua, t, "hide_hp", |_, o: mlua::UserDataRef<Object>| with(|api, _| Ok(api.hide_hp(o.0))));
    lib_fn!(lua, t, "shake_camera", |_, (magnitude, ticks): (LuaValue, LuaValue)| {
        let (magnitude, ticks) = (u16_arg(magnitude, "magnitude")?, u16_arg(ticks, "ticks")?);
        if magnitude > 3 {
            return Err(mlua::Error::runtime(format!("camera shake magnitude {magnitude} reads past byte_8030284")));
        }
        with(|api, _| Ok(api.shake_camera(magnitude, ticks)))
    });
    lib_fn!(lua, t, "shake_camera_secondary", |_, (magnitude, ticks): (LuaValue, LuaValue)| {
        let (magnitude, ticks) = (u16_arg(magnitude, "magnitude")?, u16_arg(ticks, "ticks")?);
        if magnitude > 3 {
            return Err(mlua::Error::runtime(format!("camera shake magnitude {magnitude} reads past byte_8030284")));
        }
        with(|api, _| Ok(api.shake_camera_secondary(magnitude, ticks)))
    });
    lib_fn!(lua, t, "set_shake_through_pause", |_, on: bool| with(|api, _| Ok(api.set_shake_through_pause(on))));
    lib_fn!(lua, t, "burst", |_, navi: mlua::UserDataRef<Object>| {
        with(|api, _| Ok(api.spawn_burst(navi.0).map(Object)))
    });
    lib_fn!(lua, t, "screen_fade", |_, (fade, speed): (mlua::LuaString, LuaValue)| {
        let fade = named(&fade, "screen fade", ScreenFade::from_name)?;
        let speed = u8_arg(speed, "fade speed")?;
        with(|api, _| Ok(api.screen_fade(fade, speed)))
    });
    lib_fn!(lua, t, "show_hud", |_, (parts, shown): (mlua::Table, bool)| {
        let parts = parts
            .sequence_values::<mlua::LuaString>()
            .map(|p| named(&p?, "HUD part", HudPart::from_name))
            .collect::<mlua::Result<Vec<_>>>()?;
        with(|api, _| {
            for p in parts {
                api.show_hud(p, shown);
            }
            Ok(())
        })
    });
    lib_fn!(lua, t, "navi", |_, side: LuaValue| Ok(Navi(u8_arg(side, "side")? & 1)));
    lib_fn!(lua, t, "emotion", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| Ok(api.emotion(side)))
    });
    lib_fn!(lua, t, "set_mood", |_, (side, mood): (LuaValue, LuaValue)| {
        let (side, mood) = (u8_arg(side, "side")? & 1, u8_arg(mood, "mood")?);
        with(|api, _| Ok(api.set_mood(side, mood)))
    });
    lib_fn!(lua, t, "gain_mood", |_, (side, n): (LuaValue, LuaValue)| {
        let (side, n) = (u8_arg(side, "side")? & 1, int(&n, "amount")? as u16);
        with(|api, _| Ok(api.gain_mood(side, n)))
    });
    lib_fn!(lua, t, "lose_mood", |_, (side, n): (LuaValue, LuaValue)| {
        let (side, n) = (u8_arg(side, "side")? & 1, int(&n, "amount")? as u16);
        with(|api, _| Ok(api.lose_mood(side, n)))
    });
    lib_fn!(lua, t, "set_emotion_window_glitch", |_, (side, on): (LuaValue, bool)| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| Ok(api.set_emotion_window_glitch(side, on)))
    });
    // The folder a tool asks the rules to check (`folder_check`): { side,
    // chips = { { chip = <the definition>, code = "A" } }, regular = n?,
    // tags = { a, b }?, complete } (entries counting from 1), or nil.
    lib_fn!(lua, t, "checked_folder", |lua, ()| {
        let Some(f) = with(|api, _| Ok(api.checked_folder()))? else { return Ok(LuaValue::Nil) };
        let out = lua.create_table()?;
        out.raw_set("side", f.side)?;
        let chips = lua.create_table()?;
        for (i, (chip, code)) in f.chips.iter().enumerate() {
            let entry = lua.create_table()?;
            entry.raw_set("chip", bound(|b| b.def_value(Registry::Chip, *chip))?)?;
            let letter = if *code == 26 { "*".to_string() } else { ((b'A' + code) as char).to_string() };
            entry.raw_set("code", letter)?;
            chips.raw_set(i + 1, entry)?;
        }
        out.raw_set("chips", chips)?;
        if let Some(r) = f.regular {
            out.raw_set("regular", r as i64 + 1)?;
        }
        if let Some((a, b)) = f.tags {
            out.raw_set("tags", lua.create_sequence_from([a as i64 + 1, b as i64 + 1])?)?;
        }
        out.raw_set("complete", f.complete)?;
        Ok(LuaValue::Table(out))
    });
    lib_fn!(lua, t, "folder_problem", |_, (rule, text): (String, String)| {
        with(|api, _| Ok(api.folder_problem(&rule, &text)))
    });
    // A definition's name in the content's own strings, else its key: for
    // what the rules say to a tool (`validate`).
    lib_fn!(lua, t, "name", |_, def: LuaValue| {
        let (registry, h) = bound(|b| Ok(b.def(&def)))?.ok_or_else(|| mlua::Error::runtime("battle.name takes a definition"))?;
        with(|api, _| Ok(api.def_name(registry, h)))
    });
    lib_fn!(lua, t, "side_special", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| Ok(api.side_special(side).name()))
    });
    lib_fn!(lua, t, "take_over", |_, (side, ticks): (LuaValue, LuaValue)| {
        let side = u8_arg(side, "side")? & 1;
        let ticks = int(&ticks, "ticks")? as u16;
        with(|api, _| {
            api.take_over(side, ticks);
            Ok(())
        })
    });
    lib_fn!(lua, t, "end_takeover", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| {
            api.end_takeover(side);
            Ok(())
        })
    });
    lib_fn!(lua, t, "takeover_ticks", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| Ok(api.takeover_ticks(side)))
    });
    lib_fn!(lua, t, "player", |lua, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        let p = with(|api, _| Ok(api.player(side)))?;
        object_value(lua, p)
    });
    lib_fn!(lua, t, "alive_actor_slot", |lua, (side, i): (LuaValue, LuaValue)| {
        let side = u8_arg(side, "side")? & 1;
        let i = int(&i, "slot")?;
        if !(1..=4).contains(&i) {
            return Err(mlua::Error::runtime("battle.alive_actor_slot: a side has slots 1 to 4"));
        }
        let o = with(|api, _| Ok(api.alive_actor_slot(side, (i - 1) as u8)))?;
        object_value(lua, o)
    });
    lib_fn!(lua, t, "alive_actors", |lua, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        let list = with(|api, _| Ok(api.alive_actors(side)))?;
        lua.create_sequence_from(list.into_iter().map(Object))
    });
    lib_fn!(lua, t, "tracked", |lua, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        let o = with(|api, _| Ok(api.tracked(side)))?;
        object_value(lua, o)
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
    lib_fn!(lua, t, "console_rng_positive", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| Ok(api.console_rng_positive(side)))
    });
    lib_fn!(lua, t, "jitter", |_, (mask, pos): (LuaValue, mlua::UserDataRef<LVec3>)| {
        let mask = int(&mask, "mask")? as u32;
        with(|api, _| Ok(LVec3(api.jitter(mask, pos.0))))
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
        t.raw_set("chip", bound(|b| chip_value(b, r.chip))?)?;
        t.raw_set("bonus", r.bonus)?;
        t.raw_set("damage", r.damage)?;
        t.raw_set("owner", object_value(lua, r.owner)?)?;
        t.raw_set("object", object_value(lua, r.object)?)?;
        Ok(t)
    });
    lib_fn!(lua, t, "set_linked", |_, (side, r): (LuaValue, mlua::Table)| {
        let side = u8_arg(side, "side")? & 1;
        let rec = LinkedChip {
            chip: bound(|b| chip_arg(b, &r.raw_get("chip")?, "a linked record's chip"))?,
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
    lib_fn!(lua, t, "clear_bugs", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| Ok(api.clear_bugs(side)))
    });
    lib_fn!(lua, t, "clear_emotion_window_glitch", |_, ()| with(|api, _| Ok(api.clear_emotion_window_glitch())));
    lib_fn!(lua, t, "fill_custom_gauge", |_, ()| with(|api, _| Ok(api.fill_custom_gauge())));
    lib_fn!(lua, t, "drain_custom_gauge", |_, n: LuaValue| {
        let n = u16_arg(n, "gauge")?;
        with(|api, _| Ok(api.drain_custom_gauge(n)))
    });
    lib_fn!(lua, t, "add_side_gauge", |_, (side, n): (LuaValue, LuaValue)| {
        let (side, n) = (u8_arg(side, "side")? & 1, u16_arg(n, "gauge")?);
        with(|api, _| Ok(api.add_side_gauge(side, n)))
    });
    lib_fn!(lua, t, "drain_side_gauge", |_, (side, n): (LuaValue, LuaValue)| {
        let (side, n) = (u8_arg(side, "side")? & 1, u16_arg(n, "gauge")?);
        with(|api, _| Ok(api.drain_side_gauge(side, n)))
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
    lib_fn!(lua, t, "gauge_damage", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| Ok(api.gauge_damage(side)))
    });
    lib_fn!(lua, t, "sword_pick", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| Ok(api.sword_pick(side)))
    });
    lib_fn!(lua, t, "set_sword_pick", |_, (side, pick): (LuaValue, LuaValue)| {
        let (side, pick) = (u8_arg(side, "side")? & 1, u8_arg(pick, "sword pick")?);
        with(|api, _| Ok(api.set_sword_pick(side, pick)))
    });
    lib_fn!(lua, t, "set_face_variant", |_, (side, variant, mode): (LuaValue, bool, Option<mlua::LuaString>)| {
        let side = u8_arg(side, "side")? & 1;
        let charged = match mode.as_ref().map(|m| m.to_str().map(|s| s.to_string())).transpose()?.as_deref() {
            None | Some("base") => false,
            Some("charged") => true,
            Some(other) => return Err(mlua::Error::runtime(format!("set_face_variant: a mode is \"base\" or \"charged\", not {other:?}"))),
        };
        with(|api, _| Ok(api.set_face_variant(side, variant, charged)))
    });
    lib_fn!(lua, t, "set_name_variant", |_, (side, variant): (LuaValue, bool)| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| Ok(api.set_name_variant(side, variant)))
    });
    lib_fn!(lua, t, "set_window_count", |_, (side, shown): (LuaValue, bool)| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| Ok(api.set_window_count(side, shown)))
    });
    lib_fn!(lua, t, "bump_side_stat", |_, (side, i, n): (LuaValue, LuaValue, LuaValue)| {
        let (side, i, n) = (u8_arg(side, "side")? & 1, u8_arg(i, "stat")?, u8_arg(n, "count")?);
        with(|api, _| Ok(api.bump_side_stat(side, i, n)))
    });
    lib_fn!(lua, t, "set_side_stat", |_, (side, i, n): (LuaValue, LuaValue, LuaValue)| {
        let (side, i, n) = (u8_arg(side, "side")? & 1, u8_arg(i, "stat")?, u8_arg(n, "count")?);
        with(|api, _| Ok(api.set_side_stat(side, i, n)))
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
        let rec = nettai_content_api::api::DamageCarryInfo {
            this_tick: table_int(&r, "this_tick")? as u16,
            previous: table_int(&r, "previous")? as u16,
            source: object_arg(&r.raw_get("source")?, "source")?,
            target: object_arg(&r.raw_get("target")?, "target")?,
        };
        with(|api, _| Ok(api.set_damage_carry(side, rec)))
    });
    lib_fn!(lua, t, "loop_register", |_, ()| with(|api, _| Ok(api.loop_register())));
    lib_fn!(lua, t, "spawn", |lua, (kind, pos): (LuaValue, Option<mlua::UserDataRef<LVec3>>)| {
        spawn_kind_def(lua, kind, vec3_arg(pos), SpawnAt::AfterCurrent)
    });
    lib_fn!(lua, t, "spawn_first", |lua, (kind, pos): (LuaValue, Option<mlua::UserDataRef<LVec3>>)| {
        spawn_kind_def(lua, kind, vec3_arg(pos), SpawnAt::First)
    });
    lib_fn!(lua, t, "spawn_at_end", |lua, (kind, pos): (LuaValue, Option<mlua::UserDataRef<LVec3>>)| {
        spawn_kind_def(lua, kind, vec3_arg(pos), SpawnAt::End)
    });
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
        "effect_after_spawn",
        |lua, (z, id, flip, palette_add, priority): (
            i32,
            LuaValue,
            Option<LuaValue>,
            Option<LuaValue>,
            Option<LuaValue>
        )| {
            let opt = |v: Option<LuaValue>, what| v.map_or(Ok(0), |v| u8_arg(v, what));
            let (flip, add, prio) = (opt(flip, "flip")?, opt(palette_add, "palette")?, opt(priority, "priority")?);
            let o = with(|api, b| {
                let look = EffectHandle(def_arg(b, &id, Registry::Effect, "battle.effect_after_spawn")?);
                Ok(api.spawn_effect_after_spawn(z, look, flip, add, prio))
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
            h.map(CollisionHandle).ok_or_else(|| mlua::Error::runtime(format!("battle.hitbox: `{key}` is a collision type (new.collision)")))
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
                    (_, Some((Registry::Status, h))) => Ok(Some(nettai_content_api::StatusHandle(h))),
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
        let sprite = sprite_id(spec.raw_get("sprite")?)?;
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
    lib_fn!(lua, t, "set_hand_attack_bonus", |_, (side, i, n): (LuaValue, LuaValue, LuaValue)| {
        let (side, i, n) = (u8_arg(side, "side")? & 1, u8_arg(i, "hand index")?, u16_arg(n, "bonus")?);
        with(|api, _| Ok(api.set_hand_attack_bonus(side, i, n)))
    });
    lib_fn!(lua, t, "hand_chip_damages", |_, (side, i): (LuaValue, LuaValue)| {
        let (side, i) = (u8_arg(side, "side")? & 1, u8_arg(i, "hand index")?);
        with(|api, _| Ok(api.hand_chip_damages(side, i)))
    });
    lib_fn!(lua, t, "hand_left", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| Ok(api.hand_left(side)))
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
                Some(s) => named(&s, "shadow", |n| nettai_content_api::Shadow::NAMES.iter().position(|&m| m == n).map(|i| nettai_content_api::Shadow::ALL[i]))?,
                None => nettai_content_api::Shadow::WithSprite,
            };
            let tether: Option<mlua::LuaString> = spec.raw_get("tether")?;
            let tether = match tether {
                Some(s) => named(&s, "afterimage tether", |n| match n {
                    "form" => Some(1),
                    "attack" => Some(2),
                    _ => None,
                })?,
                None => 0,
            };
            let s = nettai_content_api::api::AfterimageSpec {
                sprite: if sprite.is_nil() { None } else { Some(sprite_id(sprite)?) },
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
    lib_fn!(lua, t, "object_slot", |lua, (side, i): (LuaValue, LuaValue)| {
        let side = u8_arg(side, "side")? & 1;
        let i = int(&i, "slot")?;
        if !(1..=3).contains(&i) {
            return Err(mlua::Error::runtime("field.object_slot: a side has slots 1 to 3"));
        }
        let o = with(|api, _| Ok(api.field_object_slot(side, (i - 1) as u8)))?;
        object_value(lua, o)
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
    lib_fn!(lua, t, "break_panel", |_, (x, y, sound): (LuaValue, LuaValue, LuaValue)| {
        let p = panel(x, y)?;
        let sound = match sound {
            LuaValue::Nil => None,
            s => Some(sound_arg(s)?),
        };
        with(|api, _| Ok(api.break_panel(p, sound)))
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
            lua.create_function(move |_, (me, chip): (mlua::UserDataRef<Object>, mlua::Variadic<LuaValue>)| {
                // The chip the controller shows (nil: the zeroed chip
                // field's), for the steps that read it.
                let chip = match chip.first() {
                    Some(c) => bound(|b| chip_arg(b, c, "the dimming's chip"))?,
                    None if needs_chip => {
                        return Err(mlua::Error::runtime(format!("dimming.{} needs the chip", step.name())));
                    }
                    None => None,
                };
                with(|api, _| Ok(api.dimming(me.0, step, chip)))
            })?,
        )?;
    }
    lib_fn!(
        lua,
        t,
        "start",
        |_, (side, no_cut_in, controller, user, telop): (LuaValue, bool, LuaValue, mlua::UserDataRef<Object>, Option<mlua::Table>)| {
            let side = u8_arg(side, "side")? & 1;
            let controller = object_arg(&controller, "controller")?;
            // What the telop names: `{ chip = <chip>?, bonus = n? }`.
            let telop = match telop {
                None => None,
                Some(t) => {
                    let chip = match t.get::<LuaValue>("chip")? {
                        LuaValue::Nil => None,
                        c => Some(ChipHandle(bound(|b| def_arg(b, &c, Registry::Chip, "dimming.start's telop chip"))?)),
                    };
                    let bonus = match t.get::<LuaValue>("bonus")? {
                        LuaValue::Nil => 0,
                        v => u16_arg(v, "dimming.start's telop bonus")?,
                    };
                    Some((chip, bonus))
                }
            };
            with(|api, _| Ok(api.start_dimming(side, no_cut_in, controller, user.0, telop)))
        }
    );
    lib_fn!(lua, t, "hide_user", |_, user: mlua::UserDataRef<Object>| with(|api, _| Ok(api.hide_user(user.0))));
    lib_fn!(lua, t, "show_user", |_, user: mlua::UserDataRef<Object>| with(|api, _| Ok(api.show_user(user.0))));
    lib_fn!(lua, t, "hide_user_sparing", |_, user: mlua::UserDataRef<Object>| {
        with(|api, _| Ok(api.hide_user_sparing(user.0)))
    });
    lib_fn!(lua, t, "hide_actor", |_, o: mlua::UserDataRef<Object>| with(|api, _| Ok(api.hide_actor(o.0))));
    lib_fn!(lua, t, "show_actor", |_, o: mlua::UserDataRef<Object>| with(|api, _| Ok(api.show_actor(o.0))));
    Ok(t)
}

/// A Lua integer as the content's: mlua's is 32 bits on wasm32.
#[allow(clippy::useless_conversion)]
fn lua_int(i: mlua::Integer) -> i64 {
    i.into()
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
    lib_fn!(lua, t, "action_byte", |_, (me, a): (Me, LuaValue)| {
        let a = u8_arg(a, "action")?;
        with(|api, _| api.obstacle_action_byte(me.0, a).map_err(api_error))
    });
    lib_fn!(lua, t, "current", |_, me: Me| with(|api, _| Ok(api.obstacle_current_action(me.0))));
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
    lib_fn!(lua, t, "absorbed_look", |_, o: Me| {
        match with(|api, _| Ok(api.absorbed_look(o.0)))? {
            Some(h) => Ok(LuaValue::Table(bound(|b| b.def_value(Registry::Identity, h.0))?)),
            None => Ok(LuaValue::Nil),
        }
    });
    lib_fn!(lua, t, "swallowable", |_, o: Me| with(|api, _| Ok(api.obstacle_swallowable(o.0))));
    lib_fn!(lua, t, "throwable", |_, o: Me| with(|api, _| Ok(api.obstacle_throwable(o.0))));
    lib_fn!(lua, t, "throw", |_, (o, side, x, y, shake, damage): (Me, u8, u8, u8, u8, u32)| {
        with(|api, _| {
            api.obstacle_throw(o.0, side, x, y, shake, damage);
            Ok(())
        })
    });
    lib_fn!(lua, t, "arm_conversion", |_, (side, melee, ranged): (LuaValue, LuaValue, LuaValue)| {
        let side = u8_arg(side, "side")? & 1;
        let (melee, ranged) = (int(&melee, "melee word")? as u32, int(&ranged, "ranged word")? as u32);
        with(|api, _| Ok(api.obstacle_arm_conversion(side, melee, ranged)))
    });
    lib_fn!(lua, t, "disarm_conversion", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| Ok(api.obstacle_disarm_conversion(side)))
    });
    lib_fn!(lua, t, "conversion", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| Ok(api.obstacle_conversion(side)))
    });
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
    lua.create_userdata(State(StateOf::ActionAs(o, state)))
}

/// A hook call's arguments.
pub fn hook_args(lua: &Lua, call: HookCall, bound: &Bound) -> mlua::Result<mlua::MultiValue> {
    let obj = |o: ObjectRef| -> mlua::Result<LuaValue> { Ok(LuaValue::UserData(lua.create_userdata(Object(o))?)) };
    let values = match call {
        HookCall::Weapon { navi } => vec![obj(navi)?],
        HookCall::DimmingChip { user, spec } => {
            let t = lua.create_table()?;
            t.raw_set("element", spec.element)?;
            t.raw_set("damage", spec.damage)?;
            t.raw_set("chip", chip_value(bound, spec.chip)?)?;
            t.raw_set("bonus", spec.bonus)?;
            vec![obj(user)?, LuaValue::Table(t)]
        }
        HookCall::NaviChip { user, controller, spec } => {
            let t = lua.create_table()?;
            t.raw_set("panel_x", spec.panel.x)?;
            t.raw_set("panel_y", spec.panel.y)?;
            t.raw_set("element", spec.element)?;
            t.raw_set("damage", spec.damage)?;
            vec![obj(user)?, obj(controller)?, LuaValue::Table(t)]
        }
        HookCall::InstantChip { user, spec } => {
            let t = lua.create_table()?;
            t.raw_set("panel_x", spec.panel.x)?;
            t.raw_set("panel_y", spec.panel.y)?;
            t.raw_set("element", spec.element)?;
            t.raw_set("z", spec.z)?;
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
        HookCall::RoleNavi { navi } | HookCall::FormNavi { navi } => vec![obj(navi)?],
        HookCall::Given { side, chip } => {
            let chip = match chip {
                Some(c) => LuaValue::Table(bound.def_value(Registry::Chip, c.0)?),
                None => LuaValue::Nil,
            };
            vec![LuaValue::Integer(mlua::Integer::from(side)), chip]
        }
        HookCall::NaviLeft { controller } => vec![obj(controller)?],
        HookCall::RoleEncased { obstacle, ice, class } => {
            let class = class.map_or(LuaValue::Nil, |c| LuaValue::Integer(mlua::Integer::from(c)));
            vec![obj(obstacle)?, LuaValue::Boolean(ice), class]
        }
        // The side, then the navi, the chip and the weapon, where the hook
        // has them (nil in between).
        // A custom screen's chip hooks: the side, then the chip.
        HookCall::Rules { side, hook: RulesHook::CustomChipPicked | RulesHook::CustomChipTakenBack, chip, .. } => vec![
            LuaValue::Integer(mlua::Integer::from(side)),
            chip.map_or(Ok(LuaValue::Nil), |c| bound.def_value(Registry::Chip, c.0).map(LuaValue::Table))?,
        ],
        HookCall::Rules { side, navi, chip, weapon, .. } => {
            let mut v = vec![
                LuaValue::Integer(mlua::Integer::from(side)),
                navi.map_or(Ok(LuaValue::Nil), obj)?,
                chip.map_or(Ok(LuaValue::Nil), |c| bound.def_value(Registry::Chip, c.0).map(LuaValue::Table))?,
                weapon.map_or(Ok(LuaValue::Nil), |w| bound.def_value(Registry::Weapon, w.0).map(LuaValue::Table))?,
            ];
            while v.len() > 1 && v.last().is_some_and(LuaValue::is_nil) {
                v.pop();
            }
            v
        }
    };
    Ok(mlua::MultiValue::from_iter(values))
}

/// A hook's result as the engine takes it.
pub fn hook_result(v: LuaValue, call: HookCall, bound: &Bound) -> mlua::Result<Value> {
    match call {
        // An action definition; nothing for a weapon whose own instant
        // effect the engine runs.
        HookCall::Weapon { .. } if v.is_nil() => Ok(Value::Nil),
        // The routine ended the attack itself: the navi stays idle.
        HookCall::Weapon { .. } if v == LuaValue::Boolean(false) => Ok(Value::Bool(false)),
        HookCall::Weapon { .. } => match bound.def(&v) {
            Some((Registry::Action, h)) => Ok(Value::Def(Registry::Action, h)),
            // A chip: what its use starts (the weapon fires the chip).
            Some((Registry::Chip, h)) => Ok(Value::Def(Registry::Chip, h)),
            Some((r, _)) => Err(mlua::Error::runtime(format!("a weapon routine returns an action, not a {r}"))),
            None => Err(mlua::Error::runtime(format!(
                "a weapon routine returns an action definition, not a {}",
                v.type_name()
            ))),
        },
        HookCall::DimmingChip { .. } | HookCall::NaviChip { .. } | HookCall::Place { .. } => {
            Ok(object_arg(&v, "the object a spawner returns")?.map_or(Value::Nil, Value::Object))
        }
        // A navi's role hook may hand back an object (`navi_deleted`'s).
        HookCall::RoleNavi { .. } => Ok(object_arg(&v, "the object a role hook returns")?.map_or(Value::Nil, Value::Object)),
        HookCall::InstantChip { .. } | HookCall::RoleEncased { .. } | HookCall::NaviLeft { .. } | HookCall::FormNavi { .. } => {
            Ok(Value::Nil)
        }
        // What a side is given: a whole number, a flag or nil.
        HookCall::Given { .. } => match &v {
            LuaValue::Nil => Ok(Value::Nil),
            LuaValue::Boolean(b) => Ok(Value::Bool(*b)),
            LuaValue::Integer(n) => Ok(Value::Int(i64::from(*n))),
            LuaValue::Number(n) if n.fract() == 0.0 && n.abs() < 1e15 => Ok(Value::Int(*n as i64)),
            _ => Err(mlua::Error::runtime(format!("a function of the side returns a whole number, a flag or nil, not {v:?}"))),
        },
        // A chip check's, cost's or substitute's chip; no other rules hook
        // returns anything.
        HookCall::Rules {
            hook: hook @ (RulesHook::ChipCheck | RulesHook::ChipCost | RulesHook::ChipSubstitute), ..
        } if !v.is_nil() => match bound.def(&v) {
            Some((Registry::Chip, h)) => Ok(Value::Def(Registry::Chip, h)),
            _ => Err(mlua::Error::runtime(format!(
                "the rules' {} returns nil or a chip definition, not a {}",
                hook.name(),
                v.type_name()
            ))),
        },
        // A controller's or a takeover's outcome, by the original's number
        // (4: an attack of the takeover's own).
        HookCall::Rules { hook: RulesHook::Controller | RulesHook::Takeover, .. } => match &v {
            LuaValue::Nil => Ok(Value::Nil),
            LuaValue::String(s) => match &*s.to_str()? {
                "nothing" => Ok(Value::Int(0)),
                "chip" => Ok(Value::Int(1)),
                "buster" => Ok(Value::Int(2)),
                "moved" => Ok(Value::Int(3)),
                "own_chip" => Ok(Value::Int(4)),
                other => Err(mlua::Error::runtime(format!(
                    "a controller returns \"nothing\", \"chip\", \"buster\", \"moved\" or \"own_chip\", not {other:?}"
                ))),
            },
            _ => Err(mlua::Error::runtime(format!("a controller returns its outcome's name, not a {}", v.type_name()))),
        },
        // A button's `shown` and `state`, a window's `update`, and whether
        // the rules took the keys or took something back.
        HookCall::Rules {
            hook:
                RulesHook::ButtonShown
                | RulesHook::WindowUpdate
                | RulesHook::CustomKeys
                | RulesHook::CustomTakeBack
                | RulesHook::BugMark
                | RulesHook::HpEmptied,
            ..
        } => Ok(Value::Bool(v == LuaValue::Boolean(true))),
        // What the rules did with a hit's NaviCust bug: nothing of the
        // stats, edited them, or spared the navi the bug and the reload.
        HookCall::Rules { hook: RulesHook::NaviBug, .. } => match &v {
            LuaValue::Nil => Ok(Value::Nil),
            LuaValue::String(s) => match &*s.to_str()? {
                "edited" => Ok(Value::Int(1)),
                "spared" => Ok(Value::Int(2)),
                other => Err(mlua::Error::runtime(format!("navi_bug returns nil, \"edited\" or \"spared\", not {other:?}"))),
            },
            _ => Err(mlua::Error::runtime(format!("navi_bug returns nil, \"edited\" or \"spared\", not a {}", v.type_name()))),
        },
        // A button's chip.
        HookCall::Rules { hook: RulesHook::ButtonChip, .. } => match bound.def(&v) {
            _ if v.is_nil() => Ok(Value::Nil),
            Some((Registry::Chip, h)) => Ok(Value::Def(Registry::Chip, h)),
            _ => Err(mlua::Error::runtime(format!("a button's chip is nil or a chip definition, not a {}", v.type_name()))),
        },
        HookCall::Rules { hook: RulesHook::ButtonState, .. } => match &v {
            LuaValue::Nil => Ok(Value::Nil),
            LuaValue::String(s) => match &*s.to_str()? {
                "selectable" => Ok(Value::Int(0)),
                "unavailable" => Ok(Value::Int(1)),
                other => Err(mlua::Error::runtime(format!(
                    "a button's state is \"selectable\" or \"unavailable\", not {other:?}"
                ))),
            },
            _ => Err(mlua::Error::runtime(format!("a button's state is its name, not a {}", v.type_name()))),
        },
        // A mood or a palette.
        HookCall::Rules { hook: hook @ (RulesHook::StartingMood | RulesHook::NaviPalette), .. } => match &v {
            LuaValue::Nil => Ok(Value::Nil),
            LuaValue::Integer(n) if (0..=255).contains(n) => Ok(Value::Int(i64::from(*n))),
            LuaValue::Number(n) if n.fract() == 0.0 && (0.0..=255.0).contains(n) => Ok(Value::Int(*n as i64)),
            _ => Err(mlua::Error::runtime(format!("{} returns a byte or nil, not {v:?}", hook.name()))),
        },
        // A hand size.
        HookCall::Rules { hook: RulesHook::CustomHandSize, .. } => match &v {
            LuaValue::Nil => Ok(Value::Nil),
            LuaValue::Integer(n) if (0..=255).contains(n) => Ok(Value::Int(i64::from(*n))),
            LuaValue::Number(n) if n.fract() == 0.0 && (0.0..=255.0).contains(n) => Ok(Value::Int(*n as i64)),
            _ => Err(mlua::Error::runtime(format!("custom.hand_size returns a count of chips, not {v:?}"))),
        },
        HookCall::Rules { .. } => Ok(Value::Nil),
    }
}

/// A module's value as plain data (`ContentHost::module_data`): a flag, a
/// whole number, a string, a definition (by its key), a list, a table by
/// keys (in key order); a function left out.
pub fn plain_data(v: &LuaValue, bound: &Bound, depth: u32) -> mlua::Result<nettai_content_api::Data> {
    use nettai_content_api::{Data, DataKey};
    if depth > 16 {
        return Err(mlua::Error::runtime("data nested past 16 tables"));
    }
    Ok(match v {
        LuaValue::Nil | LuaValue::Function(_) => Data::Nil,
        LuaValue::Boolean(b) => Data::Bool(*b),
        LuaValue::Integer(i) => Data::Int(lua_int(*i)),
        LuaValue::Number(n) if n.fract() == 0.0 && n.abs() < 9.0e15 => Data::Int(*n as i64),
        LuaValue::String(s) => Data::Str(s.to_str()?.to_string()),
        LuaValue::Table(t) => {
            if let Some((r, h)) = bound.def(v) {
                let key = bound.key(r, h).ok_or_else(|| mlua::Error::runtime(format!("{r} {h} has no key")))?;
                return Ok(Data::Ref(r, key.to_string()));
            }
            let n = t.raw_len();
            let mut entries = Vec::new();
            for pair in t.clone().pairs::<LuaValue, LuaValue>() {
                let (k, x) = pair?;
                if matches!(x, LuaValue::Function(_)) {
                    continue;
                }
                let key = match &k {
                    LuaValue::String(s) => DataKey::Str(s.to_str()?.to_string()),
                    LuaValue::Integer(i) => DataKey::Int(lua_int(*i)),
                    LuaValue::Number(f) if f.fract() == 0.0 => DataKey::Int(*f as i64),
                    other => return Err(mlua::Error::runtime(format!("a table keyed by names or numbers, not {}", other.type_name()))),
                };
                entries.push((key, plain_data(&x, bound, depth + 1)?));
            }
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            // (A sequence, 1 to n, is a list.)
            if n > 0 && entries.len() == n && entries.iter().all(|(k, _)| matches!(k, DataKey::Int(i) if (1..=n as i64).contains(i))) {
                Data::List(entries.into_iter().map(|(_, x)| x).collect())
            } else {
                Data::Map(entries)
            }
        }
        other => return Err(mlua::Error::runtime(format!("no data is a {}", other.type_name()))),
    })
}

/// What the rules' `validate` answered: each problem a sentence or
/// `{ text, field?, entry? }` (an entry from 1, a list's place in Luau),
/// said to the battle as a setup problem.
pub fn setup_problems(v: LuaValue, api: &mut dyn CoreApi) -> mlua::Result<()> {
    let list = match v {
        LuaValue::Nil => return Ok(()),
        LuaValue::Table(t) => t,
        other => return Err(mlua::Error::runtime(format!("validate returns a list of problems, not {}", other.type_name()))),
    };
    for item in list.sequence_values::<LuaValue>() {
        match item? {
            LuaValue::String(text) => api.setup_problem(&text.to_str()?, None, None),
            LuaValue::Table(p) => {
                let text: String = p.get("text").map_err(|_| mlua::Error::runtime("a problem's `text` is its sentence"))?;
                let field: Option<String> = p.get("field")?;
                let entry: Option<i64> = p.get("entry")?;
                let entry = match entry {
                    Some(e) if e >= 1 => Some((e - 1) as u32),
                    Some(e) => return Err(mlua::Error::runtime(format!("a problem's entry is a list's place, from 1, not {e}"))),
                    None => None,
                };
                api.setup_problem(&text, field.as_deref(), entry);
            }
            other => return Err(mlua::Error::runtime(format!("a problem is a sentence or a table, not {}", other.type_name()))),
        }
    }
    Ok(())
}
