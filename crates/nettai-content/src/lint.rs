//! What `nettai-content check` reports about the definitions once the define
//! phase has run (docs/design/content-model-v2.md §7.7): the roles content
//! hasn't filled, kinds under `objects/` that one owner alone uses
//! (colocation, §4), and the collision types that test EXE6's 0x80 self
//! bit which a module using another root's modules names. (Two collision
//! types of one row of the original's table are the verification
//! workspace's to find, by compat's rows.)
//! Duplicate keys, references to the
//! wrong registry, unknown asset names and chips without exactly one use
//! are the define phase's own errors; two keys with one of the original's
//! numbers is compat's (`exe6_compat::Compat` refuses to read it).

use std::collections::{BTreeMap, BTreeSet};

use nettai_battle::Content;
use nettai_content_api::{Data, Registry};

use crate::report::Report;

/// Report on `c`'s definitions.
pub fn definitions(c: &Content, r: &mut Report) {
    let defs = &c.defs;
    // The game's roles (the support packs have none).
    if !defs.definitions.is_empty() {
        let file = if defs.game.is_empty() { "rules/roles.luau".to_string() } else { format!("{}/rules/roles.luau", defs.game) };
        let roles = defs.roles();
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
            BannerRole, CollisionRole, EffectRole, MusicRole, RegionRole, SoundRole, SparkRole, SpriteRole,
            StatusRole,
        };
        let mut unfilled = |group: &str, name: &str, filled: bool| {
            if !filled {
                r.warn(&file, format!("the role {group}.{name} is not filled"));
            }
        };
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
            // (A sound some game hasn't is filled where the game has it.)
            unfilled("sounds", role.name(), role.optional() || roles.sounds.contains_key(&role));
        }
        for &role in MusicRole::ALL {
            unfilled("music", role.name(), role.optional() || roles.music.contains_key(&role));
        }
        for &role in BannerRole::ALL {
            unfilled("banners", role.name(), roles.banners.contains_key(&role));
        }
        for &role in SpriteRole::ALL {
            unfilled("sprites", role.name(), roles.sprites.contains_key(&role));
        }
    }
    for (key, module, required) in self_bit_targets(c) {
        r.warn(
            format!("{module}.luau"),
            format!(
                "collision type {key} tests 0x80, the self bit {required}'s objects carry: one of {required}'s modules handed it as a target would reach objects its own game's word doesn't (docs/design/exe5-map.md §15.3 item 9)"
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

/// Collision types of a game that requires a support pack (EXE5's, which uses
/// exelib's makers, EXE6's code made to take a game's looks) whose words test
/// 0x80, named by a module of that game which uses the support pack's
/// modules: EXE6 adds that bit to the self type of every attack and object
/// and EXE5 has none, so an EXE5 target type exelib's makers take must not
/// test it (docs/design/exe5-map.md §15.3 item 9; EXE5's own row 0x3D does,
/// which is fine while only EXE5's code uses it). Each with the module that
/// names it and the pack it requires. (A module names a type as
/// `collision.<id with underscores>`, the way rules/collision exports it.)
pub fn self_bit_targets(c: &Content) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    // A game whose own types carry the self bit may test it: its words
    // reach the same objects through a support pack's makers as through its
    // own code. Its rules say so (`hit_test.float_shoe_needs_self_bit`:
    // its hit test goes by the bit; EXE6's does, EXE5's types have none).
    if c.rules().hit_test.float_shoe_needs_self_bit {
        return out;
    }
    for (d, t) in c.defs.definitions.of(Registry::Collision).iter().zip(&c.defs.collisions) {
        let root = c.game();
        // (Its words, as the loader made them of its members' names.)
        let tests = t.flags.iter().any(|w| w.contains(nettai_battle::collision::CollisionTypes::REACHES_FLOATSHOE));
        if !tests {
            continue;
        }
        // (Its field in the game's rules/collision table: its key's last
        // part, `rules/collision/probe`'s `probe`.)
        let short = nettai_content_api::keys::local(&d.key).rsplit('/').next().unwrap_or_default().to_string();
        let field = format!("collision.{}", short.replace('-', "_"));
        // (The other packs whose modules a module of the type's game uses.)
        let dirs: BTreeSet<String> = c.scripts.modules.keys().filter_map(|m| nettai_content_api::keys::root_of(m)).map(str::to_string).collect();
        for required in dirs.into_iter().filter(|n| n != root) {
            let required = &required;
            let uses = format!("@{required}/");
            let prefix = format!("{root}{}", nettai_content_api::keys::SEPARATOR);
            for (name, text) in &c.scripts.modules {
                let names = text.lines().any(|l| l.contains(&field) && !l.contains("new.collision"));
                if name.starts_with(&prefix) && text.contains(&uses) && names {
                    out.push((short.clone(), name.clone(), required.clone()));
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
    /// A module's owner folder: `exe6:chips/minibomb` for `exe6:chips/minibomb/init`.
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

/// The members a definitions file declares of each value table of the names a
/// game defines (`declare PanelType: { missing: PanelType, ... }`), by kind.
pub fn declared_names(source: &str) -> Vec<(nettai_content_api::NameKind, Vec<String>)> {
    let mut out = Vec::new();
    for kind in nettai_content_api::NameKind::ALL {
        let head = format!("declare {}: {{", kind.table());
        let Some(at) = source.find(&head) else { continue };
        let body = &source[at + head.len()..];
        let body = &body[..body.find('}').unwrap_or(body.len())];
        let members = body
            .split(',')
            .filter_map(|m| {
                let name = m.split(':').next()?.trim();
                let name = name.strip_prefix("[\"").and_then(|n| n.strip_suffix("\"]")).unwrap_or(name);
                (!name.is_empty()).then(|| name.to_string())
            })
            .collect();
        out.push((kind, members));
    }
    out
}

/// A game's types.d.luau declares its value tables' members as the game has
/// them (`Content::names`), and its support packs' theirs among them (the
/// names every game has): what the content check holds a module to is what a
/// load holds it to.
pub fn names(dir: &std::path::Path, c: &Content, r: &mut Report) {
    let game = &c.defs.game;
    let mut packs = vec![(game.clone(), true)];
    packs.extend(c.scripts.packs.iter().filter(|p| p.kind == nettai_content_api::PackKind::Support).map(|p| (p.id.clone(), false)));
    for (pack, own) in packs {
        let file = format!("{pack}/types.d.luau");
        let Ok(source) = std::fs::read_to_string(dir.join(&file)) else { continue };
        let declared = declared_names(&source);
        for kind in nettai_content_api::NameKind::ALL {
            let has = c.names.of(kind);
            let Some((_, members)) = declared.iter().find(|(k, _)| *k == kind) else {
                if own {
                    r.error(&file, format!("declares no `{}` table (the game's: {})", kind.table(), has.join(", ")));
                }
                continue;
            };
            let mut a: Vec<&str> = members.iter().map(String::as_str).collect();
            let mut b: Vec<&str> = has.iter().map(String::as_str).collect();
            a.sort();
            b.sort();
            if own && a != b {
                r.error(&file, format!("declares {} {{ {} }}, but the game's are {{ {} }} (verify declare.py writes them)", kind.table(), a.join(", "), b.join(", ")));
            } else if !own && let Some(n) = a.iter().find(|n| !b.contains(n)) {
                r.error(&file, format!("declares {}.{n}, which {game} hasn't (a support pack's are the names every game has)", kind.table()));
            }
        }
    }
}
