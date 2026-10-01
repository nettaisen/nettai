//! A navi chip's dimming controller (effect object #0x10,
//! `sub_80E17E8`): the dimming phases (`dimming`) with the navi chip's
//! own: after the dim, AntiNavi's check; after the name, the user warps
//! out, the chip's navi comes and acts, and the user warps back in. See
//! docs/engine/chips.md §3.6.7.

use bn6_content_api::{Hook, HookCall, NaviChipSpec};

use crate::battle::Battle;
use crate::kinds::{common, heal, navi_warp};
use crate::object::{ObjectRef, PanelPos, Vec3, state};
use crate::dimming::{self, DimmingChip};

/// What the controller needs to bring its navi.
#[derive(Clone, Debug, Default, Hash)]
pub struct Vars {
    pub chip: DimmingChip,
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
    pub chip: DimmingChip,
}

/// `sub_80E192C`: the controller for `user`'s navi chip, on its panel.
/// (Its position is register garbage nothing reads.) Roll's chips (navi
/// 0, which heal) against the other side's armed AntiRecv spring the
/// trap instead: the controller is AntiRecv's counterattack.
pub fn spawn(b: &mut Battle, user: ObjectRef, s: Spec) -> Option<ObjectRef> {
    let side = b.objects.get(user).alliance;
    if s.navi == ROLL && b.chip_number(b.linked[(side ^ 1) as usize].chip) == Some(heal::ANTI_RECOVERY) {
        return spring_anti_recovery(b, user, s);
    }
    let r = crate::kinds::spawn_engine(b, crate::kinds::EngineKind::NaviChip, Vec3::default(), s.params)?;
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

/// Roll's navi (`off_802CD5C[0]`), whose chips heal.
const ROLL: u8 = 0;

/// `loc_80E1968`: Roll against AntiRecv. The trap's mark over the user
/// (`sub_800ABC6`), the other side's record is spent (`sub_802CEA6`), and
/// AntiRecv's counterattack (`sub_80E37D2`) comes for the user with three
/// times Roll's damage (`sub_80E199A`) and hit parameter 0x1E, in the
/// chip's parameters. Action 0x1B registers it as the side's dimming, as it
/// would the navi chip's controller; unlike a recovery chip's heal
/// (`kinds::heal`), nothing starts one here.
fn spring_anti_recovery(b: &mut Battle, user: ObjectRef, s: Spec) -> Option<ObjectRef> {
    let side = b.objects.get(user).alliance;
    heal::trap_mark(b, user);
    b.clear_linked(side ^ 1);
    let damage = counterattack_damage(s.damage) + (heal::TRAP_HIT_PARAM << 16);
    // Its Z is the mark's, which `sub_800ABC6` left in r3.
    heal::spawn_counterattack(b, user, damage, s.params, heal::TRAP_MARK_Z)
}

/// `sub_80E199A`: three times the damage word's damage (its low 11 bits),
/// doubled first when it carries the double-damage flag (0x8000).
fn counterattack_damage(word: u32) -> u32 {
    let d = word & 0x7FF;
    let d = if word & 0x8000 != 0 { d * 2 } else { d };
    d * 3
}

/// The navi is done (`sub_80BADE4` and the like write 0 through the
/// pointer they were given).
pub fn navi_left(b: &mut Battle, controller: ObjectRef) {
    vars_mut(b, controller).navi_acting = false;
}

pub fn update(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).state {
        state::INIT => dimming::begin(b, r),
        state::UPDATE => {
            let chip = vars(b, r).chip.chip;
            match b.objects.get(r).action {
                0 => dimming::dim_screen(b, r),
                4 => dimming::check_anti_navi(b, r, chip),
                8 => dimming::show_navi_telop(b, r, chip),
                0xC => effect(b, r),
                _ => dimming::undim_screen(b, r),
            }
        }
        _ => dimming::end(b, r),
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
/// bonus: the content pack's script for the navi (`Hook::NaviChip`). (The
/// game also records the last navi chip used, `byte_203C960`, which
/// nothing in a battle reads.)
fn bring_navi(b: &mut Battle, r: ObjectRef) {
    let v = vars(b, r).clone();
    let damage = v.damage.wrapping_add(v.chip.bonus as u32);
    let o = b.objects.get(r);
    let (panel, element) = (o.panel, o.element);
    let user = user(b, r);
    // The chip's own navi, if content defines the chip; else
    // off_802CD5C, by the subtype.
    let defined = match v.chip.chip.and_then(|c| b.content.defs.chip(c).usage) {
        Some(crate::content::ChipUsage::Navi(f)) => Some(f),
        Some(u) => panic!(
            "chip {:?} is a navi chip, but its definition uses it as {u:?}",
            b.content.defs.chip(v.chip.chip.expect("a defined chip")).key
        ),
        None => None,
    };
    let navi = match defined.or_else(|| b.content.defs.hook(Hook::NaviChip(v.navi))) {
        Some(hook) => {
            let spec = NaviChipSpec { panel, element, params: v.params, damage };
            crate::behavior::call_hook(b, hook, HookCall::NaviChip { user, controller: r, spec }).object()
        }
        // HackJack's and Django's entries are NULL: the game jumps to
        // address 0.
        None if matches!(v.navi, 0x12 | 0x13) => {
            panic!("navi chip navi {:#x} is NULL in off_802CD5C (the game jumps to address 0)", v.navi)
        }
        None => panic!("content error: no script implements navi chip subtype {} (off_802CD5C)", v.navi),
    };
    // The spawner sets the flag, through the pointer it hands the navi.
    vars_mut(b, r).navi_acting = navi.is_some();
}
