//! Loading a game's battle content, and checking a game's assets against it.
//!
//! The battle content is the content directory's games (`crate::index`, content/ in this repository), whose modules
//! name the assets and define the sprites' timing: [`load_battle`] loads a game's content alone, and a battle reads
//! nothing else. Its assets (graphics and sound) are extracted from the game's ROMs (`nettai_extract`) by its recipe,
//! which the game's root gives and which loads with the content ([`recipe`]; [`extract`] is the whole of it in one
//! call); what they hold of the content's, for whoever draws and plays them, is [`asset_numbers`].

use crate::report::Report;
use nettai_assets::Bundle;
use nettai_content_api::{AssetIndex, AssetNumbers, packs};
use nettai_extract::{Extraction, Recipe, RomSource};
use std::path::Path;

thread_local! {
    /// How many times this thread read a part of the content from its files ([`reads`]).
    static READS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// How many times this thread has read a part of the content from its files so far: a game's battle content
/// ([`battle_content`]) or a language's strings (`crate::locale::load_for`). Each is read whole, by a loader a host
/// calls: nothing reads a file as a battle runs, so a host that loaded its game before a match sees the count stay
/// where it was through the match (docs/frontend.md §9).
pub fn reads() -> u64 {
    READS.with(|n| n.get())
}

/// Count a part read on this thread.
pub(crate) fn read_a_part() {
    READS.with(|n| n.set(n.get() + 1));
}

/// The battle content of game `game` of the content directory `content`
/// (`crate::index::content()`), for the engine (docs/design/
/// content-model-v2.md §4.0: one game a match): the game's top module and
/// what it requires, with the support packs it depends on, defined
/// (`Content::define`). Everything a battle reads is the content's, the
/// asset handles and the sprites' timing too: no asset is read
/// ([`asset_numbers`] says what a game's assets hold of it, for drawing and
/// sound).
pub fn load_battle(content: &Path, game: &str) -> Result<(nettai_battle::Content, Report), Report> {
    let (mut c, mut report) = battle_content(content, game)?;
    // The define phase: what the modules define, the tables registration
    // by number reads, and the registries.
    if let Err(e) = c.define() {
        report.error(content.display().to_string(), e.message);
        return Err(report);
    }
    Ok((c, report))
}

/// The extraction recipe of `content`'s game, as the content's load read it (docs/design/extraction-recipes.md §4.2):
/// the module the game's root gives as its `extract`, with the recipe's modules the define phase read and its other
/// inputs, read from the packs' folders now ([`Recipe::new`]). The content is defined ([`load_battle`]): a game's
/// recipe is had no other way. An error: its root gives none, or it doesn't read.
pub fn recipe(content: &nettai_battle::Content) -> Result<Recipe, String> {
    let game = content.game();
    let s = &content.scripts;
    let Some(entry) = s.recipe.as_deref() else {
        return Err(format!("{game}'s content has no extraction recipe: its root (its {}.luau) gives no `{}`", packs::INIT, packs::EXTRACT));
    };
    let modules = s.modules.iter().filter(|(name, _)| packs::is_extract_module(name)).map(|(n, t)| (n.clone(), t.clone()));
    Recipe::new(game, &s.packs, modules, entry, &s.dirs.0).map_err(|e| e.to_string())
}

/// Game `game` of the content directory `content`, loaded ([`load_battle`]), and its assets, extracted from `roms` by
/// its recipe ([`recipe`]): the whole of it in one call, for a tool. An error says what failed: the content (its
/// report's errors), its recipe, its ROMs or their extraction.
pub fn extract(content: &Path, game: &str, roms: &RomSource) -> Result<(nettai_battle::Content, Extraction), String> {
    let (c, _) = load_battle(content, game).map_err(|r| format!("{game}'s content doesn't load:\n{r}"))?;
    let recipe = recipe(&c)?;
    let extracted = recipe.extract(&roms.read(&recipe).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    Ok((c, extracted))
}

/// [`load_battle`]'s content before the define phase: the game's modules,
/// strings and background pool.
pub fn battle_content(content: &Path, game: &str) -> Result<(nettai_battle::Content, Report), Report> {
    let mut report = Report::default();
    let Some(read) = crate::index::read(content, &[game.to_string()], &mut report) else { return Err(report) };
    read_a_part();
    if report.has_errors() {
        return Err(report);
    }
    let scripts = read.scripts();
    // (Every game's backgrounds, from the content: the pool a round may be
    // drawn on, past the game's own.)
    let background_pool = crate::locale::background_pool(&read.dir);
    let c = nettai_battle::Content { scripts, strings: read.strings, background_pool, ..Default::default() };
    Ok((c, report))
}

/// What a game's assets, extracted from its ROMs (their index `index`, their graphics `graphics`), hold of `content`'s
/// (docs/frontend.md §1): each handle's number there, for whoever draws and plays them. Drawing looks a sprite's frame
/// up by the simulation's animation and frame, so each sprite the content defines must be the graphics' with as
/// many animations, each of as many frames (their durations are the content's: they time the battle); and every
/// other asset the content names must be in the index. Assets that aren't so are an error naming each one missing or
/// otherwise.
pub fn asset_numbers(content: &nettai_battle::Content, index: &AssetIndex, graphics: &Bundle) -> Result<(AssetNumbers, Report), Report> {
    let mut report = Report::default();
    let at = format!("{}'s assets", content.game());
    let numbers = match AssetNumbers::new(&content.assets, index) {
        Ok(n) => n,
        Err(missing) => {
            for m in missing {
                report.error(&at, format!("no {m}, which the content names"));
            }
            return Err(report);
        }
    };
    for (h, name) in content.assets.sprites.iter().enumerate() {
        let ours: Vec<usize> = content.animations.sprites.get(h).map_or(Vec::new(), |d| d.animations.iter().map(|a| a.frames.len()).collect());
        let id = numbers.sprites[h];
        let theirs: Vec<usize> = graphics.sprite(id.category, id.index).map_or(Vec::new(), |s| s.animations.iter().map(Vec::len).collect());
        if ours != theirs {
            report.error(
                &at,
                format!(
                    "sprite {name}: the content's animations have {ours:?} frames, and its sheet's {theirs:?} (drawing takes the sheet's frame by the content's animation and frame)"
                ),
            );
        }
    }
    if report.has_errors() {
        return Err(report);
    }
    Ok((numbers, report))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_assets::{SpriteFrame, SpriteSheet};

    /// The test content's sprites as sheets numbered as `nettai_battle::content::testing::asset_index` numbers them,
    /// each sprite's animations with `frames(name, animation)` frames (none: as the content's).
    fn graphics_of(c: &nettai_battle::Content, frames: impl Fn(&str, usize) -> Option<usize>) -> Bundle {
        let index = nettai_battle::content::testing::asset_index();
        let mut sprites: Vec<SpriteSheet> = Vec::new();
        for (h, name) in c.assets.sprites.iter().enumerate() {
            let id = index.sprites[name];
            let animations = c.animations.sprites[h].animations.iter().enumerate().map(|(i, a)| vec![SpriteFrame::default(); frames(name, i).unwrap_or(a.frames.len())]).collect();
            // (Several names may be one sheet: the last one's frames stand.)
            sprites.retain(|s| (s.category, s.index) != (id.category, id.index));
            sprites.push(SpriteSheet { category: id.category, index: id.index, animations, ..Default::default() });
        }
        sprites.sort_by_key(|s| (s.category, s.index));
        Bundle { sprites, ..Default::default() }
    }

    /// docs/frontend.md §1: a game's assets draw a content when they have every asset the content names, each sprite
    /// with the content's animations and as many frames in each (their durations the content's own); its numbers are
    /// the index's. Else the error names what is wrong.
    #[test]
    fn the_assets_draw_the_content_they_have_the_assets_of() {
        let c = nettai_battle::content::testing::content();
        let index = nettai_battle::content::testing::asset_index();
        let navi = c.assets.handle(nettai_content_api::AssetKind::Sprite, "test-navi").unwrap();
        let (numbers, report) = asset_numbers(&c, &index, &graphics_of(&c, |_, _| None)).unwrap_or_else(|r| panic!("{r}"));
        assert!(!report.has_errors(), "{report}");
        assert_eq!(numbers.sprite(navi), Some(nettai_battle::content::testing::NAVI_SPRITE));
        assert_eq!(numbers, nettai_battle::content::testing::asset_numbers(&c));
        // A sprite whose animation has another number of frames.
        let other = graphics_of(&c, |name, i| (name == "test-navi" && i == 3).then_some(5));
        let r = asset_numbers(&c, &index, &other).unwrap_err();
        assert!(r.to_string().contains("sprite test-navi: the content's animations have"), "{r}");
        // An asset the index hasn't.
        let mut fewer = index.clone();
        fewer.sounds.remove("test-stage-music");
        let r = asset_numbers(&c, &fewer, &other).unwrap_err();
        assert!(r.to_string().contains("no sound test-stage-music, which the content names"), "{r}");
    }
}
