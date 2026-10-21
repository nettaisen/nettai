//! A team navi switched in mid-fight (EXE5's party battle: the DS's action
//! 0x59) is the navi it is when it starts the battle: what its init, its
//! hooks and its switch's tail give it.

use crate::double_attacks::fighting;
use crate::testing::exe5_content;
use nettai_battle::Battle;
use nettai_battle::actor::status;
use nettai_battle::collision::f1;
use nettai_battle::input::PlayerTick;

/// Side 0 asks to switch to `member` (out of the normal mood: no Double
/// Attack), then 80 ticks of nothing: the switch lands and settles.
fn switch_to(b: &mut Battle, member: u8) {
    b.stats[0].mood = 0x80;
    let ask = PlayerTick { switch_to: Some(member), ..Default::default() };
    b.tick(&[ask, Default::default()], Default::default());
    for _ in 0..80 {
        b.tick(&Default::default(), Default::default());
    }
    assert_eq!(b.party[0].active, member, "the switch landed");
}

fn navi_status(b: &Battle) -> u32 {
    let a = b.objects.get(b.player(0).unwrap()).actor.expect("the navi's actor");
    b.actors.get(a).status
}

fn flags(b: &Battle) -> u32 {
    let c = b.objects.get(b.player(0).unwrap()).collision.expect("the navi's collision");
    b.collision.get(c).f1
}

/// Side 0's navi's panel turned to sea, then 10 ticks.
fn sea_under(b: &mut Battle) {
    let c = b.objects.get(b.player(0).unwrap()).collision.expect("the navi's collision");
    let p = b.collision.get(c).panel;
    let sea = b.content.rules().panels.named("sea").expect("EXE5's sea");
    b.set_panel_type(p.x, p.y, sea);
    for _ in 0..10 {
        b.tick(&Default::default(), Default::default());
    }
}

/// ToadMan dives (his post-init hook; the switch's tail clears the bit and
/// his entry in its table sets it again, 0x080119A0, the DS's 0x0206E994):
/// switched in, he goes under on sea, as he does when he starts the battle
/// (the DS's G-OTOSHI recording, frame 852). MegaMan switched back in
/// doesn't.
#[test]
fn toadman_switched_in_dives_on_sea() {
    let content = exe5_content();
    let mut b = fighting(&content, &["toadman"]);
    switch_to(&mut b, 1);
    assert_ne!(navi_status(&b) & status::DIVES, 0, "ToadMan dives");
    sea_under(&mut b);
    assert_ne!(flags(&b) & f1::BUBBLED, 0, "under the sea's surface");
    switch_to(&mut b, 0);
    assert_eq!(navi_status(&b) & status::DIVES, 0, "MegaMan doesn't dive");
    sea_under(&mut b);
    assert_eq!(flags(&b) & f1::BUBBLED, 0, "MegaMan stands on the sea");
}
