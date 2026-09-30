//! A navi chip's time-freeze controller (effect object #0x10,
//! `sub_80E17E8`): the freeze phases (`time_freeze`) with the navi chip's
//! own: after the dim, AntiNavi's check; after the name, the user warps
//! out, the chip's navi comes and acts, and the user warps back in. See
//! docs/engine/chips.md §3.6.7.

use crate::battle::Battle;
use crate::kinds::{common, elmnt_man, erase_man, navi_warp};
use crate::object::{ObjectRef, PanelPos, Pool, Vec3, state};
use crate::time_freeze::{self, FreezeChip};

pub const INDEX: u8 = 0x10;

/// What the controller needs to bring its navi.
#[derive(Clone, Debug, Default, Hash)]
pub struct Vars {
    pub chip: FreezeChip,
    /// Which navi (`off_802CD5C`; object +0x19, the chip's subtype).
    pub navi: u8,
    /// The damage word (object +0x2C).
    pub damage: u32,
    /// The chip's parameters (object +4).
    pub params: [u8; 4],
    /// Object +0x18: set while the navi acts; the navi clears it when it
    /// leaves.
    pub navi_acting: bool,
}

fn vars(b: &Battle, r: ObjectRef) -> &Vars {
    match &b.objects.get(r).vars {
        crate::kinds::Vars::NaviChip(v) => v,
        v => panic!("navi chip controller with {v:?}"),
    }
}

fn vars_mut(b: &mut Battle, r: ObjectRef) -> &mut Vars {
    match &mut b.objects.get_mut(r).vars {
        crate::kinds::Vars::NaviChip(v) => v,
        v => panic!("navi chip controller with {v:?}"),
    }
}

/// What a navi chip's controller is spawned with.
#[derive(Clone, Copy, Debug)]
pub struct Spec {
    pub element: u8,
    pub navi: u8,
    pub params: [u8; 4],
    pub damage: u32,
    pub chip: FreezeChip,
}

/// `sub_80E192C`: the controller for `user`'s navi chip, on its panel.
/// (Its position is register garbage nothing reads.)
pub fn spawn(b: &mut Battle, user: ObjectRef, s: Spec) -> Option<ObjectRef> {
    if s.navi == 0 && b.linked[(b.objects.get(user).alliance ^ 1) as usize].chip == 0xBD {
        panic!("navi chip 0 against the other side's chip 0xBD (sub_80E192C) is not implemented yet");
    }
    let r = b.objects.spawn(Pool::Effect, INDEX, Vec3::default(), s.params)?;
    let (panel, alliance, flip) = {
        let o = b.objects.get(user);
        (o.panel, o.alliance, o.flip)
    };
    let o = b.objects.get_mut(r);
    o.panel = PanelPos { x: panel.x, y: panel.y };
    o.element = s.element;
    o.related[0] = Some(user);
    o.alliance = alliance;
    o.flip = flip;
    o.vars = crate::kinds::Vars::NaviChip(Vars {
        chip: s.chip,
        navi: s.navi,
        damage: s.damage,
        params: s.params,
        navi_acting: false,
    });
    Some(r)
}

/// The navi is done (`sub_80BADE4` and the like write 0 through the
/// pointer they were given).
pub fn navi_left(b: &mut Battle, controller: ObjectRef) {
    vars_mut(b, controller).navi_acting = false;
}

pub fn update(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).state {
        state::INIT => time_freeze::begin(b, r),
        state::UPDATE => {
            let chip = vars(b, r).chip.chip;
            match b.objects.get(r).action {
                0 => time_freeze::dim_screen(b, r),
                4 => time_freeze::check_anti_navi(b, r, chip),
                8 => time_freeze::show_navi_name(b, r, chip),
                0xC => effect(b, r),
                _ => time_freeze::undim_screen(b, r),
            }
        }
        _ => time_freeze::end(b, r),
    }
}

fn user(b: &Battle, r: ObjectRef) -> ObjectRef {
    b.objects.get(r).related[0].expect("navi chip controller has a user")
}

/// Count the timer down; true once it went below 0.
fn timer_ran_out(b: &mut Battle, r: ObjectRef) -> bool {
    let o = b.objects.get_mut(r);
    let left = o.timer as i32 - 1;
    o.timer = left as u16;
    left < 0
}

/// Next phase, not yet entered.
fn set_phase(b: &mut Battle, r: ObjectRef, phase: u8) {
    let o = b.objects.get_mut(r);
    o.phase = phase;
    o.phase_init = 0;
}

/// `sub_80E1830`: the user warps out (30 ticks), the navi acts until it
/// leaves, 30 ticks, the user warps back in (30 ticks), then the undim.
/// Navi 0x17 leaves the user in place.
fn effect(b: &mut Battle, r: ObjectRef) {
    let navi = vars(b, r).navi;
    match b.objects.get(r).phase {
        // sub_80E1854
        0 => {
            if b.objects.get(r).phase_init == 0 {
                let o = b.objects.get_mut(r);
                o.timer = 0x1E;
                o.phase_init = 4;
                if navi != 0x17 {
                    let u = user(b, r);
                    navi_warp::spawn(b, u, navi_warp::Warp::Out);
                }
            }
            if timer_ran_out(b, r) {
                set_phase(b, r, 4);
            }
        }
        // sub_80E1880
        4 => {
            if b.objects.get(r).phase_init == 0 {
                bring_navi(b, r);
                b.objects.get_mut(r).phase_init = 4;
            }
            if !vars(b, r).navi_acting {
                set_phase(b, r, 8);
            }
        }
        // sub_80E18DA
        8 => {
            if b.objects.get(r).phase_init == 0 {
                let o = b.objects.get_mut(r);
                o.timer = 0x1E;
                o.phase_init = 4;
            }
            if timer_ran_out(b, r) {
                set_phase(b, r, 0xC);
            }
        }
        // sub_80E18F8
        _ => {
            if b.objects.get(r).phase_init == 0 {
                if navi == 0 || navi == 0x17 {
                    return common::set_action(b, r, 0x10);
                }
                let u = user(b, r);
                navi_warp::spawn(b, u, navi_warp::Warp::In);
                let o = b.objects.get_mut(r);
                o.timer = 0x1E;
                o.phase_init = 4;
            }
            if timer_ran_out(b, r) {
                common::set_action(b, r, 0x10);
            }
        }
    }
}

/// `off_802CD5C[navi]`: bring the chip's navi, with the damage and the
/// bonus. (The game also records the last navi chip used, `byte_203C960`,
/// for chips that copy it.)
fn bring_navi(b: &mut Battle, r: ObjectRef) {
    let v = vars(b, r).clone();
    let damage = v.damage.wrapping_add(v.chip.bonus as u32);
    let o = b.objects.get(r);
    let (panel, element) = (o.panel, o.element);
    let user = user(b, r);
    let navi = match v.navi {
        elmnt_man::NAVI => elmnt_man::spawn(b, user, r, panel, element, v.params, damage),
        erase_man::NAVI => erase_man::spawn(b, user, r, panel, element, v.params, damage),
        n => panic!("navi chip navi {n:#x} (off_802CD5C) is not implemented yet"),
    };
    // The spawner sets the flag, through the pointer it hands the navi.
    vars_mut(b, r).navi_acting = navi.is_some();
}
