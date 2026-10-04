//! Healing (`sub_800E2FC`), the ruleset's service for recovery chips
//! (action 0x20, the pack's `chips/09a-recov10`) and the heal some chip
//! uses give: the HP go up to the maximum, with a sparkle and a sound. When
//! the healer's opponent has AntiRecv armed (its defensive-chip record)
//! and the caller asks for the check, the trap springs instead: the heal
//! becomes a dimming whose controller (the role `kinds.anti_recovery`; the
//! original's effect #0x2C, `sub_80E3728`) takes the amount from the healer.

use crate::battle::Battle;
use crate::content::{EffectRole, SoundRole};
use crate::kinds::effect;
use crate::object::{ObjectRef, Vec3};

/// AntiRecv's counterattack: the counter byte its damage carries (the
/// "trap!" mark, the role `effects.trap_mark`, rises over the healer with
/// its sound: `trap_mark`).
pub(crate) const TRAP_HIT_PARAM: u32 = 0x1E;

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
    let look = b.roles_for(r).effect(EffectRole::Recovery);
    effect::spawn(b, pos, look, 0, 0, 0);
    b.sound(SoundRole::Recovery);
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
    let kind = b.arena_roles().kind(crate::content::KindRole::AntiRecovery);
    let c = crate::kinds::spawn(b, kind, nettai_content_api::SpawnAt::AfterCurrent, pos, [0; 4])?;
    let o = b.objects.get_mut(c);
    o.panel = panel;
    o.element = 0;
    o.related[0] = Some(healer);
    o.alliance = alliance;
    o.damage = damage as u16;
    o.stamina = (damage >> 16) as u16;
    // +0x30 also gets the trap's chip id, for the telop only: the chip the
    // other side's record holds, which sprang.
    let named = b.linked[(alliance ^ 1) as usize & 1].chip;
    b.objects.get_mut(c).telop_chip = named.map(|chip| crate::hud::TelopChip { chip: Some(chip), ..Default::default() });
    Some(c)
}

/// `sub_800ABC6`: the trap's mark over `r`'s panel (for the local side's
/// look, Param2), with its sound, where the sprung trap's game puts it from
/// the panel's center (the chip-use rules' `anti_navi_sparkle`: BN6's 16
/// pixels down the field and 32 up, BN5's 16 up). Its height (which the
/// routine leaves in r3).
pub(crate) fn trap_mark(b: &mut Battle, r: ObjectRef) -> i32 {
    let (panel, alliance) = {
        let o = b.objects.get(r);
        (o.panel, o.alliance)
    };
    let (x, y) = crate::kinds::player::panel_coordinates(panel.x, panel.y);
    let local = b.round.local_side;
    let look = b.roles_for(r).effect(EffectRole::TrapMark);
    let at = b.chip_rules(b.linked[(alliance ^ 1) as usize & 1].chip).chip_use.anti_navi_sparkle;
    let z = (at.z as i32) << 16;
    effect::spawn(b, Vec3 { x, y: y.wrapping_add((at.dy as i32) << 16), z }, look, local, 0, 0);
    b.sound(SoundRole::CutIn);
    z
}
