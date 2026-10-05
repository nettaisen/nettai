//! What an EXE6 player's save unlocks on the custom screen, and nettai's Cross
//! list: EXE6's cross and beast systems' setup (docs/design/rules-in-luau.md,
//! S6c). A tool states these facts once ([`Unlocks::write`]) and each
//! system that takes one gets it by name; a reader (the frontend's EXE6 look)
//! reads them back ([`Unlocks::of`]).
//!
//! Event flag 0x163 (a navi code received) isn't here: it is the setup's
//! `navi_level` (`sub_800B144` sends a level only with the flag set), which
//! EXE6's rules read as the seal on Beast Out and the Cross window.

use nettai_battle::Battle;
use crate::forms;
use nettai_battle::content::Content;
use nettai_battle::custom::{GameVersion, PlayerSetup};
use nettai_battle::rules::Fact;
use nettai_content_api::{FieldType, FieldValue, FormHandle, NaviHandle, Registry, Value};

/// The Crosses a game's window holds (and a Cross list at most).
pub const CROSSES: usize = 5;

/// EXE6's game root, whose ruleset a setup plays by.

/// EXE6's systems that take these facts.
const CROSS_SYSTEM: &str = "cross";
const BEAST_SYSTEM: &str = "beast";

/// The Crosses a setup names for a player's Cross window
/// ([`Unlocks::cross_list`]): up to five forms, each a Cross, which the
/// window offers in this order (those not used this round, and not the
/// navi's starting form). nettai's extension: the original's window offers
/// its version's five (docs/engine/custom-screen.md §4.1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct CrossList {
    forms: [Option<FormHandle>; CROSSES],
}

impl CrossList {
    /// The list of `forms`, at most the window's five.
    pub fn new(forms: &[FormHandle]) -> CrossList {
        assert!(forms.len() <= CROSSES, "a Cross window offers at most {CROSSES} Crosses, not {}", forms.len());
        let mut list = CrossList::default();
        for (slot, &f) in list.forms.iter_mut().zip(forms) {
            *slot = Some(f);
        }
        list
    }

    /// The Cross in place `place`.
    pub fn get(&self, place: u8) -> Option<FormHandle> {
        self.forms.get(place as usize).copied().flatten()
    }

    /// The Crosses, in order.
    pub fn forms(&self) -> impl Iterator<Item = FormHandle> + '_ {
        self.forms.iter().flatten().copied()
    }
}

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
    /// The Crosses the setup names for the Cross window, in place of the
    /// version's that `crosses` owns: nettai's extension, which the
    /// original has no way to say (any Crosses, of either game). None: the
    /// original's.
    pub cross_list: Option<CrossList>,
}

impl Unlocks {
    /// Every Cross and Beast Out, as in a finished game.
    pub fn everything(version: GameVersion) -> Unlocks {
        Unlocks { version, crosses: [true; CROSSES], beast_out: true, cross_list: None }
    }

    /// No Cross and no Beast Out, of `version`.
    pub fn nothing(version: GameVersion) -> Unlocks {
        Unlocks { version, crosses: [false; CROSSES], beast_out: false, cross_list: None }
    }

    /// [`Unlocks::of`], or, for a side whose ruleset has none of EXE6's
    /// systems (EXE5's), nothing unlocked, of Falzar's look.
    pub fn of_side(b: &Battle, side: u8) -> Unlocks {
        Unlocks::of(b, side).unwrap_or(Unlocks::nothing(GameVersion::Falzar))
    }

    /// Write these into `player`'s setup: each fact into every system of
    /// the game's ruleset that takes it (EXE6's cross system the version,
    /// the Crosses and the list; its beast system the version, Beast Out
    /// and the list). A ruleset with none of EXE6's systems takes none of
    /// it.
    pub fn write(&self, content: &Content, player: &mut PlayerSetup) -> Result<(), String> {
        let version = match self.version {
            GameVersion::Gregar => "gregar",
            GameVersion::Falzar => "falzar",
        };
        player.set_fact(content, "version", &[Fact::Name(version)])?;
        let crosses: Vec<Fact> = self.crosses.iter().map(|&b| Fact::Value(Value::Bool(b))).collect();
        player.set_fact(content, "crosses", &crosses)?;
        player.set_fact(content, "beast_out", &[Fact::Value(Value::Bool(self.beast_out))])?;
        let list: Vec<Fact> =
            self.cross_list.iter().flat_map(|l| l.forms()).map(|f| Fact::Value(Value::Def(Registry::Form, f.0))).collect();
        player.set_fact(content, "cross_list", &list)?;
        Ok(())
    }

    /// What side `side` brought, read back from its EXE6 systems' setup
    /// (the cross system's version, Crosses and list, the beast system's
    /// Beast Out); none when its ruleset has no EXE6 cross system.
    pub fn of(b: &Battle, side: u8) -> Option<Unlocks> {
        let (schema, block) = b.system_setup(side, CROSS_SYSTEM)?;
        let field = |name: &str| schema.index_of(name);
        let version = match block.get(schema, field("version")?) {
            FieldValue::Enum(i) => match &schema.field(field("version")?).ty {
                FieldType::Enum(names) if names.get(i as usize).map(String::as_str) == Some("gregar") => GameVersion::Gregar,
                _ => GameVersion::Falzar,
            },
            _ => return None,
        };
        let mut crosses = [false; CROSSES];
        for (k, c) in crosses.iter_mut().enumerate() {
            *c = block.get_elem(schema, field("crosses")?, k) == Some(FieldValue::Bool(true));
        }
        let forms: Vec<FormHandle> = (0..CROSSES)
            .filter_map(|k| match block.get_elem(schema, field("cross_list")?, k) {
                Some(FieldValue::Ref(Some((Registry::Form, h)))) => Some(FormHandle(h)),
                _ => None,
            })
            .collect();
        let beast_out = b.system_setup(side, BEAST_SYSTEM).is_some_and(|(schema, block)| {
            schema.index_of("beast_out").is_some_and(|i| block.get(schema, i) == FieldValue::Bool(true))
        });
        Some(Unlocks { version, crosses, beast_out, cross_list: (!forms.is_empty()).then(|| CrossList::new(&forms)) })
    }

    /// The Cross in place `place` of the player's Crosses, the places the
    /// Cross window's entries and the round's record of Crosses used go
    /// by: the setup's list's entry, else the version's Cross with that
    /// number (none: the content has no such Cross).
    pub fn cross_at(&self, content: &Content, navi: NaviHandle, place: u8) -> Option<FormHandle> {
        match &self.cross_list {
            Some(list) => list.get(place),
            None => forms::cross(content, navi, self.version, place),
        }
    }

    /// Whether the player has the Cross in place `place`: the save owns
    /// it, or the setup's list names one there.
    pub fn owns_cross(&self, place: u8) -> bool {
        match &self.cross_list {
            Some(list) => list.get(place).is_some(),
            None => self.crosses.get(place as usize).copied().unwrap_or(false),
        }
    }

    /// The Beast form Beast Out takes a navi in `form` to (`sub_802937A`,
    /// `sub_802A040`; the beast system's rule): when `tired`, Beast Over
    /// (of `beast_game`'s game); from the base form the version's Beast
    /// Out; from a Cross that Cross's form in Beast Out (with a setup's
    /// Cross list, whichever game the Cross is from: HeatCross's Beast for
    /// a Falzar player in HeatCross).
    pub fn beast_form(&self, content: &Content, navi: NaviHandle, form: FormHandle, tired: bool) -> Option<FormHandle> {
        if tired {
            forms::beast_over(content, navi, self.beast_game(content, form))
        } else if content.form(form).base {
            forms::beast_out(content, navi, self.version)
        } else {
            forms::in_beast_out(content, form)
        }
    }

    /// The game of the Beast a navi in `form` goes into, or is in (the
    /// beast system's rule): the player's version, except that with a
    /// setup's Cross list a form of the other game (one of its Crosses, or
    /// a Beast form of one) is that game's. Beast Over and the custom
    /// screen's Beast Out roar follow it, and a frontend draws the Beast
    /// Out button and pictures of its game.
    pub fn beast_game(&self, content: &Content, form: FormHandle) -> GameVersion {
        match forms::game(content, form) {
            Some(game) if self.cross_list.is_some() && !content.form(form).base => game,
            _ => self.version,
        }
    }
}
