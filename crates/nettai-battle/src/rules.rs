//! The players' rules (docs/design/rules-in-luau.md): each side plays by a
//! ruleset, a list of systems written in Luau; the framework calls each
//! side's systems at its points (a hook a system fills), and keeps each
//! system's state of each side here, in the battle, where snapshots and the
//! digest cover it.
//!
//! A system reaches only its own state of the side it was called for
//! (`system.state()` in a hook): a side's rules see the other side through
//! the engine alone.

use nettai_content_api::{ContentState, HookCall, RulesetHandle, SystemHook, Value};

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
    /// A player's rules at a round's start: the ruleset their setup names
    /// (or the content's stock one), its systems' state zeroed. Their
    /// setup's blocks are made zero for a setup that gives none, and must
    /// otherwise be the ruleset's.
    pub fn for_player(content: &Content, player: &mut PlayerSetup) -> SideRules {
        let ruleset = player.ruleset.or_else(|| content.defs.stock_ruleset());
        let Some(r) = ruleset else {
            assert!(player.rules.is_empty(), "a player's setup gives system setups, and the content has no ruleset");
            return SideRules::default();
        };
        let def = content.defs.ruleset(r);
        let systems: Vec<_> = def.systems.iter().map(|&h| content.defs.system(h)).collect();
        if player.rules.is_empty() {
            player.rules = systems.iter().map(|s| ContentState::new(s.setup)).collect();
        }
        let fits = player.rules.len() == systems.len() && player.rules.iter().zip(&systems).all(|(b, s)| b.id() == s.setup);
        assert!(fits, "a player's setup gives system setups that aren't ruleset {}'s", def.key);
        SideRules { ruleset, states: systems.iter().map(|s| ContentState::new(s.state)).collect() }
    }
}

impl PlayerSetup {
    /// Set field `field` of system `system`'s setup (by key) to `v`, for
    /// the player's ruleset (their setup's, or `content`'s stock one): how
    /// tools write what a save says.
    pub fn set_rule(&mut self, content: &Content, system: &str, field: &str, v: Value) -> Result<(), String> {
        let r = self.ruleset.or_else(|| content.defs.stock_ruleset()).ok_or("the content has no ruleset")?;
        let def = content.defs.ruleset(r);
        if self.rules.is_empty() {
            self.rules = def.systems.iter().map(|&h| ContentState::new(content.defs.system(h).setup)).collect();
        }
        let slot = def
            .systems
            .iter()
            .position(|&h| content.defs.system(h).key == system)
            .ok_or_else(|| format!("ruleset {} has no system {system}", def.key))?;
        let block = &mut self.rules[slot];
        let schema = &content.defs.schemas[block.id().0 as usize].schema;
        let i = schema.index_of(field).ok_or_else(|| format!("system {system}'s setup has no field `{field}`"))?;
        block.set(schema, i, v).map_err(|e| format!("system {system}'s setup field `{field}`: {e}"))
    }
}

impl Battle {
    /// Side `side`'s rules.
    pub fn side_rules(&self, side: u8) -> &SideRules {
        &self.rules[side as usize]
    }

    /// Call `hook` of each system of side 0's ruleset that has one, in the
    /// ruleset's order, then side 1's (the original's order wherever it
    /// loops over the sides).
    pub(crate) fn notify_systems(&mut self, hook: SystemHook) {
        for side in 0..2u8 {
            self.notify_side(side, hook);
        }
    }

    /// Call `hook` of each system of side `side`'s ruleset that has one.
    pub(crate) fn notify_side(&mut self, side: u8, hook: SystemHook) {
        let Some(r) = self.rules[side as usize].ruleset else { return };
        let content = self.content.clone();
        for (slot, &h) in content.defs.ruleset(r).systems.iter().enumerate() {
            if let Some(f) = content.defs.system(h).hook(hook) {
                crate::behavior::call_hook(self, f, HookCall::System { side, slot: slot as u8, hook });
            }
        }
    }
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
        assert_eq!(content.defs.ruleset(stock).key, "test");
        for side in 0..2u8 {
            assert_eq!(b.side_rules(side).ruleset, Some(stock));
            // (BN6's beast system first, then the counter.)
            assert_eq!(b.side_rules(side).states.len(), 2);
            assert_eq!(field(&b, side, 1, "starts"), FieldValue::U8(1), "round_start ran once for side {side}");
            assert_eq!(field(&b, side, 1, "side"), FieldValue::U8(side), "it ran for its own side");
        }
    }

    #[test]
    fn a_player_plays_by_the_ruleset_their_setup_names() {
        let content = scenario::content();
        let mut setup = scenario::setup();
        setup.players[1].ruleset = content.defs.ruleset_by_key("test-other");
        let b = started(setup);
        assert_eq!(b.side_rules(0).states.len(), 2, "side 0 keeps the stock rules");
        assert_eq!(b.side_rules(1).states.len(), 2, "side 1 plays by its own");
        assert_eq!(field(&b, 1, 0, "mark"), FieldValue::U8(0x41));
        assert_eq!(field(&b, 1, 1, "side"), FieldValue::U8(1));
        assert_eq!(field(&b, 0, 1, "side"), FieldValue::U8(0));
    }

    #[test]
    fn a_systems_player_setup_reaches_it_and_no_other() {
        let content = scenario::content();
        let mut setup = scenario::setup();
        setup.players[0].set_rule(&content, "test/counter", "bonus", Value::Int(7)).unwrap();
        assert!(setup.players[0].set_rule(&content, "test/marker", "mark", Value::Int(1)).is_err(), "not the stock rules'");
        let b = started(setup);
        assert_eq!(field(&b, 0, 1, "bonus"), FieldValue::U16(14));
        assert_eq!(field(&b, 1, 1, "bonus"), FieldValue::U16(0), "the other player's setup is its own");
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
        assert_eq!(testing::build().defs.rulesets.len(), 2);
    }
}
