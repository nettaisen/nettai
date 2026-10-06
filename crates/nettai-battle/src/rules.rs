//! The players' rules (docs/design/rules-in-luau.md): a game's rules are one
//! definition written in Luau (`define.rules { ... }`), whose hooks call the
//! game's modules as their code says; the framework calls the
//! rules' hooks at its points, and keeps their state of each side here, in
//! the battle, where snapshots and the digest cover it.
//!
//! The rules reach their own state of the side they were called for
//! (`rules.state()` in a hook): a side's rules see the other side through
//! the engine alone (their API module reads another side's,
//! `rules.state_of`).

use nettai_content_api::{Block, ChipHandle, FieldType, FieldValue, FormHandle, HookCall, ObjectRef, RulesHook, Value, WeaponHandle};

use crate::battle::Battle;
use crate::content::{Content, PlayerFact, ViewFields};
use crate::custom::PlayerSetup;
use crate::custom::screen::CROSSES;

/// A side's rules in a battle: the game's rules' state of the side (none:
/// the content has no rules).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct SideRules {
    pub state: Option<Block>,
}

impl SideRules {
    /// A player's rules at a round's start: the game's rules' state, zeroed
    /// (a game has one definition of its rules, which every match of it
    /// plays by). The player's setup of them is made the rules' defaults
    /// (`setup_defaults`, the rest zero) for a setup that gives none, and
    /// must otherwise be of the rules' layout. An enum of the setup has no
    /// default but its `setup_defaults`': the round doesn't start with one
    /// the player's setup leaves unstated (EXE6's version: nothing fills in
    /// gregar or falzar). A list left out is its default, or empty.
    pub fn for_player(content: &Content, player: &mut PlayerSetup) -> SideRules {
        let Some(rules) = content.defs.rules() else {
            assert!(player.rules.is_none(), "a player's setup gives a setup of the rules, and the content has none");
            return SideRules::default();
        };
        let setup = player.rules.get_or_insert_with(|| rules.setup_block());
        assert!(setup.id() == rules.setup, "a player's setup gives a setup that isn't of the game's rules");
        let schema = content.defs.schema(rules.setup);
        for (i, field) in schema.fields().iter().enumerate() {
            if setup.stated(schema, i) {
                continue;
            }
            let FieldType::Enum(names) = &field.ty else { continue };
            let what = names.join(" or ");
            panic!("a player's setup doesn't state the rules' `{}` ({what}): none is assumed", field.name);
        }
        SideRules { state: Some(Block::new(rules.state, content.defs.schema(rules.state))) }
    }
}

impl PlayerSetup {
    /// The player's setup of the game's rules and its layout, as the round
    /// will start with it (the rules' defaults where the setup gives none):
    /// what a tool shows of it. None: the content has no rules.
    pub fn rules_block<'a>(&self, content: &'a Content) -> Option<(&'a nettai_content_api::Schema, Block)> {
        let rules = content.defs.rules()?;
        let block = self.rules.clone().unwrap_or_else(|| rules.setup_block());
        Some((content.defs.schema(rules.setup), block))
    }

    /// Write a fact of what the player brings into the rules' setup field
    /// `field`: one value for a field, an element each for an array (the
    /// rest zero), an enum's by its name (`Fact::Name`). Whether the rules
    /// take it: none on a content without rules. ([`set_fact`], on a setup
    /// of the rules.)
    pub fn set_fact(&mut self, content: &Content, field: &str, values: &[Fact]) -> Result<bool, String> {
        set_fact(&mut self.rules, content, field, values)
    }
}

/// The setup of a player who says nothing: the game's rules' defaults
/// (`RulesDef::setup_block`); none on a content without rules.
pub fn default_setup(content: &Content) -> Option<Block> {
    content.defs.rules().map(|r| r.setup_block())
}

/// [`PlayerSetup::set_fact`] on a setup of the rules (`setup`: a player's;
/// none yet: the defaults first): what a tool that holds a side's setup
/// writes a fact with. Whether the rules take the fact: false where their
/// setup has no field of the name, or the content has no rules.
pub fn set_fact(setup: &mut Option<Block>, content: &Content, field: &str, values: &[Fact]) -> Result<bool, String> {
    let Some(rules) = content.defs.rules() else { return Ok(false) };
    let block = setup.get_or_insert_with(|| rules.setup_block());
    let schema = &content.defs.schemas[block.id().0 as usize].schema;
    let Some(i) = schema.index_of(field) else { return Ok(false) };
    let ty = &schema.field(i).ty;
    let value = |f: &Fact| -> Result<Value, String> {
        match (f, ty) {
            (Fact::Value(v), _) => Ok(*v),
            (Fact::Name(n), FieldType::Enum(names)) => {
                names.iter().position(|x| x == n).map(|i| Value::Int(i as i64)).ok_or_else(|| format!("setup field `{field}` has no variant {n:?}"))
            }
            (Fact::Name(n), _) => Err(format!("setup field `{field}` isn't an enum, for {n:?}")),
        }
    };
    if let FieldType::Array(elem, n) = ty {
        if values.len() > *n as usize {
            return Err(format!("setup field `{field}` holds {n}, not {}", values.len()));
        }
        // (Past the values given, zero: false, 0, none.)
        let zero = match **elem {
            FieldType::Bool => Value::Bool(false),
            FieldType::Ref(..) | FieldType::Asset(_) | FieldType::Object => Value::Nil,
            _ => Value::Int(0),
        };
        for k in 0..*n as usize {
            let v = values.get(k).map(value).transpose()?.unwrap_or(zero);
            block.set_elem(schema, i, k, v).map_err(|e| format!("setup field `{field}`: {e}"))?;
        }
    } else {
        let [f] = values else { return Err(format!("setup field `{field}` takes one value, not {}", values.len())) };
        block.set(schema, i, value(f)?).map_err(|e| format!("setup field `{field}`: {e}"))?;
    }
    Ok(true)
}

/// A fact read from a setup of the rules (`setup`: as [`set_fact`] takes
/// it) by its field's name. None: the rules' setup has no such field, or
/// the block isn't of their layout.
pub fn fact_in<'a>(setup: &'a Block, content: &'a Content, field: &str) -> Option<SetupFact<'a>> {
    let rules = content.defs.rules()?;
    (setup.id() == rules.setup).then_some(())?;
    let schema = content.defs.schema(rules.setup);
    Some(SetupFact { schema, block: setup, index: schema.index_of(field)? })
}

/// A value [`PlayerSetup::set_fact`] writes: a field's value, or an enum
/// variant by its name.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fact<'a> {
    Value(Value),
    Name(&'a str),
}

/// A fact of what a player brought, read back ([`Battle::fact`]): a field
/// of the rules' setup.
#[derive(Clone, Copy)]
pub struct SetupFact<'a> {
    schema: &'a nettai_content_api::Schema,
    block: &'a Block,
    index: usize,
}

impl<'a> SetupFact<'a> {
    /// Its value (an array's first element's).
    pub fn value(&self) -> nettai_content_api::FieldValue {
        self.block.get(self.schema, self.index)
    }

    /// The field's type.
    pub fn ty(&self) -> &'a FieldType {
        &self.schema.field(self.index).ty
    }

    /// Whether it holds a value: false of an enum or a list of definitions
    /// nothing stated (a round doesn't start with one).
    pub fn stated(&self) -> bool {
        self.block.stated(self.schema, self.index)
    }

    /// An enum's variant, by its name.
    pub fn name(&self) -> Option<&'a str> {
        match (self.value(), &self.schema.field(self.index).ty) {
            (nettai_content_api::FieldValue::Enum(i), FieldType::Enum(names)) => names.get(i as usize).map(String::as_str),
            _ => None,
        }
    }

    /// A flag's value.
    pub fn flag(&self) -> Option<bool> {
        match self.value() {
            nettai_content_api::FieldValue::Bool(b) => Some(b),
            _ => None,
        }
    }

    /// Element `k` of an array (none past its end, or for a field that is
    /// no array).
    pub fn elem(&self, k: usize) -> Option<nettai_content_api::FieldValue> {
        self.block.get_elem(self.schema, self.index, k)
    }

    /// Element `k` of an array of forms: the form, if it holds one.
    pub fn form(&self, k: usize) -> Option<FormHandle> {
        match self.elem(k)? {
            nettai_content_api::FieldValue::Ref(Some((nettai_content_api::Registry::Form, h))) => Some(FormHandle(h)),
            _ => None,
        }
    }
}

/// A form list as the rules keep it for their window (the window views
/// `form_list_opening`, `form_list`, `form_list_closing` and `form_chosen`,
/// [`Battle::form_list`]): the places of the forms offered among the
/// player's (which form a place holds is the game's rule), how many, which
/// entries are marked, the entry under the cursor, and the place chosen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FormList {
    pub offered: [u8; CROSSES],
    pub count: u8,
    pub marked: [bool; CROSSES],
    pub cursor: u8,
    pub chosen: Option<u8>,
}

/// What a button that offers a form has on offer (the button view
/// `form_offer`, [`Battle::offer`]): the form, none for no offer, and
/// whether the offer is the form's alternate (EXE5's Chaos Unison).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Offer {
    pub form: Option<FormHandle>,
    pub alternate: bool,
}

/// Where an icon's flight to the picked column is (the window views
/// `offer_flight` and `chip_flight`): its step and its count in the step.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Flight {
    pub step: u8,
    pub count: u8,
}

impl Battle {
    /// A fact of what side `side`'s player brought (what
    /// [`PlayerSetup::set_fact`] writes under the fact's name): the rules'
    /// setup field that keeps it, found as the content loaded
    /// (`Defs::fact_field`). For a reader of what a console shows of its
    /// player (their game's version, what their save unlocks); none when the
    /// rules take no such fact.
    pub fn fact(&self, side: u8, fact: PlayerFact) -> Option<SetupFact<'_>> {
        let defs = &self.content.defs;
        let index = defs.fact_field(fact)?;
        let schema = defs.schema(defs.rules()?.setup);
        Some(SetupFact { schema, block: self.setup.players[side as usize & 1].rules.as_ref()?, index })
    }

    /// The rules' state of side `side`, if their windows' and buttons' views
    /// show a view's fields (`pick`, of their `ViewFields`), with them and
    /// the state's layout.
    fn view_state<T>(&self, side: u8, pick: impl Fn(&ViewFields) -> Option<T>) -> Option<(T, &nettai_content_api::Schema, &Block)> {
        let rules = self.content.defs.rules()?;
        Some((pick(&rules.views)?, self.content.defs.schema(rules.state), self.rules[side as usize & 1].state.as_ref()?))
    }

    /// The form list the rules keep for side `side`'s form-list window
    /// ([`FormList`]); none: the rules have no such window.
    pub fn form_list(&self, side: u8) -> Option<FormList> {
        let (f, schema, state) = self.view_state(side, |v| v.form_list)?;
        let mut list = FormList {
            count: byte(state.get(schema, f.count)),
            cursor: byte(state.get(schema, f.cursor)),
            chosen: flag(state.get(schema, f.chosen_set)).then(|| byte(state.get(schema, f.chosen))),
            ..FormList::default()
        };
        for k in 0..CROSSES {
            list.offered[k] = state.get_elem(schema, f.offered, k).map_or(0, byte);
            list.marked[k] = state.get_elem(schema, f.marked, k).is_some_and(flag);
        }
        Some(list)
    }

    /// What side `side`'s button that offers a form has on offer
    /// ([`Offer`]); none: the rules have no such button.
    pub fn offer(&self, side: u8) -> Option<Offer> {
        let (f, schema, state) = self.view_state(side, |v| v.offer)?;
        let form = match state.get(schema, f.form) {
            FieldValue::Ref(Some((nettai_content_api::Registry::Form, h))) => Some(FormHandle(h)),
            _ => None,
        };
        Some(Offer { form, alternate: flag(state.get(schema, f.alternate)) })
    }

    /// The turns left in the form side `side`'s button that offers a form
    /// gave (the rules' `turns`), which the emotion window counts.
    pub fn form_turns(&self, side: u8) -> Option<u8> {
        let (f, schema, state) = self.view_state(side, |v| v.offer)?;
        Some(byte(state.get(schema, f.turns)))
    }

    /// Where the flight of the form on offer is (the window view
    /// `offer_flight`).
    pub fn offer_flight(&self, side: u8) -> Option<Flight> {
        let (f, schema, state) = self.view_state(side, |v| v.offer_flight)?;
        Some(Flight { step: byte(state.get(schema, f.step)), count: byte(state.get(schema, f.count)) })
    }

    /// Where the flight of a button's chip is (the window view
    /// `chip_flight`), and whose: which of the screen's buttons that show a
    /// chip, counted from 1.
    pub fn chip_flight(&self, side: u8) -> Option<(u8, Flight)> {
        let (f, schema, state) = self.view_state(side, |v| v.chip_flight)?;
        let flight = Flight { step: byte(state.get(schema, f.step)), count: byte(state.get(schema, f.count)) };
        Some((byte(state.get(schema, f.button?)), flight))
    }

    /// Side `side`'s player's setup of the rules (as the round started with
    /// it) and its layout: for a reader of what a player brought by its
    /// field's name (a game's tools; a frontend reads a fact by the
    /// engine's name for it, [`Battle::fact`]).
    pub fn rules_setup(&self, side: u8) -> Option<(&nettai_content_api::Schema, &Block)> {
        let rules = self.content.defs.rules()?;
        Some((self.content.defs.schema(rules.setup), self.setup.players[side as usize & 1].rules.as_ref()?))
    }

    /// The rules' state of side `side` and its layout: for a reader of what
    /// the rules keep by its field's name (a game's tools and tests; a
    /// frontend reads what a view shows, [`Battle::form_list`] and the
    /// like).
    pub fn rules_state(&self, side: u8) -> Option<(&nettai_content_api::Schema, &Block)> {
        let rules = self.content.defs.rules()?;
        Some((self.content.defs.schema(rules.state), self.rules[side as usize & 1].state.as_ref()?))
    }

    /// Side `side`'s rules.
    pub fn side_rules(&self, side: u8) -> &SideRules {
        &self.rules[side as usize]
    }

    /// Whether side `side` plays by rules (it has their state).
    pub(crate) fn has_rules(&self, side: u8) -> bool {
        self.rules.get(side as usize).is_some_and(|r| r.state.is_some())
    }

    /// The rules' `hook`, called for side `side` with the navi, the chip and
    /// the weapon it is about: its result, none when the rules have no such
    /// hook (or the side plays by none).
    fn call_rules(&mut self, side: u8, hook: RulesHook, navi: Option<ObjectRef>, chip: Option<ChipHandle>, weapon: Option<WeaponHandle>) -> Option<Value> {
        let f = self.content.defs.rules()?.hook(hook)?;
        if !self.has_rules(side) {
            return None;
        }
        let call = HookCall::Rules { side, hook, navi, chip, weapon };
        Some(crate::behavior::call_hook(self, f, call))
    }

    /// The rules' `hook` for side 0, then side 1 (the original's order
    /// wherever it loops over the sides).
    pub(crate) fn notify_rules(&mut self, hook: RulesHook) {
        for side in 0..2u8 {
            self.notify_side(side, hook);
        }
    }

    /// The rules' `hook(side)` for side `side`.
    pub(crate) fn notify_side(&mut self, side: u8, hook: RulesHook) {
        self.call_rules(side, hook, None, None, None);
    }

    /// What side `side`'s rules say of a folder (their `folder_check`:
    /// EXE6's folder rules), each rule it breaks named and said; nothing
    /// when it keeps them, or when the rules have none. The folder's chips
    /// in order, its Regular and tag chips (entries of `chips`); `complete`:
    /// all of a folder, else the chips so far (the rules about a whole
    /// folder wait). The rules read the side's stats as the round set them
    /// up (its folder limits). For tools (a match's checks, a random
    /// folder's draw): no part of the simulation.
    pub fn check_folder(
        &mut self,
        side: u8,
        chips: &[crate::custom::FolderChip],
        regular: Option<u8>,
        tags: Option<(u8, u8)>,
        complete: bool,
    ) -> Vec<FolderProblem> {
        self.folder_check = Some(FolderCheck {
            folder: nettai_content_api::api::CheckedFolder {
                side: side & 1,
                chips: chips.iter().map(|c| (c.id.0, c.code.0)).collect(),
                regular,
                tags,
                complete,
            },
            problems: Vec::new(),
        });
        self.notify_side(side & 1, RulesHook::FolderCheck);
        self.folder_check.take().map(|c| c.problems).unwrap_or_default()
    }

    /// The rules' `navi_intake(side, navi)`, each tick of the fight in the
    /// navi's intake. (Rules without the hook call nothing: EXE6's.)
    pub(crate) fn rules_navi_intake(&mut self, side: u8, navi: ObjectRef) {
        self.call_rules(side, RulesHook::NaviIntake, Some(navi), None, None);
    }

    /// The rules' `chip_prepared(side, navi, chip)` once a chip's use is
    /// prepared (`sub_80127C0`): `chip` the chip it uses (the zeroed chip
    /// for the empty hand).
    pub(crate) fn rules_chip_prepared(&mut self, side: u8, navi: ObjectRef, chip: ChipHandle) {
        self.call_rules(side, RulesHook::ChipPrepared, Some(navi), Some(chip), None);
    }

    /// The rules' `chip_used(side, navi, chip, weapon)` once a chip's use
    /// has started its action.
    pub(crate) fn rules_chip_used(&mut self, side: u8, navi: ObjectRef, chip: ChipHandle, weapon: Option<WeaponHandle>) {
        self.call_rules(side, RulesHook::ChipUsed, Some(navi), Some(chip), weapon);
    }

    /// The rules' `controller(side, navi)`: the outcome (the original's
    /// number: 0 nothing, 1 a chip, 2 the buster, 3 a step) they give; none
    /// is nothing.
    pub(crate) fn rules_controller(&mut self, side: u8, navi: ObjectRef) -> u8 {
        self.rules_controller_answer(side, navi).unwrap_or(0)
    }

    /// The rules' `controller(side, navi)`, asked of the side's own navi
    /// (EXE5's no-charge drive), or of a navi no player controls (the rules
    /// drive it): the outcome they give; None when they give none.
    pub(crate) fn rules_controller_answer(&mut self, side: u8, navi: ObjectRef) -> Option<u8> {
        match self.call_rules(side, RulesHook::Controller, Some(navi), None, None)? {
            Value::Int(n) => Some(n as u8),
            _ => None,
        }
    }

    /// The rules' `takeover_requested(side, navi)`.
    pub(crate) fn rules_takeover_requested(&mut self, side: u8, navi: ObjectRef) {
        self.call_rules(side, RulesHook::TakeoverRequested, Some(navi), None, None);
    }

    /// The rules' `takeover(side, navi)`: the outcome they give (as
    /// `rules_controller`'s, or 4: an attack of its own); none is nothing.
    pub(crate) fn rules_takeover(&mut self, side: u8, navi: ObjectRef) -> u8 {
        match self.call_rules(side, RulesHook::Takeover, Some(navi), None, None) {
            Some(Value::Int(n)) => n as u8,
            _ => 0,
        }
    }

    /// The rules' custom-screen buttons, in their order.
    pub(crate) fn side_buttons(&self, side: u8) -> Vec<crate::content::ButtonHandle> {
        if !self.has_rules(side) {
            return Vec::new();
        }
        self.content.defs.rules().map_or_else(Vec::new, |r| r.buttons.clone())
    }

    /// One of a button's functions (`shown`, `state`, `pressed`), as the
    /// rules' call for side `side`.
    pub(crate) fn call_button(&mut self, side: u8, button: crate::content::ButtonHandle, hook: RulesHook) -> Value {
        let content = self.content.clone();
        let d = content.defs.button(button);
        let f = match hook {
            RulesHook::ButtonShown => d.shown,
            RulesHook::ButtonState => d.state.expect("a button's state, asked only when it has one"),
            RulesHook::ButtonPressed => d.pressed,
            RulesHook::ButtonTakenBack => d.taken_back.expect("a button's taken_back, asked only when it has one"),
            RulesHook::ButtonChip => d.chip.expect("a button's chip, asked only when it has one"),
            h => panic!("{h:?} is no button's function"),
        };
        let call = HookCall::Rules { side, hook, navi: None, chip: None, weapon: None };
        crate::behavior::call_hook(self, f, call)
    }

    /// A window's `update`, as the rules' call for side `side`.
    pub(crate) fn call_window(&mut self, side: u8, window: crate::content::WindowHandle) -> Value {
        let content = self.content.clone();
        let d = content.defs.window(window);
        let call = HookCall::Rules { side, hook: RulesHook::WindowUpdate, navi: None, chip: None, weapon: None };
        crate::behavior::call_hook(self, d.update, call)
    }

    /// The rules' custom chip hook `hook(side, chip)`.
    pub(crate) fn rules_call_custom_chip(&mut self, side: u8, hook: RulesHook, chip: ChipHandle) {
        self.call_rules(side, hook, None, Some(chip), None);
    }

    /// The rules' custom hook `hook(side)`.
    pub(crate) fn rules_call_custom(&mut self, side: u8, hook: RulesHook) {
        self.call_rules(side, hook, None, None, None);
    }

    /// The rules' custom hook `hook(side)`: whether it answered true.
    pub(crate) fn rules_ask_custom(&mut self, side: u8, hook: RulesHook) -> bool {
        self.call_rules(side, hook, None, None, None) == Some(Value::Bool(true))
    }

    /// The rules' `custom.hand_size(side)`, if they answer.
    pub(crate) fn rules_custom_hand_size(&mut self, side: u8) -> Option<u8> {
        self.rules_ask(side, RulesHook::CustomHandSize, None)
    }

    /// The rules' `starting_mood(side)`, if they answer.
    pub(crate) fn rules_starting_mood(&mut self, side: u8) -> Option<u8> {
        self.rules_ask(side, RulesHook::StartingMood, None)
    }

    /// The rules' `navi_palette(side, navi)`, if they answer.
    pub(crate) fn rules_navi_palette(&mut self, side: u8, navi: ObjectRef) -> Option<u8> {
        self.rules_ask(side, RulesHook::NaviPalette, Some(navi))
    }

    /// The rules' `navi_bug(side, navi)`: whether they answered true (the
    /// bug and the weapons' reload skipped).
    pub(crate) fn rules_navi_bug(&mut self, side: u8, navi: ObjectRef) -> bool {
        self.call_rules(side, RulesHook::NaviBug, Some(navi), None, None) == Some(Value::Bool(true))
    }

    /// The rules' `hook`, if it answers a number.
    fn rules_ask(&mut self, side: u8, hook: RulesHook, navi: Option<ObjectRef>) -> Option<u8> {
        match self.call_rules(side, hook, navi, None, None)? {
            Value::Int(n) => Some(n as u8),
            _ => None,
        }
    }

    /// The rules' `countered(side, victim)`.
    pub(crate) fn rules_countered(&mut self, side: u8, victim: ObjectRef) {
        self.call_rules(side, RulesHook::Countered, Some(victim), None, None);
    }

    /// The rules' `navi_tick(side, navi)`.
    pub(crate) fn rules_navi_tick(&mut self, side: u8, navi: ObjectRef) {
        self.call_rules(side, RulesHook::NaviTick, Some(navi), None, None);
    }

    /// The rules' `form_reverted(side, navi)`.
    pub(crate) fn rules_form_reverted(&mut self, side: u8, navi: ObjectRef) {
        self.call_rules(side, RulesHook::FormReverted, Some(navi), None, None);
    }

    /// The rules' `chip_check(side, navi, chip)` as a chip's use is
    /// prepared: the chip they put in its place, or none (the use goes
    /// ahead).
    pub(crate) fn rules_chip_check(&mut self, side: u8, navi: ObjectRef, chip: Option<ChipHandle>) -> Option<ChipHandle> {
        self.rules_chip_answer(side, navi, chip, RulesHook::ChipCheck)
    }

    /// The rules' `chip_cost(side, navi, chip)`, earlier in the
    /// preparation: as `rules_chip_check`.
    pub(crate) fn rules_chip_cost(&mut self, side: u8, navi: ObjectRef, chip: Option<ChipHandle>) -> Option<ChipHandle> {
        self.rules_chip_answer(side, navi, chip, RulesHook::ChipCost)
    }

    /// The rules' `chip_substitute(side, navi, chip)` before a chip's record
    /// is loaded: the chip they put in its place (EXE6's dark chips'
    /// substitute), or none.
    pub(crate) fn rules_chip_substitute(&mut self, side: u8, navi: ObjectRef, chip: ChipHandle) -> Option<ChipHandle> {
        self.rules_chip_answer(side, navi, Some(chip), RulesHook::ChipSubstitute)
    }

    /// A chip hook `hook(side, navi, chip)`'s answer, a chip.
    fn rules_chip_answer(&mut self, side: u8, navi: ObjectRef, chip: Option<ChipHandle>, hook: RulesHook) -> Option<ChipHandle> {
        match self.call_rules(side, hook, Some(navi), chip, None)? {
            Value::Def(nettai_content_api::Registry::Chip, c) => Some(ChipHandle(c)),
            _ => None,
        }
    }
}

/// A folder being checked (`Battle::check_folder`) and what it breaks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FolderCheck {
    pub folder: nettai_content_api::api::CheckedFolder,
    pub problems: Vec<FolderProblem>,
}

/// A folder rule broken: the rule's name (the game's own: EXE6's `chip`,
/// `code`, `copies`, `mega`, `giga`, `dark`, `regular`, `tags`, `size`) and
/// what to say.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FolderProblem {
    pub rule: String,
    pub text: String,
}

/// A view's byte field (its type was checked as the content loaded).
fn byte(v: FieldValue) -> u8 {
    match v {
        FieldValue::U8(n) => n,
        _ => 0,
    }
}

/// A view's flag field.
fn flag(v: FieldValue) -> bool {
    matches!(v, FieldValue::Bool(true))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::testing;
    use crate::input::PlayerTick;
    use crate::scenario;
    use crate::setup::RoundSetup;
    use nettai_content_api::FieldValue;

    /// A battle from `setup`, run until its intro has spawned the navis.
    fn started(setup: RoundSetup) -> Battle {
        started_on(setup, scenario::content())
    }

    /// The same on `content`: the test content with its rules patched
    /// (`testing::with_rules`).
    fn started_on(mut setup: RoundSetup, content: std::sync::Arc<Content>) -> Battle {
        setup.content = content.hash();
        let mut b = Battle::new(setup, content);
        for _ in 0..3 {
            b.tick(&[PlayerTick::default(); 2], Default::default());
        }
        b
    }

    /// Field `field` of the rules' state of side `side`.
    fn field(b: &Battle, side: u8, field: &str) -> FieldValue {
        let (schema, s) = b.rules_state(side).expect("the side's rules");
        s.get(schema, schema.index_of(field).expect("a field"))
    }

    /// EXE6's bug frags are rules/dark_chips's (docs/design/
    /// rules-in-luau.md, As built S8): the player brings them in the rules'
    /// setup (a tool writes them as a fact), and the round starts with them
    /// in the rules' state, which the chips spend through EXE6's API.
    #[test]
    fn the_bug_frags_are_the_dark_chips_parts() {
        let content = scenario::content();
        let mut setup = scenario::setup();
        let took = setup.players[0]
            .set_fact(&content, "bug_frags", &[Fact::Value(nettai_content_api::Value::Int(7))])
            .expect("a count of bug frags");
        assert!(took, "the rules take them");
        let b = started(setup);
        assert_eq!((testing::bug_frags(&b, 0), testing::bug_frags(&b, 1)), (7, 0));
    }

    /// An enum of the rules' setup has no default: a player's setup that
    /// says nothing leaves EXE6's `version` (its Beast Out part's and its
    /// Crosses') unstated, and the round doesn't start (nothing fills in
    /// falzar, the enum's first name); stated by name, it starts and the
    /// rules read it. An enum of the rules' state starts at its first
    /// variant as ever.
    #[test]
    fn a_setups_enum_has_no_default() {
        let content = scenario::content();
        let mut setup = scenario::setup();
        for p in &mut setup.players {
            p.rules = None;
        }
        let (schema, block) = setup.players[0].rules_block(&content).expect("the test content's rules");
        let version = schema.index_of("version").expect("its version");
        assert!(!block.stated(schema, version) && block.stated(schema, schema.index_of("beast_out").unwrap()));
        let refused = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| Battle::new(setup.clone(), content.clone())));
        let why = refused.err().and_then(|e| e.downcast_ref::<String>().cloned()).expect("the round doesn't start");
        assert_eq!(why, "a player's setup doesn't state the rules' `version` (gregar or falzar): none is assumed");
        // One player's stated: the other's still stops it.
        setup.players[0].set_fact(&content, "version", &[Fact::Name("gregar")]).unwrap();
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| Battle::new(setup.clone(), content.clone()))).is_err());
        setup.players[1].set_fact(&content, "version", &[Fact::Name("falzar")]).unwrap();
        let b = Battle::new(setup, content);
        assert_eq!((b.fact(0, PlayerFact::Version).and_then(|f| f.name()), b.fact(1, PlayerFact::Version).and_then(|f| f.name())), (Some("gregar"), Some("falzar")));
    }

    /// The rules' setup defaults may give an array field a list: its
    /// elements from the first, the rest zero; a definition by its id. More
    /// values than the field holds, a value of another type and an id the
    /// content hasn't are content errors that name the field.
    #[test]
    fn a_setups_defaults_may_be_lists() {
        let with = |defaults: &str| -> Result<Content, String> {
            let mut c = testing::build();
            let module = c.scripts.module_mut(testing::ROOT, "rules/init").expect("the test content's rules");
            let stock = "        bonus = \"u8\",\n    },\n    setup_defaults = { beast_out = true, hp = 100, reg_up = fresh_stats.reg_up, sun = false },\n";
            assert!(module.contains(stock), "rules/init.luau's setup ends with `{stock}`");
            let fields = "        bonus = \"u8\",\n        marks = \"u8[3]\",\n        owned = \"bool[2]\",\n        wears = \"form[2]\",\n";
            *module = module.replace(stock, &format!("{fields}    }},\n    setup_defaults = {defaults},\n"));
            c.define().map_err(|e| e.message)?;
            Ok(c)
        };
        let c = with("{ bonus = 2, marks = { 4, 5 }, owned = { true }, wears = { \"base\" } }").unwrap_or_else(|e| panic!("{e}"));
        let rules = c.defs.rules().expect("the test content's rules");
        let (schema, block) = (c.defs.schema(rules.setup), rules.setup_block());
        let at = |field: &str, k: usize| block.get_elem(schema, schema.index_of(field).unwrap(), k).unwrap();
        assert_eq!(block.get(schema, schema.index_of("bonus").unwrap()), FieldValue::U8(2));
        assert_eq!([at("marks", 0), at("marks", 1), at("marks", 2)], [FieldValue::U8(4), FieldValue::U8(5), FieldValue::U8(0)]);
        assert_eq!([at("owned", 0), at("owned", 1)], [FieldValue::Bool(true), FieldValue::Bool(false)]);
        let base = c.defs.form_by_key("base").expect("the test navi's base form");
        assert_eq!([at("wears", 0), at("wears", 1)], [FieldValue::Ref(Some((nettai_content_api::Registry::Form, base.0))), FieldValue::Ref(None)]);
        // (A player's setup that gives none starts from it.)
        let mut player = crate::custom::PlayerSetup::default();
        player.set_fact(&c, "bonus", &[Fact::Value(Value::Int(9))]).unwrap();
        let (_, set) = player.rules_block(&c).unwrap();
        assert_eq!(set.get_elem(schema, schema.index_of("marks").unwrap(), 1), Some(FieldValue::U8(5)));
        let refused = |defaults: &str| with(defaults).err().unwrap_or_else(|| panic!("{defaults} is taken"));
        let e = refused("{ marks = { 1, 2, 3, 4 } }");
        assert!(e.ends_with("setup_defaults.marks: 4 values, and the field holds 3"), "{e}");
        let e = refused("{ owned = { 1 } }");
        assert!(e.contains("setup_defaults.owned: "), "{e}");
        let e = refused("{ marks = { 300 } }");
        assert!(e.contains("setup_defaults.marks: Int(300) doesn't fit the field's elements"), "{e}");
        let e = refused("{ wears = { \"nothing\" } }");
        assert!(e.ends_with("setup_defaults.wears: the content has no form \"nothing\""), "{e}");
        let e = refused("{ bonus = { 1 } }");
        assert!(e.contains("setup_defaults.bonus: "), "{e}");
    }

    /// Each side has the rules' state of its own: the test rules' counter
    /// ran once for each side as the round started, for that side.
    #[test]
    fn each_side_runs_the_rules_for_itself() {
        let b = started(scenario::setup());
        for side in 0..2u8 {
            assert!(b.side_rules(side).state.is_some());
            assert_eq!(field(&b, side, "starts"), FieldValue::U8(1), "round_start ran once for side {side}");
            assert_eq!(field(&b, side, "side"), FieldValue::U8(side), "it ran for its own side");
        }
    }

    /// A player's setup reaches the rules of their side alone; a field the
    /// rules' setup hasn't is no fact of theirs.
    #[test]
    fn a_players_setup_reaches_their_rules() {
        let content = scenario::content();
        let mut setup = scenario::setup();
        assert!(setup.players[0].set_fact(&content, "bonus", &[Fact::Value(Value::Int(7))]).unwrap());
        assert!(!setup.players[0].set_fact(&content, "mark", &[Fact::Value(Value::Int(1))]).unwrap(), "not the rules' setup's");
        let b = started(setup);
        assert_eq!(field(&b, 0, "bonus"), FieldValue::U16(14));
        assert_eq!(field(&b, 1, "bonus"), FieldValue::U16(0), "the other player's setup is its own");
    }

    /// The per-tick and chip-use hooks (docs/design/exe5-map.md §15.3 item
    /// 14): the rules' `navi_intake` is called with the side and its navi,
    /// and their `chip_check` with the chip about to be used, whose answer
    /// takes the chip's place; rules that lack them call nothing. (Rules
    /// that count the side's navi intakes, keep its panel, and refuse the
    /// test bomb, giving the test seed instead.)
    #[test]
    fn the_rules_intake_and_chip_check_hooks() {
        let content = testing::with_rules(&[
            ("        -- The counter's.\n        starts = \"u8\",", "        intakes = \"u16\",\n        x = \"u8\",\n        y = \"u8\",\n        -- The counter's.\n        starts = \"u8\","),
            ("local save = require(\"@self/save\")\n", "local save = require(\"@self/save\")\nlocal test_chips = require(\"./chips/test\")\n"),
            (
                "    hooks = {\n",
                "    hooks = {\n        navi_intake = function(_side: number, navi: Object)\n            local s = rules.state() :: { intakes: number, x: number, y: number }\n            s.intakes += 1\n            s.x, s.y = navi.panel_x, navi.panel_y\n        end,\n        chip_check = function(_side: number, _navi: Object, chip: Chip?): Chip?\n            return if chip == test_chips.bomb then test_chips.seed else nil\n        end,\n",
            ),
        ]);
        let mut b = started_on(scenario::setup_on(&content), content.clone());
        let navi = b.player(1).expect("side 1's navi");
        b.rules_navi_intake(1, navi);
        b.rules_navi_intake(1, navi);
        let p = b.objects.get(navi).panel;
        assert_eq!(field(&b, 1, "intakes"), FieldValue::U16(2));
        assert_eq!((field(&b, 1, "x"), field(&b, 1, "y")), (FieldValue::U8(p.x), FieldValue::U8(p.y)));
        let bomb = testing::chip_in(&content, "test/bomb");
        let seed = testing::chip_in(&content, "test/seed");
        assert_eq!(b.rules_chip_check(1, navi, Some(bomb)), Some(seed));
        assert_eq!(b.rules_chip_check(1, navi, Some(seed)), None);
        assert_eq!(b.rules_chip_check(1, navi, None), None);
        // The test content's own rules have neither hook.
        let mut stock = started(scenario::setup());
        let navi0 = stock.player(0).expect("side 0's navi");
        assert_eq!(stock.rules_chip_check(0, navi0, Some(bomb)), None);
    }

    #[test]
    fn the_rules_are_in_the_digest_and_the_snapshot() {
        let b = started(scenario::setup());
        let copy = b.clone();
        assert_eq!(copy.digest(), b.digest());
        let mut changed = b.clone();
        let s = changed.rules[1].state.as_mut().expect("side 1's rules");
        let schema = &b.content.defs.schemas[s.id().0 as usize].schema;
        s.set(schema, schema.index_of("starts").unwrap(), Value::Int(9)).unwrap();
        assert_ne!(changed.digest(), b.digest());
    }

    /// The rules' extension of their game's definitions
    /// (docs/design/rules-in-luau.md §7.5): kept on the definition, which a
    /// tool reads through `Defs::extension`, and checked as the content is
    /// defined: its types, its tables' fields, its variants.
    #[test]
    fn the_rules_extend_their_games_definitions() {
        use nettai_content_api::{Data, Registry};
        let content = scenario::content();
        let veil = "test/veil";
        assert_eq!(content.defs.extension(Registry::Chip, veil, "test_weight"), Some(&Data::Int(3)));
        let tag = content.defs.extension(Registry::Chip, veil, "test_tag").expect("veil's tag");
        assert_eq!((tag.field("level"), tag.field("kind")), (&Data::Int(2), &Data::Str("b".into())));
        assert_eq!(content.defs.extension(Registry::Chip, testing::BOMB, "test_weight"), None);
        let patched = |module: &str, from: &str, to: &str| {
            let mut c = testing::build();
            let src = c.scripts.module_mut(testing::ROOT, module).expect("the module");
            assert!(src.contains(from), "{from}");
            *src = src.replacen(from, to, 1);
            c.define().map(|_| ()).map_err(|e| e.message)
        };
        let refused = |module: &str, from: &str, to: &str, said: &str| {
            let e = patched(module, from, to).expect_err(said);
            assert!(e.contains(said), "{said}: {e}");
        };
        let chips = "chips/test/init";
        refused(chips, "test_weight = 3,", "test_weight = 300,", "chip test/veil.test_weight is Int(300), not u8");
        refused(chips, "kind = \"b\" }", "kind = \"c\" }", "chip test/veil.test_tag.kind");
        refused(chips, "kind = \"b\" }", "kind = \"b\", hue = 1 }", "`hue` is none of its fields (kind, level)");
        let rules = "rules/init";
        refused(rules, "            test_weight = \"u8\",", "            test_weight = \"u9\",", "no type is named \"u9\"");
        refused(rules, "    extends = {\n        chip = {", "    extends = {\n        stage = {},\n        chip = {", "the rules extend chip, form or navi");
    }

    /// A window's and a button's `view` (docs/design/rules-in-luau.md §4.8)
    /// is one of the engine's names, and what a view shows are fields of the
    /// rules' state, of the view's types; a fact a player brings is a setup
    /// field of the fact's type. Each is checked as the content is defined,
    /// and said with the rules' module.
    #[test]
    fn views_and_facts_are_checked_as_the_content_is_defined() {
        use crate::content::{ButtonView, WindowView};
        // (The rules' state and their windows, with what a test gives them.)
        let marker = "        -- The counter's.\n        starts = \"u8\",";
        let patched = |from: &str, to: &str| {
            let mut c = testing::build();
            let src = c.scripts.module_mut(testing::ROOT, "rules/init").expect("the module");
            assert!(src.contains(from), "{from}");
            *src = src.replacen(from, to, 1);
            c.define().map(|_| c).map_err(|e| e.message)
        };
        // A window `w` with view `view`, and `state` among the rules' state.
        let window_patched = |state: &str, view: &str| -> Result<Content, String> {
            let mut c = testing::build();
            let src = c.scripts.module_mut(testing::ROOT, "rules/init").expect("the module");
            *src = src.replacen(marker, &format!("        {state},\n{marker}"), 1);
            let windows = "    windows = { beast_out";
            assert!(src.contains(windows), "{windows}");
            *src = src.replacen(windows, &format!("    windows = {{ w = {{ view = \"{view}\", update = function(side: number): boolean return false end }}, beast_out"), 1);
            c.define().map(|_| c).map_err(|e| e.message)
        };
        let refused = |from: &str, to: &str, said: &[&str]| {
            let e = patched(from, to).err().unwrap_or_else(|| panic!("{said:?}: defined"));
            assert!(said.iter().all(|s| e.contains(s)), "{said:?}: {e}");
        };
        let window_refused = |state: &str, view: &str, said: &[&str]| {
            let e = window_patched(state, view).err().unwrap_or_else(|| panic!("{said:?}: defined"));
            assert!(said.iter().all(|s| e.contains(s)), "{said:?}: {e}");
        };
        let window_named = |c: &Content, name: &str| c.defs.rules().unwrap().windows.iter().copied().find(|&w| c.defs.window(w).name == name);
        let button_named = |c: &Content, name: &str| c.defs.rules().unwrap().buttons.iter().copied().find(|&b| c.defs.button(b).name == name);
        // A window's view: a name of the engine's, whose fields the rules
        // keep as the view's types.
        window_refused("mark = \"u8\"", "form_lst", &["rules/init.luau: rules", "window `w`: `view` is \"form_lst\"", "form_list_opening"]);
        window_refused(
            "mark = \"u8\"",
            "offer_flight",
            &["window `w` has the view `offer_flight`", "the state field `unite_step` (a u8): the rules' state has none"],
        );
        window_refused("unite_step = \"bool\", unite_count = \"u8\"", "offer_flight", &["the state field `unite_step` as a u8: it is Bool"]);
        let content = window_patched("unite_step = \"u8\", unite_count = \"u8\"", "offer_flight").expect("a view with its fields");
        let rules = content.defs.rules().expect("the rules");
        assert_eq!(content.defs.window(window_named(&content, "w").expect("the window")).view, Some(WindowView::OfferFlight));
        assert!(rules.views.offer_flight.is_some() && rules.views.form_list.is_none());
        // A button's.
        let buttons = "    buttons = { beast_out";
        let button = |view: &str| {
            format!(
                "    buttons = {{ b = {{ slot = 8, view = \"{view}\", shown = function(side: number): boolean return false end, pressed = function(side: number) end }}, beast_out"
            )
        };
        refused(buttons, &button("soul"), &["button `b`: `view` is \"soul\"", "form_offer, chip_picture"]);
        refused(buttons, &button("form_offer"), &["button `b` has the view `form_offer`", "the state field `offer` (a form)"]);
        let content = patched(buttons, &button("chip_picture")).expect("a view that shows no field");
        assert_eq!(content.defs.button(button_named(&content, "b").expect("the button")).view, Some(ButtonView::ChipPicture));
        // A fact: the setup field of its name, of its type.
        let setup = "        -- The counter's.\n        bonus = \"u8\",";
        refused(
            setup,
            "        crosses = \"u8\",\n        bonus = \"u8\",",
            &["rules/init.luau: rules", "their setup field `crosses` is the fact a player brings by that name, an array of forms"],
        );
        let content = patched(setup, "        crosses = \"form[5]\",\n        bonus = \"u8\",").expect("a fact of its type");
        assert!(content.defs.fact_field(PlayerFact::CrossList).is_some());
    }

    /// The rules say, for tools, the chips they can't play of a player's
    /// auto battle data, each with why (`unplayable_in_auto_battle`): the
    /// game's rules answer for a chip (`Defs::unplayable_in_auto_battle`),
    /// and an id that is no chip of the game is refused as the content is
    /// defined.
    #[test]
    fn the_rules_say_the_chips_auto_battle_cant_play() {
        let with = |entry: &str| {
            let mut c = testing::build();
            let src = c.scripts.module_mut(testing::ROOT, "rules/init").expect("the module");
            let from = "    hooks = {\n";
            assert!(src.contains(from), "{from}");
            *src = src.replacen(from, &format!("    unplayable_in_auto_battle = {entry},\n{from}"), 1);
            c.define().map(|_| c).map_err(|e| e.message)
        };
        let chip = |c: &Content, key: &str| c.defs.chip_by_key(key).unwrap_or_else(|| panic!("no chip {key}"));
        let content = with("{ [\"test/veil\"] = \"it has no weight\" }").expect("defined");
        assert_eq!(content.defs.unplayable_in_auto_battle(chip(&content, "test/veil")), Some("it has no weight"));
        assert_eq!(content.defs.unplayable_in_auto_battle(chip(&content, testing::BOMB)), None);
        let stock = scenario::content();
        assert_eq!(stock.defs.unplayable_in_auto_battle(chip(&stock, "test/veil")), None, "the stock rules say none");
        let e = with("{ [\"test/nothing\"] = \"it isn't\" }").map(|_| ()).expect_err("no such chip");
        assert!(e.contains("`unplayable_in_auto_battle` names test/nothing, which is no chip of the game"), "{e}");
        let e = with("{ \"test/veil\" }").map(|_| ()).expect_err("a list");
        assert!(e.contains("`unplayable_in_auto_battle` is a table of sentences by chip id"), "{e}");
    }

    mod patch_cards {
        use super::*;
        use crate::patch_cards::{InstalledCard, PatchCards};
        use crate::setup::{GaugeSpeed, NaviStats, Supports};

        /// A battle on the test content (whose rules apply EXE6's patch
        /// cards as the round is set up), side 0 with `cards` installed
        /// (key, switched on), its stats changed by `tweak` first.
        fn with_cards(cards: &[(&str, bool)], tweak: impl FnOnce(&mut NaviStats)) -> Battle {
            let content = scenario::content();
            let mut s = scenario::setup_on(&content);
            let p = &mut s.players[0];
            let list: Vec<InstalledCard> = cards
                .iter()
                .map(|&(key, enabled)| InstalledCard {
                    card: content.defs.patch_card_by_key(key).unwrap_or_else(|| panic!("no card {key:?}")),
                    enabled,
                })
                .collect();
            p.patch_cards = PatchCards::new(&list).unwrap();
            tweak(&mut s.navi_stats[0]);
            Battle::new(s, content)
        }

        /// Side `side`'s stats as the setup gives them, with the version
        /// byte the version fact writes (`testing::VERSION`).
        fn setup_stats(side: usize) -> NaviStats {
            let mut stats = scenario::setup().navi_stats[side];
            stats.version = 1;
            stats
        }

        #[test]
        fn the_cards_are_definitions_and_the_setups_part() {
            let content = scenario::content();
            let h = content.defs.patch_card_by_key("test-stats").expect("the test card");
            let card = content.patch_card(h);
            assert_eq!(card.mb, 20);
            let kinds: Vec<(&str, bool)> = card.effects.iter().map(|e| (e.kind.as_str(), e.bug)).collect();
            assert_eq!(kinds, [("hp_add", false), ("hp_percent_add", false), ("attack_add", false), ("body", false), ("hp_drain", true)]);
            // The cards are in the setup, which the digest covers.
            let a = with_cards(&[("test-stats", true)], |_| {});
            let b = with_cards(&[("test-stats", false)], |_| {});
            assert_ne!(a.setup.players[0].patch_cards, b.setup.players[0].patch_cards);
            assert_ne!(a.digest(), b.digest());
        }

        #[test]
        fn a_card_changes_the_stats_by_its_kinds_order() {
            let b = with_cards(&[("test-stats", true)], |_| {});
            let s = &b.stats[0];
            // HP 1000: +30 first, then +10% (the card lists them the other way).
            assert_eq!((s.max_hp, s.hp), (1133, 1133));
            assert_eq!((s.attack, s.element, s.bugs.hp_drain), (3, 2, 2));
            assert_eq!(b.reserves[0], b.stats[0], "the battle-start copy is of the stats after the cards");
            assert!(b.consoles[0].emotion_window_glitch, "the HP drain is a bug: flag 0x1723");
            assert_eq!(b.stats[1], setup_stats(1), "the other side has none");
        }

        #[test]
        fn a_later_card_writes_over_an_earlier_one() {
            let b = with_cards(&[("test-stats", true), ("test-later", true)], |_| {});
            let s = &b.stats[0];
            assert_eq!(s.attack, 2, "Attack 0 + 3 - 1");
            assert_eq!(s.giga_level, 0xFF, "GigaFolder- doesn't clamp");
        }

        #[test]
        fn abilities_choices_and_chip_shuffle() {
            let b = with_cards(&[("test-abilities", true)], |s| {
                s.support = Some(Supports::default());
                s.float_shoes = true;
                s.number_open = true;
            });
            let s = &b.stats[0];
            let content = &b.content;
            assert!(s.super_armor && !s.float_shoes);
            assert_eq!(s.first_barrier, content.defs.record("barrier/200"));
            assert_eq!(s.weapons.charge_shot_kind, content.defs.record("shot/charged-confusing"));
            assert_eq!(s.support, Some(Supports { rush: true, ..Supports::default() }));
            assert_eq!(s.gauge_speed, GaugeSpeed::Fast);
            assert!(s.chip_shuffle && !s.number_open, "ChpShufl turns NumbrOpn off");
            assert!(!b.consoles[0].emotion_window_glitch, "no bug");
        }

        #[test]
        fn a_switched_off_card_does_nothing_but_the_glitch_follows_the_stats() {
            let b = with_cards(&[("test-stats", false)], |s| s.support = Some(Supports::default()));
            let mut want = setup_stats(0);
            want.support = Some(Supports::default());
            // The HP is set to its maximum (the reload's, in the real world).
            want.hp = want.max_hp;
            assert_eq!(b.stats[0], want);
            assert!(!b.consoles[0].emotion_window_glitch);
            let bugged = with_cards(&[("test-stats", false)], |s| {
                s.support = Some(Supports::default());
                s.bugs.emotion = 1;
            });
            assert!(bugged.consoles[0].emotion_window_glitch, "a NaviCust bug counts with cards installed");
        }

        #[test]
        fn without_cards_the_stats_and_the_glitch_are_the_setups() {
            let b = with_cards(&[], |s| s.bugs.emotion = 1);
            let mut want = setup_stats(0);
            want.bugs.emotion = 1;
            assert_eq!(b.stats[0], want);
            assert!(!b.consoles[0].emotion_window_glitch, "the console's own flag (0x1720), not the cards'");
        }

        #[test]
        fn the_support_bug_keeps_supports_off() {
            let b = with_cards(&[("test-abilities", true)], |s| s.support = None);
            assert_eq!(b.stats[0].support, None, "the byte 0xFF stays 0xFF when a bit is set");
        }
    }
}
