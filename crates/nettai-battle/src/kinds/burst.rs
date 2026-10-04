//! The burst (effect object #0x90, `sub_80EA364`; BN5 has the same):
//! spawned around a navi (`sub_80EA438`; BN6's as the navi vanishes into
//! Beast Over, `battle.burst`), it waits 31 ticks, then every 8 ticks sets
//! off the role effect `burst` (BN6's #0x68) on a panel diagonal to the
//! navi's, four in all. Invisible itself.

use crate::battle::Battle;
use crate::kinds::common::{self, Progress};
use crate::kinds::effect;
use crate::object::{ObjectRef, Vec3, flags, state};

/// The panels it bursts on, from the last to the first (`byte_80EA418`:
/// dx toward the navi's front, dy).
const DIAGONALS: [(i32, i32); 4] = [(-1, -1), (1, 1), (-1, 1), (1, -1)];

/// Burst-private state (ExtraVars+0 and +4).
#[derive(Clone, Copy, Debug, Default, Hash)]
pub struct Vars {
    /// Ticks to the next burst.
    pub timer: u16,
    /// Bursts left.
    pub left: u8,
}

fn vars(b: &mut Battle, r: ObjectRef) -> &mut Vars {
    match &mut b.objects.get_mut(r).vars {
        crate::kinds::Vars::Burst(v) => v,
        v => panic!("burst with {v:?}"),
    }
}

/// `sub_80EA438`: a burst around `navi`'s panel. It runs while paused.
pub fn spawn(b: &mut Battle, navi: ObjectRef) -> Option<ObjectRef> {
    let r = crate::kinds::spawn_engine(b, crate::kinds::EngineKind::Burst, Vec3::default(), [0; 4])?;
    let (alliance, flip, panel) = {
        let n = b.objects.get(navi);
        (n.alliance, n.flip, n.panel)
    };
    let o = b.objects.get_mut(r);
    o.related[0] = Some(navi);
    o.alliance = alliance;
    o.flip = flip;
    o.panel = panel;
    o.flags |= flags::RUN_WHILE_PAUSED;
    Some(r)
}

pub fn update(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).state {
        state::INIT => {
            // sub_80EA384
            b.objects.get_mut(r).set_visible(false);
            *vars(b, r) = Vars { timer: 0x1F, left: 4 };
            common::set_progress(b, r, Progress::UPDATE);
            tick(b, r);
        }
        state::UPDATE => tick(b, r),
        _ => b.objects.free(r),
    }
}

/// `sub_80EA3A4`.
fn tick(b: &mut Battle, r: ObjectRef) {
    let v = vars(b, r);
    let t = v.timer as i32 - 1;
    v.timer = t as u16;
    if t > 0 {
        return;
    }
    let left = v.left as i32 - 1;
    v.left = left as u8;
    if left < 0 {
        return common::set_progress(b, r, Progress::DESTROY);
    }
    v.timer = 8;
    let (dx, dy) = DIAGONALS[v.left as usize];
    let o = b.objects.get(r);
    let front = common::facing(o.alliance, o.flip);
    let x = o.panel.x as i32 + front * dx;
    // A burst below the field is kept on its last row.
    let y = match o.panel.y as i32 + dy {
        4 => 3,
        y => y,
    };
    let (px, py) = crate::kinds::player::panel_coordinates(x as u8, y as u8);
    let look = b.roles().effect(crate::content::EffectRole::Burst);
    if let Some(e) = effect::spawn(b, Vec3 { x: px, y: py, z: 0 }, look, 0, 0, 0) {
        b.objects.get_mut(e).flags |= flags::RUN_WHILE_PAUSED;
        b.sound(crate::content::SoundRole::Burst);
    }
}
