//! A status's visual over a navi (effect object #6, `sub_80E08FC`): the
//! confusion's circling stars (with their sound every second) and the
//! blindness's mark, at the navi's sprite attach point 5. It keeps an
//! effect slot and a place in the update order while the status lasts: it
//! ends itself once the navi's link to it is cleared or the navi's status
//! flag is off. Spawned by the status routine (`status.rs`), which links it
//! in the navi's collision data. See docs/engine/objects-and-player.md.

use crate::battle::Battle;
use crate::collision::{f1, link};
use crate::content::{SoundRole, SpriteRole};
use crate::object::{ObjectRef, flags, state};

/// Which status it shows (Param1, a row of `byte_80E08E4`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Status {
    #[default]
    Confusion = 0,
    Blindness = 1,
    /// A third row the status routine never spawns.
    Immobilized = 2,
}

impl Status {
    /// Its sprite (by role) and the navi's status flag it lasts while.
    fn row(self) -> (SpriteRole, u32) {
        match self {
            Status::Confusion => (SpriteRole::Confusion, f1::CONFUSED),
            Status::Blindness => (SpriteRole::Blindness, f1::BLIND),
            Status::Immobilized => (SpriteRole::Immobilized, f1::IMMOBILIZED),
        }
    }

    /// The navi's collision link that holds it.
    fn link(self) -> usize {
        match self {
            Status::Confusion => link::CONFUSE,
            Status::Blindness => link::BLIND,
            Status::Immobilized => panic!("the immobilized visual has no link the status routine gives it"),
        }
    }
}

/// Visual-private state (the spawn parameter).
#[derive(Clone, Debug, Default, Hash)]
pub struct Vars {
    pub status: Status,
}

fn vars(b: &Battle, r: ObjectRef) -> Status {
    match &b.objects.get(r).vars {
        crate::kinds::Vars::StatusVisual(v) => v.status,
        v => panic!("status visual with {v:?}"),
    }
}

/// Ticks between the confusion's sounds.
const CONFUSION_SOUND_TICKS: u16 = 0x3C;
const SOUND_CONFUSION: SoundRole = SoundRole::Confusion;
/// The navi's attach point it sits at.
const ATTACH_POINT: usize = 5;

/// `sub_80E09EE`: `status`'s visual over `owner`, linked in the owner's
/// collision data. (The game spawns it at the status routine's leftover
/// registers; its first unpaused update places it on the owner.)
pub fn spawn(b: &mut Battle, owner: ObjectRef, status: Status) -> Option<ObjectRef> {
    let pos = b.objects.get(owner).pos;
    let r = crate::kinds::spawn_engine(b, crate::kinds::EngineKind::StatusVisual, pos, [status as u8, 0, 0, 0])?;
    let (alliance, flip) = {
        let o = b.objects.get(owner);
        (o.alliance, o.flip)
    };
    let o = b.objects.get_mut(r);
    o.alliance = alliance;
    o.flip = flip;
    o.related[0] = Some(owner);
    o.vars = crate::kinds::Vars::StatusVisual(Vars { status });
    let c = b.objects.get(owner).collision.expect("a status on an object without collision data");
    b.collision.get_mut(c).links[status.link()] = Some(r);
    Some(r)
}

pub fn update(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).state {
        state::INIT => init(b, r),
        state::UPDATE => follow(b, r),
        _ => b.objects.free(r),
    }
}

/// `sub_80E091C`.
fn init(b: &mut Battle, r: ObjectRef) {
    let (sprite, _) = vars(b, r).row();
    let sprite = b.content.defs.roles.sprite(sprite);
    let s = b.objects.sprite_mut(r);
    s.load(sprite);
    s.look.shadow = crate::object::sprite::Shadow::WithSprite;
    let o = b.objects.get_mut(r);
    o.flags &= !flags::NO_SPRITE_UPDATE;
    o.set_visible(true);
    let flip = o.alliance ^ o.flip;
    b.objects.sprite_mut(r).look.set_flip(flip);
    let o = b.objects.get_mut(r);
    o.state = state::UPDATE;
    o.action = 0;
    o.phase = 0;
    o.phase_init = 0;
    follow(b, r);
}

/// `sub_80E0954`: stand still while dimmed; otherwise follow the owner
/// (hidden from a blind viewer on the other side, and off the field), and
/// end once the owner no longer has the status.
fn follow(b: &mut Battle, r: ObjectRef) {
    if b.is_dimmed() {
        return;
    }
    let status = vars(b, r);
    b.objects.get_mut(r).set_visible(true);
    b.hide_from_blind(r);
    let o = b.objects.get_mut(r);
    if status == Status::Confusion {
        let t = o.timer as i32 - 1;
        o.timer = t as u16;
        if t <= 0 {
            o.timer = CONFUSION_SOUND_TICKS;
            b.sound(SOUND_CONFUSION);
        }
    }
    let owner = b.objects.get(r).related[0].expect("a status visual without its owner");
    let (dx, dz) = crate::kinds::player::attach_point(b, owner, ATTACH_POINT);
    let p = b.objects.get(owner).pos;
    let o = b.objects.get_mut(r);
    o.pos.x = p.x.wrapping_add(dx << 16);
    o.pos.y = p.y;
    o.pos.z = p.z.wrapping_add(dz << 16);
    crate::kinds::common::set_panels_from_coordinates(b, r);
    let o = b.objects.get_mut(r);
    if !crate::field::is_valid(o.panel.x, o.panel.y) {
        o.set_visible(false);
    }
    let (_, flag) = status.row();
    let c = b.objects.get(owner).collision.expect("a status on an object without collision data");
    let linked = b.collision.get(c).links[status.link()].is_some();
    if !linked || b.collision.get(c).f1 & flag == 0 {
        b.collision.get_mut(c).links[status.link()] = None;
        b.objects.free(r);
        return;
    }
    crate::kinds::common::update_sprite_while_dimmed(b, r);
}
