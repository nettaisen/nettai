//! Generated assets contain no ROM data. Missing graphics use a black/magenta
//! checkerboard; missing sound is a valid one-track silent song.
use nettai_assets::*;
use nettai_content::names::AssetNames;

pub const PALETTE: Palette = [0, 0x7c1f, 0x0421, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
pub fn tiles(n: usize) -> Tiles {
    Tiles {
        pixels: (0..n * 64)
            .map(|i| {
                if ((i % 8) / 4 + (i / 8 % 8) / 4) % 2 == 0 {
                    1
                } else {
                    2
                }
            })
            .collect(),
    }
}
pub fn picture() -> Picture {
    Picture {
        tiles: tiles(42),
        palette: PALETTE,
    }
}
pub fn sprite(category: u8, index: u8) -> SpriteSheet {
    SpriteSheet {
        category,
        index,
        tilesets: vec![tiles(16)],
        palette_sets: vec![vec![PALETTE; 16]],
        part_lists: vec![vec![SpritePart {
            width: 32,
            height: 32,
            x: -16,
            y: -32,
            ..Default::default()
        }]],
        animations: vec![
            vec![SpriteFrame {
                duration: 1,
                flags: 0xc0,
                ..Default::default()
            }];
            256
        ],
        ..Default::default()
    }
}
fn font(cell: &[String], dialogue: &[String]) -> DialogueFont {
    let chars = cell.iter().chain(dialogue).cloned().collect::<Vec<_>>();
    DialogueFont {
        pixels: vec![1; chars.len() * 16 * 12],
        advances: vec![8; chars.len()],
        chars,
    }
}
pub fn hud(names: &AssetNames, banners: usize) -> Hud {
    Hud {
        tiles: tiles(54),
        first_tile: 0x1a0,
        gauge_tiles: tiles(28),
        gauge_first_tile: 0x222,
        hp_palettes: [PALETTE; 3],
        gauge_palette: PALETTE,
        hp_box: vec![
            MapEntry {
                tile: 0x1a0,
                palette: 13,
                ..Default::default()
            };
            12
        ],
        gauge_frame: vec![
            MapEntry {
                tile: 0x222,
                palette: 9,
                ..Default::default()
            };
            36
        ],
        font: tiles(names.glyphs.len() * 2),
        font_chars: names.glyphs.clone(),
        dialogue_font: font(&names.glyphs, &names.dialogue_glyphs),
        enemy_digits: std::array::from_fn(|_| tiles(20)),
        enemy_palette: PALETTE,
        hidden_icon: tiles(4),
        icon_palette: PALETTE,
        counts: vec![tiles(4); 11],
        count_box: tiles(4),
        navi_box: tiles(4),
        pause: tiles(10),
        texts: vec![vec![0]; 32],
        banners: vec![
            BannerLayout {
                x: 40,
                y: 64,
                glyphs: tiles(40),
                ..Default::default()
            };
            banners
        ],
        banner_digits: tiles(22),
        banner_palette: PALETTE,
        waiting: tiles(16),
        waiting_palette: PALETTE,
        warning: tiles(8),
        warning_palette: PALETTE,
        chatbox: Chatbox {
            tiles: tiles(11),
            palette: PALETTE,
            boxes: vec![
                std::array::from_fn(|_| vec![
                    MapEntry::default();
                    Chatbox::COLUMNS * Chatbox::ROWS
                ]);
                2
            ],
            arrow: tiles(12),
            text_palette: PALETTE,
        },
        language: BASE_LANGUAGE.into(),
        ..Default::default()
    }
}
pub fn lettering(base: &Hud, names: &AssetNames, language: &str) -> HudLettering {
    let (cell, dialogue) = names
        .language_glyphs
        .get(language)
        .cloned()
        .unwrap_or_default();
    HudLettering {
        font: tiles(cell.len() * 2),
        font_chars: cell.clone(),
        dialogue_font: font(&cell, &dialogue),
        texts: vec![vec![0]; base.texts.len()],
        banners: base
            .banners
            .iter()
            .map(|b| {
                Some(BannerLayout {
                    glyphs: tiles(b.glyphs.len()),
                    ..b.clone()
                })
            })
            .collect(),
        banner_palette: PALETTE,
        waiting: tiles(16),
        waiting_palette: PALETTE,
        gauge_tiles: tiles(base.gauge_tiles.len()),
    }
}
pub fn slot_pictures() -> SlotPictures {
    SlotPictures {
        ok: picture(),
        ok_picked: picture(),
        other: picture(),
    }
}
pub fn button(width: u8, height: u8) -> ButtonPictures {
    ButtonPictures {
        width,
        height,
        tiles: tiles(width as usize * height as usize * 3),
        picture: picture(),
        palettes: vec![PALETTE],
        ..Default::default()
    }
}
pub fn custom() -> CustomScreen {
    CustomScreen {
        window_tiles: tiles(136),
        column_cells: tiles(4),
        turn_limit: tiles(14),
        name_bar: tiles(4),
        window_maps: vec![vec![
            MapEntry {
                tile: 1,
                palette: 9,
                ..Default::default()
            };
            300
        ]],
        frame_palettes: vec![PALETTE; 4],
        icon_palette: PALETTE,
        gray_palette: PALETTE,
        other_palette: PALETTE,
        pictures: slot_pictures(),
        codes: tiles(56),
        elements: tiles(60),
        element_colors: vec![[0x7c1f; 6]; 15],
        digits: tiles(22),
        slot_codes: tiles(56),
        empty_icon: tiles(4),
        cursor: tiles(2),
        regular: tiles(32),
        advance_name_colors: vec![[0, 0x7c1f, 0x0421, 0]; 3],
        ..Default::default()
    }
}
pub fn complete(bundle: &mut Bundle, names: &AssetNames, panel_names: &[String]) -> Vec<String> {
    let mut missing = Vec::new();
    for (_, l) in &mut bundle.custom.languages {
        l.buttons.sort_by(|a, b| a.0.cmp(&b.0));
    }
    for (&(cat, id), name) in &names.sprites {
        if !bundle
            .sprites
            .iter()
            .any(|s| (s.category, s.index) == (cat, id))
        {
            bundle.sprites.push(sprite(cat, id));
            missing.push(format!("sprite/{name}"));
        }
    }
    bundle.sprites.sort_by_key(|s| (s.category, s.index));
    if bundle.field.tiles.is_empty() {
        bundle.field = Field {
            tiles: tiles(1),
            palettes: vec![PALETTE],
            panel_types: panel_names.to_vec(),
            panels: vec![[MapEntry::default(); 15]; panel_names.len() * 6],
            highlights: vec![[MapEntry::default(); 15]; 2],
            ..Default::default()
        };
        missing.push("field".into());
    }
    for (&id, name) in &names.backgrounds {
        bundle
            .backgrounds
            .resize(bundle.backgrounds.len().max(id as usize + 1), None);
        if bundle.backgrounds[id as usize].is_none() {
            bundle.backgrounds[id as usize] = Some(Background {
                tiles: tiles(1),
                map: vec![MapEntry::default(); 32 * 32],
                map_width: 32,
                map_height: 32,
                palette: Some(PALETTE),
                ..Default::default()
            });
            missing.push(format!("background/{name}"));
        }
    }
    for (&id, name) in &names.mugshots {
        if id < NAVI_MUGSHOTS {
            while bundle.hud.mugshots.len() <= id as usize {
                bundle.hud.mugshots.push((tiles(8), PALETTE));
                missing.push(format!("mugshot/{name}"));
            }
        } else {
            while bundle.hud.navi_mugshots.len() <= (id - NAVI_MUGSHOTS) as usize {
                bundle.hud.navi_mugshots.push(NaviMugshot {
                    tiles: tiles(8),
                    palettes: [PALETTE; 2],
                });
                missing.push(format!("mugshot/{name}"));
            }
        }
    }
    for key in names.chips.values() {
        if !bundle
            .custom
            .chip_art
            .iter()
            .any(|a| &a.key == key && !a.picture.tiles.is_empty())
        {
            if let Some(a) = bundle.custom.chip_art.iter_mut().find(|a| &a.key == key) {
                a.picture = picture();
            } else {
                bundle.custom.chip_art.push(ChipArt {
                    key: key.clone(),
                    picture: picture(),
                    ..Default::default()
                });
            }
            missing.push(format!("chip-art/{key}"));
        }
        if !bundle
            .hud
            .chip_icons
            .iter()
            .any(|a| &a.key == key && !a.tiles.is_empty())
        {
            if let Some(a) = bundle.hud.chip_icons.iter_mut().find(|a| &a.key == key) {
                a.tiles = tiles(4);
            } else {
                bundle.hud.chip_icons.push(ChipIcon {
                    key: key.clone(),
                    tiles: tiles(4),
                });
            }
            missing.push(format!("chip-icon/{key}"));
        }
    }
    for name in names.navis.values() {
        if !bundle.custom.emblems.iter().any(|e| &e.navi == name) {
            bundle.custom.emblems.push(Emblem {
                navi: name.clone(),
                tiles: tiles(4),
                palette: PALETTE,
            });
            missing.push(format!("emblem/{name}"));
        }
    }
    missing
}

pub fn sound_bank() -> m4a::SoundBank {
    use m4a::bank::*;
    SoundBank {
        mixer: MixerConfig {
            mix_rate: 31536,
            ds_channels: 8,
            master_volume: 15,
            reverb: 0,
            dac_resolution: 0,
        },
        players: vec![PlayerConfig {
            max_tracks: 1,
            uses_priority: false,
            track_order: 0,
        }],
        songs: Vec::new(),
        voicegroups: vec![Voicegroup::default()],
        key_maps: Vec::new(),
        samples: Vec::new(),
        waves: Vec::new(),
    }
}
pub fn songs(bank: &mut m4a::SoundBank, names: &AssetNames, unavailable: &[u16]) -> Vec<String> {
    use m4a::bank::*;
    let mut missing = Vec::new();
    // An empty slot in a supplied bank can be intentional (EXE6 stop-music).
    // Only absent sources or explicit decoding failures need generated audio.
    for &id in unavailable {
        bank.songs
            .resize(bank.songs.len().max(id as usize + 1), None);
        if bank.songs[id as usize].is_none() {
            bank.songs[id as usize] = Some(Song {
                player: PlayerId(0),
                priority: 0,
                reverb: None,
                voicegroup: VoicegroupId(0),
                tracks: vec![Track {
                    commands: vec![Command::Fine],
                }],
            });
            missing.push(format!("sound/{}", names.song(id)));
        }
    }
    missing
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn intentional_silence_is_not_a_missing_song() {
        let mut bank = sound_bank();
        bank.songs.resize(2, None);
        let names = AssetNames {
            songs: [(0, "stop-music".into()), (1, "missing-effect".into())].into(),
            ..Default::default()
        };
        assert_eq!(songs(&mut bank, &names, &[1]), ["sound/missing-effect"]);
        assert!(bank.songs[0].is_none());
        assert!(bank.songs[1].is_some());
        bank.validate().unwrap();
    }
}
