//! What `bn6-content check` reports about the definitions once the define
//! phase has run (docs/design/content-model-v2.md §7.7): the roles content
//! hasn't filled, and kinds under `objects/` that one owner alone uses
//! (colocation, §4). Duplicate keys, references to the wrong registry,
//! unknown asset names and chips without exactly one use are the define
//! phase's own errors.

use std::collections::{BTreeMap, BTreeSet};

use bn6_battle::Content;
use bn6_content_api::{Data, Registry};

use crate::report::Report;

/// Report on `c`'s definitions.
pub fn definitions(c: &Content, r: &mut Report) {
    let defs = &c.defs;
    if !defs.definitions.is_empty() {
        use bn6_battle::content::{ActionRole, KindRole, RoleAction, RoleKind};
        for role in ActionRole::ALL {
            match defs.roles.actions.get(&role) {
                None => r.warn("rules/roles.luau", format!("the role actions.{} is not filled", role.name())),
                Some(RoleAction::Unported(n)) => r.warn(
                    "rules/roles.luau",
                    format!("the role actions.{} names action {n:#x}, which nothing implements yet", role.name()),
                ),
                Some(RoleAction::Action(_)) => {}
            }
        }
        for role in KindRole::ALL {
            match defs.roles.kinds.get(&role) {
                None => r.warn("rules/roles.luau", format!("the role kinds.{} is not filled", role.name())),
                Some(RoleKind::Missing(key)) => r.warn(
                    "rules/roles.luau",
                    format!("the role kinds.{} names the kind {key:?}, which the content doesn't have", role.name()),
                ),
                Some(RoleKind::Kind(_)) => {}
            }
        }
    }
    for (kind, owners) in single_owner_kinds(c) {
        r.warn(
            format!("{kind}"),
            format!("only {owners} uses this kind: move it into {owners}'s folder (docs/design/content-model-v2.md §4)"),
        );
    }
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
