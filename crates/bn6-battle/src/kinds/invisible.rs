//! The Invisibl chip's time-freeze controller (effect object #0x5D,
//! `sub_80E74D4`): the usual freeze phases (`time_freeze`), with an effect
//! that makes the user invisible, then 31 ticks before time starts again.
//! See docs/engine/chips.md §3.6.

use crate::battle::Battle;
use crate::collision::{f1, timer};
use crate::kinds::common;
use crate::object::{ObjectRef, PanelPos, Pool, Vec3, state};
use crate::time_freeze::{self, FreezeChip};

pub const INDEX: u8 = 0x5D;

/// The controller's own state.
#[derive(Clone, Debug, Default, Hash)]
pub struct Vars {
    pub chip: FreezeChip,
    /// Param1-2: how long the user stays invisible, in ticks.
    pub duration: u16,
}

fn vars(b: &Battle, r: ObjectRef) -> &Vars {
    match &b.objects.get(r).vars {
        crate::kinds::Vars::Invisible(v) => v,
        v => panic!("Invisibl controller with {v:?}"),
    }
}

/// `sub_80E7546`: the controller for `user`, on its panel. (Its position
/// is register garbage nothing reads.)
pub fn spawn(b: &mut Battle, user: ObjectRef, element: u8, params: [u8; 4], chip: FreezeChip) -> Option<ObjectRef> {
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
    if let crate::kinds::Vars::Invisible(v) = &mut o.vars {
        v.chip = chip;
        v.duration = u16::from_le_bytes([params[0], params[1]]);
    }
    Some(r)
}

pub fn update(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).state {
        state::INIT => time_freeze::begin(b, r),
        state::UPDATE => match b.objects.get(r).action {
            0 => time_freeze::dim_screen(b, r),
            4 => time_freeze::show_chip_name(b, r),
            8 => effect(b, r),
            _ => time_freeze::undim_screen(b, r),
        },
        _ => time_freeze::end(b, r),
    }
}

/// `sub_80E7518`: the user turns invisible (`sub_8010474`: it flashes for
/// the duration), then 31 ticks pass.
fn effect(b: &mut Battle, r: ObjectRef) {
    if b.objects.get(r).phase_init == 0 {
        let duration = vars(b, r).duration;
        let user = b.objects.get(r).related[0].expect("Invisibl controller has a user");
        let c = b.objects.get(user).collision.expect("the user has collision data");
        let cd = b.collision.get_mut(c);
        cd.status_timers[timer::FLASH] = duration;
        cd.f1 |= f1::INVISIBLE;
        b.play_sound(crate::sound::SoundId(0x93));
        let o = b.objects.get_mut(r);
        o.timer = 0x1E;
        o.phase_init = 4;
    }
    let o = b.objects.get_mut(r);
    let left = o.timer as i32 - 1;
    o.timer = left as u16;
    if left < 0 {
        common::set_action(b, r, 0xC);
    }
}
