//! The order the creator's lists keep: the game's library order (its
//! pack's `library.toml`, `nettai_content::library`): the chips by the
//! game's library tabs in their order and in each the order its library
//! screen lists them, the entries of each of the game's collections in
//! theirs; what isn't in it after, by key.

use nettai_battle::Content;
use nettai_content::library::{self, Library, Rank};
use nettai_content_api::{ChipHandle, EntryHandle};
use std::collections::HashMap;
use std::path::Path;

/// A game's library order, and its chips' places by key.
#[derive(Clone, Debug, Default)]
pub struct Order {
    lib: Library,
    chips: HashMap<String, Rank>,
}

impl Order {
    /// The order of game `game` in content folder `dir`; by key alone where
    /// it has none (or its file is wrong).
    pub fn load(dir: &Path, game: &str) -> Order {
        match library::load(&dir.join(game)) {
            Ok(Some(lib)) => {
                let chips = lib.chip_ranks().into_iter().map(|(k, r)| (k.to_string(), r)).collect();
                Order { lib, chips }
            }
            _ => Order::default(),
        }
    }

    /// A chip's place in the library's order, by its key in its game (a library's section and the place in
    /// it: `nettai_match::build::LibraryRank`); none: no library lists it.
    pub fn chip_rank(&self, key: &str) -> Option<Rank> {
        self.chips.get(key).copied()
    }

    /// Sort chips (each with what is shown of it).
    pub fn chips<T>(&self, c: &Content, list: &mut [(T, ChipHandle)]) {
        library::sort_by_rank(list, |(_, h)| nettai_match::ids::local(&c.defs.chip(*h).key), |k| self.chips.get(k).copied());
    }

    /// An entry's place in collection `collection`'s order, from 0 (a card's number less one: the library lists
    /// the cards by their numbers); none: the order hasn't it.
    pub fn entry_place(&self, collection: &str, id: &str) -> Option<usize> {
        self.lib.entry_rank(collection, id).map(|(_, place)| place)
    }

    /// Sort entries of collection `collection`.
    pub fn entries<T>(&self, c: &Content, collection: &str, list: &mut [(T, EntryHandle)]) {
        library::sort_by_rank(list, |(_, h)| c.defs.entry(*h).id(), |k| self.lib.entry_rank(collection, k));
    }
}
