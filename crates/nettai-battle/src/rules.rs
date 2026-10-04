//! The players' rules (docs/design/rules-in-luau.md): each side plays by a
//! ruleset, a list of systems written in Luau; the framework calls each
//! side's systems at its points (a hook a system fills), and keeps each
//! system's state of each side here, in the battle, where snapshots and the
//! digest cover it.
//!
//! A system reaches only its own state of the side it was called for
//! (`system.state()` in a hook): a side's rules see the other side through
//! the engine alone.

use nettai_content_api::{ChipHandle, ContentState, FieldType, HookCall, ObjectRef, RulesetHandle, SystemHook, Value, WeaponHandle};

use crate::battle::Battle;
use crate::content::Content;
use crate::custom::PlayerSetup;

/// A side's rules in a battle: its ruleset, and each of the ruleset's
/// systems' state of the side, in the ruleset's order.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct SideRules {
    /// None: the content has no ruleset (a side with no systems).
    pub ruleset: Option<RulesetHandle>,
    pub states: Vec<ContentState>,
}

impl SideRules {
    /// A player's rules at a round's start: the match's ruleset (`ruleset`,
    /// the round setup's; none: the game's stock one), its systems' state
    /// zeroed. Their setup's blocks are made the systems' defaults
    /// (`setup_defaults`, the rest zero) for a setup that gives none, and
    /// must otherwise be the ruleset's.
    pub fn for_player(content: &Content, player: &mut PlayerSetup, ruleset: Option<RulesetHandle>) -> SideRules {
        let ruleset = ruleset.or_else(|| content.defs.stock_ruleset());
        let Some(r) = ruleset else {
            assert!(player.rules.is_empty(), "a player's setup gives system setups, and the content has no ruleset");
            return SideRules::default();
        };
        let def = content.defs.ruleset(r);
        let systems: Vec<_> = def.systems.iter().map(|&h| content.defs.system(h)).collect();
        if player.rules.is_empty() {
            player.rules = systems.iter().map(|s| s.setup_block()).collect();
        }
        let fits = player.rules.len() == systems.len() && player.rules.iter().zip(&systems).all(|(b, s)| b.id() == s.setup);
        assert!(fits, "a player's setup gives system setups that aren't ruleset {}'s", def.key);
        SideRules { ruleset, states: systems.iter().map(|s| ContentState::new(s.state)).collect() }
    }
}

impl PlayerSetup {
    /// Set field `field` of system `system`'s setup (by key) to `v`, for
    /// the match's ruleset (`ruleset`; none: `content`'s stock one): how
    /// tools write what a save says.
    pub fn set_rule(&mut self, content: &Content, ruleset: Option<RulesetHandle>, system: &str, field: &str, v: Value) -> Result<(), String> {
        let (block, schema, i) = self.rule_field(content, ruleset, system, field)?;
        block.set(schema, i, v).map_err(|e| format!("system {system}'s setup field `{field}`: {e}"))
    }

    /// [`PlayerSetup::set_rule`] for an array field: element `k` of it.
    pub fn set_rule_elem(
        &mut self,
        content: &Content,
        ruleset: Option<RulesetHandle>,
        system: &str,
        field: &str,
        k: usize,
        v: Value,
    ) -> Result<(), String> {
        let (block, schema, i) = self.rule_field(content, ruleset, system, field)?;
        block.set_elem(schema, i, k, v).map_err(|e| format!("system {system}'s setup field `{field}`: {e}"))
    }

    /// The setup block of system `system` (by key) of the match's ruleset
    /// (`ruleset`; none: the stock one) and its layout, as the round will
    /// start with it (the systems' defaults where the setup gives none):
    /// what a tool shows of it.
    pub fn rule_block<'a>(
        &self,
        content: &'a Content,
        ruleset: Option<RulesetHandle>,
        system: &str,
    ) -> Option<(&'a nettai_content_api::Schema, ContentState)> {
        let r = ruleset.or_else(|| content.defs.stock_ruleset())?;
        let systems = &content.defs.ruleset(r).systems;
        let slot = systems.iter().position(|&h| content.defs.system(h).key == system)?;
        let def = content.defs.system(systems[slot]);
        let block = self.rules.get(slot).copied().unwrap_or_else(|| def.setup_block());
        Some((content.defs.schema(def.setup), block))
    }

    /// Write a fact of what the player brings into each system of the
    /// match's ruleset (`ruleset`; none: the stock one) whose setup has a
    /// field `field`: one value for a field, an element each for an array
    /// (the rest zero), an enum's by its name (`Fact::Name`). How a tool
    /// writes what several systems read (BN6's game version, which its
    /// cross and beast systems both take). The number of systems that took
    /// it: none on a content without a ruleset.
    pub fn set_fact(&mut self, content: &Content, ruleset: Option<RulesetHandle>, field: &str, values: &[Fact]) -> Result<usize, String> {
        let Some(r) = ruleset.or_else(|| content.defs.stock_ruleset()) else { return Ok(0) };
        let def = content.defs.ruleset(r);
        if self.rules.is_empty() {
            self.rules = def.systems.iter().map(|&h| content.defs.system(h).setup_block()).collect();
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

    /// The setup block of system `system` (by key) of the match's ruleset
    /// (`ruleset`; none: the stock one), its schema and the index of its
    /// field `field`; the blocks made the systems' defaults first if the
    /// setup gives none.
    fn rule_field<'a>(
        &'a mut self,
        content: &'a Content,
        ruleset: Option<RulesetHandle>,
        system: &str,
        field: &str,
    ) -> Result<(&'a mut ContentState, &'a nettai_content_api::Schema, usize), String> {
        let r = ruleset.or_else(|| content.defs.stock_ruleset()).ok_or("the content has no ruleset")?;
        let def = content.defs.ruleset(r);
        if self.rules.is_empty() {
            self.rules = def.systems.iter().map(|&h| content.defs.system(h).setup_block()).collect();
        }
        let slot = def
            .systems
            .iter()
            .position(|&h| content.defs.system(h).key == system)
            .ok_or_else(|| format!("ruleset {} has no system {system}", def.key))?;
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

impl Battle {
    /// Side `side`'s system `key`'s setup block (the player's, as the round
    /// started with it) and its layout, when the side's ruleset has that
    /// system: for a reader of what a player brought (the frontend's BN6
    /// look reads the cross system's version and Cross list).
    pub fn system_setup(&self, side: u8, key: &str) -> Option<(&nettai_content_api::Schema, &ContentState)> {
        let r = self.rules[side as usize & 1].ruleset?;
        let systems = &self.content.defs.ruleset(r).systems;
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
        let r = self.rules.get(side as usize)?.ruleset?;
        let slot = self.content.defs.ruleset(r).systems.iter().position(|&h| h == system)?;
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
    /// `folder_check`: BN6's folder rules), each rule it breaks named and
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
        let Some(r) = self.rules[side as usize].ruleset else { return };
        let content = self.content.clone();
        for (slot, &h) in content.defs.ruleset(r).systems.iter().enumerate() {
            if let Some(f) = content.defs.system(h).hook(hook) {
                let call = HookCall::System { side, slot: slot as u8, hook, navi: None, chip: None, weapon: None };
                crate::behavior::call_hook(self, f, call);
            }
        }
    }

    /// Side `side`'s systems' `navi_intake(side, navi)`, each tick of the
    /// fight in the navi's intake. (A ruleset without the hook calls
    /// nothing: BN6's.)
    pub(crate) fn systems_navi_intake(&mut self, side: u8, navi: ObjectRef) {
        let Some(r) = self.rules[side as usize].ruleset else { return };
        let content = self.content.clone();
        for (slot, &h) in content.defs.ruleset(r).systems.iter().enumerate() {
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
        let Some(r) = self.rules[side as usize].ruleset else { return };
        let content = self.content.clone();
        for (slot, &h) in content.defs.ruleset(r).systems.iter().enumerate() {
            if let Some(f) = content.defs.system(h).hook(SystemHook::ChipPrepared) {
                let call = HookCall::System { side, slot: slot as u8, hook: SystemHook::ChipPrepared, navi: Some(navi), chip: Some(chip), weapon: None };
                crate::behavior::call_hook(self, f, call);
            }
        }
    }

    /// Side `side`'s systems' `chip_used(side, navi, chip, weapon)` once a
    /// chip's use has started its action.
    pub(crate) fn systems_chip_used(&mut self, side: u8, navi: ObjectRef, chip: ChipHandle, weapon: Option<WeaponHandle>) {
        let Some(r) = self.rules[side as usize].ruleset else { return };
        let content = self.content.clone();
        for (slot, &h) in content.defs.ruleset(r).systems.iter().enumerate() {
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
        let Some(r) = self.rules[side as usize].ruleset else { return 0 };
        let content = self.content.clone();
        for (slot, &h) in content.defs.ruleset(r).systems.iter().enumerate() {
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
    /// own navi (BN5's no-charge drive): the outcome the first system that
    /// answers gives; None when none answers.
    pub(crate) fn systems_controller_answer(&mut self, side: u8, navi: ObjectRef) -> Option<u8> {
        let r = self.rules[side as usize].ruleset?;
        let content = self.content.clone();
        for (slot, &h) in content.defs.ruleset(r).systems.iter().enumerate() {
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
        let Some(r) = self.rules[side as usize & 1].ruleset else { return };
        let content = self.content.clone();
        let Some(&h) = content.defs.ruleset(r).systems.get(slot as usize) else { return };
        if let Some(f) = content.defs.system(h).hook(SystemHook::Controller) {
            let call = HookCall::System { side, slot, hook: SystemHook::Controller, navi: Some(navi), chip: None, weapon: None };
            crate::behavior::call_hook(self, f, call);
        }
    }

    /// Side `side`'s systems' `takeover_requested(side, navi)`.
    pub(crate) fn systems_takeover_requested(&mut self, side: u8, navi: ObjectRef) {
        let Some(r) = self.rules[side as usize].ruleset else { return };
        let content = self.content.clone();
        for (slot, &h) in content.defs.ruleset(r).systems.iter().enumerate() {
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
        let Some(r) = self.rules[side as usize].ruleset else { return 0 };
        let content = self.content.clone();
        for (slot, &h) in content.defs.ruleset(r).systems.iter().enumerate() {
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
    pub(crate) fn side_buttons(&self, side: u8) -> Vec<crate::content::ButtonHandle> {
        let Some(r) = self.rules[side as usize & 1].ruleset else { return Vec::new() };
        self.content.defs.ruleset(r).systems.iter().flat_map(|&h| self.content.defs.system(h).buttons.iter().copied()).collect()
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
        let Some(r) = self.rules[side as usize & 1].ruleset else { return };
        let content = self.content.clone();
        for (slot, &h) in content.defs.ruleset(r).systems.iter().enumerate() {
            if let Some(f) = content.defs.system(h).hook(hook) {
                let call = HookCall::System { side, slot: slot as u8, hook, navi: None, chip: Some(chip), weapon: None };
                crate::behavior::call_hook(self, f, call);
            }
        }
    }

    /// Side `side`'s systems' custom hook `hook(side)`, each in order.
    pub(crate) fn systems_call_custom(&mut self, side: u8, hook: SystemHook) {
        let Some(r) = self.rules[side as usize & 1].ruleset else { return };
        let content = self.content.clone();
        for (slot, &h) in content.defs.ruleset(r).systems.iter().enumerate() {
            if let Some(f) = content.defs.system(h).hook(hook) {
                let call = HookCall::System { side, slot: slot as u8, hook, navi: None, chip: None, weapon: None };
                crate::behavior::call_hook(self, f, call);
            }
        }
    }

    /// Side `side`'s systems' custom hook `hook(side)` in order, until one
    /// answers true: whether one did.
    pub(crate) fn systems_ask_custom(&mut self, side: u8, hook: SystemHook) -> bool {
        let Some(r) = self.rules[side as usize & 1].ruleset else { return false };
        let content = self.content.clone();
        for (slot, &h) in content.defs.ruleset(r).systems.iter().enumerate() {
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
    /// frontend's look of BN6's Cross window reads the cross system's).
    pub fn system_state(&self, side: u8, key: &str) -> Option<(&nettai_content_api::Schema, &ContentState)> {
        let r = self.rules[side as usize & 1].ruleset?;
        let systems = &self.content.defs.ruleset(r).systems;
        let slot = systems.iter().position(|&h| self.content.defs.system(h).key == key)?;
        let def = self.content.defs.system(systems[slot]);
        Some((self.content.defs.schema(def.state), self.rules[side as usize & 1].states.get(slot)?))
    }

    /// Side `side`'s systems' `custom.hand_size(side)`: the first answer.
    pub(crate) fn systems_custom_hand_size(&mut self, side: u8) -> Option<u8> {
        let r = self.rules[side as usize & 1].ruleset?;
        let content = self.content.clone();
        for (slot, &h) in content.defs.ruleset(r).systems.iter().enumerate() {
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
        let Some(r) = self.rules[side as usize & 1].ruleset else { return false };
        let content = self.content.clone();
        for (slot, &h) in content.defs.ruleset(r).systems.iter().enumerate() {
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
        let r = self.rules[side as usize & 1].ruleset?;
        let content = self.content.clone();
        for (slot, &h) in content.defs.ruleset(r).systems.iter().enumerate() {
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
        let Some(r) = self.rules[side as usize & 1].ruleset else { return };
        let content = self.content.clone();
        for (slot, &h) in content.defs.ruleset(r).systems.iter().enumerate() {
            if let Some(f) = content.defs.system(h).hook(hook) {
                let call = HookCall::System { side, slot: slot as u8, hook, navi: Some(navi), chip: None, weapon: None };
                crate::behavior::call_hook(self, f, call);
            }
        }
    }

    /// Side `side`'s systems' `form_reverted(side, navi)`.
    pub(crate) fn systems_form_reverted(&mut self, side: u8, navi: ObjectRef) {
        let Some(r) = self.rules[side as usize].ruleset else { return };
        let content = self.content.clone();
        for (slot, &h) in content.defs.ruleset(r).systems.iter().enumerate() {
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
    /// puts in its place (BN6's dark chips' substitute), or none.
    pub(crate) fn systems_chip_substitute(&mut self, side: u8, navi: ObjectRef, chip: ChipHandle) -> Option<ChipHandle> {
        self.systems_chip_answer(side, navi, Some(chip), SystemHook::ChipSubstitute)
    }

    /// A chip hook `hook(side, navi, chip)`'s first answer, a chip.
    fn systems_chip_answer(&mut self, side: u8, navi: ObjectRef, chip: Option<ChipHandle>, hook: SystemHook) -> Option<ChipHandle> {
        let r = self.rules[side as usize].ruleset?;
        let content = self.content.clone();
        for (slot, &h) in content.defs.ruleset(r).systems.iter().enumerate() {
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

/// A folder rule broken: the rule's name (the game's own: BN6's `chip`,
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
        let mut b = Battle::new(setup, scenario::content());
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

    #[test]
    fn each_side_runs_its_rulesets_systems_for_itself() {
        let b = started(scenario::setup());
        let content = &b.content;
        let stock = content.defs.stock_ruleset().expect("the test content's stock rules");
        assert_eq!(content.defs.ruleset(stock).key, "test:stock");
        for side in 0..2u8 {
            assert_eq!(b.side_rules(side).ruleset, Some(stock));
            // (BN6's beast system first, then the counter, then BN6's forms
            // and emotion systems.)
            assert_eq!(b.side_rules(side).states.len(), 4);
            assert_eq!(field(&b, side, 1, "starts"), FieldValue::U8(1), "round_start ran once for side {side}");
            assert_eq!(field(&b, side, 1, "side"), FieldValue::U8(side), "it ran for its own side");
        }
    }

    /// One ruleset a match (docs/design/content-model-v2.md §4.0: the arena
    /// configuration determines everything): both sides play by the one
    /// the setup names, each with its own state of its systems.
    #[test]
    fn a_match_plays_by_the_ruleset_its_setup_names() {
        let content = scenario::content();
        let mut setup = scenario::setup();
        setup.ruleset = content.defs.ruleset_by_key("test:test-other");
        let b = started(setup);
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
        setup.players[0].set_rule(&content, None, "test:test/counter", "bonus", Value::Int(7)).unwrap();
        assert!(setup.players[0].set_rule(&content, None, "test:test/marker", "mark", Value::Int(1)).is_err(), "not the stock rules'");
        let b = started(setup);
        assert_eq!(field(&b, 0, 1, "bonus"), FieldValue::U16(14));
        assert_eq!(field(&b, 1, 1, "bonus"), FieldValue::U16(0), "the other player's setup is its own");
    }

    /// The per-tick and chip-use hooks (docs/design/bn5-map.md §15.3 item
    /// 14): a system's `navi_intake` is called with the side and its navi,
    /// and its `chip_check` with the chip about to be used, whose answer
    /// takes the chip's place; a side whose rules lack them calls nothing.
    #[test]
    fn a_systems_intake_and_chip_check_hooks() {
        let content = scenario::content();
        let watch = content.defs.ruleset_by_key("test:test-watch").expect("the watcher's ruleset");
        let mut setup = scenario::setup();
        setup.ruleset = Some(watch);
        let mut b = started(setup);
        let navi = b.player(1).expect("side 1's navi");
        b.systems_navi_intake(1, navi);
        b.systems_navi_intake(1, navi);
        let p = b.objects.get(navi).panel;
        assert_eq!(field(&b, 1, 0, "intakes"), FieldValue::U16(2));
        assert_eq!((field(&b, 1, 0, "x"), field(&b, 1, 0, "y")), (FieldValue::U8(p.x), FieldValue::U8(p.y)));
        let bomb = testing::chip_in(&content, "test:test/bomb");
        let seed = testing::chip_in(&content, "test:test/seed");
        assert_eq!(b.systems_chip_check(1, navi, Some(bomb)), Some(seed));
        assert_eq!(b.systems_chip_check(1, navi, Some(seed)), None);
        assert_eq!(b.systems_chip_check(1, navi, None), None);
        // The stock rules have neither hook.
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
        assert_eq!(testing::build().defs.rulesets.len(), 5);
    }

    /// A variant (testdata's rules/mix.luau): the stock rules less BN6's
    /// forms system, the marker after them.
    #[test]
    fn a_mix_is_its_bases_systems_changed() {
        let content = scenario::content();
        let defs = &content.defs;
        let mix = defs.ruleset_by_key("test:test-mix").expect("the mix");
        let names: Vec<&str> = defs.ruleset(mix).systems.iter().map(|&h| defs.system(h).key.as_str()).collect();
        assert_eq!(names, ["test:beast", "test:test/counter", "test:emotion", "test:test/marker"]);
        assert_eq!(defs.ruleset(mix).base, defs.stock_ruleset());
        let mut setup = scenario::setup();
        setup.ruleset = Some(mix);
        let b = started(setup);
        assert_eq!(b.side_rules(1).states.len(), 4);
        assert_eq!(field(&b, 1, 3, "mark"), FieldValue::U8(0x41), "the marker ran for its side");
        assert_eq!(field(&b, 1, 1, "starts"), FieldValue::U8(1));
    }

    /// BN6's patch-cards system (content/bn6/rules/patch-cards) with the
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
        let veil = "test:test/veil";
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
        let chips = "chips/test/chips";
        refused(chips, "test_weight = 3,", "test_weight = 300,", "chip test:test/veil.test_weight is Int(300), not u8");
        refused(chips, "kind = \"b\" }", "kind = \"c\" }", "chip test:test/veil.test_tag.kind");
        refused(chips, "kind = \"b\" }", "kind = \"b\", hue = 1 }", "`hue` is none of its fields (kind, level)");
        let systems = "rules/systems";
        refused(systems, "            test_weight = \"u8\",", "            test_weight = \"u9\",", "no type is named \"u9\"");
        refused(systems, "        chip = {\n            test_weight", "        stage = {},\n        chip = {\n            test_weight", "a system extends chip, form or navi");
        refused(
            systems,
            "    id = \"test:test/marker\",",
            "    id = \"test:test/marker\",\n    extends = { chip = { test_weight = \"u8\" } },",
            "both extend chip definitions with `test_weight`",
        );
    }

    mod patch_cards {
        use super::*;
        use crate::patch_cards::{InstalledCard, PatchCards};
        use crate::setup::{GaugeSpeed, NaviStats, Supports};

        /// A battle whose side 0 plays by the test-cards ruleset with
        /// `cards` installed (key, switched on), its stats changed by
        /// `tweak` first.
        fn with_cards(cards: &[(&str, bool)], tweak: impl FnOnce(&mut NaviStats)) -> Battle {
            let content = scenario::content();
            let mut s = scenario::setup();
            s.ruleset = content.defs.ruleset_by_key("test:test-cards");
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
            let h = content.defs.patch_card_by_key("test:test-stats").expect("the test card");
            let card = content.patch_card(h);
            assert_eq!(card.mb, 20);
            let kinds: Vec<(&str, bool)> = card.effects.iter().map(|e| (e.kind.as_str(), e.bug)).collect();
            assert_eq!(kinds, [("hp_add", false), ("hp_percent_add", false), ("attack_add", false), ("body", false), ("hp_drain", true)]);
            // The cards are in the setup, which the digest covers.
            let a = with_cards(&[("test:test-stats", true)], |_| {});
            let b = with_cards(&[("test:test-stats", false)], |_| {});
            assert_ne!(a.setup.players[0].patch_cards, b.setup.players[0].patch_cards);
            assert_ne!(a.digest(), b.digest());
        }

        #[test]
        fn a_card_changes_the_stats_by_its_kinds_order() {
            let b = with_cards(&[("test:test-stats", true)], |_| {});
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
            let b = with_cards(&[("test:test-stats", true), ("test:test-later", true)], |_| {});
            let s = &b.stats[0];
            assert_eq!(s.attack, 2, "Attack 0 + 3 - 1");
            assert_eq!(s.giga_level, 0xFF, "GigaFolder- doesn't clamp");
        }

        #[test]
        fn abilities_choices_and_chip_shuffle() {
            let b = with_cards(&[("test:test-abilities", true)], |s| {
                s.support = Some(Supports::default());
                s.float_shoes = true;
                s.number_open = true;
            });
            let s = &b.stats[0];
            let content = &b.content;
            assert!(s.super_armor && !s.float_shoes);
            assert_eq!(s.first_barrier, content.defs.record("test:barrier/200"));
            assert_eq!(s.weapons.charge_shot_kind, content.defs.record("test:shot/charged-confusing"));
            assert_eq!(s.support, Some(Supports { rush: true, ..Supports::default() }));
            assert_eq!(s.gauge_speed, GaugeSpeed::Fast);
            assert!(s.chip_shuffle && !s.number_open, "ChpShufl turns NumbrOpn off");
            assert!(!b.consoles[0].emotion_window_glitch, "no bug");
        }

        #[test]
        fn a_switched_off_card_does_nothing_but_the_glitch_follows_the_stats() {
            let b = with_cards(&[("test:test-stats", false)], |s| s.support = Some(Supports::default()));
            let mut want = scenario::setup().navi_stats[0];
            want.support = Some(Supports::default());
            // The HP is set to its maximum (the reload's, in the real world).
            want.hp = want.max_hp;
            assert_eq!(b.stats[0], want);
            assert!(!b.consoles[0].emotion_window_glitch);
            let bugged = with_cards(&[("test:test-stats", false)], |s| {
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
            let b = with_cards(&[("test:test-abilities", true)], |s| s.support = None);
            assert_eq!(b.stats[0].support, None, "the byte 0xFF stays 0xFF when a bit is set");
        }
    }
}
