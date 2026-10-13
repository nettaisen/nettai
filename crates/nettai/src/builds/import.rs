//! A build from a save file: each game's compat crate reads its own game's saves (`exe6_compat::import`,
//! `exe5_compat::import`, `exe4_compat::import`: the original's bytes into the game's facts, by field name),
//! and the build's game picks which. A save of another game is refused, not switched to.
//!
//! What a save gives is then made a build, as the game's builds data says one is (`nettai_match::build`): the
//! save's version with all that version sets (its whole form list, its Giga chips), each choice on the preset
//! the save is on ([`Imported::taken`] says which, where that changed what the save held: a light/dark value
//! of 100 is the dark preset's), and the facts no build states at the rules' defaults (a save's HP, Regular
//! memory and deletion times are none of a build's).
//!
//! What the readers say of a save in words is for the log alone; what a player is told is typed
//! ([`Imported`], [`Refused`]).

use nettai_battle::content::Content;
use nettai_match::Side;
use nettai_match::build::{Builds, Taken};

/// A save, made a build's side.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Imported {
    pub side: Side,
    /// The presets the save was put on that changed what it held, each by its choice and its own id.
    pub taken: Vec<Taken>,
}

/// Why a file gives no build.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refused {
    /// It is no save of the build's game (another game's, or no save at all).
    NotThisGames,
    /// It is the game's save, and no build could be made of it: the log says why.
    Failed,
}

/// A side of game `game` from the save in `file` (a .sav's bytes, or a raw save image as Tango's netplay
/// templates hold), as a build of the game is.
pub fn side_of_save(content: &Content, builds: &Builds, file: &[u8]) -> Result<Imported, Refused> {
    let game = builds.game();
    let failed = |why: String| {
        crate::errors::log(&format!("importing a save of {game}"), &why);
        Refused::Failed
    };
    let mut side = Side::fresh(content, game).map_err(failed)?;
    // What the save holds, and its version (the id the game's builds data names it by).
    let (notes, version) = match game {
        exe6_compat::ROOT => {
            let save = read(game, file, exe6_compat::import::read)?;
            (exe6_compat::import::import(content, &mut side, &save).map_err(failed)?, save.version().name())
        }
        exe5_compat::ROOT => {
            let save = read(game, file, exe5_compat::import::read)?;
            (exe5_compat::import::import(content, game, &mut side, &save), save.version().name())
        }
        exe4_compat::ROOT => {
            let save = read(game, file, exe4_compat::import::read)?;
            (exe4_compat::import::import(content, game, &mut side, &save), save.version.name())
        }
        other => return Err(failed(format!("{other} has no save import"))),
    };
    for note in notes {
        crate::errors::log(&format!("importing a save of {game}"), &note);
    }
    builds.take_version(content, &mut side, version).map_err(failed)?;
    let taken = builds.settle(content, &mut side);
    let reset = builds.as_built(content, &mut side);
    if !reset.is_empty() {
        crate::errors::log(&format!("importing a save of {game}"), &format_args!("set to what every build has: {}", reset.join(", ")));
    }
    Ok(Imported { side, taken })
}

/// The auto battle data of the save in `file` alone, as `side`'s (its other facts kept).
#[allow(dead_code)] // (The auto battle tab's, which isn't built yet.)
pub fn auto_battle_of_save(content: &Content, builds: &Builds, file: &[u8], side: &mut Side) -> Result<(), Refused> {
    let game = builds.game();
    let said = match game {
        exe5_compat::ROOT => exe5_compat::import::auto_battle_of_save(content, game, file, side),
        other => Err(format!("{other}'s saves keep no auto battle data")),
    };
    match said {
        Ok(notes) => {
            for note in notes {
                crate::errors::log(&format!("importing a save's auto battle data of {game}"), &note);
            }
            builds.as_built(content, side);
            Ok(())
        }
        Err(why) => {
            crate::errors::log(&format!("importing a save's auto battle data of {game}"), &why);
            Err(Refused::NotThisGames)
        }
    }
}

fn read<S>(game: &str, file: &[u8], reader: fn(&[u8]) -> Result<S, String>) -> Result<S, Refused> {
    reader(file).map_err(|why| {
        crate::errors::log(&format!("reading a save of {game}"), &why);
        Refused::NotThisGames
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_match::build::Checked;
    use nettai_match::facts::Stated;
    use nettai_match::testing::{exe4_content, exe5_content, exe6_content};

    /// A build of each version of EXE4 and EXE5 has the six souls that version's save can have (its compat
    /// crate's soul flags, by its numbers of the forms): what the version sets is what an import of its save
    /// gives.
    #[test]
    fn each_versions_souls_are_its_saves() {
        let souls = |content: &Content, game: &str, version: &str, number: &dyn Fn(&str) -> Option<u8>| {
            let side = Builds::of(content, game).unwrap().new_build(content, version).unwrap();
            let mut numbers: Vec<u8> = side.facts.get(content, "souls").unwrap().defs().iter().map(|&h| {
                let key = nettai_match::ids::local(&content.defs.form(nettai_content_api::FormHandle(h)).key);
                number(key).unwrap_or_else(|| panic!("{game}: compat has no soul {key}"))
            }).collect();
            numbers.sort();
            numbers
        };
        let four = exe4_content();
        let compat = exe4_compat::Compat::exe4();
        for version in [exe4_compat::Version::RedSun, exe4_compat::Version::BlueMoon] {
            let theirs: Vec<u8> = (1..=12).filter(|&n| version.soul_flag(n).is_some()).collect();
            assert_eq!(souls(&four, "exe4", version.name(), &|k| compat.form_number(k)), theirs, "{}", version.name());
        }
        let five = exe5_content();
        let compat = exe5_compat::Compat::exe5();
        for version in [exe5_compat::Version::Protoman, exe5_compat::Version::Colonel] {
            let theirs: Vec<u8> = (1..=12).filter(|&n| version.soul_flag(n).is_some()).collect();
            assert_eq!(souls(&five, "exe5", version.name(), &|k| compat.form_number(k)), theirs, "{}", version.name());
        }
    }

    /// A save of the build's game gives a build of its version, with all the version sets; one of another
    /// game is refused. Its light/dark value takes the preset on its side of the game's line (100: dark), and
    /// the import says so, typed.
    #[test]
    fn a_save_gives_its_games_build() {
        let mut image = vec![0u8; exe5_compat::save::IMAGE_SIZE];
        image[0x29E0..0x29E0 + 20].copy_from_slice(b"REXE5TOB 20041006 US");
        image[0x554C..0x554C + 0xE0].fill(0xFF);
        image[0x52A8 + 0x44..0x52A8 + 0x46].copy_from_slice(&100u16.to_le_bytes());
        let five = exe5_content();
        let builds = Builds::of(&five, "exe5").unwrap();
        let imported = side_of_save(&five, &builds, &image).unwrap();
        let side = &imported.side;
        assert_eq!((side.facts.get(&five, "karma"), side.facts.get(&five, "hp")), (Some(Stated::Number(0)), Some(Stated::Number(997))));
        assert_eq!(imported.taken, [Taken { choice: "light_dark".into(), preset: "dark".into(), was: Some(100) }]);
        // (A Team ProtoMan save with no souls: its team's six, as every build of the version.)
        assert_eq!(builds.version_of(&five, side), Some("protoman"));
        let souls = side.facts.get(&five, "souls").unwrap().defs();
        let team: Vec<&str> = souls.iter().map(|&h| nettai_match::ids::local(&five.defs.form(nettai_content_api::FormHandle(h)).key)).collect();
        assert_eq!(team, ["protosoul", "gyrosoul", "searchsoul", "napalmsoul", "magnetsoul", "meddysoul"]);
        // (Its folder is the save's, an empty one: what is wrong with it is the folder's to say.)
        let checked = Checked::of(&five, "exe5", side);
        assert!(checked.problems.iter().all(|p| p.field.as_deref() == Some("folder")), "{:?}", checked.problems);
        let six = exe6_content();
        assert_eq!(side_of_save(&six, &Builds::of(&six, "exe6").unwrap(), &image), Err(Refused::NotThisGames));
    }
}
