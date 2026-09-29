//! The player navi (actor #0 with actor type Player): spawn, init, the
//! per-tick pipeline, and its actions. See docs/engine/objects-and-player.md
//! chapter 12, and field-collision-damage.md chapter 4 for damage intake.

pub mod actions;

use crate::actor::ActorType;
use crate::battle::Battle;
use crate::object::{ObjectRef, PanelPos, Pool, Vec3, flags, state};
use crate::setup::ActorEntry;

/// Panel center coordinates (`object_getCoordinatesForPanels`).
pub fn panel_coordinates(x: u8, y: u8) -> (i32, i32) {
    ((x as i32 * 40 - 140) << 16, (y as i32 * 24 - 20) << 16)
}

/// Spawn a player navi from an actor-list entry (`sub_800753C`).
pub fn spawn(b: &mut Battle, entry: &ActorEntry) -> Option<ObjectRef> {
    let r = b.objects.spawn(Pool::Actor, 0, Vec3::default(), [0; 4])?;
    let (x, y) = panel_coordinates(entry.x, entry.y);
    {
        let o = b.objects.get_mut(r);
        o.alliance = entry.alliance;
        o.panel = PanelPos { x: entry.x, y: entry.y };
        o.future_panel = o.panel;
        o.pos = Vec3 { x, y, z: 0 };
        o.flags |= flags::RUN_WHILE_PAUSED;
    }
    let Some(a) = b.actors.allocate() else {
        b.objects.free(r);
        return None;
    };
    b.objects.get_mut(r).actor = Some(a);
    b.actors.get_mut(a).actor_type = ActorType::Player;
    let navi = b.stats[entry.alliance as usize].navi();
    b.objects.get_mut(r).name_id = 0x1A0 + navi as u16;
    // Per-navi record (`sub_80182B4`): MegaMan is {0, 2, 0}.
    let ad = b.actors.get_mut(a);
    ad.actor_type = ActorType::Player;
    ad.ai_index = 0;
    ad.unk_03 = 1;
    ad.unk_0e = 0xFF;
    Some(r)
}

/// The player's update (`sub_80EA460`): lifecycle state, then the sprite
/// step every tick.
pub fn update(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).state {
        state::INIT => init(b, r),
        state::UPDATE => tick(b, r),
        _ => destroy(b, r),
    }
    step_sprite(b, r);
}

/// `sub_80172F0`.
fn init(_b: &mut Battle, _r: ObjectRef) {
    panic!("player init is not implemented yet");
}

/// `sub_80EA484`.
fn tick(_b: &mut Battle, _r: ObjectRef) {
    panic!("player update is not implemented yet");
}

/// `sub_8016C4E`.
fn destroy(_b: &mut Battle, _r: ObjectRef) {
    panic!("player destroy is not implemented yet");
}

/// `sub_801BCF4`.
fn step_sprite(_b: &mut Battle, _r: ObjectRef) {}
