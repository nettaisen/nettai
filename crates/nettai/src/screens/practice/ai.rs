//! The dummy that fights (its Action Fights, at a Level): nettai-training's player on the network of the nettai-ai
//! repository ([`super::weights`]: downloaded when Fights is first picked, or `NETTAI_AI_WEIGHTS`'s file), plugged
//! into the practice battle's live play, with its chips' fingerprints made on threads of their own. The `ai`
//! feature's: a build without it has no Fights and no Level (`ACTIONS`, [`shown_rows`](super::shown_rows)).

use super::weights::Weights;
use super::{LEVEL_COUNT, Session};
use crate::screens::ScreenId;
use crate::shell::Shell;
use nettai_frontend::driver::{InFight, LivePlayer, OnCustomScreen, Opponent};
use nettai_match::Set;
use nettai_training::ai::{Arriving, Fingerprinter, Level, Status, Stopped, WaitingAi};
use nettai_ui::{GettingReady, PracticeView};
use slint::ComponentHandle;
use std::path::PathBuf;

/// How well the dummy fights (its Action Fights: the AI's), in the Level row's order; Normal at first.
pub(super) const LEVELS: [Level; LEVEL_COUNT] = [Level::Easy, Level::Normal, Level::Hard];

/// The AI plugged into the dummy's live play ([`plug`]): its level (its place in [`LEVELS`]), its network (on its
/// way, maybe), the chips' fingerprints it plays with and the most it was seen to be making, and why it stopped, if
/// it did; `out`, taken out since ([`watch`]: the dummy stands still).
pub(crate) struct Plugged {
    pub(crate) level: usize,
    pub(crate) net: Arriving,
    pub(crate) fingerprints: Fingerprinter,
    chips: usize,
    pub(crate) stopped: Stopped,
    out: bool,
}

/// The dummy's AI as its Action needs it: while it fights, nettai-training's `WaitingAi` at `level` (a new one for
/// another level) on the network `net` gives (asked for then: the app's [`Weights::wanted`]; till it arrives the
/// dummy waits, its custom screen held), its draws from `seed`, the fingerprints of its game's chips made on threads
/// of their own and kept in `cache` (the app's: [`fingerprints_folder`]), both builds' folders' chips asked for at
/// once, so they are made while the round opens; none otherwise. `plugged`: the one plugged in now, if any. What is
/// plugged in after. One whose network can't be had, or is weights this encoder can't play, stops: [`watch`] says
/// why in the log and takes it out, and the dummy stands still.
pub(crate) fn plug(live: &mut LivePlayer, opponent: Opponent, level: usize, seed: u32, plugged: Option<Plugged>, cache: Option<PathBuf>, net: impl FnOnce() -> Arriving) -> Option<Plugged> {
    let wanted = (opponent.in_fight == InFight::Ai || opponent.on_custom_screen == OnCustomScreen::Ai).then_some(level);
    if wanted == plugged.as_ref().map(|p| p.level) {
        return plugged;
    }
    let Some(level) = wanted else {
        live.set_ai(None);
        return None;
    };
    let content = live.set().content().clone();
    let fingerprints = Fingerprinter::start(&content, fingerprint_threads(), cache);
    fingerprints.want(nettai_training::ai::setup_chips(&content, live.set().first()));
    let net = net();
    let ai = WaitingAi::new(net.clone(), LEVELS[level], seed as u64, fingerprints.clone());
    let plugged = Plugged { level, net, fingerprints, chips: 0, stopped: ai.stopped(), out: false };
    live.set_ai(Some(Box::new(ai)));
    Some(plugged)
}

/// The session's dummy's AI as its Action and Level now have it, on `live` (the session's battle's), its network
/// the process's (`weights`).
pub(super) fn replug(live: &mut LivePlayer, session: &mut Session, weights: &mut Weights) {
    session.ai = plug(live, session.opponent, session.level, session.seed, session.ai.take(), Some(fingerprints_folder()), || weights.wanted());
}

/// A new set (another arena, or the dummy's build): the dummy's AI, if one is plugged in, has its chips'
/// fingerprints asked for at once (a build it hasn't met: made while the round opens).
pub(super) fn want_chips(session: &Session, set: &Set) {
    if let Some(ai) = &session.ai {
        ai.fingerprints.want(nettai_training::ai::setup_chips(set.content(), set.first()));
    }
}

/// Where the dummy's AI keeps its chips' fingerprints (by content: `fingerprints-<game>-<hash>.json`), so a chip
/// is made once: the data folder's `fingerprints` (in the browser, the page's kept folder).
fn fingerprints_folder() -> PathBuf {
    crate::paths::data().join("fingerprints")
}

/// The threads the dummy's chips' fingerprints are made on: a core left to the battle, and four at most (EXE5's
/// first wait about 3 s on four: docs/design/training-ai.md §10.3); four where the system doesn't say (a browser,
/// whose are its workers).
fn fingerprint_threads() -> usize {
    std::thread::available_parallelism().map_or(4, |n| n.get().saturating_sub(1)).clamp(1, 4)
}

/// The dummy's AI as it goes, after the frame's ticks: one that stopped (its network couldn't be had or played,
/// its fingerprints couldn't be made, or it had nothing it could do) is said in the log and taken out, the dummy
/// standing still; and while its network is on its way or its chips' fingerprints are being made, the corner over
/// the battle says it is getting ready, with how many of those are done (the network one of them:
/// `PracticeView.getting-ready`, told as it changes).
pub(super) fn watch(shell: &mut Shell) {
    let Some(session) = shell.practice.session.as_mut() else { return };
    let mut ready = GettingReady::default();
    let mut take_out = false;
    if let Some(ai) = session.ai.as_mut().filter(|ai| !ai.out) {
        let failed = match ai.net.get() {
            Some(Err(why)) => Some(why.clone()),
            _ => ai.stopped.why(),
        };
        if let Some(why) = failed {
            crate::errors::log("the dummy's AI", &why);
            ai.out = true;
            take_out = true;
        } else {
            let (done, of) = match ai.fingerprints.status() {
                Status::Making { done, of } => {
                    ai.chips = ai.chips.max(of);
                    (done, of)
                }
                _ => (ai.chips, ai.chips),
            };
            let arrived = ai.net.get().is_some();
            if done < of || !arrived {
                ready = GettingReady { shown: true, made: (done + arrived as usize) as i32, asked: (of + 1) as i32 };
            }
        }
    }
    let told = (session.getting_ready != ready).then(|| {
        session.getting_ready = ready.clone();
        ready
    });
    if take_out
        && let Some(b) = shell.battle.as_mut().filter(|b| b.owner == ScreenId::Practice)
        && let Some(live) = b.stage.player.live_mut()
    {
        live.set_ai(None);
    }
    if let Some(ready) = told {
        shell.ui().global::<PracticeView>().set_getting_ready(ready);
    }
}

#[cfg(test)]
mod tests {
    use super::super::NORMAL;
    use super::*;

    /// nettai-training's fixture network (small, its parameters random), arrived.
    pub(crate) fn fixture() -> Arriving {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../nettai-training/tests/fixtures/net.safetensors");
        Arriving::arrived(nettai_training::net::Net::from_safetensors(&std::fs::read(path).unwrap()).map(std::sync::Arc::new))
    }

    /// The Level row's values, in its order.
    #[test]
    fn the_levels_are_easy_normal_and_hard() {
        assert_eq!(LEVELS, [Level::Easy, Level::Normal, Level::Hard]);
        assert_eq!(LEVELS[NORMAL], Level::Normal, "Normal at first");
    }

    /// The dummy's AI is plugged in while its Action or its custom screen is the AI's, at the level chosen (a
    /// new one for another level), and taken out otherwise; its network is asked for only as one is plugged in.
    #[test]
    fn the_dummys_ai_is_plugged_in_while_its_choices_are_the_ais() {
        let content = nettai_match::testing::exe6_content();
        let m = nettai_match::pick::live(&content, "exe6", 3, None).unwrap();
        let mut live = LivePlayer::new(Set::of(&content, &m, 3));
        let still = Opponent::default();
        let fights = Opponent { in_fight: InFight::Ai, ..still };
        let picks = Opponent { on_custom_screen: OnCustomScreen::Ai, ..still };
        let level = |p: &Option<Plugged>| p.as_ref().map(|p| p.level);
        let unasked = || -> Arriving { panic!("a network asked for with no AI") };
        let plugged = plug(&mut live, still, NORMAL, 3, None, None, unasked);
        assert_eq!(level(&plugged), None);
        let plugged = plug(&mut live, fights, NORMAL, 3, plugged, None, fixture);
        assert_eq!(level(&plugged), Some(NORMAL));
        // (Both builds' folders' chips asked for as it is plugged in.)
        let asked = |p: &Option<Plugged>| match p.as_ref().unwrap().fingerprints.status() {
            Status::Making { of, .. } => of,
            Status::Ready => nettai_training::ai::setup_chips(&content, live.set().first()).len(),
            other => panic!("{other:?}"),
        };
        assert!(asked(&plugged) > 10, "{}", asked(&plugged));
        let plugged = plug(&mut live, fights, 2, 3, plugged, None, fixture);
        assert_eq!(level(&plugged), Some(2), "another level: another AI");
        let plugged = plug(&mut live, picks, 2, 3, plugged, None, unasked);
        assert_eq!(level(&plugged), Some(2), "the custom screen alone");
        let plugged = plug(&mut live, still, 2, 3, plugged, None, unasked);
        assert_eq!(level(&plugged), None, "taken out");
    }
}
