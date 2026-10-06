pub(crate) fn asset_names() -> nettai_content::names::AssetNames {
    let c = exe5_compat::Compat::exe5();
    nettai_content::names::AssetNames {
        sprites: c.sprite_names(),
        songs: c
            .assets
            .sounds
            .iter()
            .map(|(k, &v)| (v, k.clone()))
            .collect(),
        banners: c
            .assets
            .banners
            .iter()
            .map(|(k, &v)| (v, k.clone()))
            .collect(),
        backgrounds: c
            .assets
            .backgrounds
            .iter()
            .map(|(k, &v)| (v, k.clone()))
            .collect(),
        mugshots: c
            .assets
            .mugshots
            .iter()
            .map(|(k, &v)| (v, k.clone()))
            .collect(),
        chips: c
            .chip_keys
            .iter()
            .map(|(&id, k)| (id, nettai_content_api::keys::local(k).to_string()))
            .collect(),
        navis: c
            .records
            .navis
            .iter()
            .map(|(k, &n)| (n, nettai_content_api::keys::local(k).to_string()))
            .collect(),
        glyphs: c.text.glyphs.clone(),
        dialogue_glyphs: c.text.dialogue_glyphs.clone(),
        // The Japanese ROMs' encoding is the pack's Japanese lettering's.
        language_glyphs: [(
            crate::exe5::lettering::LANGUAGE.to_string(),
            (c.text.jp.glyphs.clone(), c.text.jp.dialogue_glyphs.clone()),
        )]
        .into(),
    }
}
