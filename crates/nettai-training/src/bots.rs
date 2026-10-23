//! Scripted players (docs/design/training-ai.md §9): what the learner is
//! measured against and part of its league. Each plays any game: it reads
//! the navis' panels and its own hand, and presses what a player would. In
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
}

impl BotKind {
    pub const ALL: [BotKind; 5] = [BotKind::Still, BotKind::Mover, BotKind::Masher, BotKind::Shooter, BotKind::Dodger];

    pub fn name(self) -> &'static str {
        match self {
            BotKind::Still => "still",
            BotKind::Mover => "mover",
            BotKind::Masher => "masher",
            BotKind::Shooter => "shooter",
            BotKind::Dodger => "dodger",
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
}

impl Bot {
    pub fn new(kind: BotKind, seed: u64) -> Bot {
        let mut masher = Masher::new(seed);
        masher.buster = true;
        Bot { kind, t: 0, rng: SplitMix64::new(seed ^ 0x5eed), masher, held: 0, last: 0 }
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

    fn dodger(&mut self) -> u16 {
        if self.t.is_multiple_of(DODGE_EVERY) {
            self.held = [0, keys::UP, keys::DOWN][self.rng.below(3) as usize];
        } else if self.t % DODGE_EVERY >= 3 {
            self.held = 0;
        }
        self.held
    }
}
