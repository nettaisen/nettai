//! The app's own packs (docs/app.md §17): each game's asset pack, extracted from the player's Legacy Collection
//! (`crate::bnlc`) into the data folder's `packs/`, on a thread of its own, when it is missing or another
//! version of nettai made it.
//!
//! Each pack is stamped (`stamp.toml` in it) with the version of nettai that made it and the fingerprint of the
//! names its assets are written under (`nettai_extract::fingerprint`): two builds of one version whose content
//! names its assets otherwise write packs that differ, and the fingerprint tells them apart. A pack whose stamp
//! isn't this build's, or that has none, is extracted again. The pack's own files are the extractor's, as
//! `nettai-extract` writes them; the stamp is the one file more.
//!
//! A pack is written into a folder of its own first (`packs/.new/<game>`) and put in place by a rename once it
//! is whole, the old one moved aside (`packs/.old/<game>`) the moment before and deleted after: a crash leaves
//! the old pack or none, never half of one, and what one leaves is tidied as the app starts ([`tidy`]: a pack
//! moved aside and not replaced is put back). The two folders are inside the packs folder, so the renames are
//! on one disk, and a pack in them isn't one of the packs folder's (only its own folders are looked in).
//!
//! `$NETTAI_PACKS` wins: when it names a packs folder (development, the screenshot pass) the packs are that
//! folder's, and nothing is looked for or extracted.

use crate::bnlc::{self, Rom};
use nettai_extract::{Game, RomSet};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use nettai_platform::time::Instant;

/// A pack's stamp file.
const STAMP: &str = "stamp.toml";

/// Where the packs are: `$NETTAI_PACKS`, else the data folder's `packs`; and whether they are the app's own,
/// to extract.
pub fn dir() -> (PathBuf, bool) {
    match std::env::var_os("NETTAI_PACKS") {
        Some(dir) => (PathBuf::from(dir), false),
        None => (crate::paths::data().join("packs"), true),
    }
}

/// What made a pack: the version of nettai, and the fingerprint of its game's asset names.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stamp {
    pub nettai: String,
    pub assets: String,
}

impl Stamp {
    /// The stamp this build gives a pack of `game`.
    pub fn of(game: Game) -> Stamp {
        Stamp { nettai: env!("CARGO_PKG_VERSION").to_string(), assets: nettai_extract::fingerprint(game) }
    }

    fn read(pack: &Path) -> Option<Stamp> {
        toml::from_str(&nettai_platform::fs::read_to_string(pack.join(STAMP)).ok()?).ok()
    }

    fn write(&self, pack: &Path) -> Result<(), String> {
        let text = format!(
            "# What made this pack: nettai extracts it again when its version, or the names its assets are written under, differ\n# (docs/app.md §17).\n{}",
            toml::to_string(self).map_err(|e| e.to_string())?
        );
        nettai_platform::fs::write(pack.join(STAMP), text).map_err(|e| format!("{}: {e}", pack.join(STAMP).display()))
    }
}

/// What the packs folder has of a game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kept {
    /// No pack.
    None,
    /// One another build made (or that has no stamp).
    Stale,
    /// One this build made.
    Current,
}

/// What the packs folder `dir` has of game `id` (`game`, as the extractor knows it): its pack is `<dir>/<id>`.
pub fn kept(dir: &Path, id: &str, game: Game) -> Kept {
    let pack = dir.join(id);
    if !nettai_platform::fs::is_file(pack.join(nettai_content::pack::MANIFEST)) {
        Kept::None
    } else if Stamp::read(&pack).is_some_and(|s| s == Stamp::of(game)) {
        Kept::Current
    } else {
        Kept::Stale
    }
}

/// What an extraction cut short left in the packs folder `dir`: a pack moved aside and not replaced is put
/// back, and the rest goes. In the browser so does a pack with no stamp: the page keeps a pack's stamp after the
/// rest of it, so one without is one the page was closed while keeping (no `nettai-extract` writes there).
pub fn tidy(dir: &Path) {
    if cfg!(target_arch = "wasm32")
        && let Ok(read) = nettai_platform::fs::read_dir(dir)
    {
        for entry in read.flatten() {
            let pack = entry.path();
            let unfinished = entry.file_type().is_ok_and(|t| t.is_dir())
                && !entry.file_name().to_string_lossy().starts_with('.')
                && !nettai_platform::fs::is_file(pack.join(STAMP));
            if unfinished && let Err(why) = nettai_platform::fs::remove_dir_all(&pack) {
                crate::errors::log(&format!("removing the unfinished {}", pack.display()), &why);
            }
        }
    }
    let (new, old) = (dir.join(".new"), dir.join(".old"));
    if let Ok(read) = nettai_platform::fs::read_dir(&old) {
        for entry in read.flatten() {
            let place = dir.join(entry.file_name());
            if !nettai_platform::fs::exists(&place)
                && let Err(why) = nettai_platform::fs::rename(entry.path(), &place)
            {
                crate::errors::log(&format!("putting back {}", place.display()), &why);
            }
        }
    }
    for leftover in [new, old] {
        if nettai_platform::fs::exists(&leftover)
            && let Err(why) = nettai_platform::fs::remove_dir_all(&leftover)
        {
            crate::errors::log(&format!("removing {}", leftover.display()), &why);
        }
    }
}

/// Make game `id`'s pack in the packs folder `dir` anew: `make` writes it into the folder it is given (new and
/// empty), and once it has, it takes the old one's place. On a failure the old pack stays as it was.
pub fn replace(dir: &Path, id: &str, make: impl FnOnce(&Path) -> Result<(), String>) -> Result<(), String> {
    let (new, old, pack) = (dir.join(".new").join(id), dir.join(".old").join(id), dir.join(id));
    let said = |what: &str, path: &Path, e: std::io::Error| format!("{what} {}: {e}", path.display());
    if nettai_platform::fs::exists(&new) {
        nettai_platform::fs::remove_dir_all(&new).map_err(|e| said("removing", &new, e))?;
    }
    nettai_platform::fs::create_dir_all(dir.join(".new")).map_err(|e| said("making", dir, e))?;
    if let Err(why) = make(&new) {
        let _ = nettai_platform::fs::remove_dir_all(&new);
        return Err(why);
    }
    if nettai_platform::fs::exists(&pack) {
        if nettai_platform::fs::exists(&old) {
            nettai_platform::fs::remove_dir_all(&old).map_err(|e| said("removing", &old, e))?;
        }
        nettai_platform::fs::create_dir_all(dir.join(".old")).map_err(|e| said("making", dir, e))?;
        nettai_platform::fs::rename(&pack, &old).map_err(|e| said("moving aside", &pack, e))?;
    }
    if let Err(e) = nettai_platform::fs::rename(&new, &pack) {
        // (The old one back where it was.)
        let _ = nettai_platform::fs::rename(&old, &pack);
        return Err(said("putting in place", &new, e));
    }
    if nettai_platform::fs::exists(&old) {
        let _ = nettai_platform::fs::remove_dir_all(&old);
    }
    // (The two folders, if nothing else is in them.)
    let _ = nettai_platform::fs::remove_dir(dir.join(".new"));
    let _ = nettai_platform::fs::remove_dir(dir.join(".old"));
    Ok(())
}

/// A game's pack to extract: the game (its id, as the content has it, and the extractor's game) and its ROMs in
/// the install.
#[derive(Clone, Debug)]
pub struct Job {
    pub id: String,
    pub game: Game,
    pub roms: Vec<Rom>,
}

/// What the extraction tells the window's thread of a game: how far it is, 0 to 1, then that its pack is in
/// place (or why it isn't, for the log).
#[derive(Debug)]
pub enum Told {
    Progress(f32),
    Done(Result<(), String>),
}

/// How far into a game's extraction its stretches end: its ROMs read out of the archives, extracted; the pack
/// written (its files encoded, written and read back) is the rest, nearly all of it (a release build, each
/// game's four ROMs: about 0.15 s, 0.04 s and 1 s).
const READ: f32 = 0.12;
const EXTRACTED: f32 = 0.15;

/// Extract `job`'s pack into the packs folder `dir` (in place of the old one, once it is whole), telling
/// `progress` how far it is. The time each stretch took.
pub fn extract(dir: &Path, job: &Job, progress: &mut dyn FnMut(f32)) -> Result<[f32; 3], String> {
    let began = Instant::now();
    let mut roms = RomSet::default();
    for (i, rom) in job.roms.iter().enumerate() {
        roms.insert(bnlc::read(rom)?).map_err(|e| format!("{} in {}: {e}", rom.name, rom.archive.display()))?;
        progress(READ * (i + 1) as f32 / job.roms.len() as f32);
    }
    let read = began.elapsed().as_secs_f32();
    let extraction = nettai_extract::extract_reporting(job.game, &roms, &mut |f| progress(READ + (EXTRACTED - READ) * f)).map_err(|e| e.to_string())?;
    drop(roms);
    for warning in &extraction.warnings {
        crate::errors::log(&format!("extracting {}", job.id), warning);
    }
    let extracted = began.elapsed().as_secs_f32();
    replace(dir, &job.id, |new| {
        extraction.write_reporting(new, &mut |f| progress(EXTRACTED + (1.0 - EXTRACTED) * f)).map_err(|e| e.to_string())?;
        Stamp::of(job.game).write(new)
    })?;
    Ok([read, extracted - read, began.elapsed().as_secs_f32() - extracted])
}

/// What the extraction's thread is told by: a game's id, and what it has to say of it.
pub type Teller = Arc<dyn Fn(&str, Told) + Send + Sync>;

/// The games waiting for their packs, and whether the thread that extracts them is running.
#[derive(Default)]
struct Queue {
    jobs: VecDeque<Job>,
    running: bool,
}

/// The extraction: one thread, which extracts the games' packs one after the other, and ends when none is left.
pub struct Extractor {
    dir: PathBuf,
    queue: Arc<Mutex<Queue>>,
    told: Teller,
}

impl Extractor {
    /// An extraction into the packs folder `dir`, which tells `told` (on its own thread) how each game's goes.
    pub fn new(dir: PathBuf, told: Teller) -> Extractor {
        Extractor { dir, queue: Arc::default(), told }
    }

    /// Extract each of `jobs` after the games already waiting, starting the extraction's thread if it isn't
    /// running.
    pub fn add(&self, jobs: Vec<Job>) {
        let Ok(mut queue) = nettai_platform::thread::lock(&self.queue) else { return };
        queue.jobs.extend(jobs);
        if queue.running || queue.jobs.is_empty() {
            return;
        }
        queue.running = true;
        let (dir, waiting, told) = (self.dir.clone(), self.queue.clone(), self.told.clone());
        if let Err(why) = nettai_platform::thread::spawn("extracting", move || work(&dir, &waiting, &*told)) {
            crate::errors::log("starting the extraction", &why);
        }
    }

    /// Extract game `id` next, if it is waiting (the player stepped to it).
    pub fn first(&self, id: &str) {
        let Ok(mut queue) = nettai_platform::thread::lock(&self.queue) else { return };
        if let Some(at) = queue.jobs.iter().position(|j| j.id == id) {
            let job = queue.jobs.remove(at).expect("it is there");
            queue.jobs.push_front(job);
        }
    }
}

/// The extraction's thread: each game waiting in turn, till none is.
fn work(dir: &Path, queue: &Mutex<Queue>, told: &(dyn Fn(&str, Told) + Send + Sync)) {
    loop {
        let job = {
            let Ok(mut queue) = nettai_platform::thread::lock(queue) else { return };
            match queue.jobs.pop_front() {
                Some(job) => job,
                None => {
                    queue.running = false;
                    return;
                }
            }
        };
        // (Told when the percentage shown changes, not at each file written.)
        let mut shown = -1;
        let mut progress = |f: f32| {
            let percent = (f * 100.0) as i32;
            if percent != shown {
                shown = percent;
                told(&job.id, Told::Progress(f));
            }
        };
        let from = job.roms.first().map(|r| r.archive.parent().unwrap_or(&r.archive).display().to_string()).unwrap_or_default();
        let done = extract(dir, &job, &mut progress);
        match &done {
            Ok([read, extracted, written]) => crate::errors::log(
                &format!("extracting {} from {from}", job.id),
                &format_args!("done in {:.2} s (its {} ROMs read {read:.2} s, extracted {extracted:.2} s, the pack written and read back {written:.2} s)", read + extracted + written, job.roms.len()),
            ),
            Err(why) => crate::errors::log(&format!("extracting {} from {from}", job.id), why),
        }
        told(&job.id, Told::Done(done.map(|_| ())));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bnlc::tests::{archive, fake_rom};

    fn files(root: &Path) -> Vec<String> {
        let mut out = Vec::new();
        let mut dirs = vec![root.to_path_buf()];
        while let Some(dir) = dirs.pop() {
            for entry in nettai_platform::fs::read_dir(&dir).unwrap().flatten() {
                let path = entry.path();
                if nettai_platform::fs::is_dir(&path) {
                    dirs.push(path);
                } else {
                    out.push(path.strip_prefix(root).unwrap().to_string_lossy().into_owned());
                }
            }
        }
        out.sort();
        out
    }

    /// A pack this build made is current; one with another stamp, or none, is stale; no manifest is no pack.
    #[test]
    fn a_pack_is_current_by_its_stamp() {
        let dir = tempfile::tempdir().unwrap();
        let pack = dir.path().join("exe6");
        assert_eq!(kept(dir.path(), "exe6", Game::Exe6), Kept::None);
        nettai_platform::fs::create_dir_all(&pack).unwrap();
        nettai_platform::fs::write(pack.join(nettai_content::pack::MANIFEST), "").unwrap();
        assert_eq!(kept(dir.path(), "exe6", Game::Exe6), Kept::Stale, "no stamp");
        Stamp::of(Game::Exe6).write(&pack).unwrap();
        assert_eq!(kept(dir.path(), "exe6", Game::Exe6), Kept::Current);
        Stamp { nettai: "0.0.1".into(), ..Stamp::of(Game::Exe6) }.write(&pack).unwrap();
        assert_eq!(kept(dir.path(), "exe6", Game::Exe6), Kept::Stale, "another version");
        Stamp::of(Game::Exe5).write(&pack).unwrap();
        assert_eq!(kept(dir.path(), "exe6", Game::Exe6), Kept::Stale, "other asset names");
    }

    /// A pack made anew takes the old one's place once it is whole; one whose making fails leaves the old as it
    /// was; and nothing is left beside them.
    #[test]
    fn a_pack_is_replaced_once_it_is_whole() {
        let dir = tempfile::tempdir().unwrap();
        let write = |files: &'static [&'static str]| {
            move |new: &Path| -> Result<(), String> {
                assert!(!nettai_platform::fs::exists(&new) || nettai_platform::fs::read_dir(new).unwrap().next().is_none(), "a new, empty folder");
                for f in files {
                    nettai_content::pack::write_file(new, f, b"x").map_err(|e| e.to_string())?;
                }
                Ok(())
            }
        };
        replace(dir.path(), "exe6", write(&["content.toml", "graphics/a.png"])).unwrap();
        assert_eq!(files(dir.path()), ["exe6/content.toml", "exe6/graphics/a.png"]);
        replace(dir.path(), "exe6", write(&["content.toml", "sound/b.json"])).unwrap();
        assert_eq!(files(dir.path()), ["exe6/content.toml", "exe6/sound/b.json"], "nothing of the old one is left");
        assert_eq!(nettai_platform::fs::read_dir(dir.path()).unwrap().count(), 1, "nor the folders it was made and moved in");
        let failed = replace(dir.path(), "exe6", |new| {
            nettai_content::pack::write_file(new, "content.toml", b"half").unwrap();
            Err("the disk is full".to_string())
        });
        assert_eq!(failed, Err("the disk is full".to_string()));
        assert_eq!(files(dir.path()), ["exe6/content.toml", "exe6/sound/b.json"], "the old one, as it was");
        assert_eq!(nettai_platform::fs::read(dir.path().join("exe6/content.toml")).unwrap(), b"x");
    }

    /// What a crash leaves is tidied: a pack moved aside and not replaced is put back, one replaced is gone, and
    /// a half-written one goes.
    #[test]
    fn what_a_crash_leaves_is_tidied() {
        let dir = tempfile::tempdir().unwrap();
        let put = |path: &str| nettai_content::pack::write_file(dir.path(), path, b"x").unwrap();
        // exe6 was moved aside and the crash came before the new one was put in place; exe5's new one was.
        put(".old/exe6/content.toml");
        put(".old/exe5/content.toml");
        put("exe5/content.toml");
        put(".new/exe4/content.toml");
        tidy(dir.path());
        assert_eq!(files(dir.path()), ["exe5/content.toml", "exe6/content.toml"]);
        tidy(dir.path());
        assert_eq!(files(dir.path()), ["exe5/content.toml", "exe6/content.toml"]);
    }

    /// A game whose ROMs don't read or aren't whole isn't extracted: the failure is said, the old pack stays,
    /// and nothing is left half made. (Its ROMs are only moved here: fake bytes do, and the extractor refuses
    /// them for being no whole ROM.)
    #[test]
    fn an_extraction_that_fails_leaves_the_old_pack() {
        let (install, packs) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let data = install.path().join("exe/data");
        archive(&data.join("exe6.dat"), &[("exe6_rom_e.srl", &fake_rom("BR6E", 0x1000))]);
        let roms = crate::bnlc::Install::at(install.path()).unwrap().roms();
        nettai_content::pack::write_file(packs.path(), "exe6/content.toml", b"old").unwrap();
        let job = Job { id: "exe6".into(), game: Game::Exe6, roms };
        let mut said = Vec::new();
        let why = extract(packs.path(), &job, &mut |f| said.push(f)).unwrap_err();
        assert!(why.contains("8 MiB"), "{why}");
        assert_eq!(files(packs.path()), ["exe6/content.toml"]);
        // An archive gone since it was looked in.
        nettai_platform::fs::remove_file(data.join("exe6.dat")).unwrap();
        assert!(extract(packs.path(), &job, &mut |_| {}).unwrap_err().contains("exe6_rom_e.srl"));
    }

    /// The extraction's thread takes the games in turn, the one asked for first, tells each one's end, and stops
    /// when none is left; more given after it stopped start it again.
    #[test]
    fn the_extraction_takes_the_games_in_turn() {
        let (install, packs) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let data = install.path().join("exe/data");
        archive(&data.join("exe4.dat"), &[("exe4_rom_e.srl", &fake_rom("B4WE", 0x100))]);
        archive(&data.join("exe5.dat"), &[("exe5_rom_e.srl", &fake_rom("BRBE", 0x100))]);
        let roms = crate::bnlc::Install::at(install.path()).unwrap().roms();
        let job = |id: &str, game: Game| Job { id: id.into(), game, roms: roms.iter().filter(|r| r.game == game).cloned().collect() };
        let (send, heard) = std::sync::mpsc::channel::<(String, bool)>();
        let send = Mutex::new(send);
        let told: Teller = Arc::new(move |id, told| {
            if let Told::Done(result) = told {
                let _ = send.lock().unwrap().send((id.to_string(), result.is_ok()));
            }
        });
        let extractor = Extractor::new(packs.path().to_path_buf(), told);
        // (Both waiting before the thread starts, and the second asked for first.)
        extractor.queue.lock().unwrap().jobs.extend([job("exe4", Game::Exe4), job("exe5", Game::Exe5)]);
        extractor.first("exe5");
        extractor.add(Vec::new());
        let order: Vec<(String, bool)> = (0..2).map(|_| heard.recv_timeout(std::time::Duration::from_secs(30)).unwrap()).collect();
        assert_eq!(order, [("exe5".to_string(), false), ("exe4".to_string(), false)], "fake ROMs fail, in turn");
        while extractor.queue.lock().unwrap().running {
            std::thread::yield_now();
        }
        extractor.add(vec![job("exe4", Game::Exe4)]);
        assert_eq!(heard.recv_timeout(std::time::Duration::from_secs(30)).unwrap(), ("exe4".to_string(), false));
    }
}
