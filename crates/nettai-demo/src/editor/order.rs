//! The order the editor's lists keep: the game's library order (its pack's
//! `library.toml`, `nettai_content::library`): the chips by the game's
//! library tabs in their order and in each the order its library screen
//! lists them, the NaviCust programs and patch cards by number; what isn't
//! in it after, by key. The panes sort their lists with it alone.

use nettai_battle::Content;
use nettai_content::library::{self, Library, Rank};
use nettai_content_api::{ChipHandle, NaviCustProgramHandle, PatchCardHandle};
use std::collections::HashMap;
use std::path::Path;

/// A game's library order, and its chips' places by key.
#[derive(Clone, Debug, Default)]
pub struct Order {
    lib: Library,
    chips: HashMap<String, Rank>,
}

impl Order {
    /// The order of game `game` in content `dir`; by key alone where it has
    /// none, and why it has none if the file is wrong.
    pub fn load(dir: &Path, game: &str) -> (Order, Option<String>) {
        match library::load(&dir.join(game)) {
            Ok(Some(lib)) => {
                let chips = lib.chip_ranks().into_iter().map(|(k, r)| (k.to_string(), r)).collect();
                (Order { lib, chips }, None)
            }
            Ok(None) => (Order::default(), None),
            Err(e) => (Order::default(), Some(e)),
        }
    }

    /// Sort a list of chips (each with what the pane shows of it).
    pub fn chips<T>(&self, c: &Content, list: &mut [(T, ChipHandle)]) {
        library::sort_by_rank(list, |(_, h)| local(&c.defs.chip(*h).key), |k| self.chips.get(k).copied());
    }

    /// Sort a list of patch cards.
    pub fn patch_cards<T>(&self, c: &Content, list: &mut [(T, PatchCardHandle)]) {
        library::sort_by_rank(list, |(_, h)| local(&c.defs.patch_card(*h).key), |k| self.lib.patch_card_rank(k));
    }

    /// Sort a list of NaviCust programs.
    pub fn navicust_programs<T>(&self, c: &Content, list: &mut [(T, NaviCustProgramHandle)]) {
        library::sort_by_rank(list, |(_, h)| local(&c.defs.navicust_program(*h).key), |k| self.lib.navicust_rank(k));
    }
}

/// A definition's key in its game (`exe6:cannon` is `cannon`).
fn local(key: &str) -> &str {
    nettai_match::ids::local(key)
}
