//! The battle-intro sequencer (effect #2): fades the screen in, then waits
//! for the navis waiting to appear. Its progress bits gate the intro.
//! See docs/engine/objects-and-player.md §A.4.

use crate::battle::{Battle, FadeMode};
use crate::object::{ObjectRef, Pool, Vec3, flags, state};

#[derive(Clone, Debug, Default, Hash)]
pub struct Vars;

/// Spawn the sequencer (`sub_80E06F8`), before the navis.
pub fn spawn(b: &mut Battle) {
    // The spawn position is whatever the caller's registers held; it is
    // never read.
    if let Some(r) = b.objects.spawn(Pool::Effect, 2, Vec3::default(), [2, 0, 0, 0]) {
        b.objects.get_mut(r).flags |= flags::RUN_WHILE_PAUSED;
    }
}

pub fn update(b: &mut Battle, r: ObjectRef) {
    if b.objects.get(r).state == state::INIT {
        let o = b.objects.get_mut(r);
        o.flags |= flags::NO_SPRITE_UPDATE;
        o.state = state::UPDATE;
    }
    match b.objects.get(r).phase {
        0 => {
            if b.objects.get(r).phase_init == 0 {
                // sub_80E0684: the first battle of a set clears from white,
                // later ones from black.
                b.round.intro_bits |= 0x10;
                let s = &b.setup.settings;
                let later = if s.effects & crate::setup::effects::SET != 0 {
                    b.round.round > 1
                } else {
                    s.battle_number >= 2
                };
                let mode = if later { FadeMode::IntroFromBlack } else { FadeMode::IntroFromWhite };
                b.fade.start(mode, 0x10);
                b.objects.get_mut(r).phase_init = 4;
            }
            if !b.fade.active() {
                b.round.intro_bits |= 0x01;
                let o = b.objects.get_mut(r);
                o.phase = 4;
                o.phase_init = 0;
            }
        }
        _ => {
            if b.fadein_queue.iter().all(|e| e.is_none()) {
                b.round.intro_bits |= 0x02;
                b.objects.free(r);
            }
        }
    }
}
