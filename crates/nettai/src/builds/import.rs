//! A build from a save file: each game's compat crate reads its own game's
//! saves (`exe6_compat::import`, `exe5_compat::import`: the original's
//! bytes into the game's facts, by field name), and the build's game picks
//! which. A save of another game is refused, not switched to.

use nettai_battle::content::Content;
use nettai_match::Side;

/// A side of game `game` from the save in `file` (a .sav's bytes, or a raw
/// save image as Tango's netplay templates hold), and what is worth saying
/// about it; or why the file gives none.
pub fn side_of_save(content: &Content, game: &str, file: &[u8]) -> Result<(Side, Vec<String>), String> {
    let mut side = Side::fresh(content, game)?;
    let mut notes = match game {
        exe6_compat::ROOT => exe6_compat::import::import(content, &mut side, &read(game, file, exe6_compat::import::read)?)?,
        exe5_compat::ROOT => exe5_compat::import::import(content, game, &mut side, &read(game, file, exe5_compat::import::read)?),
        other => return Err(format!("{other} has no save import")),
    };
    // (What a build in the app never states is the rules' defaults: a save's
    // HP, Regular memory, times and patterns are none of a build's. What it
    // states as a preset is the preset on its side: EXE5's light/dark value.)
    notes.extend(built(content, game, &mut side));
    Ok((side, notes))
}

/// The side set to what a build in the app is (`layout::as_built`), and
/// what that changed, said.
fn built(content: &Content, game: &str, side: &mut Side) -> Vec<String> {
    let built = crate::builds::layout::as_built(content, game, side);
    let mut notes: Vec<String> = built.taken.iter().map(|t| format!("{}, as every build in the app", t.said())).collect();
    if !built.reset.is_empty() {
        notes.push(format!("set to the defaults, as every build in the app: {}", built.reset.join(", ")));
    }
    notes
}

/// Whether game `game`'s saves can be read.
pub fn reads_saves(game: &str) -> bool {
    matches!(game, exe6_compat::ROOT | exe5_compat::ROOT)
}

/// The auto battle data of the save in `file` alone, as `side`'s (its other
/// facts kept).
pub fn auto_battle_of_save(content: &Content, game: &str, file: &[u8], side: &mut Side) -> Result<Vec<String>, String> {
    let mut notes = match game {
        exe5_compat::ROOT => exe5_compat::import::auto_battle_of_save(content, game, file, side)?,
        other => return Err(format!("{other}'s saves keep no auto battle data")),
    };
    notes.extend(built(content, game, side));
    Ok(notes)
}

fn read<S>(game: &str, file: &[u8], reader: fn(&[u8]) -> Result<S, String>) -> Result<S, String> {
    reader(file).map_err(|e| format!("not a save of {game} ({e})"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_match::testing::{exe5_content, exe6_content};

    /// A save of the build's game gives the side; one of the other game is
    /// refused. Its light/dark value takes the preset on its side of the
    /// game's line (100: dark), and the import says so.
    #[test]
    fn a_save_gives_its_games_side() {
        use nettai_match::facts::Stated;
        let mut image = vec![0u8; exe5_compat::save::IMAGE_SIZE];
        image[0x29E0..0x29E0 + 20].copy_from_slice(b"REXE5TOB 20041006 US");
        image[0x554C..0x554C + 0xE0].fill(0xFF);
        image[0x52A8 + 0x44..0x52A8 + 0x46].copy_from_slice(&100u16.to_le_bytes());
        let five = exe5_content();
        let (side, notes) = side_of_save(&five, "exe5", &image).unwrap();
        assert_eq!((side.facts.get(&five, "karma"), side.facts.get(&five, "hp")), (Some(Stated::Number(0)), Some(Stated::Number(997))));
        assert!(notes.iter().any(|n| n == "karma 100 is dark's: the dark preset (hp 997, karma 0), as every build in the app"), "{notes:?}");
        let six = exe6_content();
        let e = side_of_save(&six, "exe6", &image).unwrap_err();
        assert!(e.starts_with("not a save of exe6"), "{e}");
    }
}
