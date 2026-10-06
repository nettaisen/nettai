//! What the content gives each side for the round: its definitions'
//! functions of the side (`HookCall::Given`), which the round's setup asks
//! once for each side, after the side's rules' `round_setup`. A game's
//! level is its own fact (the rules' `level`), and what a level gives
//! is its rules' to say: EXE6's link navis' chip bonus, charged chips and
//! Fire charge by their navi code's level (content/exe6/rules/by_level.luau),
//! EXE5's team navis' own chips' damage by their story level
//! (content/exe5/lib/navi_level.luau). The engine knows no level: it reads
//! these all round, and so does what draws the battle (a chip's damage on
//! the custom screen, the next chip's bonus), so what is shown is what the
//! battle uses, with no content run to draw it.

use std::sync::Arc;

use nettai_content_api::{ChipHandle, FnId, HookCall, Value};

use crate::battle::Battle;
use crate::content::DamageFormula;

/// What the content gives each side for the round.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Given {
    /// What each side's navi is given (its definition's functions,
    /// `NaviDef::given`).
    pub navis: [NaviGiven; 2],
    /// The chips whose damage a function of the side gives
    /// (`DamageFormula::Given`), each with its damage on each side.
    pub damage: Arc<[(ChipHandle, [u16; 2])]>,
}

/// What a side's navi is given for the round.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NaviGiven {
    /// Its bonus on its family's chips (`chip_bonus.damage`; 0 without
    /// one, or with nil).
    pub chip_bonus: u16,
    /// Whether its `charged_chips` charge (`charged_chips.when`; without
    /// it, they do).
    pub charges: bool,
    /// How far its A charge builds up the next Fire chip's damage
    /// (`fire_charge`; none: it doesn't).
    pub fire_charge: Option<u16>,
}

impl Default for NaviGiven {
    fn default() -> NaviGiven {
        NaviGiven { chip_bonus: 0, charges: true, fire_charge: None }
    }
}

impl Given {
    /// Ask the content what each side is given: each side's navi's
    /// functions, then each chip's damage on each side.
    pub fn ask(b: &mut Battle) -> Given {
        let content = b.content.clone();
        let navis = [0u8, 1].map(|side| {
            let navi = b.stats[side as usize].navi;
            let def = content.defs.navi(navi);
            let what = |field: &str| format!("navi {}'s {field} for side {side}", def.key);
            let mut given = NaviGiven::default();
            if let Some(f) = def.given.chip_bonus {
                given.chip_bonus = number(ask(b, f, side, None), &what("chip_bonus.damage")).unwrap_or(0);
            }
            if let Some(f) = def.given.charges {
                given.charges = match ask(b, f, side, None) {
                    Value::Bool(on) => on,
                    other => panic!("{} is {other:?}, not a flag", what("charged_chips.when")),
                };
            }
            if let Some(f) = def.given.fire_charge {
                given.fire_charge = number(ask(b, f, side, None), &what("fire_charge"));
            }
            given
        });
        let damage = content
            .defs
            .formula_chips
            .iter()
            .filter_map(|&chip| match content.chip(chip).formula {
                Some(DamageFormula::Given(f)) => Some((chip, f)),
                _ => None,
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|(chip, f)| {
                let mut on = |side: u8| {
                    let what = format!("chip {}'s damage for side {side}", content.defs.chip(chip).key);
                    number(ask(b, f, side, Some(chip)), &what).unwrap_or_else(|| panic!("{what} is nil, not a damage"))
                };
                (chip, [on(0), on(1)])
            })
            .collect();
        Given { navis, damage }
    }

    /// `chip`'s damage on `side`, if a function of the side gives it.
    pub fn damage(&self, chip: ChipHandle, side: u8) -> Option<u16> {
        self.damage.iter().find(|(c, _)| *c == chip).map(|(_, d)| d[side as usize & 1])
    }
}

fn ask(b: &mut Battle, f: FnId, side: u8, chip: Option<ChipHandle>) -> Value {
    crate::behavior::call_hook(b, f, HookCall::Given { side, chip })
}

/// A function's number, up to 0xFFFF; none for nil.
fn number(v: Value, what: &str) -> Option<u16> {
    match v {
        Value::Nil => None,
        Value::Int(n) => Some(u16::try_from(n).unwrap_or_else(|_| panic!("{what} is {n}, past 0 to 65535"))),
        other => panic!("{what} is {other:?}, not a number"),
    }
}
