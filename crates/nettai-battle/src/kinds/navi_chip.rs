//! A navi chip's dimming controller (effect object #0x10,
//! `sub_80E17E8`): the dimming phases (`dimming`) with the navi chip's
//! own: after the dim, AntiNavi's check; after the name, the user warps
//! out, the chip's navi comes and acts, and the user warps back in. See
//! docs/engine/chips.md §3.6.7.

use nettai_content_api::{HookCall, NaviChipSpec};

use crate::battle::Battle;
use crate::kinds::{common, heal, navi_warp};
use crate::object::{ObjectRef, PanelPos, Vec3, state};
use crate::dimming::{self, DimmingChip};

/// What the controller needs to bring its navi.
#[derive(Clone, Debug, Default, Hash)]
pub struct Vars {
    /// The chip (the original keeps its navi's number, the chip's
    /// subtype, at object +0x19: the chip says what the controller asks of
    /// it).
    pub chip: DimmingChip,
    /// The damage word (object +0x2C).
    pub damage: u32,
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
    pub damage: u32,
    pub chip: DimmingChip,
}

/// `sub_80E192C`: the controller for `user`'s navi chip, on its panel.
/// (Its position is register garbage nothing reads.) Roll's chips (navi
/// 0, which heal) against the other side's armed AntiRecv spring the
/// trap instead: the controller is AntiRecv's counterattack.
pub fn spawn(b: &mut Battle, user: ObjectRef, s: Spec) -> Option<ObjectRef> {
    let side = b.objects.get(user).alliance;
    let heals = s.chip.chip.is_some_and(|h| b.content.chip(h).traits.has(crate::content::ChipTraits::HEALS));
    if heals && b.linked_trap(side ^ 1) == Some(crate::content::Trap::AntiRecovery) {
        return spring_anti_recovery(b, user, s);
    }
    let r = crate::kinds::spawn_engine(b, crate::kinds::EngineKind::NaviChip, Vec3::default(), [0; 4])?;
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
        damage: s.damage,
        navi_acting: false,
    });
    Some(r)
}

/// `loc_80E1968`: Roll against AntiRecv. The trap's mark over the user
/// (`sub_800ABC6`), the other side's record is spent (`sub_802CEA6`), and
/// AntiRecv's counterattack (`sub_80E37D2`) comes for the user with three
/// times Roll's damage (`sub_80E199A`) and hit parameter 0x1E. Action 0x1B registers it as the side's dimming, as it
/// would the navi chip's controller; unlike a recovery chip's heal
/// (`kinds::heal`), nothing starts one here.
fn spring_anti_recovery(b: &mut Battle, user: ObjectRef, s: Spec) -> Option<ObjectRef> {
    let side = b.objects.get(user).alliance;
    heal::trap_mark(b, user);
    b.clear_linked(side ^ 1);
    let damage = counterattack_damage(s.damage) + (heal::TRAP_HIT_PARAM << 16);
    // Its Z is the mark's, which `sub_800ABC6` left in r3.
    heal::spawn_counterattack(b, user, damage, heal::TRAP_MARK_Z)
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
/// A chip whose user stays (the original's navi 0x17) warps it neither
/// way; one whose navi brings the user back (navi 0) doesn't warp it in.
fn effect(b: &mut Battle, r: ObjectRef) {
    use crate::content::ChipTraits;
    let traits = b.content.chip(b.content.chip_or_zeroed(vars(b, r).chip.chip)).traits;
    let stays = traits.has(ChipTraits::USER_STAYS);
    match b.objects.get(r).phase {
        // sub_80E1854
        0 => {
            if b.objects.get(r).phase_init == 0 {
                let o = b.objects.get_mut(r);
                o.timer = 0x1E;
                o.phase_init = 4;
                if !stays {
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
                if stays || traits.has(ChipTraits::NAVI_RETURNS_USER) {
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
/// bonus: the chip's `navi` hook. (HackJack's and Django's entries of the
/// US games' table are null, and the game jumps to address 0; their chips'
/// hooks bring the Japanese games' navis. The game also records the last
/// navi chip used, `byte_203C960`, which nothing in a battle reads.)
fn bring_navi(b: &mut Battle, r: ObjectRef) {
    let v = vars(b, r).clone();
    let damage = v.damage.wrapping_add(v.chip.bonus as u32);
    let o = b.objects.get(r);
    let (panel, element) = (o.panel, o.element);
    let user = user(b, r);
    use crate::content::ChipUsage;
    let chip = b.content.chip_or_zeroed(v.chip.chip);
    let navi = match b.content.defs.chip(chip).usage {
        ChipUsage::Navi(hook) => {
            let spec = NaviChipSpec { panel, element, damage };
            crate::behavior::call_hook(b, hook, HookCall::NaviChip { user, controller: r, spec }).object()
        }
        u => panic!("chip {:?} is a navi chip's, but it is used as {u:?}", b.content.defs.chip(chip).key),
    };
    // The spawner sets the flag, through the pointer it hands the navi.
    vars_mut(b, r).navi_acting = navi.is_some();
}
