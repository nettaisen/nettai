pub(crate) fn asset_names() -> nettai_content::names::AssetNames {
    let mut names = nettai_content::names::AssetNames::default();
    let c = exe6_compat::Compat::exe6();
    for (name, id) in &c.assets.sprites {
        let parse = |s: &str| u8::from_str_radix(s, 16).ok();
        let Some((cat, index)) = id
            .split_once('-')
            .and_then(|(a, b)| Some((parse(a)?, parse(b)?)))
        else {
            panic!("assets.toml: sprite {name} is {id:?}, not \"cc-ii\"");
        };
        names.sprites.insert((cat, index), name.clone());
    }
    names.songs = c
        .assets
        .sounds
        .iter()
        .map(|(k, &v)| (v, k.clone()))
        .collect();
    names.backgrounds = c
        .assets
        .backgrounds
        .iter()
        .map(|(k, &v)| (v, k.clone()))
        .collect();
    names.mugshots = c
        .assets
        .mugshots
        .iter()
        .map(|(k, &v)| (v, k.clone()))
        .collect();
    names.banners = c
        .assets
        .banners
        .iter()
        .map(|(k, &v)| (v, k.clone()))
        .collect();
    // (A chip's icon is named in the pack by its id: `cannon`.)
    names.chips = c
        .chips
        .iter()
        .map(|(k, e)| (e.id, nettai_content_api::keys::local(k).to_string()))
        .collect();
    // (And a navi's emblem by its key: `heatman`.)
    names.navis = c
        .navis
        .iter()
        .map(|(k, e)| (e.navi, nettai_content_api::keys::local(k).to_string()))
        .collect();
    names.glyphs = c.text.glyphs.clone();
    names.dialogue_glyphs = c.text.dialogue_glyphs.clone();
    // The Japanese ROMs' encoding is the pack's Japanese lettering's.
    names.language_glyphs.insert(
        crate::exe6::lettering::LANGUAGE.into(),
        (c.text.jp.glyphs.clone(), c.text.jp.dialogue_glyphs.clone()),
    );
    names
}
