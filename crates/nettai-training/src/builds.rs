//! Random valid builds for any game (docs/design/training-ai.md §6.2), from
//! what the game's rules state: its builds' data (`nettai_match::build::
//! Builds`: the groups, a build's choices and their presets, the facts of
//! forms), its navis, its folder rules. Nothing here names a game or a
//! game's fact; a build is kept only once the rules' `validate` and
//! `build_check` accept it (`build::Checked`).

use nettai_battle::Content;
use nettai_battle::content::PlayerFact;
use nettai_battle::rules::Fact;
use nettai_content_api::{ChipHandle, NaviHandle, Value};
use nettai_match::build::{Builds, Checked};
use nettai_match::{BackgroundPick, Match, Picks, RoundSettings, Side, check, folders};
use std::sync::Arc;

/// Tries at a build the rules accept before giving up.
const TRIES: usize = 20;

/// Makes a game's random builds and the rounds between them.
pub struct Maker {
    content: Arc<Content>,
    game: String,
    builds: Builds,
    navis: Vec<NaviHandle>,
}

impl Maker {
    /// The builds of `content`'s game.
    pub fn new(content: &Arc<Content>) -> Result<Maker, String> {
        let game = content.game().to_string();
        let builds = Builds::of(content, &game)?;
        let navis = nettai_match::navis(content, &game);
        if navis.is_empty() {
            return Err(format!("{game} has no navi a side can play"));
        }
        Ok(Maker { content: content.clone(), game, builds, navis })
    }

    pub fn game(&self) -> &str {
        &self.game
    }

    pub fn content(&self) -> &Arc<Content> {
        &self.content
    }

    /// The chips of the game whose keys are `keys` (a key the game hasn't
    /// is none of them).
    pub fn chips(&self, keys: &[String]) -> Vec<ChipHandle> {
        keys.iter().filter_map(|k| nettai_match::ids::chip(&self.content, &self.game, k)).collect()
    }

    /// A random build the rules accept (§6.2): a group's new build, a
    /// random navi of the game's (the navi that changes form half the
    /// time), each fact of forms the build chooses
    /// filled at random from what it offers, a random preset of each
    /// choice it has, and a random folder of the chips the rules let it
    /// hold, but `exclude`.
    pub fn random(&self, picks: &mut Picks, exclude: &[ChipHandle]) -> Result<Side, String> {
        let mut last = String::new();
        for _ in 0..TRIES {
            match self.try_random(picks, exclude) {
                Ok(side) => {
                    let checked = Checked::of(&self.content, &self.game, &side);
                    if checked.ready() {
                        return Ok(side);
                    }
                    last = checked.problems.iter().map(|p| p.text.clone()).collect::<Vec<_>>().join("; ");
                }
                Err(e) => last = e,
            }
        }
        Err(format!("{}: no random build the rules accept in {TRIES} tries (the last: {last})", self.game))
    }

    fn try_random(&self, picks: &mut Picks, exclude: &[ChipHandle]) -> Result<Side, String> {
        let (content, b) = (&self.content, &self.builds);
        let group = &b.groups[picks.below(b.groups.len())].id;
        let mut side = b.new_build(content, group)?;
        // (The navi that changes form half the time, as most players'
        // builds are; else any of the game's.)
        let navi = match content.form_changing_navi().filter(|n| self.navis.contains(n)) {
            Some(n) if picks.below(2) == 0 => n,
            _ => self.navis[picks.below(self.navis.len())],
        };
        b.set_navi(content, &mut side, navi)?;
        for choice in b.form_choices(content, &side) {
            if choice.forms.is_empty() {
                continue;
            }
            let mut forms = choice.forms.clone();
            picks.shuffle(&mut forms);
            let n = if choice.one { 1 } else { choice.room.min(forms.len()) };
            forms.truncate(n);
            forms.sort_by_key(|f| choice.forms.iter().position(|x| x == f));
            let values: Vec<Fact> = forms.iter().map(|f| Fact::Value(Value::Def(nettai_content_api::Registry::Form, f.0))).collect();
            side.set_fact(content, &choice.fact, &values)?;
        }
        let choices: Vec<(String, String)> = b
            .choices_of(content, &side)
            .into_iter()
            .map(|c| (c.id.clone(), c.presets[picks.below(c.presets.len())].id.clone()))
            .collect();
        for (choice, preset) in choices {
            b.take_preset(content, &mut side, &choice, &preset)?;
        }
        // The folder, of the chips the side's rules let it hold.
        let m = Match { sides: [side.clone(), side.clone()], ..Match::empty(content, &self.game)? };
        let mut battle = check::set_up(content, &m)?;
        let pool: Vec<ChipHandle> = folders::pool(content, &self.game, &mut battle, 0).into_iter().filter(|c| !exclude.contains(c)).collect();
        if pool.len() < 8 {
            return Err(format!("{}: {} chips are left to make a folder of", self.game, pool.len()));
        }
        let folder = folders::random_folder_of(content, &pool, &mut battle, 0, picks);
        side.set_folder(content, &folder.into())?;
        Ok(side)
    }

    /// A round between builds `sides` (the left first), on a random
    /// arena of the game's from `seed`, each side's base HP `hp` where its
    /// rules take one (a navi whose HP is its own, as a link navi's level
    /// gives it, keeps its own: the rules refuse a base HP for it), checked.
    pub fn round(&self, sides: [Side; 2], hp: u16, seed: u32) -> Result<Match, String> {
        let content = &self.content;
        let mut m = Match {
            game: self.game.clone(),
            seed: Some(seed),
            rounds: vec![RoundSettings::default()],
            sides,
            backgrounds: BackgroundPick::Game,
            custom_screen_time: None,
        };
        let hp_fact = content.defs.fact_name(PlayerFact::BaseHp);
        if let Some(name) = hp_fact {
            for side in &mut m.sides {
                side.set_fact(content, name, &[Fact::Value(Value::Int(hp as i64))])?;
            }
        }
        let mut problems = check::problems(content, &m);
        if let Some(name) = hp_fact {
            let refused: Vec<usize> = problems.iter().filter(|p| p.field.as_deref() == Some(name)).filter_map(|p| p.side).collect();
            if !refused.is_empty() {
                for side in refused {
                    m.sides[side].facts.reset(content, name);
                }
                problems = check::problems(content, &m);
            }
        }
        if !problems.is_empty() {
            return Err(format!("{}: a round of random builds: {}", self.game, problems.iter().map(|p| p.text.clone()).collect::<Vec<_>>().join("; ")));
        }
        Ok(m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::content;

    /// Random builds of every game are valid, of more than one navi, and
    /// keep the chips they leave out out of their folders; a round between
    /// two of them is a checked match at the HP asked where the navi takes
    /// it.
    #[test]
    fn random_builds_are_valid() {
        for game in ["exe6", "exe5", "exe4"] {
            let c = content(game);
            let maker = Maker::new(&c).unwrap();
            let mut picks = Picks::new(7);
            let exclude = maker.chips(&["cannon".to_string(), "minibomb".to_string()]);
            let mut navis = std::collections::BTreeSet::new();
            for i in 0..6 {
                let t = std::time::Instant::now();
                let a = maker.random(&mut picks, &exclude).unwrap_or_else(|e| panic!("{e}"));
                let b = maker.random(&mut picks, &exclude).unwrap_or_else(|e| panic!("{e}"));
                let made = t.elapsed();
                for s in [&a, &b] {
                    navis.insert(s.navi(&c));
                    assert!(s.folder(&c).chips().all(|chip| !exclude.contains(&chip.id)), "{game}: an excluded chip");
                }
                let t = std::time::Instant::now();
                let m = maker.round([a, b], if i % 2 == 0 { 1000 } else { 600 }, i).unwrap_or_else(|e| panic!("{e}"));
                assert!(nettai_match::check_match(&c, &m).is_empty());
                println!("{game}: two builds {made:?}, a round {:?}", t.elapsed());
            }
            println!("{game}: navis {navis:?}");
            assert!(navis.len() > 1 || nettai_match::navis(&c, game).len() == 1, "{game}: one navi");
        }
    }
}
