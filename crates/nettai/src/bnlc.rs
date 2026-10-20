//! Battle Network Legacy Collection Vol. 2, Steam's release of BN4, BN5 and BN6 (both versions of each, in
//! English and Japanese): where it is installed, and the ROMs in its archives (docs/app.md §17).
//!
//! Ported from Tango's `tango-library/src/bnlc.rs` (its `locate_app_dir`, `rom_archives` and
//! `scan_rom_archive`), Vol. 2 alone, with the crates it uses (`steamlocate`, `zip`). The install is found
//! through Steam by the volume's app id; a folder the player chose is looked in too, for an install Steam doesn't
//! list. Its ROMs are the `.srl` entries of the per-game `exeN.dat` zip archives under `<install>/exe/data/` (the
//! shared `exe.dat` beside them holds the collection's own art, and no ROM).
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
//! The ROMs are read into memory and handed to the extractor; none is written to disk. Which game a ROM is of is
//! the extractor's to say, from its header (`nettai_extract::recognize`): the app names no game here.

use nettai_extract::Game;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Vol. 2's Steam app id (Tango's `Volume::Vol2`).
const STEAM_APP_ID: u32 = 1798020;

/// An install of the collection: the folder its archives are in, and the per-game archives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Install {
    /// `<install>/exe/data`.
    pub data: PathBuf,
    pub archives: Vec<PathBuf>,
}

/// A ROM in one of an install's archives: where it is, and the game and the game code its header says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rom {
    pub archive: PathBuf,
    /// Its entry's place in the archive, and its name there.
    pub entry: usize,
    pub name: String,
    pub game: Game,
    pub code: &'static str,
}

impl Install {
    /// The install: in `chosen` (the folder the player chose, from the settings) if it holds one, else where
    /// Steam has it. None: neither has.
    pub fn find(chosen: Option<&Path>) -> Option<Install> {
        chosen.and_then(Install::at).or_else(|| steam().and_then(|dir| Install::at(&dir)))
    }

    /// The install in `folder`: the collection's own folder (the one with `exe/` in it), or its `exe` or
    /// `exe/data` folder, whichever the player chose. None: no per-game archive is there.
    pub fn at(folder: &Path) -> Option<Install> {
        [folder.join("exe").join("data"), folder.join("data"), folder.to_path_buf()].into_iter().find_map(|data| {
            let archives = archives(&data);
            (!archives.is_empty()).then_some(Install { data, archives })
        })
    }

    /// Every ROM of a game the extractor takes, in every archive, as its header says: the first of each game
    /// code (a copy of one is passed over). An archive that doesn't read is passed over, and said in the log.
    pub fn roms(&self) -> Vec<Rom> {
        let mut out: Vec<Rom> = Vec::new();
        for archive in &self.archives {
            match scan(archive) {
                Ok(roms) => {
                    for rom in roms {
                        if !out.iter().any(|r| r.code == rom.code) {
                            out.push(rom);
                        }
                    }
                }
                Err(why) => crate::errors::log(&format!("reading {}", archive.display()), &why),
            }
        }
        out
    }
}

/// The collection's folder as Steam has it installed, if it is: in the first Steam found that has it. (No Steam
/// is nothing gone wrong.)
fn steam() -> Option<PathBuf> {
    steamlocate::locate_all().ok()?.into_iter().find_map(|steam| {
        let (app, library) = steam.find_app(STEAM_APP_ID).map_err(|e| crate::errors::log("finding the Legacy Collection in Steam", &e)).ok().flatten()?;
        Some(library.resolve_app_dir(&app))
    })
}

/// The per-game archives in an install's data folder: each `exe*.dat` but the shared `exe.dat`.
fn archives(data: &Path) -> Vec<PathBuf> {
    let Ok(read) = nettai_fs::read_dir(data) else { return Vec::new() };
    let mut out: Vec<PathBuf> = read
        .flatten()
        .map(|entry| entry.path())
        .filter(|p| {
            let name = p.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
            name != "exe.dat" && name.starts_with("exe") && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("dat")) && nettai_fs::is_file(p)
        })
        .collect();
    out.sort();
    out
}

/// The ROMs in one archive, by their headers: each `.srl` entry's first bytes read, no more.
fn scan(archive: &Path) -> Result<Vec<Rom>, String> {
    let mut zip = open(archive)?;
    let mut out = Vec::new();
    for entry in 0..zip.len() {
        let mut file = zip.by_index(entry).map_err(|e| format!("entry {entry}: {e}"))?;
        let Some(name) = file.enclosed_name() else { continue };
        if !name.extension().is_some_and(|e| e.eq_ignore_ascii_case("srl")) || !file.is_file() {
            continue;
        }
        let name = name.to_string_lossy().into_owned();
        let mut header = Vec::with_capacity(nettai_extract::HEADER_LEN);
        (&mut file).take(nettai_extract::HEADER_LEN as u64).read_to_end(&mut header).map_err(|e| format!("{name}: {e}"))?;
        if let Some((game, code)) = nettai_extract::recognize(&header) {
            out.push(Rom { archive: archive.to_path_buf(), entry, name, game, code });
        }
    }
    Ok(out)
}

fn open(archive: &Path) -> Result<zip::ZipArchive<nettai_fs::File>, String> {
    let file = nettai_fs::File::open(archive).map_err(|e| e.to_string())?;
    zip::ZipArchive::new(file).map_err(|e| e.to_string())
}

/// The ROM's bytes, read out of its archive into memory.
pub fn read(rom: &Rom) -> Result<Vec<u8>, String> {
    let said = |e: &dyn std::fmt::Display| format!("{} in {}: {e}", rom.name, rom.archive.display());
    let mut zip = open(&rom.archive).map_err(|e| said(&e))?;
    let mut file = zip.by_index(rom.entry).map_err(|e| said(&e))?;
    let mut bytes = Vec::with_capacity(file.size() as usize);
    file.read_to_end(&mut bytes).map_err(|e| said(&e))?;
    Ok(bytes)
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::io::Write;

    /// A ROM's bytes as far as the code under test looks: `len` bytes, its header's game code `code`, and the
    /// rest a pattern.
    pub fn fake_rom(code: &str, len: usize) -> Vec<u8> {
        let mut rom: Vec<u8> = (0..len).map(|i| (i * 7 % 251) as u8).collect();
        rom[0xac..0xb0].copy_from_slice(code.as_bytes());
        rom
    }

    /// A zip archive at `path` of `entries` (a name and its bytes), compressed as the collection's may be.
    pub fn archive(path: &Path, entries: &[(&str, &[u8])]) {
        nettai_fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut zip = zip::ZipWriter::new(nettai_fs::File::create(path).unwrap());
        for (name, bytes) in entries {
            zip.start_file(*name, zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated)).unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap();
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
        nettai_fs::write(data.join("notes.txt"), b"").unwrap();
        let install = Install { data: data.clone(), archives: vec![data.join("exe4.dat"), data.join("exe6.dat")] };
        assert_eq!(Install::at(dir.path()).as_ref(), Some(&install));
        assert_eq!(Install::at(&dir.path().join("exe")).as_ref(), Some(&install));
        assert_eq!(Install::at(&data).as_ref(), Some(&install));
        assert_eq!(Install::find(Some(dir.path())), Some(install));
        let empty = tempfile::tempdir().unwrap();
        assert_eq!(Install::at(empty.path()), None);
        assert_eq!(Install::at(&empty.path().join("missing")), None);
    }

    /// The ROMs are the archives' `.srl` entries a game's, by their headers, wherever in an archive they are;
    /// anything else is passed over, and so is a second copy of one game code, and an archive that isn't one.
    /// Each reads back whole.
    #[test]
    fn the_roms_are_found_by_their_headers_and_read_whole() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("exe").join("data");
        let (us, jp, other) = (fake_rom("BR6E", 0x1000), fake_rom("BR5J", 0x2000), fake_rom("AREE", 0x1000));
        archive(&data.join("exe6.dat"), &[("exe6_rom_e.srl", &us), ("readme.txt", b"BR6E"), ("roms/exe6f_rom_f.srl", &jp), ("exe1_rom_e.srl", &other)]);
        archive(&data.join("exe7.dat"), &[("again/exe6_rom_e.srl", &us)]);
        nettai_fs::write(data.join("exe5.dat"), b"not a zip").unwrap();
        let install = Install::at(dir.path()).unwrap();
        assert_eq!(install.archives.len(), 3);
        let roms = install.roms();
        let found: Vec<(&str, Game, &str)> = roms.iter().map(|r| (r.name.as_str(), r.game, r.code)).collect();
        assert_eq!(found, [("exe6_rom_e.srl", Game::Exe6, "BR6E"), ("roms/exe6f_rom_f.srl", Game::Exe6, "BR5J")]);
        assert_eq!(read(&roms[0]).unwrap(), us);
        assert_eq!(read(&roms[1]).unwrap(), jp);
        // An archive gone since: a failure to read, said in words for the log.
        nettai_fs::remove_file(data.join("exe6.dat")).unwrap();
        assert!(read(&roms[0]).unwrap_err().contains("exe6_rom_e.srl"));
    }
}
