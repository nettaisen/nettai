//! What an EXE6 save unlocks on the custom screen, as the original keeps
//! it (event flags: Beast Out, and which of its version's five Crosses it
//! owns, by Cross number), and how a player's setup states it: EXE6's cross
//! and beast systems' setup (docs/design/rules-in-luau.md, S6c), which has
//! no flags. A setup states the Crosses a player has as a list of forms
//! (`crosses`); this boundary writes a save's as the list of those it owns
//! ([`Unlocks::owned_crosses`], [`Unlocks::write`]).
//!
//! Event flag 0x163 (a navi code received) isn't here: it is the save
//! system's `level` (`sub_800B144` sends a level only with the flag set), which
//! EXE6's rules read as the seal on Beast Out and the Cross window.

use crate::GameVersion;
use crate::forms;
use nettai_battle::content::Content;
use nettai_battle::custom::PlayerSetup;
use nettai_battle::rules::Fact;
use nettai_content_api::{FormHandle, NaviHandle, Registry, Value};

/// The Crosses a version has, which a save owns or not (and a Cross window
/// holds at most).
pub const CROSSES: usize = 5;

/// What a player's save unlocks on the custom screen, with the game it is
/// of.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Unlocks {
    pub version: GameVersion,
    /// The Crosses owned (Gregar's event flags 0xE2-0xE6, Falzar's
    /// 0xE7-0xEB), by Cross number.
    pub crosses: [bool; CROSSES],
    /// Beast Out is unlocked (event flag 0xE0).
    pub beast_out: bool,
}

impl Unlocks {
    /// Every Cross and Beast Out, as in a finished game.
    pub fn everything(version: GameVersion) -> Unlocks {
        Unlocks { version, crosses: [true; CROSSES], beast_out: true }
    }

    /// No Cross and no Beast Out, of `version`.
    pub fn nothing(version: GameVersion) -> Unlocks {
        Unlocks { version, crosses: [false; CROSSES], beast_out: false }
    }

    /// The Crosses the save has for a player who operates `navi`, as a
    /// setup states them: those of the navi's Crosses of the save's version
    /// (`forms::cross`, by Cross number) the save owns, in that order, with
    /// no gaps. None for a navi that doesn't change form.
    ///
    /// The original's window goes by Cross number (`sub_8029EF8`), and a
    /// save the game makes owns its Crosses from the first on, so the list
    /// is the window. Flags with a gap (a Cross owned above one that isn't:
    /// only a save written by hand) give a list that closes it.
    pub fn owned_crosses(&self, content: &Content, navi: NaviHandle) -> Vec<FormHandle> {
        (0..CROSSES as u8).filter(|&n| self.crosses[n as usize]).filter_map(|n| forms::cross(content, navi, self.version, n)).collect()
    }

    /// Write these into the setup of a player who operates `navi`: each
    /// fact into every system of the game's ruleset that takes it (EXE6's
    /// cross system the version and the Crosses, [`Unlocks::owned_crosses`];
    /// its beast system the version and Beast Out). A ruleset with none of
    /// EXE6's systems takes none of it.
    pub fn write(&self, content: &Content, navi: NaviHandle, player: &mut PlayerSetup) -> Result<(), String> {
        player.set_fact(content, "version", &[Fact::Name(self.version.name())])?;
        let crosses: Vec<Fact> = self.owned_crosses(content, navi).iter().map(|f| Fact::Value(Value::Def(Registry::Form, f.0))).collect();
        player.set_fact(content, "crosses", &crosses)?;
        player.set_fact(content, "beast_out", &[Fact::Value(Value::Bool(self.beast_out))])?;
        Ok(())
    }
}
