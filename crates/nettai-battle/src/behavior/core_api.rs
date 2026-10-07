//! The engine's side of the content API: [`CoreApi`] over a `Battle`.
//! Each call is the engine's own code (the rules' services, the navi
//! framework, the core's objects and panels); content never sees the
//! engine's bit values or offsets.

use nettai_content_api::api::ApiResult;
use nettai_content_api::api::ObstacleFlag;
use nettai_content_api::{
    ActorField, ApiError, BattleInfo, BlinkOut, CollisionField, ColumnInfo, CoreApi, DimmingStep,
    FieldType, FieldValue, HitboxSpec, HudPart, Key, Lifecycle, LinkedChip, NaviStat, NaviState,
    ObjectField, ObstacleAction, SideSpecial, ObstacleCrush, ObstacleRemoval, ObstacleRequest, Pad, PanelInfo, RequestFlag,
    ScreenFade, Shadow,
    SpriteField, SpriteId, StatusFlag, StatusTimer, Value,
};
use nettai_content_api::{
    ActionHandle, ChipHandle, CollisionHandle, EffectHandle, KindHandle, NaviAction, RecordHandle, RegionHandle, Registry,
    SparkHandle, SpawnAt, StateId, WeaponHandle,
};

/// The record type of an absorbed obstacle's look (objects/absorbed_obstacle).
const ABSORBED_LOOK: &str = "absorbed-look";
// Subtypes 8, 17, 18 (Wind, Anubis, Otenko) and the obstacle framework.
use nettai_content_api::{ObstacleHold, ObstaclePush, WindSource};

use crate::actor::{AbsorbedObstacle, ActorData, ActorType, request, status};
use crate::battle::{Battle, LinkedRecord};
use crate::collision::{CollisionData, f1, timer};
use crate::field::PanelType;
use crate::input::keys;
use crate::kinds::common::{self, Progress};
use crate::kinds::player::actions::ActionVars;
use crate::kinds;
use crate::object::sprite;
use crate::object::{ObjectRef, PanelPos, Vec3, flags, state};
use crate::sound::SoundId;

/// The collision `f1` bit behind a status flag.
/// The object kind named `key`.
/// What reading or writing a navi's action byte is: a navi runs a
/// [`kinds::player::NaviAction`] (`navi_action`, `set_attack`).
fn navi_action_byte() -> ApiError {
    ApiError::Other("a navi's action is no number: read it with navi_action(), start one with set_attack()".into())
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
        StatusFlag::Carried => kinds::obstacle::obstacle_f1::CARRIED,
        StatusFlag::Untouchable => f1::UNTOUCHABLE,
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
        // (Not action requests: `request` and `set_request` read flag2,
        // or the navi's own requests.)
        RequestFlag::Slide | RequestFlag::Anger | RequestFlag::Drag | RequestFlag::TakeOff | RequestFlag::Land => 0,
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
        RequestFlag::NaviSwitch => request::NAVI_SWITCH,
        RequestFlag::SwitchKnockout => request::SWITCH_KNOCKOUT,
        RequestFlag::Mode9A => request::MODE9_A,
        RequestFlag::Takeover => request::TAKEOVER,
        RequestFlag::Volley => request::VOLLEY,
        RequestFlag::WeaknessHit => request::WEAKNESS_HIT,
    }
}

/// The collision's flag2 bits of the slide, drag and anger requests
/// (`RequestFlag::Slide`, `RequestFlag::Drag`, `RequestFlag::Anger`).
const SLIDE_REQUEST: u32 = 0x10;
const DRAG_REQUEST: u32 = 0x100;
const ANGER_REQUEST: u32 = 0x200;

/// The flag2 bit a request that isn't an action request reads.
fn flag2_request(f: RequestFlag) -> Option<u32> {
    match f {
        RequestFlag::Slide => Some(SLIDE_REQUEST),
        RequestFlag::Drag => Some(DRAG_REQUEST),
        RequestFlag::Anger => Some(ANGER_REQUEST),
        _ => None,
    }
}

/// The bit of a navi's own requests a request that is one reads.
fn own_request(f: RequestFlag) -> Option<u8> {
    match f {
        RequestFlag::TakeOff => Some(crate::actor::own_request::TAKE_OFF),
        RequestFlag::Land => Some(crate::actor::own_request::LAND),
        _ => None,
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
        NaviState::SwitchingNavi => status::SWITCHING_NAVI,
        NaviState::SwitchKnockout => status::SWITCH_KNOCKOUT,
        NaviState::Switched => status::SWITCHED,
        NaviState::Volley => status::VOLLEY,
        NaviState::Uninterruptible => status::UNINTERRUPTIBLE,
        NaviState::FormBreaking => status::FORM_BREAKING,
        NaviState::FormChangeSpriteHeld => status::FORM_CHANGE_SPRITE_HELD,
        NaviState::HeatTrap => status::HEAT_TRAP,
        NaviState::Vanished => status::VANISHED,
        NaviState::Dives => status::DIVES,
        NaviState::Hovering => status::HOVERING,
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

    /// An absorbed obstacle's look: record `h`, which must be an
    /// absorbed-look record (objects/absorbed_obstacle's `look`).
    fn absorbed_look(&self, h: u16) -> ApiResult<RecordHandle> {
        match self.content.defs.records.get(h as usize) {
            Some(r) if r.record_type == ABSORBED_LOOK => Ok(RecordHandle(h)),
            Some(r) => Err(ApiError::Other(format!("record {} is a {}, not an {ABSORBED_LOOK}", r.key, r.record_type))),
            None => Err(ApiError::Other(format!("no record has handle {h}"))),
        }
    }

    fn collision_of(&self, o: ObjectRef) -> ApiResult<&CollisionData> {
        let c = self.objects.get(o).collision.ok_or(ApiError::NoCollision(o))?;
        Ok(self.collision.get(c))
    }

    fn collision_of_mut(&mut self, o: ObjectRef) -> ApiResult<&mut CollisionData> {
        let c = self.objects.get(o).collision.ok_or(ApiError::NoCollision(o))?;
        Ok(self.collision.get_mut(c))
    }

    /// A chip the API was given (a definition's handle), checked.
    fn chip_from_api(&self, what: &str, chip: Option<ChipHandle>) -> ApiResult<Option<ChipHandle>> {
        match chip {
            Some(h) if h.index() >= self.content.defs.chips.len() => {
                Err(ApiError::Other(format!("{what}: no chip has handle {}", h.0)))
            }
            chip => Ok(chip),
        }
    }

    /// A weapon the API was given (a definition's handle), checked.
    fn weapon_from_api(&self, what: &str, v: FieldValue) -> ApiResult<Option<WeaponHandle>> {
        match v {
            FieldValue::Ref(None) => Ok(None),
            FieldValue::Ref(Some((Registry::Weapon, h))) if (h as usize) < self.content.defs.weapons.len() => {
                Ok(Some(WeaponHandle(h)))
            }
            FieldValue::Ref(Some((Registry::Weapon, h))) => Err(ApiError::Other(format!("{what}: no weapon has handle {h}"))),
            FieldValue::Ref(Some((other, _))) => Err(ApiError::Other(format!("{what}: a {other} is not a weapon"))),
            other => unreachable!("{what} stored as {other:?}"),
        }
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

impl Battle {
    /// Side `side`'s custom screen, which a custom hook reaches (§4.4).
    /// The slot of the rules' button `button` on side `side`'s screen (its
    /// first cell), for the call `what`.
    fn own_button_slot(&self, side: u8, button: &str, what: &str) -> ApiResult<u8> {
        use crate::custom::screen::SlotKind;
        let h = self.own_button(button)?;
        let screen = self.custom_screen(side)?;
        (0..crate::custom::screen::SLOTS as u8)
            .find(|&s| matches!(screen.slots[s as usize].kind, SlotKind::Button { button, cell } if button == h && cell != crate::custom::ButtonCell::Right))
            .ok_or_else(|| ApiError::Other(format!("{what}: {button:?} isn't on the screen")))
    }

    fn custom_screen(&self, side: u8) -> ApiResult<&crate::custom::screen::Screen> {
        self.custom.sides[side as usize & 1].screen.as_ref().ok_or_else(|| ApiError::Other("no custom screen is open".into()))
    }

    /// The rules' button and window named `name`.
    fn own_button(&self, name: &str) -> ApiResult<crate::content::ButtonHandle> {
        let rules = self.content.defs.rules().ok_or_else(|| ApiError::Other("the content has no rules".into()))?;
        rules.buttons.iter().copied().find(|&b| self.content.defs.button(b).name == name)
            .ok_or_else(|| ApiError::Other(format!("the rules have no button named {name:?}")))
    }

    fn own_window(&self, name: &str) -> ApiResult<crate::content::WindowHandle> {
        let rules = self.content.defs.rules().ok_or_else(|| ApiError::Other("the content has no rules".into()))?;
        rules.windows.iter().copied().find(|&w| self.content.defs.window(w).name == name)
            .ok_or_else(|| ApiError::Other(format!("the rules have no window named {name:?}")))
    }

    /// Which of the rules' buttons or windows `name` is, as the screen keeps
    /// whose pick holds its form: a button's place, or 0x100 past a
    /// window's.
    fn screen_owner(&self, name: &str) -> ApiResult<u16> {
        if let Ok(b) = self.own_button(name) {
            return Ok(b.0);
        }
        self.own_window(name).map(|w| 0x100 + w.0).map_err(|_| ApiError::Other(format!("the rules have no button or window named {name:?}")))
    }

    fn custom_screen_mut(&mut self, side: u8) -> ApiResult<&mut crate::custom::screen::Screen> {
        self.custom.sides[side as usize & 1].screen.as_mut().ok_or_else(|| ApiError::Other("no custom screen is open".into()))
    }
}

impl CoreApi for Battle {
    // ---- The battle ------------------------------------------------------

    fn set_dimmed(&mut self, on: bool) {
        if on {
            self.set_flags(crate::battle::battle_flags::DIMMED);
        } else {
            self.clear_flags(crate::battle::battle_flags::DIMMED);
        }
    }

    fn spawn_navi(
        &mut self,
        identity: nettai_content_api::IdentityHandle,
        panel: PanelPos,
        side: u8,
        summoner: Option<ObjectRef>,
    ) -> ApiResult<Option<ObjectRef>> {
        kinds::player::spawn_ai_navi(self, identity, panel, side & 1, summoner).map_err(ApiError::Other)
    }

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

    fn next_chip_damages(&self, user: ObjectRef) -> bool {
        use crate::content::ChipFlags;
        let o = self.objects.get(user);
        let flags = if self.content.navi_record(o.identity).actor_type == crate::actor::ActorType::Player {
            let hand = &self.hands[o.alliance as usize & 1];
            match hand.ids.get(hand.cursor as usize).copied().flatten() {
                Some(h) => self.content.chip(h).flags,
                None => self.game_rules().empty_hand.flags,
            }
        } else {
            // Another object's chip word: zeroed, the zeroed chip.
            match o.chip.or_else(|| self.zeroed_chip()) {
                Some(h) => self.content.chip(h).flags,
                None => ChipFlags(0),
            }
        };
        flags.0 & ChipFlags::HAS_DAMAGE != 0
    }

    fn battle_info(&self, f: BattleInfo) -> Value {
        match f {
            BattleInfo::Link => Value::Bool(self.setup.settings.effects & crate::setup::effects::LINK != 0),
            BattleInfo::BossRank => Value::Bool(self.setup.settings.effects & crate::setup::effects::BOSS_RANK != 0),
            BattleInfo::Effects => Value::Int(self.setup.settings.effects as i64),
            BattleInfo::Mode => Value::Int(self.round.mode_copy as i64),
            BattleInfo::PanelPattern => Value::Int(self.content.stage(self.setup.settings.stage).panel_pattern as i64),
            BattleInfo::NavisIn => Value::Bool(self.round.intro_bits & 0x02 != 0),
            BattleInfo::LocalSide => Value::Int(self.round.local_side as i64),
            BattleInfo::Turn => Value::Int(self.round.turn as i64),
            BattleInfo::OwnGauges => {
                Value::Bool(self.round.flags & crate::battle::battle_flags::OWN_GAUGES != 0)
            }
            BattleInfo::Fighting => Value::Bool(self.round.flags & crate::battle::battle_flags::FIGHTING != 0),
            BattleInfo::GaugeFull => Value::Bool(self.round.flags & crate::battle::battle_flags::GAUGE_FULL != 0),
            BattleInfo::LateTurns => Value::Bool(self.late_turns()),
            BattleInfo::ScreenFading => Value::Bool(self.fade.active()),
        }
    }

    fn shake_camera(&mut self, magnitude: u16, ticks: u16) {
        Battle::shake_camera(self, magnitude, ticks);
    }

    fn shake_camera_secondary(&mut self, magnitude: u16, ticks: u16) {
        Battle::shake_camera_secondary(self, magnitude, ticks);
    }

    fn set_shake_through_pause(&mut self, on: bool) {
        use crate::battle::battle_flags::SHAKE_THROUGH_PAUSE;
        if on {
            self.set_flags(SHAKE_THROUGH_PAUSE);
        } else {
            self.round.flags &= !SHAKE_THROUGH_PAUSE;
        }
    }

    fn spawn_burst(&mut self, navi: ObjectRef) -> Option<ObjectRef> {
        kinds::burst::spawn(self, navi)
    }

    fn show_hud(&mut self, part: HudPart, shown: bool) {
        // (Each console's own navi's: the call runs on every console,
        // whichever navi makes it.)
        if part == HudPart::ChipIcons {
            for hud in &mut self.chip_hud {
                hud.icons = shown;
            }
            return;
        }
        let h = &mut self.hud_hidden;
        let hidden = match part {
            HudPart::Gauge => &mut h.gauge,
            HudPart::EmotionWindow => &mut h.emotion_window,
            HudPart::LevelGauge => &mut h.level_gauge,
            HudPart::HpBox => &mut h.hp_box,
            HudPart::ChipIcons => unreachable!("the chips' icons are each console's"),
        };
        *hidden = !shown;
    }

    fn screen_fade(&mut self, fade: ScreenFade, speed: u8) {
        let mode = match fade {
            ScreenFade::TransformOut => crate::battle::FadeMode::TransformOut,
            ScreenFade::TransformIn => crate::battle::FadeMode::TransformIn,
        };
        self.fade.start(mode, speed);
    }

    fn play_sound(&mut self, sound: u16) {
        Battle::play_sound(self, SoundId(sound));
    }

    fn play_sound_for(&mut self, side: u8, sound: u16) {
        Battle::play_sound_for(self, side & 1, SoundId(sound));
    }

    fn warn(&mut self, sound: u16, at: Option<Vec3>, side: Option<u8>) {
        Battle::warn(self, SoundId(sound), at, side);
    }

    fn show_hp(&mut self, o: ObjectRef, dx: i8, dy: i8, damage: bool) {
        Battle::show_hp(self, o, dx, dy, damage, None);
    }

    fn hide_hp(&mut self, o: ObjectRef) {
        Battle::hide_hp(self, o);
    }

    fn navi_stat(&self, side: u8, stat: NaviStat) -> Value {
        let s = &self.stats[side as usize & 1];
        let i = |v: i64| Value::Int(v);
        let weapon = |w: Option<WeaponHandle>| w.map_or(Value::Nil, |h| Value::Def(Registry::Weapon, h.0));
        let record = |r: Option<nettai_content_api::RecordHandle>| r.map_or(Value::Nil, |h| Value::Def(Registry::Record, h.0));
        match stat {
            NaviStat::Form => Value::Def(Registry::Form, s.form.0),
            NaviStat::Navi => Value::Def(Registry::Navi, s.navi.0),
            NaviStat::NaviVariant => i(s.navi_variant as i64),
            NaviStat::Element => i(s.element as i64),
            NaviStat::Attack => i(s.attack as i64),
            NaviStat::Rapid => i(s.rapid as i64),
            NaviStat::Charge => i(s.charge as i64),
            NaviStat::Mood => i(s.mood as i64),
            NaviStat::StartingForm => Value::Def(Registry::Form, s.starting_form.0),
            NaviStat::BaseForm => Value::Def(Registry::Form, self.content.base_form_for(s.navi).0),
            NaviStat::AutoStep => i(s.bugs.auto_step as i64),
            NaviStat::StartingDamage => i(s.bugs.starting_damage as i64),
            NaviStat::Folder => i(s.folder as i64),
            NaviStat::Folder1Regular => i(s.folder_reg[0] as i64),
            NaviStat::Folder2Regular => i(s.folder_reg[1] as i64),
            NaviStat::Folder1TagA => i(s.folder_tags[0][0] as i64),
            NaviStat::Folder1TagB => i(s.folder_tags[0][1] as i64),
            NaviStat::Folder2TagA => i(s.folder_tags[1][0] as i64),
            NaviStat::Folder2TagB => i(s.folder_tags[1][1] as i64),
            NaviStat::AChargeWeapon => weapon(s.weapons.a_charge),
            NaviStat::Mode9AWeapon => weapon(s.weapons.mode9_a),
            NaviStat::RegularMemory => i(s.reg_up as i64),
            NaviStat::MaxBaseHp => i(s.max_base_hp as i64),
            NaviStat::ChipRecovery => i(s.chip_recovery as i64),
            NaviStat::BusterShot => record(s.weapons.buster_shot),
            NaviStat::ChargeShotKind => record(s.weapons.charge_shot_kind),
            NaviStat::BackSpecialDamage => i(s.weapons.back_special_damage as i64),
            NaviStat::BusterBlanks => i(s.bugs.buster_blanks as i64),
            NaviStat::BusterCharged => i(s.bugs.buster_charged as i64),
            NaviStat::HpDrain => i(s.bugs.hp_drain as i64),
            NaviStat::CustomDrain => i(s.bugs.custom_drain as i64),
            NaviStat::PanelTrail => i(s.bugs.panel_trail_kind as i64),
            NaviStat::CustomLevel => i(s.custom_level as i64),
            NaviStat::HandShrinkTurn => i(s.bugs.hand_shrink_turn as i64),
            NaviStat::ChargeShotWeapon => weapon(s.weapons.charge_shot),
            NaviStat::BackSpecialWeapon => weapon(s.weapons.back_special),
            NaviStat::FloatShoes => Value::Bool(s.float_shoes),
            NaviStat::AirShoes => Value::Bool(s.air_shoes),
            NaviStat::Undershirt => Value::Bool(s.undershirt),
            NaviStat::Hp => i(s.hp as i64),
            NaviStat::MaxHp => i(s.max_hp as i64),
            NaviStat::MegaLevel => i(s.mega_level as i64),
            NaviStat::GigaLevel => i(s.giga_level as i64),
            NaviStat::SuperArmor => Value::Bool(s.super_armor),
            NaviStat::StatusGuard => Value::Bool(s.bugs.status_immunity),
            NaviStat::BusterWeapon => weapon(s.weapons.buster),
            NaviStat::FirstBarrier => record(s.first_barrier),
            NaviStat::Gauge => i(s.gauge_speed as i64),
            NaviStat::Rush => Value::Bool(s.support.is_some_and(|n| n.rush)),
            NaviStat::Beat => Value::Bool(s.support.is_some_and(|n| n.beat)),
            NaviStat::Tango => Value::Bool(s.support.is_some_and(|n| n.tango)),
            NaviStat::SupportBug => Value::Bool(s.support.is_none()),
            NaviStat::StepBug => i(s.bugs.processing as i64),
            NaviStat::PanelTrailLevel => i(s.bugs.panel_trail_level as i64),
            NaviStat::HitStatus => i(s.bugs.hit_status as i64),
            NaviStat::CustomDamage => i(s.bugs.custom_damage as i64),
            NaviStat::EmotionBug => i(s.bugs.emotion as i64),
            NaviStat::BattleStartBug => i(s.bugs.battle_start as i64),
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
        let weapon = match stat {
            NaviStat::ChargeShotWeapon | NaviStat::BackSpecialWeapon | NaviStat::BusterWeapon | NaviStat::AChargeWeapon | NaviStat::Mode9AWeapon => {
                self.weapon_from_api(stat.name(), v)?
            }
            _ => None,
        };
        // A record field's value: a shot program or a barrier.
        let record = match v {
            FieldValue::Ref(Some((Registry::Record, h))) => Some(nettai_content_api::RecordHandle(h)),
            _ => None,
        };
        let s = &mut self.stats[side as usize & 1];
        // A support bit as the original sets it in the byte: none (0xFF,
        // the support bug) stays none when one is set.
        let support = |s: &mut crate::setup::NaviStats, f: fn(&mut crate::setup::Supports) -> &mut bool, on: bool| {
            match &mut s.support {
                Some(n) => *f(n) = on,
                None if on => {}
                None => {
                    let mut n = crate::setup::Supports { rush: true, beat: true, tango: true };
                    *f(&mut n) = false;
                    s.support = Some(n);
                }
            }
        };
        match (stat, v) {
            (NaviStat::NaviVariant, FieldValue::U8(x)) => s.navi_variant = x,
            (NaviStat::RegularMemory, FieldValue::U8(x)) => s.reg_up = x,
            (NaviStat::MaxBaseHp, FieldValue::U16(x)) => s.max_base_hp = x,
            (NaviStat::Element, FieldValue::U8(x)) => s.element = x,
            (NaviStat::Mood, FieldValue::U8(x)) => s.mood = x,
            (NaviStat::ChipRecovery, FieldValue::U16(x)) => s.chip_recovery = x,
            (NaviStat::BusterShot, FieldValue::Ref(_)) => s.weapons.buster_shot = record,
            (NaviStat::ChargeShotKind, FieldValue::Ref(_)) => s.weapons.charge_shot_kind = record,
            (NaviStat::BusterBlanks, FieldValue::U8(x)) => s.bugs.buster_blanks = x,
            (NaviStat::BusterCharged, FieldValue::U8(x)) => s.bugs.buster_charged = x,
            (NaviStat::HpDrain, FieldValue::U8(x)) => s.bugs.hp_drain = x,
            (NaviStat::CustomDrain, FieldValue::U8(x)) => s.bugs.custom_drain = x,
            (NaviStat::PanelTrail, FieldValue::U8(x)) => s.bugs.panel_trail_kind = x,
            (NaviStat::Hp, FieldValue::U16(x)) => s.hp = x,
            (NaviStat::MaxHp, FieldValue::U16(x)) => s.max_hp = x,
            (NaviStat::MegaLevel, FieldValue::U8(x)) => s.mega_level = x,
            (NaviStat::GigaLevel, FieldValue::U8(x)) => s.giga_level = x,
            (NaviStat::SuperArmor, FieldValue::Bool(x)) => s.super_armor = x,
            (NaviStat::StatusGuard, FieldValue::Bool(x)) => s.bugs.status_immunity = x,
            (NaviStat::BusterWeapon, FieldValue::Ref(_)) => s.weapons.buster = weapon,
            (NaviStat::FirstBarrier, FieldValue::Ref(_)) => s.first_barrier = record,
            (NaviStat::Gauge, FieldValue::Enum(x)) => {
                s.gauge_speed = match x {
                    0 => crate::setup::GaugeSpeed::Normal,
                    1 => crate::setup::GaugeSpeed::Fast,
                    _ => crate::setup::GaugeSpeed::Slow,
                }
            }
            (NaviStat::Rush, FieldValue::Bool(x)) => support(s, |n| &mut n.rush, x),
            (NaviStat::Beat, FieldValue::Bool(x)) => support(s, |n| &mut n.beat, x),
            (NaviStat::Tango, FieldValue::Bool(x)) => support(s, |n| &mut n.tango, x),
            (NaviStat::StepBug, FieldValue::U8(x)) => s.bugs.processing = x,
            (NaviStat::PanelTrailLevel, FieldValue::U8(x)) => s.bugs.panel_trail_level = x,
            (NaviStat::HitStatus, FieldValue::U8(x)) => s.bugs.hit_status = x,
            (NaviStat::CustomDamage, FieldValue::U16(x)) => s.bugs.custom_damage = x,
            (NaviStat::EmotionBug, FieldValue::U8(x)) => s.bugs.emotion = x,
            (NaviStat::BattleStartBug, FieldValue::U8(x)) => s.bugs.battle_start = x,
            (NaviStat::Attack, FieldValue::U8(x)) => s.attack = x,
            (NaviStat::Rapid, FieldValue::U8(x)) => s.rapid = x,
            (NaviStat::Charge, FieldValue::U8(x)) => s.charge = x,
            (NaviStat::CustomLevel, FieldValue::U8(x)) => s.custom_level = x,
            (NaviStat::Form, FieldValue::Ref(Some((Registry::Form, h)))) => s.form = nettai_content_api::FormHandle(h),
            (NaviStat::Form, v) => return Err(ApiError::Other(format!("form: {v:?} is not a form"))),
            (NaviStat::HandShrinkTurn, FieldValue::U8(x)) => s.bugs.hand_shrink_turn = x,
            (NaviStat::ChargeShotWeapon, FieldValue::Ref(_)) => s.weapons.charge_shot = weapon,
            (NaviStat::AChargeWeapon, FieldValue::Ref(_)) => s.weapons.a_charge = weapon,
            (NaviStat::Mode9AWeapon, FieldValue::Ref(_)) => s.weapons.mode9_a = weapon,
            (NaviStat::StartingForm, FieldValue::Ref(Some((Registry::Form, h)))) => s.starting_form = nettai_content_api::FormHandle(h),
            (NaviStat::StartingForm, v) => return Err(ApiError::Other(format!("starting_form: {v:?} is not a form"))),
            (NaviStat::BackSpecialDamage, FieldValue::U16(x)) => s.weapons.back_special_damage = x,
            (NaviStat::AutoStep, FieldValue::U8(x)) => s.bugs.auto_step = x,
            (NaviStat::StartingDamage, FieldValue::U8(x)) => s.bugs.starting_damage = x,
            (NaviStat::Folder, FieldValue::U8(x)) => s.folder = x,
            (NaviStat::Folder1Regular, FieldValue::U8(x)) => s.folder_reg[0] = x,
            (NaviStat::Folder2Regular, FieldValue::U8(x)) => s.folder_reg[1] = x,
            (NaviStat::Folder1TagA, FieldValue::U8(x)) => s.folder_tags[0][0] = x,
            (NaviStat::Folder1TagB, FieldValue::U8(x)) => s.folder_tags[0][1] = x,
            (NaviStat::Folder2TagA, FieldValue::U8(x)) => s.folder_tags[1][0] = x,
            (NaviStat::Folder2TagB, FieldValue::U8(x)) => s.folder_tags[1][1] = x,
            (NaviStat::BackSpecialWeapon, FieldValue::Ref(_)) => s.weapons.back_special = weapon,
            (NaviStat::FloatShoes, FieldValue::Bool(x)) => s.float_shoes = x,
            (NaviStat::AirShoes, FieldValue::Bool(x)) => s.air_shoes = x,
            (NaviStat::Undershirt, FieldValue::Bool(x)) => s.undershirt = x,
            // The support bug: none (the byte 0xFF); cleared, none set.
            (NaviStat::SupportBug, FieldValue::Bool(true)) => s.support = None,
            (NaviStat::SupportBug, FieldValue::Bool(false)) => {
                s.support.get_or_insert_with(crate::setup::Supports::default);
            }
            (f, v) => unreachable!("{f:?} stored as {v:?}"),
        }
        Ok(())
    }

    fn emotion(&self, side: u8) -> String {
        kinds::player::emotion_name(self, side & 1).to_string()
    }

    fn set_mood(&mut self, side: u8, mood: u8) {
        kinds::player::set_mood(self, side & 1, mood);
    }

    fn gain_mood(&mut self, side: u8, n: u16) {
        kinds::player::gain_mood(self, side & 1, n);
    }

    fn lose_mood(&mut self, side: u8, n: u16) {
        kinds::player::lose_mood(self, side & 1, n);
    }

    fn set_emotion_window_glitch(&mut self, side: u8, on: bool) {
        self.consoles[side as usize & 1].emotion_window_glitch = on;
    }

    fn setup_problem(&mut self, text: &str, field: Option<&str>, entry: Option<u32>) {
        if let Some(problems) = &mut self.validation {
            problems.push(crate::rules::Problem { text: text.to_string(), field: field.map(str::to_string), entry: entry.map(|e| e as usize) });
        }
    }

    fn def_name(&self, registry: nettai_content_api::Registry, handle: u16) -> String {
        use nettai_content_api::Registry;
        let defs = &self.content.defs;
        let strings = &self.content.strings;
        let (key, name) = match registry {
            Registry::Chip => {
                let key = &defs.chip(nettai_content_api::ChipHandle(handle)).key;
                (key, strings.chip(key).and_then(|s| s.name.clone()))
            }
            Registry::Navi => {
                let key = &defs.navi(nettai_content_api::NaviHandle(handle)).key;
                (key, strings.navi(key).and_then(|s| s.name.clone()))
            }
            Registry::Form => {
                let key = &defs.form(nettai_content_api::FormHandle(handle)).key;
                (key, strings.form(key).and_then(|s| s.name.clone()))
            }
            Registry::Entry => {
                let e = defs.entry(nettai_content_api::EntryHandle(handle));
                (&e.key, strings.entry(&e.collection, e.id()).and_then(|s| s.name.as_ref().map(|n| n.replace('\n', " "))))
            }
            _ => return format!("{registry} {handle}"),
        };
        name.unwrap_or_else(|| key.clone())
    }

    fn checked_folder(&self) -> Option<nettai_content_api::api::CheckedFolder> {
        self.folder_check.as_ref().map(|c| c.folder.clone())
    }

    fn folder_problem(&mut self, rule: &str, text: &str) {
        if let Some(c) = &mut self.folder_check {
            c.problems.push(crate::rules::FolderProblem { rule: rule.to_string(), text: text.to_string() });
        }
    }

    fn custom_folder(&self, side: u8) -> ApiResult<Vec<Option<ChipHandle>>> {
        let folder = self.custom.sides[side as usize & 1].folder;
        Ok(folder.chips.iter().map(|c| c.map(|c| c.id)).collect())
    }

    fn custom_swap_folder(&mut self, side: u8, a: u8, b: u8) -> ApiResult<()> {
        let folder = &mut self.custom.sides[side as usize & 1].folder;
        let n = folder.chips.len();
        if a as usize >= n || b as usize >= n {
            return Err(ApiError::Other(format!("custom.swap_folder: the folder has {n} places")));
        }
        folder.chips.swap(a as usize, b as usize);
        Ok(())
    }

    fn custom_offer(&mut self, side: u8, slot: u8, chip: ChipHandle, code: Option<u8>) -> ApiResult<()> {
        let screen = self.custom_screen_mut(side)?;
        let Some(place) = screen.offers.get_mut(slot as usize) else {
            return Err(ApiError::Other(format!("custom.offer: a slot is 0 to 11, not {slot}")));
        };
        let code = crate::content::ChipCode(code.unwrap_or(crate::custom::screen::INVALID_CODE.0));
        *place = Some(crate::custom::FolderChip { id: chip, code });
        Ok(())
    }

    fn custom_hand_size(&self, side: u8) -> ApiResult<u8> {
        Ok(self.custom_screen(side)?.hand_size)
    }

    fn custom_refuse(&mut self, side: u8) -> ApiResult<()> {
        self.custom_screen_mut(side)?.refuse();
        Ok(())
    }

    fn custom_sacrifice(&mut self, side: u8) -> ApiResult<()> {
        let s = self.custom_screen_mut(side)?;
        let slot = s.cursor_button_slot().ok_or_else(|| ApiError::Other("custom.sacrifice: no button under the cursor".into()))?;
        s.start_sacrifice(slot);
        Ok(())
    }

    fn custom_redeal(&mut self, side: u8) -> ApiResult<()> {
        let s = self.custom_screen_mut(side)?;
        let slot = s.cursor_button_slot().ok_or_else(|| ApiError::Other("custom.redeal: no button under the cursor".into()))?;
        s.start_redeal(slot);
        Ok(())
    }

    fn custom_last_pick_is_chip(&self, side: u8) -> ApiResult<bool> {
        Ok(self.custom_screen(side)?.last_pick_is_chip())
    }

    fn custom_cursor_state(&self, side: u8) -> ApiResult<&'static str> {
        let s = self.custom_screen(side)?;
        let slot = s.cursor_button_slot().ok_or_else(|| ApiError::Other("custom.cursor_state: no button under the cursor".into()))?;
        use crate::custom::screen::SlotState;
        Ok(match s.slots[slot as usize].state {
            SlotState::Selectable => "selectable",
            SlotState::Unavailable => "unavailable",
            SlotState::Selected => "selected",
        })
    }

    fn custom_pick(&mut self, side: u8) -> ApiResult<()> {
        self.custom_screen_mut(side)?.pick_cursor();
        Ok(())
    }

    fn custom_play_sound(&mut self, side: u8, sound: u16) -> ApiResult<()> {
        self.custom_screen_mut(side)?.look.play(crate::custom::look::ScreenSound::Rules(crate::SoundId(sound)));
        Ok(())
    }

    fn custom_play(&mut self, side: u8, sound: &str) -> ApiResult<()> {
        if !self.custom_screen_mut(side)?.play_named(sound) {
            return Err(ApiError::Other(format!("custom.play: no screen sound is named {sound:?}")));
        }
        Ok(())
    }

    fn custom_set_column_icon(&mut self, side: u8, chip: Option<ChipHandle>) -> ApiResult<()> {
        self.custom_screen_mut(side)?.set_column_icon(chip);
        Ok(())
    }

    fn custom_open_window(&mut self, side: u8, window: &str, ticks: u16) -> ApiResult<()> {
        let w = self.own_window(window)?;
        self.custom_screen_mut(side)?.open_window(w, ticks);
        Ok(())
    }

    fn custom_window_tick(&self, side: u8) -> ApiResult<u16> {
        self.custom_screen(side)?.window_tick().ok_or_else(|| ApiError::Other("custom.window_tick: no window is up".into()))
    }

    fn custom_shake(&mut self, side: u8, magnitude: u16, ticks: u16) -> ApiResult<()> {
        self.consoles[side as usize & 1].shake_secondary(magnitude, ticks);
        Ok(())
    }

    fn custom_frame(&self, side: u8) -> ApiResult<u32> {
        Ok(self.custom_screen(side)?.look.frame)
    }

    fn custom_set_frame(&mut self, side: u8, frame: u32) -> ApiResult<()> {
        self.custom_screen_mut(side)?.look.frame = frame;
        Ok(())
    }

    fn custom_spin(&mut self, side: u8) -> ApiResult<()> {
        self.custom_screen_mut(side)?.look.spin = 1;
        Ok(())
    }

    fn custom_fade(&mut self, side: u8, mode: &str, speed: u8) -> ApiResult<()> {
        use crate::battle::FadeMode;
        let mode = match mode {
            "half_out" => FadeMode::HalfOut,
            "half_out_back" => FadeMode::HalfOutBack,
            "end_to_white" => FadeMode::EndToWhite,
            "intro_from_white" => FadeMode::IntroFromWhite,
            "flash" => FadeMode::Flash,
            "flash_back" => FadeMode::FlashBack,
            m => return Err(ApiError::Other(format!("custom.fade: no screen fade is named {m:?}"))),
        };
        self.custom_screen_mut(side)?.look.fade.start(mode, speed);
        Ok(())
    }

    fn custom_set_face(&mut self, side: u8, form: Option<nettai_content_api::FormHandle>) -> ApiResult<()> {
        self.custom_screen_mut(side)?.look.face = form;
        Ok(())
    }

    fn custom_pick_first(&mut self, side: u8, icon: Option<ChipHandle>) -> ApiResult<()> {
        let i = side as usize & 1;
        let folder = self.custom.sides[i].folder;
        self.custom_screen_mut(side)?.pick_first(&folder, icon);
        Ok(())
    }

    fn custom_set_button_state(&mut self, side: u8, button: &str, state: &str) -> ApiResult<()> {
        use crate::custom::screen::{SlotKind, SlotState};
        let state = match state {
            "selectable" => SlotState::Selectable,
            "unavailable" => SlotState::Unavailable,
            "selected" => SlotState::Selected,
            s => return Err(ApiError::Other(format!("custom.set_button_state: no state is named {s:?}"))),
        };
        let h = self.own_button(button)?;
        let screen = self.custom_screen_mut(side)?;
        let slot = screen.slots.iter_mut().find(|s| matches!(s.kind, SlotKind::Button { button, cell } if button == h && cell != crate::custom::ButtonCell::Right));
        slot.ok_or_else(|| ApiError::Other(format!("custom.set_button_state: {button:?} isn't on the screen")))?.state = state;
        Ok(())
    }

    fn custom_update_availability(&mut self, side: u8) -> ApiResult<()> {
        self.with_custom_screen(side, |screen, view, folder, _, extras| screen.update_availability(view, folder, extras))
            .ok_or_else(|| ApiError::Other("no custom screen is open".into()))
    }

    fn custom_draw_emblem(&mut self, side: u8, x: u32) -> ApiResult<()> {
        self.custom_screen_mut(side)?.look.draw_emblem(x);
        Ok(())
    }

    fn custom_set_form(&mut self, side: u8, by: &str, form: Option<nettai_content_api::FormHandle>, turns: u8, alternate: bool) -> ApiResult<()> {
        let owner = self.screen_owner(by)?;
        let screen = self.custom_screen_mut(side)?;
        screen.form = form;
        screen.form_owner = form.map(|_| owner);
        screen.form_turns = if form.is_some() { turns } else { 0 };
        screen.form_alternate = form.is_some() && alternate;
        Ok(())
    }

    fn custom_form_taken(&self, side: u8, by: &str) -> ApiResult<bool> {
        let owner = self.screen_owner(by)?;
        let screen = self.custom_screen(side)?;
        Ok(screen.form.is_some() && screen.form_owner != Some(owner))
    }

    fn custom_full(&self, side: u8) -> ApiResult<bool> {
        Ok(self.custom_screen(side)?.selected as usize >= crate::custom::screen::MAX_SELECTIONS)
    }

    fn custom_button_picked(&self, side: u8, button: &str) -> ApiResult<bool> {
        use crate::custom::screen::SlotKind;
        let h = self.own_button(button)?;
        let screen = self.custom_screen(side)?;
        Ok(screen.selection().iter().any(|&s| matches!(screen.slots[s as usize].kind, SlotKind::Button { button, .. } if button == h)))
    }

    fn custom_last_pick(&mut self, side: u8) -> ApiResult<Option<nettai_content_api::api::CustomPick>> {
        let last = self
            .with_custom_screen(side, |screen, view, folder, _, _| screen.last_pick(folder, view))
            .ok_or_else(|| ApiError::Other("no custom screen is open".into()))?;
        Ok(last.map(|p| nettai_content_api::api::CustomPick {
            slot: p.slot,
            chip: p.chip.id,
            regular: p.regular,
            navi_chip: p.navi_chip,
            attached: p.attached,
        }))
    }

    fn custom_attach_to_last_pick(&mut self, side: u8, button: &str, modifiers: u8) -> ApiResult<bool> {
        let slot = self.own_button_slot(side, button, "custom.attach_to_last_pick")?;
        self.with_custom_screen(side, |screen, view, folder, _, _| screen.attach_to_last_pick(slot, modifiers, folder, view))
            .ok_or_else(|| ApiError::Other("no custom screen is open".into()))
    }

    fn custom_hold_last_pick(&mut self, side: u8, button: &str) -> ApiResult<bool> {
        let slot = self.own_button_slot(side, button, "custom.hold_last_pick")?;
        self.with_custom_screen(side, |screen, view, folder, _, _| screen.hold_last_pick(slot, folder, view))
            .ok_or_else(|| ApiError::Other("no custom screen is open".into()))
    }

    fn custom_held_pick(&mut self, side: u8, button: &str) -> ApiResult<Option<ChipHandle>> {
        let slot = self.own_button_slot(side, button, "custom.held_pick")?;
        self.with_custom_screen(side, |screen, view, folder, _, _| screen.held_pick(slot, folder, view).map(|c| c.id))
            .ok_or_else(|| ApiError::Other("no custom screen is open".into()))
    }

    fn custom_set_held_icon(&mut self, side: u8, button: &str, shown: bool) -> ApiResult<()> {
        let slot = self.own_button_slot(side, button, "custom.set_held_icon")?;
        let held = self
            .with_custom_screen(side, |screen, view, folder, _, _| screen.set_held_icon(slot, shown, folder, view))
            .ok_or_else(|| ApiError::Other("no custom screen is open".into()))?;
        if !held {
            return Err(ApiError::Other(format!("custom.set_held_icon: {button:?} holds no chip")));
        }
        Ok(())
    }

    fn custom_trade_last_pick(&mut self, side: u8, button: &str) -> ApiResult<bool> {
        let slot = self.own_button_slot(side, button, "custom.trade_last_pick")?;
        self.with_custom_screen(side, |screen, view, folder, _, _| screen.trade_last_pick(slot, folder, view))
            .ok_or_else(|| ApiError::Other("no custom screen is open".into()))
    }

    fn custom_fading(&self, side: u8) -> ApiResult<bool> {
        Ok(self.custom_screen(side)?.look.fade.active())
    }

    fn custom_cursor(&self, side: u8) -> ApiResult<u8> {
        Ok(self.custom_screen(side)?.cursor)
    }

    fn custom_set_cursor(&mut self, side: u8, slot: u8) -> ApiResult<()> {
        if slot as usize >= crate::custom::screen::SLOTS {
            return Err(ApiError::Other(format!("custom.set_cursor: no slot {slot}")));
        }
        self.custom_screen_mut(side)?.cursor = slot;
        Ok(())
    }

    fn custom_pressed(&self, side: u8, key: &str) -> ApiResult<bool> {
        let bit = crate::input::key_named(key).ok_or_else(|| ApiError::Other(format!("custom.pressed: no key is named {key:?}")))?;
        self.custom_screen(side)?;
        Ok(self.custom.sides[side as usize & 1].joypad.pressed & bit != 0)
    }

    fn custom_repeated(&self, side: u8, key: &str) -> ApiResult<bool> {
        let bit = crate::input::key_named(key).ok_or_else(|| ApiError::Other(format!("custom.repeated: no key is named {key:?}")))?;
        self.custom_screen(side)?;
        Ok(self.custom.sides[side as usize & 1].joypad.repeat & bit != 0)
    }

    fn custom_draw_window(&mut self, side: u8) -> ApiResult<()> {
        let folder = self.custom.sides[side as usize & 1].folder;
        self.custom_screen_mut(side)?.draw_window(&folder);
        Ok(())
    }

    fn custom_draw_regular(&mut self, side: u8) -> ApiResult<()> {
        let folder = self.custom.sides[side as usize & 1].folder;
        self.custom_screen_mut(side)?.look.draw_regular(folder.regular_pending);
        Ok(())
    }

    fn custom_draw_held(&mut self, side: u8) -> ApiResult<()> {
        let screen = self.custom_screen_mut(side)?;
        screen.look.draw_held(screen.hold.is_some());
        Ok(())
    }

    fn custom_draw_form_list_cursor(&mut self, side: u8) -> ApiResult<()> {
        self.custom_screen_mut(side)?.look.draw_form_list_cursor();
        Ok(())
    }

    fn custom_show_chip_window(&mut self, side: u8) -> ApiResult<()> {
        self.with_custom_screen(side, |screen, view, folder, _, _| screen.show_chip_window(folder, view))
            .ok_or_else(|| ApiError::Other("no custom screen is open".into()))
    }

    fn custom_set_form_list_tab(&mut self, side: u8, on: bool) -> ApiResult<()> {
        self.custom_screen_mut(side)?.look.form_list_tab = on;
        Ok(())
    }

    fn custom_describe(&mut self, side: u8, form: Option<nettai_content_api::FormHandle>) -> ApiResult<()> {
        let joy = self.custom.sides[side as usize & 1].joypad;
        let described = self
            .with_custom_screen(side, |screen, view, _, _, _| {
                // (Three lines for a form the content has no description of.)
                let lines = form.map_or(3, |f| view.library.form_description_lines(f));
                screen.describe_form(&joy, lines, form)
            })
            .ok_or_else(|| ApiError::Other("no custom screen is open".into()))?;
        if !described {
            return Err(ApiError::Other("custom.describe: no window is up".into()));
        }
        Ok(())
    }

    fn custom_refresh_buttons(&mut self, side: u8) -> ApiResult<()> {
        self.with_custom_screen(side, |screen, _, _, _, extras| screen.refresh_buttons(extras))
            .ok_or_else(|| ApiError::Other("no custom screen is open".into()))
    }

    fn custom_player(&self, side: u8) -> ApiResult<nettai_content_api::api::CustomPlayer> {
        let s = &self.custom.sides[side as usize & 1];
        Ok(nettai_content_api::api::CustomPlayer {
            emotion: self.game_rules().emotion.name(s.emotion).to_string(),
            random_battle: self.setup.settings.effects & crate::setup::effects::RANDOM != 0,
        })
    }


    fn take_over(&mut self, side: u8, ticks: u16) {
        let s = &mut self.sides[side as usize & 1];
        s.takeover = 1;
        s.takeover_ticks = ticks;
    }

    fn end_takeover(&mut self, side: u8) {
        self.sides[side as usize & 1].takeover = 0;
    }

    fn takeover_ticks(&self, side: u8) -> u16 {
        self.sides[side as usize & 1].takeover_ticks
    }

    fn side_special(&self, side: u8) -> SideSpecial {
        let s = &self.sides[side as usize & 1];
        if s.select_special != 0 {
            SideSpecial::Select
        } else if s.takeover != 0 {
            SideSpecial::Takeover
        } else {
            SideSpecial::None
        }
    }

    fn player(&self, side: u8) -> Option<ObjectRef> {
        Battle::player(self, side & 1)
    }

    fn alive_actor_slot(&self, side: u8, i: u8) -> Option<ObjectRef> {
        self.round.alive_actors[side as usize & 1].get(i as usize).copied().flatten()
    }

    fn alive_actors(&self, side: u8) -> Vec<ObjectRef> {
        self.round.alive_actors[side as usize & 1].iter().flatten().copied().collect()
    }

    fn tracked(&self, side: u8) -> Option<ObjectRef> {
        self.sides[side as usize & 1].tracked
    }

    fn objects_of_kind(&self, kind: u16) -> Vec<ObjectRef> {
        self.objects.in_order().filter(|&r| self.objects.get(r).kind.0 == kind).collect()
    }

    fn rng(&mut self) -> u32 {
        self.rng.next()
    }

    fn rng_positive(&mut self) -> u32 {
        self.rng.next_positive()
    }

    fn console_rng_positive(&mut self, side: u8) -> u32 {
        self.consoles[side as usize & 1].rng.next_positive()
    }

    fn jitter(&mut self, mask: u32, pos: Vec3) -> Vec3 {
        kinds::spark::jitter(self, mask, pos)
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

    fn set_hand_attack_bonus(&mut self, side: u8, i: u8, n: u16) {
        if let Some(b) = self.hands[side as usize & 1].attack_bonus.get_mut(i as usize) {
            *b = n;
        }
    }

    fn hand_left(&self, side: u8) -> u8 {
        let h = &self.hands[side as usize & 1];
        h.ids.iter().skip(h.cursor as usize).take_while(|c| c.is_some()).count() as u8
    }

    fn hand_chip_damages(&self, side: u8, i: u8) -> bool {
        let chip = self.hands[side as usize & 1].ids.get(i as usize).copied().flatten();
        chip.is_some_and(|h| self.content.chip(h).flags.0 & crate::content::ChipFlags::HAS_DAMAGE != 0)
    }

    fn hand_chip(&self, side: u8, i: u8) -> Option<ChipHandle> {
        self.hands[side as usize & 1].ids.get(i as usize).copied().flatten()
    }

    fn linked(&self, side: u8) -> LinkedChip {
        let r = self.linked[side as usize & 1];
        LinkedChip { chip: r.chip, bonus: r.bonus, damage: r.damage, owner: r.owner, object: r.object }
    }

    fn set_linked(&mut self, side: u8, rec: LinkedChip) {
        let chip = self.chip_from_api("a linked record's chip", rec.chip).unwrap_or_else(|e| panic!("{e}"));
        self.linked[side as usize & 1] =
            LinkedRecord { chip, bonus: rec.bonus, damage: rec.damage, owner: rec.owner, object: rec.object };
    }

    fn clear_linked(&mut self, side: u8) {
        Battle::clear_linked(self, side & 1);
    }

    fn fill_custom_gauge(&mut self) {
        self.gauge.value = crate::hud::CustomGauge::FULL;
    }

    fn drain_custom_gauge(&mut self, n: u16) {
        self.gauge.value = self.gauge.value.saturating_sub(n);
    }

    fn set_gauge_rate(&mut self, rate: u16) {
        self.gauge.rate = rate;
    }

    fn set_gauge_speed_ticks(&mut self, side: u8, slow: u16, fast: u16) {
        let s = &mut self.sides[side as usize & 1];
        s.slow_gauge_ticks = slow;
        s.fast_gauge_ticks = fast;
    }

    fn gauge_damage(&self, side: u8) -> u16 {
        kinds::gauge_damage(self, side)
    }

    fn sword_pick(&self, side: u8) -> u8 {
        self.sides[side as usize & 1].sword_pick
    }

    fn set_sword_pick(&mut self, side: u8, pick: u8) {
        self.sides[side as usize & 1].sword_pick = pick;
    }

    fn set_face_variant(&mut self, side: u8, variant: bool, charged: bool) {
        let looks = &mut self.looks[side as usize & 1];
        if charged {
            looks.face_variant_charged = variant;
        } else {
            looks.face_variant = variant;
        }
    }

    fn set_name_variant(&mut self, side: u8, variant: bool) {
        self.looks[side as usize & 1].name_variant = variant;
    }

    fn set_window_count(&mut self, side: u8, shown: bool) {
        self.looks[side as usize & 1].window_count = shown;
    }

    fn add_side_gauge(&mut self, side: u8, n: u16) {
        let s = &mut self.sides[side as usize & 1];
        s.gauge = (s.gauge as u32 + n as u32).min(crate::hud::CustomGauge::FULL as u32) as u16;
    }

    fn drain_side_gauge(&mut self, side: u8, n: u16) {
        let s = &mut self.sides[side as usize & 1];
        s.gauge = s.gauge.saturating_sub(n);
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

    fn set_side_stat(&mut self, side: u8, index: u8, n: u8) {
        self.side_stats[side as usize & 1][index as usize & 0xF] = n;
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

    fn damage_carry(&self, side: u8) -> nettai_content_api::api::DamageCarryInfo {
        let c = &self.damage_carry[side as usize & 1];
        nettai_content_api::api::DamageCarryInfo {
            this_tick: c.this_tick,
            previous: c.previous,
            source: c.source,
            target: c.target,
        }
    }

    fn set_damage_carry(&mut self, side: u8, rec: nettai_content_api::api::DamageCarryInfo) {
        self.damage_carry[side as usize & 1] = crate::battle::DamageCarry {
            this_tick: rec.this_tick,
            previous: rec.previous,
            source: rec.source,
            target: rec.target,
        };
    }

    // ---- Panels -----------------------------------------------------------

    fn panel_valid(&self, p: PanelPos) -> bool {
        crate::field::is_valid(p.x, p.y)
    }

    fn all_field_objects(&self) -> Vec<ObjectRef> {
        self.field.objects.slots.iter().flatten().copied().collect()
    }

    fn field_object_slot(&self, side: u8, i: u8) -> Option<ObjectRef> {
        if i >= 3 {
            return None;
        }
        self.field.objects.slots[(side as usize & 1) * 3 + i as usize]
    }

    fn battle_time(&self) -> u32 {
        self.round.battle_time
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

    fn break_panel(&mut self, p: PanelPos, sound: Option<u16>) -> bool {
        Battle::break_panel_sounding(self, p.x, p.y, sound.map(SoundId))
    }

    fn panel_solid(&self, p: PanelPos) -> bool {
        self.field.is_solid(p.x, p.y)
    }

    fn highlight_panel(&mut self, p: PanelPos) {
        common::highlight_panel(self, p.x, p.y);
    }

    fn set_header_flags(&mut self, o: ObjectRef, flags: u8) {
        // A write that leaves VISIBLE as it was leaves who sees it as it was.
        let ob = self.objects.get_mut(o);
        let visible = flags & crate::object::flags::VISIBLE != 0;
        if visible != (ob.flags & crate::object::flags::VISIBLE != 0) {
            ob.set_visible(visible);
        }
        ob.flags = flags;
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

    fn chips_enabled(&self, o: ObjectRef) -> ApiResult<bool> {
        self.actor_of(o)?;
        Ok(kinds::player::chips_enabled(self, o))
    }

    fn move_lag(&self, o: ObjectRef) -> ApiResult<u16> {
        self.actor_of(o)?;
        Ok(kinds::player::move_lag(self, o))
    }

    fn drop_links(&mut self, o: ObjectRef) -> ApiResult<()> {
        self.actor_of(o)?;
        kinds::player::drop_links(self, o);
        Ok(())
    }

    fn drop_status_visuals(&mut self, o: ObjectRef) -> ApiResult<()> {
        let c = self.collision_of_mut(o)?;
        c.links[crate::collision::link::CONFUSE] = None;
        c.links[crate::collision::link::BLIND] = None;
        Ok(())
    }

    fn drop_chip(&mut self, o: ObjectRef) -> ApiResult<()> {
        self.actor_of(o)?;
        self.objects.get_mut(o).chip = None;
        Ok(())
    }

    fn drop_alive_count(&mut self, o: ObjectRef) -> ApiResult<()> {
        self.actor_of(o)?;
        let side = self.objects.get(o).alliance as usize;
        self.round.alive[side] = self.round.alive[side].wrapping_sub(1);
        Ok(())
    }

    fn drop_barrier(&mut self, o: ObjectRef) -> ApiResult<()> {
        self.collision_of_mut(o)?.barrier = 0;
        Ok(())
    }

    fn leave(&mut self, o: ObjectRef) -> ApiResult<()> {
        self.actor_of(o)?;
        kinds::player::leave(self, o);
        Ok(())
    }

    fn can_stand_any_side(&self, o: ObjectRef, p: PanelPos) -> bool {
        if !crate::field::is_valid(p.x, p.y) {
            return false;
        }
        let ob = self.objects.get(o);
        let airshoe = ob.collision.is_some_and(|c| self.collision.get(c).f1 & f1::AIRSHOE != 0);
        let floor_free = airshoe || !self.field.is_solid(ob.panel.x, ob.panel.y);
        let rule = self.game_rules().panels.any_side_step.get(floor_free, ob.alliance);
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

    fn spawn_def(&mut self, kind: u16, pos: Vec3, at: SpawnAt) -> ApiResult<Option<ObjectRef>> {
        if kind as usize >= self.content.defs.kinds.len() {
            return Err(ApiError::Other(format!("no kind has handle {kind}")));
        }
        Ok(kinds::spawn(self, KindHandle(kind), at, pos, [0; 4]))
    }

    fn object_kind(&self, o: ObjectRef) -> Option<u16> {
        Some(self.objects.get(o).kind.0)
    }

    fn rush_cancels(&mut self, o: ObjectRef, chip: u16) -> ApiResult<bool> {
        self.actor_of(o)?;
        Ok(kinds::player::rush_cancels(self, o, nettai_content_api::ChipHandle(chip)))
    }

    fn load_chip_attack(&mut self, o: ObjectRef, chip: u16) -> ApiResult<()> {
        self.actor_of(o)?;
        kinds::player::load_chip_attack(self, o, nettai_content_api::ChipHandle(chip));
        Ok(())
    }

    fn navi_action(&self, o: ObjectRef) -> ApiResult<NaviAction> {
        use kinds::player::{EngineAction as E, NaviAction as A};
        Ok(match self.actor_of(o)?.navi_action {
            A::Entry => NaviAction::Engine("entry"),
            A::TakeControl => NaviAction::Engine("take_control"),
            A::Deletion => NaviAction::Engine("deletion"),
            A::Flinch => NaviAction::Engine("flinch"),
            A::Paralysis => NaviAction::Engine("paralysis"),
            A::Drag => NaviAction::Engine("drag"),
            A::Freeze => NaviAction::Engine("freeze"),
            A::Bubble => NaviAction::Engine("bubble"),
            A::Idle => NaviAction::Engine("idle"),
            A::Engine(E::Move) => NaviAction::Engine("move"),
            A::Engine(E::DimmingChip) => NaviAction::Engine("dimming_chip"),
            A::Engine(E::NaviChip) => NaviAction::Engine("navi_chip"),
            A::Engine(E::InstantChip) => NaviAction::Engine("instant_chip"),
            A::Engine(E::FormChange) => NaviAction::Engine("form_change"),
            A::Content(h) => NaviAction::Content(h.0),
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
        // A navi's action is its NaviAction: the framework state the
        // progress word names (every lifecycle state starts at the first).
        if self.objects.get(o).actor.is_some() {
            let action = kinds::player::NaviAction::state(p.action).expect("a lifecycle state starts at a framework state");
            kinds::player::set_navi_action(self, o, action);
        }
    }

    fn set_lifecycle_only(&mut self, o: ObjectRef, l: Lifecycle) {
        self.objects.get_mut(o).state = match l {
            Lifecycle::Init => state::INIT,
            Lifecycle::Update => state::UPDATE,
            Lifecycle::Destroy => state::DESTROY,
            Lifecycle::Finish => state::FINISH,
        };
    }

    fn set_action(&mut self, o: ObjectRef, action: u8) -> ApiResult<()> {
        if self.objects.get(o).actor.is_some() {
            return Err(navi_action_byte());
        }
        common::set_action(self, o, action);
        Ok(())
    }

    fn get(&self, o: ObjectRef, f: ObjectField) -> ApiResult<Value> {
        let ob = self.objects.get(o);
        if let Some(bit) = flag_bit(f) {
            return Ok(Value::Bool(ob.flags & bit != 0));
        }
        let i = |v: i64| Value::Int(v);
        Ok(match f {
            // A navi's action is its NaviAction, which has no byte.
            ObjectField::Action if ob.actor.is_some() => return Err(navi_action_byte()),
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
            ObjectField::Identity => ob.identity.map_or(Value::Nil, |h| Value::Def(Registry::Identity, h.0)),
            ObjectField::PreventAnim => i(ob.prevent_anim as i64),
            ObjectField::ChipsHeld => i(ob.chips_held as i64),
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
        })
    }

    fn set(&mut self, o: ObjectRef, f: ObjectField, v: Value) -> ApiResult<()> {
        let v = store(f.name(), f.writable(), f.ty(), v)?;
        // A navi's action is its NaviAction, which has no byte.
        if f == ObjectField::Action && self.objects.get(o).actor.is_some() {
            return Err(navi_action_byte());
        }
        let ob = self.objects.get_mut(o);
        if let Some(bit) = flag_bit(f) {
            let FieldValue::Bool(on) = v else { unreachable!() };
            if f == ObjectField::Visible {
                // For every viewer (`hide_from_blind` and `copy_visibility`
                // decide it per viewer).
                ob.set_visible(on);
            } else {
                ob.flags = if on { ob.flags | bit } else { ob.flags & !bit };
            }
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
            (ObjectField::Identity, FieldValue::Ref(None)) => ob.identity = None,
            (ObjectField::Identity, FieldValue::Ref(Some((Registry::Identity, h)))) => {
                ob.identity = Some(nettai_content_api::IdentityHandle(h));
            }
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

    fn load_or_step_sprite(&mut self, o: ObjectRef) {
        common::load_or_step_sprite(self, o);
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
            self.objects.get_mut(o).set_visible(true);
        }
        // Hidden from a blind viewer when the other side's (each console's
        // rule, decided for both viewers).
        let alliance = self.objects.get(o).alliance;
        let hidden = [0u8, 1].map(|viewer| {
            viewer != alliance & 1
                && self.player(viewer).and_then(|p| self.objects.get(p).collision).is_some_and(|c| self.collision.get(c).f1 & f1::BLIND != 0)
        });
        self.hide_from(o, hidden);
    }

    fn hide_from_blind(&mut self, o: ObjectRef) {
        Battle::hide_from_blind(self, o);
    }

    fn copy_visibility(&mut self, from: ObjectRef, to: ObjectRef) {
        Battle::copy_visibility(self, from, to);
    }

    fn update_collision_panels(&mut self, o: ObjectRef) {
        Battle::update_collision_panels(self, o);
    }

    fn snap_to_future_panel(&mut self, o: ObjectRef) {
        kinds::player::snap_to_future_panel(self, o);
    }

    fn state_mut(&mut self, o: ObjectRef) -> Option<nettai_content_api::StateMut<'_>> {
        self.objects.state_mut(o)
    }

    fn rules_state_mut(&mut self, side: u8) -> ApiResult<&mut nettai_content_api::Block> {
        self.rules
            .get_mut(side as usize)
            .and_then(|r| r.state.as_mut())
            .ok_or_else(|| ApiError::Other(format!("side {side} plays by no rules")))
    }

    fn rules_setup(&self, side: u8) -> ApiResult<&nettai_content_api::Block> {
        self.setup
            .players
            .get(side as usize)
            .and_then(|p| p.rules.as_ref())
            .ok_or_else(|| ApiError::Other(format!("side {side}'s player brings no setup of the rules")))
    }

    fn navi_game_stats_mut(&mut self, side: u8) -> ApiResult<&mut nettai_content_api::SmallBlock> {
        let id = self.content.defs.rules().map(|r| r.stats).unwrap_or_default();
        let game = &mut self.stats[side as usize & 1].game;
        if game.id() != id {
            return Err(ApiError::Other(format!("side {side}'s navi's stats aren't of its game's rules' `stats`")));
        }
        Ok(game)
    }

    fn spawn_effect(&mut self, pos: Vec3, look: EffectHandle, flip: u8, palette_add: u8, priority: u8) -> Option<ObjectRef> {
        kinds::effect::spawn(self, pos, look, flip, palette_add, priority)
    }

    fn spawn_effect_after_spawn(&mut self, z: i32, look: EffectHandle, flip: u8, palette_add: u8, priority: u8) -> Option<ObjectRef> {
        kinds::effect::spawn_after_spawn(self, z, look, flip, palette_add, priority)
    }

    fn spawn_region_effects(&mut self, x: i32, y: i32, region: RegionHandle, side: u8, look: EffectHandle, z: i32) {
        kinds::effect::spawn_over_region(self, x, y, region, side, look, z);
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

    fn spawn_spark(&mut self, owner: ObjectRef, pos: Vec3, look: SparkHandle) -> Option<ObjectRef> {
        kinds::spark::spawn(self, owner, pos, look)
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

    fn add_parts(&mut self, o: ObjectRef, identity: nettai_content_api::IdentityHandle, arg: u8) {
        kinds::player::form::put_on_parts(self, o, Some(identity), arg);
    }

    fn remove_parts(&mut self, o: ObjectRef, identity: nettai_content_api::IdentityHandle) {
        kinds::player::form::navi_death_hook(self, o, Some(identity));
    }

    fn add_parts_of(&mut self, o: ObjectRef, owner: ObjectRef, keep_stepping: bool, paused_stepping: bool) {
        let identity = self.objects.get(owner).identity;
        let rec = self.content.navi_record(identity);
        let r2 = if paused_stepping { 1 } else { rec.version };
        kinds::player::form::put_on_parts(self, o, identity, r2);
        if keep_stepping && let Some(part) = self.objects.get(o).related[1] {
            kinds::player::form::keep_overlay_stepping(self, part);
        }
    }

    fn remove_parts_of(&mut self, o: ObjectRef, owner: ObjectRef) {
        let identity = self.objects.get(owner).identity;
        kinds::player::form::navi_death_hook(self, o, identity);
    }

    fn spawn_afterimage(&mut self, owner: ObjectRef, pos: Vec3, spec: &nettai_content_api::api::AfterimageSpec) -> Option<ObjectRef> {
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
            1 => Tether::Form,
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
        let weapon = |w: Option<WeaponHandle>| w.map_or(Value::Nil, |h| Value::Def(Registry::Weapon, h.0));
        Ok(match f {
            ActorField::Overlay => a.overlay.into(),
            ActorField::Step => i(at.step as i64),
            ActorField::StepInit => i(at.step_init as i64),
            ActorField::AttackElement => i(at.element as i64),
            ActorField::AttackDamage => i(at.damage as i64),
            ActorField::HitParam => i(at.hit_param as i64),
            ActorField::AttackVariant => i(at.variant as i64),
            ActorField::Charged => i(at.charged as i64),
            ActorField::AttackLockout => i(at.lockout as i64),
            ActorField::Extra => i(at.extra as i64),
            ActorField::SpecialSource => i(at.special_source as i64),
            ActorField::AttackKind => i(at.kind as i64),
            ActorField::Wrapped => i(at.wrapped as i64),
            ActorField::WrapperFresh => Value::Bool(at.wrapper_fresh),
            ActorField::ControllerFresh => Value::Bool(a.controller_fresh),
            ActorField::MoodHeld => Value::Bool(a.tired || a.exhausted),
            ActorField::Ticked => Value::Bool(a.ticked),
            ActorField::Exhausted => Value::Bool(a.exhausted),
            ActorField::FaceTarget => at.face_target.map_or(Value::Nil, Value::Object),
            ActorField::AttackChip => at.chip.map_or(Value::Nil, |h| Value::Def(Registry::Chip, h.0)),
            ActorField::Marker => i(at.marker as i64),
            ActorField::ThrownLook => at.thrown_look.map_or(Value::Nil, |h| Value::Def(Registry::Record, h.0)),
            ActorField::ThrownAnim => i(at.thrown_anim as i64),
            ActorField::AttackCount => i(at.count as i64),
            ActorField::ActorType => i(actor_type_index(a.actor_type)),
            ActorField::AiIndex => i(a.ai_index as i64),
            ActorField::TargetMarker => a.target_marker.into(),
            ActorField::ChargeGlow => a.charge_glow.into(),
            ActorField::FullSynchroAura => a.full_synchro_aura.into(),
            ActorField::ChargeLevel => i(a.charge_level as i64),
            ActorField::ChargeSource => i(a.charge_source as i64),
            ActorField::ChargeCounter => i(a.charge_counter as i64),
            ActorField::BufferedMove => i(a.buffered_move as i64),
            ActorField::ChipLockout => i(a.lockout as i64),
            ActorField::BackSpecialCooldown => i(a.back_special_cooldown as i64),
            ActorField::BusterWeapon => weapon(a.buster),
            ActorField::ChargeShotWeapon => weapon(a.charge_shot),
            ActorField::BackSpecialWeapon => weapon(a.back_special),
            ActorField::Tired => Value::Bool(a.tired),
            ActorField::BarrierVisual => a.barrier_visual.into(),
            ActorField::PlusTint => i(a.plus_tint as i64),
            ActorField::BChargeTime => a.b_charge_time.map_or(Value::Nil, |t| i(t as i64)),
            ActorField::BChargeGlow => a.b_charge_glow.map_or(Value::Nil, |s| Value::Asset(nettai_content_api::AssetKind::Sprite, s.0)),
            ActorField::BChargeAnim => a.b_charge_anim.map_or(Value::Nil, |n| i(n as i64)),
            ActorField::Primed => Value::Bool(a.primed),
            ActorField::WeaponChip => a.weapon_chip.map_or(Value::Nil, |h| Value::Def(Registry::Chip, h.0)),
            ActorField::NoChargeTimer => i(a.no_charge_timer as i64),
            ActorField::InAutoBattle => Value::Bool(a.in_auto_battle),
        })
    }

    fn actor_set(&mut self, o: ObjectRef, f: ActorField, v: Value) -> ApiResult<()> {
        let v = store(f.name(), f.writable(), f.ty(), v)?;
        let weapon = match f {
            ActorField::BusterWeapon | ActorField::ChargeShotWeapon => self.weapon_from_api(f.name(), v)?,
            _ => None,
        };
        // The attack's chip by definition: a chip, or none.
        let attack_chip = match (f, v) {
            (ActorField::AttackChip, FieldValue::Ref(Some((Registry::Chip, h)))) => {
                let h = ChipHandle(h);
                if h.index() >= self.content.defs.chips.len() {
                    return Err(ApiError::Other(format!("attack_chip: no chip has handle {}", h.0)));
                }
                Some(h)
            }
            (ActorField::AttackChip, FieldValue::Ref(Some((other, _)))) => {
                return Err(ApiError::Other(format!("attack_chip: a {other} is not a chip")));
            }
            _ => None,
        };
        // The weapon's chip by definition: a chip, or none.
        let weapon_chip = match (f, v) {
            (ActorField::WeaponChip, FieldValue::Ref(Some((Registry::Chip, h)))) => self.chip_from_api("weapon_chip", Some(ChipHandle(h)))?,
            (ActorField::WeaponChip, FieldValue::Ref(Some((other, _)))) => {
                return Err(ApiError::Other(format!("weapon_chip: a {other} is not a chip")));
            }
            _ => None,
        };
        // A thrown obstacle's look: an absorbed-look record, or none.
        let thrown_look = match (f, v) {
            (ActorField::ThrownLook, FieldValue::Ref(Some((Registry::Record, h)))) => Some(self.absorbed_look(h)?),
            (ActorField::ThrownLook, FieldValue::Ref(Some((other, _)))) => {
                return Err(ApiError::Other(format!("thrown_look: a {other} is not a record")));
            }
            _ => None,
        };
        let a = self.actor_of_mut(o)?;
        let at = &mut a.attack;
        match (f, v) {
            (ActorField::Overlay, FieldValue::Object(r)) => a.overlay = r,
            (ActorField::Step, FieldValue::U8(x)) => at.step = x,
            (ActorField::StepInit, FieldValue::U8(x)) => at.step_init = x,
            (ActorField::AttackElement, FieldValue::U8(x)) => at.element = x,
            (ActorField::AttackDamage, FieldValue::U16(x)) => at.damage = x,
            (ActorField::HitParam, FieldValue::U16(x)) => at.hit_param = x,
            (ActorField::AttackVariant, FieldValue::U8(x)) => at.variant = x,
            (ActorField::Charged, FieldValue::U8(x)) => at.charged = x,
            (ActorField::AttackLockout, FieldValue::U8(x)) => at.lockout = x,
            (ActorField::Extra, FieldValue::U16(x)) => at.extra = x,
            (ActorField::SpecialSource, FieldValue::U8(x)) => at.special_source = x,
            (ActorField::Wrapped, FieldValue::U8(x)) => at.wrapped = x,
            (ActorField::WrapperFresh, FieldValue::Bool(x)) => at.wrapper_fresh = x,
            (ActorField::ControllerFresh, FieldValue::Bool(x)) => a.controller_fresh = x,
            (ActorField::Ticked, FieldValue::Bool(x)) => a.ticked = x,
            (ActorField::Exhausted, FieldValue::Bool(x)) => a.exhausted = x,
            (ActorField::AttackChip, FieldValue::Ref(_)) => at.chip = attack_chip,
            (ActorField::Marker, FieldValue::U32(x)) => at.marker = x,
            (ActorField::ThrownLook, FieldValue::Ref(_)) => at.thrown_look = thrown_look,
            (ActorField::ThrownAnim, FieldValue::U8(x)) => at.thrown_anim = x,
            (ActorField::AttackCount, FieldValue::U16(x)) => at.count = x,
            (ActorField::TargetMarker, FieldValue::Object(r)) => a.target_marker = r,
            (ActorField::ChargeGlow, FieldValue::Object(r)) => a.charge_glow = r,
            (ActorField::FullSynchroAura, FieldValue::Object(r)) => a.full_synchro_aura = r,
            (ActorField::BufferedMove, FieldValue::U8(x)) => a.buffered_move = x,
            (ActorField::ChipLockout, FieldValue::U8(x)) => a.lockout = x,
            (ActorField::BackSpecialCooldown, FieldValue::U8(x)) => a.back_special_cooldown = x,
            (ActorField::BusterWeapon, FieldValue::Ref(_)) => a.buster = weapon,
            (ActorField::ChargeShotWeapon, FieldValue::Ref(_)) => a.charge_shot = weapon,
            (ActorField::Tired, FieldValue::Bool(x)) => a.tired = x,
            (ActorField::BarrierVisual, FieldValue::Object(r)) => a.barrier_visual = r,
            (ActorField::PlusTint, FieldValue::U16(x)) => a.plus_tint = x,
            (ActorField::BChargeTime, FieldValue::OptionalU8(x)) => a.b_charge_time = x,
            (ActorField::BChargeGlow, FieldValue::Asset(_, h)) => a.b_charge_glow = h.map(SpriteId),
            (ActorField::BChargeAnim, FieldValue::OptionalU8(x)) => a.b_charge_anim = x,
            (ActorField::Primed, FieldValue::Bool(x)) => a.primed = x,
            (ActorField::WeaponChip, FieldValue::Ref(_)) => a.weapon_chip = weapon_chip,
            (ActorField::NoChargeTimer, FieldValue::U16(x)) => a.no_charge_timer = x,
            (ActorField::InAutoBattle, FieldValue::Bool(x)) => a.in_auto_battle = x,
            (f, v) => unreachable!("{f:?} stored as {v:?}"),
        }
        Ok(())
    }

    fn request(&self, o: ObjectRef, f: RequestFlag) -> ApiResult<bool> {
        if let Some(bit) = flag2_request(f) {
            return Ok(self.collision_of(o)?.f2 & bit != 0);
        }
        if let Some(bit) = own_request(f) {
            return Ok(self.actor_of(o)?.own_requests & bit != 0);
        }
        Ok(self.actor_of(o)?.requests & request_bit(f) != 0)
    }

    fn set_request(&mut self, o: ObjectRef, f: RequestFlag, on: bool) -> ApiResult<()> {
        if let Some(bit) = flag2_request(f) {
            let c = self.collision_of_mut(o)?;
            c.f2 = if on { c.f2 | bit } else { c.f2 & !bit };
            return Ok(());
        }
        if let Some(bit) = own_request(f) {
            let a = self.actor_of_mut(o)?;
            a.own_requests = if on { a.own_requests | bit } else { a.own_requests & !bit };
            return Ok(());
        }
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

    fn action_state_mut(&mut self, o: ObjectRef) -> ApiResult<nettai_content_api::StateMut<'_>> {
        // The running action's: the content action the navi runs.
        self.actor_of(o)?;
        let kinds::player::NaviAction::Content(h) = kinds::player::navi_action(self, o) else {
            return Err(ApiError::NoState(o));
        };
        let id = self.content.defs.action(h).schema;
        self.attack_state_for(o, id)
    }

    fn attack_state_for(&mut self, o: ObjectRef, id: StateId) -> ApiResult<nettai_content_api::StateMut<'_>> {
        let a = self.objects.get(o).actor.ok_or(ApiError::NoActor(o))?;
        // The game keeps an action's variables in the shared attack state,
        // where they outlive the action; an action of another layout starts
        // from zero.
        if self.actors.get(a).attack.action != ActionVars::Content(id) {
            let size = self.content.defs.schema(id).size();
            self.actors.set_action_state(a, id, size);
        }
        Ok(self.actors.action_state_mut(a).expect("the attack state holds a content action's"))
    }

    fn action_schema(&self, action: u16) -> ApiResult<StateId> {
        let defs = &self.content.defs;
        if action as usize >= defs.actions.len() {
            return Err(ApiError::Other(format!("no action has handle {action}")));
        }
        Ok(defs.action(ActionHandle(action)).schema)
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

    fn clear_statuses(&mut self, o: ObjectRef) -> ApiResult<()> {
        self.collision_of_mut(o)?.f1 = 0;
        Ok(())
    }

    fn status_timer(&self, o: ObjectRef, t: StatusTimer) -> ApiResult<u16> {
        Ok(self.collision_of(o)?.status_timers[timer_index(t)])
    }

    fn set_status_timer(&mut self, o: ObjectRef, t: StatusTimer, v: u16) -> ApiResult<()> {
        self.collision_of_mut(o)?.status_timers[timer_index(t)] = v;
        Ok(())
    }

    fn open_counter_window(&mut self, o: ObjectRef, ticks: u8) {
        kinds::player::actions::open_counter_window(self, o, ticks);
    }

    fn check_reactive_abort(&mut self, o: ObjectRef) {
        kinds::player::actions::check_reactive_abort(self, o);
    }

    fn spawn_mode9_objects(&mut self, o: ObjectRef) {
        kinds::player::spawn_mode9_objects(self, o);
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

    fn clear_invulnerable(&mut self, o: ObjectRef) -> ApiResult<()> {
        self.actor_of(o)?;
        kinds::player::clear_invulnerable(self, o);
        Ok(())
    }

    fn face_default(&mut self, o: ObjectRef) -> ApiResult<()> {
        self.actor_of(o)?;
        kinds::player::actions::transform::face_default(self, o);
        Ok(())
    }

    fn reset_charge(&mut self, o: ObjectRef) -> ApiResult<()> {
        self.actor_of(o)?;
        kinds::player::reset_charge(self, o);
        Ok(())
    }

    fn end_full_synchro_aura(&mut self, o: ObjectRef) -> ApiResult<()> {
        self.actor_of(o)?;
        kinds::player::actions::transform::end_full_synchro_aura(self, o);
        Ok(())
    }

    fn drop_statuses(&mut self, o: ObjectRef) -> ApiResult<()> {
        self.actor_of(o)?;
        kinds::player::actions::transform::drop_statuses(self, o);
        Ok(())
    }

    fn end_statuses(&mut self, o: ObjectRef) -> ApiResult<()> {
        self.actor_of(o)?;
        kinds::player::clear_statuses(self, o);
        Ok(())
    }

    fn overlay_stepping(&mut self, o: ObjectRef, keep: bool) {
        if let Some(overlay) = self.objects.get(o).related[1] {
            if keep {
                kinds::player::form::keep_overlay_stepping(self, overlay);
            } else {
                kinds::player::form::normal_overlay_stepping(self, overlay);
            }
        }
    }

    fn set_overlay_anim_offset(&mut self, o: ObjectRef, offset: u8) {
        if let Some(overlay) = self.objects.get(o).related[1] {
            kinds::body_overlay::set_extra_offset(self, overlay, offset);
        }
    }

    fn take_off_form_overlay(&mut self, o: ObjectRef, form: nettai_content_api::FormHandle) {
        kinds::player::form::take_off_overlay(self, o, form);
    }

    fn put_on_form_overlay(&mut self, o: ObjectRef, form: nettai_content_api::FormHandle) {
        kinds::player::form::put_on_overlay(self, o, form);
    }

    fn take_off_form_parts(&mut self, o: ObjectRef, form: nettai_content_api::FormHandle) {
        kinds::player::form::take_off_form_parts(self, o, form);
    }

    fn put_on_form_parts(&mut self, o: ObjectRef, form: nettai_content_api::FormHandle) {
        kinds::player::form::put_on_form_parts(self, o, form);
    }

    fn load_form_sprite(&mut self, o: ObjectRef, form: nettai_content_api::FormHandle) -> ApiResult<()> {
        self.actor_of(o)?;
        let side = self.objects.get(o).alliance as usize & 1;
        let sprite = self.content.navi_sprite(self.stats[side].navi, form);
        let flip = self.objects.get(o).alliance ^ self.objects.get(o).flip;
        let s = self.objects.sprite_mut(o);
        s.load(sprite);
        // sprite_hasShadow, sprite_setFlip(object_getFlip()), white.
        s.look.shadow = crate::object::sprite::Shadow::Ground;
        s.look.set_flip(flip);
        s.look.white = true;
        let ob = self.objects.get_mut(o);
        ob.flags &= !crate::object::flags::NO_SPRITE_UPDATE;
        // object_setAnimation(0), then the sprite restarts it directly.
        ob.anim = 0;
        ob.anim_loaded = 0xFF;
        let content = self.content.clone();
        self.objects.sprite_mut(o).set_animation(0, &content);
        Ok(())
    }

    fn strip_body_programs(&mut self, o: ObjectRef, undershirt: bool) -> ApiResult<bool> {
        self.actor_of(o)?;
        Ok(kinds::player::strip_body_programs(self, o, undershirt))
    }

    fn refresh_form_flags(&mut self, o: ObjectRef) -> ApiResult<()> {
        self.actor_of(o)?;
        kinds::player::form::refresh_form_flags(self, o);
        Ok(())
    }

    fn take_status(&mut self, o: ObjectRef, status: nettai_content_api::StatusHandle) -> ApiResult<()> {
        self.collision_of(o)?;
        kinds::player::take_status(self, o, status);
        Ok(())
    }

    fn reset_status(&mut self, o: ObjectRef) -> ApiResult<()> {
        self.actor_of(o)?;
        kinds::player::reset_status(self, o);
        Ok(())
    }

    fn end_anger(&mut self, o: ObjectRef) -> ApiResult<()> {
        self.actor_of(o)?;
        kinds::player::end_anger(self, o);
        Ok(())
    }

    fn form_change_target(&self, o: ObjectRef) -> Option<nettai_content_api::FormHandle> {
        kinds::player::form_change_target(self, o)
    }

    fn stop_moving(&mut self, o: ObjectRef) -> ApiResult<()> {
        self.actor_of(o)?;
        kinds::player::actions::transform::stop_moving(self, o);
        Ok(())
    }

    fn form_change_terms(&self, o: ObjectRef) -> (u8, bool) {
        let t = &self.turn_transforms[self.objects.get(o).alliance as usize & 1];
        (t.turns, t.alternate)
    }

    fn pin_overlay(&mut self, o: ObjectRef) {
        kinds::player::form::pin_overlay(self, o);
    }

    fn end_attack(&mut self, o: ObjectRef) {
        kinds::player::end_attack(self, o);
    }

    fn set_content_attack(&mut self, o: ObjectRef, action: u16, kind: u8) -> ApiResult<()> {
        if action as usize >= self.content.defs.actions.len() {
            return Err(ApiError::Other(format!("no action has handle {action}")));
        }
        kinds::player::set_attack(self, o, kinds::player::NaviAction::Content(ActionHandle(action)), kind);
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

    fn next_chip(&self, o: ObjectRef) -> ApiResult<Option<ChipHandle>> {
        self.actor_of(o)?;
        Ok(kinds::player::next_chip(self, o))
    }

    fn use_chip(&mut self, o: ObjectRef) -> ApiResult<bool> {
        self.actor_of(o)?;
        Ok(kinds::player::chip_use::use_chip(self, o).is_some())
    }

    fn start_chip_attack(&mut self, o: ObjectRef, chip: ChipHandle, kind: u8) -> ApiResult<()> {
        self.actor_of(o)?;
        let action = kinds::player::chip_use::chip_action(self, o, Some(chip));
        kinds::player::set_attack(self, o, action, kind);
        Ok(())
    }

    fn start_move_to(&mut self, o: ObjectRef, target: PanelPos, end_lag: u16, face: Option<ObjectRef>) -> ApiResult<()> {
        self.actor_of(o)?;
        use kinds::player::actions::movement::{self, MoveKind};
        movement::start_absolute_facing(self, o, target, end_lag, MoveKind::Absolute, face);
        Ok(())
    }

    fn start_weapon(&mut self, o: ObjectRef, weapon: nettai_content_api::WeaponHandle, kind: u8) -> ApiResult<()> {
        self.actor_of(o)?;
        kinds::player::start_weapon(self, o, weapon, kind);
        Ok(())
    }

    fn run_wrapped(&mut self, o: ObjectRef) -> ApiResult<()> {
        self.actor_of(o)?;
        let action = kinds::player::navi_action(self, o);
        if !action.is_attack() {
            return Err(ApiError::Other(format!("run_wrapped: the navi runs {action:?}, not an attack")));
        }
        kinds::player::actions::dispatch(self, o, action);
        Ok(())
    }

    fn chain_next_chip(&mut self, o: ObjectRef) -> ApiResult<bool> {
        self.actor_of(o)?;
        Ok(kinds::player::chip_use::chain_next_chip(self, o))
    }

    fn panel_trail(&mut self, o: ObjectRef, from: PanelPos) -> ApiResult<()> {
        self.actor_of(o)?;
        kinds::player::actions::movement::panel_trail(self, o, from);
        Ok(())
    }

    fn freeze_target_marker(&mut self, marker: ObjectRef, on: bool) -> ApiResult<()> {
        if !self.objects.is_allocated(marker) || self.content.defs.engine_kind(self.objects.get(marker).kind) != Some(kinds::EngineKind::TargetMarker) {
            return Err(ApiError::Other("freeze_target_marker: not a lock-on marker".into()));
        }
        if on { kinds::target_marker::freeze(self, marker) } else { kinds::target_marker::unfreeze(self, marker) }
        Ok(())
    }

    fn face_toward(&mut self, o: ObjectRef, target: ObjectRef) -> ApiResult<()> {
        self.actor_of(o)?;
        if !self.objects.is_allocated(target) {
            return Err(ApiError::Other("face_toward: the target is gone".into()));
        }
        kinds::player::face_toward(self, o, target);
        Ok(())
    }

    fn can_move(&self, o: ObjectRef) -> bool {
        self.collision_of(o).is_ok_and(|c| c.f1 & (f1::IMMOBILIZED | f1::SLIDING | f1::MOVING) == 0)
    }

    fn heal(&mut self, o: ObjectRef, amount: u16, anti_recovery: bool) -> bool {
        kinds::heal::heal(self, o, amount, anti_recovery)
    }

    fn subtract_hp(&mut self, o: ObjectRef, amount: u16) {
        kinds::subtract_hp(self, o, amount);
    }

    fn buster_damage(&self, o: ObjectRef) -> u16 {
        kinds::player::idle::buster_damage(self, o)
    }

    fn prepare_chip(&mut self, o: ObjectRef) {
        kinds::player::prepare_chip(self, o);
    }

    fn absorbed(&self, o: ObjectRef) -> ApiResult<Vec<(u16, u8)>> {
        Ok(self.actor_of(o)?.absorbed.iter().map(|a| (a.look.0, a.anim)).collect())
    }

    fn push_absorbed(&mut self, o: ObjectRef, look: u16, anim: u8) -> ApiResult<bool> {
        let look = self.absorbed_look(look)?;
        let list = &mut self.actor_of_mut(o)?.absorbed;
        if list.len() >= 8 {
            return Ok(false);
        }
        list.push(AbsorbedObstacle { look, anim });
        Ok(true)
    }

    fn pop_absorbed(&mut self, o: ObjectRef) -> ApiResult<Option<(u16, u8)>> {
        Ok(self.actor_of_mut(o)?.absorbed.pop().map(|a| (a.look.0, a.anim)))
    }

    // ---- Sprites -------------------------------------------------------------------

    fn sprite_load(&mut self, o: ObjectRef, id: SpriteId) {
        // `sprite_load` also lets the sprite animate (header flag 0x08).
        self.objects.sprite_mut(o).load(id);
        self.objects.get_mut(o).flags &= !flags::NO_SPRITE_UPDATE;
    }

    fn sprite_load_look_of(&mut self, o: ObjectRef, owner: ObjectRef) {
        let identity = self.content.identity(self.objects.get(owner).identity);
        let id = if identity.record.actor_type == crate::actor::ActorType::Player {
            kinds::player::stats_sprite(self, self.objects.get(owner).alliance)
        } else {
            identity
                .look
                .and_then(|l| l.sprite)
                .unwrap_or_else(|| panic!("identity {:?} has no look (sub_800F26C)", identity.key))
        };
        self.sprite_load(o, id);
    }

    fn sprite_load_like(&mut self, o: ObjectRef, like: ObjectRef) -> ApiResult<()> {
        let identity = self.content.identity(self.objects.get(like).identity);
        let id = if identity.record.actor_type == crate::actor::ActorType::Player {
            // sub_800FC9E(navi stat 0x29, form stat 0x2C): MegaMan's form's
            // sprite, or the link navi's.
            let side = self.objects.get(like).alliance as usize & 1;
            let s = &self.stats[side];
            self.content.navi_sprite(s.navi, s.form)
        } else {
            // sub_800F26C: a field object's look by its identity (NameID
            // 0xCD and up); other identities' sprites (viruses, bosses)
            // aren't in the content: no netbattle object stands in for one.
            identity.look.and_then(|l| l.sprite).ok_or_else(|| {
                ApiError::Other(format!(
                    "sub_800F26C: identity {:?} has no sprite in the content (a netbattle's stand-in copies a player)",
                    identity.key
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

    fn name_look_is(&self, o: ObjectRef, sprite: SpriteId) -> ApiResult<bool> {
        use crate::content::{IdentityClass, IdentityOwner};
        let identity = self.content.identity(self.objects.get(o).identity);
        // A field object's look (none: no sprite), or a navi's or form's
        // own sprite.
        let look = match (identity.class, identity.owner) {
            (IdentityClass::FieldObject, _) => identity.look.map(|l| l.sprite),
            (_, Some(IdentityOwner::Navi(n))) => Some(Some(self.content.navi(n).sprite)),
            (_, Some(IdentityOwner::Form(f))) => Some(Some(self.content.form(f).sprite)),
            _ => None,
        };
        look.map(|l| l == Some(sprite))
            .ok_or_else(|| ApiError::Other(format!("identity {:?} has no look in the content (sub_800F26C)", identity.key)))
    }

    fn sprite_part_offset(&self, o: ObjectRef, n: u8) -> (i32, i32) {
        let s = self.objects.sprite(o);
        s.id
            .and_then(|id| self.content.animations.part_offset(id, s.anim, s.frame, n as usize))
            .map_or((0, 0), |(x, y)| (x as i32, y as i32))
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
            SpriteField::UnderObjects => Value::Bool(look.under_objects),
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
            (SpriteField::UnderObjects, FieldValue::Bool(x)) => look.under_objects = x,
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

    fn setup_collision(&mut self, o: ObjectRef, self_type: CollisionHandle, target_type: CollisionHandle, hit_mod: u8) {
        Battle::setup_collision(self, o, self_type, target_type, hit_mod);
    }

    fn reset_collision_types(&mut self, o: ObjectRef, self_type: CollisionHandle, target_type: CollisionHandle, hit_mod: u8) {
        Battle::reset_collision_types(self, o, self_type, target_type, hit_mod);
    }

    fn collision_get(&self, o: ObjectRef, f: CollisionField) -> ApiResult<Value> {
        let c = self.collision_of(o)?;
        // The definitions it names.
        let def = |registry, h: Option<u16>| Ok(h.map_or(Value::Nil, |h| Value::Def(registry, h)));
        match f {
            CollisionField::StatusBase => return def(Registry::Status, c.status_base.map(|h| h.0)),
            CollisionField::StatusFinal => return def(Registry::Status, c.status_final.map(|h| h.0)),
            CollisionField::Region => return def(Registry::Region, c.region.map(|h| h.0)),
            CollisionField::HitEffect => return def(Registry::Spark, c.hit_effect.map(|h| h.0)),
            _ => {}
        }
        Ok(Value::Int(match f {
            CollisionField::Region | CollisionField::HitEffect | CollisionField::StatusBase | CollisionField::StatusFinal => unreachable!("handled above"),
            CollisionField::PanelX => c.panel.x as i64,
            CollisionField::PanelY => c.panel.y as i64,
            CollisionField::Element => c.element as i64,
            CollisionField::SecondaryElement => c.secondary_element as i64,
            CollisionField::Bugs => c.bugs as i64,
            CollisionField::InflictedBugs => c.acc.inflicted_bugs as i64,
            CollisionField::HitModBase => c.hit_mod_base as i64,
            CollisionField::SelfDamage => c.self_damage as i64,
            CollisionField::CounterByte => c.counter_byte as i64,
            CollisionField::CounterTimer => c.counter_timer as i64,
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
        // The definitions it names: a handle of the field's registry, or
        // none.
        let defined = match f {
            CollisionField::StatusBase => Some((Registry::Status, self.content.defs.statuses.len())),
            CollisionField::StatusFinal => Some((Registry::Status, self.content.defs.statuses.len())),
            CollisionField::Region => Some((Registry::Region, self.content.defs.regions.len())),
            CollisionField::HitEffect => Some((Registry::Spark, self.content.defs.sparks.len())),
            _ => None,
        };
        if let Some((registry, len)) = defined {
            let h = match v {
                FieldValue::Ref(Some((r, h))) if r == registry && (h as usize) < len => Some(h),
                FieldValue::Ref(None) => None,
                other => return Err(ApiError::Other(format!("{}: {other:?} is not a {registry}", f.name()))),
            };
            let c = self.collision_of_mut(o)?;
            match f {
                CollisionField::StatusBase => c.status_base = h.map(nettai_content_api::StatusHandle),
                CollisionField::StatusFinal => c.status_final = h.map(nettai_content_api::StatusHandle),
                CollisionField::Region => c.region = h.map(RegionHandle),
                _ => c.hit_effect = h.map(SparkHandle),
            }
            return Ok(());
        }
        let c = self.collision_of_mut(o)?;
        let x = int(v);
        match f {
            CollisionField::Region | CollisionField::HitEffect | CollisionField::StatusBase | CollisionField::StatusFinal => unreachable!("handled above"),
            CollisionField::PanelX => c.panel.x = x as u8,
            CollisionField::PanelY => c.panel.y = x as u8,
            CollisionField::Element => c.element = x as u8,
            CollisionField::SecondaryElement => c.secondary_element = x as u8,
            CollisionField::Bugs => c.bugs = x as u16,
            CollisionField::InflictedBugs => c.acc.inflicted_bugs = x as u16,
            CollisionField::HitModBase => c.hit_mod_base = x as u8,
            CollisionField::SelfDamage => c.self_damage = x as u16,
            CollisionField::CounterByte => c.counter_byte = x as u8,
            CollisionField::HitFlags => c.acc.hit_flags = x as u32,
            CollisionField::FinalDamage
            | CollisionField::CounterTimer
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

    fn raise_barrier(&mut self, o: ObjectRef, spec: nettai_content_api::api::BarrierSpec) -> ApiResult<()> {
        let c = self.collision_of_mut(o)?;
        // The barrier byte the rules' barrier code (`sub_801A802`) tells
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
            self.content.region_offsets(s.region).iter().map(|p| (x + p.dx as i32 * dir, y + p.dy as i32)).collect();
        // `object_highlightPanel` skips panels off the field.
        for (px, py) in panels {
            if (1..=6).contains(&px) && (1..=3).contains(&py) {
                common::highlight_panel(self, px as u8, py as u8);
            }
        }
    }

    // ---- Services ------------------------------------------------------------

    fn dimming(&mut self, o: ObjectRef, step: DimmingStep, chip: Option<ChipHandle>) {
        use crate::dimming as d;
        // A controller's chip field: none is the zeroed field's.
        let chip = self.chip_from_api("a dimming step's chip", chip).unwrap_or_else(|e| panic!("{e}"));
        match step {
            DimmingStep::Begin => d::begin(self, o),
            DimmingStep::DimScreen => d::dim_screen(self, o),
            DimmingStep::ShowTelop => d::show_telop(self, o),
            DimmingStep::ShowHiddenTelop => d::show_hidden_telop(self, o),
            DimmingStep::CheckAntiNavi => d::check_anti_navi(self, o, chip),
            DimmingStep::ShowNaviTelop => d::show_navi_telop(self, o, chip),
            DimmingStep::UndimScreen => d::undim_screen(self, o),
            DimmingStep::FadeToBlack => d::fade_to_black(self, o),
            DimmingStep::FadeFromBlack => d::fade_from_black(self, o),
            DimmingStep::Finish => d::end(self, o),
        }
    }

    fn start_dimming(
        &mut self,
        side: u8,
        no_cut_in: bool,
        controller: Option<ObjectRef>,
        user: ObjectRef,
        telop: Option<(Option<ChipHandle>, u16)>,
    ) {
        // What its telop shows (the controller's +0x30 and +0x32, which the
        // controller's own spawn stored).
        if let (Some(c), Some((chip, bonus))) = (controller, telop) {
            self.objects.get_mut(c).telop_chip = Some(crate::hud::TelopChip { chip, bonus, damage: None });
        }
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

    fn hide_actor(&mut self, o: ObjectRef) {
        crate::dimming::hide_actor(self, o);
    }

    fn show_actor(&mut self, o: ObjectRef) {
        crate::dimming::show_actor(self, o);
    }

    fn clear_bugs(&mut self, side: u8) {
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

    fn clear_emotion_window_glitch(&mut self) {
        Battle::clear_emotion_window_glitch(self);
    }

    fn navi_chip_left(&mut self, controller: ObjectRef) {
        kinds::navi_chip::navi_left(self, controller);
    }

    fn last_navi_chip(&self) -> Option<(ChipHandle, u8, u32)> {
        self.last_navi_chip.map(|l| (l.chip, l.element, l.damage))
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
            ObstacleCrush::Ignores => Crush::Ignores,
            ObstacleCrush::DestroysSparingBodies => Crush::DestroysSparingBodies,
        };
        let hold = match hold {
            ObstacleHold::AfterAppearing => Hold::AfterAppearing,
            ObstacleHold::Always => Hold::Always,
        };
        Ok(kinds::obstacle::react(self, o, crush, hold))
    }

    fn obstacle_action_byte(&self, o: ObjectRef, a: u8) -> ApiResult<u8> {
        kinds::obstacle::action_byte(self, o, a).map_err(ApiError::Other)
    }

    fn obstacle_current_action(&self, o: ObjectRef) -> u8 {
        kinds::obstacle::current_action(self, o)
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

    fn obstacle_fly_to_absorber(&mut self, o: ObjectRef, look: u16) -> ApiResult<()> {
        self.collision_of(o)?;
        let look = self.absorbed_look(look)?;
        kinds::obstacle::fly_to_absorber(self, o, look);
        Ok(())
    }

    fn obstacle_release_tracking(&mut self, o: ObjectRef) {
        kinds::obstacle::release_tracking(self, o);
    }

    fn obstacle_stage_slot_free(&self) -> bool {
        self.field.objects.stage_slot_free()
    }

    fn obstacle_enter_stage(&mut self, o: ObjectRef) -> ApiResult<()> {
        if self.field.objects.enter_stage(o) {
            Ok(())
        } else {
            Err(ApiError::Other("both stage-object slots are taken".into()))
        }
    }

    fn obstacle_leave_stage(&mut self, o: ObjectRef) {
        self.field.objects.leave_stage(o);
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
        // The original tests the NameID word, whose high half is an
        // actor's next chip (0xFFFF for none) and 0 for anything else: an
        // actor's word is a field object's only with chip 0 next.
        let plain = ob.actor.is_none() || (ob.chip.is_some() && ob.chip == self.zeroed_chip());
        let identity = self.content.identity(ob.identity);
        plain && identity.class == crate::content::IdentityClass::FieldObject && identity.scrap
    }

    fn obstacle_throwable(&self, o: ObjectRef) -> bool {
        self.content.identity(self.objects.get(o).identity).throwable
    }

    fn obstacle_throw(&mut self, o: ObjectRef, side: u8, x: u8, y: u8, shake: u8, damage: u32) {
        kinds::obstacle::request_throw(self, o, side, x, y, shake, damage);
    }

    fn obstacle_arm_conversion(&mut self, side: u8, melee: u32, ranged: u32) {
        self.obstacle_conversion[side as usize & 1] = kinds::obstacle::Conversion { armed: true, words: [melee, ranged] };
    }

    fn obstacle_disarm_conversion(&mut self, side: u8) {
        self.obstacle_conversion[side as usize & 1].armed = false;
    }

    fn obstacle_conversion(&self, side: u8) -> (bool, u32, u32) {
        let s = self.obstacle_conversion[side as usize & 1];
        (s.armed, s.words[0], s.words[1])
    }

    fn obstacle_present(&self, o: ObjectRef) -> bool {
        use kinds::obstacle::f2;
        self.objects
            .get(o)
            .collision
            .is_some_and(|c| self.collision.get(c).f2 & (f2::ABSORBED | f2::VANISH | f2::REMOVED) == 0)
    }

    fn loop_register(&self) -> u32 {
        self.objects.loop_register()
    }

    fn wear_navi_image(&mut self, o: ObjectRef, user: ObjectRef, megaman: nettai_content_api::NaviHandle) -> ApiResult<bool> {
        // The user's identity when it is MegaMan's or one of his forms'
        // (the original's NameID 0x1A0, or past the link navis'), else
        // MegaMan's.
        let base = self.content.base_form_for(megaman);
        let megaman_identity = self.content.navi(megaman).identity;
        let user_name = self.objects.get(user).identity;
        let own = self.content.identity(user_name).changes_form;
        let name = if own { user_name } else { megaman_identity };
        let sprite = if !own {
            self.content.navi_sprite(megaman, base)
        } else if self.content.navi_record(name).actor_type == crate::actor::ActorType::Player {
            kinds::player::stats_sprite(self, self.objects.get(user).alliance)
        } else {
            return Err(ApiError::Other(format!(
                "identity {:?} is no player's: its sprite would be sub_800F26C's (enemy_getStruct1)",
                self.content.identity(name).key
            )));
        };
        let side = self.objects.get(o).alliance;
        // byte_80203EA covers the base form and the Crosses; the bytes
        // after it (the Beast forms') are 0: the form's `palette`.
        let palette = self.content.form(self.stats[side as usize].form).palette;
        self.sprite_load(o, sprite);
        let obj = self.objects.get_mut(o);
        obj.identity = name;
        obj.anim = 0;
        obj.anim_loaded = 0xFF;
        let look = &mut self.objects.sprite_mut(o).look;
        look.shadow = sprite::Shadow::Ground;
        look.palette = palette;
        Ok(own)
    }

    fn wear_form_image(&mut self, o: ObjectRef, navi: nettai_content_api::NaviHandle, form: nettai_content_api::FormHandle) -> ApiResult<()> {
        let data = self.content.form(form);
        // The base form has no identity of its own: it is the navi's.
        let Some(name) = self.content.form_identity(navi, form) else {
            return Err(ApiError::Other(format!(
                "form {:?} has no identity (nor has navi {:?})",
                self.content.defs.form(form).key,
                self.content.defs.navi(navi).key
            )));
        };
        let sprite = data.sprite;
        let palette = data.palette;
        self.sprite_load(o, sprite);
        let obj = self.objects.get_mut(o);
        obj.identity = Some(name);
        obj.anim = 0;
        obj.anim_loaded = 0xFF;
        let look = &mut self.objects.sprite_mut(o).look;
        look.shadow = sprite::Shadow::Ground;
        look.palette = palette;
        Ok(())
    }

    fn navi_image_parts(&mut self, o: ObjectRef, on: bool) {
        let identity = self.objects.get(o).identity;
        if on {
            kinds::player::form::put_on_parts(self, o, identity, 1);
        } else {
            kinds::player::form::navi_death_hook(self, o, identity);
        }
    }

    fn absorbed_look(&self, o: ObjectRef) -> Option<nettai_content_api::IdentityHandle> {
        // sub_800F486: the identities DustMan leaves. (An object with no
        // identity has none to give either; the original's junk would
        // then look up a virus's sprite, which no field object is.)
        let identity = self.objects.get(o).identity;
        identity.filter(|_| self.content.identity(identity).scrap)
    }

    fn wear_absorbed_look(&mut self, o: ObjectRef, look: nettai_content_api::IdentityHandle) -> ApiResult<bool> {
        // sub_800F26C: a field object's look (byte_8021220); any other
        // identity is an actor's (enemy_getStruct1), which no field object
        // is.
        let Some(identity) = self.content.defs.identities.get(look.index()) else {
            return Err(ApiError::Other(format!("no identity has handle {}", look.0)));
        };
        let Some(l) = identity.look.filter(|_| identity.class == crate::content::IdentityClass::FieldObject) else {
            return Err(ApiError::Other(format!("identity {:?} has no absorbed look (enemy_getStruct1's sprite)", identity.key)));
        };
        let Some(id) = l.sprite else { return Ok(false) };
        self.sprite_load(o, id);
        let alliance = {
            let obj = self.objects.get_mut(o);
            obj.set_visible(true);
            obj.anim = l.anim;
            obj.anim_loaded = l.anim;
            obj.alliance
        };
        let s = self.objects.sprite_mut(o);
        s.look.shadow = if l.shadow { sprite::Shadow::Ground } else { sprite::Shadow::WithSprite };
        s.set_animation(l.anim, &self.content);
        s.look.palette = l.palette;
        // The time bombs' looks keep their own flip and set a drawing bit
        // instead (sub_8002EAC: presentation).
        if !l.keeps_flip {
            s.look.set_flip(alliance);
        }
        Ok(true)
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
}
