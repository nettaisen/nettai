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

/// Side 0's navi's panel turned to `kind`, then 10 ticks.
fn panel_under(b: &mut Battle, kind: &str) {
    let c = b.objects.get(b.player(0).unwrap()).collision.expect("the navi's collision");
    let p = b.collision.get(c).panel;
    let t = b.content.rules().panels.named(kind).unwrap_or_else(|| panic!("EXE5's {kind}"));
    b.set_panel_type(p.x, p.y, t);
    for _ in 0..10 {
        b.tick(&Default::default(), Default::default());
    }
}

fn sea_under(b: &mut Battle) {
    panel_under(b, "sea");
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
    // (Switched out from under the surface, on the sea.)
    switch_to(&mut b, 0);
    assert_eq!(navi_status(&b) & status::DIVES, 0, "MegaMan doesn't dive");
    sea_under(&mut b);
    assert_eq!(flags(&b) & f1::BUBBLED, 0, "MegaMan stands on the sea");
    let ripple = b.content.defs.kind_by_key("dive-ripple").expect("EXE5's dive ripple");
    assert!(b.objects.in_order().all(|q| b.objects.get(q).kind != ripple), "and no ripple is left over him");
}

/// The navi `navi` starting the battle (side 0's own navi), fighting.
fn starting_as(content: &std::sync::Arc<nettai_battle::Content>, navi: &str) -> Battle {
    let megaman = crate::double_attacks::side(&[]);
    let left = megaman.replacen("navi = \"megaman\"", &format!("navi = \"{navi}\""), 1).replace("party = []\n", "");
    fighting_sides(content, &left, &megaman)
}

/// A round of the sides `left` and `right` (a match file's), both pressing
/// through their custom screens until the fight runs.
fn fighting_sides(content: &std::sync::Arc<nettai_battle::Content>, left: &str, right: &str) -> Battle {
    let stage = crate::ids::local(&content.defs.stage(crate::link_battle_stages(content, "exe5")[0]).key);
    let text = format!("game = \"exe5\"\n\n[[round]]\nstage = \"{stage}\"\n\n[left]\n{left}\n\n[right]\n{right}\n");
    let m = crate::parse(content, &text).unwrap_or_else(|p| panic!("{p:?}"));
    let mut b = Battle::new(m.round(content, 0x5EED), content.clone());
    let mut last = [0u16; 2];
    for _ in 0..3000 {
        if b.round.mode == nettai_battle::battle::mode::FIGHTING && b.fight.state == nettai_battle::battle::fight::FIGHTING {
            return b;
        }
        let input = [0, 1].map(|side| {
            last[side] = nettai_battle::scenario::custom_buttons(&b, side, last[side]);
            PlayerTick::buttons(last[side])
        });
        b.tick(&input, Default::default());
    }
    panic!("the fight never started");
}

/// The navi's state as it stands, beside what a switch leaves on it (its
/// status bit `SWITCHED`): its identity, sprite look, collision, statuses,
/// weapons and element, and the objects it owns.
fn state(b: &Battle) -> Vec<String> {
    let r = b.player(0).unwrap();
    let o = b.objects.get(r);
    let a = b.actors.get(o.actor.unwrap());
    let owned = b
        .objects
        .in_order()
        .filter(|&q| q != r && b.objects.get(q).related.contains(&Some(r)))
        .map(|q| b.content.defs.kind(b.objects.get(q).kind).key.to_string())
        .collect::<Vec<_>>();
    vec![
        format!("identity {:?}", o.identity),
        format!("look {:?}", b.objects.sprite(r).look),
        format!("collision {:?}", b.collision.get(o.collision.unwrap())),
        format!("status {:#x}", a.status & !status::SWITCHED),
        format!("weapons {:?}", (a.buster, a.charge_shot, a.a_charge, a.alt_a_charge, a.back_special, a.ai_index, a.identity)),
        format!("element {}", b.stats[0].element),
        format!("owned {owned:?}"),
    ]
}

/// The script both battles play from where they are compared: tap B (the
/// buster), hold B to a full charge and let go (the charge shot), step up
/// and back, tap A (the hand's first chip): each tick's buttons.
fn script() -> Vec<u16> {
    use nettai_battle::input::keys;
    let mut s = Vec::new();
    let hold = |key: u16, ticks: usize, s: &mut Vec<u16>| {
        s.extend(std::iter::repeat_n(key, ticks));
        s.extend(std::iter::repeat_n(0, 40));
    };
    hold(keys::B, 2, &mut s);
    hold(keys::B, 240, &mut s);
    s.extend(std::iter::repeat_n(0, 120));
    hold(keys::UP, 2, &mut s);
    hold(keys::DOWN, 2, &mut s);
    hold(keys::A, 2, &mut s);
    s.extend(std::iter::repeat_n(0, 200));
    s
}

/// What a navi did through the script: the kinds it spawned (in order of
/// first appearance), its sounds and the ticks the opponent was hit on.
/// (Not the damage: a party's team navi deals the DS's, by its party's
/// level, lib/navi_level.)
fn plays(b: &mut Battle) -> (Vec<String>, Vec<String>, Vec<usize>) {
    let opponent = |b: &Battle| b.objects.get(b.player(1).unwrap()).hp;
    let mut last = opponent(b);
    let (mut kinds, mut sounds, mut hits) = (Vec::new(), Vec::new(), Vec::new());
    for (i, held) in script().into_iter().enumerate() {
        b.tick(&[PlayerTick::buttons(held), Default::default()], Default::default());
        for q in b.objects.in_order() {
            let key = b.content.defs.kind(b.objects.get(q).kind).key.to_string();
            if !kinds.contains(&key) {
                kinds.push(key);
            }
        }
        sounds.extend(b.sound_cues_for(0).iter().map(|c| format!("{c:?}")));
        if opponent(b) != last {
            hits.push(i);
            last = opponent(b);
        }
    }
    (kinds, sounds, hits)
}

const TEAM_NAVIS: [&str; 12] = [
    "protoman", "gyroman", "knightman", "magnetman", "meddy", "napalmman", "numberman", "searchman", "shadowman", "toadman",
    "tomahawkman", "colonel",
];

/// Each team navi switched in mid-fight is the navi it is when it starts
/// the battle (its look, collision, statuses, weapons and the objects it
/// wears), plays as it does (the same buster, charge shot, steps and chip:
/// the same objects, sounds and hits), and takes the sea and a broken
/// panel under it as it does; switched in from another team navi, too.
/// MegaMan switched back in is MegaMan as he started.
#[test]
fn a_team_navi_switched_in_is_as_at_the_start() {
    let content = exe5_content();
    let mut wrong = Vec::new();
    let mut megaman = fighting(&content, &[]);
    for _ in 0..162 {
        megaman.tick(&Default::default(), Default::default());
    }
    let mut megaman_on_broken = megaman.clone();
    panel_under(&mut megaman_on_broken, "broken");
    for _ in 0..81 {
        megaman_on_broken.tick(&Default::default(), Default::default());
    }
    for navi in TEAM_NAVIS {
        let mut a = starting_as(&content, navi);
        for _ in 0..81 {
            a.tick(&Default::default(), Default::default());
        }
        let mut s = fighting(&content, &[navi]);
        switch_to(&mut s, 1);
        if state(&a) != state(&s) {
            wrong.push(format!("{navi}'s state:\n  at the start {:?}\n  switched in  {:?}", state(&a), state(&s)));
        }
        let (at_start, switched) = (plays(&mut a), plays(&mut s));
        if at_start != switched {
            wrong.push(format!("{navi}'s play:\n  at the start {at_start:?}\n  switched in  {switched:?}"));
        }
        // What the panel under it does to it (ToadMan dives in the sea,
        // GyroMan hovers over a broken panel).
        for kind in ["sea", "broken", "normal"] {
            panel_under(&mut a, kind);
            panel_under(&mut s, kind);
            if state(&a) != state(&s) {
                wrong.push(format!("{navi} on {kind}:\n  at the start {:?}\n  switched in  {:?}", state(&a), state(&s)));
            }
        }
        // From another team navi (the next in the list) rather than MegaMan.
        let other = TEAM_NAVIS[(TEAM_NAVIS.iter().position(|&n| n == navi).unwrap() + 1) % TEAM_NAVIS.len()];
        let mut from_other = fighting(&content, &[other, navi]);
        switch_to(&mut from_other, 1);
        switch_to(&mut from_other, 2);
        let mut a = starting_as(&content, navi);
        for _ in 0..161 {
            a.tick(&Default::default(), Default::default());
        }
        if state(&a) != state(&from_other) {
            wrong.push(format!("{navi} from {other}:\n  at the start {:?}\n  switched in  {:?}", state(&a), state(&from_other)));
        }
        let mut back = fighting(&content, &[navi]);
        switch_to(&mut back, 1);
        switch_to(&mut back, 0);
        if state(&back) != state(&megaman) {
            wrong.push(format!("MegaMan back from {navi}:\n  at the start {:?}\n  back  {:?}", state(&megaman), state(&back)));
        }
        // Switched out over a broken panel (GyroMan hovering).
        let mut back = fighting(&content, &[navi]);
        switch_to(&mut back, 1);
        panel_under(&mut back, "broken");
        switch_to(&mut back, 0);
        if state(&back) != state(&megaman_on_broken) {
            wrong.push(format!(
                "MegaMan back from {navi} on a broken panel:\n  standing there {:?}\n  back  {:?}",
                state(&megaman_on_broken),
                state(&back)
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// GyroMan switched in hovers over a broken panel (his `tick`, 0x080F0A10),
/// as he does when he starts the battle; MegaMan switched in there doesn't
/// (the settle clears the bit, 0x0202C8F8).
#[test]
fn gyroman_switched_in_hovers_over_a_broken_panel() {
    let content = exe5_content();
    let mut b = fighting(&content, &["gyroman"]);
    switch_to(&mut b, 1);
    panel_under(&mut b, "broken");
    assert_ne!(navi_status(&b) & status::HOVERING, 0, "GyroMan hovers");
    switch_to(&mut b, 0);
    assert_eq!(navi_status(&b) & status::HOVERING, 0, "MegaMan doesn't");
}

/// A dark MegaMan switches in battle as a light one does: EXE5 DS's switch
/// paths read no emotion and no light/dark value (the input handler's
/// request, 0x0203C5F6, and 0x0200E064; the bottom screen's panel,
/// 0x02036F80; action 0x59, 0x0202C67C), only its Double Attacks and
/// supports do. MegaMan of a dark value (evil from his start: mood 0) uses
/// DarkPlus (a dark chip that doesn't dim, its cost the step bug), is still
/// evil, switches to ProtoMan, and ProtoMan back to him.
#[test]
fn a_dark_megaman_switches() {
    use nettai_battle::input::keys;
    use nettai_battle::kinds::player::emotion_name;
    let content = exe5_content();
    let left = crate::double_attacks::side(&["protoman"]) + "karma = 0\n";
    let mut b = fighting_sides(&content, &left, &crate::double_attacks::side(&[]));
    assert_eq!(emotion_name(&b, 0), "evil", "a dark value starts him evil");
    b.hands[0].ids[0] = crate::ids::chip(&content, "exe5", "darkplus");
    b.hands[0].cursor = 0;
    b.tick(&[PlayerTick::buttons(keys::A), Default::default()], Default::default());
    for _ in 0..60 {
        b.tick(&Default::default(), Default::default());
    }
    assert_eq!((b.hands[0].cursor, b.stats[0].bugs.processing), (1, 1), "DarkPlus used, its step bug his");
    assert_eq!(emotion_name(&b, 0), "evil");
    let ask = |b: &mut Battle, member: u8| {
        b.tick(&[PlayerTick { switch_to: Some(member), ..Default::default() }, Default::default()], Default::default());
        for _ in 0..80 {
            b.tick(&Default::default(), Default::default());
        }
    };
    ask(&mut b, 1);
    assert_eq!((b.party[0].active, b.party[0].switches), (1, 1), "the dark MegaMan switched to ProtoMan");
    ask(&mut b, 0);
    assert_eq!((b.party[0].active, b.party[0].switches), (0, 2), "and back to him");
    assert_eq!(emotion_name(&b, 0), "evil", "his mood kept in his block");
}

/// A switch to a member whose support acts (EXE5 DS's busy flag, the
/// party block's +0xC + member, 0x02004B80) is refused in battle with the
/// refusal's sound on both consoles (0x0203C62E: the navi's input, which
/// each console runs). Benched NapalmMan with P.Spport comes out to fire;
/// asked for meanwhile, he isn't switched in.
#[test]
fn a_switch_to_a_supporting_member_is_refused() {
    use nettai_battle::content::SoundRole;
    let content = exe5_content();
    let megaman = crate::double_attacks::side(&[]);
    let left = megaman.replace("party = []\n", "party = [{ navi = \"napalmman\", programs = [\"p-spport\"] }]\n");
    let mut b = fighting_sides(&content, &left, &megaman);
    let support = b.content.defs.kind_by_key("napalmman/support").expect("NapalmMan's support");
    let out = |b: &Battle| b.objects.in_order().any(|q| b.objects.get(q).kind == support);
    for _ in 0..600 {
        if out(&b) && !b.is_dimmed() {
            break;
        }
        b.tick(&Default::default(), Default::default());
    }
    assert!(out(&b), "NapalmMan came out to support");
    b.tick(&[PlayerTick { switch_to: Some(1), ..Default::default() }, Default::default()], Default::default());
    let refused = nettai_battle::SoundCue::Effect(b.roles().sound(SoundRole::Refused));
    assert!(out(&b), "still out as the request came");
    assert!([0, 1].iter().all(|&s| b.sound_cues_for(s).contains(&refused)), "the refusal heard on both consoles");
    for _ in 0..80 {
        b.tick(&Default::default(), Default::default());
    }
    assert_eq!((b.party[0].active, b.party[0].switches), (0, 0), "no switch");
}
