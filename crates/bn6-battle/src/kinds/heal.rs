//! Healing (`sub_800E2FC`), the ruleset's service for recovery chips
//! (action 0x20, the pack's `chips/09a-recov10`) and the heal some chip
//! uses give: the HP go up to the maximum, with a sparkle and a sound. When
//! the healer's opponent has AntiRecv armed (its defensive-chip record)
//! and the caller asks for the check, the trap springs instead: the heal
//! becomes a dimming whose controller (effect #0x2C, `sub_80E3728`) takes
//! the amount from the healer.

use crate::battle::Battle;
use crate::content::ChipId;
use crate::kinds::effect;
use crate::object::{ObjectRef, Pool, Vec3};
use crate::sound::SoundId;

/// AntiRecv, the trap chip that turns a heal into damage.
pub const ANTI_RECOVERY: ChipId = 0xBD;
/// The recovery sparkle (effect #0's look), and its sound.
const SPARKLE: u8 = 6;
const HEAL_SOUND: u16 = 0x8A;
/// AntiRecv's counterattack: its dimming controller, the counter byte
/// its damage carries, the "trap!" mark (effect #0's look) raised over
/// the healer, and its sound.
const TRAP_CONTROLLER: u8 = 0x2C;
const TRAP_HIT_PARAM: u32 = 0x1E;
const TRAP_MARK: u8 = 0x46;
const TRAP_SOUND: u16 = 0xA5;

/// `sub_800E2FC`: heal `r` by `amount`; with `anti_recovery`, check the
/// opponent's AntiRecv first. True when the trap sprang (the game's
/// return value).
pub fn heal(b: &mut Battle, r: ObjectRef, amount: u16, anti_recovery: bool) -> bool {
    let alliance = b.objects.get(r).alliance;
    // sub_802CE78: the opponent's defensive-chip record.
    if anti_recovery && b.chip_number(b.linked[(alliance ^ 1) as usize].chip) == Some(ANTI_RECOVERY) {
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
    let (panel, alliance) = {
        let o = b.objects.get(r);
        (o.panel, o.alliance)
    };
    // The controller spawns with the spawner's registers as its position
    // (panel Y, element 0, the defensive record's owner word) and
    // parameters (whatever the heal's caller left in r4); a dimming
    // controller's init never reads either.
    let pos = Vec3 { x: panel.y as i32, y: 0, z: 0 };
    let controller = crate::behavior::spawn_object(b, Pool::Effect, TRAP_CONTROLLER, pos, [0; 4]);
    if let Some(c) = controller {
        let damage = amount as u32 + (TRAP_HIT_PARAM << 16);
        let o = b.objects.get_mut(c);
        o.panel = panel;
        o.element = 0;
        o.related[0] = Some(r);
        o.alliance = alliance;
        o.damage = damage as u16;
        o.stamina = (damage >> 16) as u16;
        // (+0x30 also gets the trap's chip id, for the telop only.)
    }
    // sub_800BF16, no cut-in allowed, with the spawn's result (none when
    // the effect pool is full: the game registers a null controller).
    b.start_dimming(alliance, true, controller, r);
    // sub_800ABC6: the mark over the healer's panel, for the local side's
    // look (Param2).
    let (x, y) = crate::kinds::player::panel_coordinates(panel.x, panel.y);
    let local = b.round.local_side;
    effect::spawn(b, Vec3 { x, y: y.wrapping_add(0x10_0000), z: 0x20_0000 }, TRAP_MARK, local, 0, 0);
    b.play_sound(SoundId(TRAP_SOUND));
    b.clear_linked(alliance ^ 1);
}
