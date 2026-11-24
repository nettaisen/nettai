//! The dummy that fights (its Action Fights, at Easy / Normal / Hard): `nettai_ai::Fighter`,
//! plugged directly into the practice battle's live play (`LivePlayer::set_ai`).

use super::{LEVEL_COUNT, Session};
use nettai_ai::{Fighter, Level};
use nettai_frontend::driver::{InFight, LivePlayer, OnCustomScreen, Opponent};

/// How well the dummy fights (its Action Fights: the AI's), in the Level row's order; Normal at first.
pub(super) const LEVELS: [Level; LEVEL_COUNT] = [Level::Easy, Level::Normal, Level::Hard];

/// The AI plugged into the dummy's live play ([`plug`]): its level (its place in [`LEVELS`]).
pub(crate) struct Plugged {
    pub(crate) level: usize,
}

/// The dummy's AI as its Action needs it: while it fights, `nettai_ai::Fighter` at `level`
/// (a new one when the level changes), its draws seeded from `seed`; none otherwise.
pub(crate) fn plug(live: &mut LivePlayer, opponent: Opponent, level: usize, seed: u32, plugged: Option<Plugged>) -> Option<Plugged> {
    let wanted = (opponent.in_fight == InFight::Ai || opponent.on_custom_screen == OnCustomScreen::Ai).then_some(level);
    if wanted == plugged.as_ref().map(|p| p.level) {
        return plugged;
    }
    let Some(level) = wanted else {
        live.set_ai(None);
        return None;
    };
    // In practice mode, only the player opens the custom screen.
    let ai = Fighter::new(LEVELS[level], seed as u64).with_open_custom(false);
    live.set_ai(Some(Box::new(ai)));
    Some(Plugged { level })
}

/// The session's dummy's AI as its Action and Level now have it, on `live`.
pub(super) fn replug(live: &mut LivePlayer, session: &mut Session) {
    session.ai = plug(live, session.opponent, session.level, session.seed, session.ai.take());
}

#[cfg(test)]
mod tests {
    use super::super::NORMAL;
    use super::*;
    use nettai_match::Set;

    /// The Level row's values, in its order.
    #[test]
    fn the_levels_are_easy_normal_and_hard() {
        assert_eq!(LEVELS, [Level::Easy, Level::Normal, Level::Hard]);
        assert_eq!(LEVELS[NORMAL], Level::Normal, "Normal at first");
    }

    /// The dummy's AI is plugged in while its Action or its custom screen is the AI's, at the level chosen
    /// (a new one for another level), and taken out otherwise.
    #[test]
    fn the_dummys_ai_is_plugged_in_while_its_choices_are_the_ais() {
        let content = nettai_match::testing::exe6_content();
        let m = nettai_match::pick::live(&content, "exe6", 3, None).unwrap();
        let mut live = LivePlayer::new(Set::of(&content, &m, 3));
        let still = Opponent::default();
        let fights = Opponent { in_fight: InFight::Ai, ..still };
        let picks = Opponent { on_custom_screen: OnCustomScreen::Ai, ..still };
        let level = |p: &Option<Plugged>| p.as_ref().map(|p| p.level);

        let plugged = plug(&mut live, still, NORMAL, 3, None);
        assert_eq!(level(&plugged), None);
        let plugged = plug(&mut live, fights, NORMAL, 3, plugged);
        assert_eq!(level(&plugged), Some(NORMAL));
        let plugged = plug(&mut live, fights, 2, 3, plugged);
        assert_eq!(level(&plugged), Some(2), "another level: another AI");
        let plugged = plug(&mut live, picks, 2, 3, plugged);
        assert_eq!(level(&plugged), Some(2), "the custom screen alone");
        let plugged = plug(&mut live, still, 2, 3, plugged);
        assert_eq!(level(&plugged), None, "taken out");
    }
}
