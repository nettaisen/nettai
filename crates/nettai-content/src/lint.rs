//! What `nettai-content check` reports about the definitions once the define
//! phase has run (docs/design/content-model-v2.md §7.7): the roles content
//! hasn't filled, kinds under `objects/` that one owner alone uses
//! (colocation, §4), and collision types defined twice (two definitions of
//! one row of the original's table), and the collision types that test
//! BN6's 0x80 self bit which a module using another root's modules names.
//! Duplicate keys, references to the
//! wrong registry, unknown asset names and chips without exactly one use
//! are the define phase's own errors; two keys with one of the original's
//! numbers is compat's (`bn6_compat::Compat` refuses to read it).

use std::collections::{BTreeMap, BTreeSet};

use nettai_battle::Content;
use nettai_content_api::{Data, Registry};

use crate::report::Report;

/// Report on `c`'s definitions.
pub fn definitions(c: &Content, r: &mut Report) {
    let defs = &c.defs;
    // Each game's roles (a root with a stock ruleset; content with no
    // rulesets, its own root's).
    let games: Vec<usize> = (0..defs.roles.len())
        .filter(|&i| i == 0 || defs.roots.get(i).is_some_and(|name| defs.stock_ruleset_of(name).is_some()))
        .collect();
    for i in games {
        if defs.definitions.is_empty() {
            break;
        }
        let file = match defs.roots.get(i) {
            Some(name) if defs.roots.len() > 1 => format!("{name}:rules/roles.luau"),
            _ => "rules/roles.luau".to_string(),
        };
        let roles = &defs.roles[i];
        use nettai_battle::content::{ActionRole, KindRole};
        for role in ActionRole::ALL {
            if !roles.actions.contains_key(&role) {
                r.warn(&file, format!("the role actions.{} is not filled", role.name()));
            }
        }
        for role in KindRole::ALL {
            if !roles.kinds.contains_key(&role) {
                r.warn(&file, format!("the role kinds.{} is not filled", role.name()));
            }
        }
        // The roles that name a definition of their registry, or an asset.
        use nettai_battle::content::{
            BannerRole, CollisionRole, EffectRole, LockonRole, MusicRole, RegionRole, SoundRole, SparkRole, SpriteRole,
            StatusRole,
        };
        let mut unfilled = |group: &str, name: &str, filled: bool| {
            if !filled {
                r.warn(&file, format!("the role {group}.{name} is not filled"));
            }
        };
        // (The panels' burn and splash are needed only by a game whose own
        // panels burn or hold: BN5's lava and sea.)
        let own = c.rules.get(i).map(|rules| &rules.panels.types[..]).unwrap_or(&[]);
        let needs_burn = own.iter().any(|t| t.named && t.burn.is_some());
        let needs_splash = own.iter().any(|t| t.named && t.holds.is_some());
        for role in LockonRole::ALL {
            unfilled("lockon", role.name(), roles.lockons.contains_key(&role));
        }
        for role in StatusRole::ALL {
            unfilled("statuses", role.name(), roles.statuses.contains_key(&role));
        }
        for &role in EffectRole::ALL {
            let needed = role != EffectRole::PanelSplash || needs_splash;
            unfilled("effects", role.name(), !needed || roles.effects.contains_key(&role));
        }
        for &role in SparkRole::ALL {
            let needed = role != SparkRole::PanelBurn || needs_burn;
            unfilled("sparks", role.name(), !needed || roles.sparks.contains_key(&role));
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
    for (key, module, required) in self_bit_targets(c) {
        r.warn(
            format!("{module}.luau"),
            format!(
                "collision type {key} tests 0x80, the self bit {required}'s objects carry: one of {required}'s modules handed it as a target would reach objects its own game's word doesn't (docs/design/bn5-map.md §15.3 item 9)"
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

/// Collision types of a root that requires another (BN5's, which uses BN6's
/// modules) whose words test 0x80, named by a module of that root which
/// uses the other's modules: BN6 adds that bit to the self type of every
/// attack and object and BN5 has none, so a BN5 target type BN6's modules
/// take must not test it (docs/design/bn5-map.md §15.3 item 9; BN5's own
/// row 0x3D does, which is fine while only BN5's code uses it). Each with
/// the module that names it and the root it requires. (A module names a
/// type as `collision.<id with underscores>`, the way rules/collision
/// exports it.)
pub fn self_bit_targets(c: &Content) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    for d in c.defs.definitions.of(Registry::Collision) {
        let root = nettai_content_api::keys::root_of(&d.key).unwrap_or_default();
        let Some(manifest) = c.scripts.roots.iter().find(|m| m.name == root) else { continue };
        let tests = ["side0", "side1"].iter().any(|k| d.spec.field(k).int().is_some_and(|w| w & 0x80 != 0));
        if !tests {
            continue;
        }
        let field = format!("collision.{}", nettai_content_api::keys::local(&d.key).replace('-', "_"));
        for required in &manifest.requires {
            let uses = format!("@{required}/");
            let prefix = format!("{root}{}", nettai_content_api::keys::SEPARATOR);
            for (name, text) in &c.scripts.modules {
                let names = text.lines().any(|l| l.contains(&field) && !l.contains("define.collision"));
                if name.starts_with(&prefix) && text.contains(&uses) && names {
                    out.push((d.key.clone(), name.clone(), required.clone()));
                }
            }
        }
    }
    out
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
    /// A module's owner folder: `bn6:chips/minibomb` for `bn6:chips/minibomb/chip`.
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
        if !nettai_content_api::keys::local(&d.module).starts_with("objects/") {
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
