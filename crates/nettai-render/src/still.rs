//! Still pictures of a battle's parts on their own, for a host's pictures
//! of them (Practice's grids of its arenas and of the background pool:
//! docs/app.md §15). A round's whole picture is
//! [`crate::Renderer::render`]'s.

use crate::audit::Problems;
use crate::compose::{BACKDROP_DEPTH, Fades, HEIGHT, Layer, WIDTH, compose_with_depth, to_rgb};
use crate::objects::SpriteList;
use crate::packs::PackGraphics;
use crate::stage::{Stage, StageClock};
use nettai_battle::Battle;
use nettai_battle::content::{BackgroundId, Content};
use nettai_battle::object::{ObjectRef, flags};

/// Background `id` as a round's first frame shows it (its scroll and its
/// animations at their start), `w` by `h` pixels from the screen's top
/// left, each `0x00RRGGBB`: drawn as [`crate::Renderer::render`] draws the
/// background layer, from its own pack (another game's from that game's:
/// the background pool's), the backdrop where it is clear. None: no pack
/// here has it.
pub fn background(graphics: &PackGraphics, c: &Content, id: BackgroundId, w: usize, h: usize) -> Option<Vec<u32>> {
    let packs = graphics.packs();
    let background = packs.background(c, id)?;
    let stage = Stage::new(&packs, c, Some(background), StageClock::default(), false);
    let backdrop = stage.palettes[0][0];
    Some(stage.draw_background_wide(w, h, backdrop, |c| c).into_iter().map(to_rgb).collect())
}

/// Battle `b`'s field as it is, its panels and the objects `keep` on them
/// (an arena's picture's: the two navis standing, each with its own shadow
/// under it, and the objects the arena placed as the round started), and
/// nothing else: no background, no other object, no HUD, no fade. The screen's 240 by 160,
/// each pixel `0xAARRGGBB`, clear (0) where nothing is drawn. The field's
/// panels as [`crate::Renderer::render`] draws them, from the game's pack,
/// seen from the local side; `keep`'s sprites as it draws them.
pub fn field(graphics: &PackGraphics, b: &Battle, keep: &[ObjectRef]) -> Vec<u32> {
    let packs = graphics.packs();
    let mut problems = Problems::default();
    let mut view = crate::Renderer::view(b);
    view.fade = Default::default();
    // (The others' sprites are left out as a sprite that doesn't animate is.)
    let mut only = b.clone();
    let others: Vec<ObjectRef> = only.objects.in_order().filter(|r| !keep.contains(r)).collect();
    for r in others {
        only.objects.get_mut(r).flags |= flags::NO_SPRITE_UPDATE;
    }
    let stage = Stage::new(&packs, &b.content, None, StageClock::of(b), false);
    let mut layer = Layer::new(2, 2);
    stage.draw_field(&only, &mut layer, b.setup.local_side, &view, &mut problems);
    let mut list = SpriteList::default();
    crate::objects::queue_objects(&only, &packs, &view, &[b.setup.local_side], &mut list, &mut problems, false);
    let parts = list.into_parts();
    let (pixels, depth) = compose_with_depth(0, &[&layer], &parts, Fades::default());
    let drawn = pixels.iter().zip(&depth).map(|(&c, &d)| if d == BACKDROP_DEPTH { 0 } else { 0xFF00_0000 | to_rgb(c) });
    debug_assert_eq!(pixels.len(), WIDTH * HEIGHT);
    drawn.collect()
}
