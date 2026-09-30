//! An idle overlay (actor object #0x55, `sub_80C40D8`): a second sprite a
//! navi wears only while it stands in its animation 0 (SpoutMan's, from
//! his init hook `sub_8010F7A`). It follows its owner's position,
//! visibility, palette, look and facing, and sits 255 pixels up (out of
//! sight) while the owner is in any other animation. Purely visual, but it
//! holds an actor slot and a place in the update order.

use crate::battle::Battle;
use crate::content::SpriteId;
use crate::kinds::common::{self, Progress};
use crate::object::sprite::Shadow;
use crate::object::{ObjectRef, Pool, flags, state};

pub const INDEX: u8 = 0x55;

/// Its sprites by Param1 (`dword_80C40D4`: one entry).
const SPRITES: [SpriteId; 1] = [SpriteId { category: 0x10, index: 0x21 }];

/// Overlay-private state.
#[derive(Clone, Copy, Debug, Default, Hash)]
pub struct Vars {
    /// ExtraVars[0] (`sub_80C4526`): the height is left as the owner's
    /// rather than dropped to the ground or lifted out of sight.
    pub pinned: bool,
}

fn vars(b: &Battle, r: ObjectRef) -> Vars {
    match &b.objects.get(r).vars {
        crate::kinds::Vars::IdleOverlay(v) => *v,
        _ => Vars::default(),
    }
}

/// `sub_80C41D8`: overlay `variant` on `owner`, at its position. It runs
/// while paused and dimmed.
pub fn spawn(b: &mut Battle, owner: ObjectRef, variant: u8) -> Option<ObjectRef> {
    let (pos, alliance) = {
        let o = b.objects.get(owner);
        (o.pos, o.alliance)
    };
    let r = b.objects.spawn(Pool::Actor, INDEX, pos, [variant, 0, 0, 0])?;
    let o = b.objects.get_mut(r);
    o.alliance = alliance;
    o.related[0] = Some(owner);
    o.flags |= flags::RUN_WHILE_PAUSED | flags::RUN_WHILE_DIMMED;
    o.vars = crate::kinds::Vars::IdleOverlay(Vars::default());
    Some(r)
}

/// `sub_80C4526(overlay, 1)`: keep its height as the owner's.
pub fn pin(b: &mut Battle, r: ObjectRef) {
    if let crate::kinds::Vars::IdleOverlay(v) = &mut b.objects.get_mut(r).vars {
        v.pinned = true;
    }
}

pub fn update(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).state {
        state::INIT => init(b, r),
        state::UPDATE => tick(b, r),
        _ => b.objects.free(r),
    }
}

fn owner(b: &Battle, r: ObjectRef) -> ObjectRef {
    b.objects.get(r).related[0].expect("idle overlay has an owner")
}

/// `sub_80C40F8`: its panel from its position, the sprite (no shadow,
/// animation 0, the owner's palette, its facing), visible.
fn init(b: &mut Battle, r: ObjectRef) {
    common::set_panels_from_coordinates(b, r);
    let variant = b.objects.get(r).params[0];
    let sprite = *SPRITES
        .get(variant as usize)
        .unwrap_or_else(|| panic!("idle overlay variant {variant} reads past its sprite table (sub_80C40F8)"));
    let owner_palette = b.objects.sprite(owner(b, r)).look.palette;
    let flip = {
        let o = b.objects.get(r);
        o.alliance ^ o.flip
    };
    let s = b.objects.sprite_mut(r);
    s.load(sprite);
    s.look.shadow = Shadow::WithSprite;
    s.set_animation(0, &b.content);
    s.look.palette = owner_palette;
    s.look.set_flip(flip);
    let o = b.objects.get_mut(r);
    o.flags &= !flags::NO_SPRITE_UPDATE;
    o.flags |= flags::VISIBLE;
    o.anim = 0;
    o.anim_loaded = 0;
    common::set_progress(b, r, Progress::UPDATE);
}

/// `sub_80C4146`: follow the owner; at ground level in its animation 0,
/// out of sight otherwise; step the sprite unless dimmed or paused.
fn tick(b: &mut Battle, r: ObjectRef) {
    let owner = owner(b, r);
    let (pos, owner_flags, owner_flip, owner_anim) = {
        let o = b.objects.get(owner);
        (o.pos, o.flags, o.flip, o.anim)
    };
    let pinned = vars(b, r).pinned;
    let o = b.objects.get_mut(r);
    o.pos = pos;
    o.flags = (o.flags & !flags::VISIBLE) | (owner_flags & flags::VISIBLE);
    o.flip = owner_flip;
    if !pinned {
        // The whole-pixel half of Z (a halfword store).
        let height: i32 = if owner_anim == 0 { 0 } else { 0xFF };
        o.pos.z = (o.pos.z & 0xFFFF) | (height << 16);
    }
    let alliance = o.alliance;
    let owner_look = b.objects.sprite(owner).look;
    let look = &mut b.objects.sprite_mut(r).look;
    look.palette = owner_look.palette;
    look.color_shader = owner_look.color_shader;
    look.white = owner_look.white;
    look.mosaic = owner_look.mosaic;
    look.alpha = owner_look.alpha;
    look.set_flip(alliance ^ owner_flip);
    if b.is_dimmed() || b.paused {
        return;
    }
    common::update_sprite(b, r);
}
