//! Custom assets (docs/design/asset-formats.md §7): what a game's content makes from files of its own rather than
//! its ROMs (`Content::custom_assets`, which its modules define: `asset.background("my-stage", { file =
//! "./my_stage.png" })`). Each is made into the model the ROMs' assets are extracted into and added to the game's
//! assets after them, under its name, numbered past the ROMs' ([`add`]); a frontend then draws it as it draws theirs.
//! One with the name of a ROM asset stands in its place: the name is the custom one's (another name the ROM asset has
//! keeps it).
//! They are GBA-shaped, as the ROMs' are: pictures indexed, 16 colors to a palette, cut into 8x8 tiles; songs the M4A
//! driver's, from MIDI files, with instruments of their own from WAV recordings.

mod background;
mod picture;
mod song;
mod sprite;

pub use background::BACKGROUND_FIRST_TILE;

use nettai_assets::Bundle;
use nettai_content_api::{AssetIndex, AssetKind, SpriteNumber, keys, packs};

/// Add content `c`'s custom assets to its game's assets as its ROMs gave them: their graphics `graphics`, their sound
/// `sound` and their index `index`, each at a number of its own past theirs, under its name (one the ROMs have too is
/// the custom one's now). An error names the asset and what is wrong with it: its file isn't there or isn't what its
/// kind takes.
pub fn add(c: &nettai_battle::Content, graphics: &mut Bundle, sound: &mut m4a::SoundBank, index: &mut AssetIndex) -> Result<(), String> {
    let dirs = &c.scripts.dirs.0;
    // (A custom song plays on a ROM song's instruments: the ROMs' sounds, before any custom one is added.)
    let rom_sounds = index.sounds.clone();
    for (name, def) in &c.custom_assets.sounds {
        let at = format!("{}.luau: sound {name:?}", keys::module_path(&def.module));
        let like = rom_sounds
            .get(&def.voices)
            .and_then(|&n| sound.song(m4a::SongId(n)))
            .cloned()
            .ok_or_else(|| format!("{at}: its `voices`, {:?}, is no song the ROMs have", def.voices))?;
        let max_tracks = sound.players.get(usize::from(like.player.0)).map_or(0, |p| usize::from(p.max_tracks));
        let mut song = read(dirs, &def.file).and_then(|bytes| song::song(&bytes, &like, max_tracks)).map_err(|e| format!("{at}: {}: {e}", def.file))?;
        // Its own instruments: a copy of the voicegroup, those programs its recordings (a program past the
        // voicegroup's end gets silent ones before it).
        if !def.instruments.is_empty() {
            let mut group = sound.voicegroups[usize::from(like.voicegroup.0)].clone();
            for (&program, inst) in &def.instruments {
                let made = read(dirs, &inst.file).and_then(|bytes| song::sample(&bytes, inst)).map_err(|e| format!("{at}: instrument {program}: {}: {e}", inst.file))?;
                let id = m4a::bank::SampleId(u16::try_from(sound.samples.len()).map_err(|_| format!("{at}: a game has at most 65536 samples"))?);
                sound.samples.push(made);
                let p = usize::from(program);
                if group.voices.len() <= p {
                    let silent = m4a::bank::Voice { kind: m4a::bank::VoiceKind::Silent, key: 60, pan: None, length: 0, envelope: Default::default() };
                    group.voices.resize(p + 1, silent);
                }
                group.voices[p] = song::voice(id, inst);
            }
            song.voicegroup = m4a::bank::VoicegroupId(u16::try_from(sound.voicegroups.len()).map_err(|_| format!("{at}: a game has at most 65536 voicegroups"))?);
            sound.voicegroups.push(group);
        }
        let n = u16::try_from(sound.songs.len()).map_err(|_| format!("{at}: a game has at most 65536 songs"))?;
        sound.songs.push(Some(song));
        index.sounds.insert(name.clone(), n);
    }
    if !c.custom_assets.sounds.is_empty() {
        sound.validate().map_err(|e| format!("the sound with the content's songs: {e}"))?;
    }
    for (name, def) in &c.custom_assets.backgrounds {
        let at = format!("{}.luau: background {name:?}", keys::module_path(&def.module));
        let bg = read(dirs, &def.file).and_then(|bytes| background::background(&bytes, def)).map_err(|e| format!("{at}: {}: {e}", def.file))?;
        let id = u8::try_from(graphics.backgrounds.len()).map_err(|_| format!("{at}: a game has at most 256 backgrounds"))?;
        graphics.backgrounds.push(Some(bg));
        index.backgrounds.insert(name.clone(), id);
    }
    // (Numbered from the category past the ROMs' last, index 0 on, in their names' order: the sheets stay in their
    // numbers' order.)
    let mut next = match graphics.sprites.iter().map(|s| s.category).max() {
        None => Some((0u8, 0u8)),
        Some(last) => last.checked_add(1).map(|c| (c, 0)),
    };
    for (name, def) in &c.custom_assets.sprites {
        let at = format!("{}.luau: sprite {name:?}", keys::module_path(&def.module));
        let frames: Vec<usize> = c
            .assets
            .handle(AssetKind::Sprite, name)
            .and_then(|h| c.animations.sprites.get(h as usize))
            .map(|d| d.animations.iter().map(|a| a.frames.len()).collect())
            .unwrap_or_default();
        let shadow = def.shadow.as_ref().map(|f| read(dirs, f).map_err(|e| format!("{at}: {f}: {e}"))).transpose()?;
        let mut sheet =
            read(dirs, &def.file).and_then(|bytes| sprite::sprite(&bytes, shadow.as_deref(), def, &frames)).map_err(|e| format!("{at}: {}: {e}", def.file))?;
        let (category, n) = next.ok_or_else(|| format!("{at}: no sprite numbers are left past the ROMs'"))?;
        (sheet.category, sheet.index) = (category, n);
        graphics.sprites.push(sheet);
        index.sprites.insert(name.clone(), SpriteNumber { category, index: n });
        next = match n.checked_add(1) {
            Some(n) => Some((category, n)),
            None => category.checked_add(1).map(|c| (c, 0)),
        };
    }
    Ok(())
}

/// File `file` of the content's packs (`exe6:backgrounds/my_stage.png`), read.
fn read(dirs: &packs::Dirs, file: &str) -> Result<Vec<u8>, String> {
    let path = dirs.asset_file(file).ok_or_else(|| format!("the content has no folder for its pack {:?}", keys::root_of(file).unwrap_or(file)))?;
    nettai_files::read(&path).map_err(|e| format!("{e}"))
}

/// An indexed PNG of `width` x `height` pixels of `bits` bits each, the color numbers `pixels` (row by row), its
/// palette `colors` (the tests').
#[cfg(test)]
pub(crate) fn indexed_png(width: u32, height: u32, bits: png::BitDepth, pixels: &[u8], colors: &[[u8; 3]]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut enc = png::Encoder::new(&mut out, width, height);
    enc.set_color(png::ColorType::Indexed);
    enc.set_depth(bits);
    enc.set_palette(colors.concat());
    let per_byte = 8 / bits as usize;
    let line = (width as usize).div_ceil(per_byte);
    let mut data = vec![0u8; line * height as usize];
    for (i, &p) in pixels.iter().enumerate() {
        let (x, y) = (i % width as usize, i / width as usize);
        data[y * line + x / per_byte] |= p << (8 - bits as usize * (x % per_byte + 1));
    }
    enc.write_header().unwrap().write_image_data(&data).unwrap();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use background::tests::{def, mirrored};
    use nettai_assets::Background;

    /// A sound bank with no songs.
    fn sound() -> m4a::SoundBank {
        let mixer = m4a::bank::MixerConfig { mix_rate: 13379, ds_channels: 8, master_volume: 15, reverb: 0, dac_resolution: 1 };
        m4a::SoundBank { mixer, players: Vec::new(), songs: Vec::new(), voicegroups: Vec::new(), key_maps: Vec::new(), samples: Vec::new(), waves: Vec::new() }
    }

    /// A content's custom backgrounds come after its ROMs', each numbered past them under its name, read from its
    /// pack's folder; one with a name the ROMs have stands in place of theirs (another name of theirs keeps it); a
    /// file that isn't there is said with the module that defines it.
    #[test]
    fn custom_backgrounds_come_after_the_roms() {
        let dir = std::env::temp_dir().join(format!("nettai-custom-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("backgrounds")).unwrap();
        std::fs::write(dir.join("backgrounds/x.png"), indexed_png(16, 8, png::BitDepth::Four, &mirrored(), &[[0, 0, 0]; 5])).unwrap();
        let mut c = nettai_battle::Content::default();
        c.scripts.dirs.0 = packs::Dirs([("g".to_string(), dir.clone())].into());
        c.custom_assets.backgrounds.insert("sky".into(), def());
        let mut graphics = Bundle { backgrounds: vec![Some(Background::default()), None], ..Default::default() };
        let mut index = AssetIndex::default();
        index.backgrounds.insert("comp".into(), 0);
        add(&c, &mut graphics, &mut sound(), &mut index).unwrap();
        assert_eq!(index.backgrounds["sky"], 2, "past the ROMs' numbers, holes too");
        assert_eq!(graphics.background(2).map(|b| (b.map_width, b.first_tile)), Some((2, BACKGROUND_FIRST_TILE)));
        // A name the ROMs have: the custom one's now, the ROMs' picture kept for their other name.
        let mut graphics = Bundle { backgrounds: vec![Some(Background::default())], ..Default::default() };
        let mut index = AssetIndex::default();
        index.backgrounds.insert("sky".into(), 0);
        index.backgrounds.insert("sky-alt".into(), 0);
        add(&c, &mut graphics, &mut sound(), &mut index).unwrap();
        assert_eq!((index.backgrounds["sky"], index.backgrounds["sky-alt"]), (1, 0));
        assert_eq!(graphics.background(0), Some(&Background::default()));
        // A file that isn't there.
        c.custom_assets.backgrounds.get_mut("sky").unwrap().file = "g:backgrounds/gone.png".into();
        let e = add(&c, &mut Bundle::default(), &mut sound(), &mut AssetIndex::default()).unwrap_err();
        assert!(e.starts_with("g/backgrounds/x.luau: background \"sky\": g:backgrounds/gone.png: "), "{e}");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
