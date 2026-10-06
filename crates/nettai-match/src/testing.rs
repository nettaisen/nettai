//! Content for tests: content/exe6's definitions (and EXE5's:
//! [`exe5_content`]), each game alone as a match loads it, on a made-up
//! asset index
//! (`testing::asset_names_used`), every sprite timed as the test content's
//! navi is (nothing from a ROM), with their own strings.

use nettai_battle::Content;
use std::sync::{Arc, OnceLock};

/// Every sprite of `c` timed as the test content's navi is.
fn timed(c: &mut Content) {
    use nettai_battle::content::testing;
    let mut navi = testing::content().animations.sprites[&testing::sprite(testing::NAVI_SPRITE)].clone();
    navi.resize(0x40, navi[1].clone());
    for h in 0..c.assets.sprites.len() {
        c.animations.sprites.insert(nettai_battle::content::SpriteId(h as u16), navi.clone());
    }
}

/// The repository's content directory.
const CONTENT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content");

/// content/'s games `games` (each by its index and what it requires), with
/// their strings, on a made-up asset index (each game's names in its own
/// game's pack), every sprite timed as the test content's navi is: defined,
/// or why not.
pub fn defined(games: &[&str]) -> Result<Content, String> {
    let mut report = nettai_content::report::Report::default();
    let games: Vec<String> = games.iter().map(|g| g.to_string()).collect();
    let read = nettai_content::index::read(std::path::Path::new(CONTENT), &games, &mut report).ok_or_else(|| report.to_string())?;
    defined_of(read)
}

/// What a read of content/ read, defined as [`defined`] defines it.
fn defined_of(read: nettai_content::index::Read) -> Result<Content, String> {
    let mut c = Content::default();
    c.scripts = read.scripts();
    c.strings = read.strings;
    c.assets = nettai_battle::content::testing::asset_names_for(&c.scripts);
    timed(&mut c);
    c.define().map_err(|e| e.message)?;
    Ok(c)
}

/// content/'s EXE6, defined once per test process.
pub fn exe6_content() -> Arc<Content> {
    static EXE6: OnceLock<Arc<Content>> = OnceLock::new();
    EXE6.get_or_init(|| Arc::new(defined(&["exe6"]).unwrap_or_else(|e| panic!("content/exe6: {e}")))).clone()
}

/// content/'s EXE5 (its game pack and the support packs it depends on), defined
/// once per test process.
pub fn exe5_content() -> Arc<Content> {
    static EXE5: OnceLock<Arc<Content>> = OnceLock::new();
    EXE5.get_or_init(|| Arc::new(defined(&["exe5"]).unwrap_or_else(|e| panic!("content/exe5: {e}")))).clone()
}

/// A program on a NaviCust, for a test: which, in which of its colors (its
/// place in the definition's `colors`), its center, its quarter turns,
/// compressed or not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlacedProgram {
    pub program: nettai_content_api::EntryHandle,
    pub color: u8,
    pub x: u8,
    pub y: u8,
    pub rotation: u8,
    pub compressed: bool,
}

/// NaviCust program `program`'s colors, in its variants' order (its data's
/// `colors`).
pub fn program_colors(content: &Content, program: nettai_content_api::EntryHandle) -> Vec<&str> {
    use nettai_content_api::{Data, Registry};
    match content.defs.definitions.get(Registry::Entry, &content.defs.entry(program).key).map(|d| d.spec.field("colors")) {
        Some(Data::List(colors)) => colors.iter().filter_map(|c| c.str()).collect(),
        _ => Vec::new(),
    }
}

/// NaviCust program `program`'s shape as placed (its data's `shape`, or
/// `compressed` where it has one), turned.
pub fn program_shape(content: &Content, program: nettai_content_api::EntryHandle, compressed: bool, rotation: u8) -> nettai_battle::navicust::Shape {
    use nettai_content_api::Registry;
    let d = content.defs.definitions.get(Registry::Entry, &content.defs.entry(program).key).expect("the program's definition");
    let field = if compressed && !d.spec.field("compressed").is_nil() { "compressed" } else { "shape" };
    let shape = nettai_battle::navicust::read_shape(d.spec.field(field)).expect("a program's shape");
    nettai_battle::navicust::rotate(&shape, rotation)
}

/// State `side`'s NaviCust (its rules' `navicust_expansions` and
/// `navicust_programs`): `parts` on the board of `expansions`.
pub fn set_navicust(content: &Content, side: &mut crate::Side, parts: &[PlacedProgram], expansions: u8) {
    use nettai_battle::rules::Fact;
    use nettai_content_api::{Registry, Value};
    let records: Vec<Fact> = parts
        .iter()
        .map(|p| {
            let color = program_colors(content, p.program)[p.color as usize];
            Fact::Record(vec![
                ("program", Fact::Value(Value::Def(Registry::Entry, p.program.0))),
                ("color", Fact::Name(color)),
                ("x", Fact::Value(Value::Int(p.x as i64))),
                ("y", Fact::Value(Value::Int(p.y as i64))),
                ("rotation", Fact::Value(Value::Int(p.rotation as i64))),
                ("compressed", Fact::Value(Value::Bool(p.compressed))),
            ])
        })
        .collect();
    side.set_fact(content, "navicust_programs", &records).unwrap();
    side.set_fact(content, "navicust_expansions", &[Fact::Value(Value::Int(expansions as i64))]).unwrap();
}

/// `side`'s NaviCust's expansions (its `navicust_expansions`); none: no
/// NaviCust.
pub fn navicust_expansions(content: &Content, side: &crate::Side) -> Option<u8> {
    match side.facts.fact(content, "navicust_expansions")?.value() {
        nettai_content_api::FieldValue::OptionalU8(n) => n,
        _ => None,
    }
}

/// Patch cards of `game` from a list of their names, comma-separated, in
/// the order they apply (`canodumb,shadow`), as the rules' `patch_cards`
/// take them.
pub fn patch_cards(content: &Content, game: &str, list: &str) -> Vec<nettai_battle::rules::Fact<'static>> {
    use nettai_battle::rules::Fact;
    use nettai_content_api::{Registry, Value};
    list.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|name| {
            let card = crate::ids::entry(content, game, "patch_cards", name).unwrap_or_else(|| panic!("no patch card {name:?} in {game}"));
            Fact::Value(Value::Def(Registry::Entry, card.0))
        })
        .collect()
}
