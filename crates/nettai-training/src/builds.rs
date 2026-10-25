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

/// The share of randomized builds whose lists are filled full: entries
/// tried until the list's room is reached or the rules refuse
/// [`FULL_PATIENCE`] in a row (a board as full as random places can make
/// it, as a player's is), every entry the rules take kept, bugs and all;
/// the rest to a random length (the user: random boards were far sparser
/// than a player's, "you might want to take patch cards into account as
/// well").
pub const FULL_SHARE: f64 = 0.5;

/// Refusals in a row that end a list filled full.
const FULL_PATIENCE: usize = 60;

/// Entries drawn for each list's pool ([`Maker::pools`]).
const POOL_TRIES: usize = 4000;

/// A list's entry as drawn: its fields by name (one unnamed for a list of
/// single values).
type Entry = Vec<(String, Drawn)>;

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
/// `pool` (the chips the side may hold), drawn by the kind of effect its
/// setup brings first (`Condition::category`), then within it: each chip in
/// a code both come in (or `*`), two copies each where the side's rules
/// (`battle`'s) take them; none found in a few tries, none.
fn seed_pair(content: &Content, pool: &[ChipHandle], pairs: &[crate::fingerprint::Pair], battle: &mut Battle, picks: &mut Picks) -> (Vec<nettai_battle::custom::FolderChip>, Option<crate::fingerprint::Pair>) {
    use nettai_battle::content::ChipCode;
    use nettai_battle::custom::FolderChip;
    // (A kind of effect first, of those the pairs the side may hold are of,
    // then a pair of it: no one kind's many pairs crowd out the rest.)
    let held: Vec<&crate::fingerprint::Pair> = pairs.iter().filter(|p| pool.contains(&p.setup) && pool.contains(&p.payoff)).collect();
    let mut categories: Vec<&str> = held.iter().map(|p| p.condition.category()).collect();
    categories.sort();
    categories.dedup();
    if categories.is_empty() {
        return (Vec::new(), None);
    }
    for _ in 0..8 {
        let category = categories[picks.below(categories.len())];
        let of: Vec<&&crate::fingerprint::Pair> = held.iter().filter(|p| p.condition.category() == category).collect();
        let p = **of[picks.below(of.len())];
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
    /// By list, entries the rules take alone in a new build of the navi that
    /// changes form ([`Maker::pools`]), made the first time a list is filled
    /// full.
    pools: std::sync::OnceLock<HashMap<String, Vec<Entry>>>,
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
        Ok(Maker { content: content.clone(), game, builds, navis, counts: Counters::default(), pools: Default::default() })
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
        self.random_with(picks, holdout, randomize, FULL_SHARE, combos)
    }

    /// The same, with chance `full` (of a randomized build) that its lists
    /// are filled full (the env's `full_lists`; [`FULL_SHARE`] by default).
    pub fn random_with(&self, picks: &mut Picks, holdout: &Holdout, randomize: f64, full: f64, combos: Combos) -> Result<Build, String> {
        let started = std::time::Instant::now();
        let mut last = String::new();
        let result = (|| {
            for _ in 0..TRIES {
                self.counts.tries.fetch_add(1, Ordering::Relaxed);
                let all = chance(picks, randomize);
                let full = all && chance(picks, full);
                match self.try_random(picks, holdout, all, full, combos) {
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

    fn try_random(&self, picks: &mut Picks, holdout: &Holdout, all: bool, full: bool, combos: Combos) -> Result<Build, String> {
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
            self.randomize(&mut side, &mut battle, picks, full)?;
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

    /// What the build creator lets a player change that nothing above drew
    /// (§6.2): its lists (the pieces on a board, the cards, the places: not
    /// the folder, an engine's role; not those no build states, the rules'
    /// defaults: `Builds::never_stated`; not a list of other navis, a side's
    /// party, whose switching the env doesn't play). The creator offers no
    /// single fact but through the choices' presets and the facts of forms
    /// (and a board's size, which stays the new build's: the largest), so
    /// every single fact stays as the new build has it, every build at the
    /// most its game allows. A list is filled an entry at a time, to a random
    /// length (16 at most), or with `full` to its room ([`FULL_SHARE`]: its
    /// entries mostly from its pool, [`Maker::pools`]), each entry's fields drawn from what their types
    /// allow (a flag, a variant, a definition of the game's, a number below
    /// [`SMALL`]: the creator's own ranges, a board's size, are its screens'),
    /// each entry kept only if the rules' `validate` and `build_check` (asked
    /// of `battle`, the side's round set up, its setup changed in place) say
    /// nothing of it.
    fn randomize(&self, side: &mut Side, battle: &mut Battle, picks: &mut Picks, full: bool) -> Result<(), String> {
        let content = &self.content;
        let lists = self.list_fields();
        let mut defs: HashMap<(Registry, Option<String>), Vec<u16>> = HashMap::new();
        let pools = if full { Some(self.pools()) } else { None };
        for f in lists {
            let FieldType::List(elem, room) = f.ty else { continue };
            // (A random length up to the list's room or 16 entries; drawn
            // entries tried until it is reached, or the rules refuse ten in
            // a row. Filled full, to its room, refused FULL_PATIENCE times in a
            // row: a board full. Its entries then mostly from its pool, with a
            // field drawn anew half the time, and else drawn whole.)
            let (target, patience) = if full { (*room as usize, FULL_PATIENCE) } else { (picks.below((*room as usize).min(2 * SMALL) + 1), 10) };
            let pool: &[Entry] = pools.and_then(|p| p.get(f.name)).map_or(&[], Vec::as_slice);
            let mut entries: Vec<Entry> = Vec::new();
            let mut refused = 0;
            while entries.len() < target && refused < patience {
                let entry = if !pool.is_empty() && picks.below(4) != 0 {
                    let mut e = pool[picks.below(pool.len())].clone();
                    if picks.below(2) == 0 && !e.is_empty() {
                        let at = picks.below(e.len());
                        let ty = match &**elem {
                            FieldType::Record(fields) => fields.fields().iter().find(|g| g.name == e[at].0).map(|g| g.ty.clone()),
                            ty => Some(ty.clone()),
                        };
                        if let Some(v) = ty.and_then(|ty| self.draw(&ty, picks, &mut defs)) {
                            e[at].1 = v;
                        }
                    }
                    e
                } else {
                    match self.draw_entry(elem, picks, &mut defs) {
                        Some(e) => e,
                        None => break,
                    }
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

    /// A list's entry, each field drawn (none: its type draws nothing).
    fn draw_entry(&self, elem: &FieldType, picks: &mut Picks, defs: &mut HashMap<(Registry, Option<String>), Vec<u16>>) -> Option<Entry> {
        Some(match elem {
            FieldType::Record(fields) => fields.fields().iter().filter_map(|g| Some((g.name.clone(), self.draw(&g.ty, picks, defs)?))).collect(),
            ty => vec![(String::new(), self.draw(ty, picks, defs)?)],
        })
    }

    /// By list a build states, entries the rules take alone, each the only
    /// entry of its list in a new build of the navi that changes form (its
    /// board the largest), from [`POOL_TRIES`] drawn on a seed of the list's:
    /// what a list filled full mostly draws from, so its tries are spent on
    /// where entries go rather than on fields no entry could have (a color
    /// a program doesn't come in, a turn past the last). The same whichever
    /// thread makes them first. A list none fits has none.
    fn pools(&self) -> &HashMap<String, Vec<Entry>> {
        self.pools.get_or_init(|| {
            let content = &self.content;
            let made = (|| -> Result<HashMap<String, Vec<Entry>>, String> {
                let mut side = self.builds.new_build(content, &self.builds.groups[0].id)?;
                if let Some(n) = content.form_changing_navi().filter(|n| self.navis.contains(n)) {
                    self.builds.set_navi(content, &mut side, n)?;
                }
                let m = Match { sides: [side.clone(), side.clone()], ..Match::empty(content, &self.game)? };
                let mut battle = check::set_up(content, &m)?;
                let mut defs = HashMap::new();
                let mut out = HashMap::new();
                for (i, f) in self.list_fields().into_iter().enumerate() {
                    let FieldType::List(elem, _) = f.ty else { continue };
                    let mut picks = Picks::new(0x9001 + i as u32);
                    let mut kept: Vec<Entry> = Vec::new();
                    for _ in 0..POOL_TRIES {
                        let Some(e) = self.draw_entry(elem, &mut picks, &mut defs) else { break };
                        let mut one = side.clone();
                        if set_list(content, &mut one, f.name, std::slice::from_ref(&e)).is_ok() && self.accepted(&one, &mut battle, f.name, Some(0))? && !kept.contains(&e) {
                            kept.push(e);
                        }
                    }
                    out.insert(f.name.to_string(), kept);
                }
                Ok(out)
            })();
            made.unwrap_or_default()
        })
    }

    /// The lists a randomized build states ([`Maker::randomize`]).
    fn list_fields(&self) -> Vec<facts::Field<'_>> {
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
        fields.into_iter().filter(|f| matches!(f.ty, FieldType::List(..))).collect()
    }

    /// Whether the rules say nothing of `side`'s list fact `field` (its entry
    /// `entry`, or any of its entries) with `side` in `battle`.
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

    /// [`Maker::round_with`] each side's base HP `hp`.
    pub fn round(&self, sides: [Side; 2], hp: u16, seed: u32) -> Result<Match, String> {
        self.round_with(sides, Some(hp), seed)
    }

    /// A round between builds `sides` (the left first), on a random
    /// arena of the game's from `seed`, checked; with `hp`, each side's base
    /// HP that where its rules take one (a navi whose HP is its own, as a
    /// link navi's level gives it, keeps its own: the rules refuse a base HP
    /// for it); without, each side's HP as its build has it (the creator's:
    /// the most its game allows, or its preset's).
    pub fn round_with(&self, sides: [Side; 2], hp: Option<u16>, seed: u32) -> Result<Match, String> {
        let content = &self.content;
        let mut m = Match {
            game: self.game.clone(),
            seed: Some(seed),
            rounds: vec![RoundSettings::default()],
            sides,
            backgrounds: BackgroundPick::Game,
            custom_screen_time: None,
        };
        let hp_fact = content.defs.fact_name(PlayerFact::BaseHp).zip(hp);
        if let Some((name, hp)) = hp_fact {
            for side in &mut m.sides {
                side.set_fact(content, name, &[Fact::Value(Value::Int(hp as i64))])?;
            }
        }
        let mut problems = check::problems(content, &m);
        if let Some((name, _)) = hp_fact {
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
                let m = maker.round_with([a, b], if i % 2 == 0 { None } else { Some(600) }, i).unwrap_or_else(|e| panic!("{e}"));
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

    /// Random builds stay what the build creator makes (§6.2): over many
    /// builds of each game, every one drawing all it may, each seat's max HP
    /// as a round starts is the HP the creator's overview gives the build,
    /// and every single fact (not an engine's role, a fact of forms, a
    /// choice's or a list) is as a new build of the side's navi has it.
    /// Prints the max HP's spread.
    #[test]
    fn random_builds_stay_what_the_creator_makes() {
        for game in ["exe6", "exe5", "exe4"] {
            let c = content(game);
            let maker = Maker::new(&c).unwrap();
            let b = Builds::of(&c, game).unwrap();
            let form_facts: Vec<&str> = facts::form_facts(&c).iter().map(|f| f.name).collect();
            let choice_facts: Vec<&str> = b.choices.iter().flat_map(|ch| ch.facts()).collect();
            let mut picks = Picks::new(21);
            let mut hps = Vec::new();
            for i in 0..60 {
                let side = maker.random(&mut picks, &Holdout::default(), 1.0, Combos::NONE).unwrap().side;
                let mut checked = Checked::of(&c, game, &side);
                let shown = checked.overview(&c, &side).expect("the creator's overview").hp.value;
                let m = maker.round_with([side.clone(), side.clone()], None, i).unwrap();
                let battle = nettai_battle::Battle::new(m.round(&c, i), c.clone());
                let hp = battle.stats[0].max_hp;
                assert_eq!(hp as i32, shown, "{game}: max HP against the creator's");
                hps.push(hp);
                let navi = side.navi(&c);
                let starts: Vec<Side> = b
                    .groups
                    .iter()
                    .map(|g| {
                        let mut r = b.new_build(&c, &g.id).unwrap();
                        b.set_navi(&c, &mut r, navi).unwrap();
                        r
                    })
                    .collect();
                for f in facts::fields(&c) {
                    let skip = facts::role_of(&c, f.name).is_some() || form_facts.contains(&f.name) || choice_facts.contains(&f.name) || matches!(f.ty, FieldType::List(..));
                    if !skip {
                        let value = side.facts.get(&c, f.name);
                        assert!(starts.iter().any(|r| r.facts.get(&c, f.name) == value), "{game}: {} is {value:?}, which no new build has", f.name);
                    }
                }
            }
            hps.sort();
            let under = |x: u16| 100.0 * hps.iter().filter(|&&h| h < x).count() as f64 / hps.len() as f64;
            println!("{game}: max HP min {} median {} max {}; under 100 {:.1}%, under 500 {:.1}%, at 1000 {:.1}%", hps[0], hps[hps.len() / 2], hps[hps.len() - 1], under(100), under(500), 100.0 * hps.iter().filter(|&&h| h == 1000).count() as f64 / hps.len() as f64);
        }
    }

    /// How full randomized builds' lists are (§6.2): by game, each list a
    /// build states, its entries over many builds, and the max HP the
    /// builds start with, printed. (A measure: the user, of random boards
    /// sparser than a player's, "you might want to take patch cards into
    /// account as well".)
    #[test]
    fn how_full_randomized_lists_are() {
        for game in ["exe6", "exe5", "exe4"] {
            let c = content(game);
            let maker = Maker::new(&c).unwrap();
            let lists: Vec<(String, u16)> = facts::fields(&c)
                .into_iter()
                .filter_map(|f| match f.ty {
                    FieldType::List(_, room) if facts::role_of(&c, f.name).is_none() => Some((f.name.to_string(), *room)),
                    _ => None,
                })
                .collect();
            let started = std::time::Instant::now();
            let pools: Vec<(String, usize)> = maker.pools().iter().map(|(k, v)| (k.clone(), v.len())).collect();
            println!("{game}: pools {pools:?} in {:.0} ms", started.elapsed().as_secs_f64() * 1000.0);
            for full in [false, true] {
            let mut picks = Picks::new(5);
            let (mut counts, mut hps) = (vec![Vec::new(); lists.len()], Vec::new());
            let started = std::time::Instant::now();
            for _ in 0..80 {
                let side = (0..TRIES)
                    .find_map(|_| maker.try_random(&mut picks, &Holdout::default(), true, full, Combos::NONE).ok().filter(|b| Checked::of(&c, game, &b.side).ready()))
                    .expect("a build the rules accept")
                    .side;
                for (i, (name, _)) in lists.iter().enumerate() {
                    counts[i].push(match side.facts.get(&c, name) {
                        Some(nettai_match::facts::Stated::List(e)) => e.len(),
                        _ => 0,
                    });
                }
                hps.push(Checked::of(&c, game, &side).overview(&c, &side).expect("the overview").hp.value);
            }
            let ms = started.elapsed().as_secs_f64() * 1000.0 / 80.0;
            println!("{game}{}: {ms:.0} ms a build; max HP mean {:.0}, at its least {}, at its most {}", if full { " (full)" } else { "" }, hps.iter().sum::<i32>() as f64 / hps.len() as f64, hps.iter().min().unwrap(), hps.iter().max().unwrap());
            for (i, (name, room)) in lists.iter().enumerate() {
                let v = &counts[i];
                if v.iter().all(|&n| n == 0) {
                    continue;
                }
                println!("  {name}: room {room}, entries mean {:.1}, most {}, none in {}", v.iter().sum::<usize>() as f64 / v.len() as f64, v.iter().max().unwrap(), v.iter().filter(|&&n| n == 0).count());
            }
            }
        }
    }

    /// (A measure, run by hand: ticks a second of rounds of full builds, and
    /// of the same builds with one list emptied.)
    #[test]
    #[ignore]
    fn what_full_lists_cost_a_tick() {
        for game in ["exe4", "exe5", "exe6"] {
            let c = content(game);
            let maker = Maker::new(&c).unwrap();
            let mut picks = Picks::new(11);
            let mut make = |full: bool| -> Vec<Side> {
                (0..12)
                    .map(|_| (0..TRIES).find_map(|_| maker.try_random(&mut picks, &Holdout::default(), true, full, Combos::NONE).ok().filter(|b| Checked::of(&c, game, &b.side).ready())).unwrap().side)
                    .collect()
            };
            let (builds, plain) = (make(true), make(false));
            let lists: Vec<String> = maker.list_fields().iter().map(|f| f.name.to_string()).collect();
            let rate = |builds: &[Side], emptied: Option<&str>| {
                let started = std::time::Instant::now();
                let mut ticks = 0u64;
                for (i, pair) in builds.chunks(2).enumerate() {
                    let mut sides = [pair[0].clone(), pair[1].clone()];
                    if let Some(name) = emptied {
                        for s in &mut sides {
                            let _ = set_list(&c, s, name, &[]);
                        }
                    }
                    let Ok(m) = maker.round(sides, 1000, i as u32) else { continue };
                    let mut b = nettai_match::Set::of(&c, &m, i as u32).start();
                    let mut rng = nettai_netplay::rng::SplitMix64::new(i as u64);
                    for _ in 0..3000 {
                        let keys = [0u16, 1].map(|_| (rng.next_u64() & 0x3FF) as u16 & !0x0C);
                        if !crate::env::step_battle(&mut b, keys) || b.is_battle_over() {
                            break;
                        }
                        ticks += 1;
                    }
                }
                ticks as f64 / started.elapsed().as_secs_f64()
            };
            println!("{game}: full {:.0} ticks/s, not full {:.0}", rate(&builds, None), rate(&plain, None));
            for l in &lists {
                println!("  full without {l}: {:.0} ticks/s", rate(&builds, Some(l)));
            }
        }
    }

    /// Randomized builds vary what the creator offers (§6.2): over many
    /// builds of each game, each list a build states, fact of forms and
    /// choice's fact is off its default in some; the rules accept every
    /// build made. Prints the acceptance and the time.
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
            let choice_facts: Vec<&str> = b.choices.iter().flat_map(|ch| ch.facts()).collect();
            for f in facts::fields(&c) {
                // (What the creator offers: a list a build states, a fact of forms, a choice's.)
                let offered = (matches!(f.ty, FieldType::List(..)) && !b.never_stated(&c, f.name) && !names_navis(f.ty) && facts::role_of(&c, f.name).is_none())
                    || facts::form_facts(&c).iter().any(|x| x.name == f.name)
                    || choice_facts.contains(&f.name);
                if offered {
                    assert!(varied[f.name] > 0, "{game}: {} never varied", f.name);
                }
            }
        }
    }
}

