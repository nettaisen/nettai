//! Registries, keys and handles (docs/design/content-model-v2.md §2).
//!
//! Every definition content makes belongs to a registry and has a key, a
//! string unique within it: an explicit `id` (`"minibomb"`), or one derived
//! from where the definition sits (`"minibomb/action"`). When content loads,
//! each registry's keys are sorted byte-wise and numbered from 0; that number
//! is the definition's handle, which battle state holds. The order depends
//! only on the keys, so two machines loading the same content get the same
//! handles.

use std::fmt;

/// A registry of definitions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Registry {
    Chip,
    Navi,
    Form,
    Weapon,
    Kind,
    Action,
    Stage,
    Effect,
    Spark,
    Region,
    Collision,
    Status,
    Lockon,
    /// Data only content reads (a bomb variant, a projectile variant): the
    /// engine keeps its handle and its type name.
    Record,
    /// Content state layouts: one per distinct `state` table.
    Schema,
}

impl Registry {
    pub const ALL: [Registry; 15] = [
        Registry::Chip,
        Registry::Navi,
        Registry::Form,
        Registry::Weapon,
        Registry::Kind,
        Registry::Action,
        Registry::Stage,
        Registry::Effect,
        Registry::Spark,
        Registry::Region,
        Registry::Collision,
        Registry::Status,
        Registry::Lockon,
        Registry::Record,
        Registry::Schema,
    ];

    /// The registries content defines with `define.<name>` (schemas come
    /// from the `state` tables of kinds, actions and modules).
    pub const DEFINED: [Registry; 14] = [
        Registry::Chip,
        Registry::Navi,
        Registry::Form,
        Registry::Weapon,
        Registry::Kind,
        Registry::Action,
        Registry::Stage,
        Registry::Effect,
        Registry::Spark,
        Registry::Region,
        Registry::Collision,
        Registry::Status,
        Registry::Lockon,
        Registry::Record,
    ];

    /// The registry's name: its definer's (`define.chip`), and how messages
    /// and the canonical tree name it.
    pub fn name(self) -> &'static str {
        match self {
            Registry::Chip => "chip",
            Registry::Navi => "navi",
            Registry::Form => "form",
            Registry::Weapon => "weapon",
            Registry::Kind => "kind",
            Registry::Action => "action",
            Registry::Stage => "stage",
            Registry::Effect => "effect",
            Registry::Spark => "spark",
            Registry::Region => "region",
            Registry::Collision => "collision",
            Registry::Status => "status",
            Registry::Lockon => "lockon",
            Registry::Record => "record",
            Registry::Schema => "schema",
        }
    }

    pub fn from_name(name: &str) -> Option<Registry> {
        Registry::ALL.into_iter().find(|r| r.name() == name)
    }

    /// Definitions of this registry are named from outside content
    /// (setups, compat, tools), so each needs an explicit `id`.
    pub fn keyed(self) -> bool {
        matches!(
            self,
            Registry::Chip
                | Registry::Navi
                | Registry::Form
                | Registry::Weapon
                | Registry::Kind
                | Registry::Stage
                | Registry::Collision
                | Registry::Status
                | Registry::Lockon
        )
    }
}

impl fmt::Display for Registry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Whether `key` is a valid explicit key: lowercase ASCII letters and digits
/// in `-`-separated words, optionally qualified by owners with `/`
/// (`minibomb`, `atk-10`, `eraseman/mark`).
pub fn valid_key(key: &str) -> bool {
    !key.is_empty()
        && key.split('/').all(|segment| {
            !segment.is_empty()
                && segment.split('-').all(|w| !w.is_empty() && w.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()))
        })
}

macro_rules! handles {
    ($($(#[$doc:meta])* $name:ident => $registry:ident,)*) => {
        $(
            $(#[$doc])*
            #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
            pub struct $name(pub u16);

            impl $name {
                pub const REGISTRY: Registry = Registry::$registry;

                /// The handle as an index into its registry's table.
                pub fn index(self) -> usize {
                    self.0 as usize
                }
            }
        )*
    };
}

handles! {
    /// A chip.
    ChipHandle => Chip,
    /// A navi.
    NaviHandle => Navi,
    /// One of MegaMan's forms.
    FormHandle => Form,
    /// A weapon: what a button's weapon does.
    WeaponHandle => Weapon,
    /// An object kind (the engine's own and content's).
    KindHandle => Kind,
    /// A navi action content implements.
    ActionHandle => Action,
    /// A stage.
    StageHandle => Stage,
    /// A one-shot effect's look.
    EffectHandle => Effect,
    /// A hit spark's look.
    SparkHandle => Spark,
    /// A hit region.
    RegionHandle => Region,
    /// A collision type.
    CollisionHandle => Collision,
    /// A status effect.
    StatusHandle => Status,
    /// A Beast Out lock-on mode.
    LockonHandle => Lockon,
    /// A record only content reads.
    RecordHandle => Record,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_kebab_words_qualified_by_owners() {
        for ok in ["minibomb", "atk-10", "m-cannon", "eraseman/mark", "megaman/buster", "grndman-ex", "a1/b2-c3"] {
            assert!(valid_key(ok), "{ok}");
        }
        for bad in ["", "MiniBomb", "mini_bomb", "-a", "a-", "a--b", "a//b", "/a", "a/", "a b", "a:b", "a#1"] {
            assert!(!valid_key(bad), "{bad}");
        }
    }

    #[test]
    fn registries_round_trip_their_names() {
        for r in Registry::ALL {
            assert_eq!(Registry::from_name(r.name()), Some(r));
        }
    }
}
