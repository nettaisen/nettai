//! A player's builds (docs/app.md §8): each a side of one game they made, named and kept. What any screen
//! needs of them is here, with no screen's code:
//!
//! - the files ([`store`]: a build's name, its side, what is kept aside while its navi takes none of it), and
//!   a game's builds listed and checked for a list ([`shelf`]);
//! - a build from a save ([`import`]);
//! - what a screen shows of one: the overview's model ([`overview`]), a version's tag ([`version_tag`]), the
//!   pictures of what it holds ([`pictures`]), and a typed problem in the player's words ([`words`]);
//! - a game's kit for all of that ([`Kit`], [`kit`]): its builds data (`nettai_match::build::Builds`), its
//!   pictures and its library's order, made once a game;
//! - its edits ([`edit`]: the folder's, a list's), the library's order ([`order`]), and the views of the tabs to
//!   come, kept from the first app's creator: the NaviCust's grid ([`grid`]), a patch card's lines ([`cards`]),
//!   the auto battle data ([`auto`]).
//!
//! What a game says of its builds (its versions in the order every list shows them, a new build, a build
//! checked with its problems typed, its overview, a chip's row and its sorts) is nettai-match's `build`.

use crate::games::{Names, Ready};
use crate::shell::Shell;
use nettai_battle::Content;
use nettai_content_api::ChipHandle;
use nettai_match::Side;
use nettai_match::build::{Builds, TEXT_TABLE};
use std::cell::{OnceCell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

// (What the tabs to come will call isn't called yet.)
#[allow(dead_code)]
pub mod auto;
#[allow(dead_code)]
pub mod cards;
#[allow(dead_code)]
pub mod edit;
#[allow(dead_code)]
pub mod grid;
pub mod import;
#[allow(dead_code)]
pub mod order;
pub mod overview;
pub mod pictures;
pub mod shelf;
pub mod store;
pub mod words;

/// What the builds' views need of a game, made once: its builds data, its pictures and its library's order.
pub struct Kit {
    pub game: String,
    pub ready: Rc<Ready>,
    pub builds: Builds,
    pub pictures: pictures::Pictures,
    pub order: order::Order,
    /// The chips only some versions' saves hold, each with those versions' ids (asked of the rules once).
    version_chips: OnceCell<HashMap<ChipHandle, Vec<String>>>,
}

impl Kit {
    pub fn content(&self) -> &Arc<Content> {
        self.ready.content()
    }

    /// The versions whose saves hold `chip`, where it is only some versions' (their ids, in the versions'
    /// order); none: every version's.
    pub fn versions_of(&self, chip: ChipHandle) -> Option<&[String]> {
        let chips = self.version_chips.get_or_init(|| {
            self.builds.version_chips(self.content()).unwrap_or_else(|why| {
                crate::errors::log(&format!("asking {} which chips are a version's", self.game), &why);
                HashMap::new()
            })
        });
        chips.get(&chip).map(Vec::as_slice)
    }

    /// A version's name, as the game's strings have it in `names`' language (its id where they have none).
    pub fn version_name(&self, names: &Names, id: &str) -> String {
        let key = self.builds.version(id).map(|v| v.text_key());
        key.and_then(|k| names.text(TEXT_TABLE, &k)).unwrap_or_else(|| id.to_string())
    }

    /// The tag of the version `side` is a build of: the version's name in capitals, in the version's own
    /// color (the game's builds data states both). A side of no version has no tag.
    pub fn version_tag(&self, names: &Names, side: &Side) -> nettai_ui::VersionTag {
        let Some(version) = self.builds.version_of(self.content(), side).and_then(|id| self.builds.version(id)) else { return Default::default() };
        let [_, r, g, b] = version.color.to_be_bytes();
        nettai_ui::VersionTag { text: self.version_name(names, &version.id).to_uppercase().into(), tone: slint::Color::from_rgb_u8(r, g, b) }
    }
}

thread_local! {
    /// Each game's kit, by the game's id.
    static KITS: RefCell<HashMap<String, Rc<Kit>>> = RefCell::new(HashMap::new());
}

/// The selected game's kit. None while the game loads, or if it says nothing of its builds (why is in the log).
pub fn kit(shell: &Shell) -> Option<Rc<Kit>> {
    let (game, ready) = shell.ready()?;
    kit_of(&game, &ready)
}

/// Game `game`'s kit, the game loaded.
pub fn kit_of(game: &str, ready: &Rc<Ready>) -> Option<Rc<Kit>> {
    if let Some(kit) = KITS.with(|kits| kits.borrow().get(game).cloned()) {
        return Some(kit);
    }
    let builds = match Builds::of(ready.content(), game) {
        Ok(builds) => builds,
        Err(why) => {
            crate::errors::log(&format!("reading {game}'s builds data"), &why);
            return None;
        }
    };
    let kit = Rc::new(Kit {
        game: game.to_string(),
        ready: ready.clone(),
        builds,
        pictures: pictures::Pictures::new(ready.clone()),
        order: order::Order::load(&ready.loaded.game.dir, game),
        version_chips: OnceCell::new(),
    });
    KITS.with(|kits| kits.borrow_mut().insert(game.to_string(), kit.clone()));
    Some(kit)
}

/// A side's version's tag ([`Kit::version_tag`]) in the language shown, for a screen that has no kit at hand.
/// A game that says nothing of its builds has no tag.
pub fn version_tag(shell: &Shell, game: &str, ready: &Rc<Ready>, side: &Side) -> nettai_ui::VersionTag {
    let graphics = ready.graphics(shell.lang);
    kit_of(game, ready).map(|kit| kit.version_tag(&Names::of(ready.content(), &graphics), side)).unwrap_or_default()
}

/// What module `name` of `game`'s rules returned as the content loaded, as data (a function in it left out,
/// nothing called): the boards a grid draws, the lists the auto battle data are laid out in.
#[allow(dead_code)] // (The tabs to come read them.)
pub fn module_data(content: &Content, game: &str, name: &str) -> Option<nettai_content_api::Data> {
    nettai_battle::behavior::content_module_data(content, &format!("{game}:{name}"))?.ok()
}
