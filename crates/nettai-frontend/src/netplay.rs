//! Live play against another player over the network (docs/frontend.md):
//! `--play --host PORT` or `--play --join ADDR:PORT`.
//!
//! Each player's frontend runs the whole battle on nettai-netplay's
//! [`Peer`]: a getgud rollback session whose inputs go to the other peer
//! over rennet, in UDP datagrams. Before the match, the handshake
//! (`nettai_netplay::transport`) checks that both run the same engine and
//! the same content, and swaps what each player brings (an [`Offer`]: their
//! folder, game, Crosses and patch cards; the language is each player's
//! own) and their halves of the seed. Both then build the same round
//! ([`netplay_setup`]): the host's arena (a match file's, else drawn from
//! the seed on the host's stage, if it names one), each player's side on
//! their side (the host's is side 0, the left navi).
//!
//! Every frame the driver takes what arrived, decides the player's buttons
//! for the next tick (unless clock sync or the stall guard holds it), sends
//! its datagram and advances; the frame to show is getgud's presented
//! frame, drawn from the player's side, and the sound is the cue actions a
//! tracker makes of every tick simulated (a cue played on a wrong
//! prediction is taken back). A set's rounds follow each other on the same
//! stream ([`nettai_match::next_round`]).
//!
//! The sound's tracker is the peer's world's observer, which the world tells
//! everything (ticks simulated, rewinds, ticks settled); the player reads
//! its actions through the session. A player is `Send` over a `Send`
//! datagram channel (UDP), so the network side can run on a thread of its
//! own.

use std::sync::Arc;
use std::time::{Duration, Instant};

use nettai_battle::content::Content;
use nettai_battle::cues::{CueAction, CueTracker};
use nettai_battle::custom::{CrossList, SavedFolder};
use nettai_battle::setup::RoundSetup;
use nettai_battle::{Battle, BattleResult, RoundEnd};
use nettai_battle::patch_cards::InstalledCard;
use nettai_content_api::StageHandle;
use nettai_match::{Arena, Draws, Match, Place, Side};
use nettai_netplay::protocol::BUTTONS;
use nettai_netplay::standin::StandInBattle;
use nettai_netplay::transport::{Connection, Datagram, Hello, Role};
use nettai_netplay::wire::{Reader, Writer};
use nettai_netplay::{BattleWorld, Game, Observer, Peer, PeerConfig};

use crate::driver::{Driver, Ran, Step};

/// What a player brings to a netbattle: their side of the match (a match
/// file's left side, or one drawn from their seed), and from the host the
/// arena (a match file's) or a stage the round must be fought on
/// (`--stage`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Offer {
    pub side: Side,
    pub stage: Option<StageHandle>,
    pub arena: Option<Arena>,
}

impl Offer {
    /// The offer as the handshake carries it.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        let mut w = Writer(&mut out);
        let Offer { side, stage, arena } = self;
        let Side { ruleset, navi, game, stats, emotion_window_glitch, folder, crosses, cards, navi_level, bug_frags, navicust } = side;
        w.put(ruleset);
        w.put(navi);
        w.put(game);
        w.put(stats);
        w.put(emotion_window_glitch);
        // An offer's side is one the match's checks accepted, whose folder
        // is whole.
        w.put(&folder.saved().expect("an offer's folder is whole (the checks refuse one being made)"));
        w.put(crosses);
        w.put(cards);
        w.put(navi_level);
        w.put(bug_frags);
        w.put(&navicust.is_some());
        if let Some(n) = navicust {
            w.put(&n.expansions);
            w.put(&(n.len() as u16));
            for p in n.iter() {
                w.put(&p.program.0);
                w.put(&[p.color, p.x, p.y, p.rotation]);
                w.put(&p.compressed);
            }
        }
        w.put(stage);
        w.put(&arena.is_some());
        if let Some(a) = arena {
            for p in std::iter::once(&a.first).chain(&a.later) {
                w.put(&p.stage);
                w.put(&p.background);
            }
        }
        out
    }

    /// An offer from the other side, checked against `content` as a match
    /// file's side is (`nettai_match::check_side`), its arena or stage a
    /// link battle's.
    pub fn from_bytes(content: &Arc<Content>, bytes: &[u8]) -> Result<Offer, String> {
        let mut r = Reader::new(bytes);
        let decoded = (|| -> std::io::Result<Offer> {
            let side = Side {
                ruleset: r.get()?,
                navi: r.get()?,
                game: r.get()?,
                stats: r.get()?,
                emotion_window_glitch: r.get()?,
                folder: r.get::<SavedFolder>()?.into(),
                crosses: r.get::<Option<CrossList>>()?,
                cards: r.get::<Vec<InstalledCard>>()?,
                navi_level: r.get()?,
                bug_frags: r.get()?,
                navicust: if r.get::<bool>()? {
                    let expansions: u8 = r.get()?;
                    let n: u16 = r.get()?;
                    let mut parts = Vec::with_capacity(n.min(64) as usize);
                    for _ in 0..n {
                        let program = nettai_content_api::NaviCustProgramHandle(r.get()?);
                        let [color, x, y, rotation]: [u8; 4] = r.get()?;
                        parts.push(nettai_battle::navicust::PlacedProgram { program, color, x, y, rotation, compressed: r.get()? });
                    }
                    let bad = |e: String| std::io::Error::new(std::io::ErrorKind::InvalidData, e);
                    Some(nettai_battle::navicust::NaviCust::new(&parts, expansions).map_err(bad)?)
                } else {
                    None
                },
            };
            let stage: Option<StageHandle> = r.get()?;
            let arena = if r.get::<bool>()? {
                let mut place = || -> std::io::Result<Place> { Ok(Place { stage: r.get()?, background: r.get()? }) };
                let first = place()?;
                let later = [place()?, place()?];
                Some(Arena { first, later })
            } else {
                None
            };
            Ok(Offer { side, stage, arena })
        })();
        let offer = decoded.map_err(|e| format!("the other player's setup doesn't decode ({e})"))?;
        r.finish().map_err(|e| format!("the other player's setup doesn't decode ({e})"))?;
        offer.check(content)?;
        Ok(offer)
    }

    /// The offer is one this content can play.
    pub fn check(&self, content: &Arc<Content>) -> Result<(), String> {
        let broken = nettai_match::check_side(content, &self.side);
        if !broken.is_empty() {
            return Err(format!("the other player's setup breaks the rules: {}", broken.join("; ")));
        }
        if let Some(a) = &self.arena {
            let broken = nettai_match::check::check_arena(content, a);
            if !broken.is_empty() {
                return Err(format!("the other player's arena breaks the rules: {}", broken.join("; ")));
            }
        }
        if self.stage.is_some_and(|s| !nettai_match::link_battle_stages(content).contains(&s)) {
            return Err("the other player's stage isn't a link battle stage".into());
        }
        Ok(())
    }
}

/// The round both players of a match play, and the match: the host's arena
/// (else one drawn from the match's seed, on the host's stage if it names
/// one), each player's side on their side (`offers` by side: the host's,
/// then the joiner's).
pub fn netplay_setup(content: &Content, seed: u32, offers: &[Offer; 2]) -> Result<(RoundSetup, Match), String> {
    let arena = match &offers[0].arena {
        Some(a) => a.clone(),
        None => nettai_match::draw::arena(content, &mut Draws::new(seed), offers[0].stage)?,
    };
    let m = Match { seed: Some(seed), arena, sides: [offers[0].side.clone(), offers[1].side.clone()] };
    Ok((m.round(content, seed), m))
}

/// How a netplay match plays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NetOptions {
    /// Input delay: ticks the presented frame is behind the player's newest
    /// input (getgud's present delay). More delay, fewer rollbacks.
    pub delay: u32,
    /// The stall guard: inputs ahead of the other player's before a frame
    /// waits for them.
    pub max_lead: u32,
    /// Nothing from the other side this long ends the match.
    pub timeout: Duration,
}

impl Default for NetOptions {
    fn default() -> NetOptions {
        NetOptions { delay: 2, max_lead: 30, timeout: Duration::from_secs(10) }
    }
}

/// What the player hears: every tick the peer simulates, through a cue
/// tracker for their side. The peer's world owns it.
struct Sound {
    viewer: u8,
    tracker: CueTracker,
    /// Every action of the round so far, in order: the player takes the
    /// ones after those it took the frame before.
    actions: Vec<CueAction>,
}

/// Frames a cue a re-simulation makes again may move and still be the one
/// played (docs/design/rollback.md §3.2).
const CUE_TOLERANCE: u32 = 3;

impl Sound {
    fn new(viewer: u8) -> Sound {
        Sound { viewer, tracker: CueTracker::new(CUE_TOLERANCE), actions: Vec::new() }
    }
}

impl<G: Game> Observer<G> for Sound {
    fn rolled_back(&mut self, frame: u32) {
        self.tracker.rolled_back(frame);
    }

    fn simulated(&mut self, frame: u32, game: &G) {
        self.tracker.simulated(frame, game.battle().sound_cues_for(self.viewer));
        self.actions.extend(self.tracker.drain());
    }

    fn confirmed(&mut self, frame: u32, _: Option<&Battle>) {
        self.tracker.confirmed(frame + 1);
    }
}

type NetPeer = Peer<StandInBattle, Sound>;

// A peer, its world and its sound go to another thread, and a player over
// UDP with them.
const _: () = {
    const fn send<T: Send>() {}
    send::<NetPeer>();
    send::<NetPlayer<nettai_netplay::transport::Udp>>();
};

/// Plays a match against another player over `D` (UDP, `nettai_netplay::transport::Udp`).
pub struct NetPlayer<D: Datagram> {
    content: Arc<Content>,
    conn: Connection<D>,
    side: usize,
    peer: NetPeer,
    /// The sound's actions of this round the player has taken.
    heard: usize,
    /// The set's first round, and the players' folders by side.
    first: RoundSetup,
    folders: [SavedFolder; 2],
    options: NetOptions,
    start: Instant,
    /// The set's result for this player, once it is over.
    over: Option<BattleResult>,
}

impl<D: Datagram> NetPlayer<D> {
    /// The match on `conn` (after the handshake), from `first`, its first
    /// round, between these players' folders (by side).
    pub fn new(content: Arc<Content>, conn: Connection<D>, first: RoundSetup, folders: [SavedFolder; 2], options: NetOptions) -> NetPlayer<D> {
        let side = conn.side();
        let config = PeerConfig::new(options.delay, options.max_lead);
        let world = BattleWorld::with_observer(StandInBattle::new(Battle::new(first.clone(), content.clone())), side, Sound::new(side as u8));
        NetPlayer { content, conn, side, peer: Peer::new(world, config), heard: 0, first, folders, options, start: Instant::now(), over: None }
    }

    /// The side this player plays.
    pub fn side(&self) -> usize {
        self.side
    }

    /// What the peer did: its rollbacks and waits, and its link.
    pub fn stats(&self) -> (&nettai_netplay::peer::PeerStats, &nettai_netplay::link::LinkStats) {
        (self.peer.stats(), self.peer.link_stats())
    }

    /// The settled state's tick and digest (both peers' agree).
    pub fn settled(&self) -> (u32, u64) {
        let s = self.peer.session().settled_state();
        (s.tick(), s.battle().digest())
    }

    /// The set's result for this player, once it is over.
    pub fn result(&self) -> Option<BattleResult> {
        self.over
    }

    fn now(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }

    /// The sound's actions since the player last took them.
    fn take_sound(&mut self, into: &mut Vec<CueAction>) {
        let actions = &self.peer.session().world().observer().actions;
        into.extend_from_slice(&actions[self.heard..]);
        self.heard = actions.len();
    }

    /// `battle` as this player sees it: the frontend draws the console of
    /// `setup.local_side`; the simulation's own perspective
    /// (`round.local_side`, side 0's on both peers) stays.
    fn show(&self, battle: &Battle, shown: &mut Battle) {
        shown.clone_from(battle);
        shown.setup.local_side = self.side as u8;
    }

    /// One wall-clock frame.
    fn frame(&mut self, keys: u16, shown: &mut Battle) -> Result<Ran, String> {
        let now = self.now();
        loop {
            match self.conn.try_recv_frame() {
                Ok(Some(datagram)) => self.peer.receive(datagram, now).map_err(|e| format!("netplay stopped: {e}"))?,
                Ok(None) => break,
                Err(e) => return Err(format!("netplay stopped: {e}")),
            }
        }
        if self.peer.remote_left() {
            return Err(match self.over {
                Some(r) => format!("the match is over ({}); the other player left", result_text(r)),
                None => "the other player left the match".into(),
            });
        }
        if self.conn.silence() > self.options.timeout {
            return Err(format!("nothing from the other player for {} seconds: the connection is lost", self.options.timeout.as_secs()));
        }
        let mut ran = Ran::default();
        let decided = self.over.is_none() && self.peer.wait().is_none();
        if decided {
            self.peer.decide(keys & BUTTONS);
        }
        let datagram = self.peer.datagram(now);
        self.conn.send_frame(&datagram).map_err(|e| format!("netplay stopped: {e}"))?;
        if decided {
            let side = self.side as u8;
            self.peer.advance(|advanced| {
                shown.clone_from(advanced.frame.state.battle());
                shown.setup.local_side = side;
            });
            ran.advanced = true;
            // What settled: the round's end. (The world has told the sound.)
            let (end, next) = {
                let settled = self.peer.session().settled_state();
                // (The next round's setup is the shared simulation's, side
                // 0's; the result is this player's.)
                let next = match settled.battle().round_end() {
                    Some(&RoundEnd::NextRound { settings, score }) => {
                        Some(nettai_match::next_round(&self.content, &self.first, &self.folders, settled.battle(), settings, score))
                    }
                    _ => None,
                };
                (settled.battle().round_end_for(side), next)
            };
            if let Some(RoundEnd::Over(result)) = end {
                self.over = Some(result);
            }
            if let Some(next) = next {
                let battle = Battle::new(next, self.content.clone());
                self.take_sound(&mut ran.sound);
                // A new round, a new tracker.
                let world = BattleWorld::with_observer(StandInBattle::new(battle.clone()), self.side, Sound::new(side));
                self.peer.end_round(world);
                self.heard = 0;
                self.show(&battle, shown);
                ran.new_round = true;
            }
        }
        self.take_sound(&mut ran.sound);
        Ok(ran)
    }
}

fn result_text(r: BattleResult) -> &'static str {
    match r {
        BattleResult::Won => "you won",
        BattleResult::Lost => "you lost",
        BattleResult::Drawn => "a draw",
        _ => "cut short",
    }
}

impl<D: Datagram> Driver for NetPlayer<D> {
    fn start(&mut self) -> Battle {
        let mut shown = Battle::new(self.first.clone(), self.content.clone());
        shown.setup.local_side = self.side as u8;
        shown
    }

    /// (A netplay match runs its own frames.)
    fn next(&mut self, _: &Battle, _: u16) -> Option<Step> {
        None
    }

    fn run_frame(&mut self, keys: u16, shown: &mut Battle) -> Option<Result<Ran, String>> {
        Some(self.frame(keys, shown))
    }

    fn position(&self) -> String {
        let s = self.peer.session();
        format!("netplay round {} tick {} (settled {})", self.peer.round() + 1, s.local_frontier(), s.settled_state().tick())
    }

    fn status(&self) -> Option<String> {
        let s = self.peer.stats();
        let l = self.peer.link_stats();
        let ping = l.srtt.map_or("-".to_string(), |r| format!("{r:.0}MS"));
        let mut line = format!(
            "PING {ping} LOSS {:.0}% DELAY {} ROLLBACK {} MAX {} ({}) WAIT {}",
            l.loss() * 100.0,
            self.options.delay,
            s.last_rollback,
            s.max_rollback,
            s.rollbacks,
            s.stalls + s.parked,
        );
        if let Some(r) = self.over {
            line.push_str(&format!("\nTHE MATCH IS OVER: {}", result_text(r).to_uppercase()));
        }
        Some(line)
    }

    fn real_time(&self) -> bool {
        true
    }
}

impl<D: Datagram> Drop for NetPlayer<D> {
    /// Tell the other player this one leaves (a few times: datagrams get
    /// lost; if all are, the other side's timeout ends its match).
    fn drop(&mut self) {
        self.peer.leave();
        let now = self.now();
        for _ in 0..3 {
            let datagram = self.peer.datagram(now);
            let _ = self.conn.send_frame(&datagram);
        }
    }
}

/// This side's Hello for a match on `content`, offering `offer`.
pub fn hello(role: Role, content: &Content, offer: &Offer) -> Hello {
    let entropy = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0) ^ std::process::id() as u64;
    Hello::new(role, content.hash(), offer.to_bytes(), entropy)
}

/// After the handshake: both offers by side (the other's checked), and the
/// round they play.
pub fn agree<D: Datagram>(content: &Arc<Content>, conn: &Connection<D>, mine: &Offer) -> Result<([Offer; 2], RoundSetup, Match), String> {
    let theirs = Offer::from_bytes(content, &conn.theirs().setup)?;
    let offers = if conn.side() == 0 { [mine.clone(), theirs] } else { [theirs, mine.clone()] };
    let (setup, m) = netplay_setup(content, conn.seed(), &offers)?;
    Ok((offers, setup, m))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_match::testing::bn6_content as bn6_test_content;
    use nettai_netplay::standin::Masher;
    use nettai_netplay::transport::Udp;

    fn offer(content: &Arc<Content>, seed: u32) -> Offer {
        Offer { side: Side::drawn(content, &mut Draws::new(seed)).unwrap(), stage: None, arena: None }
    }

    /// An offer goes over the wire as it is; one the content can't play is
    /// refused with a reason.
    #[test]
    fn offers_roundtrip_and_bad_ones_are_refused() {
        let content = bn6_test_content();
        let mut o = offer(&content, 5);
        o.stage = Some(nettai_match::link_battle_stages(&content)[3]);
        o.side.cards = nettai_match::patch_cards(&content, "bn6:canodumb,-bn6:shadow").unwrap_or_default();
        assert_eq!(Offer::from_bytes(&content, &o.to_bytes()).unwrap(), o);
        // A match file's arena goes too.
        let mut a = o.clone();
        a.arena = Some(nettai_match::draw::live(&content, 9, None).unwrap().arena);
        assert_eq!(Offer::from_bytes(&content, &a.to_bytes()).unwrap(), a);
        let mut bad = o.clone();
        bad.side.folder.chips = [bad.side.folder.chips[0]; 30];
        bad.side.folder.regular = None;
        assert!(Offer::from_bytes(&content, &bad.to_bytes()).unwrap_err().contains("breaks the rules"));
        let mut bad = o.clone();
        if let Some(c) = &mut bad.side.folder.chips[0] {
            c.id = nettai_content_api::ChipHandle(60_000);
        }
        assert!(Offer::from_bytes(&content, &bad.to_bytes()).unwrap_err().contains("hasn't"));
        assert!(Offer::from_bytes(&content, &o.to_bytes()[..10]).is_err());
    }

    /// What one player of [`pair`] saw: the round, the match, the settled
    /// ticks' digests, how many sounds played, a report, and why it stopped
    /// (none: it settled them all first).
    type Played = (String, Match, Vec<(u32, u64)>, usize, String, Option<String>);

    /// Two players on loopback UDP, each in a thread with its own
    /// `NetPlayer` on BN6's content, offering what `offers` makes of the
    /// content (the host's, then the joiner's), mashing with rollback until
    /// one has settled `ticks` and leaves; what each saw, by side. The
    /// settled states must agree, and the other must hear it leave.
    fn pair(ticks: u32, offers: fn(&Arc<Content>, usize) -> Offer) -> [Played; 2] {
        let host = Udp::host_on("127.0.0.1:0").unwrap();
        let addr = host.local_addr().unwrap();
        let play = move |udp: Udp, role: Role| -> Played {
            let content = bn6_test_content();
            let mine = offers(&content, role as usize);
            let seed = 11 + 11 * role as u32;
            let timeout = Duration::from_secs(20);
            let conn = match role {
                Role::Host => Connection::host(udp, hello(role, &content, &mine), timeout),
                Role::Join => Connection::join(udp, hello(role, &content, &mine), timeout),
            }
            .unwrap();
            let (offers, setup, m) = agree(&content, &conn, &mine).unwrap();
            let side = conn.side();
            assert_eq!(offers[side], mine);
            let folders = offers.map(|o| o.side.folder.saved().expect("checked offers' folders are whole"));
            let mut player = NetPlayer::new(content.clone(), conn, setup.clone(), folders, NetOptions::default());
            let mut shown = player.start();
            assert_eq!(shown.setup.local_side as usize, side);
            let mut masher = Masher::new(seed as u64);
            let start = Instant::now();
            let (mut settled, mut plays) = (Vec::new(), 0);
            let mut left = None;
            for frame in 1.. {
                assert!(frame < 10 * ticks, "side {side}: stuck at {:?}", player.settled());
                let ran = match player.run_frame(masher.buttons(), &mut shown).unwrap() {
                    Ok(ran) => ran,
                    Err(why) => {
                        left = Some(why);
                        break;
                    }
                };
                plays += ran.sound.iter().filter(|a| matches!(a, CueAction::Play(_))).count();
                if ran.advanced {
                    settled.push(player.settled());
                }
                if player.settled().0 >= ticks {
                    break;
                }
                let next = start + Duration::from_millis(3) * frame;
                std::thread::sleep(next.saturating_duration_since(Instant::now()));
            }
            let (s, l) = player.stats();
            let report = format!(
                "side {side}: settled {} ticks, rollbacks {} (deepest {}), waits {}, {} plays; {} datagrams of {:.1} bytes; {}",
                player.settled().0,
                s.rollbacks,
                s.max_rollback,
                s.stalls + s.parked,
                plays,
                l.sent,
                l.mean_size(),
                left.as_deref().unwrap_or("left first"),
            );
            (format!("{setup:?}"), m, settled, plays, report, left)
        };
        let host = std::thread::spawn(move || play(host, Role::Host));
        let joined = play(Udp::join(addr).unwrap(), Role::Join);
        let hosted = host.join().unwrap();
        eprintln!("{}\n{}", hosted.4, joined.4);
        assert_eq!(hosted.0, joined.0, "the two sides play different rounds");
        assert_eq!(hosted.1, joined.1);
        let theirs: std::collections::HashMap<u32, u64> = joined.2.iter().copied().collect();
        let mut common = 0;
        for (tick, digest) in &hosted.2 {
            if let Some(d) = theirs.get(tick) {
                assert_eq!(d, digest, "the settled states differ at tick {tick}");
                common += 1;
            }
        }
        assert!(common > ticks / 2, "{common} ticks compared");
        assert!(hosted.3 > 0 && joined.3 > 0, "no sound");
        // The one that didn't settle all of them first heard the other leave.
        for left in [&hosted.5, &joined.5].into_iter().flatten() {
            assert_eq!(left, "the other player left the match");
        }
        [hosted, joined]
    }

    /// Two players on loopback UDP, each bringing a side drawn from their
    /// seed: the handshake, the same round on both (the field from the
    /// shared seed, each player's own side), the settled states agree, and
    /// each player hears the battle.
    #[test]
    fn two_players_over_loopback() {
        pair(900, |content, role| offer(content, 11 + 11 * role as u32));
    }

    /// Netplay from match files (`--match`): each player brings their
    /// file's left side, and the host its arena; both play that match.
    #[test]
    fn two_players_over_loopback_with_match_files() {
        // Each player's file, as the editor or --save-match writes one.
        fn file(content: &Arc<Content>, seed: u32) -> Match {
            let text = nettai_match::write(content, &nettai_match::draw::live(content, seed, None).unwrap());
            nettai_match::parse(content, &text).unwrap()
        }
        let offers = |content: &Arc<Content>, role: usize| {
            let [side, _] = file(content, 40 + role as u32).sides;
            Offer { side, stage: None, arena: (role == 0).then(|| file(content, 40).arena) }
        };
        let [hosted, _] = pair(600, offers);
        let content = bn6_test_content();
        let m = &hosted.1;
        assert_eq!(m.arena, file(&content, 40).arena, "the host's arena");
        assert_eq!(m.sides[0], file(&content, 40).sides[0]);
        assert_eq!(m.sides[1], file(&content, 41).sides[0], "the joiner's left side, on the right");
    }
}
