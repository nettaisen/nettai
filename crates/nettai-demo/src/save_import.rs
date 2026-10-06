//! A side from a save file, by the match's game: each game's compat crate
//! imports its own game's saves (`exe6_compat::import`,
//! `exe5_compat::import`: the original's bytes into the game's facts, by
//! field name), and the arena's game picks which. A save of another game
//! than the match's is refused, not switched to: the arena picks the game.

use nettai_battle::content::Content;
use nettai_match::Match;

/// Fill side `side` of `m` from the save in `file` (a .sav's bytes, or a
/// raw save image as Tango's netplay templates hold) of the match's game.
/// What is worth saying about it, or why the file gives no side.
pub fn import_save(content: &Content, m: &mut Match, side: usize, file: &[u8]) -> Result<Vec<String>, String> {
    let game = m.game.clone();
    let s = &mut m.sides[side];
    match game.as_str() {
        exe6_compat::ROOT => exe6_compat::import::import(content, s, &read(&game, file, exe6_compat::import::read)?),
        exe5_compat::ROOT => Ok(exe5_compat::import::import(content, &game, s, &read(&game, file, exe5_compat::import::read)?)),
        other => Err(format!("{other} has no save import")),
    }
}

/// The auto battle data of the save in `file` (of the match's game) alone,
/// as side `side`'s: the editor's "From a save…".
pub fn auto_battle_of_save(content: &Content, m: &mut Match, side: usize, file: &[u8]) -> Result<Vec<String>, String> {
    let game = m.game.clone();
    match game.as_str() {
        exe5_compat::ROOT => exe5_compat::import::auto_battle_of_save(content, &game, file, &mut m.sides[side]),
        other => Err(format!("{other}'s saves keep no auto battle data")),
    }
}

/// A save of `game` read by its reader, or why the file is none.
fn read<S>(game: &str, file: &[u8], reader: fn(&[u8]) -> Result<S, String>) -> Result<S, String> {
    reader(file).map_err(|e| format!("not a save of {game}, the match's game ({e})"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_match::testing::{exe5_content, exe6_content};

    /// A save of the match's game gives the side; one of the other game
    /// is refused, the match left as it is.
    #[test]
    fn a_save_goes_into_the_matchs_game() {
        let mut image = vec![0u8; exe5_compat::save::IMAGE_SIZE];
        image[0x29E0..0x29E0 + 20].copy_from_slice(b"REXE5TOB 20041006 US");
        image[0x554C..0x554C + 0xE0].fill(0xFF);
        image[0x52A8 + 0x44..0x52A8 + 0x46].copy_from_slice(&100u16.to_le_bytes());
        let five = exe5_content();
        let mut m = Match::empty(&five, "exe5").unwrap();
        import_save(&five, &mut m, 0, &image).unwrap();
        assert_eq!(m.sides[0].facts.get(&five, "karma"), Some(nettai_match::facts::Stated::Number(100)));
        assert_eq!(auto_battle_of_save(&five, &mut m, 1, &image), Ok(Vec::new()));
        let six = exe6_content();
        let mut m = Match::empty(&six, "exe6").unwrap();
        let before = m.clone();
        let e = import_save(&six, &mut m, 0, &image).unwrap_err();
        assert!(e.starts_with("not a save of exe6, the match's game"), "{e}");
        assert_eq!(m, before);
        assert_eq!(auto_battle_of_save(&six, &mut m, 0, &image), Err("exe6's saves keep no auto battle data".into()));
    }
}
