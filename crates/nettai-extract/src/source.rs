//! Where a game's ROMs are read from ([`RomSource`]): Battle Network Legacy Collection Vol. 2, Steam's release of
//! BN4, BN5 and BN6 (both versions of each, in English and Japanese), installed ([`Install`]); or ROM files, a
//! folder of them or each one named.
//!
//! The install is ported from Tango's `tango-library/src/bnlc.rs` (its `locate_app_dir`, `rom_archives` and
//! `scan_rom_archive`), Vol. 2 alone, with the crates it uses (`steamlocate`, `zip`). It is found through Steam by
//! the volume's app id; a folder chosen is looked in too, for an install Steam doesn't list. Its ROMs are the `.srl`
//! entries of the per-game `exeN.dat` zip archives under `<install>/exe/data/` (the shared `exe.dat` beside them
//! holds the collection's own art, and no ROM).
//!
//! Where `steamlocate` looks for Steam: on Windows the registry's `InstallPath` (`HKLM\SOFTWARE\Wow6432Node\Valve\
//! Steam`, else `SOFTWARE\Valve\Steam`); on macOS `~/Library/Application Support/Steam`; on Linux Flatpak's
//! (`~/.var/app/com.valvesoftware.Steam/.local/share/Steam`, `.steam/steam`, `.steam/root`), the usual
//! (`$XDG_DATA_HOME/Steam` or `~/.local/share/Steam`, `~/.steam/steam`, `~/.steam/root`,
//! `~/.steam/debian-installation`) and Snap's (`$SNAP_USER_DATA` or `~/snap`, then `steam/common/.local/share/
//! Steam`, `.steam/steam`, `.steam/root`), each that has a `steamapps` folder. Then Steam's libraries
//! (`steamapps/libraryfolders.vdf`) for the app's manifest (`appmanifest_1798020.acf`), whose install folder is
//! `<library>/steamapps/common/<installdir>`. (Tango's lock has steamlocate 2.0.1, which on Linux takes the first
//! of those folders that exists, with `~/.local/share/Steam` and `~/.steam` in the two's place; this is 2.1.1,
//! and every Steam found is looked in: `locate_all`.) The collection is a Windows game: Steam on Linux installs
//! it with Proton in its own libraries, where it is found; Steam on macOS doesn't install it, and a Windows Steam
//! in a compatibility layer (CrossOver, Whisky) keeps it in a folder the player chooses.
//!
//! The ROMs are read into memory and extracted there; none is written to disk. Which game a ROM is of is its
//! content's to say: each game's recipe declares the game codes of its ROMs ([`crate::Recipe::roms`]), and a ROM is
//! found by the code in its header ([`crate::header_code`]). Nothing here names a game.
//!
//! The tools and development name the ROMs one way ([`RomSource::from_env`]): `$NETTAI_ROMS`, a folder that is
//! the collection's install or holds the ROM files; unset, the collection where Steam installed it.

use crate::{Error, Recipe, RomSet};
use std::fmt;
use std::io::Read;
use std::path::{Path, PathBuf};

/// The environment variable that names the ROMs for the tools and development ([`RomSource::from_env`]).
pub const ROMS: &str = "NETTAI_ROMS";

/// The per-game archives of a folder chosen as the install, at the three places [`Install::at`] looks: the
/// browser reads them, and no more, from the folder chosen (`nettai_platform::dialog::open_folder`). (`exe?*`: not
/// the shared `exe.dat`, which holds no ROM and is large.)
pub const ARCHIVES: [&str; 3] = ["exe/data/exe?*.dat", "data/exe?*.dat", "exe?*.dat"];

/// Vol. 2's Steam app id (Tango's `Volume::Vol2`).
#[cfg(not(target_arch = "wasm32"))]
const STEAM_APP_ID: u32 = 1798020;

/// An install of the collection: the folder its archives are in, and the per-game archives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Install {
    /// `<install>/exe/data`.
    pub data: PathBuf,
    pub archives: Vec<PathBuf>,
}

impl Install {
    /// The install: in `chosen` (a folder the player chose) if it holds one, else where Steam has it. None:
    /// neither has.
    pub fn find(chosen: Option<&Path>) -> Option<Install> {
        chosen.and_then(Install::at).or_else(|| steam().and_then(|dir| Install::at(&dir)))
    }

    /// The install in `folder`: the collection's own folder (the one with `exe/` in it), or its `exe` or
    /// `exe/data` folder, whichever was chosen. None: no per-game archive is there.
    pub fn at(folder: &Path) -> Option<Install> {
        [folder.join("exe").join("data"), folder.join("data"), folder.to_path_buf()].into_iter().find_map(|data| {
            let archives = archives(&data);
            (!archives.is_empty()).then_some(Install { data, archives })
        })
    }
}

/// Where a game's ROMs are read from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RomSource {
    /// The Legacy Collection installed: the ROMs in its archives.
    Collection(Install),
    /// ROM files (a folder's, or each one named): each one's header says which ROM it is, and a file that is none
    /// asked for is passed over.
    Files(Vec<PathBuf>),
}

/// A ROM of a [`RomSource`]: where it is, and the game code its header says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rom {
    pub code: String,
    pub at: RomAt,
}

/// Where a [`Rom`] is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RomAt {
    /// An entry of one of the collection's archives: its place in the archive, and its name there.
    Entry { archive: PathBuf, entry: usize, name: String },
    /// A file of its own.
    File(PathBuf),
}

impl fmt::Display for RomAt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RomAt::Entry { archive, name, .. } => write!(f, "{name} in {}", archive.display()),
            RomAt::File(path) => write!(f, "{}", path.display()),
        }
    }
}

impl fmt::Display for RomSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RomSource::Collection(install) => write!(f, "the Legacy Collection in {}", install.data.display()),
            RomSource::Files(files) => match common_folder(files) {
                Some(dir) => write!(f, "the ROM files in {}", dir.display()),
                None => write!(f, "the ROM files given"),
            },
        }
    }
}

/// The folder every one of `files` is in, if they share one.
fn common_folder(files: &[PathBuf]) -> Option<&Path> {
    let first = files.first()?.parent()?;
    files.iter().all(|f| f.parent() == Some(first)).then_some(first)
}

impl RomSource {
    /// The ROMs in `folder`: the Legacy Collection installed there (its own folder, its `exe` or its `exe/data`:
    /// [`Install::at`]), else the files in it whose headers have a game code (each read only as far as its
    /// header). An error: the folder can't be read, or it is neither and holds no ROM.
    pub fn at(folder: &Path) -> Result<RomSource, Error> {
        if let Some(install) = Install::at(folder) {
            return Ok(RomSource::Collection(install));
        }
        let read = nettai_platform::fs::read_dir(folder).map_err(|e| Error(format!("{}: {e}", folder.display())))?;
        let mut files: Vec<PathBuf> = read
            .flatten()
            .map(|e| e.path())
            .filter(|p| nettai_platform::fs::is_file(p) && file_code(p).is_ok_and(|c| c.is_some()))
            .collect();
        if files.is_empty() {
            return Err(Error(format!(
                "{} has no ROM: it is neither the Legacy Collection's folder (its own, its exe or its exe/data) nor a folder of ROM files",
                folder.display()
            )));
        }
        files.sort();
        Ok(RomSource::Files(files))
    }

    /// The ROMs the tools and development read: `$NETTAI_ROMS` ([`ROMS`]), a folder as [`RomSource::at`] takes
    /// it; unset, the Legacy Collection where Steam installed it. An error says how to name them.
    pub fn from_env() -> Result<RomSource, Error> {
        match std::env::var_os(ROMS) {
            Some(dir) => RomSource::at(Path::new(&dir)).map_err(|e| Error(format!("${ROMS}: {e}"))),
            None => steam().and_then(|dir| Install::at(&dir)).map(RomSource::Collection).ok_or_else(|| {
                Error(format!(
                    "no ROMs: ${ROMS} isn't set, and Steam has no Battle Network Legacy Collection Vol. 2 installed; set ${ROMS} to the collection's folder, or to a folder of its ROMs"
                ))
            }),
        }
    }

    /// The ROMs of the game codes `codes` (a game's recipe's: every one required), in their order, as each one's
    /// header says: the first of each code (a copy of one is passed over). An archive or a file that doesn't read
    /// is passed over (and said in the log: the `log` crate's). Err: the codes no ROM is of, with the ROMs found.
    pub fn find(&self, codes: &[String]) -> Result<Vec<Rom>, (Vec<String>, Vec<Rom>)> {
        let mut found: Vec<Rom> = Vec::new();
        let mut keep = |rom: Rom| {
            if !found.iter().any(|r| r.code == rom.code) {
                found.push(rom);
            }
        };
        match self {
            RomSource::Collection(install) => {
                for archive in &install.archives {
                    match scan(archive, codes) {
                        Ok(roms) => roms.into_iter().for_each(&mut keep),
                        Err(why) => log::warn!("reading {}: {why}", archive.display()),
                    }
                }
            }
            RomSource::Files(files) => {
                for file in files {
                    match file_code(file) {
                        Ok(Some(code)) if codes.contains(&code) => keep(Rom { code, at: RomAt::File(file.clone()) }),
                        Ok(_) => {}
                        Err(why) => log::warn!("reading {}: {why}", file.display()),
                    }
                }
            }
        }
        found.sort_by_key(|r| codes.iter().position(|c| *c == r.code));
        let missing: Vec<String> = codes.iter().filter(|c| !found.iter().any(|r| r.code == **c)).cloned().collect();
        if missing.is_empty() { Ok(found) } else { Err((missing, found)) }
    }

    /// Every ROM `recipe` declares, read whole into a set for it ([`Recipe::extract`]). An error names the ROMs
    /// missing (the source hasn't the whole collection's), or the one that doesn't read.
    pub fn read(&self, recipe: &Recipe) -> Result<RomSet, Error> {
        let roms = self.find(recipe.roms()).map_err(|(missing, _)| {
            Error(format!(
                "{}: {} missing from {self} (the game's ROMs are {}: every one is needed)",
                recipe.game(),
                missing.join(", "),
                recipe.roms().join(", ")
            ))
        })?;
        let mut set = recipe.rom_set();
        for rom in &roms {
            set.insert(rom.read()?).map_err(|e| Error(format!("{}: {e}", rom.at)))?;
        }
        Ok(set)
    }
}

impl Rom {
    /// The ROM's bytes, read into memory (out of its archive, for one of the collection's).
    pub fn read(&self) -> Result<Vec<u8>, Error> {
        let said = |e: &dyn fmt::Display| Error(format!("{}: {e}", self.at));
        match &self.at {
            RomAt::Entry { archive, entry, .. } => {
                let mut zip = open(archive).map_err(|e| said(&e))?;
                let mut file = zip.by_index(*entry).map_err(|e| said(&e))?;
                let mut bytes = Vec::with_capacity(file.size() as usize);
                file.read_to_end(&mut bytes).map_err(|e| said(&e))?;
                Ok(bytes)
            }
            RomAt::File(path) => nettai_platform::fs::read(path).map_err(|e| said(&e)),
        }
    }
}

/// The collection's folder as Steam has it installed, if it is: in the first Steam found that has it. (No Steam
/// is nothing gone wrong.)
fn steam() -> Option<PathBuf> {
    // (The browser can't look for Steam: the player chooses the folder.)
    #[cfg(target_arch = "wasm32")]
    return None;
    #[cfg(not(target_arch = "wasm32"))]
    steamlocate::locate_all().ok()?.into_iter().find_map(|steam| {
        let (app, library) = steam.find_app(STEAM_APP_ID).map_err(|e| log::warn!("finding the Legacy Collection in Steam: {e}")).ok().flatten()?;
        Some(library.resolve_app_dir(&app))
    })
}

/// The per-game archives in an install's data folder: each `exe*.dat` but the shared `exe.dat`.
fn archives(data: &Path) -> Vec<PathBuf> {
    let Ok(read) = nettai_platform::fs::read_dir(data) else { return Vec::new() };
    let mut out: Vec<PathBuf> = read
        .flatten()
        .map(|entry| entry.path())
        .filter(|p| {
            let name = p.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
            name != "exe.dat" && name.starts_with("exe") && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("dat")) && nettai_platform::fs::is_file(p)
        })
        .collect();
    out.sort();
    out
}

/// The ROMs of the game codes `codes` in one archive, by their headers: each `.srl` entry's first bytes read, no
/// more.
fn scan(archive: &Path, codes: &[String]) -> Result<Vec<Rom>, String> {
    let mut zip = open(archive)?;
    let mut out = Vec::new();
    for entry in 0..zip.len() {
        let mut file = zip.by_index(entry).map_err(|e| format!("entry {entry}: {e}"))?;
        let Some(name) = file.enclosed_name() else { continue };
        if !name.extension().is_some_and(|e| e.eq_ignore_ascii_case("srl")) || !file.is_file() {
            continue;
        }
        let name = name.to_string_lossy().into_owned();
        let mut header = Vec::with_capacity(crate::HEADER_LEN);
        (&mut file).take(crate::HEADER_LEN as u64).read_to_end(&mut header).map_err(|e| format!("{name}: {e}"))?;
        if let Some(code) = crate::header_code(&header).filter(|c| codes.iter().any(|d| d == c)) {
            out.push(Rom { code: code.to_string(), at: RomAt::Entry { archive: archive.to_path_buf(), entry, name } });
        }
    }
    Ok(out)
}

/// The game code a file's header says, reading no more than the header: none for a file too short to have one,
/// or with no game code there.
fn file_code(path: &Path) -> Result<Option<String>, String> {
    let file = nettai_platform::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut header = Vec::with_capacity(crate::HEADER_LEN);
    file.take(crate::HEADER_LEN as u64).read_to_end(&mut header).map_err(|e| e.to_string())?;
    Ok(crate::header_code(&header).map(str::to_string))
}

fn open(archive: &Path) -> Result<zip::ZipArchive<nettai_platform::fs::File>, String> {
    let file = nettai_platform::fs::File::open(archive).map_err(|e| e.to_string())?;
    zip::ZipArchive::new(file).map_err(|e| e.to_string())
}

/// What tests of a source are made from: ROMs and archives of made-up bytes.
#[cfg(any(test, feature = "testing"))]
pub mod testing {
    use std::io::Write;
    use std::path::Path;

    /// A ROM's bytes as far as the code under test looks: `len` bytes, its header's game code `code`, and the
    /// rest a pattern.
    pub fn fake_rom(code: &str, len: usize) -> Vec<u8> {
        let mut rom: Vec<u8> = (0..len).map(|i| (i * 7 % 251) as u8).collect();
        rom[0xac..0xb0].copy_from_slice(code.as_bytes());
        rom
    }

    /// A zip archive at `path` of `entries` (a name and its bytes), compressed as the collection's may be.
    pub fn archive(path: &Path, entries: &[(&str, &[u8])]) {
        nettai_platform::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut zip = zip::ZipWriter::new(nettai_platform::fs::File::create(path).unwrap());
        for (name, bytes) in entries {
            zip.start_file(*name, zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated)).unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::testing::{archive, fake_rom};
    use super::*;

    fn codes(c: &[&str]) -> Vec<String> {
        c.iter().map(|c| c.to_string()).collect()
    }

    /// An install is found from its own folder, its `exe` folder or its `exe/data`; its per-game archives are
    /// listed and the shared one isn't; and a folder of none is no install.
    #[test]
    fn an_install_is_found_from_any_of_its_folders() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("exe").join("data");
        archive(&data.join("exe.dat"), &[("bg/a.tga", b"art")]);
        archive(&data.join("exe6.dat"), &[]);
        archive(&data.join("exe4.dat"), &[]);
        nettai_platform::fs::write(data.join("notes.txt"), b"").unwrap();
        let install = Install { data: data.clone(), archives: vec![data.join("exe4.dat"), data.join("exe6.dat")] };
        assert_eq!(Install::at(dir.path()).as_ref(), Some(&install));
        assert_eq!(Install::at(&dir.path().join("exe")).as_ref(), Some(&install));
        assert_eq!(Install::at(&data).as_ref(), Some(&install));
        assert_eq!(Install::find(Some(dir.path())), Some(install.clone()));
        assert_eq!(RomSource::at(&data).unwrap(), RomSource::Collection(install));
        let empty = tempfile::tempdir().unwrap();
        assert_eq!(Install::at(empty.path()), None);
        assert_eq!(Install::at(&empty.path().join("missing")), None);
        assert!(RomSource::at(empty.path()).unwrap_err().to_string().contains("has no ROM"));
        assert!(RomSource::at(&empty.path().join("missing")).is_err());
    }

    /// The ROMs are the archives' `.srl` entries of the codes asked for, by their headers, wherever in an archive
    /// they are, in the codes' order; anything else is passed over, and so is a second copy of one game code, and
    /// an archive that isn't one. Codes no archive has are said, with the ROMs found. Each reads back whole.
    #[test]
    fn the_roms_are_found_by_their_headers_and_read_whole() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("exe").join("data");
        let (us, jp, other) = (fake_rom("BR6E", 0x1000), fake_rom("BR5J", 0x2000), fake_rom("AREE", 0x1000));
        archive(&data.join("exe6.dat"), &[("exe6_rom_e.srl", &us), ("readme.txt", b"BR6E"), ("roms/exe6f_rom_f.srl", &jp), ("exe1_rom_e.srl", &other)]);
        archive(&data.join("exe7.dat"), &[("again/exe6_rom_e.srl", &us)]);
        nettai_platform::fs::write(data.join("exe5.dat"), b"not a zip").unwrap();
        let RomSource::Collection(install) = RomSource::at(dir.path()).unwrap() else { panic!("an install") };
        assert_eq!(install.archives.len(), 3);
        let source = RomSource::Collection(install);
        let roms = source.find(&codes(&["BR5J", "BR6E"])).unwrap();
        let found: Vec<(String, &str)> = roms.iter().map(|r| (r.at.to_string(), r.code.as_str())).collect();
        let at = |name: &str| format!("{name} in {}", data.join("exe6.dat").display());
        assert_eq!(found, [(at("roms/exe6f_rom_f.srl"), "BR5J"), (at("exe6_rom_e.srl"), "BR6E")]);
        let (missing, some) = source.find(&codes(&["BR6E", "BR5E", "BR6J", "BR5J"])).unwrap_err();
        assert_eq!((missing, some.len()), (codes(&["BR5E", "BR6J"]), 2));
        assert_eq!(roms[1].read().unwrap(), us);
        assert_eq!(roms[0].read().unwrap(), jp);
        // An archive gone since: a failure to read, said in words for the log.
        nettai_platform::fs::remove_file(data.join("exe6.dat")).unwrap();
        assert!(roms[1].read().unwrap_err().to_string().contains("exe6_rom_e.srl"));
    }

    /// A folder of ROM files: each file whose header is of a code asked for, whatever its name; a file that is
    /// no ROM, or another game's, is passed over, and so is a second copy.
    #[test]
    fn a_folder_of_rom_files() {
        let dir = tempfile::tempdir().unwrap();
        let (us, jp) = (fake_rom("BR6E", 0x1000), fake_rom("BR5J", 0x1000));
        nettai_platform::fs::write(dir.path().join("b.gba"), &us).unwrap();
        nettai_platform::fs::write(dir.path().join("c.srl"), &us).unwrap();
        nettai_platform::fs::write(dir.path().join("a.srl"), &jp).unwrap();
        nettai_platform::fs::write(dir.path().join("game.sav"), [0u8; 0x40]).unwrap();
        nettai_platform::fs::write(dir.path().join("other.gba"), fake_rom("AREE", 0x1000)).unwrap();
        nettai_platform::fs::create_dir_all(dir.path().join("sub")).unwrap();
        let source = RomSource::at(dir.path()).unwrap();
        assert!(matches!(&source, RomSource::Files(f) if f.len() == 4), "the save isn't one: {source:?}");
        assert_eq!(source.to_string(), format!("the ROM files in {}", dir.path().display()));
        let roms = source.find(&codes(&["BR6E", "BR5J"])).unwrap();
        assert_eq!(roms.iter().map(|r| (r.code.as_str(), r.at.clone())).collect::<Vec<_>>(), [
            ("BR6E", RomAt::File(dir.path().join("b.gba"))),
            ("BR5J", RomAt::File(dir.path().join("a.srl")))
        ]);
        assert_eq!(roms[0].read().unwrap(), us);
        assert_eq!(source.find(&codes(&["BR6E", "BR5E"])).unwrap_err().0, codes(&["BR5E"]));
    }
}
