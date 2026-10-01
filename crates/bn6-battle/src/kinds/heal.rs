//! Healing (`sub_800E2FC`), the ruleset's service for recovery chips
//! (action 0x20, the pack's `chips/09a-recov10`) and the heal some chip
//! uses give: the HP go up to the maximum, with a sparkle and a sound. When
//! the healer's opponent has AntiRecv armed (its defensive-chip record)
//! and the caller asks for the check, the trap springs instead: the heal
//! becomes a dimming whose controller (the role `kinds.anti_recovery`; the
//! original's effect #0x2C, `sub_80E3728`) takes the amount from the healer.

use crate::battle::Battle;
use crate::kinds::effect;
use crate::object::{ObjectRef, Vec3};
use crate::sound::SoundId;

/// The recovery sparkle (effect #0's look), and its sound.
const SPARKLE: u8 = 6;
const HEAL_SOUND: u16 = 0x8A;
/// AntiRecv's counterattack: the counter byte its damage carries, the
/// "trap!" mark (effect #0's look) raised over the healer, and its sound.
pub(crate) const TRAP_HIT_PARAM: u32 = 0x1E;
const TRAP_MARK: u8 = 0x46;
/// The mark's height, 32 pixels.
pub(crate) const TRAP_MARK_Z: i32 = 0x20_0000;
const TRAP_SOUND: u16 = 0xA5;

/// `sub_800E2FC`: heal `r` by `amount`; with `anti_recovery`, check the
/// opponent's AntiRecv first. True when the trap sprang (the game's
/// return value).
pub fn heal(b: &mut Battle, r: ObjectRef, amount: u16, anti_recovery: bool) -> bool {
    let alliance = b.objects.get(r).alliance;
    // sub_802CE78: the opponent's defensive-chip record.
    if anti_recovery && b.linked_trap(alliance ^ 1) == Some(crate::content::Trap::AntiRecovery) {
        spring_anti_recovery(b, r, amount);
        return true;
    }
    add_hp(b, r, amount);
    let pos = b.objects.get(r).pos;
    effect::spawn(b, pos, SPARKLE, 0, 0, 0);
    b.play_sound(SoundId(HEAL_SOUND));
    false
}

/// `object_addHP`: up to the maximum.
pub fn add_hp(b: &mut Battle, r: ObjectRef, amount: u16) {
    let o = b.objects.get_mut(r);
    o.hp = (o.hp as u32 + amount as u32).min(o.max_hp as u32) as u16;
}

/// The trap springs (`loc_800E330`): AntiRecv's controller (`sub_80E37D2`)
/// starts a dimming for the healer's side that deals `amount` to it, the
/// trap's mark shows over it (`sub_800ABC6`), and the opponent's record
/// is spent (`sub_802CEA6`).
fn spring_anti_recovery(b: &mut Battle, r: ObjectRef, amount: u16) {
    let alliance = b.objects.get(r).alliance;
    // The controller spawns with the spawner's registers as its position
    // (panel Y, element 0, the defensive record's owner word) and
    // parameters (whatever the heal's caller left in r4); a dimming
    // controller's init never reads either.
    let controller = spawn_counterattack(b, r, amount as u32 + (TRAP_HIT_PARAM << 16), 0);
    // sub_800BF16, no cut-in allowed, with the spawn's result (none when
    // the effect pool is full: the game registers a null controller).
    b.start_dimming(alliance, true, controller, r);
    trap_mark(b, r);
    b.clear_linked(alliance ^ 1);
}

/// `sub_80E37D2`: AntiRecv's counterattack (the role `kinds.anti_recovery`:
/// BN6's chips/antirecv/controller, the original's effect #0x2C) against
/// `healer`, on its panel and side, dealing the damage word `damage`
/// (damage | hit parameter << 16). Its telop is AntiRecv's (object +0x30).
/// None when the effect pool is full. Its position is the spawner's
/// registers: the panel's Y, the element (0) and `z`, which nothing reads.
pub(crate) fn spawn_counterattack(b: &mut Battle, healer: ObjectRef, damage: u32, z: i32) -> Option<ObjectRef> {
    let (panel, alliance) = {
        let o = b.objects.get(healer);
        (o.panel, o.alliance)
    };
    let pos = Vec3 { x: panel.y as i32, y: 0, z };
    let kind = b.content.defs.roles.kind(crate::content::KindRole::AntiRecovery);
    let c = crate::kinds::spawn(b, kind, bn6_content_api::SpawnAt::AfterCurrent, pos, [0; 4])?;
    let o = b.objects.get_mut(c);
    o.panel = panel;
    o.element = 0;
    o.related[0] = Some(healer);
    o.alliance = alliance;
    o.damage = damage as u16;
    o.stamina = (damage >> 16) as u16;
    // (+0x30 also gets the trap's chip id, for the telop only.)
    Some(c)
}

/// `sub_800ABC6`: the trap's mark over `r`'s panel (for the local side's
/// look, Param2), with its sound.
pub(crate) fn trap_mark(b: &mut Battle, r: ObjectRef) {
    let panel = b.objects.get(r).panel;
    let (x, y) = crate::kinds::player::panel_coordinates(panel.x, panel.y);
    let local = b.round.local_side;
    effect::spawn(b, Vec3 { x, y: y.wrapping_add(0x10_0000), z: TRAP_MARK_Z }, TRAP_MARK, local, 0, 0);
    b.play_sound(SoundId(TRAP_SOUND));
}
