//! Attachments (actor object #5, `sub_80B8CD8`): a sprite a navi holds
//! during an attack, such as GunDelSol's gun. Visual only, but it takes an
//! actor slot and a place in the update order. It follows its owner's
//! position and visibility, and ends itself once the owner clears the slot
//! it was stored in. See objects-and-player.md §A.2.

use crate::actor::ActorId;
use crate::battle::Battle;
use crate::kinds::common::{Progress, set_progress, update_sprite};
use crate::object::sprite::Shadow;
use crate::object::{ObjectRef, Pool, Vec3, flags, state};

pub const INDEX: u8 = 5;

/// Where an owner keeps an attached object. The object lives while the
/// slot holds something (not necessarily the object itself).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AttachSlot {
    /// The owner's actor-data overlay slot.
    Overlay(ActorId),
    /// The owner object's first related object.
    Related(ObjectRef),
}

impl AttachSlot {
    pub fn is_occupied(self, b: &Battle) -> bool {
        match self {
            AttachSlot::Overlay(a) => b.actors.get(a).overlay.is_some(),
            AttachSlot::Related(o) => b.objects.get(o).related[0].is_some(),
        }
    }

    pub fn set(self, b: &mut Battle, v: Option<ObjectRef>) {
        match self {
            AttachSlot::Overlay(a) => b.actors.get_mut(a).overlay = v,
            AttachSlot::Related(o) => b.objects.get_mut(o).related[0] = v,
        }
    }
}

/// Attachment-private state.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Vars {
    pub slot: Option<AttachSlot>,
    /// Offset from the owner's position (its attach point), 16.16.
    pub offset_x: i32,
    pub offset_z: i32,
    /// Pixels raised by (subtracted from y and z).
    pub lift: i8,
}

fn vars(b: &mut Battle, r: ObjectRef) -> &mut Vars {
    match &mut b.objects.get_mut(r).vars {
        crate::kinds::Vars::Attachment(v) => v,
        v => panic!("attachment with {v:?}"),
    }
}

/// `sub_80B8E30`: attach an object of `kind` (`Content::attachment`)
/// to `owner`, stored in `slot` (which gets None if the pool is full). It
/// takes the owner's panel and side; its init places it.
pub fn spawn(b: &mut Battle, owner: ObjectRef, kind: u8, slot: AttachSlot) -> Option<ObjectRef> {
    spawn_with(b, owner, Params { kind, ..Params::default() }, slot)
}

/// An attachment's spawn parameters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Params {
    /// Param1: which attachment (`Content::attachment`).
    pub kind: u8,
    /// Param2: its animation.
    pub anim: u8,
    /// Param3: it animates in time stop too.
    pub in_time_stop: bool,
    /// Param4: added to its palette (drawn only).
    pub palette_add: u8,
}

/// `sub_80B8E30` with all its parameters.
pub fn spawn_with(b: &mut Battle, owner: ObjectRef, p: Params, slot: AttachSlot) -> Option<ObjectRef> {
    let params = [p.kind, p.anim, p.in_time_stop as u8, p.palette_add];
    let r = crate::behavior::spawn_object(b, Pool::Actor, INDEX, Vec3::default(), params);
    if let Some(r) = r {
        let (panel, alliance, flip) = {
            let o = b.objects.get(owner);
            (o.panel, o.alliance, o.flip)
        };
        let o = b.objects.get_mut(r);
        o.related[0] = Some(owner);
        o.panel = panel;
        o.alliance = alliance;
        o.flip = flip;
        o.flags |= flags::RUN_WHILE_PAUSED | flags::RUN_IN_TIME_STOP;
        if b.behaviors.object_kind(Pool::Actor, INDEX).is_some() {
            // Behaviors implements attachments: its slot names the owner's.
            crate::behavior::set_state_variant(b, r, "slot", content_slot(b, owner, slot));
        } else {
            b.objects.get_mut(r).vars = crate::kinds::Vars::Attachment(Vars { slot: Some(slot), ..Vars::default() });
        }
    }
    slot.set(b, r);
    r
}

/// The content slot (`lib/slot.luau`) for a slot of `owner`'s. Behaviors
/// only knows its owner's own slots.
pub(crate) fn content_slot(b: &Battle, owner: ObjectRef, slot: AttachSlot) -> &'static str {
    match slot {
        AttachSlot::Overlay(a) if b.objects.get(owner).actor == Some(a) => "overlay",
        AttachSlot::Related(o) if o == owner => "related",
        s => panic!("an attachment kept in another object's slot ({s:?}) has no content form"),
    }
}

pub fn update(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).state {
        state::INIT => init(b, r),
        state::UPDATE => follow(b, r),
        _ => b.objects.free(r),
    }
}

/// `sub_80B8CF8`.
fn init(b: &mut Battle, r: ObjectRef) {
    let [kind, anim, ..] = b.objects.get(r).params;
    let k = *b.content.attachment(kind);
    let owner = owner(b, r);
    let palette_add = b.objects.get(r).params[3];
    let sprite = b.objects.sprite_mut(r);
    sprite.load(k.sprite);
    // sprite_noShadow; the kind's palette plus the fourth parameter.
    sprite.look.shadow = Shadow::WithSprite;
    sprite.look.palette = k.palette.wrapping_add(palette_add);
    b.objects.get_mut(r).flags &= !flags::NO_SPRITE_UPDATE;
    // The Cross forms' NameIDs raise it by 8 pixels.
    let name = b.objects.get(owner).name_id;
    let cross = (0x1AC..=0x1B5).contains(&name) && b.stats[b.objects.get(r).alliance as usize].form.0 != 0;
    vars(b, r).lift = if cross { -8 } else { k.lift };
    let o = b.objects.get_mut(r);
    o.anim = anim;
    o.anim_loaded = anim;
    o.flags |= flags::VISIBLE;
    b.objects.sprite_mut(r).set_animation(anim, &b.content);
    if kind == 0xF {
        panic!("attachment kind 0xF offsets (sub_80B8CF8) are not implemented yet");
    }
    let (x, z) = match k.attach_point {
        Some(p) => crate::kinds::player::attach_point(b, owner, p as usize),
        None => (0, 0),
    };
    let v = vars(b, r);
    v.offset_x = x << 16;
    v.offset_z = z << 16;
    set_progress(b, r, Progress::UPDATE);
    follow(b, r);
}

fn owner(b: &Battle, r: ObjectRef) -> ObjectRef {
    b.objects.get(r).related[0].expect("attachment has an owner")
}

/// `sub_80B8DA6`: follow the owner's position and visibility; animate,
/// except while paused (or in time stop, unless the third parameter
/// says otherwise); end once the slot is cleared.
fn follow(b: &mut Battle, r: ObjectRef) {
    let owner = owner(b, r);
    let p = b.objects.get(owner).pos;
    let owner_visible = b.objects.get(owner).flags & flags::VISIBLE;
    let v = vars(b, r).clone();
    let lift = (v.lift as i32) << 16;
    let o = b.objects.get_mut(r);
    o.pos = Vec3 {
        x: p.x.wrapping_add(v.offset_x),
        y: p.y.wrapping_sub(lift),
        z: p.z.wrapping_add(v.offset_z).wrapping_sub(lift),
    };
    o.flags = (o.flags & !flags::VISIBLE) | owner_visible;
    // It wears the owner's colour shader, white flash, alpha and flip.
    let owner_look = b.objects.sprite(owner).look;
    let look = &mut b.objects.sprite_mut(r).look;
    look.color_shader = owner_look.color_shader;
    look.white = owner_look.white;
    look.alpha = owner_look.alpha;
    look.hflip = owner_look.hflip;
    look.vflip = owner_look.vflip;
    let slot = v.slot.expect("attachment has a slot");
    if !slot.is_occupied(b) {
        b.objects.get_mut(r).flags &= !flags::VISIBLE;
        set_progress(b, r, Progress::DESTROY);
        return;
    }
    if b.objects.get(r).params[2] == 0 && b.is_time_stop() {
        return;
    }
    if b.paused {
        return;
    }
    update_sprite(b, r);
}
