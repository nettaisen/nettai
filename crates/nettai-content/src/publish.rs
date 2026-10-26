//! `nettai-content publish <game>` (docs/design/content-deployment.md §4): publish a game's content for this
//! engine's line as a release of the game's repository (`nettaisen/<game>`), whose Pages workflow then builds the
//! game's site from its releases.
//!
//! Run in the engine checkout of the line, it does six things in order:
//!
//! 1. It refuses unless the game's submodule and its support packs' are clean, at the commits the engine's HEAD pins,
//!    and on their repositories' main branches (fetched first).
//! 2. It type-checks those packs (`nettai-content-check --pack ...`, run with cargo: that crate's Luau can't be
//!    linked beside this one's).
//! 3. It builds the bundle from the submodules' folders, numbered after the line's releases.
//! 4. It unpacks the bundle and loads it with the game's asset pack, for its content hash.
//! 5. It refuses a bundle that is the line's newest again: the same id, or the same content hash and pack commits.
//! 6. It creates the release: tag `<line>-r<revision>` on the game pack's commit, with the bundle (`<id>.tar.gz`)
//!    and its index entry (`bundle.json`, a [`Published`]).
//!
//! It shows the `gh release create` it runs; `--dry-run` does all but run it, and leaves the release's files in
//! `target/publish/<game>/<tag>/`.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

use nettai_content::bundle::{self, BundleId, Manifest, Published};
use nettai_content::pack;
use nettai_content::report::{Level, Report};
use nettai_content_api::{PackKind, packs};

/// The workspace's manifest, whose version is this engine's: the engine that computes the content hash is this
/// program's, so the line is the one it was built at.
const WORKSPACE: &str = include_str!("../../../Cargo.toml");

/// Whom the content repositories belong to (the sites' owner, [`bundle::site`]).
const OWNER: &str = "nettaisen";

/// The branch a published commit must be on.
const MAIN: &str = "origin/main";

/// The release's index entry's name among its files.
const ENTRY: &str = "bundle.json";

pub const USAGE: &str = "  nettai-content publish <game> [--packs DIR] [--dry-run]
                                                   publish the game's content for this engine's line: a
                                                   release of nettaisen/<game> with its bundle, from the
                                                   submodules (run in the engine checkout; DIR: the game's
                                                   asset pack, default data/<game>)";

struct Options {
    game: String,
    /// The game's asset pack (`data/<game>` of the checkout when not given).
    packs: Option<PathBuf>,
    dry_run: bool,
}

fn parse(mut args: impl Iterator<Item = String>) -> Result<Options, String> {
    let usage = || format!("usage:\n{USAGE}");
    let mut o = Options { game: String::new(), packs: None, dry_run: false };
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--packs" => o.packs = Some(args.next().ok_or("--packs needs the game's asset pack's folder")?.into()),
            "--dry-run" => o.dry_run = true,
            a if a.starts_with("--") => return Err(format!("unknown option {a}\n{}", usage())),
            _ if !o.game.is_empty() => return Err(format!("one game at a time\n{}", usage())),
            _ => o.game = arg,
        }
    }
    if o.game.is_empty() {
        return Err(usage());
    }
    Ok(o)
}

/// Publish (see the module's documentation).
pub fn run(args: impl Iterator<Item = String>) -> Result<(), String> {
    let o = parse(args)?;
    let root = engine_root()?;
    let line = line_of(&workspace_version(WORKSPACE)?);
    let checkout = std::fs::read_to_string(root.join("Cargo.toml")).map_err(|e| format!("{}: {e}", root.display()))?;
    let checkout = line_of(&workspace_version(&checkout)?);
    if checkout != line {
        return Err(format!("this program is nettai {line}'s, and {} is {checkout}'s: run the checkout's own", root.display()));
    }
    let content = root.join("content");
    let all: BTreeMap<String, packs::PackManifest> = packs::packs(&content)?.into_iter().map(|p| (p.id.clone(), p)).collect();
    match all.get(&o.game) {
        None => return Err(format!("{} has no pack {}", content.display(), o.game)),
        Some(p) if p.kind == PackKind::Support => return Err(format!("{} is a support pack: it's published with the games that depend on it", o.game)),
        Some(_) => {}
    }
    let members: Vec<String> = packs::with_depends(&all, &o.game)?.iter().map(|p| p.id.clone()).collect();
    let repo = format!("{OWNER}/{}", o.game);

    say(&format!("1. the packs' repositories ({})", members.join(", ")));
    let mut commits = BTreeMap::new();
    for id in &members {
        let commit = pack_commit(&root, id)?;
        eprintln!("   {id} at {commit}");
        commits.insert(id.clone(), commit);
    }

    say("2. the type check");
    type_check(&root, &members)?;

    say(&format!("3. the bundle (the releases of {repo})"));
    let tags = release_tags(&repo)?;
    let (revision, newest) = next_revision(&line, &tags)?;
    let newest = newest.map(|t| published_of(&repo, &t)).transpose()?;
    let manifest = Manifest { game: o.game.clone(), line: line.clone(), revision, packs: commits.clone() };
    let folders: Vec<(&str, PathBuf)> = members.iter().map(|id| (id.as_str(), content.join(id))).collect();
    let folders: Vec<(&str, &Path)> = folders.iter().map(|(id, dir)| (*id, dir.as_path())).collect();
    let built = bundle::build(manifest, &folders)?;
    let gz = bundle::gzip(&built.tar);
    eprintln!("   r{revision}: {} ({} bytes)", built.id, gz.len());

    say("4. the content hash");
    let assets = o.packs.unwrap_or_else(|| root.join("data").join(&o.game));
    let work = root.join("target").join("publish").join(&o.game);
    if work.exists() {
        std::fs::remove_dir_all(&work).map_err(|e| format!("{}: {e}", work.display()))?;
    }
    let unpacked = work.join(built.id.to_string());
    let hash = content_hash(&gz, built.id, &unpacked, &assets, &o.game);
    let _ = std::fs::remove_dir_all(&unpacked);
    let hash = hash?;
    let published = Published { id: built.id, content_hash: hash, revision, packs: commits, date: today(), size: gz.len() as u64 };
    eprintln!("   {:016x} (with {})", hash.0, assets.display());

    say("5. the line's newest");
    if let Some(why) = refusal(newest.as_ref(), &published) {
        return Err(why);
    }
    match &newest {
        Some(n) => eprintln!("   r{}: {:016x}, another", n.revision, n.content_hash.0),
        None => eprintln!("   none: the line's first"),
    }

    let tag = tag(&line, revision);
    let title = title(&o.game, &line, revision);
    let release = work.join(&tag);
    std::fs::create_dir_all(&release).map_err(|e| format!("{}: {e}", release.display()))?;
    let file = |name: &str, data: &[u8]| -> Result<PathBuf, String> {
        let path = release.join(name);
        std::fs::write(&path, data).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(path)
    };
    let gz_file = file(&format!("{}.tar.gz", built.id), &gz)?;
    let entry = file(ENTRY, entry_json(&published).as_bytes())?;
    let notes = file("notes.md", notes(&o.game, &line, &published).as_bytes())?;
    let target = &published.packs[&o.game];
    let args: Vec<&OsStr> = [
        OsStr::new("release"),
        OsStr::new("create"),
        OsStr::new(&tag),
        OsStr::new("--repo"),
        OsStr::new(&repo),
        OsStr::new("--target"),
        OsStr::new(target),
        OsStr::new("--title"),
        OsStr::new(&title),
        OsStr::new("--notes-file"),
        notes.as_os_str(),
        gz_file.as_os_str(),
        entry.as_os_str(),
    ]
    .into();
    let command = std::iter::once(OsStr::new("gh")).chain(args.iter().copied()).map(quoted).collect::<Vec<_>>().join(" ");
    eprint!("{}", entry_json(&published));
    if o.dry_run {
        say(&format!("6. (dry run) the release {title}, by\n{command}"));
        return Ok(());
    }

    say(&format!("6. the release {title}, by\n{command}"));
    let url = run_tool("gh", None, &args)?;
    eprintln!("   {url}");
    eprintln!("   the site builds from it: {} and {}", bundle::index_url(&o.game), bundle::bundle_url(&o.game, built.id));
    Ok(())
}

/// `arg` as a POSIX shell reads it back: bare when it is only safe characters, else single-quoted.
fn quoted(arg: &OsStr) -> String {
    let s = arg.to_string_lossy();
    if !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || "-_./:=@+,".contains(c)) {
        s.into_owned()
    } else {
        format!("'{}'", s.replace('\'', r"'\''"))
    }
}

fn say(what: &str) {
    eprintln!("{what}");
}

/// The engine checkout the program runs in: the working directory's repository, which holds the workspace.
fn engine_root() -> Result<PathBuf, String> {
    let root = PathBuf::from(run_tool("git", None, &[OsStr::new("rev-parse"), OsStr::new("--show-toplevel")])?);
    let workspace = std::fs::read_to_string(root.join("Cargo.toml")).unwrap_or_default();
    if !workspace.contains("[workspace]") || !root.join("content").is_dir() {
        return Err(format!("{} is no engine checkout: run publish in one (not in a content submodule)", root.display()));
    }
    Ok(root)
}

/// The workspace's version (`[workspace.package] version`) in its manifest's text.
fn workspace_version(manifest: &str) -> Result<String, String> {
    let doc: toml::Table = toml::from_str(manifest).map_err(|e| format!("Cargo.toml: {e}"))?;
    doc.get("workspace")
        .and_then(|w| w.get("package"))
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| "Cargo.toml states no [workspace.package] version".to_string())
}

/// A version's line: its major and minor version (`0.11` for `0.11.2`).
fn line_of(version: &str) -> String {
    version.split('.').take(2).collect::<Vec<_>>().join(".")
}

/// Run `program` (in `dir`, if given) with `args`: its output, trimmed, or what went wrong.
fn run_tool(program: &str, dir: Option<&Path>, args: &[&OsStr]) -> Result<String, String> {
    let mut cmd = Command::new(program);
    if let Some(dir) = dir {
        cmd.arg("-C").arg(dir);
    }
    let shown = || {
        let mut s = program.to_string();
        if let Some(dir) = dir {
            s += &format!(" -C {}", dir.display());
        }
        args.iter().for_each(|a| s += &format!(" {}", a.to_string_lossy()));
        s
    };
    let out = cmd.args(args).output().map_err(|e| format!("{}: {e}", shown()))?;
    if !out.status.success() {
        return Err(format!("{}: {}", shown(), String::from_utf8_lossy(&out.stderr).trim()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim_end().to_string())
}

fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    run_tool("git", Some(dir), &args.iter().map(OsStr::new).collect::<Vec<_>>())
}

/// Pack `id`'s commit, once its submodule is clean, at what the engine's HEAD pins, and on its repository's main.
fn pack_commit(root: &Path, id: &str) -> Result<String, String> {
    let path = format!("content/{id}");
    let dir = root.join(&path);
    git(&dir, &["fetch", "--quiet", "origin"])?;
    let status = git(&dir, &["status", "--porcelain", "--ignored", "--untracked-files=all"])?;
    let dirty = unclean(&status);
    if !dirty.is_empty() {
        let shown: Vec<&str> = dirty.iter().take(10).copied().collect();
        return Err(format!(
            "{path} isn't clean: a bundle holds what its commit holds, nothing else\n  {}{}",
            shown.join("\n  "),
            if dirty.len() > shown.len() { format!("\n  and {} more", dirty.len() - shown.len()) } else { String::new() }
        ));
    }
    let head = git(&dir, &["rev-parse", "HEAD"])?;
    let pinned = git(root, &["rev-parse", &format!("HEAD:{path}")])?;
    if head != pinned {
        return Err(format!("{path} is at {head}, and the engine's HEAD pins {pinned}: commit the submodule's move first"));
    }
    let on_main = Command::new("git")
        .arg("-C")
        .arg(&dir)
        .args(["merge-base", "--is-ancestor", "HEAD", MAIN])
        .status()
        .map_err(|e| format!("git: {e}"))?;
    if !on_main.success() {
        return Err(format!("{path}'s commit {head} isn't on its repository's main ({MAIN}): push it there first"));
    }
    Ok(head)
}

/// The lines of `git status --porcelain --ignored` that stop a publish: all but litter a bundle leaves out anyway
/// (`.DS_Store`).
fn unclean(status: &str) -> Vec<&str> {
    status
        .lines()
        .filter(|l| !l.is_empty())
        .filter(|l| l.get(3..).and_then(|p| p.trim_end_matches('/').rsplit('/').next()) != Some(".DS_Store"))
        .collect()
}

/// Type-check packs `members` of the checkout's content: `nettai-content-check`, run with cargo, as this program
/// was built (its profile).
fn type_check(root: &Path, members: &[String]) -> Result<(), String> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut cmd = Command::new(cargo);
    cmd.current_dir(root).args(["run", "--quiet", "-p", "nettai-content-check"]);
    if !cfg!(debug_assertions) {
        cmd.arg("--release");
    }
    cmd.arg("--").arg(root.join("content"));
    for id in members {
        cmd.args(["--pack", id]);
    }
    let status = cmd.status().map_err(|e| format!("cargo run -p nettai-content-check: {e}"))?;
    if !status.success() {
        return Err("the content doesn't type-check (above)".to_string());
    }
    Ok(())
}

fn gh(args: &[&str]) -> Result<String, String> {
    run_tool("gh", None, &args.iter().map(OsStr::new).collect::<Vec<_>>())
}

/// The tags of `repo`'s published releases (drafts aside).
fn release_tags(repo: &str) -> Result<Vec<String>, String> {
    let tags = gh(&["api", "--paginate", &format!("repos/{repo}/releases"), "--jq", ".[] | select(.draft | not) | .tag_name"])?;
    Ok(tags.lines().filter(|l| !l.is_empty()).map(String::from).collect())
}

/// Revision `revision`'s release's tag in line `line` (`0.11-r2`).
fn tag(line: &str, revision: u32) -> String {
    format!("{line}-r{revision}")
}

/// The revision a release's tag names in line `line`, if it is one of the line's.
fn revision_of(line: &str, tag_name: &str) -> Option<u32> {
    let r: u32 = tag_name.strip_prefix(line)?.strip_prefix("-r")?.parse().ok()?;
    (tag(line, r) == tag_name).then_some(r)
}

/// The next revision in line `line`, given the releases' tags (every line's): one past the line's count, with the
/// tag of its newest, if any. The line's releases must be r1 to rN.
fn next_revision(line: &str, tags: &[String]) -> Result<(u32, Option<String>), String> {
    let mut revisions: Vec<u32> = tags.iter().filter_map(|t| revision_of(line, t)).collect();
    revisions.sort_unstable();
    let count = revisions.len() as u32;
    if revisions != (1..=count).collect::<Vec<_>>() {
        let shown: Vec<String> = revisions.iter().map(|r| tag(line, *r)).collect();
        return Err(format!("line {line}'s releases are {}, not r1 to r{count}: put them right first", shown.join(", ")));
    }
    Ok((count + 1, (count > 0).then(|| tag(line, count))))
}

/// The index entry of release `tag_name` of `repo` (its `bundle.json`).
fn published_of(repo: &str, tag_name: &str) -> Result<Published, String> {
    let text = gh(&["release", "download", tag_name, "--repo", repo, "--pattern", ENTRY, "--output", "-"])?;
    serde_json::from_str(&text).map_err(|e| format!("{repo}'s release {tag_name}'s {ENTRY}: {e}"))
}

/// Why bundle `new` mustn't follow `newest`, the line's newest: it is it again, by id, or plays the same from the
/// same commits.
fn refusal(newest: Option<&Published>, new: &Published) -> Option<String> {
    let n = newest?;
    if n.id == new.id {
        return Some(format!("r{} is this bundle already ({})", n.revision, n.id));
    }
    if n.content_hash == new.content_hash && n.packs == new.packs {
        return Some(format!(
            "r{} plays the same content ({:016x}) from the same commits: nothing to publish",
            n.revision, n.content_hash.0
        ));
    }
    None
}

/// The content hash of gzipped bundle `gz` (id `id`): unpacked into `dest` as an app's store unpacks it, and loaded
/// with game `game`'s asset pack `assets`.
fn content_hash(gz: &[u8], id: BundleId, dest: &Path, assets: &Path, game: &str) -> Result<nettai_battle::ContentHash, String> {
    let mut r = Report::default();
    let Some(m) = pack::read_manifest(assets, &mut r) else {
        return Err(format!("{}: {r}", assets.display()));
    };
    if m.game != game {
        return Err(format!("{} is {}'s asset pack, not {game}'s", assets.display(), m.game));
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    bundle::unpack(gz, id, dest)?;
    match pack::load_battle(dest, assets) {
        Ok((c, loaded)) => {
            for i in loaded.issues.iter().filter(|i| i.level != Level::Note) {
                eprintln!("   {i}");
            }
            Ok(c.hash())
        }
        Err(r) => Err(format!("the bundle doesn't load with {}:\n{r}", assets.display())),
    }
}

/// A release's `bundle.json`: its index entry, pretty, in the entry's field order.
fn entry_json(p: &Published) -> String {
    serde_json::to_string_pretty(p).expect("an entry is JSON") + "\n"
}

/// A release's title (`exe6 r1 for 0.11`).
fn title(game: &str, line: &str, revision: u32) -> String {
    format!("{game} r{revision} for {line}")
}

/// A release's notes: what it is, and what names it.
fn notes(game: &str, line: &str, p: &Published) -> String {
    let packs: Vec<String> = p.packs.iter().map(|(id, commit)| format!("{id} at `{commit}`")).collect();
    format!(
        "The content {game} plays on nettai {line}.x, its revision {} for the line.\n\n\
         - Bundle: `{}` ({} bytes)\n\
         - Content hash: `{:016x}`\n\
         - Packs: {}\n",
        p.revision,
        p.id,
        p.size,
        p.content_hash.0,
        packs.join(", ")
    )
}

/// Today's date in UTC (YYYY-MM-DD).
fn today() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let (y, m, d) = civil((secs / 86_400) as i64);
    format!("{y:04}-{m:02}-{d:02}")
}

/// The date `days` days after 1970-01-01 (proleptic Gregorian; Howard Hinnant's `civil_from_days`).
fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_battle::ContentHash;

    fn entry(revision: u32, id: u8, hash: u64, exe6: &str) -> Published {
        Published {
            id: BundleId([id; 32]),
            content_hash: ContentHash(hash),
            revision,
            packs: [("exe6".to_string(), exe6.to_string()), ("exelib".to_string(), "bbbb".to_string())].into(),
            date: "2026-10-26".into(),
            size: 1234,
        }
    }

    fn tags(t: &[&str]) -> Vec<String> {
        t.iter().map(|t| t.to_string()).collect()
    }

    #[test]
    fn the_line_is_the_workspaces_major_and_minor() {
        assert_eq!(line_of("0.11.2"), "0.11");
        assert_eq!(line_of("1.0.0-beta.1"), "1.0");
        assert_eq!(workspace_version("[workspace.package]\nversion = \"0.12.3\"\n").unwrap(), "0.12.3");
        assert!(workspace_version("[package]\nversion = \"0.1.0\"\n").is_err());
        let line = line_of(&workspace_version(WORKSPACE).unwrap());
        assert_eq!(line.split('.').count(), 2, "{line}");
    }

    #[test]
    fn revisions_count_the_lines_releases() {
        assert_eq!(next_revision("0.11", &[]).unwrap(), (1, None));
        // Other lines' releases, and tags that are no release of a line, don't count.
        let t = tags(&["0.10-r1", "0.10-r2", "0.10-r3", "0.11-r1", "v0.11", "0.11-r1x", "0.110-r1", "0.11-r01"]);
        assert_eq!(next_revision("0.11", &t).unwrap(), (2, Some("0.11-r1".into())));
        assert_eq!(next_revision("0.10", &t).unwrap(), (4, Some("0.10-r3".into())));
        assert_eq!(next_revision("0.12", &t).unwrap(), (1, None));
        // (Listed newest first, as GitHub lists them.)
        let t = tags(&["0.11-r10", "0.11-r9", "0.11-r8", "0.11-r7", "0.11-r6", "0.11-r5", "0.11-r4", "0.11-r3", "0.11-r2", "0.11-r1"]);
        assert_eq!(next_revision("0.11", &t).unwrap(), (11, Some("0.11-r10".into())));
        // A gap, or a revision twice, is refused: the count would name a release that is there.
        let err = next_revision("0.11", &tags(&["0.11-r1", "0.11-r3"])).unwrap_err();
        assert!(err.contains("0.11-r1, 0.11-r3, not r1 to r2"), "{err}");
        assert!(next_revision("0.11", &tags(&["0.11-r2"])).is_err());
    }

    #[test]
    fn the_lines_newest_again_is_refused() {
        let newest = entry(2, 1, 0xA, "aaaa");
        assert_eq!(refusal(None, &entry(1, 1, 0xA, "aaaa")), None, "a line's first");
        let err = refusal(Some(&newest), &entry(3, 1, 0xB, "cccc")).unwrap();
        assert!(err.contains("r2 is this bundle"), "{err}");
        let err = refusal(Some(&newest), &entry(3, 2, 0xA, "aaaa")).unwrap();
        assert!(err.contains("same content") && err.contains("000000000000000a"), "{err}");
        // The same content from other commits (a change play doesn't see, as another language's text), and other
        // content from the same commits (the engine's line computes it anew), are new bundles.
        assert_eq!(refusal(Some(&newest), &entry(3, 2, 0xA, "cccc")), None);
        assert_eq!(refusal(Some(&newest), &entry(3, 2, 0xB, "aaaa")), None);
    }

    #[test]
    fn an_entry_is_written_in_its_field_order_and_reads_back() {
        let e = entry(1, 0xab, 0x0123_4567_89ab_cdef, "aaaa");
        let json = entry_json(&e);
        let keys: Vec<usize> = ["\"id\"", "\"content_hash\"", "\"revision\"", "\"packs\"", "\"exe6\"", "\"exelib\"", "\"date\"", "\"size\""]
            .iter()
            .map(|k| json.find(k).unwrap_or_else(|| panic!("{k} in {json}")))
            .collect();
        assert!(keys.windows(2).all(|w| w[0] < w[1]), "{json}");
        assert!(json.contains("\"content_hash\": \"0123456789abcdef\"") && json.ends_with("}\n"), "{json}");
        assert_eq!(entry_json(&e), json, "the same entry, the same text");
        assert_eq!(serde_json::from_str::<Published>(&json).unwrap(), e);
        // The site's index lists the releases' entries by line, in revision order: an entry appended reads back
        // after the others.
        let mut index = bundle::Index { game: "exe6".into(), lines: BTreeMap::new() };
        for r in 1..=3 {
            let text = entry_json(&entry(r, r as u8, u64::from(r), "aaaa"));
            index.lines.entry("0.11".into()).or_default().push(serde_json::from_str(&text).unwrap());
        }
        let back: bundle::Index = serde_json::from_str(&serde_json::to_string_pretty(&index).unwrap()).unwrap();
        assert_eq!(back.newest("0.11").unwrap().revision, 3);
        assert_eq!(back.lines["0.11"].iter().map(|p| p.revision).collect::<Vec<_>>(), [1, 2, 3]);
    }

    #[test]
    fn a_release_is_named_and_noted() {
        let e = entry(1, 0xab, 0xA, "aaaa");
        assert_eq!(tag("0.11", 1), "0.11-r1");
        assert_eq!(title("exe6", "0.11", 1), "exe6 r1 for 0.11");
        let n = notes("exe6", "0.11", &e);
        assert!(n.starts_with("The content exe6 plays on nettai 0.11.x, its revision 1 for the line.\n\n"), "{n}");
        assert!(n.contains(&format!("`{}`", e.id)) && n.contains("exe6 at `aaaa`, exelib at `bbbb`"), "{n}");
    }

    #[test]
    fn a_command_is_shown_as_a_shell_reads_it() {
        assert_eq!(quoted(OsStr::new("0.11-r1")), "0.11-r1");
        assert_eq!(quoted(OsStr::new("/x/target/publish/exe6/0.11-r1/bundle.json")), "/x/target/publish/exe6/0.11-r1/bundle.json");
        assert_eq!(quoted(OsStr::new("exe6 r1 for 0.11")), "'exe6 r1 for 0.11'");
        assert_eq!(quoted(OsStr::new("it's")), r"'it'\''s'");
        assert_eq!(quoted(OsStr::new("")), "''");
    }

    #[test]
    fn only_litter_is_clean() {
        assert!(unclean("").is_empty());
        assert!(unclean("?? .DS_Store\n!! chips/.DS_Store\n").is_empty());
        assert_eq!(unclean(" M init.luau\n?? .DS_Store\n?? chips/new.luau\n!! build/\n"), [" M init.luau", "?? chips/new.luau", "!! build/"]);
    }

    #[test]
    fn dates_are_utcs() {
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(11_016), (2000, 2, 29));
        assert_eq!(civil(20_752), (2026, 10, 26));
        assert_eq!(civil(-1), (1969, 12, 31));
        assert_eq!(today().len(), 10);
    }

    #[test]
    fn options() {
        let o = parse(tags(&["exe6", "--packs", "/x/data/exe6", "--dry-run"]).into_iter()).unwrap();
        assert_eq!((o.game.as_str(), o.packs.as_deref(), o.dry_run), ("exe6", Some(Path::new("/x/data/exe6")), true));
        assert!(parse(tags(&["exe6", "exe5"]).into_iter()).is_err());
        assert!(parse(tags(&["--dry-run"]).into_iter()).is_err());
        assert!(parse(tags(&["exe6", "--packs"]).into_iter()).is_err());
    }

    /// A bundle plays what the checkout's content plays: its content hash is the one a build run from the
    /// repository computes for the same game. (Needs the asset packs: `NETTAI_PACKS`, default the checkout's data/.)
    #[test]
    #[ignore]
    fn a_bundle_plays_what_the_checkout_plays() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let packs = std::env::var_os("NETTAI_PACKS").map_or_else(|| root.join("data"), PathBuf::from);
        let content = root.join("content");
        let all: BTreeMap<String, packs::PackManifest> = packs::packs(&content).unwrap().into_iter().map(|p| (p.id.clone(), p)).collect();
        for game in ["exe4", "exe5", "exe6"] {
            let members: Vec<String> = packs::with_depends(&all, game).unwrap().iter().map(|p| p.id.clone()).collect();
            let manifest = Manifest {
                game: game.into(),
                line: "0.11".into(),
                revision: 1,
                packs: members.iter().map(|id| (id.clone(), "0".repeat(40))).collect(),
            };
            let folders: Vec<(&str, PathBuf)> = members.iter().map(|id| (id.as_str(), content.join(id))).collect();
            let folders: Vec<(&str, &Path)> = folders.iter().map(|(id, dir)| (*id, dir.as_path())).collect();
            let built = bundle::build(manifest, &folders).unwrap();
            let dest = std::env::temp_dir().join(format!("nettai-plays-{game}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dest);
            let hash = content_hash(&bundle::gzip(&built.tar), built.id, &dest, &packs.join(game), game);
            let _ = std::fs::remove_dir_all(&dest);
            let (checkout, _) = pack::load_battle(&content, &packs.join(game)).unwrap_or_else(|r| panic!("{game}: {r}"));
            assert_eq!(hash.unwrap(), checkout.hash(), "{game}");
        }
    }

    /// Each game's published site: its index reads, and the line's newest bundle downloads, has its id, unpacks and
    /// loads with the main checkout's asset pack to the content hash the index gives. (Needs the network and the
    /// asset packs: `NETTAI_PACKS`, a folder of them, default the checkout's data/.)
    #[test]
    #[ignore]
    fn the_sites_serve_what_was_published() {
        let line = line_of(&workspace_version(WORKSPACE).unwrap());
        let packs = std::env::var_os("NETTAI_PACKS").map_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data"), PathBuf::from);
        let fetch = |url: &str| -> Vec<u8> {
            let out = Command::new("curl").args(["-fsSL", url]).output().unwrap();
            assert!(out.status.success(), "{url}: {}", String::from_utf8_lossy(&out.stderr));
            out.stdout
        };
        for game in ["exe4", "exe5", "exe6"] {
            let index: bundle::Index = serde_json::from_slice(&fetch(&bundle::index_url(game))).unwrap();
            assert_eq!(index.game, game);
            let lines = &index.lines[&line];
            assert_eq!(lines.iter().map(|p| p.revision).collect::<Vec<_>>(), (1..=lines.len() as u32).collect::<Vec<_>>());
            let newest = index.newest(&line).unwrap();
            let gz = fetch(&bundle::bundle_url(game, newest.id));
            assert_eq!(gz.len() as u64, newest.size, "{game}");
            let dest = std::env::temp_dir().join(format!("nettai-site-{game}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dest);
            let hash = content_hash(&gz, newest.id, &dest, &packs.join(game), game);
            let _ = std::fs::remove_dir_all(&dest);
            assert_eq!(hash.unwrap(), newest.content_hash, "{game} r{}", newest.revision);
        }
    }
}
