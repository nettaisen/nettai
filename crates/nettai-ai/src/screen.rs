//! Custom screen planner: combinatorial hand selection (with Program Advance
//! detection, setup→payoff combo ordering, and optimal modifier folding),
//! matchup-aware form selection (Crosses / Beast Out / EXE4 & EXE5 Soul Unison),
//! and generic button probing (re-deal, capsules).

use crate::fight::LevelConfig;
use crate::nav::{self, Navigator, ScreenAction};
use crate::sim::{navi_hp, navi_max_hp, navi_ref, seen_to, step_ticks};
use nettai_battle::battle::BattleFlags;
use nettai_battle::content::{
    ButtonView, ChipClass, ChipCode, ChipData, ChipFlags, ChipMatch, ChipTraits, Content, Element, FormData, FormTraits, Recipe,
};
use nettai_battle::custom::builder::{self, Marks, Pick};
use nettai_battle::custom::screen::{ButtonCell, FORM_LIST_ENTRIES, MAX_SELECTIONS, OK_SLOT, Phase, SLOTS, Screen, SlotKind, SlotState};
use nettai_battle::custom::{FolderChip, Layer};
use nettai_battle::input::ScreenChoice;
use nettai_battle::{Battle, PlayerTick};
use nettai_content_api::{ChipHandle, FormHandle};
use nettai_netplay::rng::SplitMix64;
use std::collections::VecDeque;

const MAX_SCREEN_DECISIONS: u16 = 20;

/// Planner state for one custom screen visit.
#[derive(Clone, Debug, Default)]
pub struct ScreenPlanner {
    probed: bool,
    decisions: u16,
    queue: VecDeque<ScreenAction>,
    tried_form_list: bool,
    tried_soul_offer: bool,
    tried_redeal: bool,
    probed_buttons: u16,
}

impl ScreenPlanner {
    pub fn reset(&mut self) {
        *self = ScreenPlanner::default();
    }

    /// Choose the next [`ScreenAction`] while the screen is choosing and the navigator is idle.
    pub fn decide(
        &mut self,
        b: &Battle,
        side: u8,
        layer: &Layer,
        nav: &mut Navigator,
        cfg: LevelConfig,
        rng: &mut SplitMix64,
    ) -> Option<ScreenAction> {
        let s = &b.custom.sides[side as usize];
        let screen = s.screen.as_ref()?;
        if !nav::chooses(screen, layer) {
            return None;
        }

        if !self.probed {
            self.probed = true;
            nav.opener = nav::probe_opener(b, layer, side);
        }

        // Hard safety cap so the AI can never stall a custom screen.
        if self.decisions >= MAX_SCREEN_DECISIONS {
            self.decisions += 1;
            return Some(ScreenAction::Slot(OK_SLOT));
        }

        let layout = &b.game_rules().custom_screen;
        let legal_s = nav::legal_slots(screen, layer, layout, nav.refused, false);
        let legal_f = nav::legal_forms(b, side, layer, nav);
        let picked_chips = screen
            .selection()
            .iter()
            .filter(|&&sl| matches!(screen.slots[sl as usize].kind, SlotKind::Chip { .. }))
            .count();

        // Drain queued planned actions while they remain legal.
        if let Some(&front) = self.queue.front() {
            // If we're queued to press a FormOffer button immediately after picking our sacrifice chip, let it run!
            let is_queued_button = matches!(front, ScreenAction::Slot(sl) if matches!(screen.slots.get(sl as usize).map(|s| s.kind), Some(SlotKind::Button { .. })));
            if !is_queued_button && picked_chips > 0 {
                // Check if a newly-enabled non-trade button (e.g. EXE5 Capsules) became selectable.
                if let Some(btn_slot) = self.pick_post_pick_button(b, side, screen, &legal_s, cfg, rng) {
                    self.decisions += 1;
                    return Some(ScreenAction::Slot(btn_slot));
                }
            }
            while let Some(next) = self.queue.pop_front() {
                let ok = match next {
                    ScreenAction::Slot(slot) => (slot as usize) < SLOTS && legal_s[slot as usize],
                    ScreenAction::Form(entry) => (entry as usize) < FORM_LIST_ENTRIES && legal_f[entry as usize],
                };
                if ok {
                    self.decisions += 1;
                    return Some(next);
                }
            }
        } else if picked_chips > 0 {
            if let Some(btn_slot) = self.pick_post_pick_button(b, side, screen, &legal_s, cfg, rng) {
                self.decisions += 1;
                return Some(ScreenAction::Slot(btn_slot));
            }
            self.decisions += 1;
            return Some(ScreenAction::Slot(OK_SLOT));
        }

        // ---- Fresh planning (selected == 0, including right after a Soul Unison trade!) ----
        let enemy_elem = visible_enemy_element(b, side);

        // 1. Consider picking a form from the rules' form list (e.g. EXE6 Crosses) when in base form.
        if !self.tried_form_list && screen.form.is_none() && screen.trade.is_none() && b.form(side as usize).base {
            self.tried_form_list = true;
            if let Some(entry) = choose_form_list_entry(b, side, screen, &s.folder, &legal_f, enemy_elem, cfg, rng) {
                self.decisions += 1;
                return Some(ScreenAction::Form(entry));
            }
        }

        // 2. Consider chip-trade form offer buttons (EXE4 Double Soul UNITE & EXE5 Soul Unison)!
        //    These buttons start greyed out until a compatible sacrifice chip is picked, so we probe
        //    which dealt chip lights up the FormOffer button and weigh the Soul's value vs the sacrifice.
        if !self.tried_soul_offer && screen.form.is_none() && screen.trade.is_none() && b.form(side as usize).base {
            self.tried_soul_offer = true;
            if let Some((chip_slot, btn_slot)) = plan_soul_unison(b, side, screen, &s.folder, &legal_s, enemy_elem, cfg, rng) {
                self.queue.push_back(ScreenAction::Slot(btn_slot));
                self.decisions += 1;
                return Some(ScreenAction::Slot(chip_slot));
            }
        }

        // 3. Plan the best chip hand from the currently dealt slots (with active or newly traded form!).
        let active_form = screen
            .form
            .or_else(|| b.offer(side).and_then(|o| o.form).filter(|_| screen.trade.is_some()))
            .map(|h| b.content.form(h))
            .unwrap_or_else(|| b.form(side as usize));
        let best_hand = plan_best_hand(b, side, screen, &s.folder, &legal_s, active_form, cfg, rng);

        // 4. Probe any selectable pre-pick buttons (e.g. Beast Out, ChpShufl re-deal).
        if let Some(btn_slot) = self.pick_pre_pick_button(b, side, screen, &legal_s, &best_hand, enemy_elem, cfg, rng) {
            self.decisions += 1;
            return Some(ScreenAction::Slot(btn_slot));
        }

        // 5. Queue the planned chip picks + OK.
        for slot in best_hand.slots {
            if legal_s[slot as usize] {
                self.queue.push_back(ScreenAction::Slot(slot));
            }
        }
        self.queue.push_back(ScreenAction::Slot(OK_SLOT));

        let first = self.queue.pop_front().unwrap_or(ScreenAction::Slot(OK_SLOT));
        self.decisions += 1;
        Some(first)
    }

    /// Check selectable buttons before picking chips (Beast Out, ChpShufl re-deal, etc.).
    #[allow(clippy::too_many_arguments)]
    fn pick_pre_pick_button(
        &mut self,
        b: &Battle,
        side: u8,
        screen: &Screen,
        legal_s: &[bool; SLOTS],
        best_hand: &PlannedHand,
        enemy_elem: Option<Element>,
        cfg: LevelConfig,
        rng: &mut SplitMix64,
    ) -> Option<u8> {
        let hp_frac = navi_hp(b, side) as f32 / navi_max_hp(b, side) as f32;
        let enemy_hp_frac = navi_hp(b, side ^ 1) as f32 / navi_max_hp(b, side ^ 1) as f32;
        let (is_tired, is_exhausted) = navi_ref(b, side)
            .and_then(|r| b.objects.get(r).actor)
            .map_or((false, false), |a| {
                let ad = b.actors.get(a);
                (ad.tired, ad.exhausted)
            });
        for slot in 0..SLOTS as u8 {
            if !legal_s[slot as usize] || self.probed_buttons & (1 << slot) != 0 {
                continue;
            }
            if !matches!(screen.slots[slot as usize].kind, SlotKind::Button { cell, .. } if cell != ButtonCell::Right) {
                continue;
            }
            self.probed_buttons |= 1 << slot;
            let effect = probe_button(b, side, slot);
            match effect {
                ButtonEffect::Form(form_h) => {
                    let f = b.content.form(form_h);
                    let is_berserk = is_tired || f.traits.contains(FormTraits::CONTROLLED);
                    let want = if is_berserk {
                        // Allow Beast Over (and similar berserk/loss-of-control forms) when closing out a low-HP
                        // opponent, when low on HP, or when meaningfully behind in the round.
                        !is_exhausted
                            && screen.form.is_none()
                            && (enemy_hp_frac < 0.35 || hp_frac < 0.40 || hp_frac + 0.15 < enemy_hp_frac)
                            && rand_unit(rng) < cfg.form_chance
                    } else {
                        // Regular Beast Out / Cross Beast: activate from base form, once damaged, or from turn 2+.
                        screen.form.is_none()
                            && (b.form(side as usize).base || hp_frac < 0.70 || b.round.turn >= 2)
                            && rand_unit(rng) < cfg.form_chance
                    };
                    if want {
                        let score = score_form(b, side, screen, &b.custom.sides[side as usize].folder, f, enemy_elem);
                        if is_berserk || score >= -10.0 {
                            return Some(slot);
                        }
                    }
                }
                ButtonEffect::Redeal => {
                    // e.g. ChpShufl: re-deal when our best hand is poor and we still have chips left in the folder.
                    let poor_hand = best_hand.slots.len() <= 1 && best_hand.score < 160.0 && !best_hand.has_pa;
                    if cfg.use_redeal && !self.tried_redeal && poor_hand && screen.chips_left > screen.hand_size {
                        self.tried_redeal = true;
                        self.probed_buttons = 0;
                        return Some(slot);
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// Check selectable buttons after picking chips (e.g. EXE5 Capsules, or an opportunistic Soul Unison).
    fn pick_post_pick_button(
        &mut self,
        b: &Battle,
        side: u8,
        screen: &Screen,
        legal_s: &[bool; SLOTS],
        cfg: LevelConfig,
        rng: &mut SplitMix64,
    ) -> Option<u8> {
        let enemy_elem = visible_enemy_element(b, side);
        for slot in 0..SLOTS as u8 {
            if !legal_s[slot as usize] || self.probed_buttons & (1 << slot) != 0 {
                continue;
            }
            if !matches!(screen.slots[slot as usize].kind, SlotKind::Button { cell, .. } if cell != ButtonCell::Right) {
                continue;
            }
            self.probed_buttons |= 1 << slot;
            match probe_button(b, side, slot) {
                ButtonEffect::AttachedMark => {
                    // e.g. EXE5 capsules: strictly upgrades the last picked chip for free!
                    if rand_unit(rng) < cfg.form_chance.max(0.5) {
                        return Some(slot);
                    }
                }
                ButtonEffect::Form(form_h) => {
                    let f = b.content.form(form_h);
                    if f.traits.contains(FormTraits::CONTROLLED) {
                        continue;
                    }
                    if screen.form.is_none() && screen.trade.is_none() && b.form(side as usize).base && rand_unit(rng) < cfg.form_chance {
                        let last_is_precious = screen
                            .selection()
                            .last()
                            .and_then(|&sl| screen.chip_in(sl, &b.custom.sides[side as usize].folder))
                            .is_some_and(|c| matches!(b.content.chip(c.id).class, ChipClass::Giga | ChipClass::ProgramAdvance));
                        if !last_is_precious && score_form(b, side, screen, &b.custom.sides[side as usize].folder, f, enemy_elem) >= 15.0 {
                            return Some(slot);
                        }
                    }
                }
                _ => {}
            }
        }
        None
    }
}

/// Find a chip slot whose selection lights up a `ButtonView::FormOffer` button (EXE4 Double Soul /
/// EXE5 Soul Unison) and yields a positive net upgrade over keeping the chip.
#[allow(clippy::too_many_arguments)]
fn plan_soul_unison(
    b: &Battle,
    side: u8,
    screen: &Screen,
    folder: &nettai_battle::custom::BattleFolder,
    legal_s: &[bool; SLOTS],
    enemy_elem: Option<Element>,
    cfg: LevelConfig,
    rng: &mut SplitMix64,
) -> Option<(u8, u8)> {
    // Locate a FormOffer button slot on the screen (starts Unavailable until a matching chip is picked).
    let offer_btn = (0..SLOTS as u8).find(|&sl| {
        matches!(
            screen.slots[sl as usize].kind,
            SlotKind::Button { button, cell }
                if cell != ButtonCell::Right && b.content.defs.button(button).view == Some(ButtonView::FormOffer)
        )
    })?;
    if rand_unit(rng) >= cfg.form_chance {
        return None;
    }

    let hp_frac = navi_hp(b, side) as f32 / navi_max_hp(b, side) as f32;
    let mut best: Option<(u8, f32)> = None;

    for slot in 0..10u8 {
        if !legal_s[slot as usize] {
            continue;
        }
        let Some(fc) = screen.chip_in(slot, folder) else { continue };
        let cd = b.content.chip(fc.id);
        // Never sacrifice Giga or Program Advance chips for a Soul.
        if matches!(cd.class, ChipClass::Giga | ChipClass::ProgramAdvance) {
            continue;
        }
        // Probe picking `slot` on a cloned battle and see if `offer_btn` becomes Selectable!
        let mut copy = b.clone();
        let mut both = [PlayerTick::default(); 2];
        both[side as usize] = PlayerTick::default().with_choice(Some(ScreenChoice::Pick(slot)));
        if !step_ticks(&mut copy, both) {
            continue;
        }
        let Some(after) = copy.custom.sides[side as usize].screen else { continue };
        if after.slots[offer_btn as usize].state != SlotState::Selectable {
            continue;
        }
        let Some(offer) = copy.offer(side) else { continue };
        let Some(form_h) = offer.form else { continue };
        // Avoid Chaos Unison (`alternate = true`) unless desperate — mistimed Chaos charge spawns Dark MegaMan.
        if offer.alternate && hp_frac > 0.35 {
            continue;
        }
        let f = b.content.form(form_h);
        if f.traits.contains(FormTraits::CONTROLLED) {
            continue;
        }
        let form_score = score_form(b, side, screen, folder, f, enemy_elem);
        // Prefer sacrificing lower-value chips (e.g. weak Standard chips) over high-damage Mega chips.
        let chip_cost = cd.damage as f32 * 0.40 + if matches!(cd.class, ChipClass::Mega) { 45.0 } else { 0.0 };
        let net = form_score - chip_cost;
        if net > 5.0 && best.is_none_or(|(_, bn)| net > bn) {
            best = Some((slot, net));
        }
    }

    best.map(|(chip_slot, _)| (chip_slot, offer_btn))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ButtonEffect {
    None,
    Form(FormHandle),
    Redeal,
    AttachedMark,
    Other,
}

/// Probe pressing a rules button slot on a cloned battle via direct `ScreenChoice::Pick(slot)`.
fn probe_button(b: &Battle, side: u8, slot: u8) -> ButtonEffect {
    let Some(before) = b.custom.sides[side as usize].screen else {
        return ButtonEffect::None;
    };
    let offered_before = b.offer(side).and_then(|o| o.form);
    let mut copy = b.clone();
    let mut both = [PlayerTick::default(); 2];
    both[side as usize] = PlayerTick::default().with_choice(Some(ScreenChoice::Pick(slot)));
    if !step_ticks(&mut copy, both) {
        return ButtonEffect::None;
    }
    let Some(after) = copy.custom.sides[side as usize].screen else {
        return ButtonEffect::None;
    };
    if let Some(f) = after.form.filter(|&f| before.form != Some(f)) {
        return ButtonEffect::Form(f);
    }
    if after.trade.is_some() && before.trade.is_none() {
        if let Some(f) = copy.offer(side).and_then(|o| o.form).or(offered_before) {
            return ButtonEffect::Form(f);
        }
    }
    if matches!(after.phase, Phase::Redealing { .. }) {
        return ButtonEffect::Redeal;
    }
    if after.slots.iter().zip(before.slots.iter()).any(|(a, b)| a.attached != b.attached && a.attached.is_some()) {
        return ButtonEffect::AttachedMark;
    }
    if after.phase != before.phase || after.selected != before.selected {
        ButtonEffect::Other
    } else {
        ButtonEffect::None
    }
}

/// Visible element of the opponent navi (if currently visible on the field).
fn visible_enemy_element(b: &Battle, side: u8) -> Option<Element> {
    let other = side ^ 1;
    let r = navi_ref(b, other)?;
    if !seen_to(b, r, side) {
        return None;
    }
    Element::of_number(b.objects.get(r).element & 0x0F).filter(|&e| e != Element::Null)
}

/// Primary elemental weakness chart in Battle Network (Fire > Wood > Elec > Aqua > Fire).
fn beats_element(attacker: Element, defender: Element) -> bool {
    matches!(
        (attacker, defender),
        (Element::Fire, Element::Wood)
            | (Element::Wood, Element::Elec)
            | (Element::Elec, Element::Aqua)
            | (Element::Aqua, Element::Fire)
    )
}

fn chip_matches(c: &ChipData, m: ChipMatch) -> bool {
    match m {
        ChipMatch::Element(e) => c.element == e,
        ChipMatch::Family(f) => c.family == f,
    }
}

/// Score a candidate form by elemental matchup + synergy with our dealt chips and folder.
fn score_form(
    b: &Battle,
    side: u8,
    screen: &Screen,
    folder: &nettai_battle::custom::BattleFolder,
    form: &FormData,
    enemy_elem: Option<Element>,
) -> f32 {
    let mut score = 45.0;
    score += form.buster_bonus as f32 * 8.0;
    score += form.null_bonus as f32 * 0.9;
    if form.front_guard.is_some() {
        score += 24.0;
    }
    if form.fire_charge.is_some() {
        score += 20.0;
    }
    if form.chip_heals.is_some() {
        score += 20.0;
    }
    if !form.paralyzing_chips.is_empty() {
        score += 24.0;
    }
    if form.charge_doubles.is_some() {
        score += 22.0;
    }

    // Matchup against visible opponent element.
    if let Some(ee) = enemy_elem {
        if beats_element(form.element, ee) {
            score += 45.0;
        }
        if beats_element(ee, form.element) {
            score -= 55.0;
        }
    }

    // Synergy with our currently dealt chips (weighted higher) and remaining folder chips.
    let dealt_chips: Vec<ChipHandle> = (0..10u8).filter_map(|sl| screen.chip_in(sl, folder).map(|c| c.id)).collect();
    let folder_chips: Vec<ChipHandle> = folder.chips.iter().flatten().map(|c| c.id).collect();

    let synergy = |chips: &[ChipHandle], weight: f32| -> f32 {
        let mut s = 0.0;
        for &id in chips {
            let c = b.content.chip(id);
            if let Some(cb) = form.chip_bonus {
                if c.family == cb.family && c.flags.contains(ChipFlags::HAS_DAMAGE) && (cb.dimming_chips || !c.flags.contains(ChipFlags::DIMMING)) {
                    s += cb.damage as f32 * 0.45 * weight;
                }
            }
            if form.null_bonus > 0 && c.element == Element::Null && c.flags.contains(ChipFlags::HAS_DAMAGE) {
                s += form.null_bonus as f32 * 0.35 * weight;
            }
            if form.charged_chips.iter().any(|cc| cc.family == c.family && (!cc.damaging || c.flags.contains(ChipFlags::HAS_DAMAGE))) {
                s += 18.0 * weight;
            }
            if form.chip_heals.is_some_and(|m| chip_matches(c, m)) {
                s += 10.0 * weight;
            }
            if form.paralyzing_chips.contains(&c.family) && c.flags.contains(ChipFlags::HAS_DAMAGE) {
                s += 16.0 * weight;
            }
        }
        s
    };

    score += synergy(&dealt_chips, 1.0);
    score += synergy(&folder_chips, 0.25);

    let hp_frac = navi_hp(b, side) as f32 / navi_max_hp(b, side) as f32;
    if hp_frac < 0.4 && form.chip_heals.is_some() {
        score += 30.0;
    }
    score
}

/// Choose an entry from `Battle::form_list` (EXE6 Crosses) if beneficial.
#[allow(clippy::too_many_arguments)]
fn choose_form_list_entry(
    b: &Battle,
    side: u8,
    screen: &Screen,
    folder: &nettai_battle::custom::BattleFolder,
    legal_f: &[bool; FORM_LIST_ENTRIES],
    enemy_elem: Option<Element>,
    cfg: LevelConfig,
    rng: &mut SplitMix64,
) -> Option<u8> {
    let list = b.form_list(side)?;
    if rand_unit(rng) >= cfg.form_chance {
        return None;
    }
    let mut best: Option<(u8, f32)> = None;
    for (k, &ok) in legal_f.iter().enumerate() {
        if !ok {
            continue;
        }
        let Some(form_h) = list.forms[k] else { continue };
        let f = b.content.form(form_h);
        if f.traits.contains(FormTraits::CONTROLLED) {
            continue;
        }
        let noise = if cfg.noise > 0.0 { (rand_unit(rng) - 0.5) * 40.0 } else { 0.0 };
        let sc = score_form(b, side, screen, folder, f, enemy_elem) + noise;
        if best.is_none_or(|(_, bw)| sc > bw) {
            best = Some((k as u8, sc));
        }
    }
    best.filter(|&(_, sc)| sc > 5.0).map(|(k, _)| k)
}

struct PlannedHand {
    slots: Vec<u8>,
    score: f32,
    has_pa: bool,
}

#[derive(Clone, Copy)]
struct DealtSlot {
    slot: u8,
    chip: FolderChip,
    regular: bool,
    listed: bool,
    coded: bool,
    marks: Marks,
}

/// Check whether a sequence of dealt chips can legally be selected together on the custom screen.
fn compatible_set(content: &Content, chips: &[DealtSlot]) -> bool {
    let mut common_code: Option<ChipCode> = None;
    let mut code_ok = true;
    let mut common_id: Option<ChipHandle> = None;
    let mut same_id_ok = true;

    for ds in chips {
        let data = content.chip(ds.chip.id);
        if data.traits.contains(ChipTraits::GOES_WITH_ANY) {
            continue;
        }
        if ds.chip.code != ChipCode::ASTERISK {
            match common_code {
                None => common_code = Some(ds.chip.code),
                Some(c) if c == ds.chip.code => {}
                Some(_) => code_ok = false,
            }
        }
        match common_id {
            None => common_id = Some(ds.chip.id),
            Some(id) if id == ds.chip.id => {}
            Some(_) => same_id_ok = false,
        }
        if !code_ok && !same_id_ok {
            return false;
        }
    }
    true
}

/// Enumerate valid chip selections and pick the highest-scoring ordered hand.
#[allow(clippy::too_many_arguments)]
fn plan_best_hand(
    b: &Battle,
    side: u8,
    screen: &Screen,
    folder: &nettai_battle::custom::BattleFolder,
    legal_s: &[bool; SLOTS],
    active_form: &FormData,
    cfg: LevelConfig,
    rng: &mut SplitMix64,
) -> PlannedHand {
    let content = &*b.content;
    let mut dealt: Vec<DealtSlot> = Vec::with_capacity(10);
    for slot in 0..10u8 {
        if !legal_s[slot as usize] {
            continue;
        }
        let Some(chip) = screen.chip_in(slot, folder) else {
            continue;
        };
        let regular = matches!(screen.slots[slot as usize].kind, SlotKind::Chip { regular: true, .. });
        let (marks, listed) = screen.offers[slot as usize].map_or((Marks::empty(), true), |o| (o.marks, o.listed));
        dealt.push(DealtSlot { slot, chip, regular, listed, coded: true, marks });
    }

    if dealt.is_empty() {
        return PlannedHand { slots: Vec::new(), score: 0.0, has_pa: false };
    }

    let remaining_picks = MAX_SELECTIONS.saturating_sub(screen.selected as usize);
    let max_picks = cfg.max_screen_picks.min(remaining_picks);
    if max_picks == 0 {
        return PlannedHand { slots: Vec::new(), score: 0.0, has_pa: false };
    }
    let hp_frac = navi_hp(b, side) as f32 / navi_max_hp(b, side) as f32;
    let own_gauges = b.round.flags.contains(BattleFlags::OWN_GAUGES);
    let pa_used = b.custom.sides[side as usize].program_advances;
    let enemy_elem = visible_enemy_element(b, side);

    // Elements doubled by any panel type in this game (e.g. Fire on Grass, Elec on Sea/Ice).
    let doubled_elems: u8 = b
        .game_rules()
        .panels
        .types
        .iter()
        .filter_map(|r| r.doubles)
        .fold(0u8, |mask, e| mask | (1u8 << (e.min(7))));

    let mut candidates: Vec<PlannedHand> = Vec::new();

    // 1. Explicitly search for any Program Advance we can form from `dealt`!
    for pa in content.program_advances() {
        if pa.operation_battle_only && !own_gauges {
            continue;
        }
        let len = pa.recipe.len();
        if len > max_picks || len > dealt.len() {
            continue;
        }
        if let Some(pa_slots) = find_pa_slots(&dealt, &pa.recipe) {
            if compatible_set(content, &pa_slots) {
                let mut seq = pa_slots.clone();
                if seq.len() < max_picks {
                    for &extra in &dealt {
                        if seq.len() >= max_picks {
                            break;
                        }
                        if seq.iter().any(|x| x.slot == extra.slot) {
                            continue;
                        }
                        let mut trial = seq.clone();
                        trial.push(extra);
                        if compatible_set(content, &trial) {
                            let c = content.chip(extra.chip.id);
                            let applies_to_pa =
                                c.modifier.is_some() && !builder::stays_unfolded(content, Some(pa.result), extra.chip.id);
                            if applies_to_pa || c.modifier.is_none() {
                                seq = trial;
                            }
                        }
                    }
                }
                let (score, has_pa) = score_ordered_hand(b, &seq, pa_used, active_form, hp_frac, enemy_elem, doubled_elems);
                candidates.push(PlannedHand { slots: seq.iter().map(|d| d.slot).collect(), score, has_pa });
            }
        }
    }

    // 2. Enumerate valid compatible subsets (2^10 = 1024 max, heavily pruned by compatibility).
    let mut current: Vec<DealtSlot> = Vec::with_capacity(max_picks);
    enumerate_compatible(content, &dealt, 0, max_picks, &mut current, &mut |subset| {
        if subset.is_empty() {
            return;
        }
        let ordered = order_subset(content, subset, hp_frac, doubled_elems);
        let (score, has_pa) = score_ordered_hand(b, &ordered, pa_used, active_form, hp_frac, enemy_elem, doubled_elems);
        candidates.push(PlannedHand { slots: ordered.iter().map(|d| d.slot).collect(), score, has_pa });
    });

    if candidates.is_empty() {
        return PlannedHand { slots: vec![dealt[0].slot], score: 0.0, has_pa: false };
    }

    let mut best_idx = 0;
    let mut best_val = f32::NEG_INFINITY;
    for (i, cand) in candidates.iter().enumerate() {
        let jitter = if cfg.noise > 0.0 { (rand_unit(rng) - 0.5) * cfg.noise * 1400.0 } else { 0.0 };
        let v = cand.score + jitter;
        if v > best_val {
            best_val = v;
            best_idx = i;
        }
    }
    candidates.swap_remove(best_idx)
}

fn enumerate_compatible(
    content: &Content,
    dealt: &[DealtSlot],
    start: usize,
    max_picks: usize,
    cur: &mut Vec<DealtSlot>,
    visit: &mut impl FnMut(&[DealtSlot]),
) {
    if !cur.is_empty() {
        visit(cur);
    }
    if cur.len() >= max_picks {
        return;
    }
    for i in start..dealt.len() {
        cur.push(dealt[i]);
        if compatible_set(content, cur) {
            enumerate_compatible(content, dealt, i + 1, max_picks, cur, visit);
        }
        cur.pop();
    }
}

/// Find dealt slots matching a Program Advance recipe (in exact recipe order).
fn find_pa_slots(dealt: &[DealtSlot], recipe: &Recipe) -> Option<Vec<DealtSlot>> {
    match recipe {
        Recipe::Sequence(ids) => {
            let mut used = [false; 10];
            let mut out = Vec::with_capacity(ids.len());
            for &want in ids {
                let idx = (0..dealt.len()).find(|&i| !used[i] && dealt[i].chip.id == want)?;
                used[idx] = true;
                out.push(dealt[idx]);
            }
            Some(out)
        }
        Recipe::CodeRun { chip, count } => {
            let len = *count as usize;
            let matches: Vec<DealtSlot> = dealt.iter().copied().filter(|d| d.chip.id == *chip).collect();
            if matches.len() < len {
                return None;
            }
            let star = ChipCode::ASTERISK.0;
            for start_code in 0..=26u8.saturating_sub(len as u8) {
                let mut used = vec![false; matches.len()];
                let mut out = Vec::with_capacity(len);
                let mut stars = 0;
                let mut ok = true;
                for offset in 0..len {
                    let want = start_code + offset as u8;
                    if let Some(i) = (0..matches.len()).find(|&i| !used[i] && matches[i].chip.code.0 == want) {
                        used[i] = true;
                        out.push(matches[i]);
                    } else if stars == 0
                        && let Some(i) = (0..matches.len()).find(|&i| !used[i] && matches[i].chip.code.0 == star)
                    {
                        used[i] = true;
                        stars += 1;
                        out.push(matches[i]);
                    } else {
                        ok = false;
                        break;
                    }
                }
                if ok {
                    return Some(out);
                }
            }
            None
        }
    }
}

/// Order a subset of chips tactically:
/// 1. Damaging attacks (and offensive dimming openers with damage like AreaGrab) FIRST, with every
///    modifier (`Atk+`, `Navi+`, `Paralyze`, `Uninstall`) folded directly behind its best compatible host.
/// 2. Pure non-damaging defensive / cut-in / utility chips (`Invisible`, `Barrier`, `Aura`, `Recovery`, traps)
///    LAST (except when critically hurt < 40% HP), so attacks fire freely and a defensive dimming chip sits
///    saved at the back of the hand ready to counter cut-in on an opponent's dimming attack!
fn order_subset(content: &Content, subset: &[DealtSlot], hp_frac: f32, doubled_elems: u8) -> Vec<DealtSlot> {
    let mut utilities = Vec::new();
    let mut attacks = Vec::new();
    let mut modifiers = Vec::new();

    for &ds in subset {
        let c = content.chip(ds.chip.id);
        if c.modifier.is_some() {
            modifiers.push(ds);
        } else if !c.flags.contains(ChipFlags::HAS_DAMAGE) && c.damage == 0 && c.formula.is_none() {
            utilities.push(ds);
        } else {
            attacks.push(ds);
        }
    }

    // Order utilities: non-dimming field/setup chips first, traps next, reactive dimming defenses (Invis/Barrier) last!
    utilities.sort_by_key(|u| {
        let c = content.chip(u.chip.id);
        let tier = if c.flags.contains(ChipFlags::DIMMING) {
            2u8
        } else if c.trap.is_some() {
            1u8
        } else {
            0u8
        };
        (tier, u.slot)
    });

    // Sort attacks: low-damage dimming openers (e.g. AreaGrab with damage=10) first so they set up swords/combos,
    // then highest-value Navi/payoff attacks!
    attacks.sort_by(|a, b| {
        let ca = content.chip(a.chip.id);
        let cb = content.chip(b.chip.id);
        let is_opener = |c: &ChipData| c.flags.contains(ChipFlags::DIMMING) && !c.flags.contains(ChipFlags::HAS_DAMAGE) && c.damage <= 20;
        let bonus = |c: &ChipData| -> i32 {
            if is_opener(c) {
                return 10_000;
            }
            let mut p = c.damage as i32;
            if c.flags.contains(ChipFlags::NAVI) {
                p += 80;
            }
            if doubled_elems & (1u8 << (c.element as u8)) != 0 {
                p += 25;
            }
            p
        };
        bonus(cb).cmp(&bonus(ca)).then(a.slot.cmp(&b.slot))
    });

    // Attach modifiers directly behind the strongest compatible attack chip.
    let mut attached: Vec<Vec<DealtSlot>> = vec![Vec::new(); attacks.len()];
    let mut leftover_mods = Vec::new();
    for m in modifiers {
        let host = (0..attacks.len()).find(|&i| !builder::stays_unfolded(content, Some(attacks[i].chip.id), m.chip.id));
        if let Some(i) = host {
            attached[i].push(m);
        } else {
            leftover_mods.push(m);
        }
    }

    let mut out = Vec::with_capacity(subset.len());
    // When critically low HP, put non-dimming utilities (e.g. Recovery) first; otherwise keep attacks first
    // so reactive defensive dimming chips (Invis/Barrier/Aura) sit saved at the back of the hand for cut-ins!
    let defenses_first = hp_frac < 0.40;
    if defenses_first {
        out.extend(utilities.iter().copied());
    }
    for (i, atk) in attacks.into_iter().enumerate() {
        out.push(atk);
        out.extend(attached[i].iter().copied());
    }
    if !defenses_first {
        out.extend(utilities);
    }
    out.extend(leftover_mods);
    out
}

/// Score an ordered hand using `builder::build` to get exact Program Advances, folded modifiers,
/// active form bonuses, and setup→payoff combo synergy.
#[allow(clippy::too_many_arguments)]
fn score_ordered_hand(
    b: &Battle,
    ordered: &[DealtSlot],
    mut pa_used: builder::ProgramAdvancesUsed,
    active_form: &FormData,
    hp_frac: f32,
    enemy_elem: Option<Element>,
    doubled_elems: u8,
) -> (f32, bool) {
    let content = &*b.content;
    let picks: Vec<Pick> = ordered
        .iter()
        .map(|d| Pick {
            chip: d.chip,
            regular: d.regular,
            marks: d.marks,
            listed: d.listed,
            coded: d.coded,
        })
        .collect();

    let built = builder::build(
        &picks,
        b.round.turn,
        &mut pa_used,
        content,
        b.round.flags.contains(BattleFlags::OWN_GAUGES),
        |id| content.chip(id).damage,
    );

    let mut score = 0.0f32;
    let has_pa = built.program_advance.is_some();
    if has_pa {
        score += 400.0;
    }

    // Encourage multi-chip burst hands and fast folder cycling.
    score += ordered.len() as f32 * 30.0;

    let mut has_setup = false;
    let mut utility_count = 0u8;
    let mut seen_utilities: [Option<ChipHandle>; 6] = [None; 6];
    let mut attack_count = 0u8;
    let mut has_doubled_payoff = false;

    for i in 0..6 {
        let Some(id) = built.hand.ids[i] else {
            break;
        };
        let c = content.chip(id);
        let marks = built.hand.modifiers[i];

        if c.modifier.is_some() {
            score -= 170.0;
            continue;
        }

        let mut dmg = built.hand.damage[i] as f32 + built.hand.attack_bonus[i] as f32;
        if c.formula.is_some() && dmg == 0.0 {
            dmg = 120.0;
        }

        // Active form damage & A-charge bonuses!
        if let Some(cb) = active_form.chip_bonus {
            if c.family == cb.family && c.flags.contains(ChipFlags::HAS_DAMAGE) && (cb.dimming_chips || !c.flags.contains(ChipFlags::DIMMING)) {
                dmg += cb.damage as f32;
            }
        }
        if active_form.null_bonus > 0 && c.element == Element::Null && c.flags.contains(ChipFlags::HAS_DAMAGE) {
            dmg += active_form.null_bonus as f32;
        }
        let a_charge_match = active_form.charged_chips.iter().any(|rule| {
            (c.family == rule.family || (rule.element_swords && c.traits.contains(ChipTraits::ELEMENT_SWORD)))
                && ((c.flags.contains(ChipFlags::HAS_DAMAGE) && !c.flags.contains(ChipFlags::DIMMING)) || !rule.damaging)
                && ((!c.flags.contains(ChipFlags::DIMMING) && !c.flags.contains(ChipFlags::DARK)) || !rule.plain)
        });
        if a_charge_match && dmg > 0.0 {
            dmg *= 1.35;
        }

        if let Some(ee) = enemy_elem {
            if beats_element(c.element, ee) && dmg > 0.0 {
                dmg *= 1.65;
            }
        }

        if dmg > 0.0 {
            attack_count += 1;
            if doubled_elems & (1u8 << (c.element as u8)) != 0 {
                has_doubled_payoff = true;
            }
            score += dmg * 1.10;
            if c.flags.contains(ChipFlags::DIMMING) {
                score += 42.0;
            }
            if c.flags.contains(ChipFlags::NAVI) {
                score += 32.0;
            }
            if matches!(c.class, ChipClass::Mega) {
                score += 22.0;
            } else if matches!(c.class, ChipClass::Giga | ChipClass::ProgramAdvance) {
                score += 65.0;
            }
            if marks.contains(Marks::PARALYZE) {
                score += 80.0;
            }
            if marks.contains(Marks::UNINSTALL) {
                score += 95.0;
            }
        } else {
            // Non-damaging setup / defensive / trap / heal chip.
            // Strongly discourage picking duplicate copies (e.g. Invisible + Invisible) or clogging the hand
            // with multiple defensive chips in one turn!
            has_setup = true;
            let dup = seen_utilities[..utility_count as usize].contains(&Some(id));
            if (utility_count as usize) < seen_utilities.len() {
                seen_utilities[utility_count as usize] = Some(id);
            }
            utility_count += 1;
            let mut util = if dup {
                -85.0
            } else if utility_count >= 2 {
                12.0
            } else {
                58.0
            };
            if !dup {
                if c.trap.is_some() {
                    util += 18.0;
                }
                if c.flags.contains(ChipFlags::DIMMING) {
                    util += 18.0;
                }
                if hp_frac < 0.65 {
                    util += 24.0;
                }
            }
            score += util;
        }

        if c.flags.contains(ChipFlags::DARK) && hp_frac > 0.45 {
            score -= 55.0;
        }
        if built.hand.attack_bonus[i] > 0 {
            score += 28.0;
        }
    }

    // Synergy bonus for pairing a setup chip (AreaGrab / panel changer / Invis) ahead of attacks!
    if has_setup && attack_count >= 1 {
        score += 28.0;
        if has_doubled_payoff {
            score += 20.0;
        }
    }

    (score, has_pa)
}

fn rand_unit(rng: &mut SplitMix64) -> f32 {
    ((rng.next_u64() >> 40) as f32) / ((1u64 << 24) as f32)
}
