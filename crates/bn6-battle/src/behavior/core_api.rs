//! The engine's side of the content API: [`CoreApi`] over a `Battle`.

use bn6_content_api::api::ApiResult;
use bn6_content_api::{
    ActorField, ApiError, CollisionField, ContentState, CoreApi, FieldValue, Lifecycle, NaviStat, ObjectField, Shadow,
    SpriteField, SpriteId, StatusFlag, Value,
};

use crate::actor::ActorData;
use crate::battle::Battle;
use crate::collision::{CollisionData, f1};
use crate::kinds::common::{self, Progress};
use crate::kinds::player::actions::ActionVars;
use crate::kinds::{self, Vars};
use crate::object::sprite;
use crate::object::{ObjectRef, PanelPos, Pool, Vec3, flags, state};
use crate::sound::SoundId;

/// The ObjectFlags1 bit behind a status flag.
fn status_bit(flag: StatusFlag) -> u32 {
    match flag {
        StatusFlag::Guard => f1::GUARD,
        StatusFlag::Invisible => f1::INVISIBLE,
        StatusFlag::Invulnerable => f1::INVULNERABLE,
        StatusFlag::Moving => f1::MOVING,
        StatusFlag::Flashing => f1::FLASHING,
        StatusFlag::Flinching => f1::FLINCHING,
        StatusFlag::Paralyzed => f1::PARALYZED,
        StatusFlag::Sliding => f1::SLIDING,
        StatusFlag::Frozen => f1::FROZEN,
        StatusFlag::SuperArmor => f1::SUPERARMOR,
        StatusFlag::MoveComplete => f1::MOVE_COMPLETE,
        StatusFlag::UsingAction => f1::USING_ACTION,
        StatusFlag::Bubbled => f1::BUBBLED,
    }
}

/// The header flag behind a boolean object field.
fn flag_bit(f: ObjectField) -> Option<u8> {
    Some(match f {
        ObjectField::Visible => flags::VISIBLE,
        ObjectField::RunWhilePaused => flags::RUN_WHILE_PAUSED,
        ObjectField::RunInTimeStop => flags::RUN_IN_TIME_STOP,
        ObjectField::NoSpriteUpdate => flags::NO_SPRITE_UPDATE,
        _ => return None,
    })
}

fn int(v: FieldValue) -> i64 {
    match v.load() {
        Value::Int(i) => i,
        v => unreachable!("an integer field stored {v:?}"),
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
fn store(name: &'static str, writable: bool, ty: bn6_content_api::FieldType, v: Value) -> ApiResult<FieldValue> {
    if !writable {
        return Err(ApiError::ReadOnly(name));
    }
    ty.store(v).map_err(|error| ApiError::Type { field: name, error })
}

impl CoreApi for Battle {
    fn is_time_stop(&self) -> bool {
        Battle::is_time_stop(self)
    }

    fn is_paused(&self) -> bool {
        self.paused
    }

    fn play_sound(&mut self, sound: u16) {
        Battle::play_sound(self, SoundId(sound));
    }

    fn navi_stat(&self, side: u8, stat: NaviStat) -> Value {
        let s = &self.stats[side as usize & 1];
        match stat {
            NaviStat::Sun => Value::Bool(s.sun),
            NaviStat::Form => Value::Int(s.form.0 as i64),
            NaviStat::Navi => Value::Int(s.navi.0 as i64),
        }
    }

    fn panel_valid(&self, p: PanelPos) -> bool {
        crate::field::is_valid(p.x, p.y)
    }

    fn panel_center(&self, p: PanelPos) -> (i32, i32) {
        kinds::player::panel_coordinates(p.x, p.y)
    }

    // ---- Objects -----------------------------------------------------------

    fn spawn(&mut self, pool: Pool, index: u8, pos: Vec3, params: [u8; 4]) -> Option<ObjectRef> {
        super::spawn_object(self, pool, index, pos, params)
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

    fn param(&self, o: ObjectRef, n: usize) -> u8 {
        self.objects.get(o).params[n]
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
            ObjectField::Pos => Value::Vec3(ob.pos),
            ObjectField::Vel => Value::Vec3(ob.vel),
            ObjectField::Related1 => ob.related[0].into(),
            ObjectField::Related2 => ob.related[1].into(),
            ObjectField::Visible
            | ObjectField::RunWhilePaused
            | ObjectField::RunInTimeStop
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

    fn attach_point(&self, o: ObjectRef, n: u8) -> (i32, i32) {
        kinds::player::attach_point(self, o, n as usize)
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

    // ---- Actors ------------------------------------------------------------------

    fn actor_get(&self, o: ObjectRef, f: ActorField) -> ApiResult<Value> {
        let a = self.actor_of(o)?;
        Ok(match f {
            ActorField::Overlay => a.overlay.into(),
            ActorField::AttackStep => Value::Int(a.attack.step as i64),
            ActorField::AttackStepInit => Value::Int(a.attack.step_init as i64),
            ActorField::AttackVariant => Value::Int(a.attack.variant as i64),
            ActorField::AttackChip => Value::Int(a.attack.chip_id as i64),
        })
    }

    fn actor_set(&mut self, o: ObjectRef, f: ActorField, v: Value) -> ApiResult<()> {
        let v = store(f.name(), f.writable(), f.ty(), v)?;
        let a = self.actor_of_mut(o)?;
        match (f, v) {
            (ActorField::Overlay, FieldValue::Object(r)) => a.overlay = r,
            (ActorField::AttackStep, FieldValue::U8(x)) => a.attack.step = x,
            (ActorField::AttackStepInit, FieldValue::U8(x)) => a.attack.step_init = x,
            (f, v) => unreachable!("{f:?} stored as {v:?}"),
        }
        Ok(())
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
            a.attack.action = ActionVars::Content(ContentState::new(id, m.schema(id)));
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

    fn open_counter_window(&mut self, o: ObjectRef) {
        kinds::player::actions::open_counter_window(self, o);
    }

    fn check_reactive_abort(&mut self, o: ObjectRef) {
        kinds::player::actions::check_reactive_abort(self, o);
    }

    fn exit_attack(&mut self, o: ObjectRef) {
        kinds::player::exit_attack_state(self, o);
    }

    // ---- Sprites -------------------------------------------------------------------

    fn sprite_load(&mut self, o: ObjectRef, id: SpriteId) {
        self.objects.sprite_mut(o).load(id);
    }

    fn sprite_set_animation(&mut self, o: ObjectRef, anim: u8) {
        self.objects.sprite_mut(o).set_animation(anim, &self.content);
    }

    fn sprite_step(&mut self, o: ObjectRef) {
        self.objects.sprite_mut(o).update(&self.content);
    }

    fn sprite_get(&self, o: ObjectRef, f: SpriteField) -> Value {
        let look = &self.objects.sprite(o).look;
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

    fn collision_get(&self, o: ObjectRef, f: CollisionField) -> ApiResult<Value> {
        let c = self.collision_of(o)?;
        Ok(Value::Int(match f {
            CollisionField::Region => c.region as i64,
            CollisionField::HitEffect => c.hit_effect as i64,
            CollisionField::StatusBase => c.status_base as i64,
            CollisionField::Bugs => c.bugs as i64,
            CollisionField::HitFlags => c.acc.hit_flags as i64,
        }))
    }

    fn collision_set(&mut self, o: ObjectRef, f: CollisionField, v: Value) -> ApiResult<()> {
        let v = store(f.name(), f.writable(), f.ty(), v)?;
        let c = self.collision_of_mut(o)?;
        let x = int(v);
        match f {
            CollisionField::Region => c.region = x as u8,
            CollisionField::HitEffect => c.hit_effect = x as u8,
            CollisionField::StatusBase => c.status_base = x as u8,
            CollisionField::Bugs => c.bugs = x as u16,
            CollisionField::HitFlags => unreachable!("read-only"),
        }
        Ok(())
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
}
