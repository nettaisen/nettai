//! The trap chips' dimming controller (effect object #0x2A,
//! `sub_80E34C0`): AntiDmg and its kin. The usual dimming phases
//! (`dimming`), with the telop hidden, and an effect that
//! registers the trap as the side's defensive chip (the linked record);
//! the trap springs from the damage intake. See docs/engine/chips.md
//! §3.6.9.

use crate::battle::{Battle, LinkedRecord};
use crate::kinds::common;
use crate::object::{ObjectRef, PanelPos, Pool, Vec3, state};
use crate::dimming::{self, DimmingChip};

pub const INDEX: u8 = 0x2A;

/// The controller's own state.
#[derive(Clone, Debug, Default, Hash)]
pub struct Vars {
    pub chip: DimmingChip,
    /// The damage word (object +0x2C), kept for the counterattack.
    pub damage: u32,
}

fn vars(b: &Battle, r: ObjectRef) -> &Vars {
    match &b.objects.get(r).vars {
        crate::kinds::Vars::TrapChip(v) => v,
        v => panic!("trap chip controller with {v:?}"),
    }
}

/// `sub_80E353E`: the controller for `user`, on its panel. (Its position
/// is register garbage nothing reads.)
pub fn spawn(
    b: &mut Battle,
    user: ObjectRef,
    element: u8,
    params: [u8; 4],
    damage: u32,
    chip: DimmingChip,
) -> Option<ObjectRef> {
    let r = b.objects.spawn(Pool::Effect, INDEX, Vec3::default(), params)?;
    let (panel, alliance) = {
        let o = b.objects.get(user);
        (o.panel, o.alliance)
    };
    let o = b.objects.get_mut(r);
    o.panel = PanelPos { x: panel.x, y: panel.y };
    o.element = element;
    o.related[0] = Some(user);
    o.alliance = alliance;
    o.vars = crate::kinds::Vars::TrapChip(Vars { chip, damage });
    Some(r)
}

pub fn update(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).state {
        state::INIT => dimming::begin(b, r),
        state::UPDATE => match b.objects.get(r).action {
            0 => dimming::dim_screen(b, r),
            4 => dimming::show_hidden_telop(b, r),
            8 => effect(b, r),
            _ => dimming::undim_screen(b, r),
        },
        _ => dimming::end(b, r),
    }
}

/// `sub_80E3504`: the side's previous defensive chip goes; this one takes
/// its place (with its object, for the kinds that have one); 61 ticks.
fn effect(b: &mut Battle, r: ObjectRef) {
    if b.objects.get(r).phase_init == 0 {
        b.objects.get_mut(r).timer = 0x3C;
        let side = b.objects.get(r).alliance;
        b.clear_linked(side);
        // sub_80E3560: only Param1 0 has an object (sub_80CE0EC).
        if b.objects.get(r).params[0] == 0 {
            panic!("the trap chip object (sub_80CE0EC) is not implemented yet");
        }
        let v = vars(b, r).clone();
        let owner = b.objects.get(r).related[0];
        // sub_802CE8A
        b.linked[side as usize] =
            LinkedRecord { chip: v.chip.chip, bonus: v.chip.bonus, damage: v.damage, owner, object: None };
        b.objects.get_mut(r).phase_init = 4;
    }
    let o = b.objects.get_mut(r);
    let left = o.timer as i32 - 1;
    o.timer = left as u16;
    if left < 0 {
        common::set_action(b, r, 0xC);
    }
}
