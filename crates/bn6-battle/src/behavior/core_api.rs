//! The engine's side of the content API: [`CoreApi`] over a `Battle`.
//! Each call is the engine's own code (the ruleset's services, the navi
//! framework, the core's objects and panels); content never sees the
//! engine's bit values or offsets.

use bn6_content_api::api::ApiResult;
use bn6_content_api::api::ObstacleFlag;
use bn6_content_api::{
    ActorField, ApiError, BattleInfo, BlinkOut, CollisionField, ColumnInfo, ContentState, CoreApi, DimmingStep,
    Emotion, FieldType, FieldValue, HitboxSpec, Key, Lifecycle, LinkedChip, NaviRecordInfo, NaviStat, NaviState,
    ObjectField, ObstacleAction, SideSpecial, ObstacleCrush, ObstacleRemoval, ObstacleRequest, Pad, PanelInfo, RequestFlag, Shadow,
    SpriteField, SpriteId, StatusFlag, StatusTimer, Value,
};
use bn6_content_api::{ActionHandle, ChipHandle, KindHandle, NaviAction, Registry, SpawnAt, StateId, WeaponHandle};
// Subtypes 8, 17, 18 (Wind, Anubis, Otenko) and the obstacle framework.
use bn6_content_api::{ObstacleHold, ObstaclePush, WindSource};

use crate::actor::{AbsorbedObstacle, ActorData, ActorType, request, status};
use crate::battle::{Battle, LinkedRecord};
use crate::collision::{CollisionData, f1, timer};
use crate::field::PanelType;
use crate::input::keys;
use crate::kinds::common::{self, Progress};
use crate::kinds::player::actions::ActionVars;
use crate::kinds::{self, Vars};
use crate::object::sprite;
use crate::object::{ObjectRef, PanelPos, Pool, Vec3, flags, state};
use crate::sound::SoundId;

/// The collision `f1` bit behind a status flag.
/// The object kind named `key`.
fn kind_named(content: &crate::content::Content, key: &str) -> ApiResult<KindHandle> {
    content.defs.kind_by_key(key).ok_or_else(|| ApiError::Other(format!("no object kind is named {key:?}")))
}

fn status_bit(flag: StatusFlag) -> u32 {
    match flag {
        StatusFlag::Guard => f1::GUARD,
        StatusFlag::Invisible => f1::INVISIBLE,
        StatusFlag::Submerged => f1::SUBMERGED,
        StatusFlag::Invulnerable => f1::INVULNERABLE,
        StatusFlag::AirShoe => f1::AIRSHOE,
        StatusFlag::FloatShoe => f1::FLOATSHOE,
        StatusFlag::Moving => f1::MOVING,
        StatusFlag::Dead => f1::DEAD,
        StatusFlag::Flashing => f1::FLASHING,
        StatusFlag::Flinching => f1::FLINCHING,
        StatusFlag::Paralyzed => f1::PARALYZED,
        StatusFlag::Sliding => f1::SLIDING,
        StatusFlag::Blind => f1::BLIND,
        StatusFlag::Immobilized => f1::IMMOBILIZED,
        StatusFlag::Confused => f1::CONFUSED,
        StatusFlag::Frozen => f1::FROZEN,
        StatusFlag::SuperArmor => f1::SUPERARMOR,
        StatusFlag::Undershirt => f1::UNDERSHIRT,
        StatusFlag::MoveComplete => f1::MOVE_COMPLETE,
        StatusFlag::Drag => f1::DRAG,
        StatusFlag::Anger => f1::ANGER,
        StatusFlag::UsingAction => f1::USING_ACTION,
        StatusFlag::AffectedByIce => f1::AFFECTED_BY_ICE,
        StatusFlag::Bubbled => f1::BUBBLED,
        StatusFlag::HitWhileDimmed => f1::HIT_WHILE_DIMMED,
    }
}

fn timer_index(t: StatusTimer) -> usize {
    match t {
        StatusTimer::Paralyze => timer::PARALYZE,
        StatusTimer::Confuse => timer::CONFUSE,
        StatusTimer::Blind => timer::BLIND,
        StatusTimer::Immobilize => timer::IMMOBILIZE,
        StatusTimer::Flash => timer::FLASH,
        StatusTimer::Submerged => timer::SUBMERGED,
        StatusTimer::Invulnerable => timer::INVULNERABLE,
        StatusTimer::Freeze => timer::FREEZE,
        StatusTimer::Bubble => timer::BUBBLE,
    }
}

fn request_bit(f: RequestFlag) -> u32 {
    match f {
        RequestFlag::Buster => request::BUSTER,
        RequestFlag::ChargedShot => request::CHARGED_SHOT,
        RequestFlag::Chip => request::CHIP,
        RequestFlag::ChargedChip => request::CHARGED_CHIP,
        RequestFlag::BackSpecial => request::BACK_SPECIAL,
        RequestFlag::ForcedChargedShot => request::FORCED_CHARGED_SHOT,
        RequestFlag::RevertForm => request::REVERT_FORM,
        RequestFlag::AntiDamageTriggered => request::ANTI_DAMAGE_TRIGGERED,
        RequestFlag::AntiSwordTriggered => request::ANTI_SWORD_TRIGGERED,
        RequestFlag::CutIn => request::CUT_IN,
        RequestFlag::TurnL => request::TURN_L,
        RequestFlag::TurnR => request::TURN_R,
        RequestFlag::FormChange => request::FORM_CHANGE,
        RequestFlag::BodyGuardTriggered => request::BODY_GUARD_TRIGGERED,
        RequestFlag::AltChip => request::ALT_CHIP,
        RequestFlag::AHeld => request::A_HELD,
        RequestFlag::BHeld => request::B_HELD,
        RequestFlag::StunStrike => request::STUN_STRIKE,
        RequestFlag::SelectSpecial => request::SELECT_SPECIAL,
        RequestFlag::CrossChange => request::CROSS_CHANGE,
        RequestFlag::CrossDeath => request::CROSS_DEATH,
        RequestFlag::Mode9A => request::MODE9_A,
        RequestFlag::CrossSpecial => request::CROSS_SPECIAL,
        RequestFlag::Volley => request::VOLLEY,
        RequestFlag::WeaknessHit => request::WEAKNESS_HIT,
    }
}

fn navi_state_bit(f: NaviState) -> u32 {
    match f {
        NaviState::Controllable => status::CONTROLLABLE,
        NaviState::ChipInProgress => status::CHIP_IN_PROGRESS,
        NaviState::FormChange => status::FORM_CHANGE,
        NaviState::RevertingForm => status::REVERTING_FORM,
        NaviState::NoCharge => status::NO_CHARGE,
        NaviState::CanTurn => status::CAN_TURN,
        NaviState::TrapArmed => status::TRAP_ARMED,
        NaviState::ChangingCross => status::CHANGING_CROSS,
        NaviState::CrossKnockout => status::CROSS_KNOCKOUT,
        NaviState::Crossed => status::CROSSED,
        NaviState::Volley => status::VOLLEY,
        NaviState::Uninterruptible => status::UNINTERRUPTIBLE,
        NaviState::CrossBreaking => status::CROSS_BREAKING,
        NaviState::FormChangeSpriteHeld => status::FORM_CHANGE_SPRITE_HELD,
        NaviState::HeatTrap => status::HEAT_TRAP,
        NaviState::Vanished => status::VANISHED,
    }
}

fn key_bit(k: Key) -> u16 {
    match k {
        Key::A => keys::A,
        Key::B => keys::B,
        Key::Select => keys::SELECT,
        Key::Start => keys::START,
        Key::Right => keys::RIGHT,
        Key::Left => keys::LEFT,
        Key::Up => keys::UP,
        Key::Down => keys::DOWN,
        Key::R => keys::R,
        Key::L => keys::L,
    }
}

/// The header flag behind a boolean object field.
fn flag_bit(f: ObjectField) -> Option<u8> {
    Some(match f {
        ObjectField::Active => flags::ACTIVE,
        ObjectField::Visible => flags::VISIBLE,
        ObjectField::RunWhilePaused => flags::RUN_WHILE_PAUSED,
        ObjectField::RunWhileDimmed => flags::RUN_WHILE_DIMMED,
        ObjectField::NoSpriteUpdate => flags::NO_SPRITE_UPDATE,
        ObjectField::HoldsReservation => flags::HOLDS_RESERVATION,
        _ => return None,
    })
}

fn actor_type_of(i: u8) -> ActorType {
    match i {
        0 => ActorType::Virus,
        1 => ActorType::Navi,
        _ => ActorType::Player,
    }
}

fn actor_type_index(t: ActorType) -> i64 {
    match t {
        ActorType::Virus => 0,
        ActorType::Navi => 1,
        ActorType::Player => 2,
    }
}

impl Battle {
    fn actor_of(&self, o: ObjectRef) -> ApiResult<&ActorData> {
        let a = self.objects.get(o).actor.ok_or(ApiError::NoActor(o))?;
        Ok(self.actors.get(a))
    }

    fn actor_of_mut(&mut self, o: ObjectRef) -> ApiResult<&mut ActorData> {
        let a = self.objects.get(o).actor.ok_or(ApiError::NoActor(o))?;
        Ok(self.actors.get_mut(a))
    }

    fn collision_of(&self, o: ObjectRef) -> ApiResult<&CollisionData> {
        let c = self.objects.get(o).collision.ok_or(ApiError::NoCollision(o))?;
        Ok(self.collision.get(c))
    }

    fn collision_of_mut(&mut self, o: ObjectRef) -> ApiResult<&mut CollisionData> {
        let c = self.objects.get(o).collision.ok_or(ApiError::NoCollision(o))?;
        Ok(self.collision.get_mut(c))
    }

    /// A chip as the numeric API gives it: the pack's number, or for a chip
    /// content defines (which has none) [`DEFINED_CHIPS`] plus its handle,
    /// so a script can hand it back.
    pub(crate) fn api_chip(&self, h: ChipHandle) -> u16 {
        self.content.chip_number(h).unwrap_or(DEFINED_CHIPS + h.0)
    }

    /// A chip field as the numeric API gives it; `none` for no chip (the
    /// field's own "none": 0 or 0xFFFF).
    pub(crate) fn api_chip_field(&self, h: Option<ChipHandle>, none: u16) -> u16 {
        h.map_or(none, |h| self.api_chip(h))
    }

    /// The chip a number from the numeric API names (see [`Self::api_chip`]);
    /// `none` for no chip.
    pub(crate) fn chip_from_api(&self, n: u16, none: u16) -> ApiResult<Option<ChipHandle>> {
        if n == none {
            return Ok(None);
        }
        if n >= DEFINED_CHIPS {
            let h = ChipHandle(n - DEFINED_CHIPS);
            return (h.index() < self.content.defs.chips.len())
                .then_some(Some(h))
                .ok_or_else(|| ApiError::Other(format!("chip {n:#x} is not in the content")));
        }
        self.content.chip_numbered(n).map(Some).ok_or_else(|| ApiError::Other(format!("chip {n:#x} is not in the content")))
    }

    /// A weapon as the numeric API gives it: its routine number (0xFF for
    /// none). A weapon content defines has no number for it to give.
    fn api_weapon(&self, w: Option<WeaponHandle>) -> i64 {
        let Some(w) = w else { return 0xFF };
        match self.content.weapon_number(w) {
            Some(n) => n as i64,
            None => panic!(
                "the numeric API can't name weapon {:?}, which content defines (the v2 API names it by handle)",
                self.content.defs.weapon(w).key
            ),
        }
    }

    /// The weapon a routine number from the numeric API names (0xFF: none).
    fn weapon_from_api(&self, n: u8) -> Option<WeaponHandle> {
        (n != 0xFF).then(|| self.content.weapon_numbered(n))
    }
}

/// Where the numeric API's chips content defines start: past every chip id
/// the original has (nine bits).
pub const DEFINED_CHIPS: u16 = 0x200;

/// Check a write and convert it by the field's type.
fn store(name: &'static str, writable: bool, ty: FieldType, v: Value) -> ApiResult<FieldValue> {
    if !writable {
        return Err(ApiError::ReadOnly(name));
    }
    ty.store(v).map_err(|error| ApiError::Type { field: name, error })
}

fn int(v: FieldValue) -> i64 {
    match v.load() {
        Value::Int(i) => i,
        v => unreachable!("an integer field stored {v:?}"),
    }
}

impl CoreApi for Battle {
    // ---- The battle ------------------------------------------------------

    fn is_dimmed(&self) -> bool {
        Battle::is_dimmed(self)
    }

    fn is_paused(&self) -> bool {
        self.paused
    }

    fn is_battle_over(&self) -> bool {
        Battle::is_battle_over(self)
    }

    fn is_time_up(&self) -> bool {
        self.is_battle_over_flag_quirk()
    }

    fn viewer_sees(&self, side: u8) -> bool {
        kinds::charge_glow::viewer_sees(self, side & 1)
    }

    fn next_chip_damages(&self, user: ObjectRef) -> bool {
        use crate::content::ChipFlags;
        let o = self.objects.get(user);
        let flags = if self.content.navi_record(o.name_id).actor_type == crate::actor::ActorType::Player {
            let hand = &self.hands[o.alliance as usize & 1];
            match hand.ids.get(hand.cursor as usize).copied().flatten() {
                Some(h) => self.content.chip(h).flags,
                None => self.content.rules.empty_hand.flags,
            }
        } else {
            // Another object's chip word: zeroed, the pack's chip 0.
            match o.chip.or_else(|| self.content.chip_numbered(0)) {
                Some(h) => self.content.chip(h).flags,
                None => ChipFlags(0),
            }
        };
        flags.0 & ChipFlags::HAS_DAMAGE != 0
    }

    fn battle_info(&self, f: BattleInfo) -> Value {
        match f {
            BattleInfo::Link => Value::Bool(self.setup.settings.effects & crate::setup::effects::LINK != 0),
            BattleInfo::Mode => Value::Int(self.round.mode_copy as i64),
            BattleInfo::PanelPattern => Value::Int(self.content.stage(self.setup.settings.stage).panel_pattern as i64),
            BattleInfo::NavisIn => Value::Bool(self.round.intro_bits & 0x02 != 0),
            BattleInfo::LocalSide => Value::Int(self.round.local_side as i64),
            BattleInfo::Turn => Value::Int(self.round.turn as i64),
            BattleInfo::PerPlayerGauges => {
                Value::Bool(self.round.flags & crate::battle::battle_flags::PER_PLAYER_GAUGES != 0)
            }
            BattleInfo::Fighting => Value::Bool(self.round.flags & crate::battle::battle_flags::FIGHTING != 0),
        }
    }

    fn shake_camera(&mut self, magnitude: u16, ticks: u16) {
        Battle::shake_camera(self, magnitude, ticks);
    }

    fn play_sound(&mut self, sound: u16) {
        Battle::play_sound(self, SoundId(sound));
    }

    fn play_sound_for(&mut self, side: u8, sound: u16) {
        Battle::play_sound_for(self, side & 1, SoundId(sound));
    }

    fn navi_stat(&self, side: u8, stat: NaviStat) -> Value {
        let s = &self.stats[side as usize & 1];
        let i = |v: i64| Value::Int(v);
        match stat {
            NaviStat::Sun => Value::Bool(s.sun),
            NaviStat::Form => i(self.content.form_number(s.form).0 as i64),
            NaviStat::Navi => i(self.content.navi_number(s.navi).0 as i64),
            NaviStat::NaviVariant => i(s.navi_variant as i64),
            NaviStat::Element => i(s.element as i64),
            NaviStat::Attack => i(s.attack as i64),
            NaviStat::Rapid => i(s.rapid as i64),
            NaviStat::Charge => i(s.charge as i64),
            NaviStat::Mood => i(s.mood as i64),
            NaviStat::BeastOutCounter => i(s.beast_out_counter as i64),
            NaviStat::Version => i(s.version as i64),
            NaviStat::MaxBaseHp => i(s.max_base_hp as i64),
            NaviStat::ChipRecovery => i(s.chip_recovery as i64),
            NaviStat::BusterShot => i(s.weapons.buster_shot as i64),
            NaviStat::ChargeShotKind => i(s.weapons.charge_shot_kind as i64),
            NaviStat::BusterBlanks => i(s.bugs.buster_blanks as i64),
            NaviStat::BusterCharged => i(s.bugs.buster_charged as i64),
            NaviStat::HpDrain => i(s.bugs.hp_drain as i64),
            NaviStat::CustomDrain => i(s.bugs.custom_drain as i64),
            NaviStat::PanelTrail => i(s.bugs.panel_trail_kind as i64),
            NaviStat::Beast => Value::Bool(self.content.form_number(s.form).is_beast()),
            NaviStat::BeastOver => Value::Bool(self.content.form_number(s.form).is_beast_over()),
            NaviStat::CustomLevel => i(s.custom_level as i64),
            NaviStat::HandShrinkTurn => i(s.bugs.hand_shrink_turn as i64),
            NaviStat::ChargeShotRoutine => i(self.api_weapon(s.weapons.charge_shot)),
            NaviStat::BackSpecialRoutine => i(self.api_weapon(s.weapons.back_special)),
            NaviStat::FloatShoes => Value::Bool(s.float_shoes),
            NaviStat::AirShoes => Value::Bool(s.air_shoes),
            NaviStat::Undershirt => Value::Bool(s.undershirt),
            NaviStat::BugKinds => {
                let b = &s.bugs;
                let kinds = [
                    b.processing == 1,
                    b.panel_trail_level != 0,
                    b.buster_blanks != 0,
                    b.hit_status != 0,
                    b.custom_damage != 0,
                    b.emotion != 0,
                    b.hp_drain != 0,
                    b.custom_drain != 0,
                    b.battle_start != 0,
                    b.hand_shrink_turn != 0,
                ];
                i(kinds.iter().filter(|&&k| k).count() as i64)
            }
        }
    }

    fn set_navi_stat(&mut self, side: u8, stat: NaviStat, v: Value) -> ApiResult<()> {
        let v = store(stat.name(), stat.writable(), stat.ty(), v)?;
        let weapon = match v {
            FieldValue::U8(x) => self.weapon_from_api(x),
            _ => None,
        };
        let s = &mut self.stats[side as usize & 1];
        match (stat, v) {
            (NaviStat::Attack, FieldValue::U8(x)) => s.attack = x,
            (NaviStat::Rapid, FieldValue::U8(x)) => s.rapid = x,
            (NaviStat::Charge, FieldValue::U8(x)) => s.charge = x,
            (NaviStat::CustomLevel, FieldValue::U8(x)) => s.custom_level = x,
            (NaviStat::HandShrinkTurn, FieldValue::U8(x)) => s.bugs.hand_shrink_turn = x,
            (NaviStat::ChargeShotRoutine, FieldValue::U8(_)) => s.weapons.charge_shot = weapon,
            (NaviStat::BackSpecialRoutine, FieldValue::U8(_)) => s.weapons.back_special = weapon,
            (NaviStat::FloatShoes, FieldValue::Bool(x)) => s.float_shoes = x,
            (NaviStat::AirShoes, FieldValue::Bool(x)) => s.air_shoes = x,
            (NaviStat::Undershirt, FieldValue::Bool(x)) => s.undershirt = x,
            (f, v) => unreachable!("{f:?} stored as {v:?}"),
        }
        Ok(())
    }

    fn emotion(&self, side: u8) -> Emotion {
        use crate::kinds::player::Emotion as E;
        match kinds::player::emotion(self, side & 1) {
            E::Normal => Emotion::Normal,
            E::Tired => Emotion::Tired,
            E::FullSynchro => Emotion::FullSynchro,
            E::Angry => Emotion::Angry,
            E::WornOut => Emotion::WornOut,
        }
    }

    fn set_mood(&mut self, side: u8, mood: u8) {
        kinds::player::set_mood(self, side & 1, mood);
    }

    fn side_special(&self, side: u8) -> SideSpecial {
        let s = &self.sides[side as usize & 1];
        if s.select_special != 0 {
            SideSpecial::Select
        } else if s.cross_special != 0 {
            SideSpecial::Cross
        } else {
            SideSpecial::None
        }
    }

    fn player(&self, side: u8) -> Option<ObjectRef> {
        Battle::player(self, side & 1)
    }

    fn alive_actors(&self, side: u8) -> Vec<ObjectRef> {
        self.round.alive_actors[side as usize & 1].iter().flatten().copied().collect()
    }

    fn rng(&mut self) -> u32 {
        self.rng.next()
    }

    fn rng_positive(&mut self) -> u32 {
        self.rng.next_positive()
    }

    fn jitter(&mut self, mask: u32, pos: Vec3) -> Vec3 {
        kinds::spark::jitter(self, mask, pos)
    }

    fn hand_chip(&self, side: u8, i: u8) -> u16 {
        // An empty entry (and one past the hand) is 0xFFFF.
        let chip = self.hands[side as usize & 1].ids.get(i as usize).copied().flatten();
        self.api_chip_field(chip, 0xFFFF)
    }

    fn hand_cursor(&self, side: u8) -> u8 {
        self.hands[side as usize & 1].cursor
    }

    fn advance_hand(&mut self, side: u8) {
        self.hands[side as usize & 1].advance();
    }

    // Subtype 18 (Otenko).
    fn hand_turn(&self, side: u8, i: u8) -> u8 {
        // (The cursor never passes 5.)
        self.hands[side as usize & 1].turn.get(i as usize).copied().unwrap_or(0)
    }

    fn add_hand_attack_bonus(&mut self, side: u8, i: u8, n: u16) {
        if let Some(b) = self.hands[side as usize & 1].attack_bonus.get_mut(i as usize) {
            *b = b.wrapping_add(n);
        }
    }

    fn linked(&self, side: u8) -> LinkedChip {
        let r = self.linked[side as usize & 1];
        let chip = self.api_chip_field(r.chip, 0);
        LinkedChip { chip, bonus: r.bonus, damage: r.damage, owner: r.owner, object: r.object }
    }

    fn set_linked(&mut self, side: u8, rec: LinkedChip) {
        let chip = self.chip_from_api(rec.chip, 0).unwrap_or_else(|e| panic!("a linked chip record: {e}"));
        self.linked[side as usize & 1] =
            LinkedRecord { chip, bonus: rec.bonus, damage: rec.damage, owner: rec.owner, object: rec.object };
    }

    fn clear_linked(&mut self, side: u8) {
        Battle::clear_linked(self, side & 1);
    }

    fn fill_custom_gauge(&mut self) {
        self.gauge.value = crate::hud::CustomGauge::FULL;
    }

    fn set_gauge_rate(&mut self, rate: u16) {
        self.gauge.rate = rate;
    }

    fn set_gauge_speed_ticks(&mut self, side: u8, slow: u16, fast: u16) {
        let s = &mut self.sides[side as usize & 1];
        s.slow_gauge_ticks = slow;
        s.fast_gauge_ticks = fast;
    }

    fn add_side_gauge(&mut self, side: u8, n: u16) {
        let s = &mut self.sides[side as usize & 1];
        s.gauge = (s.gauge as u32 + n as u32).min(crate::hud::CustomGauge::FULL as u32) as u16;
    }

    fn add_special_bonus(&mut self, side: u8, index: u8, n: u16) -> ApiResult<()> {
        let s = &mut self.sides[side as usize & 1];
        let bonus = match index {
            0 => &mut s.special_attack_bonus,
            1 => &mut s.special_navi_bonus,
            _ => {
                // +0x36 + 2 * index: 2 and 3 would be the fast and slow gauge
                // timers (+0x3A, +0x3C). No chip is known to reach them.
                return Err(ApiError::Other(format!(
                    "special bonus {index}: sub_8010488 would add to the side state's +{:#x}",
                    0x36 + 2 * index as u32
                )));
            }
        };
        *bonus = bonus.wrapping_add(n);
        Ok(())
    }

    fn bump_side_stat(&mut self, side: u8, index: u8, n: u8) {
        Battle::bump_side_stat(self, side & 1, index as usize & 0xF, n);
    }

    // Subtype 8 (Wind and Fan).
    fn wind(&self, side: u8) -> (Option<ObjectRef>, WindSource) {
        let w = self.field.winds[side as usize & 1];
        let source = match w.source {
            crate::field::WindSource::Obstacle => WindSource::Obstacle,
            crate::field::WindSource::Navi => WindSource::Navi,
        };
        (w.object, source)
    }

    fn set_wind(&mut self, o: ObjectRef, side: u8, source: WindSource) {
        let source = match source {
            WindSource::Obstacle => crate::field::WindSource::Obstacle,
            WindSource::Navi => crate::field::WindSource::Navi,
        };
        kinds::obstacle::set_wind(self, o, side & 1, source);
    }

    fn clear_wind(&mut self, o: ObjectRef) {
        kinds::obstacle::clear_wind(self, o);
    }

    fn side_stat(&self, side: u8, index: u8) -> u8 {
        self.side_stats[side as usize & 1][index as usize & 0xF]
    }

    fn damage_carry(&self, side: u8) -> bn6_content_api::api::DamageCarryInfo {
        let c = &self.damage_carry[side as usize & 1];
        bn6_content_api::api::DamageCarryInfo {
            this_tick: c.this_tick,
            previous: c.previous,
            source: c.source,
            target: c.target,
        }
    }

    fn set_damage_carry(&mut self, side: u8, rec: bn6_content_api::api::DamageCarryInfo) {
        self.damage_carry[side as usize & 1] = crate::battle::DamageCarry {
            this_tick: rec.this_tick,
            previous: rec.previous,
            source: rec.source,
            target: rec.target,
        };
    }

    fn navi_record(&self, name_id: u16) -> Option<NaviRecordInfo> {
        let r = self
            .content
            .navis
            .iter()
            .filter_map(|n| n.name_record.as_ref())
            .chain(self.content.forms.iter().filter_map(|f| f.name_record.as_ref()))
            .find(|n| n.id == name_id)
            .map(crate::content::NameData::record)
            .or_else(|| self.content.rules.actor_records.get(name_id as usize).copied())?;
        Some(NaviRecordInfo { actor_type: actor_type_index(r.actor_type) as u8, ai_index: r.ai_index })
    }

    // ---- Panels -----------------------------------------------------------

    fn panel_valid(&self, p: PanelPos) -> bool {
        crate::field::is_valid(p.x, p.y)
    }

    fn all_field_objects(&self) -> Vec<ObjectRef> {
        self.field.objects.slots.iter().flatten().copied().collect()
    }

    fn side_field_objects(&self, side: u8) -> Vec<ObjectRef> {
        let first = (side as usize & 1) * 3;
        self.field.objects.slots[first..first + 3].iter().flatten().copied().collect()
    }

    fn panel_center(&self, p: PanelPos) -> (i32, i32) {
        kinds::player::panel_coordinates(p.x, p.y)
    }

    fn panel_flags(&self, p: PanelPos) -> u32 {
        self.field.flags(p.x, p.y)
    }

    fn panel_check(&self, p: PanelPos, require: u32, forbid: u32) -> bool {
        self.field.check(p.x, p.y, require, forbid)
    }

    fn panel_info(&self, p: PanelPos) -> Option<PanelInfo> {
        let panel = self.field.panel(p.x, p.y)?;
        Some(PanelInfo { kind: panel.kind as u8, alliance: panel.alliance, home: panel.home })
    }

    fn column_info(&self, x: u8) -> ColumnInfo {
        let c = self.field.columns.get(x as usize).copied().unwrap_or_default();
        ColumnInfo { home: c.home, timer: c.timer }
    }

    fn set_column_timer(&mut self, x: u8, ticks: u16) {
        if let Some(c) = self.field.columns.get_mut(x as usize) {
            c.timer = ticks;
        }
    }

    fn set_panel_alliance(&mut self, p: PanelPos, side: u8) {
        Battle::set_panel_alliance(self, p.x, p.y, side);
    }

    fn set_panel_type(&mut self, p: PanelPos, kind: u8) {
        let t = PanelType::ALL.get(kind as usize).copied().unwrap_or_else(|| panic!("panel type {kind} doesn't exist"));
        Battle::set_panel_type(self, p.x, p.y, t);
    }

    fn crack_panel(&mut self, p: PanelPos) -> bool {
        Battle::crack_panel(self, p.x, p.y)
    }

    fn break_panel(&mut self, p: PanelPos) -> bool {
        Battle::break_panel(self, p.x, p.y)
    }

    fn panel_solid(&self, p: PanelPos) -> bool {
        self.field.is_solid(p.x, p.y)
    }

    fn highlight_panel(&mut self, p: PanelPos) {
        common::highlight_panel(self, p.x, p.y);
    }

    fn reserve_panel(&mut self, o: ObjectRef, p: PanelPos) -> bool {
        Battle::reserve_panel(self, o, p.x, p.y)
    }

    fn unreserve_panel(&mut self, o: ObjectRef, p: PanelPos) -> bool {
        Battle::unreserve_panel(self, o, p.x, p.y)
    }

    fn release_reservations(&mut self, o: ObjectRef) {
        Battle::release_reservations(self, o);
    }

    fn can_step(&self, o: ObjectRef, p: PanelPos) -> bool {
        Battle::can_step(self, o, p.x, p.y)
    }

    fn can_stand_any_side(&self, o: ObjectRef, p: PanelPos) -> bool {
        if !crate::field::is_valid(p.x, p.y) {
            return false;
        }
        let ob = self.objects.get(o);
        let airshoe = ob.collision.is_some_and(|c| self.collision.get(c).f1 & f1::AIRSHOE != 0);
        let floor_free = airshoe || !self.field.is_solid(ob.panel.x, ob.panel.y);
        let rule = self.content.rules.panels.any_side_step.get(floor_free, ob.alliance);
        self.field.meets(p.x, p.y, rule)
    }

    // Panel changes (dimming chip subtypes 2, 3, 5, 15 and 27).
    fn poison_panel(&mut self, p: PanelPos) -> bool {
        Battle::poison_panel(self, p.x, p.y)
    }

    fn blink_panel(&mut self, p: PanelPos, kind: u8, side: u8) {
        let t = PanelType::ALL.get(kind as usize).copied().unwrap_or_else(|| panic!("panel type {kind} doesn't exist"));
        Battle::blink_panel(self, p.x, p.y, t, side);
    }

    fn break_empty_panel(&mut self, p: PanelPos) -> bool {
        Battle::break_empty_panel(self, p.x, p.y)
    }

    fn shatter_panel(&mut self, p: PanelPos) -> bool {
        Battle::shatter_panel(self, p.x, p.y)
    }

    // ---- Objects -----------------------------------------------------------

    fn spawn(&mut self, pool: Pool, index: u8, pos: Vec3, params: [u8; 4]) -> Option<ObjectRef> {
        super::spawn_object(self, pool, index, pos, params)
    }

    fn spawn_kind(&mut self, name: &str, pos: Vec3, params: [u8; 4]) -> ApiResult<Option<ObjectRef>> {
        Ok(kinds::spawn(self, kind_named(&self.content, name)?, SpawnAt::AfterCurrent, pos, params))
    }

    fn spawn_kind_first(&mut self, name: &str, pos: Vec3, params: [u8; 4]) -> ApiResult<Option<ObjectRef>> {
        Ok(kinds::spawn(self, kind_named(&self.content, name)?, SpawnAt::First, pos, params))
    }

    fn spawn_kind_at_end(&mut self, name: &str, pos: Vec3, params: [u8; 4]) -> ApiResult<Option<ObjectRef>> {
        Ok(kinds::spawn(self, kind_named(&self.content, name)?, SpawnAt::End, pos, params))
    }

    fn spawn_def(&mut self, kind: u16, pos: Vec3, at: SpawnAt) -> ApiResult<Option<ObjectRef>> {
        if kind as usize >= self.content.defs.kinds.len() {
            return Err(ApiError::Other(format!("no kind has handle {kind}")));
        }
        Ok(kinds::spawn(self, KindHandle(kind), at, pos, [0; 4]))
    }

    fn object_kind(&self, o: ObjectRef) -> Option<u16> {
        Some(self.objects.get(o).kind.0)
    }

    fn def_number(&self, registry: Registry, h: u16) -> ApiResult<u8> {
        self.content.defs.number(registry, h).ok_or_else(|| ApiError::Other(format!("no {registry} has handle {h}")))
    }

    fn navi_action(&self, o: ObjectRef) -> ApiResult<NaviAction> {
        let a = self.actor_of(o)?;
        if let Some(h) = kinds::player::running_content_action(self, o) {
            return Ok(NaviAction::Content(h.0));
        }
        let action = self.objects.get(o).action;
        if action >= 0x10 {
            if let Some(h) = self.content.defs.action_numbered(action) {
                return Ok(NaviAction::Content(h.0));
            }
            return Ok(match action {
                kinds::player::actions::movement::ACTION => NaviAction::Engine("move"),
                kinds::player::actions::dimming_chip::ACTION => NaviAction::Engine("dimming_chip"),
                kinds::player::actions::navi_chip::ACTION => NaviAction::Engine("navi_chip"),
                kinds::player::actions::instant::ACTION if matches!(a.attack.action, ActionVars::FormChange(_)) => {
                    NaviAction::Engine("form_change")
                }
                kinds::player::actions::instant::ACTION => NaviAction::Engine("instant_chip"),
                kinds::player::actions::cross_special::ACTION => NaviAction::Engine("cross_special"),
                n => NaviAction::Number(n),
            });
        }
        // The framework's states; a link navi's actions past idle are its own.
        const STATES: [&str; 9] =
            ["entry", "take_control", "deletion", "flinch", "paralysis", "drag", "freeze", "bubble", "idle"];
        Ok(match STATES.get(action as usize) {
            Some(name) if a.ai_index == 0 || action <= 8 => NaviAction::Engine(name),
            _ => match self.content.defs.action_numbered(action) {
                Some(h) => NaviAction::Content(h.0),
                None => NaviAction::Number(action),
            },
        })
    }

    fn free(&mut self, o: ObjectRef) {
        self.objects.free(o);
    }

    fn destroy(&mut self, o: ObjectRef) {
        kinds::generic_destroy(self, o);
    }

    fn lifecycle(&self, o: ObjectRef) -> Lifecycle {
        match self.objects.get(o).state {
            state::INIT => Lifecycle::Init,
            state::UPDATE => Lifecycle::Update,
            state::FINISH => Lifecycle::Finish,
            _ => Lifecycle::Destroy,
        }
    }

    fn set_lifecycle(&mut self, o: ObjectRef, l: Lifecycle) {
        let p = match l {
            Lifecycle::Init => Progress::default(),
            Lifecycle::Update => Progress::UPDATE,
            Lifecycle::Destroy => Progress::DESTROY,
            Lifecycle::Finish => Progress { state: state::FINISH, action: 0, phase: 0, phase_init: 0 },
        };
        common::set_progress(self, o, p);
    }

    fn set_lifecycle_only(&mut self, o: ObjectRef, l: Lifecycle) {
        self.objects.get_mut(o).state = match l {
            Lifecycle::Init => state::INIT,
            Lifecycle::Update => state::UPDATE,
            Lifecycle::Destroy => state::DESTROY,
            Lifecycle::Finish => state::FINISH,
        };
    }

    fn set_action(&mut self, o: ObjectRef, action: u8) {
        common::set_action(self, o, action);
    }

    fn param(&self, o: ObjectRef, n: usize) -> u8 {
        self.objects.get(o).params[n]
    }

    fn set_param(&mut self, o: ObjectRef, n: usize, v: u8) {
        self.objects.get_mut(o).params[n] = v;
    }

    fn get(&self, o: ObjectRef, f: ObjectField) -> Value {
        let ob = self.objects.get(o);
        if let Some(bit) = flag_bit(f) {
            return Value::Bool(ob.flags & bit != 0);
        }
        let i = |v: i64| Value::Int(v);
        match f {
            // The object slot registration by number gives its kind; a kind
            // content defines has none.
            ObjectField::Index => match self.content.defs.kind(ob.kind).slot {
                Some((_, index)) => i(index as i64),
                None => Value::Nil,
            },
            ObjectField::Action => i(ob.action as i64),
            ObjectField::Phase => i(ob.phase as i64),
            ObjectField::PhaseInit => i(ob.phase_init as i64),
            ObjectField::PanelX => i(ob.panel.x as i64),
            ObjectField::PanelY => i(ob.panel.y as i64),
            ObjectField::FuturePanelX => i(ob.future_panel.x as i64),
            ObjectField::FuturePanelY => i(ob.future_panel.y as i64),
            ObjectField::Alliance => i(ob.alliance as i64),
            ObjectField::Flip => i(ob.flip as i64),
            ObjectField::Anim => i(ob.anim as i64),
            ObjectField::AnimLoaded => i(ob.anim_loaded as i64),
            ObjectField::Element => i(ob.element as i64),
            ObjectField::Timer => i(ob.timer as i64),
            ObjectField::Timer2 => i(ob.timer2 as i64),
            ObjectField::Hp => i(ob.hp as i64),
            ObjectField::MaxHp => i(ob.max_hp as i64),
            ObjectField::Damage => i(ob.damage as i64),
            ObjectField::Stamina => i(ob.stamina as i64),
            ObjectField::NameId => i(ob.name_id as i64),
            ObjectField::PreventAnim => i(ob.prevent_anim as i64),
            ObjectField::Pos => Value::Vec3(ob.pos),
            ObjectField::Vel => Value::Vec3(ob.vel),
            ObjectField::Related1 => ob.related[0].into(),
            ObjectField::Related2 => ob.related[1].into(),
            ObjectField::DragStep => i(match ob.drag_step {
                crate::object::DragStep::Start => 0,
                crate::object::DragStep::Slide => 1,
                crate::object::DragStep::Recover => 2,
            }),
            ObjectField::Active
            | ObjectField::Visible
            | ObjectField::RunWhilePaused
            | ObjectField::RunWhileDimmed
            | ObjectField::NoSpriteUpdate
            | ObjectField::HoldsReservation => {
                unreachable!("flag fields are read above")
            }
        }
    }

    fn set(&mut self, o: ObjectRef, f: ObjectField, v: Value) -> ApiResult<()> {
        let v = store(f.name(), f.writable(), f.ty(), v)?;
        let ob = self.objects.get_mut(o);
        if let Some(bit) = flag_bit(f) {
            let FieldValue::Bool(on) = v else { unreachable!() };
            ob.flags = if on { ob.flags | bit } else { ob.flags & !bit };
            return Ok(());
        }
        match (f, v) {
            (ObjectField::Action, FieldValue::U8(x)) => ob.action = x,
            (ObjectField::Phase, FieldValue::U8(x)) => ob.phase = x,
            (ObjectField::PhaseInit, FieldValue::U8(x)) => ob.phase_init = x,
            (ObjectField::PanelX, FieldValue::U8(x)) => ob.panel.x = x,
            (ObjectField::PanelY, FieldValue::U8(x)) => ob.panel.y = x,
            (ObjectField::FuturePanelX, FieldValue::U8(x)) => ob.future_panel.x = x,
            (ObjectField::FuturePanelY, FieldValue::U8(x)) => ob.future_panel.y = x,
            (ObjectField::Alliance, FieldValue::U8(x)) => ob.alliance = x,
            (ObjectField::Flip, FieldValue::U8(x)) => ob.flip = x,
            (ObjectField::Anim, FieldValue::U8(x)) => ob.anim = x,
            (ObjectField::AnimLoaded, FieldValue::U8(x)) => ob.anim_loaded = x,
            (ObjectField::Element, FieldValue::U8(x)) => ob.element = x,
            (ObjectField::Timer, FieldValue::U16(x)) => ob.timer = x,
            (ObjectField::Timer2, FieldValue::U16(x)) => ob.timer2 = x,
            (ObjectField::Hp, FieldValue::U16(x)) => ob.hp = x,
            (ObjectField::MaxHp, FieldValue::U16(x)) => ob.max_hp = x,
            (ObjectField::Damage, FieldValue::U16(x)) => ob.damage = x,
            (ObjectField::Stamina, FieldValue::U16(x)) => ob.stamina = x,
            (ObjectField::NameId, FieldValue::U16(x)) => ob.name_id = x,
            (ObjectField::PreventAnim, FieldValue::U8(x)) => ob.prevent_anim = x,
            (ObjectField::Pos, FieldValue::Vec3(p)) => ob.pos = p,
            (ObjectField::Vel, FieldValue::Vec3(p)) => ob.vel = p,
            (ObjectField::Related1, FieldValue::Object(r)) => ob.related[0] = r,
            (ObjectField::Related2, FieldValue::Object(r)) => ob.related[1] = r,
            (ObjectField::DragStep, FieldValue::Enum(i)) => {
                use crate::object::DragStep;
                ob.drag_step = [DragStep::Start, DragStep::Slide, DragStep::Recover][i as usize]
            }
            (f, v) => unreachable!("{f:?} stored as {v:?}"),
        }
        Ok(())
    }

    fn facing(&self, o: ObjectRef) -> i32 {
        let ob = self.objects.get(o);
        common::facing(ob.alliance, ob.flip)
    }

    fn set_animation(&mut self, o: ObjectRef, anim: u8) {
        common::set_animation(self, o, anim);
    }

    fn update_sprite(&mut self, o: ObjectRef) {
        common::update_sprite(self, o);
    }

    fn update_sprite_while_dimmed(&mut self, o: ObjectRef) {
        common::update_sprite_while_dimmed(self, o);
    }

    fn update_sprite_even_paused(&mut self, o: ObjectRef) {
        common::update_sprite_even_paused(self, o);
    }

    fn step_sprite(&mut self, o: ObjectRef) {
        common::step_sprite(self, o);
    }

    fn update_sprite_while_paused(&mut self, o: ObjectRef) {
        common::update_sprite_while_paused(self, o);
    }

    fn attach_point(&self, o: ObjectRef, n: u8) -> (i32, i32) {
        kinds::player::attach_point(self, o, n as usize)
    }

    fn set_coordinates_from_panel(&mut self, o: ObjectRef) {
        common::set_coordinates_from_panels(self, o);
    }

    fn set_panel_from_coordinates(&mut self, o: ObjectRef) {
        common::set_panels_from_coordinates(self, o);
    }

    fn update_visibility(&mut self, o: ObjectRef) {
        if !self.is_dimmed() {
            self.objects.get_mut(o).flags |= flags::VISIBLE;
        }
        let alliance = self.objects.get(o).alliance;
        if self.is_remote(alliance) {
            let blind = self.player(alliance ^ 1).and_then(|p| self.objects.get(p).collision).is_some_and(|c| self.collision.get(c).f1 & f1::BLIND != 0);
            if blind {
                self.objects.get_mut(o).flags &= !flags::VISIBLE;
            }
        }
    }

    fn update_collision_panels(&mut self, o: ObjectRef) {
        Battle::update_collision_panels(self, o);
    }

    fn snap_to_future_panel(&mut self, o: ObjectRef) {
        kinds::player::snap_to_future_panel(self, o);
    }

    fn state(&self, o: ObjectRef) -> Option<&ContentState> {
        match &self.objects.get(o).vars {
            Vars::Content(s) => Some(s),
            _ => None,
        }
    }

    fn state_mut(&mut self, o: ObjectRef) -> Option<&mut ContentState> {
        match &mut self.objects.get_mut(o).vars {
            Vars::Content(s) => Some(s),
            _ => None,
        }
    }

    fn spawn_effect(&mut self, pos: Vec3, id: u8, flip: u8, palette_add: u8, priority: u8) -> Option<ObjectRef> {
        kinds::effect::spawn(self, pos, id, flip, palette_add, priority)
    }

    fn spawn_region_effects(&mut self, x: i32, y: i32, region: u8, side: u8, id: u8, z: i32) {
        kinds::effect::spawn_over_region(self, x, y, region, side, id, z);
    }

    fn spawn_hitbox(&mut self, owner: ObjectRef, s: &HitboxSpec) -> Option<ObjectRef> {
        let spec = kinds::hitbox::HitboxSpec {
            panel: s.panel,
            element: s.element,
            z: s.z,
            region: s.region,
            hit_effect: s.hit_effect,
            target: s.target,
            self_type: s.self_type,
            damage: s.damage,
            stamina: s.stamina,
            hit_mod: s.hit_mod,
            status: s.status,
            bug: s.bug,
            bug_arg: s.bug_arg,
        };
        kinds::hitbox::spawn(self, owner, &spec)
    }

    fn spawn_spark(&mut self, owner: ObjectRef, pos: Vec3, id: u8) -> Option<ObjectRef> {
        kinds::spark::spawn(self, owner, pos, id)
    }


    fn spawn_form_overlay(
        &mut self,
        owner: ObjectRef,
        sprite: SpriteId,
        stepping: u8,
        anim_offset: u8,
        nudged: bool,
        owner_palette: bool,
    ) -> Option<ObjectRef> {
        use kinds::form_overlay::{Palette, Stepping, Vars, spawn_with};
        let palette = if owner_palette { Palette::Owner } else { Palette::Own };
        let spec = Vars { sprite: Some(sprite), nudged, anim_offset, stepping: Stepping::from_param(stepping), palette, ..Vars::default() };
        spawn_with(self, owner, spec)
    }

    fn spawn_palette_flash(&mut self, variant: u8, ticks: u8, while_dimmed: bool, while_paused: bool) -> Option<ObjectRef> {
        kinds::palette_flash::spawn_variant(self, variant, ticks, while_dimmed, while_paused)
    }

    fn death_hook(&mut self, o: ObjectRef, name_id: u16) {
        kinds::player::form::navi_death_hook(self, o, name_id);
    }

    fn add_navi_parts(&mut self, o: ObjectRef, actor_type: u8, ai_index: u8, arg: u8) {
        kinds::player::form::record_init_hook(self, o, actor_type_of(actor_type), ai_index, arg);
    }

    fn remove_navi_parts(&mut self, o: ObjectRef, actor_type: u8, ai_index: u8) {
        kinds::player::form::record_death_hook(self, o, actor_type_of(actor_type), ai_index);
    }

    fn add_parts_of(&mut self, o: ObjectRef, owner: ObjectRef, keep_stepping: bool, paused_stepping: bool) {
        let rec = self.content.navi_record(self.objects.get(owner).name_id);
        let r2 = if paused_stepping { 1 } else { rec.version };
        kinds::player::form::record_init_hook(self, o, rec.actor_type, rec.ai_index, r2);
        if keep_stepping && let Some(part) = self.objects.get(o).related[1] {
            kinds::player::form::keep_overlay_stepping(self, part);
        }
    }

    fn remove_parts_of(&mut self, o: ObjectRef, owner: ObjectRef) {
        let rec = self.content.navi_record(self.objects.get(owner).name_id);
        kinds::player::form::record_death_hook(self, o, rec.actor_type, rec.ai_index);
    }

    fn spawn_afterimage(&mut self, owner: ObjectRef, pos: Vec3, spec: &bn6_content_api::api::AfterimageSpec) -> Option<ObjectRef> {
        use kinds::afterimage::{PlainLook, PlainShadow, Tether};
        let look = PlainLook {
            color_shader: spec.color_shader,
            shadow: match spec.shadow {
                Shadow::WithSprite => PlainShadow::WithSprite,
                Shadow::Ground => PlainShadow::Ground,
                Shadow::Hidden => PlainShadow::Hidden,
            },
            palette: spec.palette,
            steady: spec.steady,
        };
        let tether = match spec.tether {
            1 => Tether::BeastForm,
            2 => Tether::Attack,
            _ => Tether::None,
        };
        match spec.sprite {
            Some(sprite) => {
                kinds::afterimage::spawn_plain(self, owner, pos, sprite, spec.anim, spec.flip, spec.lifetime, tether, look)
            }
            None => kinds::afterimage::spawn_copy(self, owner, pos, spec.anim, spec.flip, spec.lifetime, tether, look),
        }
    }

    // ---- Navis and the attack in progress -------------------------------------

    fn actor_get(&self, o: ObjectRef, f: ActorField) -> ApiResult<Value> {
        let a = self.actor_of(o)?;
        let at = &a.attack;
        let i = |v: i64| Value::Int(v);
        Ok(match f {
            ActorField::Overlay => a.overlay.into(),
            ActorField::Step => i(at.step as i64),
            ActorField::StepInit => i(at.step_init as i64),
            ActorField::Variant => i(at.variant as i64),
            ActorField::Chip => i(self.api_chip_field(at.chip, 0) as i64),
            ActorField::AttackElement => i(at.element as i64),
            ActorField::AttackDamage => i(at.damage as i64),
            ActorField::HitParam => i(at.hit_param as i64),
            ActorField::Charged => i(at.charged as i64),
            ActorField::AttackLockout => i(at.lockout as i64),
            ActorField::Extra => i(at.extra as i64),
            ActorField::SpecialSource => i(at.special_source as i64),
            ActorField::AttackKind => i(at.kind as i64),
            ActorField::BeastLockon => i(at.beast_lockon as i64),
            ActorField::Marker => i(at.marker as i64),
            ActorField::Recovery => i(at.recovery as i64),
            ActorField::ActorType => i(actor_type_index(a.actor_type)),
            ActorField::AiIndex => i(a.ai_index as i64),
            ActorField::LockonMarker => a.lockon_marker.into(),
            ActorField::ChargeGlow => a.charge_glow.into(),
            ActorField::FullSynchroAura => a.full_synchro_aura.into(),
            ActorField::ChargeLevel => i(a.charge_level as i64),
            ActorField::ChargeSource => i(a.charge_source as i64),
            ActorField::ChargeCounter => i(a.charge_counter as i64),
            ActorField::BufferedMove => i(a.buffered_move as i64),
            ActorField::ChipLockout => i(a.lockout as i64),
            ActorField::BackSpecialCooldown => i(a.back_special_cooldown as i64),
            ActorField::BusterRoutine => i(self.api_weapon(a.buster)),
            ActorField::ChargeShotRoutine => i(self.api_weapon(a.charge_shot)),
            ActorField::BackSpecialRoutine => i(self.api_weapon(a.back_special)),
            ActorField::AChargeRoutine => i(self.api_weapon(a.a_charge)),
            ActorField::AltAChargeRoutine => i(self.api_weapon(a.alt_a_charge)),
            ActorField::Mode9ARoutine => i(self.api_weapon(a.mode9_a)),
            ActorField::BeastOutSpent => Value::Bool(a.beast_out_spent),
            ActorField::BarrierVisual => a.barrier_visual.into(),
        })
    }

    fn actor_set(&mut self, o: ObjectRef, f: ActorField, v: Value) -> ApiResult<()> {
        let v = store(f.name(), f.writable(), f.ty(), v)?;
        let chip = match (f, v) {
            (ActorField::Chip, FieldValue::U16(x)) => self.chip_from_api(x, 0)?,
            _ => None,
        };
        let weapon = match v {
            FieldValue::U8(x) if matches!(f, ActorField::BusterRoutine | ActorField::ChargeShotRoutine) => {
                self.weapon_from_api(x)
            }
            _ => None,
        };
        let a = self.actor_of_mut(o)?;
        let at = &mut a.attack;
        match (f, v) {
            (ActorField::Overlay, FieldValue::Object(r)) => a.overlay = r,
            (ActorField::Step, FieldValue::U8(x)) => at.step = x,
            (ActorField::StepInit, FieldValue::U8(x)) => at.step_init = x,
            (ActorField::Variant, FieldValue::U8(x)) => at.variant = x,
            (ActorField::Chip, FieldValue::U16(_)) => at.chip = chip,
            (ActorField::AttackElement, FieldValue::U8(x)) => at.element = x,
            (ActorField::AttackDamage, FieldValue::U16(x)) => at.damage = x,
            (ActorField::HitParam, FieldValue::U16(x)) => at.hit_param = x,
            (ActorField::Charged, FieldValue::U8(x)) => at.charged = x,
            (ActorField::AttackLockout, FieldValue::U8(x)) => at.lockout = x,
            (ActorField::Extra, FieldValue::U16(x)) => at.extra = x,
            (ActorField::SpecialSource, FieldValue::U8(x)) => at.special_source = x,
            (ActorField::BeastLockon, FieldValue::U8(x)) => at.beast_lockon = x,
            (ActorField::Marker, FieldValue::U32(x)) => at.marker = x,
            (ActorField::Recovery, FieldValue::U16(x)) => at.recovery = x,
            (ActorField::LockonMarker, FieldValue::Object(r)) => a.lockon_marker = r,
            (ActorField::ChargeGlow, FieldValue::Object(r)) => a.charge_glow = r,
            (ActorField::FullSynchroAura, FieldValue::Object(r)) => a.full_synchro_aura = r,
            (ActorField::BufferedMove, FieldValue::U8(x)) => a.buffered_move = x,
            (ActorField::ChipLockout, FieldValue::U8(x)) => a.lockout = x,
            (ActorField::BackSpecialCooldown, FieldValue::U8(x)) => a.back_special_cooldown = x,
            (ActorField::BusterRoutine, FieldValue::U8(_)) => a.buster = weapon,
            (ActorField::ChargeShotRoutine, FieldValue::U8(_)) => a.charge_shot = weapon,
            (ActorField::BeastOutSpent, FieldValue::Bool(x)) => a.beast_out_spent = x,
            (ActorField::BarrierVisual, FieldValue::Object(r)) => a.barrier_visual = r,
            (f, v) => unreachable!("{f:?} stored as {v:?}"),
        }
        Ok(())
    }

    fn attack_param(&self, o: ObjectRef, n: usize) -> ApiResult<u8> {
        Ok(self.actor_of(o)?.attack.params[n])
    }

    fn set_attack_param(&mut self, o: ObjectRef, n: usize, v: u8) -> ApiResult<()> {
        self.actor_of_mut(o)?.attack.params[n] = v;
        Ok(())
    }

    fn request(&self, o: ObjectRef, f: RequestFlag) -> ApiResult<bool> {
        Ok(self.actor_of(o)?.requests & request_bit(f) != 0)
    }

    fn set_request(&mut self, o: ObjectRef, f: RequestFlag, on: bool) -> ApiResult<()> {
        let a = self.actor_of_mut(o)?;
        let bit = request_bit(f);
        a.requests = if on { a.requests | bit } else { a.requests & !bit };
        Ok(())
    }

    fn navi_state(&self, o: ObjectRef, f: NaviState) -> ApiResult<bool> {
        Ok(self.actor_of(o)?.status & navi_state_bit(f) != 0)
    }

    fn set_navi_state(&mut self, o: ObjectRef, f: NaviState, on: bool) -> ApiResult<()> {
        let a = self.actor_of_mut(o)?;
        let bit = navi_state_bit(f);
        a.status = if on { a.status | bit } else { a.status & !bit };
        Ok(())
    }

    fn key(&self, o: ObjectRef, pad: Pad, key: Key) -> ApiResult<bool> {
        let a = self.actor_of(o)?;
        let word = match pad {
            Pad::Held => a.pad.held,
            Pad::Pressed => a.pad.pressed,
            Pad::Released => a.pad.released,
            Pad::DimmedHeld => a.dimmed_pad.held,
            Pad::DimmedPressed => a.dimmed_pad.pressed,
            Pad::DimmedReleased => a.dimmed_pad.released,
        };
        Ok(word & key_bit(key) != 0)
    }

    fn action_state_mut(&mut self, o: ObjectRef) -> ApiResult<&mut ContentState> {
        // The running action: the content action the attack started, else
        // the one registered by the navi's action number.
        self.actor_of(o)?;
        let running = kinds::player::running_content_action(self, o);
        let action = match running {
            Some(h) => Value::Def(Registry::Action, h.0),
            None => Value::Int(self.objects.get(o).action as i64),
        };
        let id = self.action_schema(action).map_err(|_| ApiError::NoState(o))?;
        self.attack_state_for(o, id)
    }

    fn attack_state_for(&mut self, o: ObjectRef, id: StateId) -> ApiResult<&mut ContentState> {
        let a = self.actor_of_mut(o)?;
        // The game keeps an action's variables in the shared attack state,
        // where they outlive the action; an action of another layout starts
        // from zero.
        if !matches!(&a.attack.action, ActionVars::Content(s) if s.id() == id) {
            a.attack.action = ActionVars::Content(ContentState::new(id));
        }
        match &mut a.attack.action {
            ActionVars::Content(s) => Ok(s),
            _ => unreachable!(),
        }
    }

    fn action_schema(&self, action: Value) -> ApiResult<StateId> {
        let defs = &self.content.defs;
        match action {
            Value::Int(n) => u8::try_from(n)
                .ok()
                .and_then(|n| defs.action_numbered(n))
                .map(|h| defs.action(h).schema)
                .ok_or_else(|| ApiError::Other(format!("no content action has the number {n:#x}"))),
            Value::Def(Registry::Action, h) if (h as usize) < defs.actions.len() => Ok(defs.action(ActionHandle(h)).schema),
            v => Err(ApiError::Other(format!("{v:?} is not an action"))),
        }
    }

    fn status(&self, o: ObjectRef, flag: StatusFlag) -> ApiResult<bool> {
        Ok(self.collision_of(o)?.f1 & status_bit(flag) != 0)
    }

    fn set_status(&mut self, o: ObjectRef, flag: StatusFlag, on: bool) -> ApiResult<()> {
        let c = self.collision_of_mut(o)?;
        if on {
            c.f1 |= status_bit(flag);
        } else {
            c.f1 &= !status_bit(flag);
        }
        Ok(())
    }

    fn status_timer(&self, o: ObjectRef, t: StatusTimer) -> ApiResult<u16> {
        Ok(self.collision_of(o)?.status_timers[timer_index(t)])
    }

    fn set_status_timer(&mut self, o: ObjectRef, t: StatusTimer, v: u16) -> ApiResult<()> {
        self.collision_of_mut(o)?.status_timers[timer_index(t)] = v;
        Ok(())
    }

    fn open_counter_window(&mut self, o: ObjectRef) {
        kinds::player::actions::open_counter_window(self, o);
    }

    fn check_reactive_abort(&mut self, o: ObjectRef) {
        kinds::player::actions::check_reactive_abort(self, o);
    }

    fn start_stance_counter(&mut self, o: ObjectRef) {
        kinds::player::actions::reactive::stance_counter(self, o);
    }

    fn refresh_form_overlay(&mut self, o: ObjectRef) {
        kinds::player::refresh_form_overlay(self, o);
    }

    fn exit_attack(&mut self, o: ObjectRef) {
        kinds::player::exit_attack_state(self, o);
    }

    fn end_attack(&mut self, o: ObjectRef) {
        kinds::player::end_attack(self, o);
    }

    fn set_attack(&mut self, o: ObjectRef, action: u8, kind: u8) {
        kinds::player::set_attack(self, o, action, kind);
    }

    fn set_content_attack(&mut self, o: ObjectRef, action: u16, kind: u8) -> ApiResult<()> {
        if action as usize >= self.content.defs.actions.len() {
            return Err(ApiError::Other(format!("no action has handle {action}")));
        }
        let attack = kinds::player::NaviAttack::content(&self.content.defs, ActionHandle(action));
        kinds::player::set_attack(self, o, attack, kind);
        Ok(())
    }

    fn reset_attack_links(&mut self, o: ObjectRef) {
        kinds::player::reset_attack_links(self, o);
    }

    fn held_direction(&self, o: ObjectRef) -> u8 {
        kinds::player::idle::held_direction(self, o)
    }

    fn step_target(&self, o: ObjectRef, dir: u8) -> Option<PanelPos> {
        kinds::player::actions::movement::step_target(self, o, dir)
    }

    fn start_move(&mut self, o: ObjectRef, dir: u8) {
        kinds::player::idle::start_move(self, o, dir);
    }

    fn lockon_panel(&self, o: ObjectRef, target: PanelPos, mode: u8) -> PanelPos {
        kinds::player::actions::beast_rush::lockon_panel(self, o, target, mode)
    }

    fn can_move(&self, o: ObjectRef) -> bool {
        self.collision_of(o).is_ok_and(|c| c.f1 & (f1::IMMOBILIZED | f1::SLIDING | f1::MOVING) == 0)
    }

    fn heal(&mut self, o: ObjectRef, amount: u16, anti_recovery: bool) -> bool {
        kinds::heal::heal(self, o, amount, anti_recovery)
    }

    fn buster_damage(&self, o: ObjectRef) -> u16 {
        kinds::player::idle::buster_damage(self, o)
    }

    fn prepare_chip(&mut self, o: ObjectRef) -> u8 {
        kinds::player::prepare_chip(self, o)
    }

    fn absorbed(&self, o: ObjectRef) -> ApiResult<Vec<(u8, u8)>> {
        Ok(self.actor_of(o)?.absorbed.iter().map(|a| (a.kind, a.anim)).collect())
    }

    fn push_absorbed(&mut self, o: ObjectRef, kind: u8, anim: u8) -> ApiResult<bool> {
        let list = &mut self.actor_of_mut(o)?.absorbed;
        if list.len() >= 8 {
            return Ok(false);
        }
        list.push(AbsorbedObstacle { kind, anim });
        Ok(true)
    }

    fn pop_absorbed(&mut self, o: ObjectRef) -> ApiResult<Option<(u8, u8)>> {
        Ok(self.actor_of_mut(o)?.absorbed.pop().map(|a| (a.kind, a.anim)))
    }

    // ---- Sprites -------------------------------------------------------------------

    fn sprite_load(&mut self, o: ObjectRef, id: SpriteId) {
        // `sprite_load` also lets the sprite animate (header flag 0x08).
        self.objects.sprite_mut(o).load(id);
        self.objects.get_mut(o).flags &= !flags::NO_SPRITE_UPDATE;
    }

    fn sprite_load_look_of(&mut self, o: ObjectRef, owner: ObjectRef) {
        let name_id = self.objects.get(owner).name_id;
        let id = if self.content.navi_record(name_id).actor_type == crate::actor::ActorType::Player {
            kinds::player::stats_sprite(self, self.objects.get(owner).alliance)
        } else {
            self.content
                .objects
                .name_looks
                .iter()
                .find(|l| l.name_id == name_id)
                .and_then(|l| l.sprite)
                .unwrap_or_else(|| panic!("NameID {name_id:#x} has no look (sub_800F26C)"))
        };
        self.sprite_load(o, id);
    }

    fn sprite_load_like(&mut self, o: ObjectRef, like: ObjectRef) -> ApiResult<()> {
        let name_id = self.objects.get(like).name_id;
        let id = if self.content.navi_record(name_id).actor_type == crate::actor::ActorType::Player {
            // sub_800FC9E(navi stat 0x29, form stat 0x2C): MegaMan's form's
            // sprite, or the link navi's.
            let side = self.objects.get(like).alliance as usize & 1;
            let s = &self.stats[side];
            if self.navi(side) == crate::setup::Navi::MEGAMAN {
                self.content.form(s.form).sprite
            } else {
                self.content.navi(s.navi).sprite
            }
        } else {
            // sub_800F26C: a field object's look by its NameID (0xCD and
            // up); other NameIDs' sprites (viruses, bosses) aren't in the
            // pack: no netbattle object stands in for one.
            let look = self.content.objects.name_looks.iter().find(|l| l.name_id == name_id);
            look.and_then(|l| l.sprite).ok_or_else(|| {
                ApiError::Other(format!(
                    "sub_800F26C: NameID {name_id:#x} has no sprite in the pack (a netbattle's stand-in copies a player)"
                ))
            })?
        };
        self.sprite_load(o, id);
        Ok(())
    }

    fn sprite_set_animation(&mut self, o: ObjectRef, anim: u8) {
        self.objects.sprite_mut(o).set_animation(anim, &self.content);
    }

    fn sprite_step(&mut self, o: ObjectRef) {
        self.objects.sprite_mut(o).update(&self.content);
    }

    fn sprite_get(&self, o: ObjectRef, f: SpriteField) -> Value {
        let s = self.objects.sprite(o);
        let look = &s.look;
        match f {
            SpriteField::Palette => Value::Int(look.palette as i64),
            SpriteField::HFlip => Value::Bool(look.hflip),
            SpriteField::VFlip => Value::Bool(look.vflip),
            SpriteField::Shadow => Value::Int(match look.shadow {
                sprite::Shadow::Hidden => 0,
                sprite::Shadow::Ground => 1,
                sprite::Shadow::WithSprite => 2,
            }),
            SpriteField::White => Value::Bool(look.white),
            SpriteField::ColorShader => Value::Int(look.color_shader as i64),
            SpriteField::Alpha => look.alpha.map_or(Value::Nil, |a| Value::Int(a as i64)),
            SpriteField::Mosaic => look.mosaic.map_or(Value::Nil, |a| Value::Int(a as i64)),
            SpriteField::Priority => Value::Int(look.priority as i64),
            SpriteField::HiddenParts => Value::Int(look.hidden_parts as i64),
            SpriteField::Animation => Value::Int(s.anim as i64),
            SpriteField::FrameFlags => Value::Int(s.frame_parameters() as i64),
            SpriteField::Finished => Value::Bool(s.finished()),
        }
    }

    fn sprite_set(&mut self, o: ObjectRef, f: SpriteField, v: Value) -> ApiResult<()> {
        let v = store(f.name(), f.writable(), f.ty(), v)?;
        let look = &mut self.objects.sprite_mut(o).look;
        match (f, v) {
            (SpriteField::Palette, FieldValue::U8(x)) => look.palette = x,
            (SpriteField::HFlip, FieldValue::Bool(x)) => look.hflip = x,
            (SpriteField::VFlip, FieldValue::Bool(x)) => look.vflip = x,
            (SpriteField::Shadow, FieldValue::Enum(i)) => {
                look.shadow = match Shadow::ALL[i as usize] {
                    Shadow::Hidden => sprite::Shadow::Hidden,
                    Shadow::Ground => sprite::Shadow::Ground,
                    Shadow::WithSprite => sprite::Shadow::WithSprite,
                }
            }
            (SpriteField::White, FieldValue::Bool(x)) => look.white = x,
            (SpriteField::ColorShader, FieldValue::U16(x)) => look.color_shader = x,
            (SpriteField::Alpha, FieldValue::OptionalU8(x)) => look.alpha = x,
            (SpriteField::Mosaic, FieldValue::OptionalU8(x)) => look.mosaic = x,
            (SpriteField::Priority, FieldValue::U8(x)) => look.priority = x,
            (SpriteField::HiddenParts, FieldValue::U32(x)) => look.hidden_parts = x,
            (f, v) => unreachable!("{f:?} stored as {v:?}"),
        }
        Ok(())
    }

    // ---- Collision ---------------------------------------------------------------------

    fn create_collision(&mut self, o: ObjectRef) -> bool {
        Battle::create_collision(self, o).is_some()
    }

    fn setup_collision(&mut self, o: ObjectRef, self_type: u8, target_type: u8, hit_mod: u8) {
        Battle::setup_collision(self, o, self_type, target_type, hit_mod);
    }

    fn reset_collision_types(&mut self, o: ObjectRef, self_type: u8, target_type: u8, hit_mod: u8) {
        Battle::reset_collision_types(self, o, self_type, target_type, hit_mod);
    }

    fn collision_get(&self, o: ObjectRef, f: CollisionField) -> ApiResult<Value> {
        let c = self.collision_of(o)?;
        Ok(Value::Int(match f {
            CollisionField::Region => c.region as i64,
            CollisionField::PanelX => c.panel.x as i64,
            CollisionField::PanelY => c.panel.y as i64,
            CollisionField::HitEffect => c.hit_effect as i64,
            CollisionField::StatusBase => c.status_base as i64,
            CollisionField::Bugs => c.bugs as i64,
            CollisionField::HitModBase => c.hit_mod_base as i64,
            CollisionField::SelfDamage => c.self_damage as i64,
            CollisionField::CounterByte => c.counter_byte as i64,
            CollisionField::HitFlags => c.acc.hit_flags as i64,
            CollisionField::FinalDamage => c.acc.final_damage as i64,
            CollisionField::Direction => c.direction as i64,
            CollisionField::GuardDirs => c.guard_dirs as i64,
            CollisionField::DamageElements => c.acc.damage_elements as i64,
            CollisionField::HitModFinal => c.hit_mod_final as i64,
            // BARRIER_STATES
            CollisionField::Barrier => match c.barrier {
                0 => 0,
                0x10 => 2,
                _ => 1,
            },
            CollisionField::BarrierHp => c.barrier_hp as i64,
            CollisionField::BarrierPopHitMod => c.barrier_saved_hmf as i64,
        }))
    }

    fn collision_element_damage(&self, o: ObjectRef, element: u8) -> ApiResult<u16> {
        let damage = self.collision_of(o)?.acc.element_damage;
        damage
            .get(element as usize)
            .copied()
            .ok_or_else(|| ApiError::Other(format!("element {element} has no damage slot (0 to 5)")))
    }

    fn collision_hit_by(&self, o: ObjectRef) -> ApiResult<Vec<ObjectRef>> {
        let mask = self.collision_of(o)?.acc.hit_by;
        let hitters = (0..32u8).filter(|&k| mask & (0x8000_0000 >> k) != 0);
        Ok(hitters.filter_map(|k| self.collision.get(crate::collision::CollisionId(k)).parent).collect())
    }

    fn collision_set(&mut self, o: ObjectRef, f: CollisionField, v: Value) -> ApiResult<()> {
        let v = store(f.name(), f.writable(), f.ty(), v)?;
        let c = self.collision_of_mut(o)?;
        let x = int(v);
        match f {
            CollisionField::Region => c.region = x as u8,
            CollisionField::PanelX => c.panel.x = x as u8,
            CollisionField::PanelY => c.panel.y = x as u8,
            CollisionField::HitEffect => c.hit_effect = x as u8,
            CollisionField::StatusBase => c.status_base = x as u8,
            CollisionField::Bugs => c.bugs = x as u16,
            CollisionField::HitModBase => c.hit_mod_base = x as u8,
            CollisionField::SelfDamage => c.self_damage = x as u16,
            CollisionField::CounterByte => c.counter_byte = x as u8,
            CollisionField::HitFlags => c.acc.hit_flags = x as u32,
            CollisionField::FinalDamage
            | CollisionField::GuardDirs
            | CollisionField::DamageElements
            | CollisionField::HitModFinal
            | CollisionField::Direction
            | CollisionField::Barrier
            | CollisionField::BarrierHp
            | CollisionField::BarrierPopHitMod => {
                unreachable!("read-only")
            }
        }
        Ok(())
    }

    fn raise_barrier(&mut self, o: ObjectRef, spec: bn6_content_api::api::BarrierSpec) -> ApiResult<()> {
        let c = self.collision_of_mut(o)?;
        // The barrier byte the ruleset's barrier code (`sub_801A802`) tells
        // the behaviors apart by: a plain barrier as the game's type 1
        // (types 1..7, 9 and 0xB..0xF behave alike), a bubble as type 8, a
        // regenerating one as type 0xA.
        c.barrier = match spec.behavior {
            0 => 1,
            1 => 8,
            _ => 0xA,
        };
        c.barrier_weak = spec.weak_element;
        c.barrier_hp = spec.hp;
        c.barrier_threshold = spec.threshold;
        c.barrier_timer = spec.timer;
        Ok(())
    }

    fn take_damage(&mut self, o: ObjectRef, mode: u8) -> i32 {
        kinds::common::take_damage(self, o, mode)
    }

    fn present_collision(&mut self, o: ObjectRef) {
        let c = self.objects.get(o).collision.expect("presenting an object without collision data");
        Battle::present_collision(self, c);
    }

    fn remove_collision(&mut self, o: ObjectRef) {
        let c = self.objects.get(o).collision.expect("removing an object without collision data");
        Battle::remove_collision(self, c);
    }

    fn free_collision(&mut self, o: ObjectRef) {
        let c = self.objects.get(o).collision.expect("freeing an object without collision data");
        self.collision.free(c);
    }

    fn hit_spark(&mut self, o: ObjectRef) {
        kinds::spark::spawn_collision_effect(self, o);
    }

    fn set_collision_panel(&mut self, o: ObjectRef) {
        let obj = self.objects.get(o);
        let (Some(id), panel) = (obj.collision, obj.panel) else { return };
        self.collision.get_mut(id).panel = panel;
    }

    fn highlight_collision_panels(&mut self, o: ObjectRef) {
        let ob = self.objects.get(o);
        let c = ob.collision.expect("highlighting the panels of an object without collision data");
        let dir = common::facing(ob.alliance, ob.flip);
        let s = self.collision.get(c);
        let (x, y) = (s.panel.x as i32, s.panel.y as i32);
        let panels: Vec<(i32, i32)> =
            self.content.region(s.region).iter().map(|p| (x + p.dx as i32 * dir, y + p.dy as i32)).collect();
        // `object_highlightPanel` skips panels off the field.
        for (px, py) in panels {
            if (1..=6).contains(&px) && (1..=3).contains(&py) {
                common::highlight_panel(self, px as u8, py as u8);
            }
        }
    }

    // ---- Services ------------------------------------------------------------

    fn dimming(&mut self, o: ObjectRef, step: DimmingStep, chip: u16) {
        use crate::dimming as d;
        // A controller's chip field: 0 for none.
        let chip = self.chip_from_api(chip, 0).unwrap_or_else(|e| panic!("a dimming step's chip: {e}"));
        match step {
            DimmingStep::Begin => d::begin(self, o),
            DimmingStep::DimScreen => d::dim_screen(self, o),
            DimmingStep::ShowTelop => d::show_telop(self, o),
            DimmingStep::ShowHiddenTelop => d::show_hidden_telop(self, o),
            DimmingStep::CheckAntiNavi => d::check_anti_navi(self, o, chip),
            DimmingStep::ShowNaviTelop => d::show_navi_telop(self, o, chip),
            DimmingStep::UndimScreen => d::undim_screen(self, o),
            DimmingStep::Finish => d::end(self, o),
        }
    }

    fn start_dimming(&mut self, side: u8, no_cut_in: bool, controller: Option<ObjectRef>, user: ObjectRef) {
        Battle::start_dimming(self, side & 1, no_cut_in, controller, user);
    }

    fn hide_user(&mut self, user: ObjectRef) {
        crate::dimming::hide_user(self, user);
    }

    fn show_user(&mut self, user: ObjectRef) {
        crate::dimming::show_user(self, user);
    }

    fn hide_user_sparing(&mut self, user: ObjectRef) {
        crate::dimming::hide_user_sparing(self, user);
    }

    fn clear_navicust_bugs(&mut self, side: u8) {
        let b = &mut self.stats[side as usize & 1].bugs;
        b.processing = 0;
        b.panel_trail_level = 0;
        b.buster_blanks = 0;
        b.hit_status = 0;
        b.custom_damage = 0;
        b.emotion = 0;
        b.custom_drain = 0;
        b.hp_drain = 0;
        b.battle_start = 0;
        b.hand_shrink_turn = 0;
    }

    fn navi_chip_left(&mut self, controller: ObjectRef) {
        kinds::navi_chip::navi_left(self, controller);
    }

    fn navi_warp(&mut self, user: ObjectRef, out: bool) {
        use kinds::navi_warp::{Warp, spawn};
        spawn(self, user, if out { Warp::Out } else { Warp::In });
    }

    // ---- Obstacles ----------------------------------------------------------------------

    fn obstacle_register(&mut self, o: ObjectRef, side: u8, class: u8) {
        kinds::obstacle::register(self, o, side & 1, class & 1);
    }

    fn obstacle_unregister(&mut self, o: ObjectRef) {
        kinds::obstacle::unregister(self, o);
    }

    fn obstacle_take_hits(&mut self, o: ObjectRef, push: ObstaclePush) -> ApiResult<()> {
        use kinds::obstacle::Push;
        self.collision_of(o)?;
        let push = match push {
            ObstaclePush::ForgetsDamage => Push::ForgetsDamage,
            ObstaclePush::KeepsDamage => Push::KeepsDamage,
            ObstaclePush::AnyHit => Push::AnyHit,
            ObstaclePush::Ignored => Push::Ignored,
        };
        kinds::obstacle::take_hits(self, o, push);
        Ok(())
    }

    fn obstacle_tick_lifetime(&mut self, o: ObjectRef) -> ApiResult<()> {
        self.collision_of(o)?;
        kinds::obstacle::tick_lifetime(self, o);
        Ok(())
    }

    fn obstacle_react(&mut self, o: ObjectRef, crush: ObstacleCrush, hold: ObstacleHold) -> ApiResult<Option<u8>> {
        use kinds::obstacle::{Crush, Hold};
        self.collision_of(o)?;
        let crush = match crush {
            ObstacleCrush::Breaks => Crush::Breaks,
            ObstacleCrush::Destroys => Crush::Destroys,
            ObstacleCrush::SparesBodies => Crush::SparesBodies,
        };
        let hold = match hold {
            ObstacleHold::AfterAppearing => Hold::AfterAppearing,
            ObstacleHold::Always => Hold::Always,
        };
        Ok(kinds::obstacle::react(self, o, crush, hold))
    }

    fn obstacle_action(&mut self, o: ObjectRef, a: ObstacleAction) -> ApiResult<()> {
        use kinds::obstacle::SharedAction as S;
        self.collision_of(o)?;
        let a = match a {
            ObstacleAction::ReturnToIdle => S::ReturnToIdle,
            ObstacleAction::Slide => S::Slide,
            ObstacleAction::KnockedBack => S::KnockedBack,
            ObstacleAction::Flinch => S::Flinch,
            ObstacleAction::Paralyzed => S::Paralyzed,
            ObstacleAction::Frozen => S::Frozen,
            ObstacleAction::Bubbled => S::Bubbled,
        };
        kinds::obstacle::shared_action(self, o, a).map_err(ApiError::Other)
    }

    fn obstacle_removal(&self, o: ObjectRef) -> ApiResult<ObstacleRemoval> {
        use kinds::obstacle::Removal;
        self.collision_of(o)?;
        Ok(match kinds::obstacle::removal(self, o) {
            Removal::Broken => ObstacleRemoval::Broken,
            Removal::Removed => ObstacleRemoval::Removed,
            Removal::Vanished => ObstacleRemoval::Vanished,
            Removal::Absorbed { .. } => ObstacleRemoval::Absorbed,
        })
    }

    fn obstacle_blink_out(&mut self, o: ObjectRef) -> ApiResult<BlinkOut> {
        use kinds::obstacle::BlinkOut as B;
        self.collision_of(o)?;
        Ok(match kinds::obstacle::blink_out(self, o) {
            B::No => BlinkOut::No,
            B::Blinking => BlinkOut::Blinking,
            B::Done => BlinkOut::Done,
        })
    }

    fn obstacle_fly_to_absorber(&mut self, o: ObjectRef, kind: u8) -> ApiResult<()> {
        self.collision_of(o)?;
        kinds::obstacle::fly_to_absorber(self, o, kind);
        Ok(())
    }

    fn obstacle_release_tracking(&mut self, o: ObjectRef) {
        kinds::obstacle::release_tracking(self, o);
    }

    fn obstacle_request(&mut self, o: ObjectRef, request: ObstacleRequest, by: ObjectRef) {
        match request {
            ObstacleRequest::Remove => kinds::obstacle::remove(self, o),
            ObstacleRequest::Vanish => kinds::obstacle::vanish(self, o),
            ObstacleRequest::Absorb => kinds::obstacle::absorb(self, o, by),
        }
    }

    fn obstacle_absorb_all(&mut self, absorber: ObjectRef) {
        kinds::obstacle::absorb_all(self, absorber);
    }

    fn obstacle_swallowable(&self, o: ObjectRef) -> bool {
        let ob = self.objects.get(o);
        // The NameID word's high half: an actor's next chip (0xFFFF for
        // none), nothing else's (0).
        let high = if ob.actor.is_some() { self.chip_number(ob.chip).map_or(0xFFFF, u32::from) } else { 0 };
        let word = ob.name_id as u32 | high << 16;
        (0xCD..=0xFF).contains(&word) && !matches!(word, 0xD3 | 0xDA | 0xE9 | 0xEA)
    }

    fn obstacle_present(&self, o: ObjectRef) -> bool {
        use kinds::obstacle::f2;
        self.objects
            .get(o)
            .collision
            .is_some_and(|c| self.collision.get(c).f2 & (f2::ABSORBED | f2::VANISH | f2::REMOVED) == 0)
    }

    // ---- Field objects (obstacles) -------------------------------------------

    fn obstacle_flag(&self, o: ObjectRef, flag: ObstacleFlag) -> ApiResult<bool> {
        use kinds::obstacle::f2;
        let mask = match flag {
            ObstacleFlag::Destroy => f2::DESTROY,
            ObstacleFlag::Flinch => f2::FLINCH,
            ObstacleFlag::Pushed => f2::PUSHED,
            ObstacleFlag::Thrown => f2::THROWN,
            ObstacleFlag::Encased => f2::ENCASED,
            ObstacleFlag::Removed => f2::REMOVED,
            ObstacleFlag::Vanish => f2::VANISH,
            ObstacleFlag::Absorbed => f2::ABSORBED,
            ObstacleFlag::AbsorbedBy0 => f2::ABSORBED_BY_0,
            ObstacleFlag::AbsorbedBy1 => f2::ABSORBED_BY_1,
        };
        Ok(self.collision_of(o)?.f2 & mask != 0)
    }

    fn name_attach_point(&self, name_id: u16, point: u8, alliance: u8, flip: u8) -> (i32, i32) {
        kinds::player::name_attach_point(self, name_id, point as usize, alliance, flip)
    }
}
