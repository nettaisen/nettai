//! The host on synthetic data (docs/design/extraction-recipes.md §4.5: CI has no ROMs): the primitives, the codec,
//! the declaration, the fingerprint and the refusals.

use super::*;
use crate::rom;
use nettai_content_api::PackKind;
use std::path::PathBuf;

/// A ROM image of `code`, 8 MiB, with `at` (ROM addresses) holding these bytes.
fn image(code: &str, at: &[(u32, &[u8])]) -> Vec<u8> {
    let mut bytes = vec![0; 8 * 1024 * 1024];
    bytes[0xac..0xb0].copy_from_slice(code.as_bytes());
    for (a, b) in at {
        let o = (*a & 0x01FF_FFFF) as usize;
        bytes[o..o + b.len()].copy_from_slice(b);
    }
    bytes
}

fn manifest(id: &str, kind: PackKind, depends: &[&str]) -> PackManifest {
    PackManifest { id: id.into(), kind, depends: depends.iter().map(|d| d.to_string()).collect(), ..Default::default() }
}

/// A recipe's VM over modules in memory (`g:extract/init` and what it requires), with the primitives and `data`
/// (`reads`, by path), and what its function returns given the set of `roms`, read by `read` while the VM lives.
fn run_recipe<T>(modules: &[(&str, &str)], reads: &[(&str, &str)], roms: &[&Arc<Rom>], read: impl FnOnce(Value) -> T) -> Result<T, String> {
    let m: BTreeMap<String, String> = modules.iter().map(|(n, s)| (n.to_string(), s.to_string())).collect();
    let data = prims::Data { game: "g".into(), files: reads.iter().map(|(p, t)| (p.to_string(), t.as_bytes().into())).collect() };
    let packs = vec![manifest("lib", PackKind::Support, &[]), manifest("g", PackKind::Game, &["lib"])];
    let vm = tool::open(m, packs, "g:extract", "extract", ToolOptions::default(), |lua| prims::install(lua, data)).map_err(|e| e.to_string())?;
    let codes = ["BR6E", "BR5E", "BR6J", "BR5J"];
    let set = prims::rom_set(
        vm.lua(),
        roms.iter().enumerate().map(|(i, r)| prims::RomHandle { rom: (*r).clone(), code: codes[i].into(), slot: i as u32 + 1 }),
    )
    .map_err(|e| e.to_string())?;
    vm.call::<Value>(set).map_err(|e| e.to_string()).map(read)
}

/// The recipe's result read as a model, of a recipe whose function's body is `body` (its set is `set`).
fn model_of(body: &str) -> Result<model::Extracted, String> {
    let rom = Arc::new(Rom(image("BR6E", &[])));
    let init = format!("return function(set)\n{body}\nend");
    run_recipe(&[("g:extract/init", &init)], &[], &[&rom], |v| model::extracted(&v))?
}

#[test]
fn the_primitives_read_the_rom_and_check_their_bounds() {
    // A pointer at 0x08000100 to a palette and two tiles at 0x08000200; a map at 0x08000300.
    let mut tiles = vec![0u8; 64];
    tiles[0] = 0x21; // tile 0's first two pixels: 1, 2
    tiles[32] = 0x43; // tile 1's: 3, 4
    let palette: Vec<u8> = (0..16u16).flat_map(|c| (c | 0x8000).to_le_bytes()).collect();
    let map: Vec<u8> = [0x54A7u16, 0x0003].iter().flat_map(|e| e.to_le_bytes()).collect();
    let rom = Arc::new(Rom(image(
        "BR6E",
        &[(0x0800_0100, &0x0800_0200u32.to_le_bytes()), (0x0800_0200, &tiles), (0x0800_0240, &palette), (0x0800_0300, &map)],
    )));
    let other = Arc::new(Rom(image("BR5E", &[])));
    let init = r#"
        local place = require("@lib/extract/place")
        return function(set)
            local rom = set.BR6E
            local p = place.at(rom, place.pointer(0x08000100, 0))
            local t = gba.tiles(rom, p, 64)
            local picked = t:pick({ 1, 0 })
            local m = gba.map(rom, 0x08000300, 2, 3)
            local e = m:entry(0)
            local b = rom:bytes(0x08000200, 4)
            local ok, err = pcall(function() return rom:u32(0x087FFFFE) end)
            local lz, hex = rom:lz77(0x08000200), bytes.hex("10 04 00 00 40 78 f0 00")
            local x = gba.tiles(hex, 0, 8)
            return {
                code = rom.code, slot = set.BR5E.slot, p = p, count = t:count(), joined = gba.join({ t, picked }):count(),
                same = picked:pick({ 1, 0 }) == t, palette = gba.palette(rom, p + 0x40), tile = e.tile, hflip = e.hflip,
                palette_of = e.palette, wrapped = m:entry(1).tile, b = b:u16(2), err = err, lz = lz == nil,
                hex = hex:len(), x = x:count(), first = b:u8(0), sub = b:sub(1, 2):u8(0), holds = rom:holds(0x08800000),
                read = data("compat/x.toml").a.b, missing = select(2, pcall(data, "compat/y.toml")),
            }
        end"#;
    let lib = "return { at = function(rom, p) local a = p.at[rom.slot] if p.pointer then return rom:u32(a) end return a end, \
               pointer = function(us, jp) return { at = { us, us, jp, jp }, pointer = true } end }";
    run_recipe(&[("g:extract/init", init), ("lib:extract/place", lib)], &[("compat/x.toml", "[a]\nb = 0x2A\n")], &[&rom, &other], check).unwrap();
}

fn check(v: Value) {
    let Value::Table(t) = v else { panic!("{v:?}") };
    let get = |k: &str| t.get::<Value>(k).unwrap();
    let n = |k: &str| match get(k) {
        Value::Integer(i) => i as f64,
        Value::Number(f) => f,
        v => panic!("{k}: {v:?}"),
    };
    assert_eq!(get("code").to_string().unwrap(), "BR6E");
    assert_eq!((n("slot"), n("p"), n("count"), n("joined")), (2.0, 0x0800_0200 as f64, 2.0, 4.0));
    assert!(get("same").as_boolean().unwrap(), "picked twice back is the tiles");
    let Value::Table(pal) = get("palette") else { panic!() };
    assert_eq!(pal.get::<u32>(2).unwrap(), 0x8001, "a palette's colors as stored");
    assert_eq!((n("tile"), get("hflip").as_boolean(), n("palette_of")), (0xA7 as f64 - 3.0, Some(true), 5.0));
    assert_eq!(n("wrapped"), 0.0, "the first tile taken off");
    assert_eq!(n("b"), 0.0);
    assert!(get("err").to_string().unwrap().contains("past BR6E"), "{:?}", get("err"));
    assert!(get("lz").as_boolean().unwrap(), "no LZ77 there");
    assert_eq!((n("hex"), n("x"), n("first"), n("sub")), (8.0, 0.0, 0x21 as f64, 0.0));
    assert!(!get("holds").as_boolean().unwrap());
    assert_eq!(n("read"), 42.0);
    assert!(get("missing").to_string().unwrap().contains("doesn't name it in [extract] reads"), "{:?}", get("missing"));
}

#[test]
fn reads_are_recorded_with_the_recipes_lines() {
    let rom = Arc::new(Rom(image("BR6E", &[(0x0800_0010, &0x0800_0020u32.to_le_bytes())])));
    let init = "return function(set)\n    local rom = set.BR6E\n    return rom:u32(0x08000010) + rom:u8(0x08000020)\nend";
    let (v, reads) = rom::record_reads(|| run_recipe(&[("g:extract/init", init)], &[], &[&rom], |_| ()));
    assert!(v.is_ok());
    let own: Vec<_> = reads.iter().filter(|r| r.rom == rom.0.as_ptr() as usize).collect();
    assert!(own.iter().any(|r| r.address == 0x0800_0010 && r.kind == rom::ReadKind::Pointer(0x0800_0020)));
    assert!(own.iter().all(|r| r.at.as_deref() == Some("g:extract/init.luau:3")), "{:?}", own.iter().map(|r| r.at.clone()).collect::<Vec<_>>());
}

#[test]
fn the_codec_reads_the_model_and_names_what_it_refuses() {
    let m = model_of(
        r#"
        local t = gba.tiles(set.BR6E, 0x08000000, 64)
        return {
            names = { sprites = { [0x20] = { [5] = "roll" } }, songs = { [0x63] = "stop-music" }, glyphs = { "A" },
                      language_glyphs = { ja = { glyphs = { "あ" } } } },
            graphics = {
                sprites = {},
                backgrounds = { n = 3, [2] = { tiles = t, scroll = { -8, 4 }, map = { { tile = 1, hflip = false, vflip = true, palette = 2 } } } },
                hud = { layout = { pause = { 100, 64 } }, banners = { { kind = 1, number_at = { 1, 2 } } }, language = "en",
                        languages = { { name = "ja", value = { banners = { n = 1 } } } } },
                custom = { buttons = { { name = "redeal", value = { sets = "other", hidden = 3 } } },
                           versioned = { base_version = "falzar", base = {}, versions = { { name = "gregar", value = { form_names = t } } } } },
            },
            sound = { from = set.BR6E },
        }"#,
    )
    .unwrap();
    assert_eq!(m.names.sprites.get(&(0x20, 5)).map(String::as_str), Some("roll"));
    assert_eq!(m.names.language_glyphs["ja"].0, ["あ"]);
    let g = &m.graphics;
    assert_eq!(g.backgrounds.len(), 3);
    assert!(g.backgrounds[0].is_none() && g.backgrounds[2].as_ref().is_some_and(|b| b.scroll == (-8, 4) && b.map[0].vflip && b.tiles.len() == 2));
    assert_eq!(g.hud.layout.pause, (100, 64));
    assert_eq!(g.hud.layout.message, nettai_assets::HudLayout::default().message, "a field left out takes the default");
    assert_eq!(g.hud.banners[0].number_at, Some((1, 2)));
    assert_eq!(g.hud.languages[0].1.banners.len(), 1);
    assert_eq!(g.custom.buttons[0].1.sets, nettai_assets::ButtonSets::Other);
    assert_eq!(g.custom.versioned.versions[0].0, "gregar");
    assert_eq!(m.sound.from.0, "BR6E");

    let refused = |body: &str| model_of(body).err().unwrap();
    let e = refused("return { names = {}, graphics = { hud = { chatbox = { boxes = { { \"x\" } } } } }, sound = { from = set.BR6E } }");
    assert!(e.contains("graphics.hud.chatbox.boxes[1][1]: expected Map, got string"), "{e}");
    let e = refused("return { names = {}, graphics = { hud = { titles = 1 } }, sound = { from = set.BR6E } }");
    assert!(e.contains("graphics.hud.titles: no such field"), "{e}");
    let e = refused("return { names = {}, graphics = { hud = { first_tile = 0x10000 } }, sound = { from = set.BR6E } }");
    assert!(e.contains("graphics.hud.first_tile: expected an integer from 0 to 65535"), "{e}");
    let e = refused("return { names = {}, graphics = { backgrounds = { n = 1, [1] = {} } }, sound = { from = set.BR6E } }");
    assert!(e.contains("past its length"), "{e}");
    let e = refused("return { names = {}, graphics = { custom = { buttons = { { name = \"x\", value = { sets = \"some\" } } } } }, sound = { from = set.BR6E } }");
    assert!(e.contains("is no button sets' kind"), "{e}");
    let e = refused("return { names = {}, graphics = { hud = { counts = { [1] = gba.tiles(set.BR6E, 0, 32), [3] = gba.tiles(set.BR6E, 0, 32) } } }, sound = { from = set.BR6E } }");
    assert!(e.contains("graphics.hud.counts"), "{e}");
    assert!(refused("return { names = {}, graphics = {} }").contains("no sound"));
}

/// A bundle on disk: `files` by path in it.
fn bundle(files: &[(String, String)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (path, text) in files {
        let p: PathBuf = dir.path().join(path);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    dir
}

const GAME: &str = "id = \"g\"\nkind = \"game\"\ndepends = [\"lib\"]\n[extract]\nprimitives = 1\nroms = [\"BR6E\", \"BR5E\"]\nsize = 0x800000\nreads = [\"compat/assets.toml\"]\n";

/// A synthetic game `g` and its support pack `lib`, the game's manifest `game`'s.
fn synthetic(game: &str) -> Vec<(String, String)> {
    [
        ("g/manifest.toml", game),
        ("lib/manifest.toml", "id = \"lib\"\nkind = \"support\"\n"),
        ("g/extract/init.luau", "return function(set) return {} end\n"),
        ("g/extract/shapes/hud/hp-box.same", "lib/extract/shapes/hud/hp-box.svg\n"),
        ("lib/extract/place.luau", "return {}\n"),
        ("lib/extract/shapes/hud/hp-box.svg", "<svg/>\n"),
        ("g/compat/assets.toml", "[sprites]\n"),
        ("g/rules/init.luau", "return {}\n"),
    ]
    .into_iter()
    .map(|(p, t)| (p.to_string(), t.to_string()))
    .collect()
}

#[test]
fn a_recipe_is_its_inputs_and_its_fingerprint_digests_them() {
    let files = synthetic(GAME);
    let dir = bundle(&files);
    let recipe = Recipe::of_content(dir.path(), "g").unwrap();
    assert_eq!((recipe.game(), recipe.roms(), recipe.primitives()), ("g", &["BR6E".to_string(), "BR5E".to_string()][..], 1));
    let inputs: Vec<&String> = recipe.files.keys().collect();
    assert_eq!(
        inputs,
        ["g/compat/assets.toml", "g/extract/init.luau", "g/extract/shapes/hud/hp-box.same", "lib/extract/place.luau", "lib/extract/shapes/hud/hp-box.svg"]
    );
    let print = recipe.fingerprint();
    assert!(print.len() == 16 && print.chars().all(|c| c.is_ascii_hexdigit()), "{print}");
    assert_eq!(Recipe::of_content(dir.path(), "g").unwrap().fingerprint(), print, "the same bundle, the same fingerprint");
    assert_eq!(recipe.drawings().game(), "g");
    // A byte changed in any input changes it; a change outside them doesn't.
    for (path, _) in files.iter().filter(|(p, _)| !p.ends_with("manifest.toml")) {
        let changed: Vec<(String, String)> = files.iter().map(|(p, t)| (p.clone(), if p == path { format!("{t} ") } else { t.clone() })).collect();
        let again = Recipe::of_content(bundle(&changed).path(), "g").unwrap().fingerprint();
        if path == "g/rules/init.luau" {
            assert_eq!(again, print, "{path} isn't an input");
        } else {
            assert_ne!(again, print, "{path} is an input");
        }
    }
    // The declaration is digested too.
    let declared = synthetic(&GAME.replace("0x800000", "0x400000"));
    assert_ne!(Recipe::of_content(bundle(&declared).path(), "g").unwrap().fingerprint(), print);
}

#[test]
fn a_recipe_refuses_what_it_cannot_run() {
    let dir = bundle(&synthetic(GAME));
    let recipe = Recipe::of_content(dir.path(), "g").unwrap();
    // An incomplete set, before any Luau runs.
    let mut roms = RomSet::default();
    roms.insert(image("BR6E", &[])).unwrap();
    let e = recipe.extract(&roms).err().unwrap().to_string();
    assert!(e.contains("incomplete") && e.contains("BR5E missing"), "{e}");
    // A recipe that needs newer primitives.
    let dir = bundle(&synthetic(&GAME.replace("primitives = 1", "primitives = 99")));
    let e = Recipe::of_content(dir.path(), "g").unwrap().extract(&roms).err().unwrap().to_string();
    assert!(e.contains("needs a newer nettai") && e.contains("primitives 99"), "{e}");
    // A file it reads that isn't there; a game with no recipe.
    let missing: Vec<(String, String)> = synthetic(GAME).into_iter().filter(|(p, _)| p != "g/compat/assets.toml").collect();
    assert!(Recipe::of_content(bundle(&missing).path(), "g").unwrap_err().to_string().contains("compat/assets.toml, which [extract] reads"));
    let none = synthetic("id = \"g\"\nkind = \"game\"\ndepends = [\"lib\"]\n");
    assert!(Recipe::of_content(bundle(&none).path(), "g").unwrap_err().to_string().contains("declares no [extract]"));
}

#[test]
fn a_named_asset_the_recipe_doesnt_give_is_an_error() {
    let mut names = AssetNames::default();
    names.sprites.insert((0, 0), "megaman".into());
    let mut g = nettai_assets::Bundle::default();
    assert_eq!(complete(&mut g, &names).unwrap_err(), "sprite/megaman is named in the asset names and no ROM gave it");
    g.sprites.push(nettai_assets::SpriteSheet::default());
    assert_eq!(complete(&mut g, &names).unwrap_err(), "the recipe gave no field");
}
