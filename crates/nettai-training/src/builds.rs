//! Random valid builds for any game (docs/design/training-ai.md §6.2), from
//! what the game's rules state: its builds' data (`nettai_match::build::
//! Builds`: the groups, a build's choices and their presets, the facts of
//! forms), its navis, its folder rules. Nothing here names a game or a
//! game's fact; a build is kept only once the rules' `validate` and
//! `build_check` accept it (`build::Checked`).

use nettai_battle::content::PlayerFact;
use nettai_battle::rules::Fact;
use nettai_battle::{Battle, Content};
use nettai_content_api::{ChipHandle, FieldType, NaviHandle, Registry, Value};
use nettai_match::build::{Builds, Checked};
use nettai_match::facts;
use nettai_match::{BackgroundPick, Match, Picks, RoundSettings, Side, check, folders};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Chips held out of training (§6.6's `chip_holdout`): those a folder may
/// hold whose key, with `seed`, is below `fraction` ([`crate::names::unit`]).
/// Random builds leave them out (`Exclude`), or lean toward them (`Favor`:
/// about half a folder's draws are theirs), for evaluation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Holdout {
    pub fraction: f64,
    pub seed: u64,
    pub mode: HoldoutMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HoldoutMode {
    Exclude,
    Favor,
}

impl HoldoutMode {
    pub fn named(name: &str) -> Option<HoldoutMode> {
        match name {
            "exclude" => Some(HoldoutMode::Exclude),
            "favor" => Some(HoldoutMode::Favor),
            _ => None,
        }
    }
}

impl Default for Holdout {
    /// None held out.
    fn default() -> Holdout {
        Holdout { fraction: 0.0, seed: 0, mode: HoldoutMode::Exclude }
    }
}

impl Holdout {
    /// Whether the chip of key `key` is held out.
    pub fn holds(&self, key: &str) -> bool {
        self.fraction > 0.0 && crate::names::unit(self.seed, key) < self.fraction
    }

    /// The chips a random folder is drawn from, of `pool` (those the rules
    /// let it hold): without those held out, or with them repeated to about
    /// half the draws.
    pub fn pool(&self, content: &Content, pool: Vec<ChipHandle>) -> Vec<ChipHandle> {
        let (held, rest): (Vec<ChipHandle>, Vec<ChipHandle>) = pool.into_iter().partition(|&c| self.holds(&content.defs.chip(c).key));
        match self.mode {
            HoldoutMode::Exclude => rest,
            HoldoutMode::Favor if held.is_empty() => rest,
            HoldoutMode::Favor => {
                let copies = (rest.len() / held.len()).max(1);
                let mut out = rest;
                for _ in 0..copies {
                    out.extend(&held);
                }
                out
            }
        }
    }
}

/// A random build, and the pair of chips its folder was seeded with, if it
/// was (§6.2).
#[derive(Clone, Debug)]
pub struct Build {
    pub side: Side,
    pub pair: Option<crate::fingerprint::Pair>,
}

/// The pairs random folders may be seeded with (a game's, from its chips'
/// fingerprints), and the chance a folder is.
#[derive(Clone, Copy, Debug)]
pub struct Combos<'a> {
    pub pairs: &'a [crate::fingerprint::Pair],
    pub rate: f64,
}

impl Combos<'_> {
    /// No folder seeded.
    pub const NONE: Combos<'static> = Combos { pairs: &[], rate: 0.0 };
}

/// Tries at a build the rules accept before giving up.
const TRIES: usize = 20;

/// The numbers a drawn fact is below (a board's place, a slot, a size: a
/// game's small numbers; the rules refuse what is past theirs).
const SMALL: usize = 8;

/// Whether `p` comes up, a chance in [0, 1].
pub fn chance(picks: &mut Picks, p: f64) -> bool {
    p > 0.0 && (picks.next() >> 11) as f64 / (1u64 << 53) as f64 <= p
}

/// A value drawn for a fact or a list entry's field.
#[derive(Clone, Debug, PartialEq)]
enum Drawn {
    Nil,
    Bool(bool),
    Int(i64),
    Name(String),
    Code(u8),
    Def(Registry, u16),
}

impl Drawn {
    fn fact(&self) -> Fact<'_> {
        match self {
            Drawn::Nil => Fact::Value(Value::Nil),
            Drawn::Bool(b) => Fact::Value(Value::Bool(*b)),
            Drawn::Int(n) => Fact::Value(Value::Int(*n)),
            Drawn::Name(n) => Fact::Name(n),
            Drawn::Code(c) => Fact::Value(Value::Code(*c)),
            Drawn::Def(r, h) => Fact::Value(Value::Def(*r, *h)),
        }
    }
}

/// State list fact `field` of `side` as `entries` (each a record's fields
/// by name, or one value under no name).
fn set_list(content: &Content, side: &mut Side, field: &str, entries: &[Vec<(String, Drawn)>]) -> Result<(), String> {
    let values: Vec<Fact> = entries
        .iter()
        .map(|e| match &e[..] {
            [(name, v)] if name.is_empty() => v.fact(),
            fields => Fact::Record(fields.iter().map(|(n, v)| (n.as_str(), v.fact())).collect()),
        })
        .collect();
    side.set_fact(content, field, &values)
}

/// A folder's start seeded with a pair of `pairs` whose chips are both in
/// `pool` (the chips the side may hold): each chip in a code both come in
/// (or `*`), two copies each where the side's rules (`battle`'s) take them;
/// none found in a few tries, none.
fn seed_pair(content: &Content, pool: &[ChipHandle], pairs: &[crate::fingerprint::Pair], battle: &mut Battle, picks: &mut Picks) -> (Vec<nettai_battle::custom::FolderChip>, Option<crate::fingerprint::Pair>) {
    use nettai_battle::content::ChipCode;
    use nettai_battle::custom::FolderChip;
    if pairs.is_empty() {
        return (Vec::new(), None);
    }
    for _ in 0..8 {
        let p = pairs[picks.below(pairs.len())];
        if !pool.contains(&p.setup) || !pool.contains(&p.payoff) {
            continue;
        }
        let (a, b) = (&content.chip(p.setup).codes, &content.chip(p.payoff).codes);
        let common: Vec<ChipCode> = a.iter().filter(|c| b.contains(c)).copied().collect();
        let star = ChipCode::ASTERISK;
        let (ca, cb) = if !common.is_empty() {
            let c = common[picks.below(common.len())];
            (c, c)
        } else if a.contains(&star) && !b.is_empty() {
            (star, b[picks.below(b.len())])
        } else if b.contains(&star) && !a.is_empty() {
            (a[picks.below(a.len())], star)
        } else {
            continue;
        };
        let mut start: Vec<FolderChip> = Vec::new();
        for c in [FolderChip::new(p.setup, ca), FolderChip::new(p.payoff, cb), FolderChip::new(p.setup, ca), FolderChip::new(p.payoff, cb)] {
            start.push(c);
            if !battle.check_folder(0, &start, None, None, false).is_empty() {
                start.pop();
            }
        }
        if start.iter().any(|c| c.id == p.setup) && start.iter().any(|c| c.id == p.payoff) {
            return (start, Some(p));
        }
    }
    (Vec::new(), None)
}

/// Whether a fact's type names navis (a list of other navis: a side's
/// party).
fn names_navis(ty: &FieldType) -> bool {
    match ty {
        FieldType::Ref(Registry::Navi, _) => true,
        FieldType::Array(e, _) | FieldType::List(e, _) => names_navis(e),
        FieldType::Record(fields) => fields.fields().iter().any(|f| names_navis(&f.ty)),
        _ => false,
    }
}

/// Makes a game's random builds and the rounds between them.
pub struct Maker {
    content: Arc<Content>,
    game: String,
    builds: Builds,
    navis: Vec<NaviHandle>,
    /// What its builds took ([`Maker::counts`]).
    counts: Counters,
}

/// What a maker's builds took, counted as they are made (from any thread).
#[derive(Debug, Default)]
struct Counters {
    builds: AtomicU64,
    tries: AtomicU64,
    randomized: AtomicU64,
    entries_tried: AtomicU64,
    entries_kept: AtomicU64,
    nanos: AtomicU64,
}

/// What a maker's builds took: builds made, tries at them (a try the
/// rules refuse whole is made again), those whose every setup fact was
/// drawn, the list entries tried and kept, and the time.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub builds: u64,
    pub tries: u64,
    pub randomized: u64,
    pub entries_tried: u64,
    pub entries_kept: u64,
    pub nanos: u64,
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
        Ok(Maker { content: content.clone(), game, builds, navis, counts: Counters::default() })
    }

    /// The game's default build: its first group's new build (the navi that
    /// changes form, the forms of that version, a new save's folder), as
    /// built.
    pub fn default_build(&self) -> Result<Side, String> {
        self.builds.new_build(&self.content, &self.builds.groups[0].id)
    }

    /// What its builds took so far.
    pub fn counts(&self) -> Counts {
        let c = &self.counts;
        let get = |a: &AtomicU64| a.load(Ordering::Relaxed);
        Counts {
            builds: get(&c.builds),
            tries: get(&c.tries),
            randomized: get(&c.randomized),
            entries_tried: get(&c.entries_tried),
            entries_kept: get(&c.entries_kept),
            nanos: get(&c.nanos),
        }
    }

    pub fn game(&self) -> &str {
        &self.game
    }

    pub fn content(&self) -> &Arc<Content> {
        &self.content
    }

    /// A random build the rules accept (§6.2): a group's new build, a
    /// random navi of the game's (the navi that changes form half the
    /// time), each fact of forms filled at random from what it offers, a
    /// random preset of each choice, a random folder of the chips the rules
    /// let it hold, those held out left out or favored (`holdout`; its
    /// Regular chip, and tag chips half the time where the rules take
    /// them); and with chance `randomize`, every
    /// other fact a build states drawn too ([`Maker::randomize`]: the
    /// NaviCust, the patch cards...), else the rules' defaults. With chance
    /// `combos.rate` the folder is seeded with one of `combos.pairs` (both
    /// chips in a code they share, two copies each where the rules take
    /// them), the rest drawn.
    pub fn random(&self, picks: &mut Picks, holdout: &Holdout, randomize: f64, combos: Combos) -> Result<Build, String> {
        let started = std::time::Instant::now();
        let mut last = String::new();
        let result = (|| {
            for _ in 0..TRIES {
                self.counts.tries.fetch_add(1, Ordering::Relaxed);
                let all = chance(picks, randomize);
                match self.try_random(picks, holdout, all, combos) {
                    Ok(build) => {
                        let checked = Checked::of(&self.content, &self.game, &build.side);
                        if checked.ready() {
                            self.counts.randomized.fetch_add(all as u64, Ordering::Relaxed);
                            return Ok(build);
                        }
                        last = checked.problems.iter().map(|p| p.text.clone()).collect::<Vec<_>>().join("; ");
                    }
                    Err(e) => last = e,
                }
            }
            Err(format!("{}: no random build the rules accept in {TRIES} tries (the last: {last})", self.game))
        })();
        self.counts.builds.fetch_add(result.is_ok() as u64, Ordering::Relaxed);
        self.counts.nanos.fetch_add(started.elapsed().as_nanos() as u64, Ordering::Relaxed);
        result
    }

    fn try_random(&self, picks: &mut Picks, holdout: &Holdout, all: bool, combos: Combos) -> Result<Build, String> {
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
            // (A list full half the time, as most players' are; else of a
            // random length.)
            let full = choice.room.min(forms.len());
            let n = if choice.one { 1 } else if picks.below(2) == 0 { full } else { 1 + picks.below(full) };
            forms.truncate(n);
            forms.sort_by_key(|f| choice.forms.iter().position(|x| x == f));
            let values: Vec<Fact> = forms.iter().map(|f| Fact::Value(Value::Def(Registry::Form, f.0))).collect();
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
        // The rest of what a build states, drawn, before the folder (the
        // folder's rules read the stats the side's facts make).
        let m = Match { sides: [side.clone(), side.clone()], ..Match::empty(content, &self.game)? };
        let mut battle = check::set_up(content, &m)?;
        if all {
            self.randomize(&mut side, &mut battle, picks)?;
            let m = Match { sides: [side.clone(), side.clone()], ..Match::empty(content, &self.game)? };
            battle = check::set_up(content, &m)?;
        }
        // The folder, of the chips the side's rules let it hold.
        let pool = holdout.pool(content, folders::pool(content, &self.game, &mut battle, 0));
        if pool.len() < 8 {
            return Err(format!("{}: {} chips are left to make a folder of", self.game, pool.len()));
        }
        let (start, pair) = if chance(picks, combos.rate) { seed_pair(content, &pool, combos.pairs, &mut battle, picks) } else { (Vec::new(), None) };
        let mut folder: nettai_match::Folder = folders::random_folder_from(content, &pool, &start, &mut battle, 0, picks).into();
        // (Tag chips half the time, where the rules take them: two entries
        // the rules accept as the pair.)
        if content.defs.fact_field(PlayerFact::TagChips).is_some() && picks.below(2) == 0 {
            let chips: Vec<_> = folder.chips.iter().flatten().copied().collect();
            for _ in 0..8 {
                let (a, b) = (picks.below(chips.len()) as u8, picks.below(chips.len()) as u8);
                if a != b && Some(a) != folder.regular && Some(b) != folder.regular && battle.check_folder(0, &chips, folder.regular, Some((a.min(b), a.max(b))), true).is_empty() {
                    folder.tags = Some((a.min(b), a.max(b)));
                    break;
                }
            }
        }
        side.set_folder(content, &folder)?;
        Ok(Build { side, pair })
    }

    /// Every fact a build states that nothing above drew (§6.2): not the
    /// navi, the folder and its Regular and tag chips, the facts of forms,
    /// the facts a choice's presets set, those no build states (the rules'
    /// defaults: `Builds::never_stated`), nor a list of other navis (a side's
    /// party: the env plays no switch to one). Each drawn from what its type
    /// allows (a flag, a variant, a definition of the game's, a small
    /// number), the side's single facts first; a list filled an entry at a
    /// time, to a random length (16 at most), each entry kept only if the rules'
    /// `validate` and `build_check` (asked of `battle`, the side's round set
    /// up, its setup changed in place) say nothing of it; a single fact they
    /// say something of goes back to its default.
    fn randomize(&self, side: &mut Side, battle: &mut Battle, picks: &mut Picks) -> Result<(), String> {
        let content = &self.content;
        let skipped: Vec<String> = {
            let mut out: Vec<String> = facts::form_facts(content).iter().map(|f| f.name.to_string()).collect();
            for c in &self.builds.choices {
                out.extend(c.facts().into_iter().map(str::to_string));
            }
            out
        };
        let fields: Vec<facts::Field> = facts::fields(content)
            .into_iter()
            .filter(|f| facts::role_of(content, f.name).is_none() && !self.builds.never_stated(content, f.name) && !skipped.iter().any(|s| s == f.name) && !names_navis(f.ty))
            .collect();
        let (lists, singles): (Vec<_>, Vec<_>) = fields.into_iter().partition(|f| matches!(f.ty, FieldType::List(..)));
        let mut defs: HashMap<(Registry, Option<String>), Vec<u16>> = HashMap::new();
        for f in singles {
            let Some(value) = self.draw(f.ty, picks, &mut defs) else { continue };
            let before = side.clone();
            if side.set_fact(content, f.name, &[value.fact()]).is_err() || !self.accepted(side, battle, f.name, None)? {
                *side = before;
            }
        }
        for f in lists {
            let FieldType::List(elem, room) = f.ty else { continue };
            // (A random length up to the list's room or 16 entries; drawn
            // entries tried until it is reached, or the rules refuse ten in
            // a row: a board full.)
            let target = picks.below((*room as usize).min(2 * SMALL) + 1);
            let mut entries: Vec<Vec<(String, Drawn)>> = Vec::new();
            let mut refused = 0;
            while entries.len() < target && refused < 10 {
                let entry: Vec<(String, Drawn)> = match &**elem {
                    FieldType::Record(fields) => fields.fields().iter().filter_map(|g| Some((g.name.clone(), self.draw(&g.ty, picks, &mut defs)?))).collect(),
                    ty => match self.draw(ty, picks, &mut defs) {
                        Some(v) => vec![(String::new(), v)],
                        None => break,
                    },
                };
                entries.push(entry);
                self.counts.entries_tried.fetch_add(1, Ordering::Relaxed);
                let before = side.clone();
                let kept = set_list(content, side, f.name, &entries).is_ok() && self.accepted(side, battle, f.name, Some(entries.len() - 1))?;
                if kept {
                    self.counts.entries_kept.fetch_add(1, Ordering::Relaxed);
                    refused = 0;
                } else {
                    entries.pop();
                    *side = before;
                    refused += 1;
                }
            }
        }
        // (The battle's setup back to the side's, as it now is.)
        battle.setup.players[0].rules = side.facts.block().cloned();
        Ok(())
    }

    /// Whether the rules say nothing of `side`'s fact `field` (its entry
    /// `entry` of a list, or any of its entries) with `side` in `battle`.
    fn accepted(&self, side: &Side, battle: &mut Battle, field: &str, entry: Option<usize>) -> Result<bool, String> {
        battle.setup.players[0].rules = side.facts.block().cloned();
        let asked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut p = battle.validate(0);
            p.extend(battle.build_problems(0));
            p
        }));
        let Ok(problems) = asked else { return Ok(false) };
        Ok(!problems.iter().any(|p| p.field.as_deref() == Some(field) && (entry.is_none() || p.entry.is_none() || p.entry == entry)))
    }

    /// A value of type `ty`, drawn: none for a type nothing draws (a
    /// record or a list within one, an object, a vector).
    fn draw(&self, ty: &FieldType, picks: &mut Picks, defs: &mut HashMap<(Registry, Option<String>), Vec<u16>>) -> Option<Drawn> {
        Some(match ty {
            FieldType::Bool => Drawn::Bool(picks.below(2) == 1),
            FieldType::U8 | FieldType::U16 | FieldType::U32 | FieldType::I8 | FieldType::I16 | FieldType::I32 => Drawn::Int(picks.below(SMALL) as i64),
            FieldType::OptionalU8 => match picks.below(4) {
                0 => Drawn::Nil,
                _ => Drawn::Int(picks.below(SMALL) as i64),
            },
            FieldType::Enum(names) => Drawn::Name(names[picks.below(names.len())].clone()),
            FieldType::Code => Drawn::Code(b'A' + picks.below(26) as u8),
            FieldType::Ref(registry, of) => {
                let all = defs.entry((*registry, of.clone())).or_insert_with(|| {
                    nettai_match::ids::all_of(&self.content, *registry, of.as_deref())
                        .into_iter()
                        .filter(|&h| nettai_match::ids::key_of(&self.content, *registry, h).is_some_and(|k| nettai_match::ids::in_game(&self.content, &self.game, k)))
                        .collect()
                });
                if all.is_empty() {
                    return None;
                }
                Drawn::Def(*registry, all[picks.below(all.len())])
            }
            _ => return None,
        })
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
    /// keep the chips held out out of their folders; a round between
    /// two of them is a checked match at the HP asked where the navi takes
    /// it.
    #[test]
    fn random_builds_are_valid() {
        for game in ["exe6", "exe5", "exe4"] {
            let c = content(game);
            let maker = Maker::new(&c).unwrap();
            let mut picks = Picks::new(7);
            let holdout = Holdout { fraction: 0.3, seed: 5, mode: HoldoutMode::Exclude };
            let mut navis = std::collections::BTreeSet::new();
            for i in 0..6 {
                let t = std::time::Instant::now();
                let a = maker.random(&mut picks, &holdout, 0.75, Combos::NONE).unwrap_or_else(|e| panic!("{e}")).side;
                let b = maker.random(&mut picks, &holdout, 0.75, Combos::NONE).unwrap_or_else(|e| panic!("{e}")).side;
                let made = t.elapsed();
                for s in [&a, &b] {
                    navis.insert(s.navi(&c));
                    assert!(s.folder(&c).chips().all(|chip| !holdout.holds(&c.defs.chip(chip.id).key)), "{game}: a chip held out");
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

    /// Favoring held-out chips, about half a folder's chips are theirs.
    #[test]
    fn favored_chips_fill_about_half_a_folder() {
        let c = content("exe6");
        let maker = Maker::new(&c).unwrap();
        let holdout = Holdout { fraction: 0.1, seed: 2, mode: HoldoutMode::Favor };
        let mut picks = Picks::new(9);
        let (mut held, mut all) = (0, 0);
        for _ in 0..6 {
            let side = maker.random(&mut picks, &holdout, 0.0, Combos::NONE).unwrap().side;
            for chip in side.folder(&c).chips() {
                held += holdout.holds(&c.defs.chip(chip.id).key) as u32;
                all += 1;
            }
        }
        println!("{held} of {all} chips held out");
        assert!(held * 4 > all && held * 4 < 3 * all, "{held} of {all}");
    }

    /// Randomized builds vary every fact a build states (§6.2): over many
    /// builds of each game, each such fact is off its default in some; the
    /// rules accept every build made. Prints the acceptance and the time.
    #[test]
    fn randomized_builds_vary_every_fact_a_build_states() {
        for game in ["exe6", "exe5", "exe4"] {
            let c = content(game);
            let maker = Maker::new(&c).unwrap();
            let mut picks = Picks::new(3);
            let mut varied: std::collections::BTreeMap<String, u32> = std::collections::BTreeMap::new();
            let n = 24;
            for _ in 0..n {
                let side = maker.random(&mut picks, &Holdout::default(), 1.0, Combos::NONE).unwrap_or_else(|e| panic!("{e}")).side;
                for f in facts::fields(&c) {
                    *varied.entry(f.name.to_string()).or_default() += (!side.facts.is_default(&c, f.name)) as u32;
                }
            }
            let k = maker.counts();
            println!(
                "{game}: {} builds in {} tries ({:.0}% accepted whole), {:.1} ms a build (test build), list entries kept {}/{} ({:.0}%)",
                k.builds,
                k.tries,
                100.0 * k.builds as f64 / k.tries as f64,
                k.nanos as f64 / k.builds as f64 / 1e6,
                k.entries_kept,
                k.entries_tried,
                100.0 * k.entries_kept as f64 / k.entries_tried.max(1) as f64
            );
            println!("  builds off the default, by fact: {varied:?}");
            let b = Builds::of(&c, game).unwrap();
            for f in facts::fields(&c) {
                let stated = !b.never_stated(&c, f.name) && !names_navis(f.ty);
                if stated {
                    assert!(varied[f.name] > 0, "{game}: {} never varied", f.name);
                }
            }
        }
    }
}
