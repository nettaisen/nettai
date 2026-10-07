//! The players' rules (docs/design/rules-in-luau.md): a game's rules are one
//! definition written in Luau (the game's root's `rules`), whose hooks call the
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

/// What a side's rules did with a hit's NaviCust bug (`navi_bug`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NaviBug {
    /// Nothing of the navi's stats changed (no bug, or one that leaves
    /// none: a panel trail, the uninstalls): its weapons are reloaded.
    Untouched,
    /// They edited its stats: its abilities and form flags come back, and
    /// its weapons are reloaded.
    Edited,
    /// They spared it the bug and the reload (EXE5's light MegaMan, a drain
    /// that wouldn't rise).
    Spared,
}

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
    /// rest zero) or a list (its length theirs), an enum's by its name
    /// (`Fact::Name`), a record's by its fields (`Fact::Record`). Whether the rules
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
    let place = schema.place(i);
    let at = format!("setup field `{field}`");
    match place.ty() {
        FieldType::Array(..) | FieldType::List(..) => write_fact(block, place, &Fact::List(values.to_vec()), &at)?,
        _ => {
            let [f] = values else { return Err(format!("{at} takes one value, not {}", values.len())) };
            write_fact(block, place, f, &at)?;
        }
    }
    Ok(true)
}

/// Write `f` at `place` of a setup of the rules: a value; an enum's
/// variant by its name; an array's elements from the first (the rest
/// zero), a list's (its length theirs); a record's fields by name (the
/// rest zero). `at` says where, in a message.
fn write_fact(block: &mut Block, place: nettai_content_api::Place, f: &Fact, at: &str) -> Result<(), String> {
    match (place.ty(), f) {
        (FieldType::Array(..) | FieldType::List(..), Fact::List(items)) => {
            let room = place.capacity().expect("an array or a list");
            if items.len() > room {
                return Err(format!("{at} holds {room}, not {}", items.len()));
            }
            block.clear_at(place);
            if let FieldType::List(..) = place.ty() {
                block.set_len_at(place, items.len())?;
            }
            for (k, item) in items.iter().enumerate() {
                write_fact(block, place.elem(k).expect("within its room"), item, &format!("{at}[{}]", k + 1))?;
            }
            Ok(())
        }
        (FieldType::Record(fields), Fact::Record(entries)) => {
            block.clear_at(place);
            for (name, x) in entries.iter() {
                let Some(p) = place.field(name) else {
                    let names: Vec<&str> = fields.fields().iter().map(|f| f.name.as_str()).collect();
                    return Err(format!("{at} has no field `{name}` ({})", names.join(", ")));
                };
                write_fact(block, p, x, &format!("{at}.{name}"))?;
            }
            Ok(())
        }
        (FieldType::Enum(names), Fact::Name(n)) => {
            let i = names.iter().position(|x| x == n).ok_or_else(|| format!("{at} has no variant {n:?}"))?;
            block.set_at(place, Value::Int(i as i64)).map_err(|e| format!("{at}: {e}"))
        }
        (_, Fact::Name(n)) => Err(format!("{at} isn't an enum, for {n:?}")),
        (ty, Fact::Value(v)) if ty.is_scalar() => block.set_at(place, *v).map_err(|e| format!("{at}: {e}")),
        (ty, f) => Err(format!("{at} is a {ty}, not {}", match f {
            Fact::List(_) => "a list",
            Fact::Record(_) => "a record",
            _ => "one value",
        })),
    }
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

/// A value [`PlayerSetup::set_fact`] writes: a field's value, an enum
/// variant by its name, a list's or an array's elements, a record's fields
/// by name.
#[derive(Clone, Debug, PartialEq)]
pub enum Fact<'a> {
    Value(Value),
    Name(&'a str),
    List(Vec<Fact<'a>>),
    Record(Vec<(&'a str, Fact<'a>)>),
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

    /// Element `k` of an array or a list (none past its end, or for a
    /// field that is neither).
    pub fn elem(&self, k: usize) -> Option<nettai_content_api::FieldValue> {
        let place = self.place();
        (k < self.block.len_at(place)?).then(|| place.elem(k).map(|p| self.block.get_at(p))).flatten()
    }

    /// Where it is in the setup's block: to read a record's or a list's
    /// parts (`place.elem(k)`, `place.field(name)`, [`SetupFact::block`]).
    pub fn place(&self) -> nettai_content_api::Place<'a> {
        self.schema.place(self.index)
    }

    /// The setup's block it is read of.
    pub fn block(&self) -> &'a Block {
        self.block
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
/// player's form list (the setup's field of the role `form_list`), the
/// forms there, how many, which entries are marked, the entry under the
/// cursor, and the place chosen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FormList {
    pub offered: [u8; CROSSES],
    /// The forms offered: the player's form list's at the places offered
    /// (none: no form there).
    pub forms: [Option<FormHandle>; CROSSES],
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

    /// Side `side`'s navi's stat of its game's own that has `role` (the
    /// rules' `stats`), if the game has one.
    pub fn stat(&self, side: u8, role: crate::content::StatRole) -> Option<nettai_content_api::FieldValue> {
        let defs = &self.content.defs;
        let index = defs.stat_field(role)?;
        let rules = defs.rules()?;
        let game = &self.stats[side as usize & 1].game;
        (game.id() == rules.stats).then(|| game.get(defs.schema(rules.stats), index))
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
        let get = |p| state.get_at(schema.place_of(p));
        let mut list = FormList {
            count: byte(get(f.count)),
            cursor: byte(get(f.cursor)),
            chosen: flag(get(f.chosen_set)).then(|| byte(get(f.chosen))),
            ..FormList::default()
        };
        let (offered, marked) = (schema.place_of(f.offered), schema.place_of(f.marked));
        let player = self.fact(side, PlayerFact::FormList);
        for k in 0..CROSSES {
            list.offered[k] = offered.elem(k).map_or(0, |p| byte(state.get_at(p)));
            list.forms[k] = player.as_ref().and_then(|f| f.form(list.offered[k] as usize));
            list.marked[k] = marked.elem(k).is_some_and(|p| flag(state.get_at(p)));
        }
        Some(list)
    }

    /// What side `side`'s button that offers a form has on offer
    /// ([`Offer`]); none: the rules have no such button.
    pub fn offer(&self, side: u8) -> Option<Offer> {
        let (f, schema, state) = self.view_state(side, |v| v.offer)?;
        let form = match state.get_at(schema.place_of(f.form)) {
            FieldValue::Ref(Some((nettai_content_api::Registry::Form, h))) => Some(FormHandle(h)),
            _ => None,
        };
        Some(Offer { form, alternate: flag(state.get_at(schema.place_of(f.alternate))) })
    }

    /// The turns left in the form side `side`'s button that offers a form
    /// gave (the rules' `turns`), which the emotion window counts.
    pub fn form_turns(&self, side: u8) -> Option<u8> {
        let (f, schema, state) = self.view_state(side, |v| v.offer)?;
        Some(byte(state.get_at(schema.place_of(f.turns))))
    }

    /// Where the flight of the form on offer is (the window view
    /// `offer_flight`).
    pub fn offer_flight(&self, side: u8) -> Option<Flight> {
        let (f, schema, state) = self.view_state(side, |v| v.offer_flight)?;
        let get = |p| byte(state.get_at(schema.place_of(p)));
        Some(Flight { step: get(f.step), count: get(f.count) })
    }

    /// Where the flight of a button's chip is (the window view
    /// `chip_flight`), and whose: which of the screen's buttons that show a
    /// chip, counted from 1.
    pub fn chip_flight(&self, side: u8) -> Option<(u8, Flight)> {
        let (f, schema, state) = self.view_state(side, |v| v.chip_flight)?;
        let get = |p| byte(state.get_at(schema.place_of(p)));
        let flight = Flight { step: get(f.step), count: get(f.count) };
        Some((get(f.button?), flight))
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

    /// What side `side`'s rules say is wrong with its setup (their
    /// `validate`): each problem, in their order; nothing when they find
    /// none, or have no `validate`. The rules read the setup and the stats
    /// as the round set them up. For tools (a match's checks, netplay's
    /// offer, the build creator): no part of the simulation.
    pub fn validate(&mut self, side: u8) -> Vec<Problem> {
        self.validation = Some(Vec::new());
        self.notify_side(side & 1, RulesHook::Validate);
        self.validation.take().unwrap_or_default()
    }

    /// For tools: what module `module` of the content (by its name,
    /// `exe6:rules/navicust/board`) returned as it loaded, as plain data, a
    /// function in it left out; nothing is called (the build creator reads the
    /// NaviCust's boards so). None: no module of that name.
    pub fn module_data(&self, module: &str) -> Option<Result<nettai_content_api::Data, String>> {
        crate::behavior::module_data(self, module)
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

    /// The rules' `navi_bug(side, navi)`: what they did with the navi's
    /// hit's NaviCust bug ([`NaviBug`]; no answer: untouched).
    pub(crate) fn rules_navi_bug(&mut self, side: u8, navi: ObjectRef) -> NaviBug {
        match self.call_rules(side, RulesHook::NaviBug, Some(navi), None, None) {
            Some(Value::Int(1)) => NaviBug::Edited,
            Some(Value::Int(2)) => NaviBug::Spared,
            _ => NaviBug::Untouched,
        }
    }

    /// The rules' `charge_released(side, navi)`.
    pub(crate) fn rules_charge_released(&mut self, side: u8, navi: ObjectRef) {
        self.call_rules(side, RulesHook::ChargeReleased, Some(navi), None, None);
    }

    /// The rules' `release_taken(side, navi)`.
    pub(crate) fn rules_release_taken(&mut self, side: u8, navi: ObjectRef) {
        self.call_rules(side, RulesHook::ReleaseTaken, Some(navi), None, None);
    }

    /// The rules' `hit_bug(side, navi)`.
    pub(crate) fn rules_hit_bug(&mut self, side: u8, navi: ObjectRef) {
        self.call_rules(side, RulesHook::HitBug, Some(navi), None, None);
    }

    /// The rules' `bug_mark(side, navi)`: whether they answered true (the
    /// marker shows).
    pub(crate) fn rules_bug_mark(&mut self, side: u8, navi: ObjectRef) -> bool {
        self.call_rules(side, RulesHook::BugMark, Some(navi), None, None) == Some(Value::Bool(true))
    }

    /// The rules' `navi_flinched(side, navi)`.
    pub(crate) fn rules_navi_flinched(&mut self, side: u8, navi: ObjectRef) {
        self.call_rules(side, RulesHook::NaviFlinched, Some(navi), None, None);
    }

    /// The rules' `hp_emptied(side, navi)`: whether they answered true (the
    /// hit shows).
    pub(crate) fn rules_hp_emptied(&mut self, side: u8, navi: ObjectRef) -> bool {
        self.call_rules(side, RulesHook::HpEmptied, Some(navi), None, None) == Some(Value::Bool(true))
    }

    /// The rules' `obstacle_reaction(side, obstacle)`.
    pub(crate) fn rules_obstacle_reaction(&mut self, side: u8, obstacle: ObjectRef) {
        self.call_rules(side, RulesHook::ObstacleReaction, Some(obstacle), None, None);
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

/// A problem of a side's setup, as its rules' `validate` says it: what to
/// say, and the setup field it is of and the entry of a list field (from
/// 0), where the rules say (a tool shows it there).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    pub text: String,
    pub field: Option<String>,
    pub entry: Option<usize>,
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
        assert!(e.contains("setup_defaults.owned[1]: expected bool"), "{e}");
        let e = refused("{ marks = { 300 } }");
        assert!(e.contains("setup_defaults.marks[1]: Int(300) doesn't fit the field"), "{e}");
        let e = refused("{ wears = { \"nothing\" } }");
        assert!(e.ends_with("setup_defaults.wears[1]: the content has no form \"nothing\""), "{e}");
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
                "    hooks = {\n        navi_intake = function(_side: number, navi: Object)\n            local s = rules.state() :: { intakes: number, x: number, y: number }\n            s.intakes += 1\n            s.x, s.y = navi.panel_x, navi.panel_y\n        end,\n        chip_check = function(_side: number, _navi: Object, chip: Chip?): Chip?\n            return if chip == test_chips[\"test/bomb\"] then test_chips[\"test/seed\"] else nil\n        end,\n",
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

    /// Records, lists and chip codes in the rules' state (step c1): a
    /// script reads and writes a record's fields and a list's elements by
    /// their places (`s.drive.mode`, `s.picks[1].code`, `#s.picks`), grows a
    /// list by its next element, and gives a record or a list a table
    /// whole; the engine finds a field by its name in the records
    /// (`Schema::find`).
    #[test]
    fn records_lists_and_codes_in_the_rules_state() {
        let content = testing::with_rules(&[
            (
                "        -- The counter's.\n        starts = \"u8\",",
                "        drive = { mode = \"u8\", target = \"chip\" },\n        picks = schema.list({ chip = \"chip\", code = \"code\" }, 3),\n        letters = schema.list(\"code\", 2),\n        -- The counter's.\n        starts = \"u8\",",
            ),
            ("local save = require(\"@self/save\")\n", "local save = require(\"@self/save\")\nlocal test_chips = require(\"./chips/test\")\n"),
            (
                "    hooks = {\n",
                "    hooks = {\n        navi_intake = function(_side: number, _navi: Object)\n            local s = rules.state()\n            s.drive.mode += 2\n            s.drive.target = test_chips[\"test/bomb\"]\n            s.picks[#s.picks + 1] = { chip = test_chips[\"test/seed\"], code = \"B\" }\n            s.picks[1].code = \"*\"\n            s.letters = { \"A\", \"Z\" }\n        end,\n",
            ),
        ]);
        let mut b = started_on(scenario::setup_on(&content), content.clone());
        let navi = b.player(1).expect("side 1's navi");
        b.rules_navi_intake(1, navi);
        b.rules_navi_intake(1, navi);
        let (schema, state) = b.rules_state(1).expect("the side's rules");
        let place = |name: &str| schema.place_of(schema.find(name).unwrap().unwrap_or_else(|| panic!("no `{name}`")));
        let (bomb, seed) = (testing::chip_in(&content, "test/bomb"), testing::chip_in(&content, "test/seed"));
        let chip = |c: nettai_content_api::ChipHandle| FieldValue::Ref(Some((nettai_content_api::Registry::Chip, c.0)));
        assert_eq!(state.get_at(place("mode")), FieldValue::U8(4), "a record's field, by its name in the records");
        assert_eq!(state.get_at(place("drive").field("target").unwrap()), chip(bomb));
        let picks = place("picks");
        assert_eq!(state.len_at(picks), Some(2));
        let pick = |k: usize, f: &str| state.get_at(picks.elem(k).unwrap().field(f).unwrap());
        assert_eq!((pick(0, "chip"), pick(0, "code")), (chip(seed), FieldValue::Code(Some(b'*'))));
        assert_eq!((pick(1, "chip"), pick(1, "code")), (chip(seed), FieldValue::Code(Some(b'B'))));
        assert_eq!(pick(2, "chip"), FieldValue::Ref(None), "past the list's length, nothing");
        let letters = place("letters");
        assert_eq!(state.len_at(letters), Some(2));
        assert_eq!(state.get_at(letters.elem(1).unwrap()), FieldValue::Code(Some(b'Z')));
        // A list's elements' fields are no one field: each element's has a
        // place of its own.
        assert_eq!(schema.find("chip"), Ok(None));
    }

    /// An SP navi chip's damage goes by how long its user's save took to
    /// delete its SP navi (step c2: EXE6's rules/sp_chips, a function of the
    /// side and the chip, which the round's setup asks): a step down per
    /// two seconds past ten, by the deletion time the side's `sp_times`
    /// states for the chip (none: no time, the best damage).
    #[test]
    fn an_sp_chips_damage_goes_by_its_deletion_time() {
        use nettai_content_api::Registry;
        let content = scenario::content();
        let count_sp = testing::chip_in(&content, "count-sp");
        let damage = |frames: Option<u16>| {
            let mut setup = scenario::setup();
            if let Some(f) = frames {
                let time = Fact::Record(vec![("chip", Fact::Value(Value::Def(Registry::Chip, count_sp.0))), ("frames", Fact::Value(Value::Int(f as i64)))]);
                setup.players[0].set_fact(&content, "sp_times", &[time]).unwrap();
            }
            started(setup).given.damage(count_sp, 0)
        };
        // Count[SP]'s steps: 50, 45, 45, 45, 40, ... (its `sp.by_time`).
        assert_eq!(damage(None), Some(50), "no time stated: in no time");
        assert_eq!(damage(Some(600)), Some(50), "10.00 s: no step passed");
        assert_eq!(damage(Some(601)), Some(45), "10.01 s: past the first");
        assert_eq!(damage(Some(721)), Some(45), "12.01 s: past the second");
        assert_eq!(damage(Some(60 * 60)), Some(30), "a minute: past every step");
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
        // A fact: the setup field that declares its role (`schema.role`),
        // else the field of its name, of its type.
        let setup = "        -- The counter's.
        bonus = \"u8\",";
        refused(
            setup,
            "        crosses = schema.role(\"form_list\", \"u8\"),
        bonus = \"u8\",",
            &["rules/init.luau: rules", "their setup field `crosses` is the fact a player brings as `form_list`, an array of forms"],
        );
        let content = patched(setup, "        crosses = schema.role(\"form_list\", \"form[5]\"),
        bonus = \"u8\",").expect("a fact of its type");
        assert_eq!(content.defs.fact_name(PlayerFact::FormList), Some("crosses"));
        let content = patched(setup, "        form_list = \"form[5]\",
        bonus = \"u8\",").expect("a fact by its name");
        assert_eq!(content.defs.fact_name(PlayerFact::FormList), Some("form_list"));
        // (A field named for another role than its own is that role's alone.)
        let content = patched(setup, "        form_list = schema.role(\"level\", \"u8?\"),
        bonus = \"u8\",").expect("a role elsewhere");
        assert_eq!((content.defs.fact_name(PlayerFact::FormList), content.defs.fact_name(PlayerFact::Level)), (None, Some("form_list")));
    }

    mod patch_cards {
        use super::*;
        use crate::setup::{GaugeSpeed, NaviStats, Supports};
        use nettai_content_api::Registry;

        /// A battle on the test content (whose rules apply EXE6's patch
        /// cards as the round is set up), side 0 with `cards` installed (by
        /// key), its stats changed by `tweak` first.
        fn with_cards(cards: &[&str], tweak: impl FnOnce(&mut NaviStats)) -> Battle {
            let content = scenario::content();
            let mut s = scenario::setup_on(&content);
            // (The setup's `patch_cards`: a card each.)
            let list: Vec<Fact> = cards
                .iter()
                .map(|&key| {
                    let card = content.defs.entry_in("patch_cards", key).unwrap_or_else(|| panic!("no card {key:?}"));
                    Fact::Value(Value::Def(Registry::Entry, card.0))
                })
                .collect();
            s.players[0].set_fact(&content, "patch_cards", &list).unwrap();
            tweak(&mut s.navi_stats[0]);
            Battle::new(s, content)
        }

        /// Side `side`'s stats as the setup gives them.
        fn setup_stats(side: usize) -> NaviStats {
            scenario::setup().navi_stats[side]
        }

        #[test]
        fn the_cards_are_entries_and_the_setups_part() {
            use nettai_content_api::Data;
            let content = scenario::content();
            // (An entry of the root's `patch_cards`: data the rules read.)
            let h = content.defs.entry_in("patch_cards", "test-stats").expect("the test card");
            let card = content.defs.definitions.get(Registry::Entry, &content.defs.entry(h).key).expect("its definition");
            assert_eq!(card.spec.field("mb").int(), Some(20));
            let Data::List(effects) = card.spec.field("effects") else { panic!("its effects") };
            let kinds: Vec<(&str, bool)> =
                effects.iter().map(|e| (e.field("kind").str().expect("a kind"), *e.field("bug") == Data::Bool(true))).collect();
            assert_eq!(kinds, [("hp_add", false), ("hp_percent_add", false), ("attack_add", false), ("body", false), ("hp_drain", true)]);
            // The cards are in the setup, which the digest covers.
            let a = with_cards(&["test-stats"], |_| {});
            let b = with_cards(&["test-later"], |_| {});
            assert_ne!(a.setup.players[0].rules, b.setup.players[0].rules);
            assert_ne!(a.digest(), b.digest());
        }

        #[test]
        fn a_card_changes_the_stats_by_its_kinds_order() {
            let b = with_cards(&["test-stats"], |_| {});
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
            let b = with_cards(&["test-stats", "test-later"], |_| {});
            let s = &b.stats[0];
            assert_eq!(s.attack, 2, "Attack 0 + 3 - 1");
            assert_eq!(s.giga_level, 0xFF, "GigaFolder- doesn't clamp");
        }

        #[test]
        fn abilities_choices_and_chip_shuffle() {
            let content = scenario::content();
            let b = with_cards(&["test-abilities"], |s| {
                s.support = Some(Supports::default());
                s.float_shoes = true;
                s.set_game_stat(&content, "number_open", Value::Bool(true)).unwrap();
            });
            let s = &b.stats[0];
            let content = &b.content;
            assert!(s.super_armor && !s.float_shoes);
            assert_eq!(s.first_barrier, content.defs.record("barrier/200"));
            assert_eq!(s.weapons.charge_shot_kind, content.defs.record("shot/charged-confusing"));
            assert_eq!(s.support, Some(Supports { rush: true, ..Supports::default() }));
            assert_eq!(s.gauge_speed, GaugeSpeed::Fast);
            let stat = |name: &str| s.game_stat(content, name);
            assert_eq!(
                (stat("chip_shuffle"), stat("number_open")),
                (Some(FieldValue::Bool(true)), Some(FieldValue::Bool(false))),
                "ChpShufl turns NumbrOpn off"
            );
            assert!(!b.consoles[0].emotion_window_glitch, "no bug");
        }

        #[test]
        fn with_cards_installed_the_glitch_follows_the_stats() {
            let bugged = with_cards(&["test-abilities"], |s| {
                s.support = Some(Supports::default());
                s.bugs.emotion = 1;
            });
            assert!(bugged.consoles[0].emotion_window_glitch, "a NaviCust bug counts with cards installed (flag 0x1723)");
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
            let b = with_cards(&["test-abilities"], |s| s.support = None);
            assert_eq!(b.stats[0].support, None, "the byte 0xFF stays 0xFF when a bit is set");
        }
    }
}
