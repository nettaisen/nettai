//! How a round went, for §9's measures (docs/design/training-ai.md): the
//! result, HP taken and lost, hits taken, attack chips and the share that
//! landed, chips picked a screen, steps back to the panel just left
//! (pacing), forms taken. An env measures every round it plays
//! ([`Measure`]); the pool hands the rounds over (`round_stats`), and
//! [`Table`] sums them by game and seats.

use nettai_battle::Battle;
use nettai_battle::battle::mode;
use nettai_battle::custom::screen::Phase;
use nettai_content_api::PanelPos;

/// The ticks after an attack chip's use within which the other navi losing
/// HP counts as it landing.
pub const LANDS: u32 = 120;
/// The ticks within which a step back to the panel just left counts as
/// pacing (a third of a second).
pub const BACK: u32 = 20;

/// One round, as the pool reports it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RoundStats {
    /// The env that played it (the pool's numbering).
    pub env: usize,
    pub game: String,
    /// Who played each seat: "policy" or a scripted player's name.
    pub controllers: [String; 2],
    pub navis: [String; 2],
    pub delay: [u16; 2],
    /// +1, −1 or 0 by seat (0 for a round cut off or stopped).
    pub result: [i8; 2],
    pub truncated: bool,
    /// How it ended where it wasn't cut (`env::BY_KO`, `BY_JUDGE`,
    /// `BY_LIMIT`).
    pub ending: u8,
    /// Why the engine stopped the round, if it did.
    pub error: Option<String>,
    pub ticks: u32,
    pub fight_ticks: u32,
    pub max_hp: [u16; 2],
    /// HP each side lost in all (heals taken off).
    pub hp_lost: [i32; 2],
    /// Ticks on which each side's navi lost HP.
    pub hits_taken: [u32; 2],
    pub chips_used: [u32; 2],
    /// Chips used that attack (a damage, or a damage formula), and of
    /// those the ones after which the other navi lost HP within
    /// [`LANDS`] ticks.
    pub attack_chips: [u32; 2],
    pub attacks_landed: [u32; 2],
    pub screens: u32,
    pub chips_picked: [u32; 2],
    pub steps: [u32; 2],
    pub steps_back: [u32; 2],
    /// Screens each side left with a form chosen.
    pub forms_taken: [u32; 2],
    /// Times each side's navi changed from its game's base form to another
    /// (a Cross, a Soul, Beast Out: however it came to), and the fight
    /// ticks it spent out of the base form.
    pub forms_entered: [u32; 2],
    pub form_ticks: [u32; 2],
    /// The practice options the round played with (§6.3).
    pub full_gauge: bool,
    pub folder_order: bool,
    /// By seat, for a folder seeded with a pair (§6.2): the pair (setup's
    /// key, payoff's), whether both were picked, whether picked on one
    /// screen the setup first, whether used in that order within
    /// [`PAIR_USE`] ticks, and the most the payoff did (the other navi's HP
    /// lost within [`LANDS`] ticks of its use) as a ratio to its plain
    /// fingerprint's best.
    pub pair: [Option<(String, String)>; 2],
    pub pair_picked: [bool; 2],
    pub pair_ordered: [bool; 2],
    pub pair_used: [bool; 2],
    pub payoff_damage: [Option<f32>; 2],
}

/// The ticks within which a payoff used after its setup counts as the pair
/// used (five seconds).
pub const PAIR_USE: u32 = 300;

/// What a seat's watched pair has done so far.
#[derive(Clone, Debug, Default)]
struct PairWatch {
    pair: Option<crate::fingerprint::Pair>,
    plain: u16,
    picked: [bool; 2],
    setup_used: Option<u32>,
    /// The payoff's uses: the tick and the other navi's HP then.
    payoff_uses: Vec<(u32, u16)>,
}

/// What an env follows of a round as it is played.
#[derive(Clone, Debug, Default)]
pub struct Measure {
    pub stats: RoundStats,
    hp: [Option<u16>; 2],
    hand: [u8; 2],
    at: [Option<PanelPos>; 2],
    last_step: [Option<(PanelPos, PanelPos, u32)>; 2],
    flying: Vec<(u8, u32, u16)>,
    closing: [bool; 2],
    custom: bool,
    in_form: [bool; 2],
    pairs: [PairWatch; 2],
}

/// Side `side`'s navi's HP: none before it is on the field, 0 once it has
/// left it.
pub fn navi_hp(b: &Battle, side: u8, before: Option<u16>) -> Option<u16> {
    match b.player(side).filter(|&r| b.objects.is_allocated(r)) {
        Some(r) => Some(b.objects.get(r).hp),
        None => before.map(|_| 0),
    }
}

impl Measure {
    /// A round starts.
    pub fn new(stats: RoundStats) -> Measure {
        Measure { stats, ..Measure::default() }
    }

    /// Seat `seat`'s folder was seeded with `pair`, whose payoff's plain
    /// fingerprint's best damage is `plain`.
    pub fn watch_pair(&mut self, seat: usize, pair: crate::fingerprint::Pair, plain: u16, content: &nettai_battle::Content) {
        self.pairs[seat] = PairWatch { pair: Some(pair), plain, ..PairWatch::default() };
        self.stats.pair[seat] = Some((content.defs.chip(pair.setup).key.clone(), content.defs.chip(pair.payoff).key.clone()));
    }

    /// After tick `t` of `b`.
    pub fn tick(&mut self, b: &Battle, t: u32) {
        let s = &mut self.stats;
        s.ticks = t;
        let custom = b.round.mode == mode::CUSTOM;
        if b.round.mode == mode::FIGHTING {
            s.fight_ticks += 1;
        }
        if custom && !self.custom {
            s.screens += 1;
            self.closing = [false; 2];
        }
        self.custom = custom;
        for side in 0..2u8 {
            let i = side as usize;
            let hp = navi_hp(b, side, self.hp[i]);
            if let Some(r) = b.player(side).filter(|&r| b.objects.is_allocated(r)) {
                s.max_hp[i] = s.max_hp[i].max(b.objects.get(r).max_hp);
            }
            if let (Some(was), Some(now)) = (self.hp[i], hp) {
                s.hp_lost[i] += was as i32 - now as i32;
                if now < was {
                    s.hits_taken[i] += 1;
                }
            }
            self.hp[i] = hp;
            // Forms: out of the base form, however it came to.
            let in_form = b.player(side).is_some() && !b.form(i).base;
            if in_form && !self.in_form[i] {
                s.forms_entered[i] += 1;
            }
            if in_form && b.round.mode == mode::FIGHTING {
                s.form_ticks[i] += 1;
            }
            self.in_form[i] = in_form;
            // Chips used in the fight.
            let hand = b.hands[i].remaining();
            let used_now = (b.round.mode == mode::FIGHTING && hand < self.hand[i]).then(|| b.hands[i].ids[b.hands[i].cursor.saturating_sub(1) as usize]).flatten();
            if b.round.mode == mode::FIGHTING && hand < self.hand[i] {
                s.chips_used[i] += 1;
                let used = b.hands[i].ids[b.hands[i].cursor.saturating_sub(1) as usize];
                if used.is_some_and(|c| {
                    let d = b.content.chip(c);
                    d.damage > 0 || d.formula.is_some()
                }) {
                    s.attack_chips[i] += 1;
                    if let Some(other) = navi_hp(b, side ^ 1, Some(0)) {
                        self.flying.push((side, t, other));
                    }
                }
            }
            self.hand[i] = hand;
            // A watched pair's use: the setup, then the payoff.
            if let (Some(p), Some(used)) = (self.pairs[i].pair, used_now) {
                let used = Some(used);
                if used == Some(p.setup) {
                    self.pairs[i].setup_used = Some(t);
                }
                if used == Some(p.payoff) {
                    if self.pairs[i].setup_used.is_some_and(|u| t - u <= PAIR_USE) {
                        s.pair_used[i] = true;
                    }
                    if let Some(other) = navi_hp(b, side ^ 1, Some(0)) {
                        self.pairs[i].payoff_uses.push((t, other));
                    }
                }
            }
            // Steps, and steps back to the panel just left.
            let at = b.player(side).filter(|&r| b.objects.is_allocated(r)).map(|r| b.objects.get(r).panel);
            if let (Some(from), Some(to)) = (self.at[i], at)
                && from != to
                && b.round.mode == mode::FIGHTING
            {
                s.steps[i] += 1;
                if self.last_step[i].is_some_and(|(was, left_to, when)| was == to && left_to == from && t - when <= BACK) {
                    s.steps_back[i] += 1;
                }
                self.last_step[i] = Some((from, to, t));
            }
            self.at[i] = at;
            // Chips picked and forms chosen, as the screen closes.
            if let Some(screen) = b.custom.sides[i].screen.filter(|_| custom)
                && matches!(screen.phase, Phase::Closing { .. })
                && !self.closing[i]
            {
                self.closing[i] = true;
                s.chips_picked[i] += screen.selected as u32;
                s.forms_taken[i] += screen.form.is_some() as u32;
                // (A watched pair's picks: the hand OK built lists them in
                // pick order.)
                if let (Some(p), Some((Some(hand), _))) = (self.pairs[i].pair, &b.custom.sides[i].built) {
                    let picked: Vec<_> = hand.selection.iter().flatten().map(|c| c.id).collect();
                    let (at_setup, at_payoff) = (picked.iter().position(|&c| c == p.setup), picked.iter().position(|&c| c == p.payoff));
                    self.pairs[i].picked[0] |= at_setup.is_some();
                    self.pairs[i].picked[1] |= at_payoff.is_some();
                    s.pair_picked[i] = self.pairs[i].picked == [true, true];
                    s.pair_ordered[i] |= matches!((at_setup, at_payoff), (Some(a), Some(b)) if a < b);
                }
            }
        }
        let hp = self.hp;
        let stats = &mut self.stats;
        // A payoff's damage: the most the other navi lost within its window.
        for (i, w) in self.pairs.iter_mut().enumerate() {
            let Some(now) = hp[i ^ 1] else { continue };
            let plain = w.plain;
            let best = &mut stats.payoff_damage[i];
            w.payoff_uses.retain(|&(when, before)| {
                let r = crate::fingerprint::ratio(before.saturating_sub(now), plain);
                *best = Some(best.unwrap_or(0.0).max(r));
                t - when < LANDS
            });
        }
        self.flying.retain(|&(side, when, other)| {
            if hp[(side ^ 1) as usize].is_some_and(|now| now < other) {
                stats.attacks_landed[side as usize] += 1;
                return false;
            }
            t - when < LANDS
        });
    }
}

/// Rounds summed, by game and the seats' controllers: the measures of §9
/// as a table.
#[derive(Clone, Debug, Default)]
pub struct Table {
    rows: Vec<Row>,
}

#[derive(Clone, Debug, Default)]
struct Row {
    game: String,
    controllers: [String; 2],
    rounds: u32,
    wins: [u32; 2],
    truncated: u32,
    errors: u32,
    fight_ticks: u64,
    hp_lost: [i64; 2],
    hits: [u32; 2],
    attack_chips: [u32; 2],
    landed: [u32; 2],
    picked: [u32; 2],
    screens: u32,
    steps_back: [u32; 2],
    forms: [u32; 2],
}

impl Table {
    pub fn add(&mut self, r: &RoundStats) {
        let at = match self.rows.iter().position(|x| x.game == r.game && x.controllers == r.controllers) {
            Some(i) => i,
            None => {
                self.rows.push(Row { game: r.game.clone(), controllers: r.controllers.clone(), ..Row::default() });
                self.rows.len() - 1
            }
        };
        let row = &mut self.rows[at];
        row.rounds += 1;
        row.truncated += r.truncated as u32;
        row.errors += r.error.is_some() as u32;
        row.fight_ticks += r.fight_ticks as u64;
        row.screens += r.screens;
        for i in 0..2 {
            row.wins[i] += (r.result[i] > 0) as u32;
            row.hp_lost[i] += r.hp_lost[i] as i64;
            row.hits[i] += r.hits_taken[i];
            row.attack_chips[i] += r.attack_chips[i];
            row.landed[i] += r.attacks_landed[i];
            row.picked[i] += r.chips_picked[i];
            row.steps_back[i] += r.steps_back[i];
            row.forms[i] += r.forms_taken[i];
        }
    }

    /// The table, a row a game and pairing: rounds won by each seat
    /// (truncated, stopped), HP taken and lost a minute of fight by the
    /// left seat, hits it took, its attack chips a minute and the share
    /// that landed, chips it picked a screen, its steps back a minute, and
    /// forms it took.
    pub fn print(&self) -> String {
        let mut out = String::from("| Game | Left | Right | Rounds | Won–lost (cut, stopped) | HP taken / lost a minute | Hits taken | Attack chips a minute, landed | Chips a screen | Steps back a minute | Forms |\n|---|---|---|---|---|---|---|---|---|---|---|\n");
        for r in &self.rows {
            let minutes = (r.fight_ticks as f64 / 3600.0).max(1e-9);
            let share = if r.attack_chips[0] > 0 { 100.0 * r.landed[0] as f64 / r.attack_chips[0] as f64 } else { 0.0 };
            out += &format!(
                "| {} | {} | {} | {} | {}–{} ({}, {}) | {:.0} / {:.0} | {} | {:.1}, {:.0}% | {:.1} | {:.1} | {} |\n",
                r.game,
                r.controllers[0],
                r.controllers[1],
                r.rounds,
                r.wins[0],
                r.wins[1],
                r.truncated,
                r.errors,
                r.hp_lost[1] as f64 / minutes,
                r.hp_lost[0] as f64 / minutes,
                r.hits[0],
                r.attack_chips[0] as f64 / minutes,
                share,
                r.picked[0] as f64 / r.screens.max(1) as f64,
                r.steps_back[0] as f64 / minutes,
                r.forms[0],
            );
        }
        out
    }
}
