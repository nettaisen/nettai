//! The Beast Out lock-on marker (effect object #0x0F, `sub_80E1520`). It
//! sits on the opponent MegaMan's Beast Out chips home in on, and its panel
//! is the target they read (`sub_80E164A`). Shown only to the Beast Out
//! navi's own side. See docs/engine/objects-and-player.md §A.7.

use crate::battle::Battle;
use crate::content::SpriteId;
use crate::kinds::common::{self, Progress, set_progress};
use crate::object::{ObjectRef, PanelPos, Vec3, flags, state};
use crate::setup::Navi;

const SPRITE: SpriteId = SpriteId { category: 0x0C, index: 0x09 };

/// Its attach point on the target.
const TARGET_ATTACH_POINT: usize = 0x11;

/// Marker-private state.
#[derive(Clone, Debug, Default, Hash)]
pub struct Vars {
    /// Held in place while a chip is locked on (`sub_80E1654`).
    pub frozen: bool,
}

fn vars(b: &mut Battle, r: ObjectRef) -> &mut Vars {
    match &mut b.objects.get_mut(r).vars {
        crate::kinds::Vars::LockonMarker(v) => v,
        v => panic!("lock-on marker with {v:?}"),
    }
}

/// `sub_80E1620`: give `owner` a marker (kept in its actor data).
pub fn spawn(b: &mut Battle, owner: ObjectRef) -> Option<ObjectRef> {
    let r = crate::kinds::spawn_engine(b, crate::kinds::EngineKind::LockonMarker, Vec3::default(), [0; 4])?;
    let (alliance, flip) = {
        let o = b.objects.get(owner);
        (o.alliance, o.flip)
    };
    let o = b.objects.get_mut(r);
    o.related[0] = Some(owner);
    o.alliance = alliance;
    o.flip = flip;
    o.flags |= flags::RUN_WHILE_PAUSED;
    let actor = b.objects.get(owner).actor.expect("lock-on marker owner has actor data");
    b.actors.get_mut(actor).lockon_marker = Some(r);
    Some(r)
}

/// `sub_80E1654`: hold the marker where it is (a chip locked on).
pub fn freeze(b: &mut Battle, r: ObjectRef) {
    vars(b, r).frozen = true;
    b.objects.get_mut(r).anim = 1;
}

/// `sub_80E1662`: let the marker follow its target again.
pub fn unfreeze(b: &mut Battle, r: ObjectRef) {
    vars(b, r).frozen = false;
    b.objects.get_mut(r).anim = 0;
}

/// `sub_80E164A`: the panel the marker is on.
pub fn panel(b: &Battle, r: ObjectRef) -> PanelPos {
    b.objects.get(r).panel
}

/// The lock-on panel read without a marker (outside Beast Out, where the
/// Cross special's chips rush too): the game reads it through a null
/// pointer, from BIOS memory, whose open-bus bytes (the BIOS's last opcode
/// fetched; after an SWI, 0xE3A02004: 0xA0, 0xE3) name no panel on the
/// field, as any of its values would.
pub const NO_MARKER_PANEL: PanelPos = PanelPos { x: 0xA0, y: 0xE3 };

/// `sub_80E164A` on the navi's marker slot (AIData+0x40), which may be
/// empty.
pub fn panel_of(b: &Battle, marker: Option<ObjectRef>) -> PanelPos {
    marker.map_or(NO_MARKER_PANEL, |m| panel(b, m))
}

pub fn update(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).state {
        state::INIT => {
            // sub_80E1540
            b.objects.sprite_mut(r).load(SPRITE);
            b.objects.sprite_mut(r).look.shadow = crate::object::sprite::Shadow::WithSprite;
            let o = b.objects.get_mut(r);
            o.flags &= !flags::NO_SPRITE_UPDATE;
            o.flags |= flags::VISIBLE;
            set_progress(b, r, Progress::UPDATE);
            tick(b, r);
        }
        state::UPDATE => tick(b, r),
        _ => b.objects.free(r),
    }
}

fn owner(b: &Battle, r: ObjectRef) -> ObjectRef {
    b.objects.get(r).related[0].expect("lock-on marker has an owner")
}

/// The owner's actor-data link to this marker.
fn link(b: &mut Battle, r: ObjectRef) -> &mut Option<ObjectRef> {
    let actor = b.objects.get(owner(b, r)).actor.expect("lock-on marker owner has actor data");
    &mut b.actors.get_mut(actor).lockon_marker
}

/// `sub_80E1566`.
fn tick(b: &mut Battle, r: ObjectRef) {
    let alliance = b.objects.get(r).alliance;
    let side = alliance as usize;
    let shown = b.navi(side) == Navi::MEGAMAN && b.form(side).is_beast() && !b.is_remote(alliance);
    let o = b.objects.get_mut(r);
    o.flags &= !flags::VISIBLE;
    if shown {
        o.flags |= flags::VISIBLE;
    }
    if b.is_dimmed() {
        return;
    }
    if b.is_battle_over() {
        *link(b, r) = None;
        b.objects.free(r);
        return;
    }
    if !vars(b, r).frozen {
        let pos = aim(b, r);
        b.objects.get_mut(r).pos = pos;
    }
    common::set_panels_from_coordinates(b, r);
    let p = b.objects.get(r).panel;
    if !crate::field::is_valid(p.x, p.y) {
        b.objects.get_mut(r).flags &= !flags::VISIBLE;
    }
    if link(b, r).is_none() {
        b.objects.free(r);
        return;
    }
    common::update_sprite_while_dimmed(b, r);
}

/// Over the target: its attach point, raised 8 pixels (none for the
/// bosses 0x173..=0x178 in their animation 0x4F).
fn aim(b: &Battle, r: ObjectRef) -> Vec3 {
    let target = target(b, r);
    let t = b.objects.get(target);
    let (dx, dz) = if (0x173..=0x178).contains(&t.name_id) && t.anim == 0x4F {
        (0, 0)
    } else {
        crate::kinds::player::attach_point(b, target, TARGET_ATTACH_POINT)
    };
    Vec3 {
        x: t.pos.x.wrapping_add(dx << 16),
        y: t.pos.y.wrapping_add(8 << 16),
        z: t.pos.z.wrapping_add((dz + 8) << 16),
    }
}

/// `sub_80E1670`: the opposing actor to lock on to: ahead of the owner
/// first, then behind, then in its column.
fn target(b: &Battle, r: ObjectRef) -> ObjectRef {
    let owner = b.objects.get(owner(b, r));
    let opponents = b.round.alive_actors[(b.objects.get(r).alliance ^ 1) as usize];
    let candidates: Vec<ObjectRef> =
        opponents.iter().flatten().copied().filter(|&c| b.objects.get(c).name_id <= 0x1C3).collect();
    let facing_right = owner.alliance ^ owner.flip == 0;
    let x = owner.panel.x;
    let ahead = if facing_right { (x + 1, 6) } else { (1, x.wrapping_sub(1)) };
    let behind = if facing_right { (1, x.wrapping_sub(1)) } else { (x + 1, 6) };
    let owner_ref = self::owner(b, r);
    for (lo, hi) in [ahead, behind, (x, x)] {
        // sub_80E1704: the first in the range, then each later one against
        // the one held.
        let found = candidates.iter().copied().filter(|&c| (lo..=hi).contains(&b.objects.get(c).panel.x));
        if let Some(best) = found.reduce(|held, c| closer(b, owner_ref, held, c)) {
            return best;
        }
    }
    panic!("a lock-on marker with no opponent to target reads through a null pointer");
}

/// `sub_80E1730`: of two targets in range, the one in the owner's row;
/// with both or neither there, `sub_80E175C`: the nearer column, then
/// (same distance) the one further ahead of the owner, then (same column,
/// `sub_80E17AC`) the nearer row, then the one not below the owner.
fn closer(b: &Battle, owner: ObjectRef, held: ObjectRef, candidate: ObjectRef) -> ObjectRef {
    let (o, h, c) = (b.objects.get(owner).panel, b.objects.get(held).panel, b.objects.get(candidate).panel);
    match (h.y == o.y, c.y == o.y) {
        (true, false) => return held,
        (false, true) => return candidate,
        _ => {}
    }
    let (hd, cd) = ((h.x as i32 - o.x as i32).abs(), (c.x as i32 - o.x as i32).abs());
    if cd != hd {
        return if cd < hd { candidate } else { held };
    }
    if c.x != h.x {
        let owner_o = b.objects.get(owner);
        let facing_right = owner_o.alliance ^ owner_o.flip == 0;
        return if (c.x > h.x) == facing_right { candidate } else { held };
    }
    let (hd, cd) = ((h.y as i32 - o.y as i32).abs(), (c.y as i32 - o.y as i32).abs());
    if cd != hd {
        return if cd < hd { candidate } else { held };
    }
    if o.y < c.y { held } else { candidate }
}
