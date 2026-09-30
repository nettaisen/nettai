//! A navi layer (actor object #0x55, `sub_80C40D8`): SpoutMan's second
//! sprite (the water he stands in), which the parts of his actor record
//! add (`navi_parts`, AI index 6: `sub_8010F7A`). It follows its owner's
//! position, visibility, palette, look and facing every tick, and is
//! lifted out of sight (Z 255 pixels) whenever the owner's animation isn't
//! his first. Purely visual, but it holds an actor slot and a place in the
//! update order.

use crate::battle::Battle;
use crate::content::SpriteId;
use crate::kinds::common::{self, Progress, set_progress};
use crate::object::sprite::Shadow;
use crate::object::{ObjectRef, Pool, flags, state};

pub const INDEX: u8 = 0x55;

/// The layer's sprite (`dword_80C40D4[Param1]`; its only spawner,
/// `sub_80C41D8`, passes Param1 0).
const SPRITE: SpriteId = SpriteId { category: 0x10, index: 0x21 };

/// `sub_80C41D8`: a layer on `owner`, where the owner is, on its side. It
/// runs its first update right after the owner's, while paused and while
/// dimmed.
pub fn spawn(b: &mut Battle, owner: ObjectRef) -> Option<ObjectRef> {
    let (pos, alliance) = {
        let o = b.objects.get(owner);
        (o.pos, o.alliance)
    };
    let r = b.objects.spawn(Pool::Actor, INDEX, pos, [0; 4])?;
    let o = b.objects.get_mut(r);
    o.pos = pos;
    o.alliance = alliance;
    o.related[0] = Some(owner);
    o.flags |= flags::RUN_WHILE_PAUSED | flags::RUN_WHILE_DIMMED;
    Some(r)
}

/// `sub_80C4204`: remove the layer at its next update.
pub fn remove(b: &mut Battle, r: ObjectRef) {
    set_progress(b, r, Progress::DESTROY);
}

pub fn update(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).state {
        state::INIT => init(b, r),
        state::UPDATE => tick(b, r),
        _ => b.objects.free(r),
    }
}

fn owner(b: &Battle, r: ObjectRef) -> ObjectRef {
    b.objects.get(r).related[0].expect("navi layer has an owner")
}

/// `sub_80C40F8`: its panel from where it is; the sprite, without a
/// shadow, with the owner's palette.
fn init(b: &mut Battle, r: ObjectRef) {
    common::set_panels_from_coordinates(b, r);
    let owner_palette = b.objects.sprite(owner(b, r)).look.palette;
    let o = b.objects.get_mut(r);
    o.flags |= flags::VISIBLE;
    o.flags &= !flags::NO_SPRITE_UPDATE;
    o.anim = 0;
    o.anim_loaded = 0;
    let flip = o.alliance ^ o.flip;
    let s = b.objects.sprite_mut(r);
    s.load(SPRITE);
    s.look.shadow = Shadow::WithSprite;
    s.set_animation(0, &b.content);
    s.look.palette = owner_palette;
    s.look.set_flip(flip);
    set_progress(b, r, Progress::UPDATE);
}

/// `sub_80C4146`: follow the owner, then step the sprite unless dimmed or
/// paused.
fn tick(b: &mut Battle, r: ObjectRef) {
    let owner = owner(b, r);
    let (pos, owner_flags, owner_flip, owner_anim) = {
        let o = b.objects.get(owner);
        (o.pos, o.flags, o.flip, o.anim)
    };
    let owner_look = b.objects.sprite(owner).look;
    let o = b.objects.get_mut(r);
    o.pos = pos;
    o.flags = (o.flags & !flags::VISIBLE) | (owner_flags & flags::VISIBLE);
    o.flip = owner_flip;
    let flip = o.alliance ^ owner_flip;
    // (ExtraVars[0], which nothing sets, would keep the height.) Out of
    // sight unless the owner is in his first animation: Z's whole part
    // (a halfword store) 0 or 255.
    let lift: i32 = if owner_anim == 0 { 0 } else { 0xFF };
    o.pos.z = (o.pos.z & 0xFFFF) | (lift << 16);
    let look = &mut b.objects.sprite_mut(r).look;
    look.palette = owner_look.palette;
    look.color_shader = owner_look.color_shader;
    look.white = owner_look.white;
    look.mosaic = owner_look.mosaic;
    look.set_flip(flip);
    look.alpha = owner_look.alpha;
    if !b.is_dimmed() && !b.paused {
        common::update_sprite(b, r);
    }
}
