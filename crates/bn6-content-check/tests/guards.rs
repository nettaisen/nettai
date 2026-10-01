//! Guards for the end state of content model v2 (docs/design/
//! content-model-v2.md §12, step 13): nothing in the content is named by
//! one of the original's numbers, and no definition carries a `legacy`
//! marker. Each has its exceptions listed here, which only shrink: an
//! exception nothing needs any more fails too.

use std::path::{Path, PathBuf};

use bn6_content_check::lints::Scanned;

/// The repository's root.
fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The content the guards cover: BN6's, and the engine's test content and
/// test pack.
const ROOTS: [&str; 3] = ["content/bn6", "crates/bn6-battle/testdata/content", "crates/bn6-battle/testdata/pack"];

/// Folders whose name starts like a number of the original's
/// (`00f-gundels1`, `0a-tengu-beast-charge`) but isn't one: none.
const NUMBERED_FOLDERS: &[&str] = &[];

/// The modules that still hold `legacy` markers, by path (`*`: any one
/// folder): the navis' and forms' numbers, and the original's numbering of
/// the body overlays, which the ruleset still reads by number. Step 13's
/// second part converts them and empties this list.
const LEGACY_MARKERS: &[&str] = &[
    "content/bn6/navis/*/navi.luau",
    "content/bn6/navis/megaman/forms/*/form.luau",
    "content/bn6/rules/body-overlays.luau",
    "crates/bn6-battle/testdata/content/navis/test.luau",
];

/// Every folder and every `.luau` module under `dir`, as paths from the
/// repository's root with `/`.
fn walk(root: &Path, dir: &Path, folders: &mut Vec<String>, modules: &mut Vec<String>) {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())).map(|e| e.unwrap().path()).collect();
    entries.sort();
    for path in entries {
        let rel = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
        if path.is_dir() {
            folders.push(rel);
            walk(root, &path, folders, modules);
        } else if rel.ends_with(".luau") && !rel.ends_with(".d.luau") {
            modules.push(rel);
        }
    }
}

fn content() -> (Vec<String>, Vec<String>) {
    let root = repository().canonicalize().unwrap();
    let (mut folders, mut modules) = (Vec::new(), Vec::new());
    for dir in ROOTS {
        walk(&root, &root.join(dir), &mut folders, &mut modules);
    }
    (folders, modules)
}

/// Whether `path` is `pattern`, a `*` of which is any one folder or file
/// name.
fn matches(pattern: &str, path: &str) -> bool {
    let (p, q): (Vec<&str>, Vec<&str>) = (pattern.split('/').collect(), path.split('/').collect());
    p.len() == q.len() && p.iter().zip(&q).all(|(a, b)| *a == "*" || a == b)
}

/// Whether a folder's name starts with two or three hex digits and a
/// dash: the form the original's numbers took in folder names.
fn numbered(name: &str) -> bool {
    let digits = name.bytes().take_while(|c| matches!(c, b'0'..=b'9' | b'a'..=b'f')).count();
    matches!(digits, 2 | 3) && name.as_bytes().get(digits) == Some(&b'-')
}

#[test]
fn no_folder_carries_an_original_number() {
    let (folders, _) = content();
    assert!(folders.len() > 300, "found the content's folders ({})", folders.len());
    let found: Vec<&String> = folders.iter().filter(|f| numbered(f.rsplit('/').next().unwrap())).collect();
    let new: Vec<&&String> = found.iter().filter(|f| !NUMBERED_FOLDERS.contains(&f.as_str())).collect();
    assert!(
        new.is_empty(),
        "folders named with one of the original's numbers (content is named by key; the numbers are compat's):\n{}",
        new.iter().map(|f| format!("  {f}")).collect::<Vec<_>>().join("\n")
    );
    let stale: Vec<&&str> = NUMBERED_FOLDERS.iter().filter(|e| !found.iter().any(|f| f.as_str() == **e)).collect();
    assert!(stale.is_empty(), "exceptions no folder needs any more: {stale:?}");
    // (What the guard catches.)
    assert!(numbered("00f-gundels1") && numbered("0a-tengu-beast-charge") && numbered("00-megaman"));
    assert!(!numbered("megaman") && !numbered("z-saver") && !numbered("element-pillar") && !numbered("dblbeast"));
}

/// The lines of `source` that hold a `legacy` marker (`legacy { ... }`,
/// `legacy = { ... }`, `legacy(...)`), outside comments and strings.
fn legacy_markers(source: &str) -> Vec<usize> {
    let s = Scanned::new(source);
    s.find("legacy")
        .filter(|&at| {
            let after = &s.code[at + "legacy".len()..];
            !after.starts_with(|c: char| c.is_alphanumeric() || c == '_') && after.trim_start().starts_with(['{', '(', '='])
        })
        .map(|at| s.line(at))
        .collect()
}

#[test]
fn no_definition_carries_a_legacy_marker() {
    let (_, modules) = content();
    assert!(modules.len() > 600, "found the content's modules ({})", modules.len());
    let root = repository();
    let mut new = Vec::new();
    let mut used = vec![false; LEGACY_MARKERS.len()];
    for module in &modules {
        let lines = legacy_markers(&std::fs::read_to_string(root.join(module)).unwrap());
        if lines.is_empty() {
            continue;
        }
        match LEGACY_MARKERS.iter().position(|e| matches(e, module)) {
            Some(i) => used[i] = true,
            None => new.push(format!("  {module}: line {}", lines.iter().map(|l| l.to_string()).collect::<Vec<_>>().join(", "))),
        }
    }
    assert!(
        new.is_empty(),
        "`legacy` markers (a definition says what it is by name; the original's numbers are compat's):\n{}",
        new.join("\n")
    );
    let stale: Vec<&str> = LEGACY_MARKERS.iter().zip(&used).filter(|(_, u)| !**u).map(|(e, _)| *e).collect();
    assert!(stale.is_empty(), "exceptions no module needs any more (remove them): {stale:?}");
    // (What the guard catches.)
    assert_eq!(legacy_markers("local n = define.navi {\n    legacy = legacy { number = 1 },\n}\n-- legacy { x }\n"), [2, 2]);
    assert_eq!(legacy_markers("turn = { legacy = { action = 0x3B } }\nlocal s = 'legacy {'\nlocal legacy_name = 1\n"), [1]);
}
