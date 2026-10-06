//! What a frontend draws of a system's own on the custom screen, by what
//! the engine names it (docs/design/rules-in-luau.md §4.8). A frontend has
//! drawing of its own for some of what content declares: a window with a
//! list of forms, an icon flying to the picked column, a button that offers
//! a form. Content says which of them a window or a button is, in its
//! definition (`view`), by one of these names; the names are checked as the
//! content loads, and the frontend reads the values, never a system's, a
//! window's or a button's own name.
//!
//! A view shows what its system keeps: state fields the frontend reads by
//! the names given here (a view's contract with its system). They are found
//! once, as the content loads ([`ViewFields`]), and a system that has the
//! view without a field, or with one of another type, is refused there.
//! Nothing here is state of its own.

use nettai_content_api::{FieldType, Registry, Schema};

/// What a frontend draws while a system's custom-screen window is up (the
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

/// What a frontend draws of a system's custom-screen button besides its
/// look (the button's `view`; a button with none is its pack's look alone).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ButtonView {
    /// It offers a form (EXE5's soul button): picked, the picked column
    /// shows the offered form's icon, and its picture in the chip window is
    /// in its second palette for the form's alternate offer (a Chaos
    /// Unison).
    FormOffer,
    /// Its picture in the chip window is a chip's too: the chip the
    /// ruleset's role `beast_out` names shows it in place of art of its own
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
/// fills in for a person: a field of a system's setup, by this name
/// (`Battle::fact`). It is read of the first
/// system of the game's ruleset that declares the field (a setup writes it
/// into every one that does); a game none of whose systems does has none.
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
    /// a game's save system writes it into the stats as the round is set
    /// up. A tool fills it in from what a navi's level gives (EXE5's
    /// `story`).
    BaseHp,
}

impl PlayerFact {
    pub const ALL: &'static [PlayerFact] =
        &[PlayerFact::Version, PlayerFact::BeastOut, PlayerFact::CrossList, PlayerFact::Level, PlayerFact::BaseHp];

    /// The setup field's name.
    pub fn name(self) -> &'static str {
        match self {
            PlayerFact::Version => "version",
            PlayerFact::BeastOut => "beast_out",
            PlayerFact::CrossList => "crosses",
            PlayerFact::Level => "level",
            PlayerFact::BaseHp => "hp",
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
        })
    }
}

/// A form list's fields in its system's state (the views `form_list_opening`,
/// `form_list`, `form_list_closing` and `form_chosen`): the places of the
/// forms offered among the player's (`offered`, a u8 array) and how many
/// (`offered_count`), which entries are marked (`marked`, a bool array), the
/// entry under the cursor (`window_cursor`), and whether one is chosen
/// (`cross_chosen`) and its place (`chosen`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FormListFields {
    pub offered: usize,
    pub count: usize,
    pub marked: usize,
    pub cursor: usize,
    pub chosen_set: usize,
    pub chosen: usize,
}

/// A form offer's fields in its system's state (the button view
/// `form_offer`): the form on offer, none for no offer (`offer`, a form),
/// whether the offer is the form's alternate (`offer_chaos`), and the turns
/// left in the form it gave (`turns`), which the emotion window counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct OfferFields {
    pub form: usize,
    pub alternate: usize,
    pub turns: usize,
}

/// A flight's fields in its system's state: its step and its count in the
/// step. The offer's (the window view `offer_flight`: `unite_step`,
/// `unite_count`), or a chip's (`chip_flight`: `mix_step`, `mix_count`, and
/// `button`, which of the screen's buttons that show a chip, counted from
/// 1: `mix_capsule`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FlightFields {
    pub step: usize,
    pub count: usize,
    pub button: Option<usize>,
}

/// The state fields a frontend reads of a system for its windows' and
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
    /// The fields the views of a system's windows and buttons read of its
    /// state (`state`). `Err`: a view's field the state lacks or keeps as
    /// another type, said with the view's name and the window or button
    /// that has it (`what`).
    pub(crate) fn of<'a>(
        state: &Schema,
        windows: impl Iterator<Item = (&'a str, WindowView)>,
        buttons: impl Iterator<Item = (&'a str, ButtonView)>,
    ) -> Result<ViewFields, String> {
        let mut fields = ViewFields::default();
        let find = |who: &str, view: &str, name: &str, want: Want| -> Result<usize, String> {
            let Some(i) = state.index_of(name) else {
                return Err(format!("{who} has the view `{view}`, which shows the state field `{name}` ({}): the system's state has none", want.says()));
            };
            if !want.fits(&state.field(i).ty) {
                return Err(format!("{who} has the view `{view}`, which shows the state field `{name}` as {}: it is {:?}", want.says(), state.field(i).ty));
            }
            Ok(i)
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
                // (It shows a picture, nothing its system keeps.)
                ButtonView::ChipPicture => {}
            }
        }
        Ok(fields)
    }
}
