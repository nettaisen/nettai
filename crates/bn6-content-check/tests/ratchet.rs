//! The ratchet (docs/design/content-model-v2.md §12): each module's uses of
//! the numeric API the v2 API replaces, against an allowance that may only
//! shrink (tests/deprecated.txt). A module that uses more fails; one that
//! uses fewer fails until its allowance is lowered, so the list tracks the
//! conversion. `BN6_RATCHET_LOWER=1` rewrites the list when every module is
//! at or under its allowance (it never raises one).

use std::collections::BTreeMap;
use std::path::Path;

fn pack() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/bn6")
}

const ALLOWANCE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/deprecated.txt");

fn allowance() -> BTreeMap<String, usize> {
    let text = std::fs::read_to_string(ALLOWANCE).unwrap_or_default();
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| {
            let (path, n) = l.rsplit_once(' ').expect("`path count`");
            (path.to_string(), n.parse().expect("a count"))
        })
        .collect()
}

#[test]
fn deprecated_uses_only_shrink() {
    let uses = bn6_content_check::deprecated_uses(&pack()).unwrap();
    let counts: BTreeMap<String, usize> = uses.iter().map(|(p, u)| (p.clone(), u.len())).collect();
    let allowed = allowance();
    let mut over = Vec::new();
    let mut under = Vec::new();
    for (path, &n) in &counts {
        let a = allowed.get(path).copied().unwrap_or(0);
        if n > a {
            let new: Vec<String> =
                uses[path].iter().map(|d| format!("    line {}: {} (use {})", d.line, d.what, d.instead)).collect();
            over.push(format!("{path}: {n} deprecated uses, {a} allowed:\n{}", new.join("\n")));
        } else if n < a {
            under.push(format!("{path}: {n} (allowance {a})"));
        }
    }
    for (path, &a) in &allowed {
        if !counts.contains_key(path) {
            under.push(format!("{path}: 0 (allowance {a})"));
        }
    }
    assert!(over.is_empty(), "new uses of the deprecated numeric API:\n{}", over.join("\n"));
    if !under.is_empty() && std::env::var_os("BN6_RATCHET_LOWER").is_some() {
        let mut text = String::from(
            "# Deprecated numeric API uses allowed per module (crates/bn6-content-check/tests/ratchet.rs).\n\
             # Only ever lowered: BN6_RATCHET_LOWER=1 cargo test -p bn6-content-check --test ratchet\n",
        );
        for (path, n) in &counts {
            text += &format!("{path} {n}\n");
        }
        std::fs::write(ALLOWANCE, text).unwrap();
        return;
    }
    assert!(
        under.is_empty(),
        "modules use fewer deprecated calls than allowed; lower their allowance \
         (BN6_RATCHET_LOWER=1 cargo test -p bn6-content-check --test ratchet):\n{}",
        under.join("\n")
    );
}
