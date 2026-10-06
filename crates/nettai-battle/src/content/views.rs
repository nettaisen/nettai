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
    /// The version of the game they play, one of the names its field lists
    /// (EXE6's "gregar" or "falzar": the version's own pictures).
    Version,
    /// Their save has Beast Out (EXE6's event flag 0xE0: the emotion window
    /// shows its count).
    BeastOut,
    /// The forms they have for their form list, in its order (EXE6's
    /// Crosses, up to five of either version): an empty list is none.
    CrossList,
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
    /// How long their save took to delete each SP navi, by its SP navi
    /// chip, in frames: a list of `{ chip, frames }` (EXE6's and EXE5's;
    /// their rules' sp_chips read it for the chips' damage). A tool shows
    /// and edits them as times, and a boundary writes a save's.
    SpTimes,
}

impl PlayerFact {
    pub const ALL: &'static [PlayerFact] =
        &[PlayerFact::Version, PlayerFact::BeastOut, PlayerFact::CrossList, PlayerFact::Level, PlayerFact::BaseHp, PlayerFact::SpTimes];

    /// The setup field's name.
    pub fn name(self) -> &'static str {
        match self {
            PlayerFact::Version => "version",
            PlayerFact::BeastOut => "beast_out",
            PlayerFact::CrossList => "crosses",
            PlayerFact::Level => "level",
            PlayerFact::BaseHp => "hp",
            PlayerFact::SpTimes => "sp_times",
        }
    }

    /// Whether a setup field of type `ty` holds the fact, and what it must
    /// be if not.
    pub(crate) fn fits(self, ty: &FieldType) -> Result<(), &'static str> {
        let ok = match self {
            PlayerFact::Version => matches!(ty, FieldType::Enum(_)),
            PlayerFact::BeastOut => matches!(ty, FieldType::Bool),
            PlayerFact::CrossList => matches!(ty, FieldType::Array(e, _) if matches!(**e, FieldType::Ref(Registry::Form, _))),
            PlayerFact::Level => matches!(ty, FieldType::OptionalU8),
            PlayerFact::BaseHp => matches!(ty, FieldType::U16),
            PlayerFact::SpTimes => match ty {
                FieldType::List(elem, _) => match &**elem {
                    FieldType::Record(fields) => {
                        let names: Vec<(&str, &FieldType)> = fields.fields().iter().map(|f| (f.name.as_str(), &f.ty)).collect();
                        matches!(names[..], [("chip", FieldType::Ref(Registry::Chip, _)), ("frames", FieldType::U16)])
                    }
                    _ => false,
                },
                _ => false,
            },
        };
        if ok {
            return Ok(());
        }
        Err(match self {
            PlayerFact::Version => "a list of the versions' names",
            PlayerFact::BeastOut => "a bool",
            PlayerFact::CrossList => "an array of forms",
            PlayerFact::Level => "a u8? (a level, or none)",
            PlayerFact::BaseHp => "a u16",
            PlayerFact::SpTimes => "a list of { chip = \"chip\", frames = \"u16\" }",
        })
    }
}

/// A form list's fields in the rules' state (the views `form_list_opening`,
/// `form_list`, `form_list_closing` and `form_chosen`): the places of the
/// forms offered among the player's (`offered`, a u8 array) and how many
/// (`offered_count`), which entries are marked (`marked`, a bool array), the
/// entry under the cursor (`window_cursor`), and whether one is chosen
/// (`cross_chosen`) and its place (`chosen`).
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
/// `form_offer`): the form on offer, none for no offer (`offer`, a form),
/// whether the offer is the form's alternate (`offer_chaos`), and the turns
/// left in the form it gave (`turns`), which the emotion window counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct OfferFields {
    pub form: FieldPath,
    pub alternate: FieldPath,
    pub turns: FieldPath,
}

/// A flight's fields in the rules' state: its step and its count in the
/// step. The offer's (the window view `offer_flight`: `unite_step`,
/// `unite_count`), or a chip's (`chip_flight`: `mix_step`, `mix_count`, and
/// `button`, which of the screen's buttons that show a chip, counted from
/// 1: `mix_capsule`).
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
    /// state (`state`). `Err`: a view's field the state lacks or keeps as
    /// another type, said with the view's name and the window or button
    /// that has it (`what`).
    pub(crate) fn of<'a>(
        state: &Schema,
        windows: impl Iterator<Item = (&'a str, WindowView)>,
        buttons: impl Iterator<Item = (&'a str, ButtonView)>,
    ) -> Result<ViewFields, String> {
        let mut fields = ViewFields::default();
        // (A field by its name, the state's own or one of its records': the
        // one of the name there is.)
        let find = |who: &str, view: &str, name: &str, want: Want| -> Result<FieldPath, String> {
            let found = state.find(name).map_err(|e| format!("{who} has the view `{view}`, which shows the state field `{name}`: {e}"))?;
            let Some(path) = found else {
                return Err(format!("{who} has the view `{view}`, which shows the state field `{name}` ({}): the rules' state has none", want.says()));
            };
            let ty = state.place_of(path).ty();
            if !want.fits(ty) {
                return Err(format!("{who} has the view `{view}`, which shows the state field `{name}` as {}: it is {ty:?}", want.says()));
            }
            Ok(path)
        };
        for (name, view) in windows {
            let who = format!("window `{name}`");
            let f = |field: &str, want: Want| find(&who, view.name(), field, want);
            match view {
                WindowView::FormListOpening | WindowView::FormList | WindowView::FormListClosing | WindowView::FormChosen => {
                    fields.form_list = Some(FormListFields {
                        offered: f("offered", Want::U8s)?,
                        count: f("offered_count", Want::U8)?,
                        marked: f("marked", Want::Bools)?,
                        cursor: f("window_cursor", Want::U8)?,
                        chosen_set: f("cross_chosen", Want::Bool)?,
                        chosen: f("chosen", Want::U8)?,
                    });
                }
                WindowView::OfferFlight => {
                    fields.offer_flight = Some(FlightFields { step: f("unite_step", Want::U8)?, count: f("unite_count", Want::U8)?, button: None });
                }
                WindowView::ChipFlight => {
                    fields.chip_flight = Some(FlightFields {
                        step: f("mix_step", Want::U8)?,
                        count: f("mix_count", Want::U8)?,
                        button: Some(f("mix_capsule", Want::U8)?),
                    });
                }
            }
        }
        for (name, view) in buttons {
            let who = format!("button `{name}`");
            match view {
                ButtonView::FormOffer => {
                    fields.offer = Some(OfferFields {
                        form: find(&who, view.name(), "offer", Want::Form)?,
                        alternate: find(&who, view.name(), "offer_chaos", Want::Bool)?,
                        turns: find(&who, view.name(), "turns", Want::U8)?,
                    });
                }
                // (It shows a picture, nothing the rules keep.)
                ButtonView::ChipPicture => {}
            }
        }
        Ok(fields)
    }
}
