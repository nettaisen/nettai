//! What the buster shots share (the buster 0x11, the charged shot 0x16 and
//! the blank shot 0x33): the arm the navi raises, the recovery after a
//! shot, and the move that may cut the recovery short. See
//! docs/engine/objects-and-player.md §B6.

use crate::actor::{ActorType, status};
use crate::battle::Battle;
use crate::collision::f1;
use crate::data::player as pdata;
use crate::kinds::attachment::{self, AttachSlot};
use crate::kinds::player::{Emotion, actor_id, ai, ai_mut, emotion, exit_attack_state, flag1, stats};
use crate::kinds::player::actions::movement;
use crate::kinds::player::idle;
use crate::object::ObjectRef;

/// The buster arm attachment.
const ARM: u8 = 6;

/// `sub_80EB562`: raise the buster arm (kept in the actor's overlay slot);
/// its animation shows the form, its palette the element or state.
pub(in crate::kinds::player) fn raise_arm(b: &mut Battle, r: ObjectRef) {
    let a = ai(b, r);
    let slot = AttachSlot::Overlay(actor_id(b, r));
    let params = match a.actor_type {
        ActorType::Virus => attachment::Params { kind: ARM, ..Default::default() },
        ActorType::Navi => {
            attachment::Params { kind: 0x2B, anim: a.ai_index.wrapping_sub(1), palette_add: 0xD, ..Default::default() }
        }
        ActorType::Player if a.ai_index != 0 => {
            ai_mut(b, r).overlay = None;
            return;
        }
        ActorType::Player => {
            let form = stats(b, r).form.0;
            let palette_add = match form {
                0 if a.status & status::NO_CHARGE != 0 => 0xE,
                0 => match stats(b, r).element {
                    0 => 0,
                    e => 0x14u8.wrapping_add(e),
                },
                0x0B | 0x0C if emotion(b, b.objects.get(r).alliance) == Emotion::FullSynchro => 0xE,
                0x0D..=0x11 => 5 + form - 0x0D,
                _ => 0,
            };
            attachment::Params { kind: ARM, anim: form, palette_add, ..Default::default() }
        }
    };
    attachment::spawn_with(b, r, params, slot);
}

/// `sub_800FAF6`: ticks of recovery after a shot: by the Rapid stat and
/// the open panels ahead of (x, y), counting (x, y) itself. The navi's
/// own body closes its panel.
pub(in crate::kinds::player) fn recovery(b: &Battle, r: ObjectRef, x: u8, y: u8) -> u16 {
    // byte_800FB4C: bodies, neutral objects, blockers and reservations,
    // but only the other side's barrier bits.
    const CLOSED: [u32; 2] = [0x0D88_0080, 0x0E88_0080];
    let o = b.objects.get(r);
    let front = crate::kinds::common::facing(o.alliance, o.flip);
    let closed = CLOSED[o.alliance as usize];
    let mut open = 0u8;
    let mut px = x as i32;
    loop {
        let f = b.field.flags(px as u8, y);
        if f == 0 || f & closed != 0 {
            break;
        }
        open = open.saturating_add(1);
        px += front;
    }
    pdata::buster_recovery(stats(b, r).rapid, open) as u16
}

/// The move that cuts a shot's recovery short: `object_canMove`, a held
/// direction (`sub_800FA54`) and a panel to step to (`sub_800F964`). Then
/// the attachment is dropped and the move starts; true if it did.
pub(in crate::kinds::player) fn move_cancel(b: &mut Battle, r: ObjectRef) -> bool {
    if flag1(b, r) & (f1::IMMOBILIZED | f1::SLIDING | f1::MOVING) != 0 {
        return false;
    }
    let dir = idle::held_direction(b, r);
    if dir == 0 || movement::step_target(b, r, dir).is_none() {
        return false;
    }
    b.objects.get_mut(r).related[0] = None;
    ai_mut(b, r).overlay = None;
    exit_attack_state(b, r);
    idle::start_move(b, r, dir);
    true
}
