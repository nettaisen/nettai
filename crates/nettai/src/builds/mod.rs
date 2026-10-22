//! A player's builds (docs/app.md §8): each a side of one game they made, named and kept. What any screen
//! needs of them is here, with no screen's code:
//!
//! - the files ([`store`]: a build's name, its side, what is kept aside while its navi takes none of it), and
//!   a game's builds listed and checked for a list ([`shelf`]);
//! - a build from a save ([`import`]);
//! - what a screen shows of one: the overview's model ([`overview`]), the pictures of what it holds
//!   ([`pictures`]), and a typed problem in the player's words ([`words`]); a build has no version
//!   (docs/design/versions.md §3.3: a new build's starting group has its tag, [`Kit::version_tags`]);
//! - a game's kit for all of that ([`Kit`], [`kit`]): its builds data (`nettai_match::build::Builds`), its
//!   pictures and its library's order, made once a game;
//! - its edits ([`edit`]: the folder's, a list's), the library's order ([`order`]), and the views of the tabs to
//!   come, kept from the first app's creator: the NaviCust's grid ([`grid`]), a patch card's lines ([`cards`]),
//!   the auto battle data ([`auto`]).
//!
//! What a game says of its builds (its versions as groups in the order every list shows them, a new build from
//! one, the forms a build chooses, a build checked with its problems typed, its overview, a chip's row and its
//! sorts) is nettai-match's `build`.

use crate::games::{Names, Ready};
use crate::shell::Shell;
use nettai_battle::Content;
use nettai_content_api::Registry;
use nettai_match::build::{Builds, Choice, Group, Preset, TEXT_TABLE};
use nettai_match::ruleset::{Item, Rulesets};
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
pub mod party;
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
    /// The auto battle data's lists (read of the rules once).
    auto_layout: OnceCell<Option<auto::Layout>>,
    /// The game's rulesets (read of its rules once).
    rulesets: OnceCell<Option<Arc<Rulesets>>>,
}

impl Kit {
    pub fn content(&self) -> &Arc<Content> {
        self.ready.content()
    }

    /// The auto battle data's lists, as the game's rules lay them out (`auto::Layout`: their `MODULE`); none where
    /// its setup takes no such data.
    pub fn auto_layout(&self) -> Option<&auto::Layout> {
        self.auto_layout.get_or_init(|| auto::Layout::read(self.content(), module_data(self.content(), &self.game, auto::MODULE).as_ref())).as_ref()
    }

    /// The game's ruleset categories, groups and presets (`nettai_match::ruleset::Rulesets`); none where it says
    /// nothing of its rulesets (why is in the log).
    pub fn rulesets(&self) -> Option<Arc<Rulesets>> {
        self.rulesets
            .get_or_init(|| match Rulesets::of(self.content(), &self.game) {
                Ok(r) => Some(Arc::new(r)),
                Err(why) => {
                    crate::errors::log(&format!("reading {}'s rulesets", self.game), &why);
                    None
                }
            })
            .clone()
    }

    /// A version's (a group's) name, as the game's strings have it in `names`' language (its id where they have
    /// none).
    pub fn version_name(&self, names: &Names, id: &str) -> String {
        let key = self.builds.group(id).map(|g| g.text_key());
        key.and_then(|k| names.text(TEXT_TABLE, &k)).unwrap_or_else(|| id.to_string())
    }

    /// Every version's tag (a group's: its name in capitals, in its own color), in the game's order: what a new
    /// build starts from (Play's NEW BUILD).
    pub fn version_tags(&self, names: &Names) -> Vec<nettai_ui::VersionTag> {
        self.builds.groups.iter().map(|group| self.tag_of(names, group)).collect()
    }

    /// The tag of the version item `item` of `registry` is one's own (a Giga chip, a soul, a Cross, a Beast form: a
    /// definition whose `version` names a group; a card's variant), for its mark in the build editor's rows; none
    /// (no text) where it is no version's.
    pub fn version_tag_of(&self, names: &Names, registry: Registry, item: &Item) -> nettai_ui::VersionTag {
        let rulesets = self.rulesets();
        let id = rulesets.as_ref().and_then(|r| r.group_of(registry, item)).map(|g| g.id.clone());
        id.and_then(|id| self.builds.group(&id)).map(|g| self.tag_of(names, g)).unwrap_or_default()
    }

    fn tag_of(&self, names: &Names, group: &Group) -> nettai_ui::VersionTag {
        let [_, r, g, b] = group.color.to_be_bytes();
        nettai_ui::VersionTag { text: self.version_name(names, &group.id).into(), tone: slint::Color::from_rgb_u8(r, g, b) }
    }
}

/// Preset `preset` of choice `choice` (EXE5's Light or Dark), by its name in the game's strings in `names`' language
/// (its id where they have none).
pub fn preset_name(names: &Names, choice: &Choice, preset: &Preset) -> String {
    names.text(TEXT_TABLE, &choice.text_key(preset)).unwrap_or_else(|| preset.id.clone())
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
        auto_layout: OnceCell::new(),
        rulesets: OnceCell::new(),
    });
    KITS.with(|kits| kits.borrow_mut().insert(game.to_string(), kit.clone()));
    Some(kit)
}

/// What module `name` of `game`'s rules returned as the content loaded, as data (a function in it left out,
/// nothing called): the boards a grid draws, the lists the auto battle data are laid out in.
#[allow(dead_code)] // (The tabs to come read them.)
pub fn module_data(content: &Content, game: &str, name: &str) -> Option<nettai_content_api::Data> {
    nettai_battle::behavior::content_module_data(content, &format!("{game}:{name}"))?.ok()
}
