//! Navis no player controls (actor type navi): EXE5's Chaos Unison failure
//! brings one for the other side, Dark MegaMan (docs/design/exe5-map.md
//! §15.9). It is the player kind's object (actor object 0) with an actor
//! record of type navi, so its init, its tick and its idle are the navi
//! type's (EXE5's 0x080F2228: 0x08013BF6, 0x080F224C; EXE6's `sub_8016F56`
//! and `sub_80F2354`'s family), its reactions and attacks the player's.
//! It is on its side's list of alive actors but not counted: neither its
//! coming nor its deletion changes how many navis the side has, so the
//! round goes on and ends with the side's player. No input reaches it: the
//! rules of its summoner's side drive it (their `controller`, with the
//! navi's own state, their `navi_state`).

use super::{ai, ai_mut, face_toward, intake, panel_coordinates, set_flag1, status, NaviAction};
use crate::actor::ActorType;
use crate::battle::Battle;
use crate::object::{ObjectRef, PanelPos, Vec3, flags, state};
use nettai_content_api::IdentityHandle;

/// Who drives a navi no player controls: the rules of a side (their
/// `controller` and `navi_state`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Controller {
    pub side: u8,
}

/// The navi type's palette by actor record version (`sub_800F334`'s
/// `byte_800F354`, both games'), times the body's palette scale.
const PALETTE_BY_VERSION: [u8; 8] = [0, 0, 0, 0, 3, 1, 0, 0];

/// EXE5's 0x08006AAE (EXE6's `sub_80076A0` with its "not counted" flag):
/// the navi `identity` (its actor record of type navi, its `body`) comes
/// onto (x, y) for `side`, `summoner`'s doing, driven by the rules (the
/// summoner's side's, else `side`'s). None when the object or
/// actor pools or the side's list of alive actors (four slots) are full.
pub(crate) fn spawn(
    b: &mut Battle,
    identity: IdentityHandle,
    panel: PanelPos,
    side: u8,
    summoner: Option<ObjectRef>,
) -> Result<Option<ObjectRef>, String> {
    let content = b.content.clone();
    let id = content.identity(Some(identity));
    let Some(body) = id.body.as_ref() else {
        return Err(format!("identity {} has no `body`: it is no navi a battle can bring", id.key));
    };
    if id.record.actor_type != ActorType::Navi {
        return Err(format!("identity {} is no navi's (its actor type is {:?})", id.key, id.record.actor_type));
    }
    // The driving rules' side, and the navi's state.
    let ruling_side = summoner.map_or(side, |s| b.objects.get(s).alliance) & 1;
    if !b.has_rules(ruling_side) {
        return Err(format!("side {ruling_side} plays by no rules: none drives a navi for it"));
    }
    let Some(navi_state) = content.defs.rules().and_then(|r| r.navi_state) else {
        return Err("the rules have no `navi_state`: they drive no navi".into());
    };
    let (x, y) = panel_coordinates(panel.x, panel.y);
    let Some(r) = crate::kinds::spawn_engine(b, crate::kinds::EngineKind::Player, Vec3 { x, y, z: 0 }, [0; 4]) else {
        return Ok(None);
    };
    b.objects.get_mut(r).alliance = side;
    let Some(a) = b.actors.allocate() else {
        b.objects.free(r);
        return Ok(None);
    };
    b.objects.get_mut(r).actor = Some(a);
    // AIData+2 and Param2: not counted.
    b.actors.get_mut(a).not_counted = 1;
    b.objects.get_mut(r).params[1] = 1;
    // sub_8007778: onto the side's list (not counted, so the side's count
    // stays).
    let Some(place) = b.round.alive_actors[side as usize].iter().position(|s| s.is_none()) else {
        b.objects.free(r);
        b.actors.free(a);
        return Ok(None);
    };
    b.round.alive_actors[side as usize][place] = Some(r);
    let o = b.objects.get_mut(r);
    o.flags |= flags::RUN_WHILE_PAUSED;
    o.identity = Some(identity);
    o.hp = body.hp;
    o.max_hp = body.hp;
    o.element = body.element;
    o.damage = body.body_damage;
    o.stamina = 10;
    o.panel = panel;
    o.future_panel = panel;
    o.chip = None;
    let size = content.defs.schema(navi_state).size();
    b.objects.set_state(r, navi_state, size);
    let ad = b.actors.get_mut(a);
    ad.actor_type = id.record.actor_type;
    ad.ai_index = id.record.ai_index;
    ad.identity = Some(identity);
    ad.summoner = summoner;
    ad.controller = Some(Controller { side: ruling_side });
    Ok(Some(r))
}

/// EXE5's 0x080F2228: the state, then the sprite steps (`object_updateSprite`).
pub(super) fn update(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).state {
        state::INIT => init(b, r),
        state::UPDATE => tick(b, r),
        _ => super::destroy(b, r),
    }
    crate::kinds::common::update_sprite(b, r);
}

/// EXE5's 0x08013BF6 (EXE6's `sub_8016F56`): its sprite (the body's, its
/// animation 0 from the start, its palette by version), its collision as
/// its body says, its starting flags, its target the other side's player
/// (whom it faces), its identity's parts; then its post-init hook's palette
/// and its HP number gone.
fn init(b: &mut Battle, r: ObjectRef) {
    // (sub_800F35C: the navi type's per-AI init hooks are all empty,
    // 0x080F2458.)
    let content = b.content.clone();
    let id = content.identity(b.objects.get(r).identity);
    let body = id.body.as_ref().expect("a navi no player controls has a body (its spawn checked)");
    let flip = b.objects.get(r).alliance ^ b.objects.get(r).flip;
    let palette = PALETTE_BY_VERSION[id.record.version as usize & 7].wrapping_mul(body.palette_scale);
    let sprite = b.objects.sprite_mut(r);
    sprite.load(body.sprite);
    // sprite_hasShadow, or sprite_noShadow (drawn with the sprite).
    sprite.look.shadow = if body.shadow { crate::object::sprite::Shadow::Ground } else { crate::object::sprite::Shadow::WithSprite };
    sprite.look.palette = palette;
    sprite.look.set_flip(flip);
    let o = b.objects.get_mut(r);
    o.anim = 0;
    o.anim_loaded = 0xFF;
    o.flags &= !flags::NO_SPRITE_UPDATE;
    crate::kinds::common::set_animation(b, r, 0);
    if b.create_collision(r).is_none() {
        b.objects.free(r);
        return;
    }
    let (self_type, target_type, hit_mod) = body.collision;
    b.setup_collision(r, self_type, target_type, hit_mod);
    set_flag1(b, r, body.flags);
    // (sub_801DB84: its HP number's place on the HUD; the post-init hook
    // takes it off again for a body that hides its HP.)
    if body.hides_hp {
        b.hide_hp(r);
    }
    // sub_80103BC(other side), sub_800F318: its target, the other side's
    // player.
    let side = b.objects.get(r).alliance;
    let target = b.player(side ^ 1);
    ai_mut(b, r).target = target;
    if matches!(b.panel_pattern(), 0x38 | 0x30 | 0x3C) {
        // sub_800F2C6: facing its side's way.
        b.objects.get_mut(r).flip = 0;
        let facing = b.objects.get(r).alliance;
        b.objects.sprite_mut(r).look.set_flip(facing);
    } else if let Some(t) = target {
        // sub_800F2F0
        face_toward(b, r, t);
    }
    // sub_8010DD0: its identity's init hook (what it wears).
    let identity = b.objects.get(r).identity;
    super::form::navi_init_hook(b, r, identity);
    // (AIData+0x0C, the other player's max HP / 100 from 1 to 10, is read
    // only for actors of version 4: EXE5's 0x0800D396 makes their HP from
    // it; no navi here has that version, its load refuses it.)
    // sub_800F378: its post-init hook (EXE5's Dark MegaMan's 0x08104270:
    // palette 1; its flag and its HP number are the body's).
    if let Some(p) = body.palette {
        b.objects.sprite_mut(r).look.palette = p;
    }
    b.objects.get_mut(r).state = state::UPDATE;
    super::set_action(b, r, NaviAction::Entry);
}

/// EXE5's 0x080F224C: its hits (0x080F22E8's entry: the navi intake,
/// 0x08017688), its damage, reactions and action (`sub_801AF44`), its
/// driver's tick (0x080F23FC's entry: the side's rules' `navi_tick`), its
/// chip lockout running down, the hit statistics (0x080F2624), its
/// collision presented.
fn tick(b: &mut Battle, r: ObjectRef) {
    intake::collect_hits_navi(b, r);
    status::update(b, r);
    if ai(b, r).ticked {
        let side = ai(b, r).controller.map_or(b.objects.get(r).alliance, |c| c.side);
        b.rules_navi_tick(side, r);
    }
    let a = ai_mut(b, r);
    a.lockout = a.lockout.saturating_sub(1);
    hit_statistics(b, r);
    if let Some(c) = b.objects.get(r).collision {
        b.present_collision(c);
    }
}

/// EXE5's 0x080F2624: a counter hit on it, and each byte of the bugs its
/// hits inflicted, count for the other side (0x0802AEA6: its per-player
/// battle record's first four counters, at most 10 each, and its
/// statistic 7).
fn hit_statistics(b: &mut Battle, r: ObjectRef) {
    let c = super::coll(b, r);
    let counter = c.acc.hit_flags & 0x40 != 0;
    // (The word the original sums from its hitters' +0x14 holds their bug
    // codes and arguments, then barrier bytes no attack has.)
    let bugs = (c.acc.inflicted_bugs as u32).to_le_bytes();
    let opp = b.objects.get(r).alliance ^ 1;
    if counter {
        count_hit(b, opp, 0);
    }
    for (i, &byte) in bugs.iter().enumerate() {
        if byte != 0 {
            count_hit(b, opp, i);
        }
    }
}

/// 0x0802AEA6: one more of the side's counter `i` (none past 10), and its
/// statistic 7.
fn count_hit(b: &mut Battle, side: u8, i: usize) {
    let n = &mut b.navi_hit_counts[side as usize][i];
    if *n + 1 > 10 {
        return;
    }
    *n += 1;
    b.bump_side_stat(side, 7, 1);
}

/// Its idle (the navi type's action 6, EXE5's 0x08104210 for Dark MegaMan):
/// its driver decides all of it.
pub(super) fn idle(b: &mut Battle, r: ObjectRef) {
    if let Some(c) = ai(b, r).controller {
        b.rules_controller_answer(c.side, r);
    }
}

/// The navi type's deletion (its action 2: EXE5's 0x08013D24), not while
/// paused.
pub(super) fn deletion(b: &mut Battle, r: ObjectRef) {
    if b.paused {
        return;
    }
    match b.objects.get(r).phase {
        0 => begin_deletion(b, r),
        4 => flash_out(b, r),
        p => panic!("the navi type's deletion phase {p:#x} reads past its table (0x08013D3C)"),
    }
}

/// EXE5's 0x08013D44 (the player's deletion's first step without its
/// music): no collision region, its status visuals, the dive, the charge,
/// no chips, its barrier; off the side's lists (counted or not, the slot);
/// a side tracking it tracks another.
fn begin_deletion(b: &mut Battle, r: ObjectRef) {
    let c = super::coll_mut(b, r);
    c.region = None;
    // sub_801A5E2
    c.links[crate::collision::link::CONFUSE] = None;
    c.links[crate::collision::link::BLIND] = None;
    super::cancel_submerged(b, r);
    super::reset_charge(b, r);
    let o = b.objects.get_mut(r);
    o.chips_held = 0;
    o.chip = None;
    let flip = o.alliance ^ o.flip;
    b.objects.sprite_mut(r).look.set_flip(flip);
    // sub_801A7F4
    super::coll_mut(b, r).barrier = 0;
    ai_mut(b, r).barrier_visual = None;
    if b.objects.get(r).params[1] < 1 {
        super::reactions::remove_from_alive(b, r);
    }
    // sub_802EF5C
    crate::kinds::obstacle::release_tracking(b, r);
    let o = b.objects.get_mut(r);
    o.phase = 4;
    o.phase_init = 0;
}

/// EXE5's 0x08013D82: once not dimmed, its deletion animation, its
/// sparkles (the role hook `navi_deleted`), the deletion flash for 90
/// ticks over it, 90 ticks of blinking white; then its reservation, its
/// sparkles and its parts go, it leaves every list slot, and its object
/// is destroyed.
fn flash_out(b: &mut Battle, r: ObjectRef) {
    if b.objects.get(r).phase_init == 0 {
        if b.is_dimmed() {
            return;
        }
        let o = b.objects.get_mut(r);
        o.prevent_anim = 0;
        o.related[0] = None;
        o.anim = 2;
        ai_mut(b, r).overlay = None;
        if let Some(hook) = b.roles().try_hook(crate::content::HookRole::NaviDeleted) {
            let v = crate::behavior::call_hook(b, hook, nettai_content_api::HookCall::RoleNavi { navi: r });
            if let nettai_content_api::Value::Object(s) = v {
                ai_mut(b, r).deletion_sparkles = Some(s);
            }
        }
        let pos = b.objects.get(r).pos;
        let look = b.roles().effect(crate::content::EffectRole::Deletion);
        let at = Vec3 { z: pos.z.wrapping_add(0x20_0000), ..pos };
        if let Some(e) = crate::kinds::effect::spawn(b, at, look, 0, 0, 0) {
            b.objects.get_mut(e).timer = 90;
        }
        b.objects.get_mut(r).timer = 90;
        // 0x0800DFC0: no bubble.
        super::coll_mut(b, r).status_timers[crate::collision::timer::BUBBLE] = 0;
        super::clear_flag1(b, r, crate::collision::f1::BUBBLED);
        b.objects.get_mut(r).phase_init = 4;
    }
    let o = b.objects.get_mut(r);
    let t = o.timer as i32 - 1;
    o.timer = t as u16;
    if t >= 0 {
        if t & 2 != 0 {
            b.objects.sprite_mut(r).look.white = true;
        }
        return;
    }
    // sub_802CDD0: the side's damage-carry record forgets it.
    let side = b.objects.get(r).alliance as usize;
    if b.damage_carry[side].target == Some(r) {
        b.damage_carry[side].target = None;
    }
    let fp = b.objects.get(r).future_panel;
    b.unreserve_panel(r, fp.x, fp.y);
    // sub_80E1A86: its sparkles end.
    if let Some(s) = ai_mut(b, r).deletion_sparkles.take()
        && b.objects.is_allocated(s)
    {
        b.objects.get_mut(s).state = state::DESTROY;
    }
    // sub_8011020: its death hook.
    let identity = b.objects.get(r).identity;
    super::form::navi_death_hook(b, r, identity);
    // 0x08006BC2: out of every slot of the alive lists.
    for slot in b.round.alive_actors.iter_mut().flatten() {
        if *slot == Some(r) {
            *slot = None;
        }
    }
    let o = b.objects.get_mut(r);
    o.set_visible(false);
    o.state = state::DESTROY;
    o.action = 0;
    o.phase = 0;
    o.phase_init = 0;
}

/// Whether `r` is a navi no player controls.
pub(super) fn is_ai_navi(b: &Battle, r: ObjectRef) -> bool {
    b.objects.get(r).actor.is_some_and(|a| b.actors.get(a).actor_type == ActorType::Navi)
}
