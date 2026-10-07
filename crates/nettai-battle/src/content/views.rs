//! What a frontend draws of the rules' own on the custom screen, by what
//! the engine names it (docs/design/rules-in-luau.md §4.8). A frontend has
//! drawing of its own for some of what content declares: a window with a
//! list of forms, an icon flying to the picked column, a button that offers
//! a form. Content says which of them a window or a button is, in its
//! definition (`view`), by one of these names; the names are checked as the
//! content loads, and the frontend reads the values, never the rules', a
//! window's or a button's own name.
//!
//! A view shows what the rules keep: state fields the frontend reads by
//! the names given here (a view's contract with the rules). They are found
//! once, as the content loads ([`ViewFields`]), and rules that have the
//! view without a field, or with one of another type, is refused there.
//! Nothing here is state of its own.

use nettai_content_api::{FieldPath, FieldType, Registry, Schema};

/// What a frontend draws while a custom-screen window of the rules is up (the
/// window's `view`; a window with none shows the screen as it is).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WindowView {
    /// A list of forms to choose from, unfolding (EXE6's Cross window as it
    /// opens, `sub_8027834`).
    FormListOpening,
    /// The list up: its entries' names, the one under the cursor in its own
    /// look (EXE6's Cross window, `sub_802794A`; its descriptions too).
    FormList,
    /// The list folding away (`sub_802790C`).
    FormListClosing,
    /// A form of the list chosen: the screen whitens and the form's face is
    /// put on (`sub_8027A58`).
    FormChosen,
    /// The form a button offers flies to the picked column as its icon
    /// (EXE5's soul's choice, the screen's state 9).
    OfferFlight,
    /// The chip a button shows flies to the picked column as its icon
    /// (EXE5's capsule's mix, the screen's state 0x3C).
    ChipFlight,
}

impl WindowView {
    pub const ALL: &'static [WindowView] = &[
        WindowView::FormListOpening,
        WindowView::FormList,
        WindowView::FormListClosing,
        WindowView::FormChosen,
        WindowView::OfferFlight,
        WindowView::ChipFlight,
    ];

    /// Its name in a window's definition.
    pub fn name(self) -> &'static str {
        match self {
            WindowView::FormListOpening => "form_list_opening",
            WindowView::FormList => "form_list",
            WindowView::FormListClosing => "form_list_closing",
            WindowView::FormChosen => "form_chosen",
            WindowView::OfferFlight => "offer_flight",
            WindowView::ChipFlight => "chip_flight",
        }
    }

    pub fn named(name: &str) -> Option<WindowView> {
        WindowView::ALL.iter().copied().find(|v| v.name() == name)
    }
}

/// What a frontend draws of a custom-screen button of the rules besides its
/// look (the button's `view`; a button with none is its pack's look alone).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ButtonView {
    /// It offers a form (EXE5's soul button): picked, the picked column
    /// shows the offered form's icon, and its picture in the chip window is
    /// in its second palette for the form's alternate offer (a Chaos
    /// Unison).
    FormOffer,
    /// Its picture in the chip window is a chip's too: the chip the
    /// rules' role `beast_out` names shows it in place of art of its own
    /// (EXE6's Beast Out button and BeastOut chip).
    ChipPicture,
}

impl ButtonView {
    pub const ALL: &'static [ButtonView] = &[ButtonView::FormOffer, ButtonView::ChipPicture];

    /// Its name in a button's definition.
    pub fn name(self) -> &'static str {
        match self {
            ButtonView::FormOffer => "form_offer",
            ButtonView::ChipPicture => "chip_picture",
        }
    }

    pub fn named(name: &str) -> Option<ButtonView> {
        ButtonView::ALL.iter().copied().find(|v| v.name() == name)
    }
}

/// What a player brings that a frontend shows their console by, or a tool
/// fills in for a person: a field of the rules' setup, by this name
/// (`Battle::fact`). It is read of the
/// rules' setup; a game whose rules don't declare it has none.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlayerFact {
    /// The navi they play (a definition): the round's stats are its
    /// (`Battle::new` holds the stats' navi to it, where it is stated).
    Navi,
    /// Their folder, as their save keeps it: a list of `{ chip, code }`,
    /// an entry with no chip empty (a folder being made). A tool deals the
    /// round's battle folder from it (`Match::round`), and a recording
    /// gives the battle folder dealt.
    Folder,
    /// Their Regular chip, an entry of the folder (from 0), dealt first;
    /// none: no Regular chip.
    RegularChip,
    /// Their two tag chips, entries of the folder (from 0), dealt next to
    /// each other (EXE6's); an empty list: none.
    TagChips,
    /// The version of the game they play, one of the names its field lists
    /// (EXE6's "gregar" or "falzar": the version's own pictures).
    Version,
    /// The forms they have for their form list, in its order: the forms a
    /// form list window offers (EXE6's Crosses, up to five of either
    /// version, its setup field `crosses`): an empty list is none.
    FormList,
    /// The level their navi has (EXE6's: the navi code the save received,
    /// 0 to 14, none without one; EXE5's team navis': the story's
    /// progress): a link navi's chip bonus and charge limits, the damage of
    /// the chips that go by it (`DamageFormula::NaviLevel`, `Level`). None:
    /// no level (0xFF in the battle).
    Level,
    /// The base HP their save holds for their navi (MegaMan's, which HP
    /// Memories raise; EXE5's team navis', the story's at their progress):
    /// a game's rules/save writes it into the stats as the round is set
    /// up. A tool fills it in from what a navi's level gives (EXE5's
    /// `story`).
    BaseHp,
}

impl PlayerFact {
    pub const ALL: &'static [PlayerFact] =
        &[
            PlayerFact::Navi,
            PlayerFact::Folder,
            PlayerFact::RegularChip,
            PlayerFact::TagChips,
            PlayerFact::Version,
            PlayerFact::FormList,
            PlayerFact::Level,
            PlayerFact::BaseHp,
        ];

    /// Its name: the setup field that holds it is the one the rules' setup
    /// gives this role (`schema.role(name, T)`), else the field of this
    /// name (`Defs::fact_name`: the field's own name).
    pub fn name(self) -> &'static str {
        match self {
            PlayerFact::Navi => "navi",
            PlayerFact::Folder => "folder",
            PlayerFact::RegularChip => "regular_chip",
            PlayerFact::TagChips => "tag_chips",
            PlayerFact::Version => "version",
            PlayerFact::FormList => "form_list",
            PlayerFact::Level => "level",
            PlayerFact::BaseHp => "hp",
        }
    }

    /// Whether a setup field of type `ty` holds the fact, and what it must
    /// be if not.
    pub(crate) fn fits(self, ty: &FieldType) -> Result<(), &'static str> {
        let record_of = |ty: &FieldType, want: &[(&str, fn(&FieldType) -> bool)]| match ty {
            FieldType::Record(fields) => {
                fields.fields().len() == want.len() && want.iter().all(|(name, ok)| fields.index_of(name).is_some_and(|i| ok(&fields.field(i).ty)))
            }
            _ => false,
        };
        let ok = match self {
            PlayerFact::Navi => matches!(ty, FieldType::Ref(Registry::Navi, _)),
            PlayerFact::Folder => match ty {
                FieldType::List(elem, _) => {
                    record_of(elem, &[("chip", |t| matches!(t, FieldType::Ref(Registry::Chip, _))), ("code", |t| matches!(t, FieldType::Code))])
                }
                _ => false,
            },
            PlayerFact::RegularChip => matches!(ty, FieldType::OptionalU8),
            PlayerFact::TagChips => matches!(ty, FieldType::List(e, 2) if matches!(**e, FieldType::U8)),
            PlayerFact::Version => matches!(ty, FieldType::Enum(_)),
            PlayerFact::FormList => matches!(ty, FieldType::Array(e, _) if matches!(**e, FieldType::Ref(Registry::Form, _))),
            PlayerFact::Level => matches!(ty, FieldType::OptionalU8),
            PlayerFact::BaseHp => matches!(ty, FieldType::U16),
        };
        if ok {
            return Ok(());
        }
        Err(match self {
            PlayerFact::Navi => "a navi",
            PlayerFact::Folder => "a list of { chip = \"chip\", code = \"code\" }",
            PlayerFact::RegularChip => "a u8? (an entry of the folder, or none)",
            PlayerFact::TagChips => "a list of two u8 (entries of the folder)",
            PlayerFact::Version => "a list of the versions' names",
            PlayerFact::FormList => "an array of forms",
            PlayerFact::Level => "a u8? (a level, or none)",
            PlayerFact::BaseHp => "a u16",
        })
    }
}

/// What a frontend reads of a navi's stats of its game's own (the rules'
/// `stats`, `NaviStats::game`) by the engine's name for it: the stat the
/// rules give the role (`schema.role(name, T)`), else the stat of the name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StatRole {
    /// The count the emotion window's box shows beside the navi's face,
    /// where the HUD shows one (EXE6's Beast Out turns left, its
    /// `beast_out_counter`).
    WindowCount,
}

impl StatRole {
    pub const ALL: &'static [StatRole] = &[StatRole::WindowCount];

    pub fn name(self) -> &'static str {
        match self {
            StatRole::WindowCount => "window_count",
        }
    }

    /// Whether a stat of type `ty` holds the role, and what it must be if
    /// not.
    pub(crate) fn fits(self, ty: &FieldType) -> Result<(), &'static str> {
        match self {
            StatRole::WindowCount if matches!(ty, FieldType::U8) => Ok(()),
            StatRole::WindowCount => Err("a u8"),
        }
    }
}

/// A form list's fields in the rules' state (the views `form_list_opening`,
/// `form_list`, `form_list_closing` and `form_chosen`), each the field of
/// its role (`schema.role`): the places of the forms offered among the
/// player's (`form_list.offered`, a u8 array) and how many
/// (`form_list.count`), which entries are marked (`form_list.marked`, a
/// bool array), the entry under the cursor (`form_list.cursor`), and
/// whether one is chosen (`form_list.chosen`, a bool) and its place
/// (`form_list.chosen_place`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FormListFields {
    pub offered: FieldPath,
    pub count: FieldPath,
    pub marked: FieldPath,
    pub cursor: FieldPath,
    pub chosen_set: FieldPath,
    pub chosen: FieldPath,
}

/// A form offer's fields in the rules' state (the button view
/// `form_offer`), each the field of its role: the form on offer, none for
/// no offer (`form_offer.form`, a form), whether the offer is the form's
/// alternate (`form_offer.alternate`, a bool), and the turns left in the
/// form it gave (`form_offer.turns`), which the emotion window counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct OfferFields {
    pub form: FieldPath,
    pub alternate: FieldPath,
    pub turns: FieldPath,
}

/// A flight's fields in the rules' state, each the field of its role: its
/// step and its count in the step. The offer's (the window view
/// `offer_flight`: `offer_flight.step`, `offer_flight.count`), or a chip's
/// (`chip_flight`: `chip_flight.step`, `chip_flight.count`, and
/// `chip_flight.button`, which of the screen's buttons that show a chip,
/// counted from 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FlightFields {
    pub step: FieldPath,
    pub count: FieldPath,
    pub button: Option<FieldPath>,
}

/// The state fields a frontend reads of the rules for their windows' and
/// buttons' views, each found by its name as the content loads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ViewFields {
    pub form_list: Option<FormListFields>,
    pub offer: Option<OfferFields>,
    pub offer_flight: Option<FlightFields>,
    pub chip_flight: Option<FlightFields>,
}

/// A type a view's field must be.
#[derive(Clone, Copy)]
enum Want {
    U8,
    Bool,
    U8s,
    Bools,
    Form,
}

impl Want {
    fn fits(self, ty: &FieldType) -> bool {
        match (self, ty) {
            (Want::U8, FieldType::U8) | (Want::Bool, FieldType::Bool) | (Want::Form, FieldType::Ref(Registry::Form, _)) => true,
            (Want::U8s, FieldType::Array(e, _)) => **e == FieldType::U8,
            (Want::Bools, FieldType::Array(e, _)) => **e == FieldType::Bool,
            _ => false,
        }
    }

    fn says(self) -> &'static str {
        match self {
            Want::U8 => "a u8",
            Want::Bool => "a bool",
            Want::U8s => "a u8 array",
            Want::Bools => "a bool array",
            Want::Form => "a form",
        }
    }
}

impl ViewFields {
    /// The fields the views of the rules' windows and buttons read of their
    /// state (`state`), each the field the declaration gives the view's
    /// role (`schema.role("form_list.offered", "u8[5]")`), never one by a
    /// game's name for it. `Err`: a view's role no field has, or a field of
    /// it of another type, said with the view's name and the window or
    /// button that has it.
    pub(crate) fn of<'a>(
        state: &Schema,
        windows: impl Iterator<Item = (&'a str, WindowView)>,
        buttons: impl Iterator<Item = (&'a str, ButtonView)>,
    ) -> Result<ViewFields, String> {
        let mut fields = ViewFields::default();
        // (A field by its role, the state's own or one of its records': the
        // one of the role there is.)
        let find = |who: &str, view: &str, role: &str, want: Want| -> Result<FieldPath, String> {
            let found = state.find_role(role).map_err(|e| format!("{who} has the view `{view}`, which shows the state field of the role `{role}`: {e}"))?;
            let Some(path) = found else {
                return Err(format!(
                    "{who} has the view `{view}`, which shows the state field of the role `{role}` ({}): the rules' state has none (`schema.role(\"{role}\", T)`)",
                    want.says()
                ));
            };
            let ty = state.place_of(path).ty();
            if !want.fits(ty) {
                return Err(format!(
                    "{who} has the view `{view}`, which shows the state field of the role `{role}` (`{}`) as {}: it is {ty:?}",
                    state.path_name(path),
                    want.says()
                ));
            }
            Ok(path)
        };
        for (name, view) in windows {
            let who = format!("window `{name}`");
            let f = |field: &str, want: Want| find(&who, view.name(), field, want);
            match view {
                WindowView::FormListOpening | WindowView::FormList | WindowView::FormListClosing | WindowView::FormChosen => {
                    fields.form_list = Some(FormListFields {
                        offered: f("form_list.offered", Want::U8s)?,
                        count: f("form_list.count", Want::U8)?,
                        marked: f("form_list.marked", Want::Bools)?,
                        cursor: f("form_list.cursor", Want::U8)?,
                        chosen_set: f("form_list.chosen", Want::Bool)?,
                        chosen: f("form_list.chosen_place", Want::U8)?,
                    });
                }
                WindowView::OfferFlight => {
                    fields.offer_flight =
                        Some(FlightFields { step: f("offer_flight.step", Want::U8)?, count: f("offer_flight.count", Want::U8)?, button: None });
                }
                WindowView::ChipFlight => {
                    fields.chip_flight = Some(FlightFields {
                        step: f("chip_flight.step", Want::U8)?,
                        count: f("chip_flight.count", Want::U8)?,
                        button: Some(f("chip_flight.button", Want::U8)?),
                    });
                }
            }
        }
        for (name, view) in buttons {
            let who = format!("button `{name}`");
            match view {
                ButtonView::FormOffer => {
                    fields.offer = Some(OfferFields {
                        form: find(&who, view.name(), "form_offer.form", Want::Form)?,
                        alternate: find(&who, view.name(), "form_offer.alternate", Want::Bool)?,
                        turns: find(&who, view.name(), "form_offer.turns", Want::U8)?,
                    });
                }
                // (It shows a picture, nothing the rules keep.)
                ButtonView::ChipPicture => {}
            }
        }
        Ok(fields)
    }
}
