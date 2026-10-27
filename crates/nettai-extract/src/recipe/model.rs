//! The codec (docs/design/extraction-recipes.md §3.10): what a recipe
//! returns, read: its table (§4.2: its ROMs and its function,
//! [`declaration`]), and the table its function returns, read into the
//! model: `nettai_assets::Bundle` (the graphics), the asset names and the
//! sound's plan.
//!
//! The tables use the model's own field names. An unknown field, or a value
//! of the wrong kind, is an error naming its path
//! (`graphics.hud.chatbox.boxes[2][1]: expected Map, got table`); a field
//! left out takes the model's default. Lists are arrays (from 1, as Luau
//! counts in a path); a list with holes is a table keyed by the game's
//! number from 0 with its length, `n` (`{ n = 22, [0x00] = ..., [0x05] =
//! ... }`); a list of named things (`buttons`, `languages`, a `Versioned`'s
//! versions) is a list of `{ name = ..., value = ... }`; a tuple is an
//! array (a place `{ x, y }`); an enum is its variant's name
//! (`sets = "other"`). Keyed maps go into `BTreeMap`s, so the order Luau
//! iterates them in never reaches a file. Bulk data is the primitives'
//! handles (`Tiles`, `Map`, `Sheet`, `GfxAnim`, `DialogueFont`); a palette
//! is sixteen numbers, a map entry `{ tile, hflip, vflip, palette }` (a
//! `Map` stands for a list of them).

use super::prims::{FontHandle, GfxAnimHandle, MapHandle, RomHandle, SheetHandle, TilesHandle};
use crate::rom::Rom;
use nettai_assets::*;
use nettai_content::names::AssetNames;
use nettai_luau::mlua::{Function, Table, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

type R<T> = Result<T, String>;

/// What a recipe returns, read.
pub(crate) struct Extracted {
    pub names: AssetNames,
    pub graphics: Bundle,
    pub sound: SoundPlan,
}

/// Which ROM the sound bank is read from, and which ROMs are versions of it
/// (§3.8).
pub(crate) struct SoundPlan {
    pub from: (String, Arc<Rom>),
    pub base_version: String,
    pub versions: Vec<(String, (String, Arc<Rom>))>,
}

/// What a value is, for messages.
fn kind(v: &Value) -> String {
    match v {
        Value::UserData(u) => {
            let names: [(&str, bool); 8] = [
                ("Tiles", u.is::<TilesHandle>()),
                ("Map", u.is::<MapHandle>()),
                ("Sheet", u.is::<SheetHandle>()),
                ("GfxAnim", u.is::<GfxAnimHandle>()),
                ("DialogueFont", u.is::<FontHandle>()),
                ("Rom", u.is::<RomHandle>()),
                ("Bytes", u.is::<super::prims::Bytes>()),
                ("userdata", true),
            ];
            names.into_iter().find(|(_, is)| *is).map_or("userdata", |(n, _)| n).to_string()
        }
        v => v.type_name().to_string(),
    }
}

fn expected<T>(path: &str, what: &str, v: &Value) -> R<T> {
    Err(format!("{path}: expected {what}, got {}", kind(v)))
}

// ---- Values ----------------------------------------------------------------------------------

/// An integer in `T`'s range.
fn int<T: TryFrom<i64> + Into<i64> + Copy + Bounds>(v: &Value, path: &str) -> R<T> {
    let n = match v {
        Value::Integer(i) => *i as f64,
        Value::Number(n) => *n,
        v => return expected(path, "a number", v),
    };
    if n.fract() != 0.0 || !n.is_finite() || n < T::MIN as f64 || n > T::MAX as f64 {
        return Err(format!("{path}: expected an integer from {} to {}, got {n}", T::MIN, T::MAX));
    }
    T::try_from(n as i64).map_err(|_| format!("{path}: {n} is out of range"))
}

/// The range of an integer type, for [`int`].
trait Bounds {
    const MIN: i64;
    const MAX: i64;
}
macro_rules! bounds {
    ($($t:ty),*) => {$(impl Bounds for $t { const MIN: i64 = <$t>::MIN as i64; const MAX: i64 = <$t>::MAX as i64; })*};
}
bounds!(u8, i8, u16, i16, u32, i32);

fn boolean(v: &Value, path: &str) -> R<bool> {
    match v {
        Value::Boolean(b) => Ok(*b),
        v => expected(path, "a boolean", v),
    }
}

fn string(v: &Value, path: &str) -> R<String> {
    match v {
        Value::String(s) => s.to_str().map(|s| s.to_string()).map_err(|_| format!("{path}: a string that isn't UTF-8")),
        v => expected(path, "a string", v),
    }
}

/// A handle of type `H`, its value cloned.
fn handle<H: 'static, T>(v: &Value, path: &str, what: &str, get: impl Fn(&H) -> T) -> R<T> {
    match v {
        Value::UserData(u) => match u.borrow::<H>() {
            Ok(h) => Ok(get(&h)),
            Err(_) => expected(path, what, v),
        },
        v => expected(path, what, v),
    }
}

fn tiles(v: &Value, path: &str) -> R<Tiles> {
    handle(v, path, "Tiles", |t: &TilesHandle| t.0.clone())
}

fn sheet(v: &Value, path: &str) -> R<SpriteSheet> {
    handle(v, path, "Sheet", |s: &SheetHandle| s.0.clone())
}

fn gfx_anim(v: &Value, path: &str) -> R<GfxAnim> {
    handle(v, path, "GfxAnim", |a: &GfxAnimHandle| a.0.clone())
}

fn dialogue_font(v: &Value, path: &str) -> R<DialogueFont> {
    handle(v, path, "DialogueFont", |f: &FontHandle| f.0.clone())
}

fn rom(v: &Value, path: &str) -> R<(String, Arc<Rom>)> {
    handle(v, path, "Rom", |r: &RomHandle| (r.code.clone(), r.rom.clone()))
}

fn palette(v: &Value, path: &str) -> R<Palette> {
    let list = list(v, path, int::<u16>)?;
    list.try_into().map_err(|l: Vec<u16>| format!("{path}: a palette is 16 colors, not {}", l.len()))
}

fn map_entry(v: &Value, path: &str) -> R<MapEntry> {
    let mut e = MapEntry::default();
    let mut f = Fields::new(v, path)?;
    f.set("tile", &mut e.tile, int)?;
    f.set("hflip", &mut e.hflip, boolean)?;
    f.set("vflip", &mut e.vflip, boolean)?;
    f.set("palette", &mut e.palette, int)?;
    f.finish()?;
    Ok(e)
}

/// Map entries: a `Map`, or a list of entries.
fn map(v: &Value, path: &str) -> R<Vec<MapEntry>> {
    match v {
        Value::UserData(u) if u.is::<MapHandle>() => handle(v, path, "Map", |m: &MapHandle| m.0.clone()),
        Value::Table(_) => list(v, path, map_entry),
        v => expected(path, "Map", v),
    }
}

/// `N` map entries.
fn map_of<const N: usize>(v: &Value, path: &str) -> R<[MapEntry; N]> {
    map(v, path)?.try_into().map_err(|m: Vec<MapEntry>| format!("{path}: {N} map entries, not {}", m.len()))
}

// ---- Tables ----------------------------------------------------------------------------------

/// A table's fields read one by one: each named is taken, and any left is
/// an error ([`Fields::finish`]).
struct Fields {
    table: Table,
    path: String,
    left: BTreeSet<String>,
}

impl Fields {
    fn new(v: &Value, path: &str) -> R<Fields> {
        let Value::Table(t) = v else { return expected(path, "a table", v) };
        let mut left = BTreeSet::new();
        for pair in t.pairs::<Value, Value>() {
            let (k, _) = pair.map_err(|e| format!("{path}: {e}"))?;
            match k {
                Value::String(s) => {
                    left.insert(s.to_str().map_err(|_| format!("{path}: a key that isn't UTF-8"))?.to_string());
                }
                k => return Err(format!("{path}: a field named by a {} (the model's fields are named)", kind(&k))),
            }
        }
        Ok(Fields { table: t.clone(), path: path.to_string(), left })
    }

    /// Field `key`'s value and path, if it is given.
    fn take(&mut self, key: &str) -> R<Option<(Value, String)>> {
        self.left.remove(key);
        let v: Value = self.table.raw_get(key).map_err(|e| format!("{}.{key}: {e}", self.path))?;
        Ok((!v.is_nil()).then(|| (v, format!("{}.{key}", self.path))))
    }

    /// Field `key` read into `out`, if it is given.
    fn set<T>(&mut self, key: &str, out: &mut T, read: impl Fn(&Value, &str) -> R<T>) -> R<()> {
        if let Some((v, path)) = self.take(key)? {
            *out = read(&v, &path)?;
        }
        Ok(())
    }

    /// Field `key` read into an optional `out`, if it is given.
    fn opt<T>(&mut self, key: &str, out: &mut Option<T>, read: impl Fn(&Value, &str) -> R<T>) -> R<()> {
        if let Some((v, path)) = self.take(key)? {
            *out = Some(read(&v, &path)?);
        }
        Ok(())
    }

    fn finish(self) -> R<()> {
        match self.left.into_iter().next() {
            Some(k) => Err(format!("{}.{k}: no such field", self.path)),
            None => Ok(()),
        }
    }
}

/// An array's entries, from 1.
fn list<T>(v: &Value, path: &str, read: impl Fn(&Value, &str) -> R<T>) -> R<Vec<T>> {
    let Value::Table(t) = v else { return expected(path, "a list", v) };
    let n = t.raw_len();
    let mut count = 0;
    for pair in t.pairs::<Value, Value>() {
        let (k, _) = pair.map_err(|e| format!("{path}: {e}"))?;
        let at = match k {
            Value::Integer(i) => Some(i as i64),
            Value::Number(f) if f.fract() == 0.0 => Some(f as i64),
            _ => None,
        };
        if !at.is_some_and(|i| (1..=n as i64).contains(&i)) {
            return Err(format!("{path}: a list's keys are 1 to its length; it has {}", key_text(&k)));
        }
        count += 1;
    }
    if count != n {
        return Err(format!("{path}: a list with holes"));
    }
    (1..=n).map(|i| read(&t.raw_get::<Value>(i).map_err(|e| e.to_string())?, &format!("{path}[{i}]"))).collect()
}

fn key_text(k: &Value) -> String {
    match k {
        Value::String(s) => format!("{:?}", s.to_string_lossy()),
        Value::Integer(i) => i.to_string(),
        Value::Number(n) => n.to_string(),
        k => kind(k),
    }
}

/// A list `N` long.
fn array<T, const N: usize>(v: &Value, path: &str, read: impl Fn(&Value, &str) -> R<T>) -> R<[T; N]> {
    list(v, path, read)?.try_into().map_err(|l: Vec<T>| format!("{path}: {N} entries, not {}", l.len()))
}

/// A list with holes: keyed by number from 0, its length `n`.
fn holey<T>(v: &Value, path: &str, read: impl Fn(&Value, &str) -> R<T>) -> R<Vec<Option<T>>> {
    let Value::Table(t) = v else { return expected(path, "a table keyed by number from 0, with its length n", v) };
    let n: usize = int::<u16>(&t.raw_get::<Value>("n").map_err(|e| e.to_string())?, &format!("{path}.n"))? as usize;
    let mut out: Vec<Option<T>> = (0..n).map(|_| None).collect();
    for pair in t.pairs::<Value, Value>() {
        let (k, x) = pair.map_err(|e| format!("{path}: {e}"))?;
        if matches!(&k, Value::String(s) if s == "n") {
            continue;
        }
        let i = match k {
            Value::Integer(i) => i as i64,
            Value::Number(f) if f.fract() == 0.0 => f as i64,
            k => return Err(format!("{path}: keyed by number from 0, with its length n; it has {}", key_text(&k))),
        };
        if !(0..n as i64).contains(&i) {
            return Err(format!("{path}: [{i:#x}] is past its length, n = {n}"));
        }
        out[i as usize] = Some(read(&x, &format!("{path}[{i:#x}]"))?);
    }
    Ok(out)
}

/// A table keyed by number.
fn by_number<K: Ord + TryFrom<i64> + Into<i64> + Copy + Bounds, T>(v: &Value, path: &str, read: impl Fn(&Value, &str) -> R<T>) -> R<BTreeMap<K, T>> {
    let Value::Table(t) = v else { return expected(path, "a table keyed by number", v) };
    let mut out = BTreeMap::new();
    for pair in t.pairs::<Value, Value>() {
        let (k, x) = pair.map_err(|e| format!("{path}: {e}"))?;
        let key: K = int(&k, &format!("{path}: a key"))?;
        out.insert(key, read(&x, &format!("{path}[{:#x}]", key.into()))?);
    }
    Ok(out)
}

/// A table keyed by name.
fn by_name<T>(v: &Value, path: &str, read: impl Fn(&Value, &str) -> R<T>) -> R<BTreeMap<String, T>> {
    let Value::Table(t) = v else { return expected(path, "a table keyed by name", v) };
    let mut out = BTreeMap::new();
    for pair in t.pairs::<Value, Value>() {
        let (k, x) = pair.map_err(|e| format!("{path}: {e}"))?;
        let key = string(&k, &format!("{path}: a key"))?;
        let at = format!("{path}.{key}");
        out.insert(key, read(&x, &at)?);
    }
    Ok(out)
}

/// A list of named things, `{ name = ..., value = ... }` each, in order.
fn named<T>(v: &Value, path: &str, read: impl Fn(&Value, &str) -> R<T>) -> R<Vec<(String, T)>> {
    list(v, path, |e, at| {
        let mut f = Fields::new(e, at)?;
        let name = f.take("name")?.map(|(v, p)| string(&v, &p)).transpose()?.ok_or_else(|| format!("{at}.name: missing"))?;
        let value = f.take("value")?.map(|(v, p)| read(&v, &p)).transpose()?.ok_or_else(|| format!("{at}.value: missing"))?;
        f.finish()?;
        Ok((name, value))
    })
}

/// A pair `{ a, b }`.
fn pair<A, B>(v: &Value, path: &str, a: impl Fn(&Value, &str) -> R<A>, b: impl Fn(&Value, &str) -> R<B>) -> R<(A, B)> {
    let [x, y] = array(v, path, |v, _| Ok(v.clone()))?;
    Ok((a(&x, &format!("{path}[1]"))?, b(&y, &format!("{path}[2]"))?))
}

/// A record read field by field, from the type's default.
fn record<T: Default>(v: &Value, path: &str, fields: impl FnOnce(&mut Fields, &mut T) -> R<()>) -> R<T> {
    let mut out = T::default();
    let mut f = Fields::new(v, path)?;
    fields(&mut f, &mut out)?;
    f.finish()?;
    Ok(out)
}

// ---- What a recipe returns -------------------------------------------------------------------

/// The table a recipe returns.
/// What a recipe's `extract/init.luau` returns (§4.2): the game's ROMs by
/// their header codes, every one required, in the order its places read an
/// address by (a ROM's slot, from 1), and the function from the set of them
/// to what it extracts ([`extracted`]).
pub(crate) struct Declaration {
    pub roms: Vec<String>,
    pub extract: Function,
}

/// The table a recipe's `extract/init.luau` returns, read: `roms` and
/// `extract`, nothing else.
pub(crate) fn declaration(v: &Value) -> R<Declaration> {
    let path = "the recipe's table";
    let mut f = Fields::new(v, path)?;
    let (roms, at) = f.take("roms")?.ok_or_else(|| format!("{path}: no roms"))?;
    let (extract, extract_at) = f.take("extract")?.ok_or_else(|| format!("{path}: no extract"))?;
    f.finish()?;
    let roms = list(&roms, &at, string)?;
    if roms.is_empty() {
        return Err(format!("{at}: no ROM"));
    }
    for (i, code) in roms.iter().enumerate() {
        if code.len() != 4 || !code.bytes().all(|b| b.is_ascii_alphanumeric()) {
            return Err(format!("{at}[{}]: {code:?} isn't a game code (four letters and digits, a header's at 0xAC)", i + 1));
        }
        if roms[..i].contains(code) {
            return Err(format!("{at}[{}]: {code} twice", i + 1));
        }
    }
    let Value::Function(extract) = extract else { return expected(&extract_at, "a function", &extract) };
    Ok(Declaration { roms, extract })
}

pub(crate) fn extracted(v: &Value) -> R<Extracted> {
    let mut f = Fields::new(v, "the recipe's result")?;
    let names = f.take("names")?.ok_or("the recipe's result: no names")?;
    let graphics = f.take("graphics")?.ok_or("the recipe's result: no graphics")?;
    let sound = f.take("sound")?.ok_or("the recipe's result: no sound")?;
    f.finish()?;
    Ok(Extracted { names: asset_names(&names.0, "names")?, graphics: bundle(&graphics.0, "graphics")?, sound: sound_plan(&sound.0, "sound")? })
}

fn asset_names(v: &Value, path: &str) -> R<AssetNames> {
    record(v, path, |f, n: &mut AssetNames| {
        if let Some((v, p)) = f.take("sprites")? {
            for (category, index) in by_number::<u8, _>(&v, &p, |v, p| by_number::<u8, _>(v, p, string))? {
                for (index, name) in index {
                    n.sprites.insert((category, index), name);
                }
            }
        }
        f.set("songs", &mut n.songs, |v, p| by_number(v, p, string))?;
        f.set("backgrounds", &mut n.backgrounds, |v, p| by_number(v, p, string))?;
        f.set("mugshots", &mut n.mugshots, |v, p| by_number(v, p, string))?;
        f.set("banners", &mut n.banners, |v, p| by_number(v, p, string))?;
        f.set("chips", &mut n.chips, |v, p| by_number(v, p, string))?;
        f.set("navis", &mut n.navis, |v, p| by_number(v, p, string))?;
        f.set("glyphs", &mut n.glyphs, |v, p| list(v, p, string))?;
        f.set("dialogue_glyphs", &mut n.dialogue_glyphs, |v, p| list(v, p, string))?;
        f.set("language_glyphs", &mut n.language_glyphs, |v, p| {
            by_name(v, p, |v, p| {
                let l: (Vec<String>, Vec<String>) = record(v, p, |f, out: &mut (Vec<String>, Vec<String>)| {
                    f.set("glyphs", &mut out.0, |v, p| list(v, p, string))?;
                    f.set("dialogue_glyphs", &mut out.1, |v, p| list(v, p, string))
                })?;
                Ok(l)
            })
        })
    })
}

fn sound_plan(v: &Value, path: &str) -> R<SoundPlan> {
    let mut f = Fields::new(v, path)?;
    let from = f.take("from")?.map(|(v, p)| rom(&v, &p)).transpose()?.ok_or_else(|| format!("{path}.from: missing"))?;
    let mut base_version = String::new();
    f.set("base_version", &mut base_version, string)?;
    let mut versions = Vec::new();
    f.set("versions", &mut versions, |v, p| {
        list(v, p, |v, p| {
            let mut f = Fields::new(v, p)?;
            let name = f.take("name")?.map(|(v, p)| string(&v, &p)).transpose()?.ok_or_else(|| format!("{p}.name: missing"))?;
            let from = f.take("from")?.map(|(v, p)| rom(&v, &p)).transpose()?.ok_or_else(|| format!("{p}.from: missing"))?;
            f.finish()?;
            Ok((name, from))
        })
    })?;
    f.finish()?;
    if !versions.is_empty() && base_version.is_empty() {
        return Err(format!("{path}.base_version: versions of the sound need the base's name"));
    }
    Ok(SoundPlan { from, base_version, versions })
}

fn bundle(v: &Value, path: &str) -> R<Bundle> {
    record(v, path, |f, b: &mut Bundle| {
        f.set("sprites", &mut b.sprites, |v, p| list(v, p, sheet))?;
        f.set("field", &mut b.field, field)?;
        f.set("backgrounds", &mut b.backgrounds, |v, p| holey(v, p, background))?;
        f.set("hud", &mut b.hud, hud)?;
        f.set("custom", &mut b.custom, custom)
    })
}

// ---- Field and backgrounds -------------------------------------------------------------------

fn field(v: &Value, path: &str) -> R<Field> {
    record(v, path, |f, x: &mut Field| {
        f.set("tiles", &mut x.tiles, tiles)?;
        f.set("first_tile", &mut x.first_tile, int)?;
        f.set("palettes", &mut x.palettes, |v, p| list(v, p, palette))?;
        f.set("first_palette", &mut x.first_palette, int)?;
        f.set("palette_anims", &mut x.palette_anims, |v, p| list(v, p, palette_anim))?;
        f.set("panel_types", &mut x.panel_types, |v, p| list(v, p, string))?;
        f.set("panels", &mut x.panels, |v, p| list(v, p, map_of::<15>))?;
        f.set("front_edges", &mut x.front_edges, |v, p| array(v, p, map_of::<5>))?;
        f.set("highlights", &mut x.highlights, |v, p| list(v, p, map_of::<15>))
    })
}

fn palette_anim(v: &Value, path: &str) -> R<PaletteAnim> {
    record(v, path, |f, a: &mut PaletteAnim| {
        f.set("slot", &mut a.slot, int)?;
        f.set("frames", &mut a.frames, |v, p| list(v, p, |v, p| pair(v, p, palette, int)))?;
        f.set("initial_timer", &mut a.initial_timer, int)
    })
}

fn background(v: &Value, path: &str) -> R<Background> {
    record(v, path, |f, b: &mut Background| {
        f.set("tiles", &mut b.tiles, tiles)?;
        f.set("first_tile", &mut b.first_tile, int)?;
        f.set("map", &mut b.map, map)?;
        f.set("map_width", &mut b.map_width, int)?;
        f.set("map_height", &mut b.map_height, int)?;
        f.opt("palette", &mut b.palette, palette)?;
        f.set("scroll", &mut b.scroll, |v, p| pair(v, p, int, int))?;
        f.opt("speeding", &mut b.speeding, |v, p| {
            record(v, p, |f, s: &mut Speeding| {
                f.set("step", &mut s.step, |v, p| pair(v, p, int, int))?;
                f.set("top", &mut s.top, |v, p| pair(v, p, int, int))?;
                f.set("from_battle", &mut s.from_battle, boolean)
            })
        })?;
        f.set("anims", &mut b.anims, |v, p| list(v, p, gfx_anim))
    })
}

// ---- The HUD ---------------------------------------------------------------------------------

fn place(v: &Value, path: &str) -> R<(u8, u8)> {
    pair(v, path, int, int)
}

fn banner(v: &Value, path: &str) -> R<BannerLayout> {
    record(v, path, |f, b: &mut BannerLayout| {
        f.set("x", &mut b.x, int)?;
        f.set("y", &mut b.y, int)?;
        f.set("kind", &mut b.kind, int)?;
        f.set("glyphs", &mut b.glyphs, tiles)?;
        f.opt("number_at", &mut b.number_at, place)
    })
}

fn texts(v: &Value, path: &str) -> R<Vec<Vec<u16>>> {
    list(v, path, |v, p| list(v, p, int))
}

fn hud(v: &Value, path: &str) -> R<Hud> {
    record(v, path, |f, h: &mut Hud| {
        f.set("tiles", &mut h.tiles, tiles)?;
        f.set("first_tile", &mut h.first_tile, int)?;
        f.set("gauge_tiles", &mut h.gauge_tiles, tiles)?;
        f.set("gauge_first_tile", &mut h.gauge_first_tile, int)?;
        f.set("hp_palettes", &mut h.hp_palettes, |v, p| array(v, p, palette))?;
        f.set("gauge_palette", &mut h.gauge_palette, palette)?;
        f.set("hp_box", &mut h.hp_box, map)?;
        f.set("gauge_frame", &mut h.gauge_frame, map)?;
        f.set("font", &mut h.font, tiles)?;
        f.set("font_chars", &mut h.font_chars, |v, p| list(v, p, string))?;
        f.set("enemy_digits", &mut h.enemy_digits, |v, p| array(v, p, tiles))?;
        f.set("enemy_palette", &mut h.enemy_palette, palette)?;
        f.set("chip_icons", &mut h.chip_icons, |v, p| {
            list(v, p, |v, p| {
                record(v, p, |f, c: &mut ChipIcon| {
                    f.set("key", &mut c.key, string)?;
                    f.set("tiles", &mut c.tiles, tiles)
                })
            })
        })?;
        f.set("hidden_icon", &mut h.hidden_icon, tiles)?;
        f.set("icon_palette", &mut h.icon_palette, palette)?;
        f.set("mugshots", &mut h.mugshots, |v, p| list(v, p, |v, p| pair(v, p, tiles, palette)))?;
        f.set("counts", &mut h.counts, |v, p| list(v, p, tiles))?;
        f.set("count_box", &mut h.count_box, tiles)?;
        f.set("mugshot_boxes", &mut h.mugshot_boxes, |v, p| list(v, p, tiles))?;
        f.set("navi_mugshots", &mut h.navi_mugshots, |v, p| {
            list(v, p, |v, p| {
                record(v, p, |f, m: &mut NaviMugshot| {
                    f.set("tiles", &mut m.tiles, tiles)?;
                    f.set("palettes", &mut m.palettes, |v, p| array(v, p, palette))
                })
            })
        })?;
        f.set("navi_box", &mut h.navi_box, tiles)?;
        f.set("pause", &mut h.pause, tiles)?;
        f.set("texts", &mut h.texts, texts)?;
        f.set("banners", &mut h.banners, |v, p| list(v, p, banner))?;
        f.set("banner_digits", &mut h.banner_digits, tiles)?;
        f.set("banner_palette", &mut h.banner_palette, palette)?;
        f.set("waiting", &mut h.waiting, tiles)?;
        f.set("waiting_palette", &mut h.waiting_palette, palette)?;
        f.set("warning", &mut h.warning, tiles)?;
        f.set("warning_palette", &mut h.warning_palette, palette)?;
        f.set("dialogue_font", &mut h.dialogue_font, dialogue_font)?;
        f.set("chatbox", &mut h.chatbox, chatbox)?;
        f.set("layout", &mut h.layout, hud_layout)?;
        f.set("language", &mut h.language, string)?;
        f.set("languages", &mut h.languages, |v, p| named(v, p, hud_lettering))
    })
}

fn chatbox(v: &Value, path: &str) -> R<Chatbox> {
    record(v, path, |f, c: &mut Chatbox| {
        f.set("tiles", &mut c.tiles, tiles)?;
        f.set("palette", &mut c.palette, palette)?;
        f.set("boxes", &mut c.boxes, |v, p| list(v, p, |v, p| array(v, p, map)))?;
        f.set("arrow", &mut c.arrow, tiles)?;
        f.set("text_palette", &mut c.text_palette, palette)
    })
}

fn hud_layout(v: &Value, path: &str) -> R<HudLayout> {
    record(v, path, |f, l: &mut HudLayout| {
        f.set("pause", &mut l.pause, place)?;
        f.set("hp_number_priority", &mut l.hp_number_priority, int)?;
        f.set("hud_priority", &mut l.hud_priority, int)?;
        f.set("message", &mut l.message, |v, p| {
            let [a, b, c] = array(v, p, int)?;
            Ok((a, b, c))
        })?;
        f.set("judge_from_hold", &mut l.judge_from_hold, boolean)?;
        f.set("chatbox_text", &mut l.chatbox_text, place)?;
        f.set("chatbox_arrows", &mut l.chatbox_arrows, |v, p| array(v, p, place))?;
        f.opt("telop", &mut l.telop, |v, p| {
            record(v, p, |f, t: &mut TelopLook| {
                f.set("places", &mut t.places, |v, p| array(v, p, place))?;
                f.set("glyphs", &mut t.glyphs, tiles)?;
                f.set("palette", &mut t.palette, palette)?;
                f.opt("tiles", &mut t.tiles, |v, p| {
                    record(v, p, |f, t: &mut TelopTiles| {
                        f.set("banner", &mut t.banner, int)?;
                        f.set("names", &mut t.names, |v, p| array(v, p, int))?;
                        f.set("signs", &mut t.signs, int)?;
                        f.set("pause_loads_banner_palette", &mut t.pause_loads_banner_palette, boolean)
                    })
                })
            })
        })
    })
}

fn hud_lettering(v: &Value, path: &str) -> R<HudLettering> {
    record(v, path, |f, l: &mut HudLettering| {
        f.set("font", &mut l.font, tiles)?;
        f.set("font_chars", &mut l.font_chars, |v, p| list(v, p, string))?;
        f.set("dialogue_font", &mut l.dialogue_font, dialogue_font)?;
        f.set("texts", &mut l.texts, texts)?;
        f.set("banners", &mut l.banners, |v, p| holey(v, p, banner))?;
        f.set("banner_palette", &mut l.banner_palette, palette)?;
        f.set("waiting", &mut l.waiting, tiles)?;
        f.set("waiting_palette", &mut l.waiting_palette, palette)?;
        f.set("gauge_tiles", &mut l.gauge_tiles, tiles)?;
        f.opt("chatbox_text", &mut l.chatbox_text, place)
    })
}

// ---- The custom screen -----------------------------------------------------------------------

fn cursor_place(v: &Value, path: &str) -> R<CursorPlace> {
    record(v, path, |f, c: &mut CursorPlace| {
        f.set("x", &mut c.x, int)?;
        f.set("y", &mut c.y, int)?;
        f.set("corners", &mut c.corners, |v, p| {
            array(v, p, |v, p| {
                array(v, p, |v, p| {
                    let [a, b, c, d] = array(v, p, |v, _| Ok(v.clone()))?;
                    Ok((int(&a, &format!("{p}[1]"))?, int(&b, &format!("{p}[2]"))?, boolean(&c, &format!("{p}[3]"))?, boolean(&d, &format!("{p}[4]"))?))
                })
            })
        })
    })
}

fn custom_layout(v: &Value, path: &str) -> R<CustomLayout> {
    record(v, path, |f, l: &mut CustomLayout| {
        f.set("column_cells", &mut l.column_cells, int)?;
        f.set("turn_limit", &mut l.turn_limit, int)?;
        f.set("turn_limit_palette", &mut l.turn_limit_palette, int)?;
        f.set("name", &mut l.name, int)?;
        f.set("art", &mut l.art, int)?;
        f.set("code", &mut l.code, int)?;
        f.set("element", &mut l.element, int)?;
        f.set("digits", &mut l.digits, int)?;
        f.set("slots", &mut l.slots, int)?;
        f.set("column_icons", &mut l.column_icons, int)?;
        f.set("name_bar", &mut l.name_bar, int)?;
        f.set("form_names", &mut l.form_names, int)?;
        f.set("slot_blank", &mut l.slot_blank, int)?;
        f.set("detail_blank", &mut l.detail_blank, int)?;
        f.opt("empty_palette", &mut l.empty_palette, int)?;
        f.set("ok_cursor", &mut l.ok_cursor, cursor_place)?;
        f.set("regular", &mut l.regular, |v, p| pair(v, p, int, int))?;
        f.set("column_columns", &mut l.column_columns, |v, p| array(v, p, int))?;
        f.set("column_row", &mut l.column_row, int)?;
        f.set("form_names_at", &mut l.form_names_at, |v, p| array(v, p, int))
    })
}

fn picture(v: &Value, path: &str) -> R<Picture> {
    record(v, path, |f, x: &mut Picture| {
        f.set("tiles", &mut x.tiles, tiles)?;
        f.set("palette", &mut x.palette, palette)
    })
}

fn slot_pictures(v: &Value, path: &str) -> R<SlotPictures> {
    record(v, path, |f, s: &mut SlotPictures| {
        f.set("ok", &mut s.ok, picture)?;
        f.set("ok_picked", &mut s.ok_picked, picture)?;
        f.set("other", &mut s.other, picture)
    })
}

fn button(v: &Value, path: &str) -> R<ButtonPictures> {
    record(v, path, |f, b: &mut ButtonPictures| {
        f.set("width", &mut b.width, int)?;
        f.set("height", &mut b.height, int)?;
        f.set("tiles", &mut b.tiles, tiles)?;
        f.set("sets", &mut b.sets, |v, p| match string(v, p)?.as_str() {
            "each" => Ok(ButtonSets::Each),
            "other" => Ok(ButtonSets::Other),
            "unavailable" => Ok(ButtonSets::Unavailable),
            s => Err(format!("{p}: {s:?} is no button sets' kind (each, other, unavailable)")),
        })?;
        f.opt("hidden", &mut b.hidden, int)?;
        f.set("cursor", &mut b.cursor, cursor_place)?;
        f.set("picture", &mut b.picture, picture)?;
        f.set("palettes", &mut b.palettes, |v, p| list(v, p, palette))?;
        f.set("uses_digit", &mut b.uses_digit, boolean)?;
        f.opt("held_at", &mut b.held_at, |v, p| pair(v, p, int, int))?;
        f.set("icons", &mut b.icons, tiles)?;
        f.set("icon_palette", &mut b.icon_palette, palette)?;
        f.set("icon_palettes", &mut b.icon_palettes, |v, p| named(v, p, palette))?;
        f.opt("place", &mut b.place, |v, p| {
            record(v, p, |f, x: &mut ButtonPlace| {
                f.set("x", &mut x.x, int)?;
                f.set("y", &mut x.y, int)?;
                f.set("first_tile", &mut x.first_tile, int)?;
                f.set("palette", &mut x.palette, int)
            })
        })
    })
}

fn patch_list(v: &Value, path: &str) -> R<PatchList> {
    record(v, path, |f, l: &mut PatchList| {
        f.set("first_tile", &mut l.first_tile, int)?;
        f.set("patches", &mut l.patches, |v, p| {
            list(v, p, |v, p| {
                record(v, p, |f, m: &mut MapPatch| {
                    f.set("x", &mut m.x, int)?;
                    f.set("y", &mut m.y, int)?;
                    f.set("width", &mut m.width, int)?;
                    f.set("height", &mut m.height, int)?;
                    f.set("palette", &mut m.palette, int)?;
                    f.set("by_column", &mut m.by_column, boolean)
                })
            })
        })
    })
}

fn version_pictures(v: &Value, path: &str) -> R<VersionPictures> {
    record(v, path, |f, x: &mut VersionPictures| {
        f.set("buttons", &mut x.buttons, |v, p| named(v, p, button))?;
        f.set("form_names", &mut x.form_names, tiles)?;
        f.set("form_name_palettes", &mut x.form_name_palettes, |v, p| list(v, p, palette))
    })
}

fn custom(v: &Value, path: &str) -> R<CustomScreen> {
    record(v, path, |f, c: &mut CustomScreen| {
        f.set("layout", &mut c.layout, custom_layout)?;
        f.set("labels", &mut c.labels, |v, p| {
            list(v, p, |v, p| {
                record(v, p, |f, l: &mut WindowLabel| {
                    f.set("name", &mut l.name, string)?;
                    f.set("first", &mut l.first, int)?;
                    f.set("last", &mut l.last, int)?;
                    f.set("top", &mut l.top, int)?;
                    f.set("bottom", &mut l.bottom, int)?;
                    f.set("letters", &mut l.letters, |v, p| list(v, p, int))
                })
            })
        })?;
        f.set("buttons", &mut c.buttons, |v, p| named(v, p, button))?;
        f.set("window_tiles", &mut c.window_tiles, tiles)?;
        f.set("column_cells", &mut c.column_cells, tiles)?;
        f.set("turn_limit", &mut c.turn_limit, tiles)?;
        f.set("name_bar", &mut c.name_bar, tiles)?;
        f.set("window_maps", &mut c.window_maps, |v, p| list(v, p, map))?;
        f.set("window_patches", &mut c.window_patches, patch_list)?;
        f.set("form_list_maps", &mut c.form_list_maps, |v, p| list(v, p, map))?;
        f.set("form_list_patches", &mut c.form_list_patches, patch_list)?;
        f.set("frame_palettes", &mut c.frame_palettes, |v, p| list(v, p, palette))?;
        f.set("icon_palette", &mut c.icon_palette, palette)?;
        f.set("gray_palette", &mut c.gray_palette, palette)?;
        f.set("other_palette", &mut c.other_palette, palette)?;
        f.set("chip_art", &mut c.chip_art, |v, p| {
            list(v, p, |v, p| {
                record(v, p, |f, a: &mut ChipArt| {
                    f.set("key", &mut a.key, string)?;
                    f.set("picture", &mut a.picture, picture)?;
                    f.opt("version", &mut a.version, string)
                })
            })
        })?;
        f.set("pictures", &mut c.pictures, slot_pictures)?;
        f.set("codes", &mut c.codes, tiles)?;
        f.set("elements", &mut c.elements, tiles)?;
        f.set("element_colors", &mut c.element_colors, |v, p| list(v, p, |v, p| array(v, p, int)))?;
        f.set("element_frame", &mut c.element_frame, tiles)?;
        f.set("digits", &mut c.digits, tiles)?;
        f.set("slot_codes", &mut c.slot_codes, tiles)?;
        f.set("empty_icon", &mut c.empty_icon, tiles)?;
        f.set("versioned", &mut c.versioned, |v, p| {
            record(v, p, |f, x: &mut Versioned<VersionPictures>| {
                f.set("base_version", &mut x.base_version, string)?;
                f.set("base", &mut x.base, version_pictures)?;
                f.set("versions", &mut x.versions, |v, p| named(v, p, version_pictures))
            })
        })?;
        f.set("cursor", &mut c.cursor, tiles)?;
        f.set("form_list_cursor", &mut c.form_list_cursor, tiles)?;
        f.set("form_list_cursor_palette", &mut c.form_list_cursor_palette, palette)?;
        f.set("emblems", &mut c.emblems, |v, p| {
            list(v, p, |v, p| {
                record(v, p, |f, e: &mut Emblem| {
                    f.set("navi", &mut e.navi, string)?;
                    f.set("tiles", &mut e.tiles, tiles)?;
                    f.set("palette", &mut e.palette, palette)
                })
            })
        })?;
        f.set("regular", &mut c.regular, tiles)?;
        f.set("advance_name_colors", &mut c.advance_name_colors, |v, p| list(v, p, |v, p| array(v, p, int)))?;
        f.set("languages", &mut c.languages, |v, p| named(v, p, custom_lettering))?;
        f.opt("element_sprite", &mut c.element_sprite, |v, p| {
            record(v, p, |f, e: &mut ElementSprite| {
                f.set("x", &mut e.x, int)?;
                f.set("y", &mut e.y, int)?;
                f.set("palette", &mut e.palette, palette)
            })
        })?;
        f.opt("cursor_palette", &mut c.cursor_palette, palette)?;
        f.opt("window_emblem", &mut c.window_emblem, |v, p| {
            record(v, p, |f, e: &mut WindowEmblem| {
                f.set("x", &mut e.x, int)?;
                f.set("y", &mut e.y, int)?;
                f.set("width", &mut e.width, int)?;
                f.set("height", &mut e.height, int)?;
                f.set("first_tile", &mut e.first_tile, int)?;
                f.set("tiles", &mut e.tiles, tiles)?;
                f.set("frames", &mut e.frames, |v, p| list(v, p, map))?;
                f.set("steps", &mut e.steps, |v, p| holey(v, p, int))
            })
        })
    })
}

fn custom_lettering(v: &Value, path: &str) -> R<CustomLettering> {
    record(v, path, |f, l: &mut CustomLettering| {
        f.set("pictures", &mut l.pictures, slot_pictures)?;
        f.set("form_names", &mut l.form_names, |v, p| named(v, p, tiles))?;
        f.set("buttons", &mut l.buttons, |v, p| {
            named(v, p, |v, p| {
                record(v, p, |f, b: &mut ButtonLettering| {
                    f.opt("tiles", &mut b.tiles, tiles)?;
                    f.opt("picture", &mut b.picture, picture)
                })
            })
        })
    })
}

