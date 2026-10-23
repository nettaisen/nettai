//! The observation's layout (docs/design/training-ai.md §3): its token
//! sets in a fixed order, each a count of tokens of a width of features
//! (named, in order) and of name ids, and the spec's hash, which a network's
//! weights carry so that an observation changed since they were trained
//! can't play them.
//!
//! Each set's features are listed once here, as `name: width`; the encoder
//! writes them by those names (`global::GAUGE` is the gauge's place in a
//! global token), and `spec()` lists them for the trainer.

use crate::names;

/// A set's features, their places in its token, and its width.
macro_rules! features {
    ($(#[$doc:meta])* $set:ident { $($field:ident: $w:expr),* $(,)? }) => {
        $(#[$doc])*
        #[allow(non_upper_case_globals)]
        pub mod $set {
            /// Each feature's name and width, in order.
            pub const FIELDS: &[(&str, usize)] = &[$((stringify!($field), $w)),*];
            /// The token's width.
            pub const WIDTH: usize = 0 $(+ $w)*;
            features!(@at 0usize; $($field: $w),*);
        }
    };
    (@at $at:expr; $field:ident: $w:expr $(, $rest:ident: $rw:expr)*) => {
        pub const $field: usize = $at;
        features!(@at $at + $w; $($rest: $rw),*);
    };
    (@at $at:expr;) => {};
}

features! {
    /// The one global token: the round's phase, the gauge and turn, both
    /// navis' HP as the viewer's HUD shows them, its own navi's charge and
    /// stats, the telops, the custom screen's state, the reaction delay.
    global {
        phase: 4,
        screen_phase: 4,
        gauge: 2,
        turn: 3,
        battle_time: 1,
        own_hp: 3,
        other_hp: 4,
        own_charge: 7,
        other_charge: 4,
        own_stats: 8,
        chips_left: 3,
        telops: 8,
        used_chip: 2,
        delay: 1,
        probe: 8,
        screen: 6,
        navis: 3,
    }
}

/// How many attack-log ticks a panel token holds (§5): the last ones, the
/// newest first.
pub const LOG_TICKS: usize = 8;

features! {
    /// A panel of the playable 3 × 6, from the viewer's side (its own area
    /// on the left): its place, its type's rule, its owners, its state, what
    /// is on it, and the attacks presented on it in the last ticks.
    panels {
        place: 2,
        rule: 12,
        owner: 3,
        state: 5,
        highlight: 1,
        on_it: 7,
        attacked: 2 * super::LOG_TICKS,
    }
}

/// The fingerprint block of a chip token: what the chip did in a sandbox
/// battle (a later step, §12 step 6: zeros until then).
pub const FINGERPRINT: usize = 40;

features! {
    /// An object the viewer sees.
    objects {
        pool: 3,
        class: 5,
        side: 3,
        player: 2,
        panel: 4,
        offset: 3,
        velocity: 3,
        facing: 1,
        hp: 4,
        damage: 1,
        element: 10,
        collision: 12,
        region: 18,
        status: 21,
        timers: 9,
        barrier: 2,
        actor: 6,
        action: 11,
        anim: 1,
    }
}

features! {
    /// A chip: of its own hand in the fight (next first), on its custom
    /// screen's slots (and the screen's other slots: OK, the rules'
    /// buttons), and the other side's last used chip as its telop showed it.
    chips {
        role: 3,
        hand_place: 1,
        slot: 12,
        facts: 6 + 5 + 27 + 4 + 8 + 3 + 8 + 6 + 5 + 9 + 4 + 3,
        in_hand: 5,
        on_screen: 9 + 3 + 4,
        pointer: 3,
        would_be: 4,
        used: 4,
        fingerprint: super::FINGERPRINT,
    }
}

features! {
    /// A scalar of the viewer's side's rules' state or its navi's stats of
    /// its game's own.
    rules {
        source: 2,
        ty: 6,
        value: 4,
        place: 2,
    }
}

features! {
    /// A form: the viewer's navi's, the other's as it shows, and those a
    /// form window offers.
    forms {
        role: 3,
        window: 4,
        facts: 6 + 4 + 5,
    }
}

/// A token set's layout.
#[derive(Clone, Copy, Debug)]
pub struct Set {
    pub name: &'static str,
    /// Tokens (at most).
    pub count: usize,
    pub width: usize,
    /// Name ids per token, and what each pair is.
    pub ids: usize,
    pub id_names: &'static [&'static str],
    pub fields: &'static [(&'static str, usize)],
}

/// The sets' places in [`SETS`].
pub const GLOBAL: usize = 0;
pub const PANELS: usize = 1;
pub const OBJECTS: usize = 2;
pub const CHIPS: usize = 3;
pub const RULES: usize = 4;
pub const FORMS: usize = 5;

/// Hand tokens, screen slot tokens and the other's used chip, in the chips
/// set in that order.
pub const HAND_TOKENS: usize = 6;
pub const SLOT_TOKENS: usize = nettai_battle::custom::screen::SLOTS;
pub const FIRST_SLOT_TOKEN: usize = HAND_TOKENS;
pub const USED_TOKEN: usize = HAND_TOKENS + SLOT_TOKENS;

/// The sets, in the order the bridge and the trainer keep them.
pub const SETS: [Set; 6] = [
    Set { name: "global", count: 1, width: global::WIDTH, ids: 4, id_names: &["own_navi", "other_navi"], fields: global::FIELDS },
    Set { name: "panels", count: 18, width: panels::WIDTH, ids: 2, id_names: &["type"], fields: panels::FIELDS },
    Set { name: "objects", count: 48, width: objects::WIDTH, ids: 6, id_names: &["kind", "identity", "action"], fields: objects::FIELDS },
    Set { name: "chips", count: HAND_TOKENS + SLOT_TOKENS + 1, width: chips::WIDTH, ids: 2, id_names: &["chip"], fields: chips::FIELDS },
    Set { name: "rules", count: 96, width: rules::WIDTH, ids: 4, id_names: &["field", "value"], fields: rules::FIELDS },
    Set { name: "forms", count: 12, width: forms::WIDTH, ids: 2, id_names: &["form"], fields: forms::FIELDS },
];

/// Joint fight actions: a direction (none, up, down, toward, away) × A ×
/// B (§4.1).
pub const FIGHT_ACTIONS: usize = 20;

/// The custom screen's keys an action may press once, after the 12 slots
/// (§4.2).
pub const SCREEN_KEYS: [&str; 7] = ["up", "down", "left", "right", "a", "b", "start"];

/// Custom screen actions: a slot to point at, or a key.
pub const SCREEN_ACTIONS: usize = SLOT_TOKENS + SCREEN_KEYS.len();

/// The layout as text, which the spec's hash is of: every set's name,
/// count, width, ids and features in order, the actions, and the names'
/// hash.
pub fn text() -> String {
    let mut out = format!("nettai observation\nnames {} buckets {}\nfight actions {FIGHT_ACTIONS}\nscreen actions {SCREEN_ACTIONS} keys {}\n", names::DESCRIPTION, names::BUCKETS, SCREEN_KEYS.join(","));
    out += &format!("chips: hand {HAND_TOKENS} slots {SLOT_TOKENS} used 1\n");
    for s in SETS {
        out += &format!("set {} count {} width {} ids {} ({})\n", s.name, s.count, s.width, s.ids, s.id_names.join(","));
        for (name, w) in s.fields {
            out += &format!("  {name} {w}\n");
        }
    }
    out
}

/// The spec's hash: FNV-1a 64 of [`text`], in hex.
pub fn hash() -> String {
    names::text_hash(&text())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_sets_fields_fill_its_width() {
        for s in SETS {
            assert_eq!(s.fields.iter().map(|(_, w)| w).sum::<usize>(), s.width, "{}", s.name);
            assert_eq!(s.id_names.len() * 2, s.ids, "{}: two ids a name", s.name);
        }
        assert_eq!(global::navis + 3, global::WIDTH);
        assert_eq!(chips::fingerprint + FINGERPRINT, chips::WIDTH);
        assert_eq!(hash().len(), 16);
    }
}
