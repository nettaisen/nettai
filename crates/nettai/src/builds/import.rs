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

/// A game's compat tables (`<content>/<game>/compat`), as its compat crate reads them: what a save of the game is
/// imported by, and the numbers its patch cards are printed with.
#[derive(Clone, Debug)]
pub enum Compat {
    Exe4(Box<exe4_compat::Compat>),
    Exe5(Box<exe5_compat::Compat>),
    Exe6(Box<exe6_compat::Compat>),
}

impl Compat {
    /// Game `game`'s compat tables in the content directory `content` (a bundle's, or the repository's content/),
    /// read; none where the game has no compat crate (no save import).
    pub fn of_content(game: &str, content: &std::path::Path) -> Option<Result<Compat, String>> {
        let said = |e: String| format!("{game}'s compat tables in {}: {e}", content.display());
        Some(match game {
            exe4_compat::ROOT => exe4_compat::Compat::of_content(content).map(|c| Compat::Exe4(Box::new(c))).map_err(said),
            exe5_compat::ROOT => exe5_compat::Compat::of_content(content).map(|c| Compat::Exe5(Box::new(c))).map_err(said),
            exe6_compat::ROOT => exe6_compat::Compat::of_content(content).map(|c| Compat::Exe6(Box::new(c))).map_err(said),
            _ => return None,
        })
    }
}

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
/// templates hold), as a build of the game is, by `compat`, the compat tables of the game's content.
pub fn side_of_save(compat: &Compat, content: &Content, builds: &Builds, file: &[u8]) -> Result<Imported, Refused> {
    let game = builds.game();
    let failed = |why: String| {
        crate::errors::log(&format!("importing a save of {game}"), &why);
        Refused::Failed
    };
    let mut side = Side::fresh(content, game).map_err(failed)?;
    // What the save holds: its version is the facts it gives (docs/design/versions.md §2.4: the souls or Crosses it
    // owns, EXE6's Cybeast, a Bass-Cross MegaMan card's variant), which its compat crate states.
    let notes = match compat {
        Compat::Exe6(compat) if game == exe6_compat::ROOT => {
            let save = read(game, file, exe6_compat::import::read)?;
            exe6_compat::import::import(compat, content, &mut side, &save).map_err(failed)?
        }
        Compat::Exe5(compat) if game == exe5_compat::ROOT => {
            let save = read(game, file, exe5_compat::import::read)?;
            exe5_compat::import::import(compat, content, game, &mut side, &save)
        }
        Compat::Exe4(compat) if game == exe4_compat::ROOT => {
            let save = read(game, file, exe4_compat::import::read)?;
            exe4_compat::import::import(compat, content, game, &mut side, &save)
        }
        _ => return Err(failed(format!("{game} has no save import"))),
    };
    for note in notes {
        crate::errors::log(&format!("importing a save of {game}"), &note);
    }
    let taken = builds.settle(content, &mut side);
    let reset = builds.as_built(content, &mut side);
    if !reset.is_empty() {
        crate::errors::log(&format!("importing a save of {game}"), &format_args!("set to what every build has: {}", reset.join(", ")));
    }
    Ok(Imported { side, taken })
}

/// The auto battle data of the save in `file` alone, as `side`'s (its other facts kept).
#[allow(dead_code)] // (The auto battle tab's, which isn't built yet.)
pub fn auto_battle_of_save(compat: &Compat, content: &Content, builds: &Builds, file: &[u8], side: &mut Side) -> Result<(), Refused> {
    let game = builds.game();
    let said = match compat {
        Compat::Exe5(compat) if game == exe5_compat::ROOT => exe5_compat::import::auto_battle_of_save(compat, content, game, file, side),
        _ => Err(format!("{game}'s saves keep no auto battle data")),
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

/// The number patch card `id` (its key in the game's cards) is printed with and a save holds, as the game's compat
/// tables state it (`compat`, its content's patch-cards.toml): what a screen shows as the card's number. None: a
/// game whose compat states no such card.
pub fn card_number(compat: &Compat, id: &str) -> Option<u8> {
    match compat {
        Compat::Exe6(compat) => compat.patch_card_number(id),
        Compat::Exe5(compat) => compat.patch_card_number(id),
        Compat::Exe4(compat) => compat.patch_card_number(id),
    }
}

fn read<S>(game: &str, file: &[u8], reader: fn(&[u8]) -> Result<S, String>) -> Result<S, Refused> {
    reader(file).map_err(|why| {
        crate::errors::log(&format!("reading a save of {game}"), &why);
        Refused::NotThisGames
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use nettai_match::build::Checked;
    use nettai_match::facts::Stated;
    use nettai_match::testing::{exe4_content, exe5_content, exe6_content};

    /// `game`'s compat tables, as the repository's content has them.
    pub(crate) fn compat(game: &str) -> Compat {
        Compat::of_content(game, &nettai_content::index::content()).expect("a game with a compat crate").unwrap()
    }

    /// A build from each version's group of EXE4 and EXE5 has the six souls that version's save can have (its
    /// compat crate's soul flags, by its numbers of the forms): the forms whose own `version` is the group's.
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

    /// A save of the build's game gives a build of what it holds (its souls, docs/design/versions.md §2.4); one
    /// of another game is refused. Its light/dark value takes the preset on its side of the game's line (100:
    /// dark), and the import says so, typed.
    #[test]
    fn a_save_gives_its_games_build() {
        let mut image = vec![0u8; exe5_compat::save::IMAGE_SIZE];
        image[0x29E0..0x29E0 + 20].copy_from_slice(b"REXE5TOB 20041006 US");
        image[0x554C..0x554C + 0xE0].fill(0xFF);
        image[0x52A8 + 0x44..0x52A8 + 0x46].copy_from_slice(&100u16.to_le_bytes());
        let five = exe5_content();
        let builds = Builds::of(&five, "exe5").unwrap();
        let imported = side_of_save(&compat("exe5"), &five, &builds, &image).unwrap();
        let side = &imported.side;
        assert_eq!((side.facts.get(&five, "karma"), side.facts.get(&five, "hp")), (Some(Stated::Number(0)), Some(Stated::Number(997))));
        assert_eq!(imported.taken, [Taken { choice: "light_dark".into(), preset: "dark".into(), was: Some(100) }]);
        // (A Team ProtoMan save without Soul Unison: no souls.)
        assert!(side.facts.get(&five, "souls").unwrap().defs().is_empty());
        // (Its folder is the save's, an empty one: what is wrong with it is the folder's to say.)
        let checked = Checked::of(&five, "exe5", side);
        assert!(checked.problems.iter().all(|p| p.field.as_deref() == Some("folder")), "{:?}", checked.problems);
        let six = exe6_content();
        assert_eq!(side_of_save(&compat("exe6"), &six, &Builds::of(&six, "exe6").unwrap(), &image), Err(Refused::NotThisGames));
    }
}
