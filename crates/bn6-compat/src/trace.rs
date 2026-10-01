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
    /// Traces recorded without it leave the stages unknown.
    #[serde(default)]
    pub stages: Option<String>,
    /// Both players' SP navi deletion times (`byte_203EB00`, 0x28 bytes
    /// each), hex. Traces recorded without them read as the best times.
    #[serde(default)]
    pub sp_times: Option<[String; 2]>,
    /// Both consoles' battle folders (`eBattleFolder`, 0x50 bytes each,
    /// by side), as shuffled at the round's init. Traces recorded without
    /// them have only the local console's (`folder`).
    #[serde(default)]
    pub folders: Option<[String; 2]>,
    /// Both consoles' joypad repeat beats on the round's first frame
    /// (`eJoypad`+0x13). Traces recorded without them read as the frame
    /// number modulo 5, which is what the recording tool's consoles show.
    #[serde(default)]
    pub joypad_phases: Option<[u8; 2]>,
    /// Both players' games ("gregar" or "falzar"), by side. Traces
    /// recorded without them go by the Crosses and Beast Outs the players
    /// send, else Falzar.
    #[serde(default)]
    pub game_versions: Option<[String; 2]>,
    /// Both players' bug frags (`dword_203F7E0`). Traces recorded without
    /// them read as `RECORDED_BUG_FRAGS`.
    #[serde(default)]
    pub bug_frags: Option<[u32; 2]>,
    /// Both players' link navi levels (`dword_203CFA0`). Traces recorded
    /// without them read as 0.
    #[serde(default)]
    pub navi_levels: Option<[u8; 2]>,
}

/// The bug frags a trace without them reads as: the recording tool's
/// saves have frags to spare (their dark chips are used as themselves),
/// so the most a save holds.
pub const RECORDED_BUG_FRAGS: u32 = 9999;

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
    #[serde(default)]
    pub bs: String,
    /// The fighting-phase machine (0xC bytes at 0x0203CA70), hex.
    #[serde(default)]
    pub fight: String,
    /// Custom gauge value (0..0x4000) and fill per tick.
    #[serde(default)]
    pub gauge: u16,
    #[serde(default)]
    pub gauge_rate: u16,
    /// The battle pause byte.
    #[serde(default)]
    pub paused: u8,
    /// HUD update-task mask (bit 4 = gauge fill, bit 15 = banner).
    #[serde(default)]
    pub hud_tasks: u32,
    /// Banner state (0x10 bytes at 0x02036840), hex.
    #[serde(default)]
    pub banner: String,
    /// Per player: held, pressed, released.
    pub input: [[u16; 3]; 2],
    pub objects: Vec<Object>,
    /// (type, alliance) for the 6x3 field, row-major.
    pub panels: Vec<[u8; 2]>,
    /// Both players' 0x50-byte chip blocks, hex.
    pub chip_blocks: [String; 2],
}

#[derive(Clone, Debug)]
pub enum Line {
    Setup(Setup),
    Frame(Box<Frame>),
}

#[derive(Deserialize)]
struct SetupLine {
    setup: Setup,
}

/// Read a trace file, one line at a time.
pub fn read(path: impl AsRef<std::path::Path>) -> std::io::Result<impl Iterator<Item = Line>> {
    let f = std::io::BufReader::new(std::fs::File::open(path)?);
    Ok(f.lines().map(|l| {
        let l = l.expect("reading trace");
        if l.starts_with("{\"setup\"") {
            Line::Setup(serde_json::from_str::<SetupLine>(&l).expect("parsing setup").setup)
        } else {
            Line::Frame(Box::new(serde_json::from_str(&l).expect("parsing frame")))
        }
    }))
}

/// Decode a hex string from a trace.
pub fn unhex(s: &str) -> Vec<u8> {
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
}

/// A navi stats block from a trace's hex.
fn navi_stats(hex: &str, ids: &Ids) -> NaviStats {
    codec::navi_stats(&unhex(hex).try_into().expect("a 0x64-byte navi stats block"), ids)
}

// ---- Replaying a trace through the engine -----------------------------------

use crate::Compat;
use crate::codec::{self, Ids};
use bn6_battle::battle::{Battle, CustomResult, TickEvents};
use bn6_battle::content::Content;
use bn6_battle::console::{Console, ConsoleSetup};
use bn6_battle::custom::{Context, GameVersion, PlayerSetup, Recorded, Request, Side, Unlocks};
use bn6_battle::hand::ChipHand;
use bn6_battle::input::PlayerTick;
use bn6_battle::link::Link;
use bn6_battle::rng::Rng;
use bn6_battle::setup::{NaviStats, RoundSetup, SetScore};

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

/// Split a trace into rounds.
pub fn rounds(path: impl AsRef<std::path::Path>) -> std::io::Result<Vec<Round>> {
    let f = std::io::BufReader::new(std::fs::File::open(path)?);
    let mut rounds: Vec<Round> = Vec::new();
    let mut pending_exchanges = Vec::new();
    for l in f.lines() {
        let l = l?;
        if l.starts_with("{\"setup\"") {
            let setup = serde_json::from_str::<SetupLine>(&l).expect("setup").setup;
            rounds.push(Round { setup, exchanges: std::mem::take(&mut pending_exchanges), frames: Vec::new() });
        } else if l.starts_with("{\"exchange\"") {
            let e = serde_json::from_str::<ExchangeLine>(&l).expect("exchange").exchange;
            match rounds.last_mut() {
                Some(r) => r.exchanges.push(e),
                None => pending_exchanges.push(e),
            }
        } else if let Some(r) = rounds.last_mut() {
            r.frames.push(serde_json::from_str(&l).expect("frame"));
        }
    }
    Ok(rounds)
}

impl Round {
    /// The engine's starting point for this round, on `content` (whose
    /// numbers `compat` gives).
    pub fn round_setup(&self, content: &Content, compat: &Compat) -> RoundSetup {
        let ids = Ids::new(content, compat);
        let bs = unhex(&self.setup.battle_state);
        let stats = |s: &str| navi_stats(s, &ids);
        RoundSetup {
            content: content.hash(),
            settings: codec::battle_settings(&unhex(&self.setup.settings), &ids),
            navi_stats: [stats(&self.setup.navi_stats[0]), stats(&self.setup.navi_stats[1])],
            rng: self.setup.rng2,
            local_side: bs[0x0D],
            score: SetScore { wins: bs[0x18], losses: bs[0x19], round: bs[0x1A], max_combo: bs[0x1B] },
            // Unknown stages read as entry 0. A replay never gets to use
            // them: the tick that chains the next round is that round's
            // init, which isn't among the battle frames.
            later_stages: self.setup.stages.as_deref().map(|s| codec::later_stages(&unhex(s), &ids)).unwrap_or_default(),
            low_hp_music_latched: bs[0x20] | bs[0x21] != 0,
            sp_times: match &self.setup.sp_times {
                Some([a, b]) => [codec::sp_times(&unhex(a)), codec::sp_times(&unhex(b))],
                None => Default::default(),
            },
            players: std::array::from_fn(|p| self.player_setup(p as u8, &ids)),
            link_delay: Link::RECORDED_DELAY,
        }
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

    /// Whether the trace has this player's folder (else their custom
    /// screen can't be simulated).
    pub fn folder_known(&self, side: u8) -> bool {
        self.setup.folders.is_some() || unhex(&self.setup.battle_state)[0x0D] == side
    }

    /// A player's folder, game and joypad beat, as far as the trace knows.
    fn player_setup(&self, side: u8, ids: &Ids) -> PlayerSetup {
        let bs = unhex(&self.setup.battle_state);
        let local = bs[0x0D] == side;
        let stats = navi_stats(&self.setup.navi_stats[side as usize], ids);
        // BattleState+0x17 is the local console's Regular-chip flag; the
        // other console's follows from its navi's folder (battle mode 0).
        let regular = if local { bs[0x17] != 0 } else { stats.folder_reg[stats.folder as usize & 1] != 0xFF };
        let folder = match &self.setup.folders {
            Some(f) => Some(codec::battle_folder(&unhex(&f[side as usize]), regular, ids)),
            None if local => Some(codec::battle_folder(&unhex(&self.setup.folder), regular, ids)),
            None => None,
        };
        let version = match &self.setup.game_versions {
            Some(v) => match v[side as usize].as_str() {
                "gregar" => GameVersion::Gregar,
                "falzar" => GameVersion::Falzar,
                other => panic!("game version {other:?}"),
            },
            None => self.sent_version(side),
        };
        PlayerSetup {
            folder,
            unlocks: Unlocks::everything(version),
            joypad_phase: self.setup.joypad_phases.map(|p| p[side as usize]).unwrap_or((self.setup.frame % 5) as u8),
            bug_frags: self.setup.bug_frags.map_or(RECORDED_BUG_FRAGS, |f| f[side as usize]),
            navi_level: self.setup.navi_levels.map_or(0, |l| l[side as usize]),
            console: self.console_setup(side),
        }
    }

    /// A player's console: the recording console's RNG1 and tag pair
    /// (BattleState+0x44/+0x45) as the setup has them. The other console's
    /// aren't recorded: its RNG1 reads as 0 and it has no tag pair, which
    /// only a re-deal on that player's screen would read. The save's
    /// emotion window glitch (event flag 0x1720) isn't recorded either and
    /// reads as clear.
    fn console_setup(&self, side: u8) -> ConsoleSetup {
        let bs = unhex(&self.setup.battle_state);
        if bs[0x0D] != side {
            return ConsoleSetup::default();
        }
        ConsoleSetup { rng: self.setup.rng1, tag_pair: (bs[0x44] != 0).then_some(bs[0x45]), emotion_window_glitch: false }
    }

    /// A player's game, going by the transformations they send: Gregar's
    /// Crosses are forms 1-5 and its Beast Out 0x0B.
    fn sent_version(&self, side: u8) -> GameVersion {
        let gregar = |f: u8| matches!(f, 1..=5 | 0x0B | 0x0D..=0x11 | 0x17);
        let falzar = |f: u8| matches!(f, 6..=0x0A | 0x0C | 0x12..=0x16 | 0x18);
        for e in &self.exchanges {
            let form = unhex(&e.transform[side as usize])[0];
            if gregar(form) {
                return GameVersion::Gregar;
            }
            if falzar(form) {
                return GameVersion::Falzar;
            }
        }
        GameVersion::Falzar
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

    /// A player's buttons on a frame. The trace records the input the
    /// link delivered, which the players pressed `RECORDED_DELAY` frames
    /// earlier.
    pub fn joypad(&self, frame: u32, side: usize) -> u16 {
        self.frame(frame + Link::RECORDED_DELAY as u32).map_or(0, |f| f.input[side][0] & 0x3FF)
    }

    /// Inputs and events for a frame (the players' results decoded with
    /// `ids`).
    pub fn tick_inputs(&self, i: usize, frames: &[&Frame], ids: &Ids) -> ([PlayerTick; 2], TickEvents) {
        let f = frames[i];
        let input = std::array::from_fn(|p| PlayerTick { held: self.joypad(f.frame, p) });
        let mut events = TickEvents::default();
        // The link session closed on the tick the end state moved on.
        if i > 0 && f.state[0] == 8 && f.state[1] == 4 && frames[i - 1].state[1] == 0 {
            events.link_closed = true;
        }
        // A player whose folder the trace lacks: their custom screen's
        // status (as it arrives `RECORDED_DELAY` frames later) and, on
        // the tick before the mode leaves the custom screen, their result.
        for p in 0..2 {
            if self.folder_known(p as u8) {
                continue;
            }
            let arriving = self.frame(f.frame + Link::RECORDED_DELAY as u32).unwrap_or(f);
            let in_custom = unhex(&arriving.bs).get(0x14 + p).copied().unwrap_or(0) & 4 != 0;
            let mut result = None;
            if let Some(next) = frames.get(i + 1)
                && f.state[1] == 8
                && next.state[1] == 0x0C
            {
                let e = self.exchanges.iter().rfind(|e| e.frame <= f.frame).expect("exchange record");
                let navi_stats = navi_stats(&e.navi_stats[p], ids);
                let transform = codec::transform_request(&unhex(&e.transform[p]), ids);
                let hand = Some(codec::chip_hand(&unhex(&f.chip_blocks[p]), ids));
                result = Some(Box::new(CustomResult { hand, navi_stats, transform }));
            }
            events.recorded[p] = Some(Recorded { in_custom, result });
        }
        (input, events)
    }
}

/// Differences between the engine and a trace frame. `compat` names the
/// original's object slots for the engine's kinds.
pub fn compare(b: &Battle, f: &Frame, compat: &Compat) -> Vec<String> {
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
        check("custom screens open (as received)", format!("{:?}", b.round.remote_status), format!("{:?}", [bs[0x14], bs[0x15]]));
    }
    check("banner", (b.banner.active as u8).to_string(), ((f.hud_tasks >> 15) & 1).to_string());
    // Objects whose X and Y the engine doesn't know, and hit sparks with
    // their hitter's garbage Z fraction, are compared without them, on
    // both sides (matched by list position).
    let order: Vec<bn6_battle::object::ObjectRef> = b.objects.in_order().collect();
    let unknown: Vec<Unknown> = order
        .iter()
        .map(|&o| Unknown { xy: bn6_battle::kinds::effect::xy_unknown(b, o), z_fraction: spark_z_fraction_unknown(b, compat, o) })
        .collect();
    let ours: Vec<String> = order.iter().zip(&unknown).map(|(&o, &u)| describe(b, compat, o, u)).collect();
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
            [p.kind as u8, p.alliance]
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
fn spark_z_fraction_unknown(b: &Battle, compat: &Compat, r: bn6_battle::object::ObjectRef) -> bool {
    use bn6_battle::object::Pool;
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
    format!(
        "T{kind}#{index:#04x} f{flags:#04x} s{state:?} p{},{} a{alliance} hp{}/{} pos{pos} t{timer} an{anim} st{status:#x}",
        panel[0], panel[1], hp[0], hp[1]
    )
}

/// The kind in an object slot of the trace's numbering (type 1, 3, 4).
fn slot_kind(compat: &Compat, kind: u8, index: u8) -> Option<&crate::KindEntry> {
    use bn6_battle::object::Pool;
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
            || (k.scratch_position_without_sprite && flags & bn6_battle::object::flags::NO_SPRITE_UPDATE != 0)
    })
}

/// Kinds that keep the fraction of the Z their spawner left in a register
/// (`scratch_z_fraction`: DustCross's junk ball, whose is the low half of
/// a RAM address): only their whole pixels are compared.
fn z_fraction_is_garbage(compat: &Compat, kind: u8, index: u8) -> bool {
    slot_kind(compat, kind, index).is_some_and(|k| k.scratch_z_fraction)
}

fn describe(b: &Battle, compat: &Compat, r: bn6_battle::object::ObjectRef, unknown: Unknown) -> String {
    let o = b.objects.get(r);
    let status = o.collision.map(|c| b.collision.get(c).f1).unwrap_or(0);
    // The engine's identities as the original's numbers: the object's kind
    // as its slot, a navi's content action as its action number.
    let (index, action) = match (compat.object_slot(b, r), compat.navi_action(b, r)) {
        (Ok((_, index)), Ok(action)) => (index, action),
        (Err(e), _) | (_, Err(e)) => {
            let kind = &b.content.defs.kind(o.kind).key;
            return format!("T{} {kind}: {e}", r.pool.type_number());
        }
    };
    describe_fields(
        compat,
        r.pool.type_number(),
        index,
        o.flags,
        [o.state, action, o.phase, o.phase_init],
        [o.panel.x, o.panel.y],
        o.alliance,
        [o.hp, o.max_hp],
        [o.pos.x, o.pos.y, o.pos.z],
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
    let ids = Ids::new(content, compat);
    for i in 0..frames.len() {
        let (input, events) = round.tick_inputs(i, &frames, &ids);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| b.tick(&input, events)));
        if let Err(e) = result {
            let msg = e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()));
            return (i, Some((frames[i].frame, vec![format!("engine panicked: {}", msg.unwrap_or_default())])));
        }
        let diffs = compare(&b, frames[i], compat);
        if !diffs.is_empty() {
            return (i, Some((frames[i].frame, diffs)));
        }
    }
    (frames.len(), None)
}

// ---- The custom screens alone ----------------------------------------------

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
/// simulated: each screen reads its navi's stats from the trace, and
/// emotions from the mood alone (a tired navi is not seen). Damage from a
/// formula is not checked (it needs the battle).
pub fn check_custom_screens(round: &Round, content: &Content, compat: &Compat) -> Vec<ScreenCheck> {
    let ids = Ids::new(content, compat);
    let frames: Vec<&Frame> = round.battle_frames().collect();
    let setup = round.round_setup(content, compat);
    let mut sides: [Option<Side>; 2] =
        std::array::from_fn(|p| round.folder_known(p as u8).then(|| Side::new(&setup.players[p])));
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
            let emotion = if stats.mood == 0 {
                bn6_battle::kinds::player::Emotion::WornOut
            } else {
                bn6_battle::kinds::player::Emotion::Normal
            };
            Context {
                library: content,
                stats,
                emotion,
                turn: unhex(&f.bs)[7],
                per_player_gauges: false,
                random_battle: false,
                now: f.frame,
                link_delay: Link::RECORDED_DELAY,
            }
        };
        for (p, side) in sides.iter_mut().enumerate() {
            if let Some(side) = side {
                side.joypad.update(round.joypad(f.frame, p));
            }
        }
        let custom = f.state[0] == 4 && f.state[1] == 8;
        let prev_init = i.checked_sub(1).map(|j| frames[j].state[3]);
        if custom && f.state[3] == 1 && prev_init == Some(0) {
            for (p, side) in sides.iter_mut().enumerate() {
                if let Some(side) = side {
                    side.open(&context(p), &mut consoles[p]);
                }
            }
            open = Some((f.frame, [None; 2], [None; 2]));
            continue;
        }
        let Some((opened, confirmed, cleared)) = open.as_mut() else { continue };
        if custom {
            for (p, side) in sides.iter_mut().enumerate() {
                let Some(side) = side else { continue };
                let was_open = side.in_custom;
                let request = side.tick(&context(p), &mut consoles[p], |id| {
                    let d = content.chip(id).damage;
                    if d < 1000 { d } else { 0 }
                });
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
        let arrivals: Vec<u32> = sides.iter().flatten().filter_map(|s| s.sent.as_ref().map(|x| x.arrives)).collect();
        for (p, side) in sides.iter().enumerate() {
            let Some(side) = side else { continue };
            let mut d = Vec::new();
            match &side.sent {
                None => d.push("never sent".to_string()),
                Some(sent) => {
                    let block = codec::chip_hand(&unhex(&f.chip_blocks[p]), &ids);
                    let expected =
                        sent.result.hand.clone().unwrap_or_else(|| codec::chip_hand(&unhex(&before.chip_blocks[p]), &ids));
                    let formula = |h: &ChipHand, k: usize| h.ids[k].is_some_and(|id| content.chip(id).damage >= 1000);
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
            // The status bit's clearing reaches both consoles 1 + 4 frames
            // after it happens.
            let screen = frames[..=i].iter().filter(|g| g.frame > *opened);
            let set = |g: &&&Frame| unhex(&g.bs)[0x14 + p] & 4 != 0;
            let theirs_cleared = screen.skip_while(|g| !set(g)).find(|g| !set(g)).map(|g| g.frame);
            let ours_cleared = cleared[p].map(|c| c + 1 + Link::RECORDED_DELAY as u32);
            if ours_cleared != theirs_cleared {
                d.push(format!("status bit arrives cleared: ours {ours_cleared:?} theirs {theirs_cleared:?}"));
            }
            if sides.iter().all(|s| s.is_some()) && arrivals.iter().max() != Some(&f.frame) {
                d.push(format!("results in: ours {:?} theirs {}", arrivals.iter().max(), f.frame));
            }
            checks.push(ScreenCheck { side: p as u8, opened: *opened, resumed: f.frame + 1, confirmed: confirmed[p], differences: d });
        }
        open = None;
    }
    checks
}
