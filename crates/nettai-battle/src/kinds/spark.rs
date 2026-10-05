//! Hit sparks (effect object #4), spawned where an attack connects.

use crate::battle::Battle;
use crate::content::EffectSprite;
use crate::object::{ObjectRef, Vec3, flags, sprite::FRAME_LAST, state};
use nettai_content_api::SparkHandle;

/// `AddRandomVarianceToTwoCoords`: jitter x and z by up to ±mask/2 pixels
/// (one simulation RNG draw).
pub fn jitter(b: &mut Battle, mask: u32, mut pos: Vec3) -> Vec3 {
    let r = b.rng.next();
    pos.x = pos.x.wrapping_add((((r & mask) as i32) - (mask >> 1) as i32) << 16);
    pos.z = pos.z.wrapping_add(((((r >> 16) & mask) as i32) - (mask >> 1) as i32) << 16);
    pos
}

/// `object_spawnCollisionEffect`: a hitter that touched a body or object,
/// unguarded, shows its hit effect.
pub fn spawn_collision_effect(b: &mut Battle, hitter: ObjectRef) {
    let Some(c) = b.objects.get(hitter).collision else { return };
    let s = b.collision.get(c);
    let flags = s.acc.hit_flags;
    if flags & 0x3F80_0000 == 0 || flags & 1 != 0 {
        return;
    }
    let Some(effect) = s.hit_effect else { return };
    let pos = jitter(b, 0xF, b.objects.get(hitter).pos);
    spawn(b, hitter, pos, effect);
}

/// Spark-private state.
#[derive(Clone, Debug, Default, Hash)]
pub struct Vars {
    /// What it shows (the original's first spawn parameter, a row of its
    /// table of looks).
    pub look: Option<SparkHandle>,
}

/// `sub_80E08C4`.
pub fn spawn(b: &mut Battle, owner: ObjectRef, pos: Vec3, look: SparkHandle) -> Option<ObjectRef> {
    let r = crate::kinds::spawn_engine(b, crate::kinds::EngineKind::Spark, pos, [0; 4])?;
    let alliance = b.objects.get(owner).alliance;
    let o = b.objects.get_mut(r);
    o.vars = crate::kinds::Vars::Spark(Vars { look: Some(look) });
    o.related[0] = Some(owner);
    o.alliance = alliance;
    Some(r)
}

pub fn update(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).state {
        state::INIT => {
            let look = match &b.objects.get(r).vars {
                crate::kinds::Vars::Spark(v) => v.look,
                _ => None,
            };
            let EffectSprite { sprite: id, anim, palette } = b.content.spark(look.expect("a hit spark spawned with its look"));
            let steps = b.game_rules().effects.spark_steps_at_start;
            let sprite = b.objects.sprite_mut(r);
            sprite.load(id);
            sprite.set_animation(anim, &b.content);
            sprite.look.shadow = crate::object::sprite::Shadow::WithSprite;
            sprite.look.palette = palette;
            // (EXE5's spark, 0x080E0870, doesn't step at its start: the
            // arena's `effects` section.)
            if steps {
                sprite.update(&b.content);
            }
            let o = b.objects.get_mut(r);
            o.flags &= !flags::NO_SPRITE_UPDATE;
            o.anim = anim;
            o.anim_loaded = anim;
            o.set_visible(true);
            o.state = state::UPDATE;
            o.action = 0;
            o.phase = 0;
            o.phase_init = 0;
        }
        state::UPDATE => {
            if b.objects.sprite(r).frame_parameters() & FRAME_LAST != 0 {
                let o = b.objects.get_mut(r);
                o.set_visible(false);
                o.state = state::DESTROY;
                o.action = 0;
                o.phase = 0;
                o.phase_init = 0;
            }
            b.objects.sprite_mut(r).update(&b.content);
        }
        _ => b.objects.free(r),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{Content, SparkRole, testing};
    use std::sync::Arc;

    /// Ticks a plain hit spark lives, in an arena whose spark steps at its
    /// start or not.
    fn lifetime(steps: bool) -> u32 {
        let mut c: Content = testing::build();
        c.define().unwrap_or_else(|e| panic!("{e}"));
        {
            let rules = c.rules_mut();
            rules.effects.spark_steps_at_start = steps;
        }
        let c = Arc::new(c);
        let mut setup = testing::round_setup(testing::LINK_BATTLE, testing::megaman_on(&c));
        setup.content = c.hash();
        let mut b = Battle::new(setup, c);
        b.spawn_actors();
        b.run_objects();
        let navi = b.player(0).unwrap();
        let look = b.roles().spark(SparkRole::Plain);
        let r = spawn(&mut b, navi, Vec3::default(), look).expect("a spark");
        let mut n = 0;
        while b.objects.in_order().any(|o| o == r) {
            update(&mut b, r);
            n += 1;
            assert!(n < 1000, "the spark never goes");
        }
        n
    }

    /// docs/design/exe5-map.md §15.3 item 16: EXE5's spark (the arena's
    /// `effects`) doesn't step as it starts, so it lives a tick longer.
    #[test]
    fn exe5s_spark_lives_a_tick_longer() {
        assert_eq!(lifetime(false), lifetime(true) + 1);
    }
}
