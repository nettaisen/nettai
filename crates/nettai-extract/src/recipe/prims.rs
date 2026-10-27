//! The primitives (docs/design/extraction-recipes.md §3): the Rust a recipe
//! calls, game-neutral, every address and count a parameter. Handles for
//! bulk data (`Rom`, `Bytes`, `Tiles`, `Map`, `Sheet`, `GfxAnim`,
//! `DialogueFont`), plain values for what is small (a palette is sixteen
//! numbers, a map entry a table). Bounds are checked: a bad address is a
//! Luau error naming it, never a panic. content/nettai/extract.d.luau
//! declares them.

use crate::rom::{self, Rom};
use nettai_assets::{DialogueFont, GfxAnim, MapEntry, Palette, SpriteSheet, Tiles, palettes_from_bytes};
use nettai_luau::mlua::{self, AnyUserData, Lua, MetaMethod, Table, UserData, UserDataFields, UserDataMethods, Value};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;

type LResult<T> = mlua::Result<T>;

fn fail<T>(msg: impl Into<String>) -> LResult<T> {
    Err(mlua::Error::runtime(msg.into()))
}

/// A number argument as an integer in `min..=max`.
fn int(n: f64, what: &str, min: i64, max: i64) -> LResult<i64> {
    if n.fract() != 0.0 || !n.is_finite() || n < min as f64 || n > max as f64 {
        return fail(format!("{what}: expected an integer from {min} to {max:#x}, got {n}"));
    }
    Ok(n as i64)
}

/// An address or a word: 0 to 0xFFFFFFFF.
fn word(n: f64, what: &str) -> LResult<u32> {
    Ok(int(n, what, 0, u32::MAX as i64)? as u32)
}

/// A count or an offset.
fn count(n: f64, what: &str) -> LResult<usize> {
    Ok(int(n, what, 0, 1 << 26)? as usize)
}

/// The modules of the game whose recipe runs (`exe6:`), which the reads
/// log names a read's line in.
struct OwnModules(String);

/// Where the running recipe is (`exe6:extract/hud.luau:212`), while reads
/// are recorded: the reads log's fifth column. The innermost line of the
/// game's own modules (where a support pack's maker reads for it, the line
/// that called the maker: the place a read is of), else the innermost.
fn here(lua: &Lua) -> Option<Arc<str>> {
    if !rom::recording() {
        return None;
    }
    let own = lua.app_data_ref::<OwnModules>().map(|o| o.0.clone()).unwrap_or_default();
    let mut first = None;
    for level in 1.. {
        let Some(frame) = lua.inspect_stack(level, |d| {
            let source = d.source().source.map(|s| s.trim_start_matches('@').to_string());
            source.zip(d.current_line())
        }) else {
            break;
        };
        let Some((source, line)) = frame else { continue };
        let at = format!("{source}:{line}");
        if source.starts_with(&own) {
            return Some(at.into());
        }
        first.get_or_insert(at);
    }
    first.map(Arc::from)
}

// ---- ROMs ------------------------------------------------------------------------------------

/// One of the game's ROMs: its image (shared, never copied), its header
/// code and its place in the declaration (from 1).
pub(crate) struct RomHandle {
    pub rom: Arc<Rom>,
    pub code: String,
    pub slot: u32,
}

impl RomHandle {
    /// `n` bytes from `at` are in the image, else an error naming the read.
    fn check(&self, what: &str, at: u32, n: usize) -> LResult<()> {
        if self.rom.has(at, n) {
            Ok(())
        } else {
            fail(format!("Rom:{what}: {n} bytes at {at:#010x} are past {} ({} bytes)", self.code, self.rom.0.len()))
        }
    }
}

impl UserData for RomHandle {
    fn register(registry: &mut mlua::UserDataRegistry<Self>) {
        registry.add_field_method_get("code", |_, this| Ok(this.code.clone()));
        registry.add_field_method_get("slot", |_, this| Ok(this.slot));
        Self::add_methods(registry);
        registry.enable_namecall();
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("u8", |lua, this, at: f64| {
            let at = word(at, "Rom:u8")?;
            this.check("u8", at, 1)?;
            Ok(rom::located(here(lua), || this.rom.u8(at)))
        });
        methods.add_method("u16", |lua, this, at: f64| {
            let at = word(at, "Rom:u16")?;
            this.check("u16", at, 2)?;
            Ok(rom::located(here(lua), || this.rom.u16(at)))
        });
        methods.add_method("u32", |lua, this, at: f64| {
            let at = word(at, "Rom:u32")?;
            this.check("u32", at, 4)?;
            Ok(rom::located(here(lua), || this.rom.u32(at)))
        });
        methods.add_method("holds", |_, this, at: f64| Ok(this.rom.contains(word(at, "Rom:holds")?)));
        methods.add_method("bytes", |lua, this, (at, len): (f64, f64)| {
            let (at, len) = (word(at, "Rom:bytes")?, count(len, "Rom:bytes")?);
            this.check("bytes", at, len)?;
            Ok(Bytes(rom::located(here(lua), || this.rom.bytes(at, len)).into()))
        });
        methods.add_method("span", |lua, this, (at, max): (f64, f64)| {
            let (at, max) = (word(at, "Rom:span")?, count(max, "Rom:span")?);
            this.check("span", at, 0)?;
            Ok(Bytes(rom::located(here(lua), || this.rom.from(at, max)).into()))
        });
        methods.add_method("lz77", |lua, this, at: f64| {
            let at = word(at, "Rom:lz77")?;
            Ok(rom::located(here(lua), || this.rom.lz77(at)).map(|b| Bytes(b.into())))
        });
        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| Ok(format!("Rom({})", this.code)));
    }
}

// ---- Bytes -----------------------------------------------------------------------------------

/// Bytes read or decompressed, offsets from 0.
#[derive(Clone)]
pub(crate) struct Bytes(pub Arc<[u8]>);

impl Bytes {
    fn get(&self, what: &str, at: f64, n: usize) -> LResult<&[u8]> {
        let at = count(at, what)?;
        match at.checked_add(n).and_then(|end| self.0.get(at..end)) {
            Some(b) => Ok(b),
            None => fail(format!("{what}: {n} bytes at {at:#x} are past the {} bytes", self.0.len())),
        }
    }
}

impl UserData for Bytes {
    fn register(registry: &mut mlua::UserDataRegistry<Self>) {
        Self::add_methods(registry);
        registry.enable_namecall();
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("len", |_, this, ()| Ok(this.0.len()));
        methods.add_method("u8", |_, this, at: f64| Ok(this.get("Bytes:u8", at, 1)?[0]));
        methods.add_method("u16", |_, this, at: f64| {
            let b = this.get("Bytes:u16", at, 2)?;
            Ok(u16::from_le_bytes([b[0], b[1]]))
        });
        methods.add_method("u32", |_, this, at: f64| {
            let b = this.get("Bytes:u32", at, 4)?;
            Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        });
        methods.add_method("sub", |_, this, (from, len): (f64, f64)| {
            let len = count(len, "Bytes:sub")?;
            Ok(Bytes(this.get("Bytes:sub", from, len)?.into()))
        });
        methods.add_meta_method(MetaMethod::Eq, |_, this, other: AnyUserData| Ok(other.borrow::<Bytes>().is_ok_and(|o| o.0 == this.0)));
        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| Ok(format!("Bytes({})", this.0.len())));
    }
}

/// A `Rom | Bytes` argument's bytes: `n` of them from `at` (a ROM address,
/// or an offset), the ROM's read recorded.
fn read_from(lua: &Lua, from: &AnyUserData, what: &str, at: f64, n: usize) -> LResult<Arc<[u8]>> {
    if let Ok(r) = from.borrow::<RomHandle>() {
        let at = word(at, what)?;
        r.check(what, at, n)?;
        return Ok(rom::located(here(lua), || r.rom.bytes(at, n)).into());
    }
    if let Ok(b) = from.borrow::<Bytes>() {
        return Ok(b.get(what, at, n)?.into());
    }
    fail(format!("{what}: expected Rom or Bytes"))
}

// ---- Tiles, palettes, maps -----------------------------------------------------------------------

/// 8x8 tiles, a palette index a pixel.
#[derive(Clone)]
pub(crate) struct TilesHandle(pub Tiles);

impl UserData for TilesHandle {
    fn register(registry: &mut mlua::UserDataRegistry<Self>) {
        Self::add_methods(registry);
        registry.enable_namecall();
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("count", |_, this, ()| Ok(this.0.len()));
        methods.add_method("pick", |_, this, list: Vec<f64>| {
            let mut out = Tiles::default();
            for n in list {
                let i = count(n, "Tiles:pick")?;
                match this.0.get(i) {
                    Some(t) => out.push(t),
                    None => return fail(format!("Tiles:pick: tile {i} of {}", this.0.len())),
                }
            }
            Ok(TilesHandle(out))
        });
        methods.add_method("sub", |_, this, (first, n): (f64, f64)| {
            let (first, n) = (count(first, "Tiles:sub")?, count(n, "Tiles:sub")?);
            match this.0.pixels.get(first * Tiles::TILE..(first + n) * Tiles::TILE) {
                Some(p) => Ok(TilesHandle(Tiles { pixels: p.to_vec() })),
                None => fail(format!("Tiles:sub: tiles {first} to {} of {}", first + n, this.0.len())),
            }
        });
        methods.add_meta_method(MetaMethod::Eq, |_, this, other: AnyUserData| {
            Ok(other.borrow::<TilesHandle>().is_ok_and(|o| o.0 == this.0))
        });
        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| Ok(format!("Tiles({})", this.0.len())));
    }
}

/// Map entries, the GBA's text-mode format decoded.
#[derive(Clone)]
pub(crate) struct MapHandle(pub Vec<MapEntry>);

/// A map entry as a table: `{ tile, hflip, vflip, palette }`.
pub(crate) fn entry_table(lua: &Lua, e: &MapEntry) -> LResult<Table> {
    let t = lua.create_table()?;
    t.raw_set("tile", e.tile)?;
    t.raw_set("hflip", e.hflip)?;
    t.raw_set("vflip", e.vflip)?;
    t.raw_set("palette", e.palette)?;
    Ok(t)
}

impl UserData for MapHandle {
    fn register(registry: &mut mlua::UserDataRegistry<Self>) {
        Self::add_methods(registry);
        registry.enable_namecall();
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("count", |_, this, ()| Ok(this.0.len()));
        methods.add_method("entry", |lua, this, i: f64| {
            let i = count(i, "Map:entry")?;
            match this.0.get(i) {
                Some(e) => entry_table(lua, e),
                None => fail(format!("Map:entry: entry {i} of {}", this.0.len())),
            }
        });
        methods.add_meta_method(MetaMethod::Eq, |_, this, other: AnyUserData| Ok(other.borrow::<MapHandle>().is_ok_and(|o| o.0 == this.0)));
        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| Ok(format!("Map({})", this.0.len())));
    }
}

/// A palette as a table of sixteen colors.
fn palette_table(lua: &Lua, p: &Palette) -> LResult<Table> {
    lua.create_sequence_from(p.iter().copied())
}

// ---- Sprites, animations, fonts -------------------------------------------------------------------

/// A sprite archive's sheet, placed in the model as it is.
pub(crate) struct SheetHandle(pub SpriteSheet);

impl UserData for SheetHandle {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::Eq, |_, this, other: AnyUserData| Ok(other.borrow::<SheetHandle>().is_ok_and(|o| o.0 == this.0)));
        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| Ok(format!("Sheet({:02x}-{:02x})", this.0.category, this.0.index)));
    }
}

/// A background's graphics animation.
pub(crate) struct GfxAnimHandle(pub GfxAnim);

impl UserData for GfxAnimHandle {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::Eq, |_, this, other: AnyUserData| Ok(other.borrow::<GfxAnimHandle>().is_ok_and(|o| o.0 == this.0)));
    }
}

/// The dialogue font.
pub(crate) struct FontHandle(pub DialogueFont);

impl UserData for FontHandle {}

/// Run a decoder that indexes the ROM directly, its panic on data it
/// doesn't expect a Luau error (where unwinding is on).
fn guarded<T>(what: &str, f: impl FnOnce() -> T) -> LResult<T> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).or_else(|panic| {
        let detail = panic.downcast_ref::<String>().map(String::as_str).or_else(|| panic.downcast_ref::<&str>().copied()).unwrap_or("bad data");
        fail(format!("{what}: {detail}"))
    })
}

// ---- The libraries ---------------------------------------------------------------------------------

/// What a recipe's `data` reads: the files under its game's `compat/`, by
/// path in the pack (`compat/assets.toml`), with their bytes.
pub(crate) struct Data {
    pub game: String,
    pub files: BTreeMap<String, Arc<[u8]>>,
}

/// Install the primitives' libraries (`gba`, `bytes`, `sprite`, `gfx`,
/// `font`) and `data` as globals of `lua`.
pub(crate) fn install(lua: &Lua, data: Data) -> LResult<()> {
    lua.set_app_data(OwnModules(format!("{}:", data.game)));
    let g = lua.globals();

    let gba = lua.create_table()?;
    gba.set(
        "tiles",
        lua.create_function(|lua, (from, at, len): (AnyUserData, f64, f64)| {
            let len = count(len, "gba.tiles")?;
            Ok(TilesHandle(Tiles::from_4bpp(&read_from(lua, &from, "gba.tiles", at, len)?)))
        })?,
    )?;
    gba.set(
        "join",
        lua.create_function(|_, parts: Vec<AnyUserData>| {
            let mut out = Tiles::default();
            for p in parts {
                let t = p.borrow::<TilesHandle>().map_err(|_| mlua::Error::runtime("gba.join: expected a list of Tiles"))?;
                out.pixels.extend_from_slice(&t.0.pixels);
            }
            Ok(TilesHandle(out))
        })?,
    )?;
    gba.set(
        "palette",
        lua.create_function(|lua, (from, at): (AnyUserData, f64)| {
            palette_table(lua, &palettes_from_bytes(&read_from(lua, &from, "gba.palette", at, 32)?)[0])
        })?,
    )?;
    gba.set(
        "palettes",
        lua.create_function(|lua, (from, at, n): (AnyUserData, f64, f64)| {
            let n = count(n, "gba.palettes")?;
            let list = palettes_from_bytes(&read_from(lua, &from, "gba.palettes", at, 32 * n)?);
            let out = lua.create_table()?;
            for p in &list {
                out.push(palette_table(lua, p)?)?;
            }
            Ok(out)
        })?,
    )?;
    gba.set(
        "map",
        lua.create_function(|lua, (from, at, n, first): (AnyUserData, f64, f64, Option<f64>)| {
            let n = count(n, "gba.map")?;
            let first = int(first.unwrap_or(0.0), "gba.map: first_tile", 0, 0xFFFF)? as u16;
            let halfwords: Vec<u16> = if let Ok(r) = from.borrow::<RomHandle>() {
                let at = word(at, "gba.map")?;
                r.check("gba.map", at, 2 * n)?;
                rom::located(here(lua), || (0..n as u32).map(|i| r.rom.u16(at + 2 * i)).collect())
            } else {
                read_from(lua, &from, "gba.map", at, 2 * n)?.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect()
            };
            Ok(MapHandle(
                halfwords
                    .into_iter()
                    .map(|h| {
                        let e = MapEntry::from_gba(h);
                        MapEntry { tile: e.tile.wrapping_sub(first), ..e }
                    })
                    .collect(),
            ))
        })?,
    )?;
    g.set("gba", gba)?;

    let bytes = lua.create_table()?;
    bytes.set(
        "hex",
        lua.create_function(|_, text: String| {
            let digits: Vec<u8> = text.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
            if digits.len() % 2 != 0 {
                return fail("bytes.hex: an odd number of digits");
            }
            let mut out = Vec::with_capacity(digits.len() / 2);
            for pair in digits.chunks(2) {
                let s = std::str::from_utf8(pair).ok().and_then(|s| u8::from_str_radix(s, 16).ok());
                out.push(s.ok_or_else(|| mlua::Error::runtime(format!("bytes.hex: {:?} isn't a hex byte", String::from_utf8_lossy(pair))))?);
            }
            Ok(Bytes(out.into()))
        })?,
    )?;
    g.set("bytes", bytes)?;

    let sprite = lua.create_table()?;
    sprite.set(
        "archive",
        lua.create_function(|lua, (r, entry): (mlua::UserDataRef<RomHandle>, f64)| {
            let entry = word(entry, "sprite.archive")?;
            if entry & 0x8000_0000 == 0 && r.rom.contains(entry) {
                r.check("sprite.archive", entry, 0)?;
            }
            Ok(rom::located(here(lua), || crate::sprite::archive(&r.rom, entry)).map(|d| Bytes(d.into())))
        })?,
    )?;
    sprite.set(
        "sheet",
        lua.create_function(|_, (archive, category, index): (mlua::UserDataRef<Bytes>, f64, f64)| {
            let (category, index) = (int(category, "sprite.sheet", 0, 255)? as u8, int(index, "sprite.sheet", 0, 255)? as u8);
            Ok(guarded("sprite.sheet", || crate::sprite::sheet(&archive.0, category, index))?.map(SheetHandle))
        })?,
    )?;
    sprite.set(
        "portrait",
        lua.create_function(|_, (archive, category, index): (mlua::UserDataRef<Bytes>, f64, f64)| {
            let (category, index) = (int(category, "sprite.portrait", 0, 255)? as u8, int(index, "sprite.portrait", 0, 255)? as u8);
            Ok(guarded("sprite.portrait", || crate::sprite::portrait_sheet(&archive.0, category, index))?.map(SheetHandle))
        })?,
    )?;
    g.set("sprite", sprite)?;

    let gfx = lua.create_table()?;
    gfx.set(
        "anims",
        lua.create_function(|lua, (r, list, buffer): (mlua::UserDataRef<RomHandle>, f64, f64)| {
            let (list, buffer) = (word(list, "gfx.anims")?, word(buffer, "gfx.anims")?);
            let anims = guarded("gfx.anims", || rom::located(here(lua), || crate::gfx::gfx_anims(&r.rom, list, buffer)))?;
            lua.create_sequence_from(anims.into_iter().map(GfxAnimHandle))
        })?,
    )?;
    g.set("gfx", gfx)?;

    let font = lua.create_table()?;
    font.set(
        "dialogue",
        lua.create_function(
            |lua, (r, glyphs, advances, n, advance_bytes, chars): (mlua::UserDataRef<RomHandle>, f64, f64, f64, f64, Vec<String>)| {
                let (glyphs, advances, n) = (word(glyphs, "font.dialogue")?, word(advances, "font.dialogue")?, count(n, "font.dialogue")?);
                let step = match int(advance_bytes, "font.dialogue: advance_bytes", 1, 4)? {
                    s @ (1 | 2 | 4) => s as u32,
                    s => return fail(format!("font.dialogue: advances {s} bytes apart, not 1, 2 or 4")),
                };
                r.check("font.dialogue", glyphs, 0x60 * n)?;
                r.check("font.dialogue", advances, step as usize * n)?;
                let at = here(lua);
                let pixels = rom::located(at.clone(), || r.rom.bytes(glyphs, 0x60 * n).iter().flat_map(|&b| [b & 15, b >> 4]).collect());
                let advances = rom::located(at, || match step {
                    1 => r.rom.bytes(advances, n).to_vec(),
                    2 => (0..n as u32).map(|i| r.rom.u16(advances + 2 * i) as u8).collect(),
                    _ => (0..n as u32).map(|i| r.rom.u32(advances + 4 * i) as u8).collect(),
                });
                Ok(FontHandle(DialogueFont { pixels, advances, chars }))
            },
        )?,
    )?;
    g.set("font", font)?;

    // (Each file read once, its tables frozen: every caller sees the same.)
    let cache: Rc<RefCell<BTreeMap<String, Value>>> = Rc::default();
    g.set(
        "data",
        lua.create_function(move |lua, path: String| {
            if let Some(v) = cache.borrow().get(&path) {
                return Ok(v.clone());
            }
            let Some(bytes) = data.files.get(&path) else {
                return fail(format!("data({path:?}): {0}/{path} is no file of {0}'s compat/, which is all a recipe reads", data.game));
            };
            let text = std::str::from_utf8(bytes).map_err(|e| mlua::Error::runtime(format!("data({path:?}): {e}")))?;
            let value = if path.ends_with(".toml") {
                let table: toml::Table = toml::from_str(text).map_err(|e| mlua::Error::runtime(format!("{}/{path}: {e}", data.game)))?;
                toml_value(lua, &toml::Value::Table(table))?
            } else {
                Value::String(lua.create_string(text)?)
            };
            nettai_luau::sandbox::deep_freeze(lua, &value)?;
            cache.borrow_mut().insert(path, value.clone());
            Ok(value)
        })?,
    )?;
    Ok(())
}

/// A TOML value as Luau's: tables keyed by name, arrays from 1, integers
/// and floats as numbers, a date as its text.
fn toml_value(lua: &Lua, v: &toml::Value) -> LResult<Value> {
    Ok(match v {
        toml::Value::String(s) => Value::String(lua.create_string(s)?),
        toml::Value::Integer(i) => Value::Number(*i as f64),
        toml::Value::Float(f) => Value::Number(*f),
        toml::Value::Boolean(b) => Value::Boolean(*b),
        toml::Value::Datetime(d) => Value::String(lua.create_string(d.to_string())?),
        toml::Value::Array(a) => {
            let t = lua.create_table_with_capacity(a.len(), 0)?;
            for x in a {
                t.raw_push(toml_value(lua, x)?)?;
            }
            Value::Table(t)
        }
        toml::Value::Table(m) => {
            let t = lua.create_table_with_capacity(0, m.len())?;
            for (k, x) in m {
                t.raw_set(k.as_str(), toml_value(lua, x)?)?;
            }
            Value::Table(t)
        }
    })
}

/// The ROMs' table a recipe is given: each declared code to its ROM.
pub(crate) fn rom_set(lua: &Lua, roms: impl IntoIterator<Item = RomHandle>) -> LResult<Table> {
    let set = lua.create_table()?;
    for r in roms {
        set.raw_set(r.code.clone(), r)?;
    }
    set.set_readonly(true);
    Ok(set)
}
