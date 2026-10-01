//! What `bn6-content check` reports about the definitions once the define
//! phase has run (docs/design/content-model-v2.md §7.7): the roles content
//! hasn't filled, kinds under `objects/` that one owner alone uses
//! (colocation, §4), and collision types defined twice (two definitions of
//! one row of the original's table). Duplicate keys, references to the
//! wrong registry, unknown asset names and chips without exactly one use
//! are the define phase's own errors; two keys with one of the original's
//! numbers is compat's (`bn6_compat::Compat` refuses to read it).

use std::collections::{BTreeMap, BTreeSet};

use bn6_battle::Content;
use bn6_content_api::{Data, Registry};

use crate::report::Report;

/// Report on `c`'s definitions.
pub fn definitions(c: &Content, r: &mut Report) {
    let defs = &c.defs;
    if !defs.definitions.is_empty() {
        use bn6_battle::content::{ActionRole, KindRole};
        for role in ActionRole::ALL {
            if !defs.roles.actions.contains_key(&role) {
                r.warn("rules/roles.luau", format!("the role actions.{} is not filled", role.name()));
            }
        }
        for role in KindRole::ALL {
            if !defs.roles.kinds.contains_key(&role) {
                r.warn("rules/roles.luau", format!("the role kinds.{} is not filled", role.name()));
            }
        }
        // The roles that name a definition of their registry, or an asset.
        use bn6_battle::content::{
            BannerRole, CollisionRole, EffectRole, LockonRole, MusicRole, RegionRole, SoundRole, SparkRole, SpriteRole,
            StatusRole,
        };
        let roles = &defs.roles;
        let mut unfilled = |group: &str, name: &str, filled: bool| {
            if !filled {
                r.warn("rules/roles.luau", format!("the role {group}.{name} is not filled"));
            }
        };
        for role in LockonRole::ALL {
            unfilled("lockon", role.name(), roles.lockons.contains_key(&role));
        }
        for role in StatusRole::ALL {
            unfilled("statuses", role.name(), roles.statuses.contains_key(&role));
        }
        for &role in EffectRole::ALL {
            unfilled("effects", role.name(), roles.effects.contains_key(&role));
        }
        for &role in SparkRole::ALL {
            unfilled("sparks", role.name(), roles.sparks.contains_key(&role));
        }
        for &role in RegionRole::ALL {
            unfilled("regions", role.name(), roles.regions.contains_key(&role));
        }
        for &role in CollisionRole::ALL {
            unfilled("collision", role.name(), roles.collisions.contains_key(&role));
        }
        for &role in SoundRole::ALL {
            unfilled("sounds", role.name(), roles.sounds.contains_key(&role));
        }
        for &role in MusicRole::ALL {
            unfilled("music", role.name(), roles.music.contains_key(&role));
        }
        for &role in BannerRole::ALL {
            unfilled("banners", role.name(), roles.banners.contains_key(&role));
        }
        for &role in SpriteRole::ALL {
            unfilled("sprites", role.name(), roles.sprites.contains_key(&role));
        }
    }
    for (row, twins) in duplicate_collision_types(c) {
        let (first, rest) = twins.split_first().expect("two or more");
        let others: Vec<String> = rest.iter().map(|(key, module)| format!("{key} ({module}.luau)")).collect();
        r.error(
            format!("{}.luau", first.1),
            format!(
                "collision type {} is row {row:#04x} of the original's table, and so is {}: define a type once and share it",
                first.0,
                others.join(", ")
            ),
        );
    }
    for (kind, owners) in single_owner_kinds(c) {
        r.warn(
            format!("{kind}"),
            format!("only {owners} uses this kind: move it into {owners}'s folder (docs/design/content-model-v2.md §4)"),
        );
    }
}

/// Collision types that are one row of the original's table (their
/// `row_offset`, the row times 8): the row, and each definition's key and
/// module, for the rows two or more define.
pub fn duplicate_collision_types(c: &Content) -> Vec<(u8, Vec<(String, String)>)> {
    let mut rows: BTreeMap<i64, Vec<(String, String)>> = BTreeMap::new();
    for d in c.defs.definitions.of(Registry::Collision) {
        if let Some(offset) = d.spec.field("row_offset").int() {
            rows.entry(offset).or_default().push((d.key.clone(), d.module.clone()));
        }
    }
    rows.into_iter().filter(|(_, twins)| twins.len() > 1).map(|(offset, twins)| ((offset / 8) as u8, twins)).collect()
}

/// Kinds defined under `objects/` whose every referring definition lives in
/// one owner folder (`chips/minibomb`), with that folder.
pub fn single_owner_kinds(c: &Content) -> Vec<(String, String)> {
    fn refs(d: &Data, out: &mut BTreeSet<String>) {
        match d {
            Data::Ref(Registry::Kind, key) => {
                out.insert(key.clone());
            }
            Data::List(items) => items.iter().for_each(|i| refs(i, out)),
            Data::Map(entries) => entries.iter().for_each(|(_, v)| refs(v, out)),
            _ => {}
        }
    }
    /// A module's owner folder: `chips/minibomb` for `chips/minibomb/chip`.
    fn owner(module: &str) -> String {
        module.split('/').take(2).collect::<Vec<_>>().join("/")
    }
    let defs = &c.defs.definitions;
    let mut users: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for d in &defs.defs {
        let mut seen = BTreeSet::new();
        refs(&d.spec, &mut seen);
        for kind in seen {
            users.entry(kind).or_default().insert(owner(&d.module));
        }
    }
    let mut out = Vec::new();
    for d in defs.of(Registry::Kind) {
        if !d.module.starts_with("objects/") {
            continue;
        }
        if let Some(owners) = users.get(&d.key)
            && let [only] = owners.iter().collect::<Vec<_>>()[..]
            && *only != owner(&d.module)
        {
            out.push((format!("{}.luau", d.module), only.clone()));
        }
    }
    out
}
