//! Rollback runs reproduce straight runs bit for bit, with every content
//! runtime.

use super::*;
use crate::content::Content;
use crate::scenario;

/// Whether GunDelSol landed hits (so the test exercised it).
fn hits_landed(b: &Battle) -> bool {
    (0..2).any(|side| b.player(side).is_some_and(|p| b.objects.get(p).hp < 1000))
}

/// Run a session over `tape` with the remote input arriving `delay`
/// frames late (the local input is on time).
fn run_session(content: Content, tape: &[scenario::Tick], delay: usize) -> Session {
    let mut s = Session::new(Battle::with_content(scenario::setup(), content), 0);
    s.record_digests();
    for (f, t) in tape.iter().enumerate() {
        s.advance(t.input[0].clone(), t.events.clone());
        if f + 1 >= delay {
            s.receive_remote(tape[f + 1 - delay].input[1].clone());
        }
    }
    for t in &tape[tape.len() + 1 - delay..] {
        s.receive_remote(t.input[1].clone());
    }
    s
}

/// Play the tape straight, then with the remote input late; every
/// confirmed frame's digest must match the straight run's.
fn check(content: impl Fn() -> Content, tape: &[scenario::Tick], delays: &[usize]) {
    let mut straight = Battle::with_content(scenario::setup(), content());
    let mut want = Vec::new();
    for t in tape {
        straight.tick(&t.input, t.events.clone());
        want.push(digest(&straight));
    }
    assert!(hits_landed(&straight), "the duel should have landed hits");
    for &delay in delays {
        let s = run_session(content(), tape, delay);
        let runtime = s.battle().content.runtime().to_string();
        assert_eq!(s.confirmed_digests().len(), want.len(), "{runtime}, delay {delay}: every frame confirmed");
        for (f, (have, want)) in s.confirmed_digests().iter().zip(&want).enumerate() {
            assert_eq!(have, want, "{runtime}, delay {delay}: frame {f} differs after rollback");
        }
        assert_eq!(digest(s.battle()), *want.last().unwrap(), "{runtime}, delay {delay}: final state");
        assert!(delay <= 1 || s.stats.rollbacks > 0, "{runtime}, delay {delay}: the remote side should mispredict");
    }
}

#[test]
fn rollback_reproduces_the_straight_run() {
    let tape = scenario::record(900);
    check(Content::builtin, &tape, &[1, 2, 5, 10]);
    #[cfg(feature = "rust-content")]
    check(Content::rust, &tape, &[2, 5, 10]);
    #[cfg(feature = "luau")]
    check(|| Content::luau().unwrap(), &tape, &[2, 5, 10]);
}

#[test]
fn resimulated_cues_are_not_played_twice() {
    let tape = scenario::record(900);
    let mut straight = Vec::new();
    let mut b = Battle::with_content(scenario::setup(), Content::for_build());
    for (f, t) in tape.iter().enumerate() {
        b.tick(&t.input, t.events.clone());
        straight.extend(b.sound_cues().iter().map(|&cue| (f as u32, cue)));
    }
    let delay = 6;
    let mut s = Session::new(Battle::with_content(scenario::setup(), Content::for_build()), 0);
    let mut played: Vec<(u32, SoundCue)> = Vec::new();
    let apply = |events: Vec<CueEvent>, played: &mut Vec<(u32, SoundCue)>| {
        for e in events {
            match e {
                CueEvent::Play { frame, cue } => played.push((frame, cue)),
                CueEvent::Retract { frame, cue } => {
                    let i = played.iter().position(|&p| p == (frame, cue)).expect("retracting a played cue");
                    played.remove(i);
                }
            }
        }
    };
    for (f, t) in tape.iter().enumerate() {
        s.advance(t.input[0].clone(), t.events.clone());
        if f + 1 >= delay {
            s.receive_remote(tape[f + 1 - delay].input[1].clone());
        }
        apply(s.take_cue_events(), &mut played);
    }
    for t in &tape[tape.len() + 1 - delay..] {
        s.receive_remote(t.input[1].clone());
    }
    apply(s.take_cue_events(), &mut played);
    // Within a frame, a cue replayed after a retraction comes last.
    let order = |v: &mut Vec<(u32, SoundCue)>| v.sort_by_key(|&(f, c)| (f, format!("{c:?}")));
    order(&mut played);
    order(&mut straight);
    assert!(s.stats.rollbacks > 0);
    assert!(straight.iter().any(|&(_, c)| c == SoundCue::Effect(crate::sound::SoundId(0xF9))), "the sun beam hummed");
    assert_eq!(played, straight, "after reconciling, each frame's cues were played exactly once");
}
