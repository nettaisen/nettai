//! A content object's drive ([`Drive`]): what the engine runs in the
//! object's place of the update order instead of its kind's update. Each
//! step makes the calls the update would have made, in its order, through
//! the same API, so a drive changes nothing but who runs them.

use nettai_content_api::{CollisionField, CoreApi, Drive, Fly, Follow, Held, Lifecycle, ObjectField, ObjectRef, PanelPos, Sleep, SpriteField, Value, Vec3};

use crate::battle::Battle;

/// Run `r`'s drive for this tick: true if it ran, and the kind's update is
/// not to be called; false if the update runs (no drive, or the drive woke
/// it and ended). A drive runs in the object's update lifecycle alone: put
/// in another (something destroyed it), the update runs.
pub(crate) fn step(b: &mut Battle, r: ObjectRef) -> bool {
    let o = b.objects.get(r);
    if o.drive != Drive::None && o.state != crate::object::state::UPDATE {
        end(b, r);
        return false;
    }
    match b.objects.get(r).drive {
        Drive::None => false,
        Drive::Follow(f) => {
            follow(b, r, f);
            true
        }
        Drive::Fly(f) => fly(b, r, f),
        Drive::Sleep(s) => sleep(b, r, s),
    }
}

fn end(b: &mut Battle, r: ObjectRef) {
    b.objects.get_mut(r).drive = Drive::None;
}

fn follow(b: &mut Battle, me: ObjectRef, f: Follow) {
    let p = b.objects.get(f.target).pos;
    let at = Vec3 { x: p.x.wrapping_add(f.offset.x), y: p.y.wrapping_add(f.offset.y), z: p.z.wrapping_add(f.offset.z) };
    set(b, me, ObjectField::Pos, Value::Vec3(at));
    if f.visibility {
        for side in 0..2 {
            let v = CoreApi::get_for(b, f.target, ObjectField::Visible, side).expect("an object's visibility");
            CoreApi::set_for(b, me, ObjectField::Visible, side, v).expect("an object's visibility");
        }
    }
    for (i, &field) in SpriteField::ALL.iter().enumerate() {
        if f.looks & (1 << i) == 0 {
            continue;
        }
        let (side0, side1) = (CoreApi::sprite_get_for(b, f.target, field, 0), CoreApi::sprite_get_for(b, f.target, field, 1));
        if side0 == side1 {
            CoreApi::sprite_set(b, me, field, side0).expect("a sprite field the follower wears");
        } else {
            CoreApi::sprite_set_for(b, me, field, 0, side0).expect("a sprite field the follower wears");
            CoreApi::sprite_set_for(b, me, field, 1, side1).expect("a sprite field the follower wears");
        }
    }
    if !holds(b, f.target, f.held) {
        set(b, me, ObjectField::Visible, Value::Bool(false));
        CoreApi::set_lifecycle(b, me, Lifecycle::Destroy);
        end(b, me);
        return;
    }
    if !f.while_dimmed && CoreApi::is_dimmed(b) {
        return;
    }
    if CoreApi::is_paused(b) {
        return;
    }
    CoreApi::update_sprite(b, me);
}

/// Whether `target` still holds something where `held` says.
fn holds(b: &Battle, target: ObjectRef, held: Held) -> bool {
    match held {
        Held::Field(f) => !matches!(CoreApi::get(b, target, f).expect("an object field holding a follower"), Value::Nil),
        Held::Actor(f) => !matches!(CoreApi::actor_get(b, target, f).expect("a navi field holding a follower"), Value::Nil),
        Held::State { layout, field } => {
            let state = b.objects.state(target).filter(|s| s.id() == layout).expect("the state that holds a follower, in its layout");
            let schema = b.content.defs.schema(layout);
            !matches!(state.get(schema, field as usize), nettai_content_api::FieldValue::Object(None))
        }
    }
}

fn fly(b: &mut Battle, me: ObjectRef, f: Fly) -> bool {
    let o = b.objects.get(me);
    let (phase_init, timer, timer2, panel) = (o.phase_init, o.timer, o.timer2, o.panel);
    let facing = CoreApi::facing(b, me);
    let left = if phase_init == 0 { timer2 } else { timer } as i64 - 1;
    let next = PanelPos { x: (panel.x as i32 + facing) as u8, y: panel.y };
    let hit = CoreApi::collision_get(b, me, CollisionField::HitFlags).expect("a flying object's collision") != Value::Int(0);
    if CoreApi::is_battle_over(b) || hit || (left < 0 && !CoreApi::panel_valid(b, next)) {
        end(b, me);
        return false;
    }
    CoreApi::remove_collision(b, me);
    CoreApi::hit_spark(b, me);
    if phase_init == 0 {
        set(b, me, ObjectField::Timer, Value::Int(timer2 as i64));
        set(b, me, ObjectField::PhaseInit, Value::Int(4));
    }
    set(b, me, ObjectField::Timer, Value::Int(left));
    if left < 0 {
        set(b, me, ObjectField::PanelX, Value::Int(next.x as i64));
        set(b, me, ObjectField::PanelY, Value::Int(next.y as i64));
        CoreApi::set_coordinates_from_panel(b, me);
        if f.climbs {
            let p = b.objects.get(me).pos;
            set(b, me, ObjectField::Pos, Value::Vec3(Vec3 { x: p.x, y: p.y.wrapping_add(0x10000), z: p.z.wrapping_add(0x10000) }));
        }
        CoreApi::update_collision_panels(b, me);
        set(b, me, ObjectField::PhaseInit, Value::Int(0));
    }
    CoreApi::present_collision(b, me);
    CoreApi::update_sprite(b, me);
    true
}

fn sleep(b: &mut Battle, me: ObjectRef, s: Sleep) -> bool {
    let woken = s.ticks == 0
        || (s.wake_on_hit && CoreApi::collision_get(b, me, CollisionField::HitFlags).is_ok_and(|v| v != Value::Int(0)))
        || (s.wake_on_finish && CoreApi::sprite_get(b, me, SpriteField::Finished) == Value::Bool(true));
    if woken {
        end(b, me);
        return false;
    }
    b.objects.get_mut(me).drive = Drive::Sleep(Sleep { ticks: s.ticks - 1, ..s });
    if s.animate {
        CoreApi::update_sprite(b, me);
    }
    true
}

fn set(b: &mut Battle, o: ObjectRef, f: ObjectField, v: Value) {
    CoreApi::set(b, o, f, v).unwrap_or_else(|e| panic!("a drive's {}: {e}", f.name()));
}
