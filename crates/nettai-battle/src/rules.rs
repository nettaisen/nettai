//! The players' rules (docs/design/rules-in-luau.md): each side plays by a
//! ruleset, a list of systems written in Luau; the framework calls each
//! side's systems at its points (a hook a system fills), and keeps each
//! system's state of each side here, in the battle, where snapshots and the
//! digest cover it.
//!
//! A system reaches only its own state of the side it was called for
//! (`system.state()` in a hook): a side's rules see the other side through
//! the engine alone.

use nettai_content_api::{ChipHandle, ContentState, FieldType, HookCall, ObjectRef, SystemHook, Value, WeaponHandle};

use crate::battle::Battle;
use crate::content::Content;
use crate::custom::PlayerSetup;

/// A side's rules in a battle: each of the game's ruleset's systems' state
/// of the side, in the ruleset's order (none: the content has no ruleset).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct SideRules {
    pub states: Vec<ContentState>,
}

impl SideRules {
    /// A player's rules at a round's start: the game's ruleset's systems'
    /// state, zeroed (a game has one ruleset, which every match of it
    /// plays by). Their setup's blocks are made the systems' defaults
    /// (`setup_defaults`, the rest zero) for a setup that gives none, and
    /// must otherwise be the ruleset's. An enum of a system's setup has no
    /// default but its `setup_defaults`': the round doesn't start with one
    /// the player's setup leaves unstated (EXE6's version: nothing fills in
    /// falzar or gregar).
    pub fn for_player(content: &Content, player: &mut PlayerSetup) -> SideRules {
        if content.defs.ruleset().is_none() {
            assert!(player.rules.is_empty(), "a player's setup gives system setups, and the content has no ruleset");
            return SideRules::default();
        }
        let systems: Vec<_> = content.defs.ruleset_systems().iter().map(|&h| content.defs.system(h)).collect();
        if player.rules.is_empty() {
            player.rules = systems.iter().map(|s| s.setup_block()).collect();
        }
        let fits = player.rules.len() == systems.len() && player.rules.iter().zip(&systems).all(|(b, s)| b.id() == s.setup);
        assert!(fits, "a player's setup gives system setups that aren't the game's ruleset's");
        for (block, system) in player.rules.iter().zip(&systems) {
            let schema = content.defs.schema(system.setup);
            for (i, field) in schema.fields().iter().enumerate() {
                if let (FieldType::Enum(names), false) = (&field.ty, block.stated(schema, i)) {
                    panic!(
                        "a player's setup doesn't state the {} system's `{}` ({}): none is assumed",
                        system.key,
                        field.name,
                        names.join(" or ")
                    );
                }
            }
        }
        SideRules { states: systems.iter().map(|s| ContentState::new(s.state)).collect() }
    }
}

impl PlayerSetup {
    /// Set field `field` of system `system`'s setup (by key; a system of
    /// the game's ruleset) to `v`: how tools write what a save says.
    pub fn set_rule(&mut self, content: &Content, system: &str, field: &str, v: Value) -> Result<(), String> {
        let (block, schema, i) = self.rule_field(content, system, field)?;
        block.set(schema, i, v).map_err(|e| format!("system {system}'s setup field `{field}`: {e}"))
    }

    /// [`PlayerSetup::set_rule`] for an array field: element `k` of it.
    pub fn set_rule_elem(&mut self, content: &Content, system: &str, field: &str, k: usize, v: Value) -> Result<(), String> {
        let (block, schema, i) = self.rule_field(content, system, field)?;
        block.set_elem(schema, i, k, v).map_err(|e| format!("system {system}'s setup field `{field}`: {e}"))
    }

    /// The setup block of system `system` (by key) of the game's ruleset
    /// and its layout, as the round will start with it (the systems'
    /// defaults where the setup gives none): what a tool shows of it.
    pub fn rule_block<'a>(&self, content: &'a Content, system: &str) -> Option<(&'a nettai_content_api::Schema, ContentState)> {
        let systems = content.defs.ruleset_systems();
        let slot = systems.iter().position(|&h| content.defs.system(h).key == system)?;
        let def = content.defs.system(systems[slot]);
        let block = self.rules.get(slot).copied().unwrap_or_else(|| def.setup_block());
        Some((content.defs.schema(def.setup), block))
    }

    /// Write a fact of what the player brings into each system of the
    /// game's ruleset whose setup has a
    /// field `field`: one value for a field, an element each for an array
    /// (the rest zero), an enum's by its name (`Fact::Name`). How a tool
    /// writes what several systems read (EXE6's game version, which its
    /// cross and beast systems both take). The number of systems that took
    /// it: none on a content without a ruleset.
    pub fn set_fact(&mut self, content: &Content, field: &str, values: &[Fact]) -> Result<usize, String> {
        if content.defs.ruleset().is_none() {
            return Ok(0);
        }
        if self.rules.is_empty() {
            self.rules = content.defs.ruleset_systems().iter().map(|&h| content.defs.system(h).setup_block()).collect();
        }
        let mut took = 0;
        for block in &mut self.rules {
            let schema = &content.defs.schemas[block.id().0 as usize].schema;
            let Some(i) = schema.index_of(field) else { continue };
            let ty = &schema.field(i).ty;
            let value = |f: &Fact| -> Result<Value, String> {
                match (f, ty) {
                    (Fact::Value(v), _) => Ok(*v),
                    (Fact::Name(n), FieldType::Enum(names)) => names
                        .iter()
                        .position(|x| x == n)
                        .map(|i| Value::Int(i as i64))
                        .ok_or_else(|| format!("setup field `{field}` has no variant {n:?}")),
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
            took += 1;
        }
        Ok(took)
    }

    /// The setup block of system `system` (by key) of the game's ruleset,
    /// its schema and the index of its field `field`; the blocks made the
    /// systems' defaults first if the setup gives none.
    fn rule_field<'a>(
        &'a mut self,
        content: &'a Content,
        system: &str,
        field: &str,
    ) -> Result<(&'a mut ContentState, &'a nettai_content_api::Schema, usize), String> {
        let systems = content.defs.ruleset().ok_or("the content has no ruleset")?.systems.as_slice();
        if self.rules.is_empty() {
            self.rules = systems.iter().map(|&h| content.defs.system(h).setup_block()).collect();
        }
        let slot = systems
            .iter()
            .position(|&h| content.defs.system(h).key == system)
            .ok_or_else(|| format!("the game's ruleset has no system {system}"))?;
        let block = &mut self.rules[slot];
        let schema = &content.defs.schemas[block.id().0 as usize].schema;
        let i = schema.index_of(field).ok_or_else(|| format!("system {system}'s setup has no field `{field}`"))?;
        Ok((block, schema, i))
    }
}

/// A value [`PlayerSetup::set_fact`] writes: a field's value, or an enum
/// variant by its name.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fact<'a> {
    Value(Value),
    Name(&'a str),
}

/// A fact of what a player brought, read back ([`Battle::fact`]): a field
/// of a system's setup.
#[derive(Clone, Copy)]
pub struct SetupFact<'a> {
    schema: &'a nettai_content_api::Schema,
    block: &'a ContentState,
    index: usize,
}

impl<'a> SetupFact<'a> {
    /// Its value (an array's first element's).
    pub fn value(&self) -> nettai_content_api::FieldValue {
        self.block.get(self.schema, self.index)
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
}

impl Battle {
    /// A fact of what side `side`'s player brought, by its name (what
    /// [`PlayerSetup::set_fact`] writes): the setup field `field` of the
    /// first system of the game's ruleset that has one. For a reader of
    /// what a console shows of its player (their game's version, what their
    /// save unlocks), whichever system keeps it; none when no system does,
    /// or the player's setup gives the systems none.
    pub fn fact(&self, side: u8, field: &str) -> Option<SetupFact<'_>> {
        let rules = &self.setup.players[side as usize & 1].rules;
        self.content.defs.ruleset_systems().iter().enumerate().find_map(|(slot, &h)| {
            let schema = self.content.defs.schema(self.content.defs.system(h).setup);
            Some(SetupFact { schema, block: rules.get(slot)?, index: schema.index_of(field)? })
        })
    }

    /// Side `side`'s system `key`'s setup block (the player's, as the round
    /// started with it) and its layout, when the side's ruleset has that
    /// system: for a reader of what a player brought by the system that
    /// keeps it (a game's tools; a frontend reads a fact by its name,
    /// [`Battle::fact`]).
    pub fn system_setup(&self, side: u8, key: &str) -> Option<(&nettai_content_api::Schema, &ContentState)> {
        let systems = self.content.defs.ruleset_systems();
        let slot = systems.iter().position(|&h| self.content.defs.system(h).key == key)?;
        let def = self.content.defs.system(systems[slot]);
        Some((self.content.defs.schema(def.setup), self.setup.players[side as usize & 1].rules.get(slot)?))
    }

    /// Side `side`'s rules.
    pub fn side_rules(&self, side: u8) -> &SideRules {
        &self.rules[side as usize]
    }

    /// Where system `system` is in side `side`'s ruleset, as the binding
    /// takes it (the side and the place), if the side plays by it.
    pub(crate) fn system_slot(&self, side: u8, system: nettai_content_api::SystemHandle) -> Option<(u8, u8)> {
        self.rules.get(side as usize)?;
        let slot = self.content.defs.ruleset_systems().iter().position(|&h| h == system)?;
        Some((side, slot as u8))
    }

    /// Call `hook` of each system of side 0's ruleset that has one, in the
    /// ruleset's order, then side 1's (the original's order wherever it
    /// loops over the sides).
    pub(crate) fn notify_systems(&mut self, hook: SystemHook) {
        for side in 0..2u8 {
            self.notify_side(side, hook);
        }
    }

    /// What side `side`'s rules say of a folder (their systems'
    /// `folder_check`: EXE6's folder rules), each rule it breaks named and
    /// said; nothing when it keeps them, or when the rules have none. The
    /// folder's chips in order, its Regular and tag chips (entries of
    /// `chips`); `complete`: all of a folder, else the chips so far (the
    /// rules about a whole folder wait). The rules read the side's stats as
    /// the round set them up (its folder limits). For tools (a match's
    /// checks, a random folder's draw): no part of the simulation.
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
        self.notify_side(side & 1, SystemHook::FolderCheck);
        self.folder_check.take().map(|c| c.problems).unwrap_or_default()
    }

    /// Call `hook` of each system of side `side`'s ruleset that has one.
    pub(crate) fn notify_side(&mut self, side: u8, hook: SystemHook) {
        let content = self.content.clone();
        for (slot, &h) in content.defs.ruleset_systems().iter().enumerate() {
            if let Some(f) = content.defs.system(h).hook(hook) {
                let call = HookCall::System { side, slot: slot as u8, hook, navi: None, chip: None, weapon: None };
                crate::behavior::call_hook(self, f, call);
            }
        }
    }

    /// Side `side`'s systems' `navi_intake(side, navi)`, each tick of the
    /// fight in the navi's intake. (A ruleset without the hook calls
    /// nothing: EXE6's.)
    pub(crate) fn systems_navi_intake(&mut self, side: u8, navi: ObjectRef) {
        let content = self.content.clone();
        for (slot, &h) in content.defs.ruleset_systems().iter().enumerate() {
            if let Some(f) = content.defs.system(h).hook(SystemHook::NaviIntake) {
                let call = HookCall::System { side, slot: slot as u8, hook: SystemHook::NaviIntake, navi: Some(navi), chip: None, weapon: None };
                crate::behavior::call_hook(self, f, call);
            }
        }
    }

    /// Side `side`'s systems' `chip_prepared(side, navi, chip)` once a
    /// chip's use is prepared (`sub_80127C0`): `chip` the chip it uses (the
    /// zeroed chip for the empty hand).
    pub(crate) fn systems_chip_prepared(&mut self, side: u8, navi: ObjectRef, chip: ChipHandle) {
        let content = self.content.clone();
        for (slot, &h) in content.defs.ruleset_systems().iter().enumerate() {
            if let Some(f) = content.defs.system(h).hook(SystemHook::ChipPrepared) {
                let call = HookCall::System { side, slot: slot as u8, hook: SystemHook::ChipPrepared, navi: Some(navi), chip: Some(chip), weapon: None };
                crate::behavior::call_hook(self, f, call);
            }
        }
    }

    /// Side `side`'s systems' `chip_used(side, navi, chip, weapon)` once a
    /// chip's use has started its action.
    pub(crate) fn systems_chip_used(&mut self, side: u8, navi: ObjectRef, chip: ChipHandle, weapon: Option<WeaponHandle>) {
        let content = self.content.clone();
        for (slot, &h) in content.defs.ruleset_systems().iter().enumerate() {
            if let Some(f) = content.defs.system(h).hook(SystemHook::ChipUsed) {
                let call = HookCall::System { side, slot: slot as u8, hook: SystemHook::ChipUsed, navi: Some(navi), chip: Some(chip), weapon };
                crate::behavior::call_hook(self, f, call);
            }
        }
    }

    /// Side `side`'s systems' `controller(side, navi)`: the outcome (the
    /// original's number: 0 nothing, 1 a chip, 2 the buster, 3 a step) the
    /// first system that answers gives; none answering is nothing.
    pub(crate) fn systems_controller(&mut self, side: u8, navi: ObjectRef) -> u8 {
        let content = self.content.clone();
        for (slot, &h) in content.defs.ruleset_systems().iter().enumerate() {
            if let Some(f) = content.defs.system(h).hook(SystemHook::Controller) {
                let call = HookCall::System { side, slot: slot as u8, hook: SystemHook::Controller, navi: Some(navi), chip: None, weapon: None };
                if let Value::Int(n) = crate::behavior::call_hook(self, f, call) {
                    return n as u8;
                }
            }
        }
        0
    }

    /// Side `side`'s systems' `controller(side, navi)`, asked of the side's
    /// own navi (EXE5's no-charge drive): the outcome the first system that
    /// answers gives; None when none answers.
    pub(crate) fn systems_controller_answer(&mut self, side: u8, navi: ObjectRef) -> Option<u8> {
        let content = self.content.clone();
        for (slot, &h) in content.defs.ruleset_systems().iter().enumerate() {
            if let Some(f) = content.defs.system(h).hook(SystemHook::Controller) {
                let call = HookCall::System { side, slot: slot as u8, hook: SystemHook::Controller, navi: Some(navi), chip: None, weapon: None };
                if let Value::Int(n) = crate::behavior::call_hook(self, f, call) {
                    return Some(n as u8);
                }
            }
        }
        None
    }

    /// The `controller(side, navi)` of side `side`'s system in place `slot`
    /// (a navi no player controls: the system that drives it).
    pub(crate) fn systems_controller_at(&mut self, side: u8, slot: u8, navi: ObjectRef) {
        let content = self.content.clone();
        let Some(&h) = content.defs.ruleset_systems().get(slot as usize) else { return };
        if let Some(f) = content.defs.system(h).hook(SystemHook::Controller) {
            let call = HookCall::System { side, slot, hook: SystemHook::Controller, navi: Some(navi), chip: None, weapon: None };
            crate::behavior::call_hook(self, f, call);
        }
    }

    /// Side `side`'s systems' `takeover_requested(side, navi)`.
    pub(crate) fn systems_takeover_requested(&mut self, side: u8, navi: ObjectRef) {
        let content = self.content.clone();
        for (slot, &h) in content.defs.ruleset_systems().iter().enumerate() {
            if let Some(f) = content.defs.system(h).hook(SystemHook::TakeoverRequested) {
                let call = HookCall::System {
                    side,
                    slot: slot as u8,
                    hook: SystemHook::TakeoverRequested,
                    navi: Some(navi),
                    chip: None,
                    weapon: None,
                };
                crate::behavior::call_hook(self, f, call);
            }
        }
    }

    /// Side `side`'s systems' `takeover(side, navi)`: the outcome the
    /// first system that answers gives (as `systems_controller`'s, or 4:
    /// an attack of its own); none answering is nothing.
    pub(crate) fn systems_takeover(&mut self, side: u8, navi: ObjectRef) -> u8 {
        let content = self.content.clone();
        for (slot, &h) in content.defs.ruleset_systems().iter().enumerate() {
            if let Some(f) = content.defs.system(h).hook(SystemHook::Takeover) {
                let call = HookCall::System { side, slot: slot as u8, hook: SystemHook::Takeover, navi: Some(navi), chip: None, weapon: None };
                if let Value::Int(n) = crate::behavior::call_hook(self, f, call) {
                    return n as u8;
                }
            }
        }
        0
    }

    /// Side `side`'s systems' buttons, in the order the systems are listed.
    pub(crate) fn side_buttons(&self, _side: u8) -> Vec<crate::content::ButtonHandle> {
        self.content.defs.ruleset_systems().iter().flat_map(|&h| self.content.defs.system(h).buttons.iter().copied()).collect()
    }

    /// One of a button's functions (`shown`, `state`, `pressed`), as its
    /// system's for side `side`.
    pub(crate) fn call_button(&mut self, side: u8, button: crate::content::ButtonHandle, hook: SystemHook) -> Value {
        let content = self.content.clone();
        let d = content.defs.button(button);
        let f = match hook {
            SystemHook::ButtonShown => d.shown,
            SystemHook::ButtonState => d.state.expect("a button's state, asked only when it has one"),
            SystemHook::ButtonPressed => d.pressed,
            SystemHook::ButtonTakenBack => d.taken_back.expect("a button's taken_back, asked only when it has one"),
            SystemHook::ButtonChip => d.chip.expect("a button's chip, asked only when it has one"),
            h => panic!("{h:?} is no button's function"),
        };
        let (_, slot) = self.system_slot(side, d.system).expect("a button of the side's systems");
        let call = HookCall::System { side, slot, hook, navi: None, chip: None, weapon: None };
        crate::behavior::call_hook(self, f, call)
    }

    /// A window's `update`, as its system's for side `side`.
    pub(crate) fn call_window(&mut self, side: u8, window: crate::content::WindowHandle) -> Value {
        let content = self.content.clone();
        let d = content.defs.window(window);
        let (_, slot) = self.system_slot(side, d.system).expect("a window of the side's systems");
        let call = HookCall::System { side, slot, hook: SystemHook::WindowUpdate, navi: None, chip: None, weapon: None };
        crate::behavior::call_hook(self, d.update, call)
    }

    /// Side `side`'s systems' custom chip hook `hook(side, chip)`, each in
    /// order.
    pub(crate) fn systems_call_custom_chip(&mut self, side: u8, hook: SystemHook, chip: ChipHandle) {
        let content = self.content.clone();
        for (slot, &h) in content.defs.ruleset_systems().iter().enumerate() {
            if let Some(f) = content.defs.system(h).hook(hook) {
                let call = HookCall::System { side, slot: slot as u8, hook, navi: None, chip: Some(chip), weapon: None };
                crate::behavior::call_hook(self, f, call);
            }
        }
    }

    /// Side `side`'s systems' custom hook `hook(side)`, each in order.
    pub(crate) fn systems_call_custom(&mut self, side: u8, hook: SystemHook) {
        let content = self.content.clone();
        for (slot, &h) in content.defs.ruleset_systems().iter().enumerate() {
            if let Some(f) = content.defs.system(h).hook(hook) {
                let call = HookCall::System { side, slot: slot as u8, hook, navi: None, chip: None, weapon: None };
                crate::behavior::call_hook(self, f, call);
            }
        }
    }

    /// Side `side`'s systems' custom hook `hook(side)` in order, until one
    /// answers true: whether one did.
    pub(crate) fn systems_ask_custom(&mut self, side: u8, hook: SystemHook) -> bool {
        let content = self.content.clone();
        for (slot, &h) in content.defs.ruleset_systems().iter().enumerate() {
            if let Some(f) = content.defs.system(h).hook(hook) {
                let call = HookCall::System { side, slot: slot as u8, hook, navi: None, chip: None, weapon: None };
                if crate::behavior::call_hook(self, f, call) == Value::Bool(true) {
                    return true;
                }
            }
        }
        false
    }

    /// Side `side`'s system `key`'s state and its layout, when the side's
    /// ruleset has that system: for a reader of what a system keeps (the
    /// frontend's look of EXE6's Cross window reads the cross system's).
    pub fn system_state(&self, side: u8, key: &str) -> Option<(&nettai_content_api::Schema, &ContentState)> {
        let systems = &self.content.defs.ruleset_systems();
        let slot = systems.iter().position(|&h| self.content.defs.system(h).key == key)?;
        let def = self.content.defs.system(systems[slot]);
        Some((self.content.defs.schema(def.state), self.rules[side as usize & 1].states.get(slot)?))
    }

    /// Side `side`'s systems' `custom.hand_size(side)`: the first answer.
    pub(crate) fn systems_custom_hand_size(&mut self, side: u8) -> Option<u8> {
        let content = self.content.clone();
        for (slot, &h) in content.defs.ruleset_systems().iter().enumerate() {
            if let Some(f) = content.defs.system(h).hook(SystemHook::CustomHandSize) {
                let call = HookCall::System { side, slot: slot as u8, hook: SystemHook::CustomHandSize, navi: None, chip: None, weapon: None };
                if let Value::Int(n) = crate::behavior::call_hook(self, f, call) {
                    return Some(n as u8);
                }
            }
        }
        None
    }

    /// Side `side`'s systems' `starting_mood(side)`: the first answer.
    pub(crate) fn systems_starting_mood(&mut self, side: u8) -> Option<u8> {
        self.systems_ask(side, SystemHook::StartingMood, None)
    }

    /// Side `side`'s systems' `navi_palette(side, navi)`: the first answer.
    pub(crate) fn systems_navi_palette(&mut self, side: u8, navi: ObjectRef) -> Option<u8> {
        self.systems_ask(side, SystemHook::NaviPalette, Some(navi))
    }

    /// Side `side`'s systems' `navi_bug(side, navi)`: whether one answered
    /// true (the bug and the weapons' reload skipped).
    pub(crate) fn systems_navi_bug(&mut self, side: u8, navi: ObjectRef) -> bool {
        let content = self.content.clone();
        for (slot, &h) in content.defs.ruleset_systems().iter().enumerate() {
            if let Some(f) = content.defs.system(h).hook(SystemHook::NaviBug) {
                let call = HookCall::System { side, slot: slot as u8, hook: SystemHook::NaviBug, navi: Some(navi), chip: None, weapon: None };
                if let Value::Bool(true) = crate::behavior::call_hook(self, f, call) {
                    return true;
                }
            }
        }
        false
    }

    /// Side `side`'s systems' `hook`, in order, until one answers a number.
    fn systems_ask(&mut self, side: u8, hook: SystemHook, navi: Option<ObjectRef>) -> Option<u8> {
        let content = self.content.clone();
        for (slot, &h) in content.defs.ruleset_systems().iter().enumerate() {
            if let Some(f) = content.defs.system(h).hook(hook) {
                let call = HookCall::System { side, slot: slot as u8, hook, navi, chip: None, weapon: None };
                if let Value::Int(n) = crate::behavior::call_hook(self, f, call) {
                    return Some(n as u8);
                }
            }
        }
        None
    }

    /// Side `side`'s systems' `countered(side, victim)`.
    pub(crate) fn systems_countered(&mut self, side: u8, victim: ObjectRef) {
        self.systems_call(side, SystemHook::Countered, victim);
    }

    /// Side `side`'s systems' `navi_tick(side, navi)`.
    pub(crate) fn systems_navi_tick(&mut self, side: u8, navi: ObjectRef) {
        self.systems_call(side, SystemHook::NaviTick, navi);
    }

    /// Side `side`'s systems' `hook(side, navi)`, each in order; the
    /// results unused.
    fn systems_call(&mut self, side: u8, hook: SystemHook, navi: ObjectRef) {
        let content = self.content.clone();
        for (slot, &h) in content.defs.ruleset_systems().iter().enumerate() {
            if let Some(f) = content.defs.system(h).hook(hook) {
                let call = HookCall::System { side, slot: slot as u8, hook, navi: Some(navi), chip: None, weapon: None };
                crate::behavior::call_hook(self, f, call);
            }
        }
    }

    /// Side `side`'s systems' `form_reverted(side, navi)`.
    pub(crate) fn systems_form_reverted(&mut self, side: u8, navi: ObjectRef) {
        let content = self.content.clone();
        for (slot, &h) in content.defs.ruleset_systems().iter().enumerate() {
            if let Some(f) = content.defs.system(h).hook(SystemHook::FormReverted) {
                let call = HookCall::System { side, slot: slot as u8, hook: SystemHook::FormReverted, navi: Some(navi), chip: None, weapon: None };
                crate::behavior::call_hook(self, f, call);
            }
        }
    }

    /// Side `side`'s systems' `chip_check(side, navi, chip)` as a chip's
    /// use is prepared: the chip the first system that answers puts in its
    /// place, or none (the use goes ahead).
    pub(crate) fn systems_chip_check(&mut self, side: u8, navi: ObjectRef, chip: Option<ChipHandle>) -> Option<ChipHandle> {
        self.systems_chip_answer(side, navi, chip, SystemHook::ChipCheck)
    }

    /// Side `side`'s systems' `chip_cost(side, navi, chip)`, earlier in
    /// the preparation: as `systems_chip_check`.
    pub(crate) fn systems_chip_cost(&mut self, side: u8, navi: ObjectRef, chip: Option<ChipHandle>) -> Option<ChipHandle> {
        self.systems_chip_answer(side, navi, chip, SystemHook::ChipCost)
    }

    /// Side `side`'s systems' `chip_substitute(side, navi, chip)` before a
    /// chip's record is loaded: the chip the first system that answers
    /// puts in its place (EXE6's dark chips' substitute), or none.
    pub(crate) fn systems_chip_substitute(&mut self, side: u8, navi: ObjectRef, chip: ChipHandle) -> Option<ChipHandle> {
        self.systems_chip_answer(side, navi, Some(chip), SystemHook::ChipSubstitute)
    }

    /// A chip hook `hook(side, navi, chip)`'s first answer, a chip.
    fn systems_chip_answer(&mut self, side: u8, navi: ObjectRef, chip: Option<ChipHandle>, hook: SystemHook) -> Option<ChipHandle> {
        let content = self.content.clone();
        for (slot, &h) in content.defs.ruleset_systems().iter().enumerate() {
            if let Some(f) = content.defs.system(h).hook(hook) {
                let call = HookCall::System { side, slot: slot as u8, hook, navi: Some(navi), chip, weapon: None };
                if let Value::Def(nettai_content_api::Registry::Chip, c) = crate::behavior::call_hook(self, f, call) {
                    return Some(ChipHandle(c));
                }
            }
        }
        None
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

    /// The same on `content`: the test content with another list of
    /// systems (`testing::with_systems`: a game has one ruleset).
    fn started_on(mut setup: RoundSetup, content: std::sync::Arc<Content>) -> Battle {
        setup.content = content.hash();
        let mut b = Battle::new(setup, content);
        for _ in 0..3 {
            b.tick(&[PlayerTick::default(); 2], Default::default());
        }
        b
    }

    /// Field `field` of the state of the system in place `slot` of side
    /// `side`.
    fn field(b: &Battle, side: u8, slot: usize, field: &str) -> FieldValue {
        let s = &b.side_rules(side).states[slot];
        let schema = &b.content.defs.schemas[s.id().0 as usize].schema;
        s.get(schema, schema.index_of(field).expect("a field"))
    }

    /// EXE6's bug frags are its dark-chips system's (docs/design/
    /// rules-in-luau.md, As built S8): the player brings them in the
    /// system's setup (a tool writes them as a fact), and the round starts
    /// with them in its state, which the chips spend through EXE6's API.
    #[test]
    fn the_bug_frags_are_the_dark_chips_systems() {
        let content = scenario::content();
        let mut setup = scenario::setup();
        let took = setup.players[0]
            .set_fact(&content, "bug_frags", &[Fact::Value(nettai_content_api::Value::Int(7))])
            .expect("a count of bug frags");
        assert_eq!(took, 1, "the dark-chips system alone takes them");
        let b = started(setup);
        assert_eq!((testing::bug_frags(&b, 0), testing::bug_frags(&b, 1)), (7, 0));
    }

    /// An enum of a system's setup has no default: a player's setup that
    /// says nothing leaves EXE6's beast system's `version` unstated, and the
    /// round doesn't start (nothing fills in falzar, the enum's first name);
    /// stated by name, it starts and the system reads it. An enum of a
    /// system's state starts at its first variant as ever.
    #[test]
    fn a_setups_enum_has_no_default() {
        let content = scenario::content();
        let mut setup = scenario::setup();
        for p in &mut setup.players {
            p.rules.clear();
        }
        let (schema, block) = setup.players[0].rule_block(&content, "beast").expect("EXE6's beast system");
        let version = schema.index_of("version").expect("its version");
        assert!(!block.stated(schema, version) && block.stated(schema, schema.index_of("beast_out").unwrap()));
        let refused = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| Battle::new(setup.clone(), content.clone())));
        let why = refused.err().and_then(|e| e.downcast_ref::<String>().cloned()).expect("the round doesn't start");
        assert_eq!(why, "a player's setup doesn't state the beast system's `version` (falzar or gregar): none is assumed");
        // One player's stated: the other's still stops it.
        setup.players[0].set_fact(&content, "version", &[Fact::Name("gregar")]).unwrap();
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| Battle::new(setup.clone(), content.clone()))).is_err());
        setup.players[1].set_fact(&content, "version", &[Fact::Name("falzar")]).unwrap();
        let b = Battle::new(setup, content);
        assert_eq!((b.fact(0, "version").and_then(|f| f.name()), b.fact(1, "version").and_then(|f| f.name())), (Some("gregar"), Some("falzar")));
    }

    #[test]
    fn each_side_runs_the_rulesets_systems_for_itself() {
        let b = started(scenario::setup());
        let content = &b.content;
        assert_eq!(content.defs.ruleset().expect("the test content's rules").systems.len(), 5);
        for side in 0..2u8 {
            // (EXE6's beast system first, then the counter, then EXE6's forms,
            // emotion and dark-chips systems.)
            assert_eq!(b.side_rules(side).states.len(), 5);
            assert_eq!(field(&b, side, 1, "starts"), FieldValue::U8(1), "round_start ran once for side {side}");
            assert_eq!(field(&b, side, 1, "side"), FieldValue::U8(side), "it ran for its own side");
        }
    }

    /// One ruleset a game (the user: "there should only be one ruleset per
    /// game"): both sides play by its systems, in its order, each with its
    /// own state of them. (Another list of systems is another content's.)
    #[test]
    fn both_sides_play_by_the_games_ruleset() {
        let content = testing::with_systems("marker, counter");
        let b = started_on(scenario::setup_on(&content), content);
        for side in 0..2u8 {
            assert_eq!(b.side_rules(side).states.len(), 2, "side {side} plays by the match's");
            assert_eq!(field(&b, side, 0, "mark"), FieldValue::U8(0x40 + side));
            assert_eq!(field(&b, side, 1, "side"), FieldValue::U8(side));
        }
    }

    #[test]
    fn a_systems_player_setup_reaches_it_and_no_other() {
        let content = scenario::content();
        let mut setup = scenario::setup();
        setup.players[0].set_rule(&content, "test/counter", "bonus", Value::Int(7)).unwrap();
        assert!(setup.players[0].set_rule(&content, "test/marker", "mark", Value::Int(1)).is_err(), "not the ruleset's");
        let b = started(setup);
        assert_eq!(field(&b, 0, 1, "bonus"), FieldValue::U16(14));
        assert_eq!(field(&b, 1, 1, "bonus"), FieldValue::U16(0), "the other player's setup is its own");
    }

    /// The per-tick and chip-use hooks (docs/design/exe5-map.md §15.3 item
    /// 14): a system's `navi_intake` is called with the side and its navi,
    /// and its `chip_check` with the chip about to be used, whose answer
    /// takes the chip's place; a side whose rules lack them calls nothing.
    #[test]
    fn a_systems_intake_and_chip_check_hooks() {
        let content = testing::with_systems("watcher");
        let mut b = started_on(scenario::setup_on(&content), content.clone());
        let navi = b.player(1).expect("side 1's navi");
        b.systems_navi_intake(1, navi);
        b.systems_navi_intake(1, navi);
        let p = b.objects.get(navi).panel;
        assert_eq!(field(&b, 1, 0, "intakes"), FieldValue::U16(2));
        assert_eq!((field(&b, 1, 0, "x"), field(&b, 1, 0, "y")), (FieldValue::U8(p.x), FieldValue::U8(p.y)));
        let bomb = testing::chip_in(&content, "test/bomb");
        let seed = testing::chip_in(&content, "test/seed");
        assert_eq!(b.systems_chip_check(1, navi, Some(bomb)), Some(seed));
        assert_eq!(b.systems_chip_check(1, navi, Some(seed)), None);
        assert_eq!(b.systems_chip_check(1, navi, None), None);
        // The test content's own systems have neither hook.
        let mut stock = started(scenario::setup());
        let navi0 = stock.player(0).expect("side 0's navi");
        assert_eq!(stock.systems_chip_check(0, navi0, Some(bomb)), None);
    }

    #[test]
    fn the_rules_are_in_the_digest_and_the_snapshot() {
        let b = started(scenario::setup());
        let copy = b.clone();
        assert_eq!(copy.digest(), b.digest());
        let mut changed = b.clone();
        let s = &mut changed.rules[1].states[1];
        let schema = &b.content.defs.schemas[s.id().0 as usize].schema;
        s.set(schema, schema.index_of("starts").unwrap(), Value::Int(9)).unwrap();
        assert_ne!(changed.digest(), b.digest());
    }

    /// A game without one of EXE6's systems: the test content's rules less
    /// EXE6's forms system, the marker after them.
    #[test]
    fn a_ruleset_lists_the_systems_its_game_plays_by() {
        let content = testing::with_systems("beast, counter, emotion.system, dark_chips, marker");
        let defs = &content.defs;
        let names: Vec<&str> = defs.ruleset_systems().iter().map(|&h| defs.system(h).key.as_str()).collect();
        assert_eq!(names, ["beast", "test/counter", "emotion", "dark-chips", "test/marker"]);
        let b = started_on(scenario::setup_on(&content), content.clone());
        assert_eq!(b.side_rules(1).states.len(), 5);
        assert_eq!(field(&b, 1, 4, "mark"), FieldValue::U8(0x41), "the marker ran for its side");
        assert_eq!(field(&b, 1, 1, "starts"), FieldValue::U8(1));
    }

    /// EXE6's patch-cards system (content/exe6/rules/patch-cards) with the
    /// test content's made-up cards: its `round_setup` changes the stats
    /// before anything reads them.
    /// A system's extension of its game's definitions
    /// (docs/design/rules-in-luau.md §7.5): kept on the definition, which a
    /// tool reads through `Defs::extension`, and checked as the content is
    /// defined: its types, its tables' fields, its variants, one owner.
    #[test]
    fn a_system_extends_its_games_definitions() {
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
        let systems = "rules/systems";
        refused(systems, "            test_weight = \"u8\",", "            test_weight = \"u9\",", "no type is named \"u9\"");
        refused(systems, "        chip = {\n            test_weight", "        stage = {},\n        chip = {\n            test_weight", "a system extends chip, form or navi");
        refused(
            systems,
            "    id = \"test/marker\",",
            "    id = \"test/marker\",\n    extends = { chip = { test_weight = \"u8\" } },",
            "both extend chip definitions with `test_weight`",
        );
    }

    /// A system says, for tools, the chips its rules can't play of a
    /// player's auto battle data, each with why (`unplayable_in_auto_battle`): the game's
    /// ruleset answers for a chip (`Defs::unplayable_in_auto_battle`), a system out
    /// of the ruleset doesn't, and an id that is no chip of the game is
    /// refused as the content is defined.
    #[test]
    fn a_system_says_the_chips_auto_battle_cant_play() {
        let with = |system: &str, entry: &str| {
            let mut c = testing::build();
            let src = c.scripts.module_mut(testing::ROOT, "rules/systems").expect("the module");
            let from = format!("    id = \"{system}\",");
            assert!(src.contains(&from), "{from}");
            *src = src.replacen(&from, &format!("{from}\n    unplayable_in_auto_battle = {entry},"), 1);
            c.define().map(|_| c).map_err(|e| e.message)
        };
        let chip = |c: &Content, key: &str| c.defs.chip_by_key(key).unwrap_or_else(|| panic!("no chip {key}"));
        let content = with("test/counter", "{ [\"test/veil\"] = \"it has no weight\" }").expect("defined");
        assert_eq!(content.defs.unplayable_in_auto_battle(chip(&content, "test/veil")), Some("it has no weight"));
        assert_eq!(content.defs.unplayable_in_auto_battle(chip(&content, testing::BOMB)), None);
        let stock = scenario::content();
        assert_eq!(stock.defs.unplayable_in_auto_battle(chip(&stock, "test/veil")), None, "no system says any");
        // (The marker isn't one of the stock ruleset's systems.)
        let unused = with("test/marker", "{ [\"test/veil\"] = \"it has no weight\" }").expect("defined");
        assert_eq!(unused.defs.unplayable_in_auto_battle(chip(&unused, "test/veil")), None);
        let e = with("test/counter", "{ [\"test/nothing\"] = \"it isn't\" }").map(|_| ()).expect_err("no such chip");
        assert!(e.contains("`unplayable_in_auto_battle` names test/nothing, which is no chip of the game"), "{e}");
        let e = with("test/counter", "{ \"test/veil\" }").map(|_| ()).expect_err("a list");
        assert!(e.contains("`unplayable_in_auto_battle` is a table of sentences by chip id"), "{e}");
    }

    mod patch_cards {
        use super::*;
        use crate::patch_cards::{InstalledCard, PatchCards};
        use crate::setup::{GaugeSpeed, NaviStats, Supports};

        /// A battle on the test content with EXE6's patch-cards system
        /// (then the counter), side 0 with `cards` installed (key,
        /// switched on), its stats changed by `tweak` first.
        fn with_cards(cards: &[(&str, bool)], tweak: impl FnOnce(&mut NaviStats)) -> Battle {
            let content = testing::with_systems("patch_cards, counter");
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
            assert_eq!(b.stats[1], scenario::setup().navi_stats[1], "the other side has none");
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
            let mut want = scenario::setup().navi_stats[0];
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
            let mut want = scenario::setup().navi_stats[0];
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
