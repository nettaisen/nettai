//! The engine's side of the content API: [`CoreApi`] over a `Battle`.
//! Each call is the engine's own code (the ruleset's services, the navi
//! framework, the core's objects and panels); content never sees the
//! engine's bit values or offsets.

use bn6_content_api::api::ApiResult;
use bn6_content_api::{
    ActorField, ApiError, BattleInfo, CollisionField, ColumnInfo, ContentState, CoreApi, DimmingStep, Emotion,
    FieldType, FieldValue, HitboxSpec, Key, Lifecycle, LinkedChip, NaviRecordInfo, NaviStat, NaviState, ObjectField,
    Pad, PanelInfo, RequestFlag, Shadow, SpriteField, SpriteId, StatusFlag, StatusTimer, Value,
};

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
        _ => return None,
    })
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
}

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

    fn battle_info(&self, f: BattleInfo) -> Value {
        match f {
            BattleInfo::Link => Value::Bool(self.setup.settings.effects & crate::setup::effects::LINK != 0),
            BattleInfo::Mode => Value::Int(self.round.mode_copy as i64),
            BattleInfo::PanelPattern => Value::Int(self.setup.settings.panel_pattern as i64),
            BattleInfo::NavisIn => Value::Bool(self.round.intro_bits & 0x02 != 0),
            BattleInfo::LocalSide => Value::Int(self.round.local_side as i64),
        }
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
            NaviStat::Form => i(s.form.0 as i64),
            NaviStat::Navi => i(s.navi.0 as i64),
            NaviStat::NaviVariant => i(s.navi_variant as i64),
            NaviStat::Element => i(s.element as i64),
            NaviStat::Attack => i(s.attack as i64),
            NaviStat::Rapid => i(s.rapid as i64),
            NaviStat::Charge => i(s.charge as i64),
            NaviStat::Mood => i(s.mood as i64),
            NaviStat::BeastOutCounter => i(s.beast_out_counter as i64),
            NaviStat::MaxBaseHp => i(s.max_base_hp as i64),
            NaviStat::ChipRecovery => i(s.chip_recovery as i64),
            NaviStat::BusterShot => i(s.weapons.buster_shot as i64),
            NaviStat::ChargeShotKind => i(s.weapons.charge_shot_kind as i64),
            NaviStat::BusterBlanks => i(s.bugs.buster_blanks as i64),
            NaviStat::BusterCharged => i(s.bugs.buster_charged as i64),
            NaviStat::HpDrain => i(s.bugs.hp_drain as i64),
            NaviStat::CustomDrain => i(s.bugs.custom_drain as i64),
            NaviStat::PanelTrail => i(s.bugs.panel_trail_kind as i64),
            NaviStat::Beast => Value::Bool(s.form.is_beast()),
            NaviStat::BeastOver => Value::Bool(s.form.is_beast_over()),
        }
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
        self.hands[side as usize & 1].ids.get(i as usize).copied().unwrap_or(crate::hand::NO_CHIP)
    }

    fn hand_cursor(&self, side: u8) -> u8 {
        self.hands[side as usize & 1].cursor
    }

    fn advance_hand(&mut self, side: u8) {
        self.hands[side as usize & 1].advance();
    }

    fn linked(&self, side: u8) -> LinkedChip {
        let r = self.linked[side as usize & 1];
        LinkedChip { chip: r.chip, bonus: r.bonus, damage: r.damage, owner: r.owner, object: r.object }
    }

    fn set_linked(&mut self, side: u8, rec: LinkedChip) {
        self.linked[side as usize & 1] =
            LinkedRecord { chip: rec.chip, bonus: rec.bonus, damage: rec.damage, owner: rec.owner, object: rec.object };
    }

    fn clear_linked(&mut self, side: u8) {
        Battle::clear_linked(self, side & 1);
    }

    fn fill_custom_gauge(&mut self) {
        self.gauge.value = crate::hud::CustomGauge::FULL;
    }

    fn bump_side_stat(&mut self, side: u8, index: u8, n: u8) {
        Battle::bump_side_stat(self, side & 1, index as usize & 0xF, n);
    }

    fn navi_record(&self, name_id: u16) -> Option<NaviRecordInfo> {
        let name = self
            .content
            .navis
            .iter()
            .filter_map(|n| n.name_record.as_ref())
            .chain(self.content.forms.iter().filter_map(|f| f.name_record.as_ref()))
            .find(|n| n.id == name_id)?;
        let r = name.record();
        Some(NaviRecordInfo { actor_type: actor_type_index(r.actor_type) as u8, ai_index: r.ai_index })
    }

    // ---- Panels -----------------------------------------------------------

    fn panel_valid(&self, p: PanelPos) -> bool {
        crate::field::is_valid(p.x, p.y)
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

    // ---- Objects -----------------------------------------------------------

    fn spawn(&mut self, pool: Pool, index: u8, pos: Vec3, params: [u8; 4]) -> Option<ObjectRef> {
        super::spawn_object(self, pool, index, pos, params)
    }

    fn spawn_kind(&mut self, name: &str, pos: Vec3, params: [u8; 4]) -> ApiResult<Option<ObjectRef>> {
        let k = self.content.object_kind(name).ok_or_else(|| ApiError::UnknownKind(name.to_string()))?;
        let (pool, index) = (k.pool, k.index);
        Ok(super::spawn_object(self, pool, index, pos, params))
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
            _ => Lifecycle::Destroy,
        }
    }

    fn set_lifecycle(&mut self, o: ObjectRef, l: Lifecycle) {
        let p = match l {
            Lifecycle::Init => Progress::default(),
            Lifecycle::Update => Progress::UPDATE,
            Lifecycle::Destroy => Progress::DESTROY,
        };
        common::set_progress(self, o, p);
    }

    fn set_lifecycle_only(&mut self, o: ObjectRef, l: Lifecycle) {
        self.objects.get_mut(o).state = match l {
            Lifecycle::Init => state::INIT,
            Lifecycle::Update => state::UPDATE,
            Lifecycle::Destroy => state::DESTROY,
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
            ObjectField::Index => i(ob.index as i64),
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
            ObjectField::Active
            | ObjectField::Visible
            | ObjectField::RunWhilePaused
            | ObjectField::RunWhileDimmed
            | ObjectField::NoSpriteUpdate => {
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

    fn spawn_palette_flash(&mut self, variant: u8, ticks: u8, while_dimmed: bool, while_paused: bool) -> Option<ObjectRef> {
        kinds::palette_flash::spawn_variant(self, variant, ticks, while_dimmed, while_paused)
    }

    fn death_hook(&mut self, o: ObjectRef, name_id: u16) {
        kinds::player::form::navi_death_hook(self, o, name_id);
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
            ActorField::Chip => i(at.chip_id as i64),
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
            ActorField::BusterRoutine => i(a.buster as i64),
            ActorField::ChargeShotRoutine => i(a.charge_shot as i64),
            ActorField::BackSpecialRoutine => i(a.back_special as i64),
            ActorField::AChargeRoutine => i(a.a_charge as i64),
            ActorField::AltAChargeRoutine => i(a.alt_a_charge as i64),
            ActorField::Mode9ARoutine => i(a.mode9_a as i64),
        })
    }

    fn actor_set(&mut self, o: ObjectRef, f: ActorField, v: Value) -> ApiResult<()> {
        let v = store(f.name(), f.writable(), f.ty(), v)?;
        let a = self.actor_of_mut(o)?;
        let at = &mut a.attack;
        match (f, v) {
            (ActorField::Overlay, FieldValue::Object(r)) => a.overlay = r,
            (ActorField::Step, FieldValue::U8(x)) => at.step = x,
            (ActorField::StepInit, FieldValue::U8(x)) => at.step_init = x,
            (ActorField::Variant, FieldValue::U8(x)) => at.variant = x,
            (ActorField::Chip, FieldValue::U16(x)) => at.chip_id = x,
            (ActorField::AttackElement, FieldValue::U8(x)) => at.element = x,
            (ActorField::AttackDamage, FieldValue::U16(x)) => at.damage = x,
            (ActorField::HitParam, FieldValue::U16(x)) => at.hit_param = x,
            (ActorField::Charged, FieldValue::U8(x)) => at.charged = x,
            (ActorField::AttackLockout, FieldValue::U8(x)) => at.lockout = x,
            (ActorField::Extra, FieldValue::U16(x)) => at.extra = x,
            (ActorField::SpecialSource, FieldValue::U8(x)) => at.special_source = x,
            (ActorField::BeastLockon, FieldValue::U8(x)) => at.beast_lockon = x,
            (ActorField::Marker, FieldValue::U32(x)) => at.marker = x,
            (ActorField::LockonMarker, FieldValue::Object(r)) => a.lockon_marker = r,
            (ActorField::ChargeGlow, FieldValue::Object(r)) => a.charge_glow = r,
            (ActorField::FullSynchroAura, FieldValue::Object(r)) => a.full_synchro_aura = r,
            (ActorField::BufferedMove, FieldValue::U8(x)) => a.buffered_move = x,
            (ActorField::ChipLockout, FieldValue::U8(x)) => a.lockout = x,
            (ActorField::BackSpecialCooldown, FieldValue::U8(x)) => a.back_special_cooldown = x,
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
        let content = self.behaviors.clone();
        let action = self.objects.get(o).action;
        let (m, kind) = content.manifest().zip(content.action(action)).ok_or(ApiError::NoState(o))?;
        let id = m.action_state(kind);
        let a = self.actor_of_mut(o)?;
        // The game keeps an action's variables in the shared attack state,
        // where they outlive the action; a different action starts from
        // zero.
        if !matches!(&a.attack.action, ActionVars::Content(s) if s.id() == id) {
            a.attack.action = ActionVars::Content(ContentState::new(id));
        }
        match &mut a.attack.action {
            ActionVars::Content(s) => Ok(s),
            _ => unreachable!(),
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

    fn exit_attack(&mut self, o: ObjectRef) {
        kinds::player::exit_attack_state(self, o);
    }

    fn end_attack(&mut self, o: ObjectRef) {
        kinds::player::end_attack(self, o);
    }

    fn set_attack(&mut self, o: ObjectRef, action: u8, kind: u8) {
        kinds::player::set_attack(self, o, action, kind);
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

    fn can_move(&self, o: ObjectRef) -> bool {
        self.collision_of(o).is_ok_and(|c| c.f1 & (f1::IMMOBILIZED | f1::SLIDING | f1::MOVING) == 0)
    }

    fn buster_damage(&self, o: ObjectRef) -> u16 {
        kinds::player::idle::buster_damage(self, o)
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
            CollisionField::HitFlags => c.acc.hit_flags as i64,
            CollisionField::FinalDamage => c.acc.final_damage as i64,
        }))
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
            CollisionField::HitFlags | CollisionField::FinalDamage => unreachable!("read-only"),
        }
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

    // ---- Services ------------------------------------------------------------

    fn dimming(&mut self, o: ObjectRef, step: DimmingStep, chip: u16) {
        use crate::dimming as d;
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

    fn hide_user(&mut self, user: ObjectRef) {
        crate::dimming::hide_user(self, user);
    }

    fn show_user(&mut self, user: ObjectRef) {
        crate::dimming::show_user(self, user);
    }

    fn navi_chip_left(&mut self, controller: ObjectRef) {
        kinds::navi_chip::navi_left(self, controller);
    }
}
