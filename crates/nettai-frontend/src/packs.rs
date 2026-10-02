//! The engine's assets as their packs hold them (docs/design/rules-in-luau.md
//! §7.4). The engine knows an asset by its handle over the loaded packs; the
//! frontend draws a pack's sprite, banner, mugshot or background by the
//! pack's own id for it, which the content's asset names give.

use nettai_battle::content::{BackgroundId, BannerId, Content, MugshotId, PackSprite, SpriteId};
use nettai_content_api::AssetKind;

/// Sprite `id` as its pack holds it.
pub fn sprite(c: &Content, id: SpriteId) -> Option<PackSprite> {
    c.assets.sprite(id.0).map(|a| a.id)
}

/// The number of `kind` asset `h` in its pack.
fn number(c: &Content, kind: AssetKind, h: u16) -> Option<u16> {
    c.assets.number(kind, h).map(|a| a.id)
}

/// Banner `id`'s number in its pack.
pub fn banner(c: &Content, id: BannerId) -> Option<u8> {
    number(c, AssetKind::Banner, id.0).map(|n| n as u8)
}

/// Mugshot `id`'s number in its pack.
pub fn mugshot(c: &Content, id: MugshotId) -> Option<u8> {
    number(c, AssetKind::Mugshot, id.0).map(|n| n as u8)
}

/// Background `id`'s number in its pack.
pub fn background(c: &Content, id: BackgroundId) -> Option<u8> {
    number(c, AssetKind::Background, id.0).map(|n| n as u8)
}

/// The name content gives `kind` asset `h` (`bn6:bomb`), for a problem's
/// text.
pub fn name(c: &Content, kind: AssetKind, h: u16) -> Option<&str> {
    let names = match kind {
        AssetKind::Sprite => c.assets.sprites.keys().nth(h as usize),
        AssetKind::Sound => c.assets.sounds.keys().nth(h as usize),
        AssetKind::Banner => c.assets.banners.keys().nth(h as usize),
        AssetKind::Background => c.assets.backgrounds.keys().nth(h as usize),
        AssetKind::Mugshot => c.assets.mugshots.keys().nth(h as usize),
    };
    names.map(String::as_str)
}
