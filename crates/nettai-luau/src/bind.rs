//! The content API as Luau sees it: handles for objects, sprites,
//! collision, content state and navi stats, the `Vec3` value type, and the
//! libraries `battle`, `field`, `dimming`, `navi_chip` and `int` (declared
//! for editors in content/exe6/core.d.luau).
//!
//! Every call goes straight to the engine through [`CoreApi`]; nothing is
//! cached or deferred, so later objects in the same tick see a change at
//! once, as in the game. (A handle, which only names what it reads, is made
//! once and given again: [`Handles`]; and names of panel flags and hit
//! results, which say the same whatever the battle, are resolved once:
//! [`Names`].)
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
    ChoiceKind,
    ActorField, ApiError, BattleInfo, CollisionField, CoreApi, DimmingStep, FieldType,
    HandField, HitboxSpec, HookCall, HudField, Key, Lifecycle, LinkedChip, NamedBits, NaviStat, NaviState, ObjectField, ObstacleAction, OverlayField,
    AssetKind, SpawnAt,
    ObstacleCrush, ObstacleRequest, Pad, PanelPos, Registry, RequestFlag, ScreenFade, SpriteField, SpriteId,
    StateId, StatusFlag, StatusTimer, Value, Vec3,
};
use nettai_content_api::{NameKind, ObjectRef, PanelCall, RulesHook};
use nettai_content_api::{AttackKind, DamageTaken, FightCycle, ObstacleClass, PaletteFlashVariant, SpecialBonus, StepDirection, TakeDamageMode};
use nettai_content_api::{ChipHandle, CollisionHandle, EffectHandle, RegionHandle, SparkHandle};

use crate::Bound;
// Subtypes 8, 17, 18 (Wind, Anubis, Otenko) and the obstacle framework.
use nettai_content_api::{ObstacleHold, ObstaclePush};
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

    /// A call made while another content call runs (from an engine call of
    /// its: a hook the engine runs for it).
    pub fn nested(&self) -> bool {
        self.prev.is_some()
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

/// Handles made once for each VM and given out again: an object's
/// (`Object`), `me.sprite`, `me.collision`, `me.state`, `me.attack_state`,
/// `rules.state()` and `rules.setup()` (and of a side), `battle.navi(side)`,
/// `battle.member(side, m)`, and the views of named members ([`View`]:
/// `me.status`, `me:console(side)`, `battle.hud(side)`...). Such a handle
/// only names what it reads (a value
/// of its own type, which holds nothing else; an `Object`'s `__eq` compares
/// what two name), while making a userdata costs two allocations (Lua's and
/// mlua's) and the GC's work later, for each object updated, each hook's
/// argument, each read through a new one. (Nothing in the content keys a
/// table by a handle or compares two with `rawequal`, which would tell one
/// given again from a new one; and a handle is borrowed only shared, a call
/// further out may hold it: its fields' setters take it as a function's
/// argument, `object_of`.)
#[derive(Default)]
pub(crate) struct Handles {
    kept: std::cell::RefCell<Vec<Option<AnyUserData>>>,
}

/// What a handle names, for [`Handles`].
#[derive(Clone, Copy)]
enum Handle {
    Object(ObjectRef),
    Sprite(ObjectRef),
    Collision(ObjectRef),
    State(StateOf),
    Navi(nettai_content_api::StatsOf),
    View(View),
}

impl Handles {
    /// The objects' slots: four pools of 32.
    const OBJECTS: usize = 4 * 32;
    /// The members a side's party has room for, as handles go.
    const MEMBERS: usize = 16;
    /// The entries of a side's hand, as handles go.
    const HAND: usize = 8;
    /// An object's views: its statuses, timers, requests, navi flags, its
    /// four button records, its obstacle flags, its form's traits, its and
    /// its sprite's two consoles', its sprite's hidden parts (every
    /// console's and each one's), and what hit its collision.
    const OBJECT_VIEWS: usize = 18;
    const VIEWS: usize = 5 * Handles::OBJECTS + 4 + 2 * (1 + Handles::MEMBERS);
    const LEN: usize = Handles::VIEWS + Handles::OBJECT_VIEWS * Handles::OBJECTS + 3 + 2 * Handles::HAND + 2;

    /// Where the handle of `h` is kept (none: one not kept, made each time).
    fn place(h: Handle) -> Option<usize> {
        let object = |o: ObjectRef| ((o.pool as usize) < 4 && o.slot < 32).then(|| (o.pool as usize) * 32 + o.slot as usize);
        let side = |s: u8| (s < 2).then_some(s as usize);
        let rules = 5 * Handles::OBJECTS;
        let navis = rules + 4;
        let view = |o: ObjectRef, n: usize| object(o).map(|i| Handles::VIEWS + i * Handles::OBJECT_VIEWS + n);
        let shared = Handles::VIEWS + Handles::OBJECT_VIEWS * Handles::OBJECTS;
        match h {
            Handle::Object(o) => object(o),
            Handle::State(StateOf::Object(o)) => object(o).map(|i| Handles::OBJECTS + i),
            Handle::State(StateOf::Action(o)) => object(o).map(|i| 2 * Handles::OBJECTS + i),
            Handle::Sprite(o) => object(o).map(|i| 3 * Handles::OBJECTS + i),
            Handle::Collision(o) => object(o).map(|i| 4 * Handles::OBJECTS + i),
            Handle::State(StateOf::Rules(c)) => side(c.side).map(|s| rules + s),
            Handle::State(StateOf::Setup(c)) => side(c.side).map(|s| rules + 2 + s),
            Handle::State(StateOf::ActionAs(..) | StateOf::Stats(_)) => None,
            Handle::Navi(of) => {
                let member = match of.member {
                    None => Some(0),
                    Some(m) => ((m as usize) < Handles::MEMBERS).then_some(m as usize + 1),
                };
                side(of.side).zip(member).map(|(s, m)| navis + s * (1 + Handles::MEMBERS) + m)
            }
            Handle::View(v) => match v {
                View::Statuses(o) => view(o, 0),
                View::StatusTimers(o) => view(o, 1),
                View::Requests(o) => view(o, 2),
                View::NaviFlags(o) => view(o, 3),
                View::Buttons(o, pad) => view(o, 4 + pad as usize),
                View::ObstacleFlags(o) => view(o, 8),
                View::FormTraits(o) => view(o, 9),
                View::ObjectConsole(o, s) => side(s).and_then(|s| view(o, 10 + s)),
                View::SpriteConsole(o, s) => side(s).and_then(|s| view(o, 12 + s)),
                View::HiddenParts(o, None) => view(o, 14),
                View::HiddenParts(o, Some(s)) => side(s).and_then(|s| view(o, 15 + s)),
                View::HitBy(o) => view(o, 17),
                View::Hud(None) => Some(shared),
                View::Hud(Some(s)) => side(s).map(|s| shared + 1 + s),
                View::HandEntry(s, i) => {
                    side(s).zip(((i as usize) < Handles::HAND).then_some(i as usize)).map(|(s, i)| shared + 3 + s * Handles::HAND + i)
                }
                View::Listening(s) => side(s).map(|s| shared + 3 + 2 * Handles::HAND + s),
            },
        }
    }

    /// The handle of `h`: the one given before, if any.
    fn get(lua: &Lua, h: Handle) -> mlua::Result<AnyUserData> {
        match CTX.with(|c| c.get()) {
            // SAFETY: set by a live `Enter`; the binding only ever reads it.
            Some((_, bound)) => Handles::get_in(lua, unsafe { &*bound.as_ptr() }, h),
            // (Outside a content call, as a module loads: one of its own.)
            None => Handles::make(lua, h),
        }
    }

    fn make(lua: &Lua, h: Handle) -> mlua::Result<AnyUserData> {
        match h {
            Handle::Object(o) => lua.create_userdata(Object(o)),
            Handle::Sprite(o) => lua.create_userdata(Sprite(o)),
            Handle::Collision(o) => lua.create_userdata(Collision(o)),
            Handle::State(of) => lua.create_userdata(State(of)),
            Handle::Navi(of) => lua.create_userdata(Navi(of)),
            Handle::View(v) => match v {
                View::Statuses(o) => lua.create_userdata(Statuses(o)),
                View::StatusTimers(o) => lua.create_userdata(StatusTimers(o)),
                View::Requests(o) => lua.create_userdata(Requests(o)),
                View::NaviFlags(o) => lua.create_userdata(NaviFlags(o)),
                View::Buttons(o, pad) => lua.create_userdata(Buttons(o, pad)),
                View::ObstacleFlags(o) => lua.create_userdata(ObstacleFlags(o)),
                View::FormTraits(o) => lua.create_userdata(FormTraits(o)),
                View::ObjectConsole(o, s) => lua.create_userdata(ObjectConsole(o, s)),
                View::SpriteConsole(o, s) => lua.create_userdata(SpriteConsole(o, s)),
                View::HiddenParts(o, s) => lua.create_userdata(HiddenParts(o, s)),
                View::HitBy(o) => lua.create_userdata(HitBy(o)),
                View::Hud(s) => lua.create_userdata(Hud(s)),
                View::HandEntry(s, i) => lua.create_userdata(HandEntry(s, i)),
                View::Listening(s) => lua.create_userdata(Listening(s)),
            },
        }
    }

    /// The handle of `h` in the VM `bound` is the binding of.
    fn get_in(lua: &Lua, bound: &Bound, h: Handle) -> mlua::Result<AnyUserData> {
        let make = || Handles::make(lua, h);
        let Some(at) = Handles::place(h) else { return make() };
        let handles = &bound.handles;
        if let Some(Some(ud)) = handles.kept.borrow().get(at) {
            return Ok(ud.clone());
        }
        let ud = make()?;
        let mut kept = handles.kept.borrow_mut();
        if kept.is_empty() {
            kept.resize(Handles::LEN, None);
        }
        kept[at] = Some(ud.clone());
        Ok(ud)
    }
}

fn object_handle(lua: &Lua, o: ObjectRef) -> mlua::Result<AnyUserData> {
    Handles::get(lua, Handle::Object(o))
}

/// Objects as a list (a sequence of their handles).
fn object_list(lua: &Lua, list: Vec<ObjectRef>) -> mlua::Result<mlua::Table> {
    let handles = list.into_iter().map(|o| object_handle(lua, o)).collect::<mlua::Result<Vec<_>>>()?;
    lua.create_sequence_from(handles)
}

fn state_handle(lua: &Lua, of: StateOf) -> mlua::Result<AnyUserData> {
    Handles::get(lua, Handle::State(of))
}

fn navi_handle(lua: &Lua, of: nettai_content_api::StatsOf) -> mlua::Result<AnyUserData> {
    Handles::get(lua, Handle::Navi(of))
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

/// What a message names a value by, made only for the message (a state's
/// part's name is built from its path).
type What<'a> = &'a dyn Fn() -> String;

fn exact(n: f64, what: What) -> mlua::Result<i64> {
    if n.fract() == 0.0 && n.abs() <= EXACT {
        Ok(n as i64)
    } else {
        Err(mlua::Error::runtime(format!(
            "{}: {n} is not an integer (engine values are integers; use // or the int library)",
            what()
        )))
    }
}

pub(crate) fn int(v: &LuaValue, what: &str) -> mlua::Result<i64> {
    int_as(v, &|| what.to_string())
}

fn int_as(v: &LuaValue, what: What) -> mlua::Result<i64> {
    match v {
        LuaValue::Number(n) => exact(*n, what),
        LuaValue::Integer(i) => Ok(i64::from(*i)),
        v => Err(mlua::Error::runtime(format!("{}: expected an integer, got {}", what(), v.type_name()))),
    }
}

fn u8_arg(v: LuaValue, what: &str) -> mlua::Result<u8> {
    Ok(int(&v, what)? as u8)
}

/// A side: 0 or 1.
fn side_arg(v: LuaValue) -> mlua::Result<u8> {
    match int(&v, "side")? {
        s @ (0 | 1) => Ok(s as u8),
        s => Err(mlua::Error::runtime(format!("side: {s} is no side (0 or 1)"))),
    }
}

fn u16_arg(v: LuaValue, what: &str) -> mlua::Result<u16> {
    Ok(int(&v, what)? as u16)
}

/// The bits a set of names gives in a set of named bits (`NamedBits`): a
/// table of flags keyed by the members' names (`{ flinch = true }`), the
/// engine's names and those its game's rules declare; a member set false is
/// none. An unknown name, a value that is no flag, or anything but such a
/// table is an error (luau-analyze lets a table literal's extra keys
/// through).
fn named_bits(api: &dyn CoreApi, set: NamedBits, v: &LuaValue, what: &str) -> mlua::Result<u32> {
    let LuaValue::Table(t) = v else {
        return Err(mlua::Error::runtime(format!("{what}: expected a set of {}s (`{{ name = true }}`), got {}", set.what(), v.type_name())));
    };
    let mut bits = 0;
    for pair in t.pairs::<LuaValue, LuaValue>() {
        let (k, on) = pair?;
        let (LuaValue::String(name), LuaValue::Boolean(on)) = (&k, &on) else {
            return Err(mlua::Error::runtime(format!(
                "{what}: a set of {}s is keyed by their names, each true or false (`{{ name = true }}`), not {k:?} = {on:?}",
                set.what()
            )));
        };
        let bit = api.named_bit(set, &name.to_str()?).map_err(|e| mlua::Error::runtime(format!("{what}: {e}")))?;
        if *on {
            bits |= bit;
        }
    }
    Ok(bits)
}

/// A read-only view of a word's bits by name (`hit_modifier.bits`,
/// `damage_word.flags`): each member's name reads whether the word has
/// it; a name its set hasn't is an error.
#[derive(Clone, Copy)]
struct BitsView(NamedBits, u32);

impl UserData for BitsView {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(mlua::MetaMethod::Index, |_, this, name: mlua::LuaString| {
            let bit = with(|api, _| api.named_bit(this.0, &name.to_str()?).map_err(api_error))?;
            Ok(this.1 & bit != 0)
        });
        methods.add_meta_method(mlua::MetaMethod::NewIndex, |_, this, (_, _): (LuaValue, LuaValue)| -> mlua::Result<()> {
            Err(mlua::Error::runtime(format!("a word's {}s are read here, not written", this.0.what())))
        });
    }
}

/// `battle.hand_entry(side, i).marks`: an entry's marks by name, each read
/// and written (`NamedBits::Mark`).
#[derive(Clone, Copy)]
struct HandMarks(u8, u8);

impl UserData for HandMarks {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(mlua::MetaMethod::Index, |_, this, name: mlua::LuaString| {
            with(|api, _| {
                let bit = api.named_bit(NamedBits::Mark, &name.to_str()?).map_err(api_error)?;
                let Value::Int(marks) = api.hand_entry(this.0, this.1, HandField::Marks) else { return Ok(false) };
                Ok(marks as u32 & bit != 0)
            })
        });
        methods.add_meta_method(mlua::MetaMethod::NewIndex, |_, this, (name, on): (mlua::LuaString, bool)| {
            with(|api, _| {
                let bit = api.named_bit(NamedBits::Mark, &name.to_str()?).map_err(api_error)?;
                let Value::Int(marks) = api.hand_entry(this.0, this.1, HandField::Marks) else {
                    return Err(mlua::Error::runtime("marks: the hand has no such entry"));
                };
                let marks = if on { marks as u32 | bit } else { marks as u32 & !bit };
                api.set_hand_entry(this.0, this.1, HandField::Marks, Value::Int(marks as i64)).map_err(api_error)
            })
        });
    }
}

/// A hit modifier: a set of its bits by name (`NamedBits::HitModifier`), or
/// a modifier byte copied whole (one the engine or a state holds).
fn hit_modifier_arg(api: &dyn CoreApi, v: &LuaValue, what: &str) -> mlua::Result<u8> {
    match v {
        LuaValue::Table(_) => Ok(named_bits(api, NamedBits::HitModifier, v, what)? as u8),
        LuaValue::Integer(_) | LuaValue::Number(_) => match int(v, what)? {
            b @ 0..=0xFF => Ok(b as u8),
            b => Err(mlua::Error::runtime(format!("{what}: {b:#x} is no hit modifier byte"))),
        },
        v => Err(mlua::Error::runtime(format!("{what}: expected a hit modifier (a set of its bits by name), got {}", v.type_name()))),
    }
}

/// A damage word: a whole number of 32 bits.
fn word_arg(v: &LuaValue, what: &str) -> mlua::Result<u32> {
    match int(v, what)? {
        w @ 0..=0xFFFF_FFFF => Ok(w as u32),
        w => Err(mlua::Error::runtime(format!("{what}: {w} is no damage word (0 to 0xFFFFFFFF)"))),
    }
}

fn panel(x: LuaValue, y: LuaValue) -> mlua::Result<PanelPos> {
    Ok(PanelPos { x: u8_arg(x, "panel x")?, y: u8_arg(y, "panel y")? })
}

/// A string a script passes, read as text.
trait NameArg {
    fn with_str<R>(&self, f: impl FnOnce(&str) -> R) -> mlua::Result<R>;
}

impl NameArg for mlua::LuaString {
    fn with_str<R>(&self, f: impl FnOnce(&str) -> R) -> mlua::Result<R> {
        Ok(f(&self.to_str()?))
    }
}

impl NameArg for Name {
    fn with_str<R>(&self, f: impl FnOnce(&str) -> R) -> mlua::Result<R> {
        Ok(f(self.to_str()?))
    }
}

/// A string argument (a field's name, a flag's), its bytes copied from the
/// stack: a `LuaString` argument holds a reference to the string, which
/// mlua makes and frees for each call, and reads its text through another.
/// What isn't a string converts as for a `LuaString` (a number to its
/// text; anything else an error).
pub(crate) enum Name {
    /// Up to [`Name::SHORT`] bytes, kept without an allocation: its length
    /// and its bytes.
    Short(u8, [u8; Name::SHORT]),
    Long(Box<[u8]>),
}

impl Name {
    const SHORT: usize = 30;

    fn of(bytes: &[u8]) -> Name {
        if bytes.len() <= Name::SHORT {
            let mut short = [0; Name::SHORT];
            short[..bytes.len()].copy_from_slice(bytes);
            Name::Short(bytes.len() as u8, short)
        } else {
            Name::Long(bytes.into())
        }
    }

    fn as_bytes(&self) -> &[u8] {
        match self {
            Name::Short(len, bytes) => &bytes[..*len as usize],
            Name::Long(bytes) => bytes,
        }
    }

    /// Its text, for a message (a field it names is UTF-8).
    fn lossy(&self) -> std::borrow::Cow<'_, str> {
        String::from_utf8_lossy(self.as_bytes())
    }

    /// Its text (an error as `LuaString::to_str`'s, if it isn't UTF-8).
    fn to_str(&self) -> mlua::Result<&str> {
        std::str::from_utf8(self.as_bytes()).map_err(|e| mlua::Error::FromLuaConversionError {
            from: "string",
            to: "&str".to_string(),
            message: Some(e.to_string()),
        })
    }
}

/// A key a script indexes a part of a state by: a name (a record's field,
/// read as a [`Name`]), or another value (an element's number).
pub(crate) enum PartKey {
    Name(Name),
    Other(LuaValue),
}

impl mlua::FromLua for PartKey {
    fn from_lua(value: LuaValue, _: &Lua) -> mlua::Result<PartKey> {
        Ok(match value {
            LuaValue::String(s) => PartKey::Name(Name::of(&s.as_bytes())),
            v => PartKey::Other(v),
        })
    }

    unsafe fn from_stack(idx: std::ffi::c_int, lua: &mlua::state::RawLua) -> mlua::Result<PartKey> {
        // SAFETY: as `Name::from_stack`.
        unsafe {
            if mlua::ffi::lua_type(lua.state(), idx) == mlua::ffi::LUA_TSTRING {
                return Ok(PartKey::Name(<Name as mlua::FromLua>::from_stack(idx, lua)?));
            }
            Ok(PartKey::Other(<LuaValue as mlua::FromLua>::from_stack(idx, lua)?))
        }
    }
}

impl mlua::FromLua for Name {
    fn from_lua(value: LuaValue, lua: &Lua) -> mlua::Result<Name> {
        let s = mlua::LuaString::from_lua(value, lua)?;
        Ok(Name::of(&s.as_bytes()))
    }

    unsafe fn from_stack(idx: std::ffi::c_int, lua: &mlua::state::RawLua) -> mlua::Result<Name> {
        let state = lua.state();
        // SAFETY: a string at `idx` of the running call's stack: its bytes
        // live while it is there, and are copied out.
        unsafe {
            if mlua::ffi::lua_type(state, idx) == mlua::ffi::LUA_TSTRING {
                let mut len = 0;
                let data = mlua::ffi::lua_tolstring(state, idx, &mut len);
                return Ok(Name::of(std::slice::from_raw_parts(data as *const u8, len)));
            }
            let s = <mlua::LuaString as mlua::FromLua>::from_stack(idx, lua)?;
            Ok(Name::of(&s.as_bytes()))
        }
    }
}

// ---- Values -------------------------------------------------------------------------

/// A named field's type, made once for the program: `ty()` makes it anew
/// each time (an enum's, a list of its names).
trait FieldTypes: Copy + 'static {
    fn all() -> &'static [Self];
    fn made(self) -> FieldType;
    fn types() -> &'static std::sync::OnceLock<Vec<FieldType>>;
    fn index(self) -> usize;

    fn field_type(self) -> &'static FieldType {
        &Self::types().get_or_init(|| Self::all().iter().map(|f| f.made()).collect())[self.index()]
    }
}

macro_rules! field_types {
    ($($t:ty),* $(,)?) => {$(
        impl FieldTypes for $t {
            fn all() -> &'static [Self] {
                <$t>::ALL
            }
            fn made(self) -> FieldType {
                self.ty()
            }
            fn types() -> &'static std::sync::OnceLock<Vec<FieldType>> {
                static TYPES: std::sync::OnceLock<Vec<FieldType>> = std::sync::OnceLock::new();
                &TYPES
            }
            fn index(self) -> usize {
                // (`ALL` lists the variants in their order.)
                self as usize
            }
        }
    )*};
}

field_types!(ObjectField, ActorField, OverlayField, SpriteField, CollisionField, NaviStat, BattleInfo, HudField, HandField);

/// A Luau value going into a field of type `ty`.
fn to_api(v: LuaValue, ty: &FieldType, what: &str) -> mlua::Result<Value> {
    to_api_as(v, ty, &|| what.to_string())
}

/// [`to_api`], the field named only for a message.
fn to_api_as(v: LuaValue, ty: &FieldType, what: What) -> mlua::Result<Value> {
    Ok(match v {
        LuaValue::Nil => Value::Nil,
        LuaValue::Table(_) => bound(|b| {
            if let Some((registry, h)) = b.def(&v) {
                if let FieldType::Ref(Registry::Record, Some(want)) = ty
                    && registry == Registry::Record
                    && b.record_type(registry, h) != Some(want.as_str())
                {
                    return Err(mlua::Error::runtime(format!(
                        "{}: expected a record of type {want}, got one of type {}",
                        what(),
                        b.record_type(registry, h).unwrap_or("?")
                    )));
                }
                if let FieldType::Ref(Registry::Entry, Some(want)) = ty
                    && registry == Registry::Entry
                    && b.record_type(registry, h) != Some(want.as_str())
                {
                    return Err(mlua::Error::runtime(format!(
                        "{}: expected an entry of {want}, got one of {}",
                        what(),
                        b.record_type(registry, h).unwrap_or("?")
                    )));
                }
                return Ok(Value::Def(registry, h));
            }
            if let Some((kind, h)) = b.asset(&v) {
                return Ok(Value::Asset(kind, h));
            }
            Err(mlua::Error::runtime(format!("{}: expected {ty}, got a table that is no definition or asset", what())))
        })?,
        LuaValue::Boolean(b) => Value::Bool(b),
        LuaValue::String(s) => match ty {
            FieldType::Code => {
                let s = s.to_str()?;
                match s.as_bytes() {
                    [c] if nettai_content_api::is_code(*c) => Value::Code(*c),
                    _ => return Err(mlua::Error::runtime(format!("{}: {:?} is no chip code (a letter A to Z, or *)", what(), &*s))),
                }
            }
            FieldType::Enum(names) if let Some(kind) = crate::choice::kind_of(names) => {
                return Err(crate::choice::expected(&LuaValue::String(s), kind, &what()));
            }
            FieldType::Enum(names) => {
                let s = s.to_str()?;
                let i = names.iter().position(|n| *n == *s).ok_or_else(|| {
                    mlua::Error::runtime(format!("{}: {:?} is not one of {}", what(), &*s, names.join(", ")))
                })?;
                Value::Int(i as i64)
            }
            _ => return Err(mlua::Error::runtime(format!("{}: expected {ty}, got a string", what()))),
        },
        // (A field a kind's table declares holds the kind's values.)
        LuaValue::UserData(_) if let (FieldType::Enum(names), Some(c)) = (ty, crate::choice::of(&v)) => match crate::choice::kind_of(names) {
            Some(kind) if kind == c.kind => Value::Int(i64::from(c.index)),
            Some(kind) => return Err(crate::choice::expected(&v, kind, &what())),
            None => return Err(mlua::Error::runtime(format!("{}: expected one of {}, got {c}", what(), names.join(", ")))),
        },
        LuaValue::UserData(ud) => {
            if let Ok(o) = ud.borrow::<Object>() {
                Value::Object(o.0)
            } else if let Ok(p) = ud.borrow::<LVec3>() {
                Value::Vec3(p.0)
            } else {
                return Err(mlua::Error::runtime(format!(
                    "{}: expected {ty}, got {}",
                    what(),
                    ud.type_name().map(|s| s.to_string_lossy()).unwrap_or_default()
                )));
            }
        }
        v @ (LuaValue::Number(_) | LuaValue::Integer(_)) => Value::Int(int_as(&v, what)?),
        v => return Err(mlua::Error::runtime(format!("{}: expected {ty}, got {}", what(), v.type_name()))),
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
            FieldType::Enum(names) if let Some(kind) = crate::choice::kind_of(names) => {
                if (i as usize) < names.len() { crate::choice::value_of(lua, kind, i as u8)? } else { LuaValue::Nil }
            }
            FieldType::Enum(names) => match names.get(i as usize) {
                Some(name) => LuaValue::String(lua.create_string(name)?),
                None => LuaValue::Nil,
            },
            _ => LuaValue::Number(i as f64),
        },
        Value::Object(o) => LuaValue::UserData(object_handle(lua, o)?),
        Value::Vec3(p) => LuaValue::UserData(lua.create_userdata(LVec3(p))?),
        Value::Def(r, h) => LuaValue::Table(bound(|b| b.def_value(r, h))?),
        Value::Asset(k, h) => LuaValue::Table(bound(|b| b.asset_value(k, h))?),
        Value::Code(c) => LuaValue::String(lua.create_string([c])?),
    })
}

/// How the binding shows a field the engine keeps as a word whose values
/// content names (docs/design/scripting.md §2.1): the engine's field is the
/// same word, and only its form in Luau differs.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Shown {
    /// As the field's type says.
    Plain,
    /// An element byte: an `Element`.
    Element,
    /// The secondary elements of an element byte: a view of them by name.
    Secondary,
    /// A byte that is 0 or 1, the object turned against its side's way (its
    /// flip): a boolean.
    Turned,
    /// A navi's mood: a number, or `Mood.full_synchro` for the byte 0xFF.
    Mood,
    /// A count that holds at 0xFFFF: a number, or `Timer.holds`.
    Holds,
    /// A byte that is 0xFF for none: a number, or nil.
    NoneAt0xFF,
    /// A sprite's hidden parts (bit 31 − i hides part i), written as a
    /// table of parts (from 0) to whether each is hidden (`{ [6] = true }`);
    /// read as a view by part (`HiddenParts`).
    Parts,
    /// A choice the engine keeps as its member's place: the kind's value
    /// (`DragStep.slide`).
    Choice(ChoiceKind),
}

impl Shown {
    /// How a field of type `ty` is shown when nothing else says: a choice
    /// as its kind's value (an engine field of named variants is one of the
    /// engine's choices), anything else as its type says.
    fn of(ty: &FieldType) -> Shown {
        match ty {
            FieldType::Enum(names) => Shown::Choice(
                crate::choice::kind_of(names).unwrap_or_else(|| panic!("the engine's field of {} is no choice", names.join(" | "))),
            ),
            _ => Shown::Plain,
        }
    }
}

/// An engine value of a field shown as `shown`.
fn show(lua: &Lua, v: Value, ty: &FieldType, shown: Shown) -> mlua::Result<LuaValue> {
    let Value::Int(i) = v else { return from_api(lua, v, ty) };
    Ok(match shown {
        Shown::Plain => return from_api(lua, v, ty),
        Shown::Element => LuaValue::UserData(crate::element::value(lua, i as u8)?),
        Shown::Secondary => LuaValue::UserData(crate::element::secondary_value(lua, i as u8)?),
        Shown::Turned => LuaValue::Boolean(i != 0),
        Shown::Mood if i == 0xFF => crate::choice::value_of(lua, ChoiceKind::Mood, 0)?,
        Shown::Holds if i == 0xFFFF => crate::choice::value_of(lua, ChoiceKind::Timer, 0)?,
        Shown::NoneAt0xFF if i == 0xFF => LuaValue::Nil,
        Shown::Mood | Shown::Holds | Shown::NoneAt0xFF => LuaValue::Number(i as f64),
        Shown::Parts => unreachable!("hidden parts read as a view"),
        Shown::Choice(kind) if (i as usize) < kind.members().len() => crate::choice::value_of(lua, kind, i as u8)?,
        // (A choice nothing has stated.)
        Shown::Choice(_) => LuaValue::Nil,
    })
}

/// A Luau value going into a field shown as `shown`.
fn take(v: LuaValue, ty: &FieldType, shown: Shown, what: &str) -> mlua::Result<Value> {
    // (A word with a sentinel: a number, or the sentinel's value, the only
    // member of its kind.)
    let word = |v: LuaValue, sentinel: i64, kind: ChoiceKind| -> mlua::Result<Value> {
        match &v {
            LuaValue::Number(_) | LuaValue::Integer(_) => {
                let n = int(&v, what)?;
                if n == sentinel {
                    return Err(mlua::Error::runtime(format!("{what}: {sentinel:#x} is {}", nettai_content_api::Choice { kind, index: 0 })));
                }
                Ok(Value::Int(n))
            }
            v => crate::choice::arg(v, kind, what).map(|_| Value::Int(sentinel)),
        }
    };
    match shown {
        Shown::Plain => to_api(v, ty, what),
        Shown::Element => Ok(Value::Int(i64::from(crate::element::arg(&v, what)?))),
        Shown::Secondary => Err(mlua::Error::runtime(format!("{what} is read-only"))),
        Shown::Turned => match v {
            LuaValue::Boolean(b) => Ok(Value::Int(i64::from(b))),
            v => Err(mlua::Error::runtime(format!("{what}: expected a boolean (turned), got {}", v.type_name()))),
        },
        Shown::Mood => word(v, 0xFF, ChoiceKind::Mood),
        Shown::Holds => word(v, 0xFFFF, ChoiceKind::Timer),
        Shown::NoneAt0xFF => match v {
            LuaValue::Nil => Ok(Value::Int(0xFF)),
            v => {
                let n = int(&v, what)?;
                if n == 0xFF {
                    return Err(mlua::Error::runtime(format!("{what}: 0xff is none: nil")));
                }
                Ok(Value::Int(n))
            }
        },
        Shown::Parts => Ok(Value::Int(i64::from(parts_word(&v, what)?))),
        Shown::Choice(_) if v.is_nil() => Ok(Value::Nil),
        Shown::Choice(kind) => Ok(Value::Int(i64::from(crate::choice::arg(&v, kind, what)?.index))),
    }
}

/// A table of sprite parts (0 to 31) to whether each is hidden, as the word
/// that hides them (bit 31 − i hides part i): a part past 31 or a value not
/// a boolean is an error.
fn parts_word(v: &LuaValue, what: &str) -> mlua::Result<u32> {
    let LuaValue::Table(t) = v else {
        return Err(mlua::Error::runtime(format!("{what}: expected a table of sprite parts ({{ [6] = true }}), got {}", v.type_name())));
    };
    let mut w = 0u32;
    for pair in t.pairs::<LuaValue, LuaValue>() {
        let (part, hidden) = pair?;
        let part = part_index(&part, what)?;
        match hidden {
            LuaValue::Boolean(true) => w |= 0x8000_0000 >> part,
            LuaValue::Boolean(false) => {}
            v => return Err(mlua::Error::runtime(format!("{what}: part {part} is hidden true or false, not {}", v.type_name()))),
        }
    }
    Ok(w)
}

/// A sprite part's index, 0 to 31.
fn part_index(v: &LuaValue, what: &str) -> mlua::Result<u32> {
    let part = match v {
        LuaValue::Integer(_) | LuaValue::Number(_) => int(v, what)?,
        v => return Err(mlua::Error::runtime(format!("{what}: a sprite part is its index, not {}", v.type_name()))),
    };
    u32::try_from(part).ok().filter(|p| *p < 32).ok_or_else(|| mlua::Error::runtime(format!("{what}: a sprite has parts 0 to 31, not {part}")))
}

fn object_value(lua: &Lua, o: Option<ObjectRef>) -> mlua::Result<LuaValue> {
    o.map_or(Ok(LuaValue::Nil), |o| Ok(LuaValue::UserData(object_handle(lua, o)?)))
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

/// The object a handle names, the handle borrowed for as long as it is read:
/// a field's setter takes its handle so, as a handle given out again
/// ([`Handles`]) may be borrowed by a call further out (a method's, whose
/// engine call ran a hook), which a mutable borrow would refuse.
fn object_of(ud: &AnyUserData) -> mlua::Result<ObjectRef> {
    Ok(ud.borrow::<Object>()?.0)
}

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
            let shown = match f {
                ObjectField::Element => Shown::Element,
                ObjectField::Flip => Shown::Turned,
                _ => Shown::of(f.field_type()),
            };
            fields.add_field_method_get(f.name(), move |lua, this| {
                let v = with(|api, _| api.get(this.0, f).map_err(api_error))?;
                show(lua, v, f.field_type(), shown)
            });
            if f.writable() {
                fields.add_field_function_set(f.name(), move |_, this, v: LuaValue| {
                    let o = object_of(&this)?;
                    let v = take(v, f.field_type(), shown, f.name())?;
                    with(|api, _| api.set(o, f, v).map_err(api_error))
                });
            }
        }
        // The side whose way it faces: its own, or the other turned (its
        // alliance ^ flip).
        fields.add_field_method_get("facing_side", |_, this| {
            let (side, flip) = with(|api, _| Ok((api.get(this.0, ObjectField::Alliance).map_err(api_error)?, api.get(this.0, ObjectField::Flip).map_err(api_error)?)))?;
            match (side, flip) {
                (Value::Int(side), Value::Int(flip)) => Ok(side ^ flip),
                _ => unreachable!("alliance and flip are bytes"),
            }
        });
        for &f in ActorField::ALL {
            let shown = match f {
                ActorField::AttackElement => Shown::Element,
                ActorField::NoChargeTimer => Shown::Holds,
                _ => Shown::of(f.field_type()),
            };
            fields.add_field_method_get(f.name(), move |lua, this| {
                let v = with(|api, _| api.actor_get(this.0, f).map_err(api_error))?;
                show(lua, v, f.field_type(), shown)
            });
            if f.writable() {
                fields.add_field_function_set(f.name(), move |_, this, v: LuaValue| {
                    let o = object_of(&this)?;
                    let v = take(v, f.field_type(), shown, f.name())?;
                    with(|api, _| api.actor_set(o, f, v).map_err(api_error))
                });
            }
        }
        for &f in OverlayField::ALL {
            fields.add_field_method_get(f.name(), move |lua, this| {
                let v = with(|api, _| api.overlay_get(this.0, f).map_err(api_error))?;
                show(lua, v, f.field_type(), Shown::of(f.field_type()))
            });
            fields.add_field_function_set(f.name(), move |_, this, v: LuaValue| {
                let o = object_of(&this)?;
                let v = take(v, f.field_type(), Shown::of(f.field_type()), f.name())?;
                with(|api, _| api.overlay_set(o, f, v).map_err(api_error))
            });
        }
        fields.add_field_method_get("pool", |lua, this| crate::choice::value_of(lua, ChoiceKind::Pool, this.0.pool as u8));
        fields.add_field_method_get("kind", |lua, this| {
            let kind = with(|api, _| Ok(api.object_kind(this.0)))?;
            match kind {
                Some(h) => from_api(lua, Value::Def(Registry::Kind, h), &FieldType::Ref(Registry::Kind, None)),
                None => Ok(LuaValue::Nil),
            }
        });
        fields.add_field_method_get("lifecycle", |lua, this| {
            let l = with(|api, _| Ok(api.lifecycle(this.0)))?;
            crate::choice::value_of(lua, ChoiceKind::Lifecycle, l as u8)
        });
        fields.add_field_function_set("lifecycle", |_, this, v: LuaValue| {
            let o = object_of(&this)?;
            let l = crate::choice::parsed(&v, ChoiceKind::Lifecycle, "lifecycle", Lifecycle::from_name)?;
            with(|api, _| Ok(api.set_lifecycle_only(o, l)))
        });
        fields.add_field_method_get("facing", |_, this| with(|api, _| Ok(api.facing(this.0))));
        fields.add_field_method_get("sprite", |lua, this| Handles::get(lua, Handle::Sprite(this.0)));
        fields.add_field_method_get("collision", |lua, this| Handles::get(lua, Handle::Collision(this.0)));
        fields.add_field_method_get("state", |lua, this| state_handle(lua, StateOf::Object(this.0)));
        fields.add_field_method_get("attack_state", |lua, this| state_handle(lua, StateOf::Action(this.0)));
        // Its named members, each a view's properties.
        fields.add_field_method_get("status", |lua, this| view(lua, View::Statuses(this.0)));
        fields.add_field_method_get("status_timers", |lua, this| view(lua, View::StatusTimers(this.0)));
        fields.add_field_method_get("requests", |lua, this| view(lua, View::Requests(this.0)));
        fields.add_field_method_get("navi_flags", |lua, this| view(lua, View::NaviFlags(this.0)));
        for &pad in Pad::ALL {
            fields.add_field_method_get(pad.name(), move |lua, this| view(lua, View::Buttons(this.0, pad)));
        }
        fields.add_field_method_get("obstacle_flags", |lua, this| view(lua, View::ObstacleFlags(this.0)));
        fields.add_field_method_get("form_traits", |lua, this| view(lua, View::FormTraits(this.0)));
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::Eq, |_, this, other: AnyUserData| {
            Ok(other.borrow::<Object>().is_ok_and(|o| o.0 == this.0))
        });
        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| {
            Ok(format!("Object({} {})", this.0.pool.name(), this.0.slot))
        });

        // Its fields as a console has them.
        methods.add_method("console", |lua, this, side: LuaValue| view(lua, View::ObjectConsole(this.0, side_arg(side)?)));

        // Lifecycle: entering a state, from its first action and phase.
        for (method, l) in [
            ("enter_init", Lifecycle::Init),
            ("enter_update", Lifecycle::Update),
            ("enter_destroy", Lifecycle::Destroy),
            ("enter_finish", Lifecycle::Finish),
        ] {
            methods.add_method(method, move |_, this, ()| with(|api, _| Ok(api.set_lifecycle(this.0, l))));
        }
        methods.add_method("set_action", |_, this, a: LuaValue| {
            let a = u8_arg(a, "action")?;
            with(|api, _| api.set_action(this.0, a).map_err(api_error))
        });
        // Drives: what the engine runs in the object's place of the update order instead of its kind's update.
        methods.add_method("follow", |_, this, (target, spec): (mlua::UserDataRef<Object>, mlua::Table)| {
            use nettai_content_api::{Drive, Follow, Held};
            let offset = vec3_arg(spec.raw_get::<Option<mlua::UserDataRef<LVec3>>>("offset")?);
            let mut looks = 0u16;
            let set = spec.raw_get::<LuaValue>("looks")?;
            if !set.is_nil() {
                for name in name_set(&set, "follow's looks")? {
                    let f = SpriteField::from_name(&name).ok_or_else(|| mlua::Error::runtime(format!("follow's looks: {name:?} is no sprite field")))?;
                    looks |= 1 << SpriteField::ALL.iter().position(|x| *x == f).expect("a sprite field of ALL");
                }
            }
            let held = match (spec.raw_get::<Option<Name>>("held")?, spec.raw_get::<Option<Name>>("held_in_state")?) {
                (Some(n), None) => {
                    let object = nettai_content_api::FieldType::Object;
                    let field = n.with_str(|n| (ObjectField::from_name(n), ActorField::from_name(n)))?;
                    match field {
                        (Some(f), _) if *f.field_type() == object => Held::Field(f),
                        (None, Some(f)) if *f.field_type() == object => Held::Actor(f),
                        _ => return Err(mlua::Error::runtime(format!("follow: `held` is an object field or a navi field that holds an object, not {:?}", n.lossy()))),
                    }
                }
                (None, Some(n)) => with(|api, bound| {
                    let layout = api.state_mut(target.0).map(|s| s.id()).ok_or_else(|| api_error(ApiError::NoState(target.0)))?;
                    let field = field_named(bound.manifest.schema(layout), n.as_bytes())
                        .ok_or_else(|| mlua::Error::runtime(format!("follow: the followed object's state has no field {:?}", n.lossy())))?;
                    Ok(Held::State { layout, field: field as u16 })
                })?,
                _ => return Err(mlua::Error::runtime("follow: what holds it is `held` (an object field) or `held_in_state` (a state field), one of them")),
            };
            let f = Follow {
                target: target.0,
                offset,
                looks,
                visibility: spec.raw_get::<Option<bool>>("visibility")?.unwrap_or(false),
                held,
                while_dimmed: spec.raw_get::<Option<bool>>("while_dimmed")?.unwrap_or(false),
            };
            with(|api, _| api.set_drive(this.0, Drive::Follow(f)).map_err(api_error))
        });
        methods.add_method("fly", |_, this, climbs: Option<bool>| {
            let d = nettai_content_api::Drive::Fly(nettai_content_api::Fly { climbs: climbs.unwrap_or(false) });
            with(|api, _| api.set_drive(this.0, d).map_err(api_error))
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
        methods.add_method("set_header_flags", |_, this, flags: mlua::Table| {
            let mut byte = 0u8;
            for pair in flags.pairs::<LuaValue, LuaValue>() {
                let (k, on) = pair?;
                let name = match &k {
                    LuaValue::String(s) => s.to_str()?.to_string(),
                    k => return Err(mlua::Error::runtime(format!("set_header_flags: a flag is named, not {}", k.type_name()))),
                };
                let bit = HEADER_FLAGS.iter().find(|(n, _)| *n == name).map(|&(_, b)| b).ok_or_else(|| {
                    let names: Vec<&str> = HEADER_FLAGS.iter().map(|(n, _)| *n).collect();
                    mlua::Error::runtime(format!("set_header_flags: {name:?} is no header flag ({})", names.join(", ")))
                })?;
                match on {
                    LuaValue::Boolean(true) => byte |= bit,
                    LuaValue::Boolean(false) => {}
                    v => return Err(mlua::Error::runtime(format!("set_header_flags: `{name}` is true or false, not {}", v.type_name()))),
                }
            }
            with(|api, _| Ok(api.set_header_flags(this.0, byte)))
        });
        methods.add_method("unreserve_panel", |_, this, (x, y): (LuaValue, LuaValue)| {
            let p = panel(x, y)?;
            with(|api, _| Ok(api.unreserve_panel(this.0, p)))
        });
        methods.add_method("can_step", |_, this, (x, y): (LuaValue, LuaValue)| {
            let p = panel(x, y)?;
            with(|api, _| Ok(api.can_step(this.0, p)))
        });
        methods.add_method("chips_enabled", |_, this, ()| with(|api, _| api.chips_enabled(this.0).map_err(api_error)));
        methods.add_method("move_lag", |_, this, ()| with(|api, _| api.move_lag(this.0).map_err(api_error)));
        methods.add_method("burn_on_panel", |_, this, ()| with(|api, _| api.burn_on_panel(this.0).map_err(api_error)));
        methods.add_method("leave", |_, this, ()| with(|api, _| api.leave(this.0).map_err(api_error)));
        methods.add_method("can_stand_any_side", |_, this, (x, y): (LuaValue, LuaValue)| {
            let p = panel(x, y)?;
            with(|api, _| Ok(api.can_stand_any_side(this.0, p)))
        });

        // Status (the flags and timers are views: `me.status`, `me.status_timers`).
        methods.add_method("clear_statuses", |_, this, ()| with(|api, _| api.clear_statuses(this.0).map_err(api_error)));

        // Collision.
        methods.add_method("create_collision", |_, this, ()| with(|api, _| Ok(api.create_collision(this.0))));
        methods.add_method(
            "setup_collision",
            |_, this, (self_type, target_type, hit_mod): (LuaValue, LuaValue, LuaValue)| {
                with(|api, b| {
                    let h = hit_modifier_arg(api, &hit_mod, "setup_collision: hit modifier")?;
                    let s = CollisionHandle(def_arg(b, &self_type, Registry::Collision, "setup_collision")?);
                    let t = CollisionHandle(def_arg(b, &target_type, Registry::Collision, "setup_collision")?);
                    Ok(api.setup_collision(this.0, s, t, h))
                })
            },
        );
        methods.add_method(
            "reset_collision_types",
            |_, this, (self_type, target_type, hit_mod): (LuaValue, LuaValue, LuaValue)| {
                with(|api, b| {
                    let h = hit_modifier_arg(api, &hit_mod, "reset_collision_types: hit modifier")?;
                    let s = CollisionHandle(def_arg(b, &self_type, Registry::Collision, "reset_collision_types")?);
                    let t = CollisionHandle(def_arg(b, &target_type, Registry::Collision, "reset_collision_types")?);
                    Ok(api.reset_collision_types(this.0, s, t, h))
                })
            },
        );
        methods.add_method("hit_spark", |_, this, ()| with(|api, _| Ok(api.hit_spark(this.0))));
        methods.add_method("take_damage", |lua, this, mode: LuaValue| {
            let mode = crate::choice::parsed(&mode, ChoiceKind::TakeDamageMode, "damage mode", TakeDamageMode::from_name)? as u8;
            let taken = with(|api, _| Ok(api.take_damage(this.0, mode)))?;
            crate::choice::named(lua, ChoiceKind::DamageTaken, DamageTaken::of(taken).name())
        });
        methods.add_method("raise_barrier", |_, this, t: mlua::Table| {
            let behavior = crate::choice::arg(&t.raw_get("behavior")?, ChoiceKind::BarrierBehavior, "barrier `behavior`")?.index;
            let byte = |k: &str| -> mlua::Result<u8> {
                u8::try_from(table_int(&t, k)?).map_err(|_| mlua::Error::runtime(format!("barrier `{k}` is not a byte")))
            };
            // (No timer: the game's 0xFFFF, which never runs out.)
            let timer = match t.raw_get::<LuaValue>("timer")? {
                LuaValue::Nil => 0xFFFF,
                v => match u16::try_from(int(&v, "barrier `timer`")?) {
                    Ok(0xFFFF) | Err(_) => return Err(mlua::Error::runtime("barrier `timer` is a count below 0xffff (nil: none)")),
                    Ok(n) => n,
                },
            };
            // (Breaks to no element: the byte 0; null is no weak element.)
            let weak_element = match t.raw_get::<LuaValue>("weak_element")? {
                LuaValue::Nil => 0,
                v => match crate::element::primary_arg(&v, "barrier `weak_element`")? {
                    0 => return Err(mlua::Error::runtime("barrier `weak_element`: null breaks no barrier (nil: none)")),
                    e => e,
                },
            };
            let spec = nettai_content_api::api::BarrierSpec {
                behavior,
                hp: byte("hp")?,
                threshold: byte("threshold")?,
                timer,
                weak_element,
            };
            with(|api, _| api.raise_barrier(this.0, spec).map_err(api_error))
        });

        // Navis: the attack (the requests, flags and buttons are views:
        // `me.requests`, `me.navi_flags`, `me.held`...).
        methods.add_method("open_counter_window", |_, this, ticks: Option<u8>| {
            with(|api, _| Ok(api.open_counter_window(this.0, ticks.unwrap_or(0x10))))
        });
        methods.add_method("check_reactive_abort", |_, this, guard: Option<bool>| {
            with(|api, _| Ok(api.check_reactive_abort(this.0, guard.unwrap_or(false))))
        });
        methods.add_method("spawn_mode9_objects", |_, this, ()| with(|api, _| Ok(api.spawn_mode9_objects(this.0))));
        methods.add_method("start_stance_counter", |_, this, ()| with(|api, _| Ok(api.start_stance_counter(this.0))));
        methods.add_method("refresh_form_overlay", |_, this, ()| with(|api, _| Ok(api.refresh_form_overlay(this.0))));
        methods.add_method("exit_attack", |_, this, keeps_lockout: Option<bool>| {
            with(|api, _| Ok(api.exit_attack(this.0, keeps_lockout.unwrap_or(false))))
        });
        // What a game's rules do to a navi (docs/design/rules-in-luau.md §4.5).
        methods.add_method("face_default", |_, this, ()| with(|api, _| api.face_default(this.0).map_err(api_error)));
        methods.add_method("end_full_synchro_aura", |_, this, ()| {
            with(|api, _| api.end_full_synchro_aura(this.0).map_err(api_error))
        });
        methods.add_method("drop_statuses", |_, this, ()| with(|api, _| api.drop_statuses(this.0).map_err(api_error)));
        methods.add_method("end_statuses", |_, this, ()| with(|api, _| api.end_statuses(this.0).map_err(api_error)));
        methods.add_method("reset_status", |_, this, ()| with(|api, _| api.reset_status(this.0).map_err(api_error)));
        methods.add_method("strip_body_programs", |_, this, undershirt: bool| {
            with(|api, _| api.strip_body_programs(this.0, undershirt).map_err(api_error))
        });
        methods.add_method("refresh_form_flags", |_, this, ()| with(|api, _| api.refresh_form_flags(this.0).map_err(api_error)));
        methods.add_method("take_status", |_, this, status: LuaValue| {
            let status = nettai_content_api::StatusHandle(bound(|b| def_arg(b, &status, Registry::Status, "take_status"))?);
            with(|api, _| api.take_status(this.0, status).map_err(api_error))
        });
        methods.add_method("take_back", |_, this, ()| with(|api, _| api.party_take_back(this.0).map_err(api_error)));
        methods.add_method("form_change_target", |_, this, ()| {
            let form = with(|api, _| Ok(api.form_change_target(this.0)))?;
            match form {
                Some(h) => Ok(LuaValue::Table(bound(|b| b.def_value(Registry::Form, h.0))?)),
                None => Ok(LuaValue::Nil),
            }
        });
        methods.add_method("ask_form_change", |_, this, form: LuaValue| {
            let form = nettai_content_api::FormHandle(bound(|b| def_arg(b, &form, Registry::Form, "ask_form_change"))?);
            with(|api, _| api.ask_form_change(this.0, form).map_err(api_error))
        });
        methods.add_method("form_change_terms", |_, this, ()| with(|api, _| Ok(api.form_change_terms(this.0))));
        methods.add_method("end_attack", |_, this, ()| with(|api, _| Ok(api.end_attack(this.0))));
        methods.add_method("set_attack", |_, this, (action, kind): (LuaValue, LuaValue)| {
            let kind = crate::choice::parsed(&kind, ChoiceKind::AttackKind, "attack kind", AttackKind::from_name)?.byte();
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
                nettai_content_api::NaviAction::Engine(name) => crate::choice::named(lua, ChoiceKind::EngineAction, name),
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
        methods.add_method("reset_attack_links", |_, this, ()| with(|api, _| Ok(api.reset_attack_links(this.0))));
        methods.add_method("held_direction", |lua, this, ()| {
            let dir = with(|api, _| Ok(api.held_direction(this.0)))?;
            StepDirection::of_byte(dir).map_or(Ok(LuaValue::Nil), |d| crate::choice::named(lua, ChoiceKind::StepDirection, d.name()))
        });
        methods.add_method("step_target", |_, this, dir: LuaValue| {
            let dir = crate::choice::parsed(&dir, ChoiceKind::StepDirection, "step direction", StepDirection::from_name)?.byte();
            let p = with(|api, _| Ok(api.step_target(this.0, dir)))?;
            Ok(p.map_or((None, None), |p| (Some(p.x), Some(p.y))))
        });
        methods.add_method("start_move", |_, this, dir: LuaValue| {
            let dir = crate::choice::parsed(&dir, ChoiceKind::StepDirection, "step direction", StepDirection::from_name)?.byte();
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
            let kind = crate::choice::parsed(&kind, ChoiceKind::AttackKind, "attack kind", AttackKind::from_name)?.byte();
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
            let kind = crate::choice::parsed(&kind, ChoiceKind::AttackKind, "attack kind", AttackKind::from_name)?.byte();
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
        methods.add_method("lock_on_panel", |_, this, ()| {
            let p = with(|api, _| Ok(api.lock_on_panel(this.0)))?;
            Ok(p.map_or((None, None), |p| (Some(p.x), Some(p.y))))
        });
        methods.add_method("face_toward", |_, this, target: LuaValue| {
            let target = object_arg(&target, "face_toward")?.ok_or_else(|| mlua::Error::runtime("face_toward: expected an Object"))?;
            with(|api, _| api.face_toward(this.0, target).map_err(api_error))
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
            let shown = sprite_shown(f);
            fields.add_field_method_get(f.name(), move |lua, this| {
                if shown == Shown::Parts {
                    return Ok(LuaValue::UserData(view(lua, View::HiddenParts(this.0, None))?));
                }
                let v = with(|api, _| Ok(api.sprite_get(this.0, f)))?;
                show(lua, v, f.field_type(), shown)
            });
            if f.writable() {
                fields.add_field_function_set(f.name(), move |_, this, v: LuaValue| {
                    let o = this.borrow::<Sprite>()?.0;
                    let v = take(v, f.field_type(), shown, f.name())?;
                    with(|api, _| api.sprite_set(o, f, v).map_err(api_error))
                });
            }
        }
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("load", |_, this, sprite: LuaValue| {
            let id = sprite_id(sprite)?;
            with(|api, _| Ok(api.sprite_load(this.0, id)))
        });
        methods.add_method("set_animation", |_, this, anim: LuaValue| {
            let anim = u8_arg(anim, "anim")?;
            with(|api, _| Ok(api.sprite_set_animation(this.0, anim)))
        });
        methods.add_method("step", |_, this, ()| with(|api, _| Ok(api.sprite_step(this.0))));
        methods.add_method("attach", |_, this, (owner, part): (LuaValue, LuaValue)| {
            let to = match object_arg(&owner, "sprite:attach's owner")? {
                Some(o) => Some((o, u8_arg(part, "part")?)),
                None => None,
            };
            with(|api, _| Ok(api.sprite_attach(this.0, to)))
        });
        // `sprite_setFlip` with the side whose way it faces: mirrored for
        // side 1's way, never upside down. `face(o)`: the way `o` faces (its
        // alliance ^ flip, as the game's routines pass it).
        methods.add_method("face_side", |_, this, side: LuaValue| {
            let side = u8_arg(side, "side")?;
            if side > 1 {
                return Err(mlua::Error::runtime(format!("sprite:face_side: a side is 0 or 1, not {side}")));
            }
            set_sprite_flip(this.0, side)
        });
        methods.add_method("face", |_, this, o: mlua::UserDataRef<Object>| {
            let flip = with(|api, _| match (api.get(o.0, ObjectField::Alliance), api.get(o.0, ObjectField::Flip)) {
                (Ok(Value::Int(side)), Ok(Value::Int(flip))) => Ok((side ^ flip) as u8),
                (Err(e), _) | (_, Err(e)) => Err(api_error(e)),
                _ => unreachable!("alliance and flip are bytes"),
            })?;
            set_sprite_flip(this.0, flip)
        });
        // The hidden parts as the engine keeps them for content: the word an
        // absorbed obstacle's state `hidden_parts` holds, which the engine
        // fills from the absorbed object's sprite (`fly_to_absorber`).
        methods.add_method("hide_saved_parts", |_, this, saved: LuaValue| {
            let w = u32::try_from(int(&saved, "saved parts")?).map_err(|_| mlua::Error::runtime("sprite:hide_saved_parts: expected a state's saved parts"))?;
            with(|api, _| api.sprite_set(this.0, SpriteField::HiddenParts, Value::Int(i64::from(w))).map_err(api_error))
        });
        // Its fields as a console draws them.
        methods.add_method("console", |lua, this, side: LuaValue| view(lua, View::SpriteConsole(this.0, side_arg(side)?)));
    }
}

/// The header flag byte's bits, by name (`set_header_flags`): those the
/// object's flag fields are, and the top two, which nothing reads.
const HEADER_FLAGS: [(&str, u8); 8] = [
    ("active", 0x01),
    ("visible", 0x02),
    ("run_while_paused", 0x04),
    ("no_sprite_update", 0x08),
    ("run_while_dimmed", 0x10),
    ("holds_reservation", 0x20),
    ("unused_low", 0x40),
    ("unused_high", 0x80),
];

/// The side whose way an effect faces (the game's flip value's bit 0).
fn facing_arg(v: LuaValue, what: &str) -> mlua::Result<u8> {
    let side = u8_arg(v, "facing")?;
    if side > 1 {
        return Err(mlua::Error::runtime(format!("{what}: it faces a side's way, 0 or 1, not {side}")));
    }
    Ok(side)
}

/// How a sprite field is shown.
fn sprite_shown(f: SpriteField) -> Shown {
    if f == SpriteField::HiddenParts { Shown::Parts } else { Shown::of(f.field_type()) }
}

/// `sprite_setFlip` with the game's flip value (bit 0 horizontal, bit 1
/// vertical), every console's.
fn set_sprite_flip(o: ObjectRef, flip: u8) -> mlua::Result<()> {
    with(|api, _| {
        api.sprite_set(o, SpriteField::HFlip, Value::Bool(flip & 1 != 0)).map_err(api_error)?;
        api.sprite_set(o, SpriteField::VFlip, Value::Bool(flip & 2 != 0)).map_err(api_error)
    })
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
        fields.add_field_method_get("hit_by", |lua, this| view(lua, View::HitBy(this.0)));
        for &f in CollisionField::ALL {
            // (The element is its two bytes as one `Element`, below; the
            // guard's directions are `guarded_from`.)
            if matches!(f, CollisionField::Element | CollisionField::SecondaryElement | CollisionField::GuardDirs) {
                continue;
            }
            let shown = if f == CollisionField::DamageElements { Shown::Secondary } else { Shown::of(f.field_type()) };
            fields.add_field_method_get(f.name(), move |lua, this| {
                let v = with(|api, _| api.collision_get(this.0, f).map_err(api_error))?;
                show(lua, v, f.field_type(), shown)
            });
            if f.writable() {
                fields.add_field_function_set(f.name(), move |_, this, v: LuaValue| {
                    let o = this.borrow::<Collision>()?.0;
                    // (A hit modifier by its bits' names, or a byte copied whole.)
                    if f == CollisionField::HitModBase {
                        return with(|api, _| {
                            let bits = hit_modifier_arg(api, &v, f.name())?;
                            api.collision_set(o, f, Value::Int(bits as i64)).map_err(api_error)
                        });
                    }
                    let v = take(v, f.field_type(), shown, f.name())?;
                    with(|api, _| api.collision_set(o, f, v).map_err(api_error))
                });
            }
        }
        // The element its hits carry: the primary element (CollisionData+0x02)
        // and the secondary elements (+0x19) as one value, as setup and
        // `sub_8019F8C` take them from an element byte (the low four bits and
        // the high four).
        fields.add_field_method_get("element", |lua, this| {
            let (p, s) = with(|api, _| {
                let p = api.collision_get(this.0, CollisionField::Element).map_err(api_error)?;
                let s = api.collision_get(this.0, CollisionField::SecondaryElement).map_err(api_error)?;
                Ok((p, s))
            })?;
            match (p, s) {
                (Value::Int(p), Value::Int(s)) => Ok(crate::element::value(lua, (p | s) as u8)?),
                _ => unreachable!("the element bytes are bytes"),
            }
        });
        fields.add_field_function_set("element", |_, this, v: LuaValue| {
            let o = this.borrow::<Collision>()?.0;
            let e = crate::element::arg(&v, "element")?;
            with(|api, _| {
                api.collision_set(o, CollisionField::Element, Value::Int(i64::from(e & 0xF))).map_err(api_error)?;
                api.collision_set(o, CollisionField::SecondaryElement, Value::Int(i64::from(e & 0xF0))).map_err(api_error)
            })
        });
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
            let element = crate::element::primary_arg(&element, "element_damage")?;
            with(|api, _| api.collision_element_damage(this.0, element).map_err(api_error))
        });
        // A guard blocked a hit this window from a hitter turned so (its
        // `flip`): the guard's directions, bit 1 << the hitter's flip.
        methods.add_method("guarded_from", |_, this, turned: bool| {
            let dirs = with(|api, _| api.collision_get(this.0, CollisionField::GuardDirs).map_err(api_error))?;
            let Value::Int(dirs) = dirs else { unreachable!("a byte") };
            Ok(dirs & (1 << u8::from(turned)) != 0)
        });
        methods.add_method("hitters", |lua, this, ()| {
            let hitters = with(|api, _| api.collision_hit_by(this.0).map_err(api_error))?;
            object_list(lua, hitters)
        });
        // The hit results have any of the set's members' bits (side-relative
        // ones by the object's side).
        methods.add_method("hit_any", |lua, this, set: LuaValue| {
            let bits = Names::hits(lua, &set)?;
            with(|api, _| api.collision_hit_any(this.0, bits).map_err(api_error))
        });
        methods.add_method("add_damage", |_, this, (element, amount, raw): (LuaValue, LuaValue, bool)| {
            let (element, amount) = (crate::element::primary_arg(&element, "add_damage")?, u16_arg(amount, "amount")?);
            with(|api, _| Ok(api.add_damage(this.0, element, amount, raw)))
        });
        methods.add_method("add_raw_damage", |_, this, (element, amount): (LuaValue, LuaValue)| {
            let (element, amount) = (crate::element::primary_arg(&element, "add_raw_damage")?, u16_arg(amount, "amount")?);
            with(|api, _| Ok(api.add_raw_damage(this.0, element, amount)))
        });
        methods.add_method("add_mood_damage", |_, this, amount: LuaValue| {
            let amount = u16_arg(amount, "amount")?;
            with(|api, _| Ok(api.add_mood_damage(this.0, amount)))
        });
        methods.add_method("add_hit_mod", |_, this, hit_mod: LuaValue| {
            with(|api, _| {
                let bits = hit_modifier_arg(api, &hit_mod, "add_hit_mod")?;
                Ok(api.add_hit_mod(this.0, bits))
            })
        });
        methods.add_method("add_final_hit_mod", |_, this, hit_mod: LuaValue| {
            with(|api, _| {
                let bits = hit_modifier_arg(api, &hit_mod, "add_final_hit_mod")?;
                Ok(api.add_final_hit_mod(this.0, bits))
            })
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
    /// names `battle.navi(side)` has beside the engine's), or a member's
    /// block's.
    Stats(nettai_content_api::StatsOf),
}

impl State {
    /// Run `f` on the state's fields (`write`: to store into them). `key`
    /// names the field a write to a player's setup is refused for (made only
    /// then).
    fn with_fields<R>(
        self,
        write: bool,
        key: impl FnOnce(&Bound) -> mlua::Result<String>,
        f: impl FnOnce(&mut dyn nettai_content_api::Fields, &Bound) -> mlua::Result<R>,
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
                        "setup field `{}`: a player's setup is read-only in battle",
                        key(bound)?
                    )));
                }
                StateOf::Setup(c) => {
                    setup = api.rules_setup(c.side).map_err(api_error)?.clone();
                    &mut setup
                }
            };
            f(s, bound)
        })
    }

    /// Run `f` on the state's field `key` (by its index in the state's
    /// layout). (A key that is no UTF-8 is refused as `LuaString::to_str`
    /// refuses it, before anything else: no field has its name.)
    fn with_state<R>(
        self,
        key: &Name,
        write: bool,
        f: impl FnOnce(&mut dyn nettai_content_api::Fields, &nettai_content_api::Schema, usize) -> mlua::Result<R>,
    ) -> mlua::Result<R> {
        self.with_fields(write, |_| key.to_str().map(str::to_string), |s, bound| {
            let schema = bound.manifest.schema(s.id());
            let i = match field_named(schema, key.as_bytes()) {
                Some(i) => i,
                None => return Err(field_missing(schema, key.to_str()?)),
            };
            f(s, schema, i)
        })
    }
}

/// Field `name` of a layout or a record, as `Schema::index_of` finds it.
fn field_named(schema: &nettai_content_api::Schema, n: &[u8]) -> Option<usize> {
    let (Some(&first), Some(&last)) = (n.first(), n.last()) else {
        return schema.fields().iter().position(|f| f.name.is_empty());
    };
    // (Names of the length with the same first and last letters compared
    // whole: a comparison of each would cost a call.)
    schema.fields().iter().position(|f| {
        let b = f.name.as_bytes();
        b.len() == n.len() && b[0] == first && b[b.len() - 1] == last && b == n
    })
}

/// Field `key` of a state's layout.
fn field_index(schema: &nettai_content_api::Schema, key: &str) -> mlua::Result<usize> {
    field_named(schema, key.as_bytes()).ok_or_else(|| field_missing(schema, key))
}

fn field_missing(schema: &nettai_content_api::Schema, key: &str) -> mlua::Error {
    let names: Vec<&str> = schema.fields().iter().map(|f| f.name.as_str()).collect();
    mlua::Error::runtime(format!("state has no field `{key}` (it declares {})", names.join(", ")))
}

impl UserData for State {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::Index, |lua, this, key: Name| {
            // (A scalar's value; else the part the field is, in the state's
            // layout.)
            let got = this.with_state(&key, false, |s, schema, i| {
                let ty = &schema.field(i).ty;
                if ty.is_scalar() { from_api(lua, s.get(schema, i).load(), ty).map(Ok) } else { Ok(Err((s.id(), i))) }
            })?;
            match got {
                Ok(v) => Ok(v),
                Err((id, field)) => {
                    Ok(LuaValue::UserData(lua.create_userdata(StatePart { state: State(this.0), id, field, path: Vec::new() })?))
                }
            }
        });
        methods.add_meta_method(MetaMethod::NewIndex, |_, this, (key, v): (Name, LuaValue)| {
            this.with_state(&key, true, |s, schema, i| assign(s, schema.place(i), v, &|| format!("state field `{}`", key.lossy())))
        });
    }
}

/// Store `v` at `place` of state `s`, as the place's type takes it: a
/// value; a record from a table of its fields (those it leaves out zero);
/// an array or a list from a table of its elements, from the first (a
/// list's length the table's, an array's rest zero). `name` says the place
/// in a message.
fn assign(s: &mut dyn nettai_content_api::Fields, place: nettai_content_api::Place, v: LuaValue, name: What) -> mlua::Result<()> {
    let ty = place.ty();
    if ty.is_scalar() {
        let v = to_api_as(v, ty, name)?;
        return s.set_at(place, v).map_err(|e| mlua::Error::runtime(format!("{}: {e}", name())));
    }
    let LuaValue::Table(t) = v else {
        return Err(mlua::Error::runtime(format!("{} is a {ty}: give it a table, not a {}", name(), v.type_name())));
    };
    match ty {
        FieldType::Record(fields) => {
            for pair in t.clone().pairs::<LuaValue, LuaValue>() {
                let (k, _) = pair?;
                let known = match &k {
                    LuaValue::String(k) => field_named(fields, k.to_str()?.as_bytes()).is_some(),
                    _ => false,
                };
                if !known {
                    let names: Vec<&str> = fields.fields().iter().map(|f| f.name.as_str()).collect();
                    return Err(mlua::Error::runtime(format!("{} has no field {k:?} ({})", name(), names.join(", "))));
                }
            }
            s.clear_at(place);
            for (i, f) in fields.fields().iter().enumerate() {
                let x: LuaValue = t.raw_get(f.name.as_str())?;
                if !x.is_nil() {
                    assign(s, place.field_at(i), x, &|| format!("{}.{}", name(), f.name))?;
                }
            }
            Ok(())
        }
        FieldType::Array(..) | FieldType::List(..) => {
            let n = t.raw_len();
            let room = place.capacity().expect("an array or a list");
            if n > room {
                return Err(mlua::Error::runtime(format!("{} holds {room}, not {n}", name())));
            }
            s.clear_at(place);
            if matches!(ty, FieldType::List(..)) {
                s.set_len_at(place, n).map_err(mlua::Error::runtime)?;
            }
            for k in 0..n {
                let x: LuaValue = t.raw_get(k + 1)?;
                assign(s, place.elem(k).expect("within its room"), x, &|| format!("{}[{}]", name(), k + 1))?;
            }
            Ok(())
        }
        _ => unreachable!("a scalar"),
    }
}

/// A step into a part of a state field: a record's field (by its index in
/// the record), an element.
#[derive(Clone, Copy, Debug)]
enum Step {
    Field(usize),
    Elem(usize),
}

/// The part `step` names of `place` (none: no such part).
fn place_step(place: nettai_content_api::Place<'_>, step: Step) -> Option<nettai_content_api::Place<'_>> {
    match step {
        Step::Field(i) => place.record().filter(|r| i < r.fields().len()).map(|_| place.field_at(i)),
        Step::Elem(k) => place.elem(k),
    }
}

/// What `step` adds to a part's name, from `place`: `.code`, `[2]`.
fn step_name(place: nettai_content_api::Place<'_>, step: Step) -> String {
    match step {
        Step::Field(i) => format!(".{}", place.record().and_then(|r| r.fields().get(i)).map_or("?", |f| f.name.as_str())),
        Step::Elem(k) => format!("[{}]", k + 1),
    }
}

/// A part of a content state field that holds parts: an array's elements
/// (`s.targets[i]`), a list's (`s.folder[i]`, `#s.folder`), a record's
/// fields (`s.folder[i].code`), as deep as the schema nests them. Indexes
/// are 1-based like Luau arrays. A list grows by its next element
/// (`s.list[#s.list + 1] = v`); a record, a list or an array takes a table
/// whole (`s.folder[2] = { chip = c, code = "A" }`, [`assign`]).
///
/// A part holds its field and its steps by index, in the layout of the
/// state it was made from (`id`); a state that has another layout by the
/// time the part is used (its object's slot taken by another kind, its navi
/// running another action) is reached by their names, as written.
pub struct StatePart {
    state: State,
    id: StateId,
    field: usize,
    path: Vec<Step>,
}

/// A part's place as the state's layout has it: the layout, its field and
/// its steps (what a part of the part takes on).
struct Resolved<'a> {
    id: StateId,
    field: usize,
    path: &'a [Step],
}

impl StatePart {
    /// The names of the part's field and of each of its steps, in the
    /// layout it was made in: `folder`, `[2]`, `.code`.
    fn names(&self, bound: &Bound) -> (String, Vec<String>) {
        let schema = bound.manifest.schema(self.id);
        let mut place = Some(schema.place(self.field));
        let mut steps = Vec::with_capacity(self.path.len());
        for &step in &self.path {
            steps.push(place.map_or_else(|| "?".to_string(), |p| step_name(p, step)));
            place = place.and_then(|p| place_step(p, step));
        }
        (schema.field(self.field).name.clone(), steps)
    }

    /// What it is called in a message: `folder[2].code`; then `next`'s
    /// step, from the place it is a step of.
    fn name(&self, bound: &Bound, next: Option<(nettai_content_api::Place, Step)>) -> String {
        let (mut out, steps) = self.names(bound);
        out.extend(steps);
        if let Some((place, step)) = next {
            out.push_str(&step_name(place, step));
        }
        out
    }

    /// Step `n` of its path leads to no part of the state.
    fn no_part(&self, bound: &Bound, n: usize) -> mlua::Error {
        let (mut out, steps) = self.names(bound);
        let again = steps.get(n).cloned().unwrap_or_default();
        out.extend(steps);
        out.push_str(&again);
        mlua::Error::runtime(format!("{out}: no such part"))
    }

    /// Run `f` on the state with this part's place.
    fn with_place<R>(
        &self,
        write: bool,
        f: impl FnOnce(&mut dyn nettai_content_api::Fields, nettai_content_api::Place, &Bound, Resolved) -> mlua::Result<R>,
    ) -> mlua::Result<R> {
        let field_name = |bound: &Bound| Ok(bound.manifest.schema(self.id).field(self.field).name.clone());
        self.state.with_fields(write, field_name, |s, bound| {
            let id = s.id();
            let schema = bound.manifest.schema(id);
            if id == self.id {
                let mut place = schema.place(self.field);
                for (n, &step) in self.path.iter().enumerate() {
                    place = place_step(place, step).ok_or_else(|| self.no_part(bound, n))?;
                }
                return f(s, place, bound, Resolved { id, field: self.field, path: &self.path });
            }
            // (The state has another layout: the part's names, in it.)
            let made = bound.manifest.schema(self.id);
            let field = field_index(schema, &made.field(self.field).name)?;
            let (mut was, mut place) = (made.place(self.field), schema.place(field));
            let mut path = Vec::with_capacity(self.path.len());
            for (n, &step) in self.path.iter().enumerate() {
                let here = match step {
                    Step::Field(i) => {
                        let name = &was.record().expect("a part's steps are of its layout").field(i).name;
                        place.record().and_then(|r| field_named(r, name.as_bytes())).map(Step::Field)
                    }
                    Step::Elem(k) => Some(Step::Elem(k)),
                };
                let (Some(here), Some(next)) = (here, here.and_then(|h| place_step(place, h))) else {
                    return Err(self.no_part(bound, n));
                };
                place = next;
                path.push(here);
                was = place_step(was, step).expect("a part's steps are of its layout");
            }
            f(s, place, bound, Resolved { id, field, path: &path })
        })
    }

    /// The step `key` names into this part (a record's field by name, an
    /// element by its 1-based index), and whether the element is one the
    /// list may take next (one past its end, for a write).
    fn step(&self, bound: &Bound, key: &PartKey, s: &dyn nettai_content_api::Fields, place: nettai_content_api::Place, write: bool) -> mlua::Result<Step> {
        match place.ty() {
            FieldType::Record(fields) => {
                let PartKey::Name(name) = key else {
                    return Err(mlua::Error::runtime(format!("{}: a record's fields have names", self.name(bound, None))));
                };
                match field_named(fields, name.as_bytes()) {
                    Some(i) => Ok(Step::Field(i)),
                    None => {
                        let name = name.to_str()?;
                        let names: Vec<&str> = fields.fields().iter().map(|f| f.name.as_str()).collect();
                        Err(mlua::Error::runtime(format!("{} has no field `{name}` ({})", self.name(bound, None), names.join(", "))))
                    }
                }
            }
            FieldType::Array(..) | FieldType::List(..) => {
                let k = match key {
                    PartKey::Other(v) => int_as(v, &|| self.name(bound, None))?,
                    // (As `int_as` refuses a string.)
                    PartKey::Name(_) => {
                        return Err(mlua::Error::runtime(format!("{}: expected an integer, got string", self.name(bound, None))));
                    }
                };
                let len = s.len_at(place).expect("an array or a list") as i64;
                let room = place.capacity().expect("an array or a list") as i64;
                let last = if write && matches!(place.ty(), FieldType::List(..)) { (len + 1).min(room) } else { len };
                if k < 1 || k > last {
                    let what = if matches!(place.ty(), FieldType::List(..)) { "list" } else { "array" };
                    return Err(mlua::Error::runtime(format!("{}[{k}] is past the {what}'s end ({len})", self.name(bound, None))));
                }
                Ok(Step::Elem(k as usize - 1))
            }
            ty => Err(mlua::Error::runtime(format!("{}: a {ty} has no parts", self.name(bound, None)))),
        }
    }
}

impl StatePart {
    /// `this[key]`: a scalar's value, or the part a record's field or a
    /// list's element is.
    fn index(lua: &Lua, this: &StatePart, key: PartKey) -> mlua::Result<LuaValue> {
        let got = this.with_place(false, |s, place, bound, at| {
            let step = this.step(bound, &key, s, place, false)?;
            let part = place_step(place, step).expect("a step into the part");
            let ty = part.ty();
            if ty.is_scalar() {
                return from_api(lua, s.get_at(part).load(), ty).map(Ok);
            }
            let mut path = Vec::with_capacity(at.path.len() + 1);
            path.extend_from_slice(at.path);
            path.push(step);
            Ok(Err(StatePart { state: State(this.state.0), id: at.id, field: at.field, path }))
        })?;
        match got {
            Ok(v) => Ok(v),
            Err(part) => Ok(LuaValue::UserData(lua.create_userdata(part)?)),
        }
    }

    /// How many elements the list or array this part is holds.
    fn len(&self) -> mlua::Result<usize> {
        self.with_place(false, |s, place, bound, _| {
            s.len_at(place).ok_or_else(|| mlua::Error::runtime(format!("{}: a record has no length", self.name(bound, None))))
        })
    }
}

impl UserData for StatePart {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::Index, |lua, this, key: PartKey| StatePart::index(lua, this, key));
        // `for i, x in list`: a list's or an array's elements from the
        // first, as a table's.
        methods.add_meta_method(MetaMethod::Iter, |lua, this, ()| {
            this.len()?;
            let next = lua.create_function(|lua, (part, i): (AnyUserData, mlua::Integer)| {
                let this = part.borrow::<StatePart>()?;
                if i < 0 || i as usize >= this.len()? {
                    return Ok((LuaValue::Nil, LuaValue::Nil));
                }
                Ok((LuaValue::Integer(i + 1), StatePart::index(lua, &this, PartKey::Other(LuaValue::Integer(i + 1)))?))
            })?;
            let me = lua.create_userdata(StatePart { state: State(this.state.0), id: this.id, field: this.field, path: this.path.clone() })?;
            Ok((next, me, 0i64))
        });
        methods.add_meta_method(MetaMethod::NewIndex, |_, this, (key, v): (PartKey, LuaValue)| {
            this.with_place(true, |s, place, bound, _| {
                let step = this.step(bound, &key, s, place, true)?;
                if let (Step::Elem(k), FieldType::List(..)) = (step, place.ty())
                    && Some(k) == s.len_at(place)
                {
                    s.set_len_at(place, k + 1).map_err(mlua::Error::runtime)?;
                }
                let at = place_step(place, step).expect("a step into the part");
                assign(s, at, v, &|| this.name(bound, Some((place, step))))
            })
        });
        methods.add_meta_method(MetaMethod::Len, |_, this, ()| this.len().map(|n| n as i64));
    }
}

/// A side's navi stats (`battle.navi(side)`), or a member's block of its
/// party (`battle.member(side, m)`): the engine's by their names, and its
/// game's own (the rules' `stats`) by theirs, as a state's fields.
#[derive(Clone, Copy)]
pub struct Navi(nettai_content_api::StatsOf);

impl UserData for Navi {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        for &f in NaviStat::ALL {
            let shown = match f {
                NaviStat::Element => Shown::Element,
                NaviStat::Mood => Shown::Mood,
                NaviStat::Folder1Regular
                | NaviStat::Folder2Regular
                | NaviStat::Folder1TagA
                | NaviStat::Folder1TagB
                | NaviStat::Folder2TagA
                | NaviStat::Folder2TagB => Shown::NoneAt0xFF,
                _ => Shown::of(f.field_type()),
            };
            fields.add_field_method_get(f.name(), move |lua, this| {
                let v = with(|api, _| Ok(api.navi_stat(this.0, f)))?;
                show(lua, v, f.field_type(), shown)
            });
            if f.writable() {
                fields.add_field_function_set(f.name(), move |_, this, v: LuaValue| {
                    let of = this.borrow::<Navi>()?.0;
                    let v = take(v, f.field_type(), shown, f.name())?;
                    with(|api, _| api.set_navi_stat(of, f, v).map_err(api_error))
                });
            }
        }
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // (A name the engine's stats haven't: the game's own.)
        methods.add_meta_method(MetaMethod::Index, |lua, this, key: Name| {
            State(StateOf::Stats(this.0)).with_state(&key, false, |s, schema, i| from_api(lua, s.get(schema, i).load(), &schema.field(i).ty))
        });
        methods.add_meta_method(MetaMethod::NewIndex, |_, this, (key, v): (Name, LuaValue)| {
            State(StateOf::Stats(this.0)).with_state(&key, true, |s, schema, i| assign(s, schema.place(i), v, &|| format!("stat `{}`", key.lossy())))
        });
    }
}

// ---- Views ----------------------------------------------------------------------------------

/// A view of named members: what it reads (an object's status flags, a
/// side's HUD...), each member a property (`me.status.guard = true`,
/// `battle.hud(side).gauge`). Made once and given again ([`Handles`]), so
/// reading a member allocates nothing; an unknown member is an error.
#[derive(Clone, Copy)]
enum View {
    Statuses(ObjectRef),
    StatusTimers(ObjectRef),
    Requests(ObjectRef),
    NaviFlags(ObjectRef),
    Buttons(ObjectRef, Pad),
    ObstacleFlags(ObjectRef),
    FormTraits(ObjectRef),
    ObjectConsole(ObjectRef, u8),
    SpriteConsole(ObjectRef, u8),
    HiddenParts(ObjectRef, Option<u8>),
    HitBy(ObjectRef),
    Hud(Option<u8>),
    HandEntry(u8, u8),
    Listening(u8),
}

fn view(lua: &Lua, v: View) -> mlua::Result<AnyUserData> {
    Handles::get(lua, Handle::View(v))
}

/// `me.status`: an object's status flags (its collision data's).
#[derive(Clone, Copy)]
struct Statuses(ObjectRef);

impl UserData for Statuses {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        for &f in StatusFlag::ALL {
            fields.add_field_method_get(f.name(), move |_, this| with(|api, _| api.status(this.0, f).map_err(api_error)));
            fields.add_field_function_set(f.name(), move |_, this, on: bool| {
                let o = this.borrow::<Statuses>()?.0;
                with(|api, _| api.set_status(o, f, on).map_err(api_error))
            });
        }
    }
}

/// `me.status_timers`: an object's status timers.
#[derive(Clone, Copy)]
struct StatusTimers(ObjectRef);

impl UserData for StatusTimers {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        for &t in StatusTimer::ALL {
            fields.add_field_method_get(t.name(), move |_, this| with(|api, _| api.status_timer(this.0, t).map_err(api_error)));
            fields.add_field_function_set(t.name(), move |_, this, v: LuaValue| {
                let o = this.borrow::<StatusTimers>()?.0;
                let v = u16_arg(v, "status timer")?;
                with(|api, _| api.set_status_timer(o, t, v).map_err(api_error))
            });
        }
    }
}

/// `me.requests`: a navi's requests.
#[derive(Clone, Copy)]
struct Requests(ObjectRef);

impl UserData for Requests {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        for &f in RequestFlag::ALL {
            fields.add_field_method_get(f.name(), move |_, this| with(|api, _| api.request(this.0, f).map_err(api_error)));
            fields.add_field_function_set(f.name(), move |_, this, on: bool| {
                let o = this.borrow::<Requests>()?.0;
                with(|api, _| api.set_request(o, f, on).map_err(api_error))
            });
        }
    }
}

/// `me.navi_flags`: a navi's state flags.
#[derive(Clone, Copy)]
struct NaviFlags(ObjectRef);

impl UserData for NaviFlags {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        for &f in NaviState::ALL {
            fields.add_field_method_get(f.name(), move |_, this| with(|api, _| api.navi_state(this.0, f).map_err(api_error)));
            fields.add_field_function_set(f.name(), move |_, this, on: bool| {
                let o = this.borrow::<NaviFlags>()?.0;
                with(|api, _| api.set_navi_state(o, f, on).map_err(api_error))
            });
        }
    }
}

/// `me.held`, `me.pressed`, `me.dimmed_held`, `me.dimmed_pressed`: a navi's
/// buttons in one of its records (read-only).
#[derive(Clone, Copy)]
struct Buttons(ObjectRef, Pad);

impl UserData for Buttons {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        for &key in Key::ALL {
            fields.add_field_method_get(key.name(), move |_, this| with(|api, _| api.key(this.0, this.1, key).map_err(api_error)));
        }
    }
}

/// `me.obstacle_flags`: what other objects asked of a field object
/// (read-only).
#[derive(Clone, Copy)]
struct ObstacleFlags(ObjectRef);

impl UserData for ObstacleFlags {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        for &f in nettai_content_api::api::ObstacleFlag::ALL {
            fields.add_field_method_get(f.name(), move |_, this| with(|api, _| api.obstacle_flag(this.0, f).map_err(api_error)));
        }
    }
}

/// `me.form_traits`: the traits of the form of a navi's side (read-only;
/// the engine knows their names).
#[derive(Clone, Copy)]
struct FormTraits(ObjectRef);

impl UserData for FormTraits {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::Index, |_, this, name: Name| {
            let name = name.to_str()?;
            with(|api, _| api.form_trait(this.0, name).map_err(api_error))
        });
        methods.add_meta_method(MetaMethod::NewIndex, |_, _, (name, _): (Name, LuaValue)| -> mlua::Result<()> {
            Err(mlua::Error::runtime(format!("form_traits.{}: a form's traits are its definition's (read-only)", name.lossy())))
        });
    }
}

/// `o:console(side)`: an object's fields as side `side`'s console has them:
/// `visible`, which the consoles each decide.
#[derive(Clone, Copy)]
struct ObjectConsole(ObjectRef, u8);

impl UserData for ObjectConsole {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        let f = ObjectField::Visible;
        fields.add_field_method_get(f.name(), move |lua, this| {
            let v = with(|api, _| api.get_for(this.0, f, this.1).map_err(api_error))?;
            show(lua, v, f.field_type(), Shown::of(f.field_type()))
        });
        fields.add_field_function_set(f.name(), move |_, this, v: LuaValue| {
            let ObjectConsole(o, side) = *this.borrow::<ObjectConsole>()?;
            let v = take(v, f.field_type(), Shown::of(f.field_type()), f.name())?;
            with(|api, _| api.set_for(o, f, side, v).map_err(api_error))
        });
    }
}

/// The sprite fields the consoles draw each their own way (where the
/// original's consoles decide a look by their own side).
const CONSOLE_LOOKS: [SpriteField; 6] =
    [SpriteField::Palette, SpriteField::HFlip, SpriteField::ColorShader, SpriteField::Alpha, SpriteField::Mosaic, SpriteField::HiddenParts];

/// `sprite:console(side)`: a sprite's fields as side `side`'s console draws
/// them: each its own way for [`CONSOLE_LOOKS`], which are set there alone;
/// the others alike (read as the sprite's).
#[derive(Clone, Copy)]
struct SpriteConsole(ObjectRef, u8);

impl UserData for SpriteConsole {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        for &f in SpriteField::ALL {
            fields.add_field_method_get(f.name(), move |lua, this| {
                if sprite_shown(f) == Shown::Parts {
                    return Ok(LuaValue::UserData(view(lua, View::HiddenParts(this.0, Some(this.1)))?));
                }
                let v = with(|api, _| Ok(api.sprite_get_for(this.0, f, this.1)))?;
                show(lua, v, f.field_type(), sprite_shown(f))
            });
        }
        for f in CONSOLE_LOOKS {
            fields.add_field_function_set(f.name(), move |_, this, v: LuaValue| {
                let SpriteConsole(o, side) = *this.borrow::<SpriteConsole>()?;
                let v = take(v, f.field_type(), sprite_shown(f), f.name())?;
                with(|api, _| api.sprite_set_for(o, f, side, v).map_err(api_error))
            });
        }
    }
}

/// `sprite.hidden_parts`: a sprite's parts, each hidden or not by its index
/// from 0 (`sprite.hidden_parts[6] = true`); `sprite:console(side).hidden_parts`,
/// a console's.
#[derive(Clone, Copy)]
struct HiddenParts(ObjectRef, Option<u8>);

impl HiddenParts {
    fn word(self) -> mlua::Result<u32> {
        let v = with(|api, _| {
            Ok(match self.1 {
                None => api.sprite_get(self.0, SpriteField::HiddenParts),
                Some(side) => api.sprite_get_for(self.0, SpriteField::HiddenParts, side),
            })
        })?;
        let Value::Int(w) = v else { unreachable!("the hidden parts are a word") };
        Ok(w as u32)
    }
}

impl UserData for HiddenParts {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::Index, |_, this, part: LuaValue| {
            let part = part_index(&part, "sprite.hidden_parts")?;
            Ok(this.word()? & (0x8000_0000 >> part) != 0)
        });
        methods.add_meta_method(MetaMethod::NewIndex, |_, this, (part, hidden): (LuaValue, LuaValue)| {
            let part = part_index(&part, "sprite.hidden_parts")?;
            let LuaValue::Boolean(hidden) = hidden else {
                return Err(mlua::Error::runtime(format!("sprite.hidden_parts[{part}] is true or false, not {}", hidden.type_name())));
            };
            let bit = 0x8000_0000u32 >> part;
            let w = this.word()?;
            let w = if hidden { w | bit } else { w & !bit };
            let v = Value::Int(i64::from(w));
            with(|api, _| match this.1 {
                None => api.sprite_set(this.0, SpriteField::HiddenParts, v).map_err(api_error),
                Some(side) => api.sprite_set_for(this.0, SpriteField::HiddenParts, side, v).map_err(api_error),
            })
        });
    }
}

/// `battle.hud(side)`: what the HUD shows, a side's; `battle.hud()`, every
/// side's (setting a field sets each side's; the fields every console shows
/// alike are read and set so alone).
#[derive(Clone, Copy)]
struct Hud(Option<u8>);

impl UserData for Hud {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        for &f in HudField::ALL {
            fields.add_field_method_get(f.name(), move |lua, this| {
                let side = match this.0 {
                    Some(side) => side,
                    // (Every console shows it alike: any side's.)
                    None if f.shared() => 0,
                    None => {
                        return Err(mlua::Error::runtime(format!(
                            "battle.hud().{}: each side has its own; read it with a side",
                            f.name()
                        )));
                    }
                };
                let v = with(|api, _| Ok(api.hud(f, side)))?;
                show(lua, v, f.field_type(), Shown::of(f.field_type()))
            });
            fields.add_field_function_set(f.name(), move |_, this, v: LuaValue| {
                let side = this.borrow::<Hud>()?.0;
                let v = take(v, f.field_type(), Shown::of(f.field_type()), f.name())?;
                with(|api, _| api.set_hud(f, side, v).map_err(api_error))
            });
        }
    }
}

/// `battle.hand_entry(side, i)`: an entry of a side's hand (from 0, used
/// ones too): each field nil past its entries, and setting one there an
/// error.
#[derive(Clone, Copy)]
struct HandEntry(u8, u8);

impl UserData for HandEntry {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        for &f in HandField::ALL {
            fields.add_field_method_get(f.name(), move |lua, this| {
                let v = with(|api, _| Ok(api.hand_entry(this.0, this.1, f)))?;
                // (Its marks by name, a view: `NamedBits::Mark`.)
                if let (HandField::Marks, Value::Int(_)) = (f, &v) {
                    let HandEntry(side, i) = *this;
                    return Ok(LuaValue::UserData(lua.create_userdata(HandMarks(side, i))?));
                }
                show(lua, v, f.field_type(), Shown::of(f.field_type()))
            });
            fields.add_field_function_set(f.name(), move |_, this, v: LuaValue| {
                let HandEntry(side, i) = *this.borrow::<HandEntry>()?;
                if f == HandField::Marks {
                    return with(|api, _| {
                        let bits = named_bits(api, NamedBits::Mark, &v, "marks")?;
                        api.set_hand_entry(side, i, f, Value::Int(bits as i64)).map_err(api_error)
                    });
                }
                let v = take(v, f.field_type(), Shown::of(f.field_type()), f.name())?;
                with(|api, _| api.set_hand_entry(side, i, f, v).map_err(api_error))
            });
        }
    }
}

/// `rules.listening(side)`: whether the engine calls each hook of side
/// `side`'s rules (the view only the rules' own calls are given: its
/// members' accessors run from mlua's own chunk, which `rules_only` can't
/// tell from the caller's). The custom screen's hooks
/// (`custom.*`, `window.*`, `button.*`: no identifier names them) are
/// always called.
#[derive(Clone, Copy)]
struct Listening(u8);

impl UserData for Listening {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        for hook in RulesHook::ALL.into_iter().filter(|h| !h.name().contains('.')) {
            fields.add_field_method_get(hook.name(), move |_, this| with(|api, _| api.listening(this.0, hook).map_err(api_error)));
            fields.add_field_function_set(hook.name(), move |_, this, on: bool| {
                let side = this.borrow::<Listening>()?.0;
                with(|api, _| api.listen(side, hook, on).map_err(api_error))
            });
        }
    }
}

/// `c.hit_by`: what an object's collision's last resolution's hit results
/// hold, each a property (read-only): the collision types of what it hit
/// and what hit it, by their members (side-relative ones by the object's
/// side), and the hit bits (`guarded`, `countered`, `guard_spark`).
#[derive(Clone, Copy)]
struct HitBy(ObjectRef);

impl UserData for HitBy {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        let names = nettai_content_api::CollisionMember::ALL.iter().map(|m| m.name()).chain(nettai_content_api::HitBit::ALL.iter().map(|b| b.name()));
        for name in names {
            fields.add_field_method_get(name, move |_, this| {
                with(|api, _| {
                    let side = |s: u8| api.hit_bits(name, Some(s)).map_err(mlua::Error::runtime);
                    api.collision_hit_any(this.0, [side(0)?, side(1)?]).map_err(api_error)
                })
            });
        }
    }
}

/// `battle.effects`: the battle's effects its game's rules declare of their
/// own (their `flow.effect_bits`), each a property by its name (read-only;
/// another name is an error). The engine's are `battle.link()`,
/// `battle.boss_rank()` and `battle.random()`.
struct BattleEffects;

impl UserData for BattleEffects {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::Index, |_, _, name: mlua::LuaString| {
            let name = name.to_str()?.to_string();
            with(|api, _| api.has_effect(&name).map_err(api_error))
        });
        methods.add_meta_method(MetaMethod::NewIndex, |_, _, (name, _): (mlua::LuaString, LuaValue)| -> mlua::Result<()> {
            Err(mlua::Error::runtime(format!("battle.effects.{} is read-only: a stage states its effects", name.to_string_lossy())))
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

/// The keys of a table a library function makes on each call, made once:
/// a key the function makes itself is hashed and looked up anew each time.
/// (The table is made whole, `create_table_from`: each `raw_set` reaches
/// it anew.)
fn keys<const N: usize>(lua: &Lua, names: [&str; N]) -> mlua::Result<[mlua::LuaString; N]> {
    let mut out = Vec::with_capacity(N);
    for name in names {
        out.push(lua.create_string(name)?);
    }
    Ok(out.try_into().unwrap_or_else(|_| unreachable!("N keys")))
}

/// A field of a table argument, as an integer (0 when absent).
fn table_int(t: &mlua::Table, key: &str) -> mlua::Result<i64> {
    let v: LuaValue = t.raw_get(key)?;
    if v.is_nil() { Ok(0) } else { int(&v, key) }
}

/// Install `battle`, `field`, `dimming`, `navi_chip`, `obstacle`, `Vec3`,
/// `Element` and `int`.
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
    lib_fn!(
        lua,
        navi_chip,
        "spring_anti_recovery",
        |lua, (user, damage, names_trap): (mlua::UserDataRef<Object>, LuaValue, Option<bool>)| {
            let damage = int(&damage, "damage")? as u32;
            let c = with(|api, _| Ok(api.spring_anti_recovery(user.0, damage, names_trap.unwrap_or(false))))?;
            object_value(lua, c)
        }
    );
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
    g.set("damage_word", damage_word_lib(lua)?)?;
    g.set("hit_modifier", hit_modifier_lib(lua)?)?;
    g.set("schema", crate::schema::lib(lua)?)?;
    g.set("Element", crate::element::lib(lua)?)?;
    crate::choice::install(lua, &g)?;
    g.set("rules", rules_lib(lua)?)?;
    g.set("custom", custom_lib(lua)?)?;
    Ok(())
}

/// A chip code as the rules write it: a letter, `*`, or nil for the code
/// outside the alphabet (its number: A 0, `*` 26; none).
fn code_arg(v: &LuaValue, what: &str) -> mlua::Result<Option<u8>> {
    match v {
        LuaValue::Nil => Ok(None),
        LuaValue::String(s) => {
            let s = s.to_str()?.to_string();
            match s.as_bytes() {
                [b'*'] => Ok(Some(26)),
                [c] if c.is_ascii_uppercase() => Ok(Some(c - b'A')),
                _ => Err(mlua::Error::runtime(format!("{what}: {s:?} is not a chip code (A-Z or *)"))),
            }
        }
        other => Err(mlua::Error::runtime(format!("{what}: a code is a letter or nil, not {}", other.type_name()))),
    }
}

/// ... and as they read it.
fn code_letter(code: Option<u8>) -> Option<String> {
    code.map(|c| if c == 26 { "*".to_string() } else { ((b'A' + c) as char).to_string() })
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
                Some((chip, code)) => {
                    let entry = lua.create_table()?;
                    entry.raw_set("chip", bound(|b| chip_value(b, Some(chip)))?)?;
                    entry.raw_set("code", code_letter(code))?;
                    LuaValue::Table(entry)
                }
                None => LuaValue::Boolean(false),
            };
            list.raw_set(i + 1, v)?;
        }
        Ok(list)
    });
    lib_fn!(lua, t, "set_folder", |_, (side, place, chip, code): (LuaValue, LuaValue, LuaValue, LuaValue)| {
        let side = u8_arg(side, "side")? & 1;
        let place = match u8_arg(place, "place")? {
            n @ 1..=30 => n - 1,
            n => return Err(mlua::Error::runtime(format!("custom.set_folder: a place is 1 to 30, not {n}"))),
        };
        let chip = bound(|b| chip_arg(b, &chip, "chip"))?.ok_or_else(|| mlua::Error::runtime("custom.set_folder: no chip"))?;
        let code = code_arg(&code, "custom.set_folder")?;
        with(|api, _| api.custom_set_folder(side, place, chip, code).map_err(api_error))
    });
    lib_fn!(lua, t, "slot", |lua, (side, slot): (LuaValue, LuaValue)| {
        let side = u8_arg(side, "side")? & 1;
        let slot = u8_arg(slot, "slot")?;
        let Some(s) = with(|api, _| api.custom_slot(side, slot).map_err(api_error))? else { return Ok(LuaValue::Nil) };
        let t = lua.create_table()?;
        t.raw_set("chip", bound(|b| chip_value(b, Some(s.chip)))?)?;
        t.raw_set("code", code_letter(s.code))?;
        t.raw_set("place", s.place.map(|p| p as i64 + 1))?;
        t.raw_set("regular", s.regular)?;
        t.raw_set("picked", s.picked)?;
        t.raw_set("listed", s.listed)?;
        Ok(LuaValue::Table(t))
    });
    lib_fn!(lua, t, "picks", |lua, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        let picks = with(|api, _| api.custom_picks(side).map_err(api_error))?;
        lua.create_sequence_from(picks)
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
    lib_fn!(
        lua,
        t,
        "offer",
        |_, (side, slot, chip, code, what): (LuaValue, LuaValue, LuaValue, LuaValue, Option<mlua::Table>)| {
            let side = u8_arg(side, "side")? & 1;
            let slot = u8_arg(slot, "slot")?;
            let chip = bound(|b| chip_arg(b, &chip, "chip"))?.ok_or_else(|| mlua::Error::runtime("custom.offer: no chip"))?;
            let code = code_arg(&code, "custom.offer")?;
            // (What the offer says of itself: no marks, listed, not shading.)
            let (mut marks, mut listed, mut shades) = (LuaValue::Nil, true, false);
            if let Some(what) = what {
                marks = what.raw_get::<LuaValue>("marks")?;
                listed = what.raw_get::<Option<bool>>("listed")?.unwrap_or(true);
                shades = what.raw_get::<Option<bool>>("shades")?.unwrap_or(false);
            }
            with(|api, _| {
                // (Its marks by name: `NamedBits::Mark`.)
                let marks = if marks.is_nil() { 0 } else { named_bits(api, NamedBits::Mark, &marks, "custom.offer: marks")? as u8 };
                api.custom_offer(side, slot, chip, code, marks, listed, shades).map_err(api_error)
            })
        }
    );
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
    lib_fn!(lua, t, "cursor_state", |lua, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        let state = with(|api, _| api.custom_cursor_state(side).map_err(api_error))?;
        crate::choice::named(lua, ChoiceKind::ButtonState, state)
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
    lib_fn!(lua, t, "play", |_, (side, sound): (LuaValue, LuaValue)| {
        let side = u8_arg(side, "side")? & 1;
        let sound = crate::choice::name(&sound, ChoiceKind::ScreenSound, "custom.play's sound")?;
        with(|api, _| api.custom_play(side, sound).map_err(api_error))
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
    lib_fn!(lua, t, "open_window", move |_, (side, window, ticks): (LuaValue, LuaValue, LuaValue)| {
        let side = u8_arg(side, "side")? & 1;
        own("custom.open_window")?;
        let window = name_arg(&window, NameKind::Window, "custom.open_window")?;
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
    lib_fn!(lua, t, "fade", |_, (side, mode, speed): (LuaValue, LuaValue, LuaValue)| {
        let side = u8_arg(side, "side")? & 1;
        let mode = crate::choice::name(&mode, ChoiceKind::CustomFade, "custom.fade's mode")?;
        let speed = int(&speed, "speed")? as u8;
        with(|api, _| api.custom_fade(side, mode, speed).map_err(api_error))
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
    lib_fn!(lua, t, "set_button_state", move |_, (side, button, state): (LuaValue, LuaValue, LuaValue)| {
        let side = u8_arg(side, "side")? & 1;
        let state = crate::choice::name(&state, ChoiceKind::ButtonState, "custom.set_button_state's state")?;
        own("custom.set_button_state")?;
        let button = name_arg(&button, NameKind::Button, "custom.set_button_state")?;
        with(|api, _| api.custom_set_button_state(side, &button, state).map_err(api_error))
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
    lib_fn!(lua, t, "set_form", move |_, (side, by, form, turns, alternate): (LuaValue, LuaValue, LuaValue, Option<LuaValue>, Option<bool>)| {
        let side = u8_arg(side, "side")? & 1;
        own("custom.set_form")?;
        let (by_kind, by) = screen_part_arg(&by, "custom.set_form")?;
        let form = form_or_nil(&form, "custom.set_form")?;
        let turns = match turns {
            Some(v) if !v.is_nil() => int(&v, "turns")? as u8,
            _ => 0,
        };
        with(|api, _| api.custom_set_form(side, screen_part(by_kind, &by), form, turns, alternate.unwrap_or(false)).map_err(api_error))
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
    lib_fn!(lua, t, "attach_to_last_pick", move |_, (side, button, marks): (LuaValue, LuaValue, LuaValue)| {
        let side = u8_arg(side, "side")? & 1;
        own("custom.attach_to_last_pick")?;
        let button = name_arg(&button, NameKind::Button, "custom.attach_to_last_pick")?;
        with(|api, _| {
            let marks = named_bits(api, NamedBits::Mark, &marks, "custom.attach_to_last_pick: marks")? as u8;
            api.custom_attach_to_last_pick(side, &button, marks).map_err(api_error)
        })
    });
    lib_fn!(lua, t, "hold_last_pick", move |_, (side, button): (LuaValue, LuaValue)| {
        let side = u8_arg(side, "side")? & 1;
        own("custom.hold_last_pick")?;
        let button = name_arg(&button, NameKind::Button, "custom.hold_last_pick")?;
        with(|api, _| api.custom_hold_last_pick(side, &button).map_err(api_error))
    });
    lib_fn!(lua, t, "held_pick", move |_, (side, button): (LuaValue, LuaValue)| {
        let side = u8_arg(side, "side")? & 1;
        own("custom.held_pick")?;
        let button = name_arg(&button, NameKind::Button, "custom.held_pick")?;
        let chip = with(|api, _| api.custom_held_pick(side, &button).map_err(api_error))?;
        bound(|b| chip_value(b, chip))
    });
    lib_fn!(lua, t, "set_held_icon", move |_, (side, button, shown): (LuaValue, LuaValue, bool)| {
        let side = u8_arg(side, "side")? & 1;
        own("custom.set_held_icon")?;
        let button = name_arg(&button, NameKind::Button, "custom.set_held_icon")?;
        with(|api, _| api.custom_set_held_icon(side, &button, shown).map_err(api_error))
    });
    lib_fn!(lua, t, "trade_last_pick", move |_, (side, button): (LuaValue, LuaValue)| {
        let side = u8_arg(side, "side")? & 1;
        own("custom.trade_last_pick")?;
        let button = name_arg(&button, NameKind::Button, "custom.trade_last_pick")?;
        with(|api, _| api.custom_trade_last_pick(side, &button).map_err(api_error))
    });
    lib_fn!(lua, t, "fading", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| api.custom_fading(side).map_err(api_error))
    });
    lib_fn!(lua, t, "form_taken", move |_, (side, by): (LuaValue, LuaValue)| {
        let side = u8_arg(side, "side")? & 1;
        own("custom.form_taken")?;
        let (by_kind, by) = screen_part_arg(&by, "custom.form_taken")?;
        with(|api, _| api.custom_form_taken(side, screen_part(by_kind, &by)).map_err(api_error))
    });
    lib_fn!(lua, t, "full", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| api.custom_full(side).map_err(api_error))
    });
    lib_fn!(lua, t, "button_picked", move |_, (side, button): (LuaValue, LuaValue)| {
        let side = u8_arg(side, "side")? & 1;
        own("custom.button_picked")?;
        let button = name_arg(&button, NameKind::Button, "custom.button_picked")?;
        with(|api, _| api.custom_button_picked(side, &button).map_err(api_error))
    });
    lib_fn!(lua, t, "draw_window", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| api.custom_draw_window(side).map_err(api_error))
    });
    lib_fn!(lua, t, "draw_regular", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| api.custom_draw_regular(side).map_err(api_error))
    });
    lib_fn!(lua, t, "draw_cursor", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| api.custom_draw_cursor(side).map_err(api_error))
    });
    lib_fn!(lua, t, "button_uses", |_, (side, button): (LuaValue, LuaValue)| {
        let side = u8_arg(side, "side")? & 1;
        let button = name_arg(&button, NameKind::Button, "custom.button_uses")?;
        with(|api, _| api.custom_button_uses(side, &button).map_err(api_error))
    });
    lib_fn!(lua, t, "set_button_uses", |_, (side, button, uses): (LuaValue, LuaValue, LuaValue)| {
        let side = u8_arg(side, "side")? & 1;
        let button = name_arg(&button, NameKind::Button, "custom.set_button_uses")?;
        let uses = u8_arg(uses, "uses")?;
        with(|api, _| api.custom_set_button_uses(side, &button, uses).map_err(api_error))
    });
    lib_fn!(lua, t, "draw_held", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| api.custom_draw_held(side).map_err(api_error))
    });
    lib_fn!(lua, t, "show_chip_window", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| api.custom_show_chip_window(side).map_err(api_error))
    });
    lib_fn!(lua, t, "set_form_list_tab", |_, (side, on): (LuaValue, bool)| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| api.custom_set_form_list_tab(side, on).map_err(api_error))
    });
    lib_fn!(lua, t, "refresh_buttons", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| api.custom_refresh_buttons(side).map_err(api_error))
    });
    lib_fn!(lua, t, "player", |lua, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        let p = with(|api, _| api.custom_player(side).map_err(api_error))?;
        let t = lua.create_table()?;
        t.raw_set("emotion", crate::names::value(lua, NameKind::Emotion, &p.emotion)?)?;
        t.raw_set("random_battle", p.random_battle)?;
        t.raw_set("member", p.member)?;
        Ok(t)
    });
    Ok(t)
}

/// `rules`: what the game's rules' own calls reach (docs/design/
/// rules-in-luau.md §4.5, §5.3): their state and the player's setup, of
/// the side they were called for, and that side.
fn rules_lib(lua: &Lua) -> mlua::Result<mlua::Table> {
    let t = lua.create_table()?;
    lib_fn!(lua, t, "state", |lua, ()| state_handle(lua, StateOf::Rules(rules_ctx("rules.state")?)));
    lib_fn!(lua, t, "setup", |lua, ()| state_handle(lua, StateOf::Setup(rules_ctx("rules.setup")?)));
    // The rules' state of side `side`, for a game's rules alone (its API
    // module, docs/design/rules-in-luau.md, As built S8): the game's API
    // reaches what its rules keep (EXE6's bug frags) for its chips, which
    // never call it themselves.
    lib_fn!(lua, t, "state_of", |lua, side: LuaValue| {
        rules_only(lua, "rules.state_of", "state")?;
        state_handle(lua, StateOf::Rules(RulesCtx { side: u8_arg(side, "side")? & 1 }))
    });
    // Side `side`'s player's setup of the rules, read-only, likewise a
    // game's rules' alone (its API module: EXE6's and EXE5's navi level,
    // the rules' `level`).
    lib_fn!(lua, t, "setup_of", |lua, side: LuaValue| {
        rules_only(lua, "rules.setup_of", "setup")?;
        state_handle(lua, StateOf::Setup(RulesCtx { side: u8_arg(side, "side")? & 1 }))
    });
    // Whether the engine calls each hook of side `side`'s rules.
    lib_fn!(lua, t, "listening", |lua, side: LuaValue| {
        rules_only(lua, "rules.listening", "hooks")?;
        let side = u8_arg(side, "side")? & 1;
        view(lua, View::Listening(side))
    });
    // The panel kinds `navi_panel` is called on, for side `side`.
    lib_fn!(lua, t, "listen_panels", |lua, (side, kinds): (LuaValue, LuaValue)| {
        rules_only(lua, "rules.listen_panels", "hooks")?;
        let side = u8_arg(side, "side")? & 1;
        let mut mask = 0u128;
        for name in name_set(&kinds, "rules.listen_panels")? {
            let kind = with(|api, _| Ok(api.panel_type_named(&name)))?
                .ok_or_else(|| mlua::Error::runtime(format!("rules.listen_panels: {name:?} is no panel type")))?;
            if kind >= 128 {
                return Err(mlua::Error::runtime(format!("rules.listen_panels: panel type {name:?} is number {kind}, past 127")));
            }
            mask |= 1 << kind;
        }
        with(|api, _| api.listen_panels(side, mask).map_err(api_error))
    });
    Ok(t)
}

/// A call only a game's rules make (a module under rules/: its API module).
fn rules_only(lua: &Lua, what: &str, reaches: &str) -> mlua::Result<()> {
    // (The caller's module as `define::caller` names it, read in place: the
    // chunk's name at stack level 1, as mlua's `inspect_stack` and `source`
    // read it.)
    let mut ar: mlua::ffi::lua_Debug = unsafe { std::mem::zeroed() };
    // SAFETY: the running state, level 1, what `s` fills in: its chunk's
    // name, a NUL-terminated string that lives as long as the function's
    // prototype.
    let found = unsafe { mlua::ffi::lua_getinfo(lua.state(), 1, c"s".as_ptr(), &mut ar) } != 0 && !ar.source.is_null();
    if found {
        let bytes = unsafe { std::ffi::CStr::from_ptr(ar.source) }.to_bytes();
        // (As `from_utf8_lossy` reads it, its quicker check first.)
        let source = std::str::from_utf8(bytes).map_or_else(|_| String::from_utf8_lossy(bytes), std::borrow::Cow::Borrowed);
        // (`trim_end_matches(".luau")`, without its searcher's setup.)
        let mut module = source.trim_start_matches('@');
        while let Some(m) = module.strip_suffix(".luau") {
            module = m;
        }
        if nettai_content_api::keys::local(module).starts_with("rules/") {
            return Ok(());
        }
    }
    let caller = crate::define::caller(lua);
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
    lib_fn!(lua, t, "cycle", |_, which: LuaValue| {
        let which = crate::choice::parsed(&which, ChoiceKind::FightCycle, "fight cycle", FightCycle::from_name)? as u8 + 1;
        with(|api, _| Ok(api.cycle(which)))
    });
    lib_fn!(lua, t, "weakness", |_, (receiver, hitter): (LuaValue, LuaValue)| {
        let receiver = crate::element::primary_arg(&receiver, "battle.weakness's receiver")?;
        let hitter = crate::element::primary_arg(&hitter, "battle.weakness's hitter")?;
        with(|api, _| Ok(api.weakness(receiver, hitter)))
    });
    // The other side: 1 for 0, 0 for 1.
    lib_fn!(lua, t, "other_side", |_, side: LuaValue| {
        let side = u8_arg(side, "side")?;
        if side > 1 {
            return Err(mlua::Error::runtime(format!("battle.other_side: a side is 0 or 1, not {side}")));
        }
        Ok(side ^ 1)
    });
    lib_fn!(lua, t, "lose_round", |_, side: u8| with(|api, _| {
        api.lose_round(side & 1);
        Ok(())
    }));
    lib_fn!(lua, t, "next_chip_damages", |_, user: mlua::UserDataRef<Object>| {
        with(|api, _| Ok(api.next_chip_damages(user.0)))
    });
    // The battle's effects its game's rules declare of their own, each a
    // property (`battle.effects.karma_held`; read-only).
    t.set("effects", lua.create_userdata(BattleEffects)?)?;
    for &f in BattleInfo::ALL {
        t.set(
            f.name(),
            lua.create_function(move |lua, ()| {
                let v = with(|api, _| Ok(api.battle_info(f)))?;
                show(lua, v, f.field_type(), Shown::of(f.field_type()))
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
        let console = match spec.get::<LuaValue>("console")? {
            LuaValue::Nil => None,
            v => Some(side_arg(v)?),
        };
        with(|api, _| Ok(api.show_hp(o.0, dx, dy, damage, console)))
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
    lib_fn!(lua, t, "set_chips_enabled", |_, (side, on): (LuaValue, bool)| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| Ok(api.set_chips_enabled(side, on)))
    });
    lib_fn!(lua, t, "burst", |lua, navi: mlua::UserDataRef<Object>| {
        let o = with(|api, _| Ok(api.spawn_burst(navi.0)))?;
        object_value(lua, o)
    });
    lib_fn!(lua, t, "screen_fade", |_, (fade, speed): (LuaValue, LuaValue)| {
        let fade = crate::choice::parsed(&fade, ChoiceKind::ScreenFade, "screen fade", ScreenFade::from_name)?;
        let speed = u8_arg(speed, "fade speed")?;
        with(|api, _| Ok(api.screen_fade(fade, speed)))
    });
    lib_fn!(lua, t, "hud", |lua, side: Option<LuaValue>| {
        let side = side.filter(|s| !s.is_nil()).map(side_arg).transpose()?;
        view(lua, View::Hud(side))
    });
    lib_fn!(lua, t, "navi", |lua, side: LuaValue| navi_handle(lua, nettai_content_api::StatsOf::side(u8_arg(side, "side")?)));
    lib_fn!(lua, t, "member", |lua, (side, m): (LuaValue, LuaValue)| {
        let side = u8_arg(side, "side")? & 1;
        let m = u8_arg(m, "member")?;
        let count = with(|api, _| Ok(api.party(side).count))?;
        (m < count).then(|| navi_handle(lua, nettai_content_api::StatsOf { side, member: Some(m) })).transpose()
    });
    lib_fn!(lua, t, "mic_level", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| Ok(api.mic_level(side)))
    });
    let party_keys = keys(lua, ["count", "active", "switches"])?;
    lib_fn!(lua, t, "party", move |lua, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        let p = with(|api, _| Ok(api.party(side)))?;
        let [count, active, switches] = &party_keys;
        lua.create_table_from([(count, p.count), (active, p.active), (switches, p.switches)])
    });
    lib_fn!(lua, t, "join_party", |lua, (side, navi): (LuaValue, LuaValue)| {
        let side = u8_arg(side, "side")? & 1;
        let m = with(|api, b| match b.def(&navi) {
            Some((Registry::Navi, h)) => api.join_party(side, h).map_err(api_error),
            _ => Err(mlua::Error::runtime("battle.join_party: expected a navi definition")),
        })?;
        navi_handle(lua, nettai_content_api::StatsOf { side, member: Some(m) })
    });
    lib_fn!(lua, t, "emotion", |lua, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        let emotion = with(|api, _| Ok(api.emotion(side)))?;
        crate::names::value(lua, NameKind::Emotion, &emotion)
    });
    lib_fn!(lua, t, "set_mood", |_, (side, mood): (LuaValue, LuaValue)| {
        let side = u8_arg(side, "side")? & 1;
        let Value::Int(mood) = take(mood, &FieldType::U8, Shown::Mood, "battle.set_mood's mood")? else { unreachable!("a mood") };
        let mood = u8::try_from(mood).map_err(|_| mlua::Error::runtime(format!("battle.set_mood: a mood is a byte, not {mood}")))?;
        with(|api, _| Ok(api.set_mood(side, mood)))
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
        if !f.offered.is_empty() {
            let offered = lua.create_table()?;
            for (i, chip) in f.offered.iter().enumerate() {
                offered.raw_set(i + 1, bound(|b| b.def_value(Registry::Chip, *chip))?)?;
            }
            out.raw_set("offered", offered)?;
        }
        Ok(LuaValue::Table(out))
    });
    // The limit `name` a tool checks a build under (its ruleset's: the
    // game's rules/rulesets names the limits): `own`, the rules' own, where
    // the ruleset leaves it (and in a round); its number in place of it; nil
    // where it holds the build to none.
    lib_fn!(lua, t, "limit", |_, (name, own): (String, LuaValue)| {
        use nettai_content_api::api::Limit;
        Ok(match with(|api, _| Ok(api.limit(&name)))? {
            None => own,
            Some(Limit::Lifted) => LuaValue::Nil,
            Some(Limit::AtMost(n)) => LuaValue::Integer(n.into()),
        })
    });
    // A definition's name in the content's own strings, else its key: for
    // what the rules say to a tool (`validate`).
    lib_fn!(lua, t, "name", |_, def: LuaValue| {
        let (registry, h) = bound(|b| Ok(b.def(&def)))?.ok_or_else(|| mlua::Error::runtime("battle.name takes a definition"))?;
        with(|api, _| Ok(api.def_name(registry, h)))
    });
    lib_fn!(lua, t, "side_special", |lua, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        let special = with(|api, _| Ok(api.side_special(side)))?;
        crate::choice::named(lua, ChoiceKind::SideSpecial, special.name())
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
    lib_fn!(lua, t, "sees", |_, (viewer, side): (LuaValue, LuaValue)| {
        let (viewer, side) = (side_arg(viewer)?, u8_arg(side, "side")?);
        with(|api, _| Ok(api.sees(viewer, side)))
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
    lib_fn!(lua, t, "alive_count", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| Ok(api.alive_count(side)))
    });
    lib_fn!(lua, t, "set_alive_count", |_, (side, n): (LuaValue, LuaValue)| {
        let (side, n) = (u8_arg(side, "side")? & 1, u8_arg(n, "count")?);
        with(|api, _| Ok(api.set_alive_count(side, n)))
    });
    lib_fn!(lua, t, "alive_actors", |lua, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        let list = with(|api, _| Ok(api.alive_actors(side)))?;
        object_list(lua, list)
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
        object_list(lua, list)
    });
    lib_fn!(lua, t, "rng", |_, ()| with(|api, _| Ok(api.rng())));
    lib_fn!(lua, t, "rng_positive", |_, ()| with(|api, _| Ok(api.rng_positive())));
    lib_fn!(lua, t, "console_rng_positive", |_, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        with(|api, _| Ok(api.console_rng_positive(side)))
    });
    // A shuffle's new order, as a list of 1-based indices.
    fn count(v: &LuaValue, what: &str) -> mlua::Result<usize> {
        usize::try_from(int(v, what)?).map_err(|_| mlua::Error::runtime(format!("{what} is a count, 0 or more")))
    }
    lib_fn!(lua, t, "shuffle_order", |lua, (n, rounds): (LuaValue, LuaValue)| {
        let (n, rounds) = (count(&n, "n")?, count(&rounds, "rounds")?);
        let order = with(|api, _| api.shuffle_order(n, rounds).map_err(api_error))?;
        lua.create_sequence_from(order.into_iter().map(|i| i + 1))
    });
    lib_fn!(lua, t, "console_shuffle_order", |lua, (side, n, rounds): (LuaValue, LuaValue, LuaValue)| {
        let side = u8_arg(side, "side")? & 1;
        let (n, rounds) = (count(&n, "n")?, count(&rounds, "rounds")?);
        let order = with(|api, _| api.console_shuffle_order(side, n, rounds).map_err(api_error))?;
        lua.create_sequence_from(order.into_iter().map(|i| i + 1))
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
    lib_fn!(lua, t, "show_used_chip", |_, (side, chip, damage, bonus): (LuaValue, LuaValue, LuaValue, LuaValue)| {
        let side = u8_arg(side, "side")? & 1;
        let chip = bound(|b| chip_arg(b, &chip, "battle.show_used_chip's chip"))?
            .ok_or_else(|| mlua::Error::runtime("battle.show_used_chip: a chip, not nil"))?;
        let (damage, bonus) = (u16_arg(damage, "damage")?, u16_arg(bonus, "bonus")?);
        with(|api, _| Ok(api.show_used_chip(side, chip, damage, bonus)))
    });
    lib_fn!(lua, t, "trap_mark", |_, o: mlua::UserDataRef<Object>| with(|api, _| Ok(api.trap_mark(o.0))));
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
    lib_fn!(lua, t, "add_special_bonus", |_, (side, bonus, n): (LuaValue, LuaValue, LuaValue)| {
        let index = crate::choice::parsed(&bonus, ChoiceKind::SpecialBonus, "special bonus", SpecialBonus::from_name)? as u8;
        let (side, n) = (u8_arg(side, "side")? & 1, u16_arg(n, "bonus")?);
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
    // A side's statistics, each by its name (its game's rules' `side_stats`).
    lib_fn!(lua, t, "bump_side_stat", |_, (side, stat, n): (LuaValue, Name, LuaValue)| {
        let (side, n) = (u8_arg(side, "side")? & 1, u8_arg(n, "count")?);
        let stat = stat.to_str()?;
        with(|api, _| {
            let i = api.side_stat_named(stat).map_err(api_error)?;
            Ok(api.bump_side_stat(side, i, n))
        })
    });
    lib_fn!(lua, t, "set_side_stat", |_, (side, stat, n): (LuaValue, Name, LuaValue)| {
        let (side, n) = (u8_arg(side, "side")? & 1, u8_arg(n, "count")?);
        let stat = stat.to_str()?;
        with(|api, _| {
            let i = api.side_stat_named(stat).map_err(api_error)?;
            Ok(api.set_side_stat(side, i, n))
        })
    });
    lib_fn!(lua, t, "side_stat", |_, (side, stat): (LuaValue, Name)| {
        let side = u8_arg(side, "side")? & 1;
        let stat = stat.to_str()?;
        with(|api, _| {
            let i = api.side_stat_named(stat).map_err(api_error)?;
            Ok(api.side_stat(side, i))
        })
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
    lib_fn!(lua, t, "spawn_at_end", |lua, (kind, pos): (LuaValue, Option<mlua::UserDataRef<LVec3>>)| {
        spawn_kind_def(lua, kind, vec3_arg(pos), SpawnAt::End)
    });
    lib_fn!(
        lua,
        t,
        "effect",
        |lua, (pos, id, facing, palette_add, priority, upside_down): (
            mlua::UserDataRef<LVec3>,
            LuaValue,
            Option<LuaValue>,
            Option<LuaValue>,
            Option<LuaValue>,
            Option<bool>
        )| {
            let opt = |v: Option<LuaValue>, what| v.map_or(Ok(0), |v| u8_arg(v, what));
            // (The side whose way it faces, or each side's console's: { side
            // 0's, side 1's }; upside down, the game's flip value's bit 1.)
            let vertical = if upside_down.unwrap_or(false) { 2 } else { 0 };
            let flips = match facing {
                Some(LuaValue::Table(t)) => {
                    [facing_arg(t.raw_get(1)?, "battle.effect")? | vertical, facing_arg(t.raw_get(2)?, "battle.effect")? | vertical]
                }
                Some(v) => [facing_arg(v, "battle.effect")? | vertical; 2],
                None => [vertical; 2],
            };
            let (add, prio) = (opt(palette_add, "palette")?, opt(priority, "priority")?);
            let o = with(|api, b| {
                let look = EffectHandle(def_arg(b, &id, Registry::Effect, "battle.effect")?);
                Ok(api.spawn_effect(pos.0, look, flips, add, prio))
            })?;
            object_value(lua, o)
        }
    );
    lib_fn!(
        lua,
        t,
        "effect_after_spawn",
        |lua, (z, id, facing, palette_add, priority): (
            i32,
            LuaValue,
            Option<LuaValue>,
            Option<LuaValue>,
            Option<LuaValue>
        )| {
            let opt = |v: Option<LuaValue>, what| v.map_or(Ok(0), |v| u8_arg(v, what));
            let flip = facing.map_or(Ok(0), |v| facing_arg(v, "battle.effect_after_spawn"))?;
            let (add, prio) = (opt(palette_add, "palette")?, opt(priority, "priority")?);
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
            let variant = crate::choice::parsed(&variant, ChoiceKind::PaletteFlashVariant, "palette flash variant", PaletteFlashVariant::from_name)? as u8;
            let ticks = u8_arg(ticks, "palette flash ticks")?;
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
        // (luau-analyze lets a table literal's extra keys through.)
        const KEYS: [&str; 13] =
            ["panel_x", "panel_y", "element", "z", "region", "hit_effect", "target", "self_type", "damage", "hit_mod", "status", "bug", "bug_arg"];
        for pair in spec.pairs::<LuaValue, LuaValue>() {
            let (k, _) = pair?;
            if !matches!(&k, LuaValue::String(k) if KEYS.iter().any(|n| k.as_bytes() == n.as_bytes())) {
                return Err(mlua::Error::runtime(format!("battle.hitbox: a hitbox has no field {k:?} (it has {})", KEYS.join(", "))));
            }
        }
        let word = match spec.raw_get::<LuaValue>("damage")? {
            LuaValue::Nil => 0,
            w => word_arg(&w, "battle.hitbox: damage")?,
        };
        let hit_mod: LuaValue = spec.raw_get("hit_mod")?;
        let hit_mod = if hit_mod.is_nil() { 0 } else { with(|api, _| hit_modifier_arg(api, &hit_mod, "battle.hitbox: hit_mod"))? };
        let s = HitboxSpec {
            panel: PanelPos { x: table_int(&spec, "panel_x")? as u8, y: table_int(&spec, "panel_y")? as u8 },
            element: crate::element::arg(&spec.raw_get("element")?, "battle.hitbox's element")?,
            z: table_int(&spec, "z")? as i32,
            region: def("region", Registry::Region)?.map(RegionHandle),
            hit_effect: def("hit_effect", Registry::Spark)?.map(SparkHandle),
            target: collision("target")?,
            self_type: collision("self_type")?,
            damage: word as u16,
            stamina: (word >> 16) as u16,
            hit_mod,
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
        let stepping = match crate::choice::opt_arg(&spec.raw_get("stepping")?, ChoiceKind::FormOverlayStepping, "form overlay `stepping`")? {
            Some(c) => c.index,
            None => 0,
        };
        let anim_offset = table_int(&spec, "anim_offset")? as u8;
        let pinned = spec.raw_get::<Option<bool>>("pinned")?.unwrap_or(false);
        let owner_palette = spec.raw_get::<Option<bool>>("owner_palette")?.unwrap_or(false);
        let o = with(|api, _| Ok(api.spawn_form_overlay(owner.0, sprite, stepping, anim_offset, pinned, owner_palette)))?;
        object_value(lua, o)
    });
    // A hand's entries, by field (from 0; past the hand, as a negative
    // entry, none).
    let entry = |i: LuaValue| -> mlua::Result<u8> { Ok(u8::try_from(int(&i, "hand entry")?).unwrap_or(u8::MAX)) };
    lib_fn!(lua, t, "hand_entry", move |lua, (side, i): (LuaValue, LuaValue)| {
        let (side, i) = (u8_arg(side, "side")? & 1, entry(i)?);
        view(lua, View::HandEntry(side, i))
    });
    lib_fn!(lua, t, "fold_hand_chip", |_, (side, i, whole): (LuaValue, LuaValue, bool)| {
        let (side, i) = (u8_arg(side, "side")? & 1, u8_arg(i, "hand index")?);
        with(|api, _| Ok(api.fold_hand_chip(side, i, whole)))
    });
    lib_fn!(
        lua,
        t,
        "afterimage",
        |lua, (owner, pos, spec): (mlua::UserDataRef<Object>, mlua::UserDataRef<LVec3>, mlua::Table)| {
            // (luau-analyze lets a table's extra keys through: one the spec
            // hasn't is refused here.)
            const KEYS: [&str; 10] =
                ["sprite", "anim", "facing", "upside_down", "lifetime", "color_shader", "palette", "shadow", "steady", "tether"];
            for pair in spec.pairs::<LuaValue, LuaValue>() {
                let (k, _) = pair?;
                let known = matches!(&k, LuaValue::String(s) if KEYS.contains(&&*s.to_str()?));
                if !known {
                    return Err(mlua::Error::runtime(format!("battle.afterimage: {k:?} is no field of an afterimage's spec ({})", KEYS.join(", "))));
                }
            }
            let sprite: LuaValue = spec.raw_get("sprite")?;
            let shadow = match crate::choice::opt_arg(&spec.raw_get("shadow")?, ChoiceKind::Shadow, "afterimage `shadow`")? {
                Some(c) => nettai_content_api::Shadow::ALL[c.index as usize],
                None => nettai_content_api::Shadow::WithSprite,
            };
            // (None 0, form 1, attack 2.)
            let tether = match crate::choice::opt_arg(&spec.raw_get("tether")?, ChoiceKind::Tether, "afterimage `tether`")? {
                Some(c) => c.index + 1,
                None => 0,
            };
            let s = nettai_content_api::api::AfterimageSpec {
                sprite: if sprite.is_nil() { None } else { Some(sprite_id(sprite)?) },
                anim: table_int(&spec, "anim")? as u8,
                flip: {
                    let facing: LuaValue = spec.raw_get("facing")?;
                    let facing = if facing.is_nil() { 0 } else { facing_arg(facing, "battle.afterimage's facing")? };
                    facing | if spec.raw_get::<Option<bool>>("upside_down")?.unwrap_or(false) { 2 } else { 0 }
                },
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
    lib_fn!(lua, t, "panel_at", |_, (x, y, side): (LuaValue, LuaValue, LuaValue)| {
        let (x, y, side) = (int(&x, "x")? as i32, int(&y, "y")? as i32, int(&side, "side")? as u8);
        let p = nettai_content_api::PanelPos::under(x, y, side);
        Ok((p.x as i64, p.y as i64))
    });
    lib_fn!(lua, t, "canonical_side", |_, (): ()| Ok(nettai_content_api::CANONICAL_SIDE as i64));
    lib_fn!(lua, t, "side_sign", |_, side: LuaValue| Ok(nettai_content_api::side_sign(int(&side, "side")? as u8) as i64));
    lib_fn!(lua, t, "passed", |_, (new, old, target, side): (LuaValue, LuaValue, LuaValue, LuaValue)| {
        let (new, old, target) = (int(&new, "new")? as i32, int(&old, "old")? as i32, int(&target, "target")? as i32);
        Ok(match side {
            LuaValue::Nil => nettai_content_api::passed(new, old, target),
            side => nettai_content_api::passed_x(new, old, target, int(&side, "side")? as u8),
        })
    });
    lib_fn!(lua, t, "check", |lua, (x, y, cond, side): (LuaValue, LuaValue, LuaValue, LuaValue)| {
        let p = panel(x, y)?;
        let words = Names::condition(lua, &cond)?;
        let (require, forbid) = Names::for_side(words, &side, "field.check")?;
        with(|api, _| Ok(api.panel_check(p, require, forbid)))
    });
    lib_fn!(lua, t, "any", |lua, (x, y, set, side): (LuaValue, LuaValue, LuaValue, LuaValue)| {
        let p = panel(x, y)?;
        let bits = Names::for_side(Names::any(lua, &set)?, &side, "field.any")?;
        with(|api, _| Ok(api.panel_flags(p) & bits != 0))
    });
    lib_fn!(lua, t, "low_byte", |_, (x, y): (LuaValue, LuaValue)| {
        let p = panel(x, y)?;
        with(|api, _| Ok(api.panel_flags(p) & 0xFF))
    });
    let panel_keys = keys(lua, ["kind", "alliance", "home"])?;
    lib_fn!(lua, t, "panel", move |lua, (x, y): (LuaValue, LuaValue)| {
        let p = panel(x, y)?;
        let Some((info, name)) = with(|api, _| Ok(api.panel_info(p).map(|i| (i, api.panel_type_name(i.kind)))))? else {
            return Ok(LuaValue::Nil);
        };
        let [kind, alliance, home] = &panel_keys;
        let t = lua.create_table_from([
            (kind, mlua::Either::Left(crate::names::value(lua, NameKind::PanelType, &name)?)),
            (alliance, mlua::Either::Right(info.alliance)),
            (home, mlua::Either::Right(info.home)),
        ])?;
        Ok(LuaValue::Table(t))
    });
    lib_fn!(lua, t, "all_objects", |lua, side: Option<LuaValue>| {
        let mut list = with(|api, _| Ok(api.all_field_objects()))?;
        // (For a side other than the canonical, the two sides' slots trade
        // places: the canonical side's order, mirrored.
        // docs/engine/side-symmetry.md.)
        if let Some(side) = side.filter(|v| !v.is_nil()) {
            let side = u8_arg(side, "side")? & 1;
            if nettai_content_api::side_sign(side) < 0 {
                let sides: Vec<_> = (0..2u8).map(|s| with(|api, _| Ok(api.side_field_objects(s)))).collect::<mlua::Result<_>>()?;
                let stage: Vec<_> = list.iter().copied().filter(|o| !sides.iter().any(|l: &Vec<_>| l.contains(o))).collect();
                list = sides[1].iter().chain(sides[0].iter()).chain(stage.iter()).copied().collect();
            }
        }
        object_list(lua, list)
    });
    lib_fn!(lua, t, "objects", |lua, side: LuaValue| {
        let side = u8_arg(side, "side")? & 1;
        let list = with(|api, _| Ok(api.side_field_objects(side)))?;
        object_list(lua, list)
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
    let column_keys = keys(lua, ["home", "timer"])?;
    lib_fn!(lua, t, "column", move |lua, x: LuaValue| {
        let x = u8_arg(x, "column")?;
        let c = with(|api, _| Ok(api.column_info(x)))?;
        let [home, timer] = &column_keys;
        lua.create_table_from([(home, c.home as u16), (timer, c.timer)])
    });
    lib_fn!(lua, t, "set_column_timer", |_, (x, ticks): (LuaValue, LuaValue)| {
        let (x, ticks) = (u8_arg(x, "column")?, u16_arg(ticks, "ticks")?);
        with(|api, _| Ok(api.set_column_timer(x, ticks)))
    });
    lib_fn!(lua, t, "set_alliance", |_, (x, y, side): (LuaValue, LuaValue, LuaValue)| {
        let (p, side) = (panel(x, y)?, u8_arg(side, "side")?);
        with(|api, _| Ok(api.set_panel_alliance(p, side)))
    });
    lib_fn!(lua, t, "set_type", |_, (x, y, kind): (LuaValue, LuaValue, LuaValue)| {
        let p = panel(x, y)?;
        let kind = panel_type_arg(&kind, "field.set_type")?;
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
    lib_fn!(lua, t, "timer", |_, (x, y): (LuaValue, LuaValue)| {
        let p = panel(x, y)?;
        with(|api, _| Ok(api.panel_timer(p)))
    });
    lib_fn!(lua, t, "set_timer", |_, (x, y, ticks): (LuaValue, LuaValue, LuaValue)| {
        let (p, ticks) = (panel(x, y)?, u16_arg(ticks, "ticks")?);
        with(|api, _| Ok(api.set_panel_timer(p, ticks)))
    });
    lib_fn!(lua, t, "cycle", |_, ()| with(|api, _| Ok(api.field_cycle())));
    lib_fn!(lua, t, "grounded_body", |_, (x, y): (LuaValue, LuaValue)| {
        let p = panel(x, y)?;
        with(|api, _| Ok(api.panel_body_grounded(p)))
    });
    lib_fn!(lua, t, "highlight", |_, (x, y): (LuaValue, LuaValue)| {
        let p = panel(x, y)?;
        with(|api, _| Ok(api.highlight_panel(p)))
    });
    // Panel changes (dimming chip subtypes 2, 3, 5, 15 and 27).
    lib_fn!(lua, t, "overwrite", |_, (x, y, kind, sound): (LuaValue, LuaValue, LuaValue, LuaValue)| {
        let p = panel(x, y)?;
        let kind = panel_type_arg(&kind, "field.overwrite")?;
        let sound = match sound {
            LuaValue::Nil => None,
            s => Some(sound_arg(s)?),
        };
        with(|api, _| Ok(api.overwrite_panel(p, kind, sound)))
    });
    lib_fn!(lua, t, "blink", |_, (x, y, kind, side): (LuaValue, LuaValue, LuaValue, LuaValue)| {
        let (p, side) = (panel(x, y)?, u8_arg(side, "side")?);
        let kind = panel_type_arg(&kind, "field.blink")?;
        with(|api, _| Ok(api.blink_panel(p, kind, side)))
    });
    lib_fn!(lua, t, "set_visible", |_, (x, y, visible): (LuaValue, LuaValue, bool)| {
        let p = panel(x, y)?;
        with(|api, _| Ok(api.set_panel_visible(p, visible)))
    });
    Ok(t)
}

/// Sets and conditions of named members (a table of booleans keyed by
/// member: `{ solid = true, occupied = false }`), resolved by the engine
/// ([`CoreApi::panel_flag_bits`], [`CoreApi::hit_bits`]) into the bits they
/// stand for, side 0's and side 1's. In a set a member set to false is one
/// left out; in a condition it must not hold. A table frozen through (a
/// module's constant: what content passes) is resolved once and kept by
/// itself in a table whose keys are weak, so a table collected takes its
/// entry with it; any other table (one built at the call) is resolved at
/// each call. What is kept depends on the members alone, not on the battle.
/// A key that is no name, a value that is no boolean, or a name the engine
/// doesn't know, is an error.
pub(crate) struct Names;

/// A resolution [`Names`] keeps: side 0's and side 1's bits.
struct NamesWords<T>([T; 2]);

impl<T: 'static> UserData for NamesWords<T> {}

/// What names are resolved as: panel flags or hit results.
#[derive(Clone, Copy)]
enum NamesOf {
    Panel,
    Hit,
}

/// A table of panel flags resolved for a side: the bits its members set to
/// true stand for (set, clear), and those of its members set to false.
#[derive(Clone, Copy, Default, PartialEq)]
struct PanelSet {
    holds: (u32, u32),
    not: (u32, u32),
}

impl Names {
    /// The kept resolution of `v`, or `resolve`'s, kept if `v` is frozen
    /// through.
    fn cached<T: Copy + 'static>(
        lua: &Lua,
        of: NamesOf,
        v: &mlua::Table,
        resolve: impl FnOnce() -> mlua::Result<[T; 2]>,
    ) -> mlua::Result<[T; 2]> {
        if !v.is_readonly() {
            return resolve();
        }
        let key = match of {
            NamesOf::Panel => "nettai.panel_names",
            NamesOf::Hit => "nettai.hit_names",
        };
        let cache: mlua::Table = match lua.named_registry_value::<Option<mlua::Table>>(key)? {
            Some(c) => c,
            None => {
                let c = lua.create_table()?;
                let meta = lua.create_table()?;
                meta.raw_set("__mode", "k")?;
                c.set_metatable(Some(meta))?;
                lua.set_named_registry_value(key, &c)?;
                c
            }
        };
        if let Some(ud) = cache.raw_get::<Option<AnyUserData>>(v)?
            && let Ok(r) = ud.borrow::<NamesWords<T>>()
        {
            return Ok(r.0);
        }
        let words = resolve()?;
        cache.raw_set(v, lua.create_userdata(NamesWords(words))?)?;
        Ok(words)
    }

    /// The members `v` names (a table of booleans keyed by name), each with
    /// its value.
    fn read(v: &LuaValue, what: &str) -> mlua::Result<(mlua::Table, Vec<(mlua::LuaString, bool)>)> {
        let LuaValue::Table(t) = v else {
            return Err(mlua::Error::runtime(format!("{what}: a table of booleans keyed by name (`{{ name = true }}`), not {}", v.type_name())));
        };
        let mut names = Vec::new();
        for pair in t.clone().pairs::<LuaValue, LuaValue>() {
            match pair? {
                (LuaValue::String(k), LuaValue::Boolean(on)) => names.push((k, on)),
                (LuaValue::String(k), other) => {
                    return Err(mlua::Error::runtime(format!("{what}: `{}` is {}, not true or false", k.to_string_lossy(), other.type_name())));
                }
                (k, _) => {
                    return Err(mlua::Error::runtime(format!("{what}: a table of booleans keyed by name (`{{ name = true }}`), not keyed by {}", k.type_name())));
                }
            }
        }
        Ok((t.clone(), names))
    }

    /// A table of panel flags resolved, side 0's and side 1's.
    fn panel(lua: &Lua, v: &LuaValue, what: &str) -> mlua::Result<[PanelSet; 2]> {
        let (t, names) = Names::read(v, what)?;
        Names::cached(lua, NamesOf::Panel, &t, || {
            let mut sets = [PanelSet::default(); 2];
            with(|api, _| {
                for (side, set) in sets.iter_mut().enumerate() {
                    for (name, on) in &names {
                        let n = api.panel_flag_bits(&name.to_str()?, Some(side as u8)).map_err(|m| mlua::Error::runtime(format!("{what}: {m}")))?;
                        let bits = if *on { &mut set.holds } else { &mut set.not };
                        bits.0 |= n.set;
                        bits.1 |= n.clear;
                    }
                }
                Ok(())
            })?;
            Ok(sets)
        })
    }

    /// A set of hit names' bits (`Collision:hit_any`), side 0's and side
    /// 1's.
    fn hits(lua: &Lua, v: &LuaValue) -> mlua::Result<[u32; 2]> {
        let (t, names) = Names::read(v, "hit_any")?;
        Names::cached(lua, NamesOf::Hit, &t, || {
            let mut bits = [0u32; 2];
            with(|api, _| {
                for (side, w) in bits.iter_mut().enumerate() {
                    for (name, on) in &names {
                        let b = api.hit_bits(&name.to_str()?, Some(side as u8)).map_err(|m| mlua::Error::runtime(format!("hit_any: {m}")))?;
                        if *on {
                            *w |= b;
                        }
                    }
                }
                Ok(())
            })?;
            Ok(bits)
        })
    }

    /// `field.check`'s condition, side 0's and side 1's: (require, forbid),
    /// the bits that must be set and those that must be clear; none for nil.
    fn condition(lua: &Lua, cond: &LuaValue) -> mlua::Result<[(u32, u32); 2]> {
        if cond.is_nil() {
            return Ok([(0, 0); 2]);
        }
        let sets = Names::panel(lua, cond, "field.check")?;
        Ok(sets.map(|s| (s.holds.0 | s.not.1, s.not.0 | s.holds.1)))
    }

    /// `field.any`'s bits, side 0's and side 1's: its members set to true. A
    /// side's area, which is a bit clear for one side, isn't one.
    fn any(lua: &Lua, set: &LuaValue) -> mlua::Result<[u32; 2]> {
        let sets = Names::panel(lua, set, "field.any")?;
        if sets.iter().any(|s| s.holds.1 != 0) {
            return Err(mlua::Error::runtime("field.any: a side's area is no bit of a panel's: field.check tests it"));
        }
        Ok(sets.map(|s| s.holds.0))
    }

    /// `words` for the side a call gives: a side's own where they differ by
    /// side, which takes one.
    fn for_side<T: Copy + PartialEq>(words: [T; 2], side: &LuaValue, what: &str) -> mlua::Result<T> {
        match side {
            LuaValue::Nil if words[0] == words[1] => Ok(words[0]),
            LuaValue::Nil => Err(mlua::Error::runtime(format!("{what}: its names are of a side (own_, enemy_, an area): give the side"))),
            side => match int(side, "side")? {
                s @ 0..=1 => Ok(words[s as usize]),
                _ if words[0] == words[1] => Ok(words[0]),
                s => Err(mlua::Error::runtime(format!("{what}: side {s} has no own or enemy side"))),
            },
        }
    }
}

/// A set of names a call takes once (no cache: the call isn't made each
/// tick), its members set to true: a table of booleans keyed by name.
fn name_set(v: &LuaValue, what: &str) -> mlua::Result<Vec<String>> {
    let (_, names) = Names::read(v, what)?;
    names.into_iter().filter(|(_, on)| *on).map(|(n, _)| Ok(n.to_str()?.to_string())).collect()
}

/// A panel type (`PanelType.cracked`): one of the game's (its panel rules'
/// `numbers`), as its number.
fn panel_type_arg(v: &LuaValue, what: &str) -> mlua::Result<u8> {
    let s = crate::names::arg(v, NameKind::PanelType, what)?;
    with(|api, _| Ok(api.panel_type_named(&s)))?
        .ok_or_else(|| mlua::Error::runtime(format!("{what}: PanelType.{s} is none of the game's panel types (its rules' panels.numbers)")))
}

/// A name of the rules' of `kind` (`Button.soul`, `Window.cross_chosen`).
fn name_arg(v: &LuaValue, kind: NameKind, what: &str) -> mlua::Result<std::rc::Rc<str>> {
    crate::names::arg(v, kind, what)
}

/// One of the rules' buttons or windows (`Button.soul`, `Window.cross_chosen`).
fn screen_part_arg(v: &LuaValue, what: &str) -> mlua::Result<(NameKind, std::rc::Rc<str>)> {
    match crate::names::of(v) {
        Some((kind @ (NameKind::Button | NameKind::Window), name)) => Ok((kind, name)),
        _ => Err(mlua::Error::runtime(format!("{what}: expected a Button or a Window (`Button.soul`, `Window.cross_chosen`)"))),
    }
}

fn screen_part(kind: NameKind, name: &str) -> nettai_content_api::api::ScreenPart<'_> {
    match kind {
        NameKind::Button => nettai_content_api::api::ScreenPart::Button(name),
        _ => nettai_content_api::api::ScreenPart::Window(name),
    }
}

fn dimming_lib(lua: &Lua) -> mlua::Result<mlua::Table> {
    let t = lua.create_table()?;
    for &step in DimmingStep::ALL {
        t.set(
            step.name(),
            lua.create_function(move |_, me: mlua::UserDataRef<Object>| with(|api, _| Ok(api.dimming(me.0, step))))?,
        )?;
    }
    lib_fn!(lua, t, "telop_running", |_, (me, chip): (mlua::UserDataRef<Object>, LuaValue)| {
        // The chip the telop names unless the controller's names one (nil:
        // the zeroed chip field's).
        let chip = bound(|b| chip_arg(b, &chip, "dimming.telop_running's chip"))?;
        with(|api, _| Ok(api.dimming_telop_running(me.0, chip)))
    });
    lib_fn!(lua, t, "user", |lua, side: LuaValue| {
        let side = u8_arg(side, "side")?;
        let u = with(|api, _| Ok(api.dimming_user(side)))?;
        object_value(lua, u)
    });
    lib_fn!(lua, t, "run", |_, side: LuaValue| {
        let side = u8_arg(side, "side")?;
        with(|api, _| Ok(api.dimming_run(side)))
    });
    lib_fn!(lua, t, "chip_telop", |_, (side, chip): (LuaValue, LuaValue)| {
        let side = u8_arg(side, "side")?;
        let chip = bound(|b| chip_arg(b, &chip, "dimming.chip_telop's chip"))?;
        with(|api, _| Ok(api.dimming_chip_telop(side, chip)))
    });
    lib_fn!(lua, t, "telop_done", |_, side: LuaValue| {
        let side = u8_arg(side, "side")?;
        with(|api, _| Ok(api.dimming_telop_done(side)))
    });
    lib_fn!(lua, t, "turn", |_, me: mlua::UserDataRef<Object>| with(|api, _| Ok(api.dimming_turn(me.0))));
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
    // (The record type of the look `fly_to_absorber` takes: the engine's own.)
    t.set("absorbed_look", crate::schema::record_type(nettai_content_api::ABSORBED_LOOK).map_err(mlua::Error::runtime)?)?;
    lib_fn!(lua, t, "register", |_, (me, side, class): (Me, LuaValue, LuaValue)| {
        let side = u8_arg(side, "side")? & 1;
        let class = crate::choice::parsed(&class, ChoiceKind::ObstacleClass, "obstacle class", ObstacleClass::from_name)? as u8;
        with(|api, _| Ok(api.obstacle_register(me.0, side, class)))
    });
    lib_fn!(lua, t, "unregister", |_, me: Me| with(|api, _| Ok(api.obstacle_unregister(me.0))));
    lib_fn!(lua, t, "take_hits", |_, (me, push): (Me, Option<LuaValue>)| {
        let push = match push {
            Some(p) => crate::choice::parsed(&p, ChoiceKind::ObstaclePush, "push", ObstaclePush::from_name)?,
            None => ObstaclePush::ForgetsDamage,
        };
        with(|api, _| api.obstacle_take_hits(me.0, push).map_err(api_error))
    });
    lib_fn!(lua, t, "tick_lifetime", |_, me: Me| with(|api, _| api.obstacle_tick_lifetime(me.0).map_err(api_error)));
    lib_fn!(lua, t, "react", |_, (me, crush, hold): (Me, Option<LuaValue>, Option<LuaValue>)| {
        let crush = match crush {
            Some(c) => crate::choice::parsed(&c, ChoiceKind::ObstacleCrush, "crush", ObstacleCrush::from_name)?,
            None => ObstacleCrush::Breaks,
        };
        let hold = match hold {
            Some(h) => crate::choice::parsed(&h, ChoiceKind::ObstacleHold, "dimming hold", ObstacleHold::from_name)?,
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
    lib_fn!(lua, t, "removal", |lua, me: Me| {
        let r = with(|api, _| api.obstacle_removal(me.0).map_err(api_error))?;
        crate::choice::named(lua, ChoiceKind::ObstacleRemoval, r.name())
    });
    lib_fn!(lua, t, "blink_out", |lua, me: Me| {
        let b = with(|api, _| api.obstacle_blink_out(me.0).map_err(api_error))?;
        crate::choice::named(lua, ChoiceKind::BlinkOut, b.name())
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
    lib_fn!(lua, t, "throw", |_, (o, side, x, y, shake, damage): (Me, u8, u8, u8, u8, u32)| {
        with(|api, _| {
            api.obstacle_throw(o.0, side, x, y, shake, damage);
            Ok(())
        })
    });
    for &r in ObstacleRequest::ALL {
        t.set(
            r.name(),
            lua.create_function(move |_, o: Me| with(|api, _| Ok(api.obstacle_request(o.0, r))))?,
        )?;
    }
    Ok(t)
}

/// The damage word's parts, by the original's layout: the damage (the low
/// half), whose low twelve bits are the amount and whose top four the
/// flags; the counter byte above it, its counter strength (bits 16-22) and
/// its can't-counter bit (23); and above that only what a carry leaves.
mod word {
    pub const DAMAGE: u32 = 0xFFFF;
    pub const AMOUNT: u32 = 0x0FFF;
    pub const FLAGS: u32 = 0xF000;
    pub const COUNTER_SHIFT: u32 = 16;
    pub const COUNTER: u32 = 0x7F;
    pub const CANT_COUNTER: u32 = 0x80 << COUNTER_SHIFT;
}

/// `damage_word`: a damage word built and read by its parts' names (the
/// amount, the flags by name, the counter strength and can't-counter); a
/// word content copies whole it passes as one value.
fn damage_word_lib(lua: &Lua) -> mlua::Result<mlua::Table> {
    let t = lua.create_table()?;
    let flags = |v: &LuaValue, what: &str| -> mlua::Result<u32> { with(|api, _| named_bits(api, NamedBits::DamageWord, v, what)) };
    lib_fn!(lua, t, "of", move |_, spec: mlua::Table| {
        const KEYS: [&str; 4] = ["amount", "flags", "counter", "cant_counter"];
        for pair in spec.pairs::<LuaValue, LuaValue>() {
            let (k, _) = pair?;
            if !matches!(&k, LuaValue::String(k) if KEYS.iter().any(|n| k.as_bytes() == n.as_bytes())) {
                return Err(mlua::Error::runtime(format!("damage_word.of: a damage word has no part {k:?} (it has {})", KEYS.join(", "))));
            }
        }
        // (Wrapped to the damage's halfword, as its store is.)
        let amount = table_int(&spec, "amount")? as u16 as u32;
        let counter = match table_int(&spec, "counter")? {
            c @ 0..=0x7F => c as u32,
            c => return Err(mlua::Error::runtime(format!("damage_word.of: counter {c} is past a counter strength's 127"))),
        };
        let cant = spec.raw_get::<Option<bool>>("cant_counter")?.unwrap_or(false);
        let named: LuaValue = spec.raw_get("flags")?;
        let named = if named.is_nil() { 0 } else { flags(&named, "damage_word.of: flags")? };
        Ok(amount | named | counter << word::COUNTER_SHIFT | if cant { word::CANT_COUNTER } else { 0 })
    });
    lib_fn!(lua, t, "damage", |_, w: LuaValue| Ok(word_arg(&w, "damage_word.damage")? & word::DAMAGE));
    lib_fn!(lua, t, "amount", |_, w: LuaValue| Ok(word_arg(&w, "damage_word.amount")? & word::AMOUNT));
    lib_fn!(lua, t, "read_amount", |_, w: LuaValue| {
        let w = word_arg(&w, "damage_word.read_amount")?;
        with(|api, _| Ok(w & api.damage_amount_bits() as u32))
    });
    lib_fn!(lua, t, "flag_word", |_, w: LuaValue| Ok(word_arg(&w, "damage_word.flag_word")? & word::FLAGS));
    lib_fn!(lua, t, "flags", |_, w: LuaValue| Ok(BitsView(NamedBits::DamageWord, word_arg(&w, "damage_word.flags")?)));
    lib_fn!(lua, t, "high_half", |_, w: LuaValue| Ok(word_arg(&w, "damage_word.high_half")? >> word::COUNTER_SHIFT));
    lib_fn!(lua, t, "counter", |_, w: LuaValue| Ok(word_arg(&w, "damage_word.counter")? >> word::COUNTER_SHIFT & word::COUNTER));
    lib_fn!(lua, t, "cant_counter", |_, w: LuaValue| Ok(word_arg(&w, "damage_word.cant_counter")? & word::CANT_COUNTER != 0));
    lib_fn!(lua, t, "with", move |_, (w, names): (LuaValue, LuaValue)| {
        Ok(word_arg(&w, "damage_word.with")? | flags(&names, "damage_word.with")?)
    });
    lib_fn!(lua, t, "without", move |_, (w, names): (LuaValue, LuaValue)| {
        Ok(word_arg(&w, "damage_word.without")? & !flags(&names, "damage_word.without")?)
    });
    lib_fn!(lua, t, "plus", |_, (w, n): (LuaValue, LuaValue)| {
        let w = word_arg(&w, "damage_word.plus")?;
        let n = int(&n, "damage_word.plus: bonus")?;
        Ok((w as i64 + n).rem_euclid(1 << 32) as u32)
    });
    lib_fn!(lua, t, "plus_damage", |_, (w, n): (LuaValue, LuaValue)| {
        let w = word_arg(&w, "damage_word.plus_damage")?;
        let n = int(&n, "damage_word.plus_damage: bonus")?;
        // (A halfword's add: it wraps, nothing carries.)
        let d = ((w & word::DAMAGE) as i64 + n) as u16;
        Ok(w & !word::DAMAGE | d as u32)
    });
    lib_fn!(lua, t, "with_damage", |_, (w, d): (LuaValue, LuaValue)| {
        let w = word_arg(&w, "damage_word.with_damage")?;
        // (Wrapped to the damage's halfword, as its store is.)
        let d = int(&d, "damage_word.with_damage: damage")? as u16;
        Ok(w & !word::DAMAGE | d as u32)
    });
    Ok(t)
}

/// `hit_modifier`: a hit modifier's byte, to keep in a state, and a view of
/// its bits by name.
fn hit_modifier_lib(lua: &Lua) -> mlua::Result<mlua::Table> {
    let t = lua.create_table()?;
    lib_fn!(lua, t, "byte", |_, m: LuaValue| with(|api, _| hit_modifier_arg(api, &m, "hit_modifier.byte")));
    lib_fn!(lua, t, "bits", |_, m: LuaValue| Ok(BitsView(NamedBits::HitModifier, with(|api, _| hit_modifier_arg(api, &m, "hit_modifier.bits"))? as u32)));
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
pub fn object(lua: &Lua, bound: &Bound, o: ObjectRef) -> mlua::Result<AnyUserData> {
    Handles::get_in(lua, bound, Handle::Object(o))
}

/// Wrap the attack state of `o`, as a state of layout `state` (the action
/// it runs), as a script value.
pub fn action_state(lua: &Lua, o: ObjectRef, state: StateId) -> mlua::Result<AnyUserData> {
    lua.create_userdata(State(StateOf::ActionAs(o, state)))
}

/// A hook call's arguments.
pub fn hook_args(lua: &Lua, call: HookCall, bound: &Bound) -> mlua::Result<mlua::MultiValue> {
    let obj = |o: ObjectRef| -> mlua::Result<LuaValue> { Ok(LuaValue::UserData(Handles::get_in(lua, bound, Handle::Object(o))?)) };
    let values = match call {
        HookCall::Weapon { navi } => vec![obj(navi)?],
        HookCall::DimmingChip { user, spec } => {
            let t = lua.create_table()?;
            t.raw_set("element", crate::element::value(lua, spec.element)?)?;
            t.raw_set("damage", spec.damage)?;
            t.raw_set("chip", chip_value(bound, spec.chip)?)?;
            t.raw_set("bonus", spec.bonus)?;
            vec![obj(user)?, LuaValue::Table(t)]
        }
        HookCall::InstantChip { user, spec } => {
            let t = lua.create_table()?;
            t.raw_set("panel_x", spec.panel.x)?;
            t.raw_set("panel_y", spec.panel.y)?;
            t.raw_set("element", crate::element::value(lua, spec.element)?)?;
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
            if let Some(hp) = spec.hp {
                t.raw_set("hp", hp)?;
            }
            vec![LuaValue::Table(t)]
        }
        HookCall::RoleNavi { navi } | HookCall::FormNavi { navi } => vec![obj(navi)?],
        // A panel type's: the body, then `burn`'s whether it is a player's
        // navi, `slide`'s what reaches the panel, `hit`'s the element; or the
        // panel.
        HookCall::Panel(PanelCall::Burn { body, player }) => vec![obj(body)?, LuaValue::Boolean(player)],
        HookCall::Panel(PanelCall::Stand { body } | PanelCall::Rest { body } | PanelCall::MoveEnd { body }) => vec![obj(body)?],
        HookCall::Panel(PanelCall::Slide { body, how }) => vec![obj(body)?, crate::choice::value_of(lua, ChoiceKind::SlideHow, how as u8)?],
        HookCall::Panel(PanelCall::Hit { body, element }) => vec![obj(body)?, crate::element::primary_value(lua, element)?],
        HookCall::Panel(PanelCall::Tick { x, y } | PanelCall::Changed { x, y } | PanelCall::Start { x, y }) => {
            vec![LuaValue::Integer(mlua::Integer::from(x)), LuaValue::Integer(mlua::Integer::from(y))]
        }
        HookCall::Given { side, chip } => {
            let chip = match chip {
                Some(c) => LuaValue::Table(bound.def_value(Registry::Chip, c.0)?),
                None => LuaValue::Nil,
            };
            vec![LuaValue::Integer(mlua::Integer::from(side)), chip]
        }
        HookCall::Encased { obstacle } => vec![obj(obstacle)?],
        HookCall::RoleEncased { obstacle, ice, class } => {
            let class = match class {
                None => LuaValue::Nil,
                Some(c) => {
                    if c as usize >= ObstacleClass::NAMES.len() {
                        return Err(mlua::Error::runtime(format!("obstacle class {c}")));
                    }
                    crate::choice::value_of(lua, ChoiceKind::ObstacleClass, c)?
                }
            };
            vec![obj(obstacle)?, LuaValue::Boolean(ice), class]
        }
        // The side, then the navi, the chip and the weapon, where the hook
        // has them (nil in between).
        // A custom screen's form list's choice: the side, then the entry.
        HookCall::Rules { side, hook: RulesHook::CustomChooseForm, entry, .. } => vec![
            LuaValue::Integer(mlua::Integer::from(side)),
            entry.map_or(LuaValue::Nil, |e| LuaValue::Integer(mlua::Integer::from(e))),
        ],
        // A custom screen's chip hooks: the side, then the chip, and a
        // take-back's place among the picks.
        HookCall::Rules { side, hook: RulesHook::CustomChipPicked | RulesHook::CustomChipTakenBack, chip, place, .. } => {
            let mut v = vec![
                LuaValue::Integer(mlua::Integer::from(side)),
                chip.map_or(Ok(LuaValue::Nil), |c| bound.def_value(Registry::Chip, c.0).map(LuaValue::Table))?,
            ];
            if let Some(k) = place {
                v.push(LuaValue::Integer(mlua::Integer::from(k)));
            }
            v
        }
        // A button's take-back: the side, then the place among the picks.
        HookCall::Rules { side, hook: RulesHook::ButtonTakenBack, place, .. } => vec![
            LuaValue::Integer(mlua::Integer::from(side)),
            place.map_or(LuaValue::Nil, |k| LuaValue::Integer(mlua::Integer::from(k))),
        ],
        // A party's hooks: the side, the navi where they have one, then the
        // member and the mood.
        HookCall::Rules {
            side,
            hook: RulesHook::SwitchRequested | RulesHook::MemberBusy | RulesHook::SwitchAsked | RulesHook::NaviSwitched,
            navi,
            member,
            mood,
            ..
        } => {
            let n = |x: Option<u8>| x.map_or(LuaValue::Nil, |x| LuaValue::Integer(mlua::Integer::from(x)));
            let mut v = vec![LuaValue::Integer(mlua::Integer::from(side))];
            if let Some(o) = navi {
                v.push(obj(o)?);
            }
            v.push(n(member));
            v.push(match mood {
                Some(m) => show(lua, Value::Int(i64::from(m)), &FieldType::U8, Shown::Mood)?,
                None => LuaValue::Nil,
            });
            while v.len() > 1 && v.last().is_some_and(LuaValue::is_nil) {
                v.pop();
            }
            v
        }
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
    Ok(mlua::MultiValue::from_vec(values))
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
        HookCall::DimmingChip { .. } | HookCall::Place { .. } => {
            Ok(object_arg(&v, "the object a spawner returns")?.map_or(Value::Nil, Value::Object))
        }
        // A navi's role hook may hand back an object (`navi_deleted`'s).
        HookCall::RoleNavi { .. } => Ok(object_arg(&v, "the object a role hook returns")?.map_or(Value::Nil, Value::Object)),
        // A panel type's `rest`: whether it handled the body.
        HookCall::Panel(PanelCall::Rest { .. }) => match v {
            LuaValue::Boolean(b) => Ok(Value::Bool(b)),
            LuaValue::Nil => Ok(Value::Bool(false)),
            _ => Err(mlua::Error::runtime(format!("a panel type's `rest` returns whether it handled the body, not a {}", v.type_name()))),
        },
        // Its `slide`: an answer's number (1 up), nil none.
        HookCall::Panel(PanelCall::Slide { .. }) => match &v {
            LuaValue::Nil => Ok(Value::Nil),
            v => Ok(Value::Int(i64::from(crate::choice::arg(v, ChoiceKind::SlideAnswer, "a panel type's `slide`'s answer")?.index) + 1)),
        },
        HookCall::InstantChip { .. }
        | HookCall::RoleEncased { .. }
        | HookCall::Encased { .. }
        | HookCall::FormNavi { .. }
        | HookCall::Panel(_) => Ok(Value::Nil),
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
        // (`ControlOutcome`'s members are in the original's order.)
        HookCall::Rules { hook: RulesHook::Controller | RulesHook::Takeover, .. } => match &v {
            LuaValue::Nil => Ok(Value::Nil),
            v => Ok(Value::Int(i64::from(crate::choice::arg(v, ChoiceKind::ControlOutcome, "a controller's outcome")?.index))),
        },
        // A button's `shown` and `state`, a window's `update`, and whether
        // the rules took something back.
        HookCall::Rules {
            hook:
                RulesHook::ButtonShown
                | RulesHook::WindowUpdate
                | RulesHook::CustomUnchooseForm
                | RulesHook::BugMark
                | RulesHook::HpEmptied
                | RulesHook::NaviFalling
                | RulesHook::MemberBusy
                | RulesHook::OpponentChip
                | RulesHook::LowHp
                | RulesHook::HitGuard
                | RulesHook::CutInBarred,
            ..
        } => Ok(Value::Bool(v == LuaValue::Boolean(true))),
        // What the rules did with a hit's NaviCust bug: nothing of the
        // stats, edited them, or spared the navi the bug and the reload.
        // (Edited 1, spared 2.)
        HookCall::Rules { hook: RulesHook::NaviBug, .. } => match &v {
            LuaValue::Nil => Ok(Value::Nil),
            v => Ok(Value::Int(i64::from(crate::choice::arg(v, ChoiceKind::BugOutcome, "navi_bug's answer")?.index) + 1)),
        },
        // A button's chip, and the chip it puts in the hand.
        HookCall::Rules { hook: RulesHook::ButtonChip | RulesHook::ButtonHandChip, .. } => match bound.def(&v) {
            _ if v.is_nil() => Ok(Value::Nil),
            Some((Registry::Chip, h)) => Ok(Value::Def(Registry::Chip, h)),
            _ => Err(mlua::Error::runtime(format!("a button's chip is nil or a chip definition, not a {}", v.type_name()))),
        },
        // (Selectable 0, unavailable 1; a button's `state` answers no other.)
        HookCall::Rules { hook: RulesHook::ButtonState, .. } => match &v {
            LuaValue::Nil => Ok(Value::Nil),
            v => match crate::choice::arg(v, ChoiceKind::ButtonState, "a button's state")? {
                c @ nettai_content_api::Choice { index: 0 | 1, .. } => Ok(Value::Int(i64::from(c.index))),
                c => Err(mlua::Error::runtime(format!("a button's state is ButtonState.selectable or ButtonState.unavailable, not {c}"))),
            },
        },
        // A mood (a byte, or `Mood.full_synchro`: the byte 0xFF).
        HookCall::Rules { hook: RulesHook::StartingMood, .. } => match &v {
            LuaValue::Nil => Ok(Value::Nil),
            LuaValue::UserData(_) => take(v.clone(), &FieldType::U8, Shown::Mood, "starting_mood"),
            LuaValue::Integer(n) if (0..0xFF).contains(n) => Ok(Value::Int(i64::from(*n))),
            LuaValue::Number(n) if n.fract() == 0.0 && (0.0..255.0).contains(n) => Ok(Value::Int(*n as i64)),
            _ => Err(mlua::Error::runtime(format!("starting_mood returns a mood (a byte below 0xff, or Mood.full_synchro) or nil, not {v:?}"))),
        },
        // A palette.
        HookCall::Rules { hook: hook @ RulesHook::NaviPalette, .. } => match &v {
            LuaValue::Nil => Ok(Value::Nil),
            LuaValue::Integer(n) if (0..=255).contains(n) => Ok(Value::Int(i64::from(*n))),
            LuaValue::Number(n) if n.fract() == 0.0 && (0.0..=255.0).contains(n) => Ok(Value::Int(*n as i64)),
            _ => Err(mlua::Error::runtime(format!("{} returns a byte or nil, not {v:?}", hook.name()))),
        },
        // A color shader (15 bits).
        HookCall::Rules { hook: RulesHook::NaviShader, .. } => match &v {
            LuaValue::Nil => Ok(Value::Nil),
            LuaValue::Integer(n) if (0..=0x7FFF).contains(n) => Ok(Value::Int(i64::from(*n))),
            LuaValue::Number(n) if n.fract() == 0.0 && (0.0..=32767.0).contains(n) => Ok(Value::Int(*n as i64)),
            _ => Err(mlua::Error::runtime(format!("navi_shader returns a 15-bit color or nil, not {v:?}"))),
        },
        // A hand size.
        HookCall::Rules { hook: RulesHook::CustomHandSize, .. } => match &v {
            LuaValue::Nil => Ok(Value::Nil),
            LuaValue::Integer(n) if (0..=255).contains(n) => Ok(Value::Int(i64::from(*n))),
            LuaValue::Number(n) if n.fract() == 0.0 && (0.0..=255.0).contains(n) => Ok(Value::Int(*n as i64)),
            _ => Err(mlua::Error::runtime(format!("custom.hand_size returns a count of chips, not {v:?}"))),
        },
        // The ticks a party battle's custom screen is held (none: 0).
        HookCall::Rules { hook: RulesHook::CustomHold, .. } => match &v {
            LuaValue::Nil => Ok(Value::Nil),
            LuaValue::Integer(n) if (0..=0x7FFF).contains(n) => Ok(Value::Int(i64::from(*n))),
            LuaValue::Number(n) if n.fract() == 0.0 && (0.0..=32767.0).contains(n) => Ok(Value::Int(*n as i64)),
            _ => Err(mlua::Error::runtime(format!("custom_hold returns ticks or nil, not {v:?}"))),
        },
        // Whether a switch request stands: an answer of false refuses it.
        HookCall::Rules { hook: RulesHook::SwitchRequested | RulesHook::SwitchAsked, .. } => match &v {
            LuaValue::Nil => Ok(Value::Nil),
            LuaValue::Boolean(b) => Ok(Value::Bool(*b)),
            _ => Err(mlua::Error::runtime(format!("a switch's hook returns a boolean or nil, not a {}", v.type_name()))),
        },
        HookCall::Rules { .. } => Ok(Value::Nil),
    }
}

/// A module's value as plain data (`ContentHost::module_data`): a flag, a
/// whole number, a string, a definition (by its key), a list, a table by
/// keys (in key order); a function and a state's field type (`schema.u8`)
/// left out.
pub fn plain_data(v: &LuaValue, bound: &Bound, depth: u32) -> mlua::Result<nettai_content_api::Data> {
    use nettai_content_api::{Data, DataKey};
    if depth > 16 {
        return Err(mlua::Error::runtime("data nested past 16 tables"));
    }
    Ok(match v {
        LuaValue::Nil | LuaValue::Function(_) => Data::Nil,
        _ if crate::schema::of(v).is_some() => Data::Nil,
        LuaValue::Boolean(b) => Data::Bool(*b),
        LuaValue::Integer(i) => Data::Int(lua_int(*i)),
        LuaValue::Number(n) if n.fract() == 0.0 && n.abs() < 9.0e15 => Data::Int(*n as i64),
        LuaValue::String(s) => Data::Str(s.to_str()?.to_string()),
        LuaValue::UserData(_) if let Some(c) = crate::choice::of(v) => Data::Choice(c),
        LuaValue::Table(t) => {
            if let Some((r, h)) = bound.def(v) {
                let key = bound.key(r, h).ok_or_else(|| mlua::Error::runtime(format!("{r} {h} has no key")))?;
                return Ok(Data::Ref(r, key.to_string()));
            }
            let n = t.raw_len();
            let mut entries = Vec::new();
            for pair in t.clone().pairs::<LuaValue, LuaValue>() {
                let (k, x) = pair?;
                if matches!(x, LuaValue::Function(_)) || crate::schema::of(&x).is_some() {
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
        // (A name a game defines is its kind's; an element the byte it is.)
        _ if let Some((kind, name)) = crate::names::of(v) => Data::Name(kind, name.to_string()),
        _ if let Some(b) = crate::element::of(v) => Data::Int(i64::from(b)),
        other => return Err(mlua::Error::runtime(format!("no data is a {}", other.type_name()))),
    })
}

#[cfg(test)]
mod field_type_tests {
    use super::*;

    /// The types made once are each field's own: a variant's number is its
    /// place in `ALL`.
    #[test]
    fn a_field_type_is_its_fields() {
        fn check<T: FieldTypes + std::fmt::Debug>() {
            for (i, &f) in T::all().iter().enumerate() {
                assert_eq!(f.index(), i, "{f:?}");
                assert_eq!(*f.field_type(), f.made(), "{f:?}");
            }
        }
        check::<ObjectField>();
        check::<ActorField>();
        check::<OverlayField>();
        check::<SpriteField>();
        check::<CollisionField>();
        check::<NaviStat>();
        check::<BattleInfo>();
        check::<HudField>();
        check::<HandField>();
    }
}

