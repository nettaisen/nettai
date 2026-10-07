//! Recorded battles: per-frame inputs and observable state captured from the
//! original game, as JSON lines, and the comparison the engine is verified
//! with. A round starts with a `{"setup": ...}` line (battle settings, navi
//! stats, RNG), followed by one line per frame. The setup's records decode
//! through [`codec`](crate::codec); the comparison maps the engine's objects
//! to the original's through [`Compat`].

use serde::Deserialize;
use std::io::BufRead;
use std::sync::Arc;

/// The static inputs of a round, captured when its battle state machine
/// starts running.
#[derive(Clone, Debug, Deserialize)]
pub struct Setup {
    /// The game the recording is of: this crate's ([`crate::ROOT`]; another
    /// game's recording is refused, and so is one that names none).
    pub game: String,
    pub frame: u32,
    pub settings_ptr: u32,
    /// BattleSettings (0x10 bytes), hex.
    pub settings: String,
    /// Both players' in-battle navi stats (0x64 bytes each), hex.
    pub navi_stats: [String; 2],
    /// eBattleFolder (0x50 bytes), hex.
    pub folder: String,
    /// BattleState (0xF0 bytes), hex.
    pub battle_state: String,
    pub rng1: u32,
    pub rng2: u32,
    /// The stages of the set's later rounds (`byte_203CA50`, 4 bytes), hex.
    pub stages: String,
    /// Both players' SP navi deletion times (`byte_203EB00`, 0x28 bytes
    /// each), hex.
    pub sp_times: [String; 2],
    /// Both consoles' battle folders (`eBattleFolder`, 0x50 bytes each,
    /// by side), as shuffled at the round's init.
    pub folders: [String; 2],
    /// Both consoles' joypad repeat beats on the round's first frame
    /// (`eJoypad`+0x13).
    pub joypad_phases: [u8; 2],
    /// Both players' games ("gregar" or "falzar"), by side.
    pub game_versions: [String; 2],
    /// Both consoles' regions ("us" or "jp"), by side.
    pub game_regions: [String; 2],
    /// Both players' bug frags (`dword_203F7E0`).
    pub bug_frags: [u32; 2],
    /// Both players' link navi levels (`dword_203CFA0`; 0xFF: no navi
    /// code's level, MegaMan's).
    pub navi_levels: [u8; 2],
    /// Both consoles' save event flag 0x1720 (the NaviCust ran a bug's
    /// routine at load: MegaMan's emotion window flickers, bugs the window
    /// counts in his stats or not). No setup takes it: the engine's rules
    /// make the glitch from the recorded stats (every recorded MegaMan
    /// with the flag has a NaviCust bug in his stats, and none without it
    /// has one; a link navi's flag, left by MegaMan's NaviCust, nothing
    /// reads), and a replay checks theirs against it (`setup_differences`,
    /// for MegaMan's).
    pub emotion_window_glitches: [bool; 2],
    /// Both consoles' installed patch cards (the Japanese games'), each
    /// its save's card list (the card's number, bit 7 set when switched
    /// off). Traces recorded without them have none.
    #[serde(default)]
    pub patch_cards: Option<[Vec<u8>; 2]>,
    /// For a console with patch cards: MegaMan's stats as the card routine
    /// found them (its entry, the last call before the round), hex; none
    /// where it didn't run. The round's setup takes the bytes the cards
    /// write from them ([`CARD_BYTES`]), so that the engine applies the
    /// cards itself.
    #[serde(default)]
    pub navi_stats_before_cards: Option<[Option<String>; 2]>,
    /// The link's delay in ticks, from a packet's sending to its arrival.
    /// Traces recorded without it are an emulated cable's,
    /// [`RECORDED_DELAY`]; matches recorded by Tango's first netplay
    /// engine, which ran each console alone and gave it both players'
    /// packets a tick after they were built, say 1.
    #[serde(default)]
    pub link_delay: Option<u8>,
    /// The recording console's frame counter on the setup's frame (the
    /// halfword its 16-frame sounds go by). Traces recorded without it read
    /// it as the frame number and 2: in a cable recording the counter is
    /// the frame number and 50 (machgun's two rounds), the same modulo 16.
    #[serde(default)]
    pub frame_counter: Option<u16>,
    /// Both consoles' RNG1 on the setup's frame, by side (the recording
    /// console's is also `rng1`), and their tag pairs (BattleState+0x44,
    /// +0x45: the pair is in the folder, and where). Traces recorded
    /// without them know the recording console's only.
    #[serde(default)]
    pub rng1s: Option<[u32; 2]>,
    #[serde(default)]
    pub tag_pairs: Option<[[u8; 2]; 2]>,
    /// Both consoles' Regular-chip flags (BattleState+0x17: the folder's
    /// Regular chip is still to come), by side.
    #[serde(default)]
    pub regular_flags: Option<[u8; 2]>,
    /// Both consoles' save event flag bytes that decide what the custom
    /// screen offers (`eEventFlags`+0x1C, +0x1D and +0x2C: flags 0xE0-0xEF
    /// and 0x160-0x167), hex, by side. Traces recorded without them read
    /// as a finished game's ([`Unlocks::everything`]).
    #[serde(default)]
    pub unlock_flags: Option<[String; 2]>,
}

/// What a save unlocks, from its event flag bytes as the setup records them
/// (`Setup::unlock_flags`): Beast Out (flag 0xE0), the version's five
/// Crosses (`sub_8029EF8`'s table: Gregar's flags 0xE2-0xE6, Falzar's
/// 0xE7-0xEB, by Cross number). (Flag 0x163, a navi code received, is the
/// the rules' `level`: the recordings that have both agree.)
fn unlocks_from_flags(version: GameVersion, flags: &[u8]) -> Unlocks {
    let flag = |f: u16| {
        let byte = match f >> 3 {
            0x1C => flags[0],
            0x1D => flags[1],
            0x2C => flags[2],
            _ => unreachable!("flag {f:#x} isn't recorded"),
        };
        byte & (0x80 >> (f & 7)) != 0
    };
    let first = match version {
        GameVersion::Gregar => 0xE2,
        GameVersion::Falzar => 0xE7,
    };
    Unlocks {
        version,
        crosses: std::array::from_fn(|i| flag(first + i as u16)),
        beast_out: flag(0xE0),
    }
}

/// What the save's reload starts a link navi's stats from (a navi with
/// `levels` that doesn't change form: `reloadCurNaviBaseStats_8120df0`), of
/// `recorded`: its fresh stats (`NaviStats::fresh`) with what the save keeps
/// (`byte_81210C8`: the folder, its Regular and tag chips and the HP; the
/// Regular memory, rules/save's to write). None for any other navi:
/// its stats are as recorded.
pub fn link_navi_reset(content: &Content, recorded: &NaviStats) -> Option<NaviStats> {
    let navi = content.navi(recorded.navi);
    if navi.changes_form() || navi.levels.is_none() {
        return None;
    }
    let fresh = NaviStats::fresh(recorded.navi, content)?;
    Some(NaviStats {
        folder: recorded.folder,
        folder_reg: recorded.folder_reg,
        folder_tags: recorded.folder_tags,
        hp: recorded.hp,
        ..fresh
    })
}

/// A battle object as the trace records it.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Object {
    #[serde(rename = "type")]
    pub kind: u8,
    pub index: u8,
    pub flags: u8,
    pub params: u32,
    /// CurState, CurAction, CurPhase, PhaseInitialized.
    pub state: [u8; 4],
    pub panel: [u8; 2],
    pub alliance: u8,
    pub flip: u8,
    pub hp: u16,
    pub max_hp: u16,
    /// 16.16 fixed point x, y, z.
    pub pos: [i32; 3],
    pub timer: u16,
    pub anim: u8,
    /// Collision ObjectFlags1 (status: invisible, flinching, paralyzed...).
    pub status: u32,
}

/// One battle frame of the original game.
#[derive(Clone, Debug, Deserialize)]
pub struct Frame {
    /// The traced console's game (its round's setup's; not in the line:
    /// [`rounds`] gives a round's frames theirs).
    #[serde(skip)]
    pub console: Option<Game>,
    pub frame: u32,
    /// BattleState bytes 0-3: top state, mode sub-state, and two sub-sub-states.
    pub state: [u8; 4],
    /// Frames since the round started (counts stalled frames).
    pub frames: u32,
    /// Logic ticks since the round started.
    pub ticks: u32,
    /// Link status this frame (2 = inputs arrived, 8 = stalled).
    pub link: u8,
    pub rng1: u32,
    pub rng2: u32,
    /// BattleState (0xF0 bytes), hex.
    pub bs: String,
    /// The fighting-phase machine (0xC bytes at 0x0203CA70), hex.
    pub fight: String,
    /// Custom gauge value (0..0x4000) and fill per tick.
    pub gauge: u16,
    pub gauge_rate: u16,
    /// The battle pause byte.
    pub paused: u8,
    /// HUD update-task mask (bit 4 = gauge fill, bit 15 = banner).
    pub hud_tasks: u32,
    /// Banner state (0x10 bytes at 0x02036840), hex.
    pub banner: String,
    /// Per player: held, pressed, released.
    pub input: [[u16; 3]; 2],
    pub objects: Vec<Object>,
    /// (type, alliance) for the 6x3 field, row-major.
    pub panels: Vec<[u8; 2]>,
    /// Both players' 0x50-byte chip blocks, hex.
    pub chip_blocks: [String; 2],
}

#[derive(Deserialize)]
struct SetupLine {
    setup: Setup,
}

/// What a recording that names no game is refused with: nothing takes it
/// for this game's.
pub const NO_GAME: &str = "a recording that names no game (one recorded before recordings named theirs: the verification workspace's tools/rename-exe.py converts them)";

/// A setup line's setup. Another game's recording is refused, and one that
/// names no game ([`NO_GAME`]); the game is read first, since another
/// game's line hasn't this game's keys. A line that doesn't parse panics,
/// like any malformed line.
fn setup_of(line: &str) -> Result<Setup, String> {
    #[derive(Deserialize)]
    struct Stated {
        game: Option<String>,
    }
    #[derive(Deserialize)]
    struct StatedLine {
        setup: Stated,
    }
    match serde_json::from_str::<StatedLine>(line).unwrap_or_else(|e| panic!("parsing setup: {e}")).setup.game {
        None => return Err(NO_GAME.to_string()),
        Some(game) if game != crate::ROOT => return Err(format!("an {game} recording")),
        Some(_) => {}
    }
    Ok(serde_json::from_str::<SetupLine>(line).unwrap_or_else(|e| panic!("parsing setup: {e}")).setup)
}

/// Decode a hex string from a trace.
pub fn unhex(s: &str) -> Vec<u8> {
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
}

/// The NaviStats bytes the patch cards' routine (and the reload's HP rule
/// after it) writes: the stats, the abilities, the weapons, the first
/// barrier, the gauge, the supports, the bugs, ChpShufl and NumbrOpn, the
/// HP, BugStop's byte and +0x4C.
pub const CARD_BYTES: &[usize] = &[
    0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x0A, 0x0B, 0x0C, 0x0D, 0x10, 0x12, 0x13, 0x14, 0x15, 0x16, 0x18,
    0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1F, 0x23, 0x24, 0x31, 0x40, 0x41, 0x42, 0x43, 0x4C, 0x4D, 0x4F, 0x50, 0x51, 0x52,
    0x54, 0x55, 0x60, 0x61,
];

/// The battle's stats block with the bytes the cards write as the card
/// routine found them: the stats the engine applies the cards to.
pub fn before_cards(recorded: &[u8; 0x64], before: &[u8; 0x64]) -> [u8; 0x64] {
    let mut b = *recorded;
    for &i in CARD_BYTES {
        b[i] = before[i];
    }
    b
}

/// A navi stats block from a trace's hex.
fn navi_stats(hex: &str, ids: &Ids) -> NaviStats {
    codec::navi_stats(&unhex(hex).try_into().expect("a 0x64-byte navi stats block"), ids)
}

// ---- Replaying a trace through the engine -----------------------------------

use crate::{Compat, Game};
use crate::codec::{self, Ids};
use nettai_battle::battle::{Battle, TickEvents};
use nettai_battle::content::Content;
use nettai_battle::console::{Console, ConsoleSetup};
use crate::unlocks::Unlocks;
use crate::GameVersion;
use nettai_battle::custom::{Context, PlayerSetup, Request, Side};
use nettai_battle::hand::ChipHand;
use nettai_battle::input::PlayerTick;
use nettai_battle::kinds::player::Emotion;
use nettai_battle::rng::Rng;
use nettai_battle::setup::{NaviStats, RoundSetup, SetScore};
use nettai_content_api::Value;

/// The recorded sessions' link: an emulated cable, which delivered each
/// console's packet 4 ticks after it went out.
pub const RECORDED_DELAY: u8 = 4;

/// A custom-screen exchange record from a trace.
#[derive(Clone, Debug, Deserialize)]
pub struct Exchange {
    pub frame: u32,
    pub navi_stats: [String; 2],
    pub transform: [String; 2],
}

#[derive(Deserialize)]
struct ExchangeLine {
    exchange: Exchange,
}

/// One round of a trace: its setup, exchanges and frames.
pub struct Round {
    pub setup: Setup,
    pub exchanges: Vec<Exchange>,
    pub frames: Vec<Frame>,
}

/// Split a trace into rounds. Another game's recording is an error
/// ("an exe5 recording"), and so is one that names no game ([`NO_GAME`]).
pub fn rounds(path: impl AsRef<std::path::Path>) -> std::io::Result<Vec<Round>> {
    let f = std::io::BufReader::new(std::fs::File::open(path)?);
    let mut rounds: Vec<Round> = Vec::new();
    let mut pending_exchanges = Vec::new();
    for l in f.lines() {
        let l = l?;
        if l.starts_with("{\"setup\"") {
            let setup = setup_of(&l).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            rounds.push(Round { setup, exchanges: std::mem::take(&mut pending_exchanges), frames: Vec::new() });
        } else if l.starts_with("{\"exchange\"") {
            let e = serde_json::from_str::<ExchangeLine>(&l).expect("exchange").exchange;
            match rounds.last_mut() {
                Some(r) => r.exchanges.push(e),
                None => pending_exchanges.push(e),
            }
        } else if let Some(r) = rounds.last_mut() {
            let mut frame: Frame = serde_json::from_str(&l).expect("frame");
            frame.console = Some(r.console_game());
            r.frames.push(frame);
        }
    }
    Ok(rounds)
}

impl Setup {
    /// The traced console's game: its side's (BattleState+0x0D, the local
    /// side) in `game_versions` and `game_regions`.
    pub fn console_game(&self) -> Game {
        let local = unhex(&self.battle_state)[0x0D] as usize & 1;
        let version = self.game_versions[local].as_str();
        let region = self.game_regions[local].as_str();
        Game::of_names(version, region).unwrap_or_else(|| panic!("a console of version {version:?} and region {region:?}"))
    }
}

impl Round {
    /// The traced console's game (the setup's).
    pub fn console_game(&self) -> Game {
        self.setup.console_game()
    }

    /// The engine's starting point for this round, on `content` (whose
    /// numbers `compat` gives).
    pub fn round_setup(&self, content: &Content, compat: &Compat) -> RoundSetup {
        let ids = Ids::new(content, compat);
        let bs = unhex(&self.setup.battle_state);
        let recorded = |p: usize| match self.stats_before_cards(p) {
            Some(b) => codec::navi_stats(&b, &ids),
            None => navi_stats(&self.setup.navi_stats[p], &ids),
        };
        let mut players: [PlayerSetup; 2] = std::array::from_fn(|p| self.player_setup(p as u8, &ids));
        // What each save brings to its navi's stats (EXE6's rules/save's
        // setup), from the recorded block: the rules write it into a side
        // whose stats they build, which then comes out as recorded.
        for (p, player) in players.iter_mut().enumerate() {
            let s = recorded(p);
            for (field, value) in [
                ("hp", Value::Int(s.max_base_hp as i64)),
                ("reg_up", Value::Int(s.reg_up as i64)),
                ("sun", Value::Bool(s.game_stat(content, "sun") == Some(nettai_content_api::FieldValue::Bool(true)))),
            ] {
                player.set_fact(content, field, &[nettai_battle::rules::Fact::Value(value)]).unwrap_or_else(|e| panic!("the save's {field}: {e}"));
            }
        }
        // A link navi's stats are the rules' to build from its level (EXE6's
        // rules/save runs the save's reload): its fresh stats with what
        // the save keeps, which the replay then compares with the block the
        // console's own reload made.
        let stats = |p: usize| {
            let s = recorded(p);
            match self.setup.navi_levels[p] {
                0xFF => s,
                _ => link_navi_reset(content, &s).unwrap_or(s),
            }
        };
        RoundSetup {
            content: content.hash(),
            settings: codec::battle_settings_of(self.console_game(), &unhex(&self.setup.settings), &ids),
            // EXE6's rules; its systems' setups say what the save
            // unlocks.
            navi_stats: [stats(0), stats(1)],
            rng: self.setup.rng2,
            local_side: bs[0x0D],
            score: SetScore { wins: bs[0x18], losses: bs[0x19], round: bs[0x1A], max_combo: bs[0x1B] },
            later_stages: codec::later_stages(&unhex(&self.setup.stages), &ids),
            low_hp_music_latched: bs[0x20] | bs[0x21] != 0,
            players,
        }
    }

    /// A player's installed patch cards, as the trace has them: the rules'
    /// setup's `patch_cards` (none: the trace has no lists).
    fn patch_cards(&self, side: u8, ids: &Ids) -> Vec<nettai_battle::rules::Fact<'static>> {
        let Some(lists) = &self.setup.patch_cards else { return Vec::new() };
        codec::patch_cards(&lists[side as usize & 1], ids)
    }

    /// For a player with patch cards: the stats the engine applies them to
    /// (the recorded battle stats, with what the cards write as the card
    /// routine found it).
    pub fn stats_before_cards(&self, side: usize) -> Option<[u8; 0x64]> {
        let cards = self.setup.patch_cards.as_ref()?;
        if cards[side].is_empty() {
            return None;
        }
        let before = self.setup.navi_stats_before_cards.as_ref()?[side].as_deref()?;
        let recorded: [u8; 0x64] = unhex(&self.setup.navi_stats[side]).try_into().expect("a 0x64-byte navi stats block");
        Some(before_cards(&recorded, &unhex(before).try_into().expect("a 0x64-byte navi stats block")))
    }

    /// The round's start against the trace: the emotion window's glitch
    /// the rules made for MegaMan (the navi whose window reads it) is the
    /// save's recorded flag, and a player's stats after the engine applied
    /// their patch cards are the recorded ones (the fields the engine
    /// models).
    pub fn setup_differences(&self, b: &Battle, compat: &Compat) -> Vec<String> {
        let ids = Ids::new(&b.content, compat);
        let mut d = Vec::new();
        for side in 0..2 {
            let flags = self.setup.emotion_window_glitches;
            if b.content.navi(b.stats[side].navi).changes_form() && b.consoles[side].emotion_window_glitch != flags[side] {
                d.push(format!(
                    "side {side}'s emotion window glitch as the round is set up: ours {} theirs {}",
                    b.consoles[side].emotion_window_glitch, flags[side]
                ));
            }
            if self.stats_before_cards(side).is_none() {
                continue;
            }
            let mut ours = codec::navi_stats_bytes(&b.stats[side], &ids);
            let theirs = codec::navi_stats_bytes(&navi_stats(&self.setup.navi_stats[side], &ids), &ids);
            // (The version byte, +0x20, is the side's version fact's place
            // among those the rules declare: the battle's start sets it.)
            let recorded: [u8; 0x64] = unhex(&self.setup.navi_stats[side]).try_into().expect("a 0x64-byte navi stats block");
            let place = b.fact(side as u8, nettai_battle::content::PlayerFact::Version).filter(|f| f.stated()).and_then(|f| match f.value() {
                nettai_content_api::FieldValue::Enum(i) => Some(i),
                _ => None,
            });
            ours[0x20] = place.unwrap_or(0);
            let theirs = { let mut t = theirs; t[0x20] = recorded[0x20]; t };
            for i in 0..0x64 {
                if ours[i] != theirs[i] {
                    d.push(format!("side {side}'s stats after its patch cards, +{i:#04x}: ours {:#04x} theirs {:#04x}", ours[i], theirs[i]));
                }
            }
        }
        d
    }

    /// The link's delay in ticks (`Setup::link_delay`).
    pub fn link_delay(&self) -> u8 {
        self.setup.link_delay.unwrap_or(RECORDED_DELAY)
    }

    /// The round's battle at its start on `content`, with the counters
    /// its init carried in.
    pub fn start(&self, content: Arc<Content>, compat: &Compat) -> Battle {
        let mut b = Battle::new(self.round_setup(&content, compat), content);
        let bs = unhex(&self.setup.battle_state);
        b.round.frames = u32::from_le_bytes(bs[0x60..0x64].try_into().unwrap());
        b.round.ticks = u32::from_le_bytes(bs[0x64..0x68].try_into().unwrap());
        b
    }

    /// A player's folder, game and joypad beat.
    fn player_setup(&self, side: u8, ids: &Ids) -> PlayerSetup {
        let bs = unhex(&self.setup.battle_state);
        let local = bs[0x0D] == side;
        let stats = navi_stats(&self.setup.navi_stats[side as usize], ids);
        // BattleState+0x17 is the local console's Regular-chip flag; the
        // other console's is in `regular_flags` when the trace has it, else
        // it follows from its navi's folder (battle mode 0; a later round
        // after the Regular chip's use reads it wrong).
        let regular = match self.setup.regular_flags {
            _ if local => bs[0x17] != 0,
            Some(r) => r[side as usize & 1] != 0,
            None => stats.folder_reg[stats.folder as usize & 1] != 0xFF,
        };
        let folder = codec::battle_folder(&unhex(&self.setup.folders[side as usize]), regular, ids);
        let named = self.setup.game_versions[side as usize].as_str();
        let version = GameVersion::from_name(named).unwrap_or_else(|| panic!("game version {named:?}"));
        let unlocks = match &self.setup.unlock_flags {
            Some(f) => unlocks_from_flags(version, &unhex(&f[side as usize])),
            None => Unlocks::everything(version),
        };
        let mut player = PlayerSetup {
            folder,
            joypad_phase: self.setup.joypad_phases[side as usize],
            console: self.console_setup(side),
            rules: None,
        };
        // The navi and the folder (the engine's facts): the recording's
        // navi, and no folder (the recording's is dealt: `folder` above).
        let navi = nettai_battle::rules::Fact::Value(nettai_content_api::Value::Def(nettai_content_api::Registry::Navi, stats.navi.0));
        player.set_fact(ids.content, "navi", &[navi]).unwrap_or_else(|e| panic!("the navi: {e}"));
        // The patch cards, and no NaviCust: a recording's stats are what its
        // NaviCust made.
        player.set_fact(ids.content, "patch_cards", &self.patch_cards(side, ids)).unwrap_or_else(|e| panic!("the trace's patch cards: {e}"));
        player
            .set_fact(ids.content, "navicust_expansions", &[nettai_battle::rules::Fact::Value(nettai_content_api::Value::Nil)])
            .unwrap_or_else(|e| panic!("no NaviCust: {e}"));
        // (The Crosses the console's save owns, as the list a setup states:
        // those of its navi's of its version, in Cross-number order.)
        unlocks.write(ids.content, stats.navi, &mut player).unwrap_or_else(|e| panic!("the save's unlocks: {e}"));
        // The navi code's level (0xFF: none): the rules' `level`.
        let level = match self.setup.navi_levels[side as usize] {
            0xFF => nettai_content_api::Value::Nil,
            l => nettai_content_api::Value::Int(l as i64),
        };
        player.set_fact(ids.content, "level", &[nettai_battle::rules::Fact::Value(level)]).unwrap_or_else(|e| panic!("the navi code's level: {e}"));
        // The SP navi deletion times: the rules' (their setup's
        // `sp_times`), by the chips compat names the slots by.
        let times = ids.sp_times(&codec::sp_times(&unhex(&self.setup.sp_times[side as usize])));
        codec::write_sp_times(&mut player, ids.content, &times).unwrap_or_else(|e| panic!("the save's SP times: {e}"));
        // The bug frags: the rules' (their setup's `bug_frags`).
        let frags = self.setup.bug_frags[side as usize];
        player
            .set_fact(ids.content, "bug_frags", &[nettai_battle::rules::Fact::Value(nettai_content_api::Value::Int(frags as i64))])
            .unwrap_or_else(|e| panic!("the save's bug frags: {e}"));
        player
    }

    /// A player's console: the recording console's RNG1 and tag pair
    /// (BattleState+0x44/+0x45) as the setup has them. The other console's
    /// are in `rng1s` and `tag_pairs` when the trace has them; without
    /// them its RNG1 reads as 0 and it has no tag pair, which only a
    /// re-deal on that player's screen would read. (The save's emotion
    /// window glitch, event flag 0x1720, which a trace records, is no
    /// setup's: EXE6's rules make it from the recorded stats' NaviCust bugs
    /// and the patch cards, `setup_differences` holding them to the
    /// recorded flag.)
    fn console_setup(&self, side: u8) -> ConsoleSetup {
        let bs = unhex(&self.setup.battle_state);
        // The console's counter before the round's first tick: one less
        // than on the setup's frame, the round's first. The trace gives the
        // recording console's; without it (and for the other console,
        // which isn't recorded) it is the frame number and 2 on a battle
        // frame, which in a cable recording is the counter modulo 16, all
        // its 16-frame sounds read. A recording that starts from a
        // savestate (Tango's first netplay engine) gives the counter.
        let frames = match self.setup.frame_counter {
            Some(c) => (c as u32).wrapping_sub(1) & 0xFFFF,
            None => self.battle_frames().next().map_or(0, |f| f.frame + 1),
        };
        // The other console's RNG1 and tag pair, when the trace has them.
        if bs[0x0D] != side {
            let s = side as usize & 1;
            let rng = self.setup.rng1s.map_or(0, |r| r[s]);
            let tag_pair = self.setup.tag_pairs.and_then(|t| (t[s][0] != 0).then_some(t[s][1]));
            return ConsoleSetup { rng, tag_pair, frames };
        }
        ConsoleSetup { rng: self.setup.rng1, tag_pair: (bs[0x44] != 0).then_some(bs[0x45]), frames }
    }

    /// The frame record with this frame number, if the round has it.
    fn frame(&self, number: u32) -> Option<&Frame> {
        let first = self.frames.first()?.frame;
        let f = self.frames.get(number.checked_sub(first)? as usize)?;
        (f.frame == number).then_some(f).or_else(|| self.frames.iter().find(|f| f.frame == number))
    }

    /// Frames of this round the engine simulates (the running and end
    /// states, up to the next round's init).
    pub fn battle_frames(&self) -> impl Iterator<Item = &Frame> {
        self.frames.iter().filter(|f| f.frame >= self.setup.frame).take_while(|f| f.state[0] == 4 || f.state[0] == 8)
    }

    /// A player's buttons on a frame as the original's fight saw them:
    /// what the link delivered, which the trace records (pressed
    /// `link_delay` frames earlier). The engine has no link: its fight and
    /// both custom screens read them, so its screens run `link_delay`
    /// frames behind the original's, which read the joypad at once, and
    /// their results arrive on the original's frame.
    pub fn joypad(&self, frame: u32, side: usize) -> u16 {
        self.frame(frame).map_or(0, |f| f.input[side][0] & 0x3FF)
    }

    /// The buttons the engine is fed for side `side` on the `i`th of
    /// `frames`, so that it plays as the original did, with no link of its
    /// own. The original's fight read the buttons the link delivered,
    /// `link_delay` frames late (what the trace records, [`Round::joypad`]),
    /// and its custom screen read the joypad at once; the engine's fight and
    /// screens read the same buttons. So the engine is fed:
    ///
    /// - while the fight runs, what the original's fight saw;
    /// - from a custom screen's opening to the side's OK, what the
    ///   original's screen saw (the buttons `link_delay` frames on), so the
    ///   screen plays as the original's;
    /// - for `link_delay` frames from the OK, the buttons of the frame
    ///   before it; then again what the fight sees. The OK reaches the
    ///   engine's screen `link_delay` frames late, so its result goes out
    ///   that much later and, its words over, arrives on the original's
    ///   frame (the screen takes no buttons from its OK on, and the fight
    ///   is paused until the results are in).
    pub fn fed(&self, i: usize, frames: &[&Frame], side: usize) -> u16 {
        let d = self.link_delay() as u32;
        let frame = frames[i].frame;
        let Some(opened) = self.screen_opened(frame) else { return self.joypad(frame, side) };
        match self.screen_ok(opened, side) {
            Some(ok) if frame >= ok + d => self.joypad(frame, side),
            Some(ok) if frame >= ok => self.joypad((ok + d).saturating_sub(1), side),
            _ => self.joypad(frame + d, side),
        }
    }

    /// How many frames late the engine's custom screen of the recording
    /// console's side is on the `i`th of `frames`: `link_delay` from its OK
    /// until the fight resumes ([`Round::fed`]: its OK came that late), else
    /// none. What the screen does then from its OK on (the OK's sound, the
    /// slide-out) is the original's that many frames earlier; what it began
    /// before (a dark chip's fade, stepping on) the original's on the same
    /// frame.
    pub fn screen_late(&self, i: usize, frames: &[&Frame]) -> u32 {
        let d = self.link_delay() as u32;
        let frame = frames[i].frame;
        match self.local_ok(frame) {
            Some(ok) if frame >= ok + d => d,
            _ => 0,
        }
    }

    /// The recording console's side (its setup's BattleState +0x0D).
    fn local_side(&self) -> usize {
        unhex(&self.setup.battle_state)[0x0D] as usize & 1
    }

    /// The recording console's side's OK on the custom screen open on frame
    /// `frame`, if the side has pressed it by the screen's end (none while
    /// the fight runs): the frame the original's screen took it, which the
    /// replay feeds the engine's `link_delay` frames later ([`Round::fed`]).
    /// For the frame comparison's known shift: from it until the engine's
    /// screen takes it, that screen holds the OK back; then it runs
    /// `link_delay` frames behind the original's until the fight resumes
    /// ([`Round::screen_late`]).
    pub fn local_ok(&self, frame: u32) -> Option<u32> {
        self.screen_opened(frame).and_then(|o| self.screen_ok(o, self.local_side()))
    }

    /// The recording console's side's first OK of the round ([`Round::local_ok`]
    /// of its first screen it pressed OK on): from `link_delay` frames
    /// after it on, what counts from that side's send (the HUD's full
    /// gauge's stripes, which "waiting" sets) runs that many frames behind
    /// the original's, to the round's end.
    pub fn first_local_ok(&self) -> Option<u32> {
        let custom = |f: &Frame| f.state[0] == 4 && f.state[1] == 8;
        let mut i = 0;
        while i < self.frames.len() {
            if custom(&self.frames[i]) {
                if let Some(ok) = self.screen_ok(i, self.local_side()) {
                    return Some(ok);
                }
                while i < self.frames.len() && custom(&self.frames[i]) {
                    i += 1;
                }
                continue;
            }
            i += 1;
        }
        None
    }

    /// The index of frame `number` in the round's frames.
    fn index(&self, number: u32) -> Option<usize> {
        let first = self.frames.first()?.frame;
        let i = number.checked_sub(first)? as usize;
        match self.frames.get(i) {
            Some(f) if f.frame == number => Some(i),
            _ => self.frames.iter().position(|f| f.frame == number),
        }
    }

    /// The index of the frame the custom screen open on frame `number`
    /// opened on (the first frame of the running battle's custom mode);
    /// none while the fight runs.
    fn screen_opened(&self, number: u32) -> Option<usize> {
        let custom = |f: &Frame| f.state[0] == 4 && f.state[1] == 8;
        let i = self.index(number)?;
        if !custom(&self.frames[i]) {
            return None;
        }
        let mut o = i;
        while o > 0 && custom(&self.frames[o - 1]) {
            o -= 1;
        }
        Some(o)
    }

    /// The frame side `side`'s custom screen bit, set as the screen opened on
    /// the `opened`th of the round's frames opens, arrives at the recording
    /// console (BattleState +0x14 + side, bit 2): `link_delay` frames after
    /// the screen set it.
    fn screen_bit_arrives(&self, opened: usize, side: usize) -> Option<u32> {
        let screen = self.frames[opened..].iter().take_while(|g| g.state[0] == 4 && g.state[1] == 8);
        screen.map(|g| (g.frame, received_bit(g, side))).find(|&(_, set)| set).map(|(frame, _)| frame)
    }

    /// The frame of side `side`'s OK on the custom screen opened on the
    /// `opened`th of the round's frames, if the side pressed it: found from
    /// the side's custom screen bit as the recording console received it
    /// (BattleState +0x14 + side, bit 2). The screen clears its bit on the
    /// tick after the OK, and the next tick's packet takes it over the
    /// link, `link_delay` frames late: it arrives cleared `2 + link_delay`
    /// frames after the OK.
    fn screen_ok(&self, opened: usize, side: usize) -> Option<u32> {
        let open = |g: &&Frame| received_bit(g, side);
        let screen = self.frames[opened..].iter().take_while(|g| g.state[0] == 4 && g.state[1] == 8);
        let cleared = screen.skip_while(|g| !open(g)).find(|g| !open(g))?;
        cleared.frame.checked_sub(2 + self.link_delay() as u32)
    }

    /// Inputs and events for a frame: the players' buttons (both custom
    /// screens run from them, on the folders the setup records).
    pub fn tick_inputs(&self, i: usize, frames: &[&Frame]) -> ([PlayerTick; 2], TickEvents) {
        let f = frames[i];
        let input = std::array::from_fn(|p| PlayerTick { held: self.fed(i, frames, p) });
        let mut events = TickEvents::default();
        // The link session closed on the tick the end state moved on.
        if i > 0 && f.state[0] == 8 && f.state[1] == 4 && frames[i - 1].state[1] == 0 {
            events.link_closed = true;
        }
        (input, events)
    }
}

/// Differences between the engine and a trace frame. `compat` names the
/// original's object slots for the engine's kinds. (A replay compares with
/// [`compare_at`], which knows what the original's link made differ.)
pub fn compare(b: &Battle, f: &Frame, compat: &Compat) -> Vec<String> {
    compare_with(b, f, Known { banner: f, opening: [false; 2] }, compat)
}

/// What differs between the engine and a recording only as the original's
/// link made it: the engine has none ([`Round::fed`]).
struct Known<'a> {
    /// The trace frame the banner is compared with: the frame's own, but
    /// while the recording console's custom screen runs late
    /// ([`Round::screen_late`]) the one that many frames earlier.
    banner: &'a Frame,
    /// Per side: the custom screen opened within the link's delay, so its
    /// bit is the engine's at once and still on its way in the recording.
    opening: [bool; 2],
}

/// [`compare`] on the `i`th of a round's `frames`, the engine having been fed
/// [`Round::fed`]: what differs only as the original's link made it is
/// compared as the original had it, and nothing else is let go. For
/// `link_delay` frames from a custom screen's opening, the engine's custom
/// screen bits are set at once where the recording console received them
/// that much later; and while the recording console's screen runs late
/// (from its OK until the fight resumes), the banner it shows is the
/// recording's that many frames earlier.
pub fn compare_at(b: &Battle, round: &Round, frames: &[&Frame], i: usize, compat: &Compat) -> Vec<String> {
    let f = frames[i];
    let delay = round.link_delay() as u32;
    let late = round.screen_late(i, frames);
    let banner = if late > 0 { round.frame(f.frame - late).unwrap_or(f) } else { f };
    let opened = round.screen_opened(f.frame);
    let opening = std::array::from_fn(|side| {
        opened.and_then(|o| round.screen_bit_arrives(o, side)).is_some_and(|arrives| (arrives.saturating_sub(delay)..arrives).contains(&f.frame))
    });
    compare_with(b, f, Known { banner, opening }, compat)
}

fn compare_with(b: &Battle, f: &Frame, known: Known, compat: &Compat) -> Vec<String> {
    let mut d = Vec::new();
    let mut check = |what: &str, ours: String, theirs: String| {
        if ours != theirs {
            d.push(format!("{what}: ours {ours} theirs {theirs}"));
        }
    };
    let r = &b.round;
    check("state", format!("{:?}", [r.top, r.mode, r.sub, r.init]), format!("{:?}", f.state));
    check("ticks", r.ticks.to_string(), f.ticks.to_string());
    check("rng2", format!("{:#010x}", b.rng.state), format!("{:#010x}", f.rng2));
    check("paused", (b.paused as u8).to_string(), f.paused.to_string());
    let bs = unhex(&f.bs);
    if bs.len() >= 0xF0 {
        check("battle flags", format!("{:#x}", r.flags), format!("{:#x}", u16::from_le_bytes([bs[0x32], bs[0x33]])));
        check("intro bits", format!("{:#x}", r.intro_bits), format!("{:#x}", bs[0x5C]));
        check("alive", format!("{:?}", r.alive), format!("{:?}", [bs[0x12], bs[0x13]]));
        check("turn", r.turn.to_string(), bs[7].to_string());
        check("battle time", r.battle_time.to_string(), u32::from_le_bytes(bs[0x40..0x44].try_into().unwrap()).to_string());
    }
    check("gauge", format!("{:#x}", b.gauge.value), format!("{:#x}", f.gauge));
    if bs.len() >= 0xF0 {
        // (On a screen's opening, the engine's bits are set and the
        // recording console's still clear.)
        let received = [bs[0x14], bs[0x15]];
        let theirs: [u8; 2] = std::array::from_fn(|p| if known.opening[p] && received[p] & 4 == 0 { received[p] | 4 } else { received[p] });
        check("custom screens open (as received)", format!("{:?}", b.round.remote_status), format!("{theirs:?}"));
    }
    // The banner: its task (bit 15 of the HUD's tasks) and, while it shows,
    // the number the traced console's record holds (its +1: the banner
    // `sub_801E792` started, 0 for a telop), which for a result's is the
    // console's own (`sub_80081A4`, `sub_800825A`).
    let task = (known.banner.hud_tasks >> 15) & 1 != 0;
    check("banner", (b.banner.active as u8).to_string(), (task as u8).to_string());
    if let (Some(id), true) = (b.banner_for(r.local_side), task) {
        let ours = match b.telop_for(r.local_side) {
            Some(_) => Some(0),
            None => b.content.assets.number(nettai_content_api::AssetKind::Banner, id.0).map(|n| n.id as u8),
        };
        let theirs = unhex(&known.banner.banner).get(1).copied();
        let show = |n: Option<u8>| n.map_or("none".to_string(), |n| format!("{n:#04x}"));
        check("banner number", show(ours), show(theirs));
    }
    // Objects whose X and Y the engine doesn't know, and hit sparks with
    // their hitter's garbage Z fraction, are compared without them, on
    // both sides (matched by list position).
    let order: Vec<nettai_battle::object::ObjectRef> = b.objects.in_order().collect();
    let unknown: Vec<Unknown> = order
        .iter()
        .map(|&o| Unknown { xy: nettai_battle::kinds::effect::xy_unknown(b, o), z_fraction: spark_z_fraction_unknown(b, compat, o) })
        .collect();
    // The traced console's game (its round's setup's): a Gregar or a
    // Japanese console's objects keep its own ROM's addresses where the
    // content has the US Falzar's (games.toml).
    let game = f.console.expect("a round's frame (trace::rounds gives it its console)");
    let ours: Vec<String> = order.iter().zip(&unknown).map(|(&o, &u)| describe(b, compat, o, u, game)).collect();
    let theirs: Vec<String> =
        f.objects.iter().enumerate().map(|(i, o)| describe_trace(compat, o, unknown.get(i).copied().unwrap_or_default())).collect();
    if ours != theirs {
        check(
            "objects",
            format!("\n    ours   {}", ours.join("\n           ")),
            format!("\n    theirs {}", theirs.join("\n           ")),
        );
    }
    let panels: Vec<[u8; 2]> = (1..=3)
        .flat_map(|y| (1..=6).map(move |x| (x, y)))
        .map(|(x, y)| {
            let p = b.field.panel(x, y).unwrap();
            [p.kind.0, p.alliance]
        })
        .collect();
    check("panels", format!("{panels:?}"), format!("{:?}", f.panels));
    let ids = Ids::new(&b.content, compat);
    for p in 0..2 {
        let ours = codec::chip_hand_bytes(&b.hands[p], &ids).iter().map(|x| format!("{x:02x}")).collect::<String>();
        check(&format!("hand {p}"), ours, f.chip_blocks[p].clone());
    }
    d
}

/// What of an object's position the engine can't know (and the
/// comparison skips on both sides).
#[derive(Clone, Copy, Default)]
struct Unknown {
    /// X and Y (`effect::xy_unknown`).
    xy: bool,
    /// Z's fraction (`spark_z_fraction_unknown`).
    z_fraction: bool,
}

/// A hit spark (effect object #4) starts at its hitter's position moved by
/// whole pixels (`object_spawnCollisionEffect`), so it keeps its hitter's
/// Z fraction, which is garbage when the hitter's kind keeps a spawner's
/// register there (`scratch_z_fraction`: Sensor's laser). The spark
/// outlives a hitter that ends with the battle (the lab's
/// chips/0x071-sensor1/ko): a freed slot keeps its object's kind until it
/// is used again, as in the game, and that kind is asked.
fn spark_z_fraction_unknown(b: &Battle, compat: &Compat, r: nettai_battle::object::ObjectRef) -> bool {
    use nettai_battle::object::Pool;
    if compat.object_slot(b, r) != Ok((Pool::Effect, 4)) {
        return false;
    }
    let Some(hitter) = b.objects.get(r).related[0] else { return false };
    compat
        .object_slot(b, hitter)
        .is_ok_and(|(pool, index)| compat.kind_at(pool, index).is_some_and(|(_, k)| k.scratch_z_fraction))
}

/// One object's observable state as the comparison sees it.
#[allow(clippy::too_many_arguments)]
fn describe_fields(
    compat: &Compat,
    kind: u8,
    index: u8,
    flags: u8,
    state: [u8; 4],
    panel: [u8; 2],
    alliance: u8,
    hp: [u16; 2],
    pos: [i32; 3],
    timer: u16,
    anim: u8,
    status: u32,
    unknown: Unknown,
) -> String {
    let pos = if pos_is_garbage(compat, kind, index, flags) {
        "-".to_string()
    } else if unknown.xy {
        format!("-,-,{}", pos[2])
    } else if unknown.z_fraction || z_fraction_is_garbage(compat, kind, index) {
        format!("{},{},{}+?", pos[0], pos[1], pos[2] >> 16)
    } else {
        format!("{},{},{}", pos[0], pos[1], pos[2])
    };
    let flags = flags & !open_bus_flags(compat, kind, index);
    format!(
        "T{kind}#{index:#04x} f{flags:#04x} s{state:?} p{},{} a{alliance} hp{}/{} pos{pos} t{timer} an{anim} st{status:#x}",
        panel[0], panel[1], hp[0], hp[1]
    )
}

/// The kind in an object slot of the trace's numbering (type 1, 3, 4).
fn slot_kind(compat: &Compat, kind: u8, index: u8) -> Option<&crate::KindEntry> {
    use nettai_battle::object::Pool;
    let pool = match kind {
        1 => Pool::Actor,
        3 => Pool::Attack,
        4 => Pool::Effect,
        _ => return None,
    };
    compat.kind_at(pool, index).map(|(_, e)| e)
}

/// Positions that are register garbage in the game and never read, by
/// the kinds' compat entries (`scratch_position`): the intro sequencer's
/// (objects-and-player.md §A.4), a palette flash's (§A.7), the navi chip
/// controller's, spawned with the user's panel Y, the element and the
/// spawner's address as X, Y and Z (chips.md §3.6), the dimming chips'
/// controllers and the other content kinds that say so; and a charge
/// glow's before its first unpaused update, while it has no sprite yet
/// (`scratch_position_without_sprite`, §A.5). The X and Y of effects the
/// engine marks as not knowing them are skipped too
/// (`effect::xy_unknown`): the second deletion explosion, which the game
/// spawns with the object allocator's list-node addresses as X and Y
/// (§A.3).
fn pos_is_garbage(compat: &Compat, kind: u8, index: u8, flags: u8) -> bool {
    slot_kind(compat, kind, index).is_some_and(|k| {
        k.scratch_position
            || (k.scratch_position_without_sprite && flags & nettai_battle::object::flags::NO_SPRITE_UPDATE != 0)
    })
}

/// The header-flag bits a kind's spawner reads from the console's open bus
/// (`open_bus_flags`): the boulder's, whose value depends on whether an
/// interrupt came just before the read, so each console may hold another.
/// Nothing the game does with them differs (objects-and-player.md §2).
fn open_bus_flags(compat: &Compat, kind: u8, index: u8) -> u8 {
    slot_kind(compat, kind, index).map_or(0, |k| k.open_bus_flags)
}

/// Kinds that keep the fraction of the Z their spawner left in a register
/// (`scratch_z_fraction`: DustCross's junk ball, whose is the low half of
/// a RAM address): only their whole pixels are compared.
fn z_fraction_is_garbage(compat: &Compat, kind: u8, index: u8) -> bool {
    slot_kind(compat, kind, index).is_some_and(|k| k.scratch_z_fraction)
}

/// An object's Z as the `game` console has it (`Games::z`): a Z that is the
/// spawner's address, or keeps its low half, that game's; and while a kind
/// drops from such a Z (`Games::drop_z_offset`), it and what follows it (an
/// attachment, whose first related object it is) that game's drop.
fn console_z(b: &Battle, compat: &Compat, r: nettai_battle::object::ObjectRef, game: Game) -> i32 {
    let o = b.objects.get(r);
    let mapped = compat.games.z(game, o.pos.z);
    if mapped != o.pos.z {
        return mapped;
    }
    let drop = |d: nettai_battle::object::ObjectRef| {
        let d = b.objects.get(d);
        compat.games.drop_z_offset(game, &b.content.defs.kind(d.kind).key, d.pos.z, d.timer)
    };
    let offset = match drop(r) {
        0 => o.related[0].map_or(0, drop),
        own => own,
    };
    o.pos.z.wrapping_add(offset)
}

fn describe(b: &Battle, compat: &Compat, r: nettai_battle::object::ObjectRef, unknown: Unknown, game: Game) -> String {
    let o = b.objects.get(r);
    let status = o.collision.map(|c| b.collision.get(c).f1).unwrap_or(0);
    // The engine's identities as the original's numbers: the object's kind
    // as its slot, a navi's content action as its action number.
    let (index, action) = match (compat.object_slot(b, r), compat.navi_action(b, r)) {
        (Ok((_, index)), Ok(action)) => (index, action),
        (Err(e), _) | (_, Err(e)) => {
            let kind = &b.content.defs.kind(o.kind).key;
            return format!("T{} {kind}: {e}", crate::pool_type(r.pool));
        }
    };
    describe_fields(
        compat,
        crate::pool_type(r.pool),
        index,
        o.flags,
        [o.state, action, o.phase, o.phase_init],
        [o.panel.x, compat.games.panel_y(game, &b.content.defs.kind(o.kind).key, o.panel.y)],
        o.alliance,
        [o.hp, o.max_hp],
        [compat.games.x(game, &b.content.defs.kind(o.kind).key, o.pos.x), o.pos.y, console_z(b, compat, r, game)],
        o.timer,
        o.anim,
        status,
        unknown,
    )
}

fn describe_trace(compat: &Compat, o: &Object, unknown: Unknown) -> String {
    let hp = [o.hp, o.max_hp];
    describe_fields(compat, o.kind, o.index, o.flags, o.state, o.panel, o.alliance, hp, o.pos, o.timer, o.anim, o.status, unknown)
}

/// Run a round through the engine on `content`; returns the number of
/// frames that matched before the first difference, and that difference.
pub fn run_round(round: &Round, content: &Arc<Content>, compat: &Compat) -> (usize, Option<(u32, Vec<String>)>) {
    let frames: Vec<&Frame> = round.battle_frames().collect();
    let mut b = round.start(content.clone(), compat);
    let setup = round.setup_differences(&b, compat);
    if !setup.is_empty() {
        return (0, Some((round.setup.frame, setup)));
    }
    for i in 0..frames.len() {
        let (input, events) = round.tick_inputs(i, &frames);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| b.tick(&input, events)));
        if let Err(e) = result {
            let msg = e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()));
            return (i, Some((frames[i].frame, vec![format!("engine panicked: {}", msg.unwrap_or_default())])));
        }
        let diffs = compare_at(&b, round, &frames, i, compat);
        if !diffs.is_empty() {
            return (i, Some((frames[i].frame, diffs)));
        }
    }
    (frames.len(), None)
}

// ---- The custom screens alone ----------------------------------------------

/// The emotion a custom screen sees (`sub_8015B64`), as far as the trace's
/// stats tell it; the screen asks only whether the navi is worn out (no
/// Cross, no Beast Out) or tired (Beast Out becomes Beast Over). Worn out:
/// mood 0, or past a Beast Over this round (AIData+0x36, which also keeps
/// the mood from dropping to 0). Tired: its Beast Out turns are spent
/// (NaviStats+0x21 is 0) and it is out of the Beast (the turn's check that
/// raises AIData+0x32 has run: EXE6's rules/beast's `turn_check`). Not seen: anger, and
/// the NaviCust emotion bug's swings to tired, which need the fight.
fn screen_emotion(stats: &NaviStats, content: &Content, beast_over_before: bool) -> Emotion {
    let kind = crate::forms::kind(content, stats.form);
    let name = if stats.mood == 0 || (beast_over_before && kind != Some(crate::forms::Kind::BeastOver)) {
        "worn_out"
    } else if stats.game_stat(content, "beast_out_counter") == Some(nettai_content_api::FieldValue::U8(0)) && !kind.is_some_and(crate::forms::Kind::is_beast) {
        "tired"
    } else {
        "normal"
    };
    content.rules().emotion.by_name(name).unwrap_or_else(|| panic!("EXE6's emotions have no {name}"))
}

/// One player's custom screen checked against a trace.
#[derive(Clone, Debug)]
pub struct ScreenCheck {
    pub side: u8,
    /// The frame the screen opened, and the one the fight resumed on.
    pub opened: u32,
    pub resumed: u32,
    /// The frame the player pressed OK.
    pub confirmed: Option<u32>,
    pub differences: Vec<String>,
}

/// Run every custom screen of a round on its own, from the players'
/// recorded buttons, and compare what each player sends with the trace:
/// the hand (as installed), the transformation, when their status bit
/// arrives cleared, and when the fight resumes. The fight is not
/// simulated: each screen reads its navi's stats from the trace, and the
/// emotions the screen asks about from them (`screen_emotion`). Damage
/// from a formula is not checked (it needs the battle).
pub fn check_custom_screens(round: &Round, content: &Arc<Content>, compat: &Compat) -> Vec<ScreenCheck> {
    let ids = Ids::new(content, compat);
    let frames: Vec<&Frame> = round.battle_frames().collect();
    let setup = round.round_setup(content, compat);
    // A battle for the screens' extras (the sides' rules' `custom`
    // hooks): their state through the round, and the stats and turn each
    // screen reads, set from the trace as it opens.
    let mut battle = Battle::new(setup.clone(), content.clone());
    let mut sides: [Side; 2] = std::array::from_fn(|p| Side::new(&setup.players[p]));
    // Each console's RNG as far as the screens alone go: its draws outside
    // them (camera shakes, the emotion window) aren't simulated here. The
    // recording console's are in the trace instead, which samples its RNG
    // on every frame after the battle's update, before the main loop's
    // draw: each frame starts from the frame before's sample.
    let mut consoles = setup.players.each_ref().map(|p| Console::new(&p.console));
    let recording = unhex(&round.setup.battle_state)[0x0D] as usize;
    let mut checks = Vec::new();
    let mut open: Option<(u32, [Option<u32>; 2], [Option<u32>; 2])> = None;
    let stats_at = |frame: u32, p: usize| -> NaviStats {
        let e = round.exchanges.iter().rfind(|e| e.frame <= frame).expect("exchange record");
        navi_stats(&e.navi_stats[p], &ids)
    };
    for (i, f) in frames.iter().enumerate() {
        if i > 0 {
            consoles[recording].rng = Rng::new(frames[i - 1].rng1);
            // The previous frame's main-loop draw.
            for c in &mut consoles {
                c.rng.next();
            }
        }
        let context = |p: usize| {
            let stats = stats_at(f.frame, p);
            let beast_over_before = round
                .exchanges
                .iter()
                .take_while(|e| e.frame <= f.frame)
                .any(|e| crate::forms::kind(content, navi_stats(&e.navi_stats[p], &ids).form) == Some(crate::forms::Kind::BeastOver));
            let emotion = screen_emotion(&stats, content, beast_over_before);
            Context {
                library: &**content,
                stats,
                emotion,
                turn: unhex(&f.bs)[7],
                own_gauges: false,
                random_battle: false,
                late_turns: false,
                now: f.frame,
            }
        };
        for (p, side) in sides.iter_mut().enumerate() {
            side.joypad.update(round.fed(i, &frames, p));
        }
        let custom = f.state[0] == 4 && f.state[1] == 8;
        let prev_init = i.checked_sub(1).map(|j| frames[j].state[3]);
        if custom && f.state[3] == 1 && prev_init == Some(0) {
            for (p, side) in sides.iter_mut().enumerate() {
                let ctx = context(p);
                battle.stats[p] = ctx.stats;
                battle.round.turn = ctx.turn;
                side.open_with(&ctx, &mut consoles[p], &mut battle.custom_extras(p as u8, ctx.emotion));
            }
            open = Some((f.frame, [None; 2], [None; 2]));
            continue;
        }
        let Some((opened, confirmed, cleared)) = open.as_mut() else { continue };
        if custom {
            for (p, side) in sides.iter_mut().enumerate() {
                let was_open = side.in_custom;
                let ctx = context(p);
                battle.stats[p] = ctx.stats;
                battle.round.turn = ctx.turn;
                let damage = |id| {
                    let d = content.chip(id).damage;
                    if d < 1000 { d } else { 0 }
                };
                let request = side.tick_with(&ctx, &mut consoles[p], damage, &mut battle.custom_extras(p as u8, ctx.emotion));
                if request == Some(Request::Confirm) {
                    confirmed[p] = Some(f.frame);
                }
                if was_open && !side.in_custom {
                    cleared[p] = Some(f.frame);
                }
            }
        }
        let resumes = frames.get(i + 1).is_some_and(|n| custom && n.state[1] == 0x0C);
        if !resumes {
            continue;
        }
        // The fight resumes next frame: this frame installed the results.
        let before = frames[i.saturating_sub(1)];
        let arrivals: Vec<u32> = sides.iter().filter_map(|s| s.sent.as_ref().map(|x| x.arrives)).collect();
        for (p, side) in sides.iter().enumerate() {
            let mut d = Vec::new();
            match &side.sent {
                None => d.push("never sent".to_string()),
                Some(sent) => {
                    let block = codec::chip_hand(&unhex(&f.chip_blocks[p]), &ids);
                    let expected =
                        sent.result.hand.clone().unwrap_or_else(|| codec::chip_hand(&unhex(&before.chip_blocks[p]), &ids));
                    let formula = |h: &ChipHand, k: usize| h.ids[k].is_some_and(|id| content.chip(id).formula.is_some());
                    let mut ours = expected.clone();
                    for k in 0..6 {
                        if formula(&ours, k) {
                            ours.damage[k] = block.damage[k];
                        }
                    }
                    if ours != block {
                        d.push(format!("hand: ours {:?} theirs {:?}", ours, block));
                    }
                    let e = round.exchanges.iter().rfind(|e| e.frame <= f.frame).expect("exchange record");
                    let theirs = codec::transform_request(&unhex(&e.transform[p]), &ids);
                    if sent.result.transform.form != theirs.form {
                        d.push(format!("transformation: ours {:?} theirs {:?}", sent.result.transform.form, theirs.form));
                    }
                }
            }
            // The status bit's clearing reaches both consoles 1 + link_delay
            // frames after it happens in the original: on the frame after the
            // engine's, whose OK came link_delay frames late (`Round::fed`).
            let screen = frames[..=i].iter().filter(|g| g.frame > *opened);
            let set = |g: &&&Frame| unhex(&g.bs)[0x14 + p] & 4 != 0;
            let theirs_cleared = screen.skip_while(|g| !set(g)).find(|g| !set(g)).map(|g| g.frame);
            let ours_cleared = cleared[p].map(|c| c + 1);
            if ours_cleared != theirs_cleared {
                d.push(format!("status bit arrives cleared: ours {ours_cleared:?} theirs {theirs_cleared:?}"));
            }
            if arrivals.iter().max() != Some(&f.frame) {
                d.push(format!("results in: ours {:?} theirs {}", arrivals.iter().max(), f.frame));
            }
            checks.push(ScreenCheck { side: p as u8, opened: *opened, resumed: f.frame + 1, confirmed: confirmed[p], differences: d });
        }
        open = None;
    }
    checks
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A recording is this game's when its setup says so: another game's
    /// is refused by name.
    #[test]
    fn another_games_recording_is_refused() {
        let e = setup_of(r#"{"setup": {"game": "exe5", "frame": 10}}"#).unwrap_err();
        assert_eq!(e, "an exe5 recording");
    }

    /// A setup that names no game is no recording of this game's: nothing
    /// takes it for one, and the refusal says what it is.
    #[test]
    fn a_setup_names_its_game() {
        let e = setup_of(r#"{"setup": {"frame": 72, "game_versions": ["falzar", "falzar"]}}"#).unwrap_err();
        assert!(e.starts_with("a recording that names no game"), "{e}");
    }

    #[test]
    fn unlocks_from_event_flags() {
        // The chip lab's Falzar save: Beast Out and every Cross.
        assert_eq!(unlocks_from_flags(GameVersion::Falzar, &[0x81, 0xF3, 0x00]), Unlocks::everything(GameVersion::Falzar));
        // Its Gregar save: Gregar's Crosses are flags 0xE2-0xE6.
        assert_eq!(unlocks_from_flags(GameVersion::Gregar, &[0xBE, 0x03, 0x00]), Unlocks::everything(GameVersion::Gregar));
        // TomahawkCross alone (custom/one-cross-owned).
        let u = unlocks_from_flags(GameVersion::Falzar, &[0x80, 0x83, 0x00]);
        assert_eq!(u.crosses, [false, true, false, false, false]);
        assert!(u.beast_out);
        // No Beast Out (custom/no-beast-out).
        assert!(!unlocks_from_flags(GameVersion::Falzar, &[0x01, 0xF3, 0x00]).beast_out);
    }

    #[test]
    fn a_boulders_open_bus_flag_bits_are_skipped() {
        let compat = Compat::exe6();
        let flags = |kind, index, flags| {
            describe_fields(compat, kind, index, flags, [4, 0, 0, 0], [5, 3], 1, [500, 500], [0; 3], 0, 0, 0, Unknown::default())
        };
        // The column-5 boulder of stage 0x18 as each console read it (side
        // 0's after an interrupt): the same object to the comparison.
        assert_eq!(flags(3, 0x6E, 0xD4), flags(3, 0x6E, 0x34));
        // Only bits 0x20, 0x40 and 0x80, and only the boulder's.
        assert_ne!(flags(3, 0x6E, 0x36), flags(3, 0x6E, 0x34));
        assert_ne!(flags(3, 0x59, 0xD4), flags(3, 0x59, 0x34));
    }

    /// Django's drop on a JP Gregar console (jp/chips/0x116-django/
    /// gregar-ride): from 60 pixels up with its own spawner's address as
    /// the fraction, a tick into the drop it is 5616 (16.16) above the
    /// content's, and level with it once it lands.
    #[test]
    fn django_drops_from_each_consoles_spawner_address() {
        let games = &Compat::exe6().games;
        let drop = |z0: i32, t: i32| {
            let v = (-z0 + 0x12_C000) / 10;
            z0 + t * v - 0x6000 * (t * (t - 1) / 2)
        };
        let (ours, theirs) = ((60 << 16) | 0xD6A3, (60 << 16) | 0xEF03);
        for t in 1..=10 {
            let z = drop(ours, t);
            let timer = (10 - t) as u16;
            assert_eq!(z + games.drop_z_offset(Game::JpGregar, "django/navi", z, timer), drop(theirs, t), "tick {t}");
            assert_eq!(games.drop_z_offset(Game::JpFalzar, "django/navi", z, timer), 0);
        }
        assert_eq!(games.drop_z_offset(Game::JpGregar, "django/navi", drop(ours, 1), 9), 5616);
        assert_eq!(drop(ours, 10), drop(theirs, 10));
    }
}

/// Side `side`'s custom screen bit as the recording console received it on
/// frame `g` (BattleState +0x14 + side, bit 2).
fn received_bit(g: &Frame, side: usize) -> bool {
    g.bs.get(2 * (0x14 + side)..2 * (0x15 + side)).and_then(|h| u8::from_str_radix(h, 16).ok()).is_some_and(|b| b & 4 != 0)
}
