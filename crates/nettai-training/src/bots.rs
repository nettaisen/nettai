//! Scripted players (docs/design/training-ai.md §9): what the learner is
//! measured against and part of its league. Each plays any game: it reads
//! the navis' panels and its own hand, and presses what a player would (the
//! duelist reads the attacks on the field too: an opponent, it may read
//! what the learner can't). In
//! the fight none opens the custom screen (the env does, §6.3: the env
//! strips their L and R); on the screen each picks the chips it can in slot
//! order and presses OK ([`crate::act::scripted_pointer`]).

use nettai_battle::Battle;
use nettai_battle::input::keys;
use nettai_netplay::rng::SplitMix64;
use nettai_netplay::standin::Masher;

/// A scripted player, by the name the bridge and `set_seat` take.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BotKind {
    /// Presses nothing in the fight.
    Still,
    /// Moves about at random and never attacks.
    Mover,
    /// Presses at random what a player fights with: moves, shoots and
    /// charges, uses its chips (`standin::Masher`).
    Masher,
    /// Walks to the other navi's row and attacks it there: shoots every
    /// [`SHOOT_EVERY`] ticks and uses its chip at once.
    Shooter,
    /// Steps up or down (or stays) every [`DODGE_EVERY`] ticks, never
    /// attacks.
    Dodger,
    /// Plays as a competent player does, the opponent that punishes what the
    /// others let pass: every [`DECIDE_EVERY`] ticks it tries its options
    /// out on a copy of the battle (the other navi pressing nothing): it uses
    /// its chip where that lands, fires its charged buster where that lands,
    /// else takes the step (or none) that costs it least HP over the next
    /// ticks, toward the other navi's row now and then. Its buster charges
    /// all along.
    Duelist,
}

impl BotKind {
    pub const ALL: [BotKind; 6] = [BotKind::Still, BotKind::Mover, BotKind::Masher, BotKind::Shooter, BotKind::Dodger, BotKind::Duelist];

    pub fn name(self) -> &'static str {
        match self {
            BotKind::Still => "still",
            BotKind::Mover => "mover",
            BotKind::Masher => "masher",
            BotKind::Shooter => "shooter",
            BotKind::Dodger => "dodger",
            BotKind::Duelist => "duelist",
        }
    }

    pub fn named(name: &str) -> Option<BotKind> {
        BotKind::ALL.into_iter().find(|k| k.name() == name)
    }
}

/// The ticks between a shooter's shots.
pub const SHOOT_EVERY: u32 = 12;
/// The ticks between a dodger's steps.
pub const DODGE_EVERY: u32 = 40;
/// How often the duelist decides, and how far ahead it tries a step, its
/// shot and its chip out on a copy of the battle.
pub const DECIDE_EVERY: u32 = 4;
const LOOK_TICKS: u32 = 24;
const CHIP_LOOK_TICKS: u32 = 48;
/// The ticks the duelist holds B before it fires: a charged shot.
const CHARGE_TICKS: u32 = 50;

const MOVES: u16 = keys::UP | keys::DOWN | keys::LEFT | keys::RIGHT;

/// A scripted player's state in the fight.
#[derive(Clone, Debug)]
pub struct Bot {
    pub kind: BotKind,
    t: u32,
    rng: SplitMix64,
    masher: Masher,
    held: u16,
    last: u16,
    /// The duelist's: ticks B has been held, the panel it is stepping from
    /// and the direction, and ticks it has pressed it.
    charge: u32,
    step: Option<(nettai_content_api::PanelPos, u16, u32)>,
}

impl Bot {
    pub fn new(kind: BotKind, seed: u64) -> Bot {
        let mut masher = Masher::new(seed);
        masher.buster = true;
        Bot { kind, t: 0, rng: SplitMix64::new(seed ^ 0x5eed), masher, held: 0, last: 0, charge: 0, step: None }
    }

    /// Its buttons this tick of the fight for side `side`'s navi in `b`.
    pub fn fight(&mut self, b: &Battle, side: u8) -> u16 {
        self.t += 1;
        let pressed = match self.kind {
            BotKind::Still => 0,
            BotKind::Mover => self.masher.buttons() & MOVES,
            BotKind::Masher => self.masher.buttons(),
            BotKind::Shooter => self.shooter(b, side),
            BotKind::Dodger => self.dodger(),
            BotKind::Duelist => self.duelist(b, side),
        };
        self.last = pressed;
        pressed & !(keys::L | keys::R)
    }

    /// The panel of side `side`'s navi.
    fn at(b: &Battle, side: u8) -> Option<nettai_content_api::PanelPos> {
        b.player(side).filter(|&r| b.objects.is_allocated(r)).map(|r| b.objects.get(r).panel)
    }

    fn shooter(&mut self, b: &Battle, side: u8) -> u16 {
        let (Some(at), Some(them)) = (Bot::at(b, side), Bot::at(b, side ^ 1)) else { return 0 };
        // (A press is let go the tick after; a step is held until the row is reached, ten ticks at most.)
        if self.last != 0 {
            self.held += 1;
            if self.last & MOVES != 0 && self.held < 10 && at.y != them.y {
                return self.last;
            }
            return 0;
        }
        self.held = 0;
        if at.y != them.y {
            return if them.y < at.y { keys::UP } else { keys::DOWN };
        }
        if b.hands[side as usize].remaining() > 0 {
            return keys::A;
        }
        if self.t % SHOOT_EVERY < 2 { keys::B } else { 0 }
    }

    /// The panel a step `dir` from `at` reaches, if it is on the field and
    /// side `side`'s.
    fn stepped(b: &Battle, side: u8, at: nettai_content_api::PanelPos, dir: u16) -> Option<(u8, u8)> {
        let (x, y) = match dir {
            keys::UP => (at.x as i32, at.y as i32 - 1),
            keys::DOWN => (at.x as i32, at.y as i32 + 1),
            keys::LEFT => (at.x as i32 - 1, at.y as i32),
            keys::RIGHT => (at.x as i32 + 1, at.y as i32),
            _ => (at.x as i32, at.y as i32),
        };
        if !(1..=6).contains(&x) || !(1..=3).contains(&y) {
            return None;
        }
        let (x, y) = (x as u8, y as u8);
        b.field.panel(x, y).filter(|p| p.alliance == side).map(|_| (x, y))
    }

    /// Each side's navi's HP in `b` (0 once it is gone).
    fn hps(b: &Battle) -> [i32; 2] {
        [0u8, 1].map(|s| b.player(s).filter(|&r| b.objects.is_allocated(r)).map_or(0, |r| b.objects.get(r).hp as i32))
    }

    /// What pressing `first` for `press` ticks, then `then`, does over `ticks` ticks of a copy of `b`, the other
    /// side pressing nothing: the HP side `side` loses and the HP the other loses.
    fn try_out(b: &Battle, side: u8, first: u16, press: u32, then: u16, ticks: u32) -> (i32, i32) {
        let mut c = b.clone();
        let before = Bot::hps(&c);
        for t in 0..ticks {
            let mine = if t < press { first } else { then };
            let mut held = [0u16; 2];
            held[side as usize] = mine;
            if !crate::env::step_battle(&mut c, held) || c.is_battle_over() || c.round.mode != b.round.mode {
                break;
            }
        }
        let after = Bot::hps(&c);
        let (me, them) = (side as usize, (side ^ 1) as usize);
        (before[me] - after[me], before[them] - after[them])
    }

    fn duelist(&mut self, b: &Battle, side: u8) -> u16 {
        let (Some(at), Some(them)) = (Bot::at(b, side), Bot::at(b, side ^ 1)) else { return 0 };
        self.charge += 1;
        // (Between decisions, what it chose is held: a step till the panel changes, ten ticks at most.)
        if let Some((from, dir, held)) = self.step {
            if at == from && held < 10 {
                self.step = Some((from, dir, held + 1));
                return keys::B | dir;
            }
            self.step = None;
        }
        if self.t % DECIDE_EVERY != 0 {
            return keys::B;
        }
        // Its chip, where using it now lands (the other navi staying put), costing it no more than staying.
        let (stay_loss, _) = Bot::try_out(b, side, keys::B, 0, keys::B, LOOK_TICKS);
        if b.hands[side as usize].remaining() > 0 && (at.y as i32 - them.y as i32).abs() <= 1 {
            let (loss, dealt) = Bot::try_out(b, side, keys::A | keys::B, 2, keys::B, CHIP_LOOK_TICKS);
            if dealt > 0 && loss <= stay_loss {
                return keys::A | keys::B;
            }
        }
        // Its charged shot, where it lands.
        if self.charge >= CHARGE_TICKS && at.y == them.y {
            let (_, dealt) = Bot::try_out(b, side, 0, 1, keys::B, LOOK_TICKS);
            if dealt > 0 {
                self.charge = 0;
                return 0;
            }
        }
        // The step that costs it least over the next ticks; at no cost, toward the other navi's row.
        let toward = if them.y < at.y { keys::UP } else if them.y > at.y { keys::DOWN } else { 0 };
        let mut best = (stay_loss, 1, 0u16);
        for dir in [keys::UP, keys::DOWN, keys::LEFT, keys::RIGHT] {
            if Bot::stepped(b, side, at, dir).is_none() {
                continue;
            }
            let (loss, _) = Bot::try_out(b, side, dir | keys::B, 3, keys::B, LOOK_TICKS);
            let rank = (loss, if dir == toward { 0 } else { 2 }, dir);
            if (rank.0, rank.1) < (best.0, best.1) {
                best = rank;
            }
        }
        // (Toward the row only now and then: a step every decision would leave it no time to attack.)
        if best.2 != 0 && (best.0 < stay_loss || self.rng.below(3) == 0) {
            self.step = Some((at, best.2, 0));
            return keys::B | best.2;
        }
        keys::B
    }

    fn dodger(&mut self) -> u16 {
        if self.t.is_multiple_of(DODGE_EVERY) {
            self.held = [0, keys::UP, keys::DOWN][self.rng.below(3) as usize];
        } else if self.t % DODGE_EVERY >= 3 {
            self.held = 0;
        }
        self.held
    }
}
