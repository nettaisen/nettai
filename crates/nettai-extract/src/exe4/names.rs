pub(crate) fn asset_names() -> nettai_content::names::AssetNames {
    let c = exe4_compat::Compat::exe4();
    nettai_content::names::AssetNames {
        sprites: c.sprite_names(),
        songs: c.assets.sounds.iter().map(|(k, &v)| (v, k.clone())).collect(),
        chips: c.chip_keys.iter().map(|(&id, k)| (id, k.clone())).collect(),
        glyphs: c.text.glyphs.clone(),
        dialogue_glyphs: c.text.dialogue_glyphs.clone(),
        // The Japanese ROMs' encoding is the pack's Japanese lettering's.
        language_glyphs: [(
            crate::exe4::graphics::LANGUAGE.to_string(),
            (c.text.jp.glyphs.clone(), c.text.jp.dialogue_glyphs.clone()),
        )]
        .into(),
        ..Default::default()
    }
}
