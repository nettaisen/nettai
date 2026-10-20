//! Play in the screenshot pass (docs/app.md §13): its steps, and the second player they need.
//!
//! The lobby, the loading, a match and what ends them take two players. The pass has the second in the same
//! process: a signaling server of the link library's own (its `testing` server, on loopback), and a [`Peer`]
//! that meets the app's player through it over a real link, with a lobby and a match's player of its own, as
//! the library's tests seat two players (`nettai_frontend::netplay::tests::Seat`). The app's side is the Play
//! screen itself, driven through its controls as a player drives it; the peer does what the steps tell it
//! (ready, load to a point and hold, leave). The pass's builds are made here too, new builds of the game's
//! versions: its pictures don't depend on what the player keeps.
//!
//! None of this is there outside the pass: [`net`] and [`builds`] say nothing, and [`frame`] does nothing.

use super::session::Net;
use crate::input::Action;
use crate::screens::ScreenId;
use crate::shell::{App, Shell};
use crate::tour::{Pair, Step, Tour};
use nettai_battle::Content;
use nettai_battle::battle::{fight, mode};
use nettai_battle::content::PlayerFact;
use nettai_battle::input::keys;
use nettai_battle::rules::Fact;
use nettai_content_api::Value;
use nettai_frontend::connection::Reconnecting;
use nettai_frontend::driver::bot_buttons;
use nettai_frontend::game::{Graphics, GraphicsMode};
use nettai_frontend::lobby::{Lobby, Role, Status};
use nettai_frontend::netplay::{NetOptions, NetPlayer};
use nettai_frontend::player::Player;
use nettai_frontend::preload::Preload;
use nettai_match::{Set, Side};
use nettai_rtc::{Config, Link};
use nettai_ui::{AppWindow, BattleKind, LobbyNote, PlayPart, PlayView};
use slint::ComponentHandle;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;
use nettai_platform::time::Instant;

/// The pass's builds, by name: a new build each, of the game's versions in turn. The last breaks a rule.
const BUILDS: [&str; 5] = ["Ladder 10-18", "Swords", "Rush", "Tournament", "ジャッジ対策 v3"];

/// The second player's name.
const PEER: &str = "weenie";

/// What the two players' lobbies draw their nonces from in the pass, so that a match's seed (its arenas, each
/// folder's order) is the same from run to run.
const ENTROPY: [u64; 2] = [1, 2];

/// How long the second player's network is gone for, where the pass shows a link being made again: longer
/// than the link takes to call silence a drop (3 seconds).
const OUTAGE: Duration = Duration::from_secs(6);

/// What the pass has of its own: the server the two players meet through, and the second player.
struct Rig {
    server: nettai_rtc::testing::Server,
    peer: Option<Peer>,
    /// The app's player presses by itself in a match (through the custom screen, then the buster), so a set
    /// comes to its end.
    shoots: bool,
    /// The buttons the rig holds for the app's player.
    held: u16,
    /// The player has no build of the game.
    no_builds: bool,
    /// The other player is another app on this machine ([`apart`]): no server, a port both know.
    apart: bool,
    watch: Option<Watch>,
}

impl Rig {
    fn new() -> Rig {
        Rig { server: nettai_rtc::testing::Server::start(), peer: None, shoots: false, held: 0, no_builds: false, apart: false, watch: None }
    }
}

thread_local! {
    static RIG: RefCell<Option<Rig>> = const { RefCell::new(None) };
}

fn with_rig<R>(f: impl FnOnce(&mut Rig) -> R) -> Option<R> {
    RIG.with(|rig| rig.borrow_mut().as_mut().map(f))
}

fn with_peer(f: impl FnOnce(&mut Peer)) {
    with_rig(|rig| {
        if let Some(peer) = &mut rig.peer {
            f(peer);
        }
    });
}

/// How two players of one process connect: the loopback interface offered (the rig's server gives no STUN or
/// TURN server, and a link has none of its own).
fn config() -> Config {
    Config { loopback: true, ..Config::default() }
}

/// Where Play makes its links in the pass: the rig's server. None outside the pass.
pub fn net() -> Option<Net> {
    with_rig(|rig| match rig.apart {
        true => Net { server: None, config: config(), port: APART_PORT },
        false => Net { server: Some(rig.server.url()), config: config(), port: 0 },
    })
}

/// The loopback UDP port the host listens on where two apps of one machine meet ([`apart`]).
const APART_PORT: u16 = 47_476;

/// What a player of the two-app check ([`apart`]) counts of its link, written to the log a second at a time:
/// the frames the app ran (and how many of them the window drew), the datagrams the other player's link
/// brought and the longest it brought none for, the frames in which the link was down, and the match's ticks.
struct Watch {
    who: &'static str,
    since: Instant,
    frames: u32,
    drawn: u32,
    heard: usize,
    last_heard: Option<Instant>,
    silence: Duration,
    down: u32,
    ticks: Option<u64>,
}

impl Watch {
    fn new(who: &'static str) -> Watch {
        Watch { who, since: Instant::now(), frames: 0, drawn: 0, heard: 0, last_heard: None, silence: Duration::ZERO, down: 0, ticks: None }
    }

    /// A frame of the app.
    fn frame(&mut self, shell: &Shell, now: Instant) {
        self.frames += 1;
        self.drawn += shell.drawing as u32;
        if now.saturating_duration_since(self.since) < Duration::from_secs(1) {
            return;
        }
        let ticks = shell.battle.as_ref().filter(|b| b.kind == BattleKind::Online).map(|b| b.stage.player.ticks() as u64);
        let ran = match (self.ticks, ticks) {
            (Some(was), Some(is)) => format!("{} ticks", is.saturating_sub(was)),
            _ => "no match".to_string(),
        };
        let silence = self.silence.max(self.last_heard.map_or(Duration::ZERO, |at| now.saturating_duration_since(at)));
        let Watch { frames, drawn, heard, down, .. } = *self;
        let said = format!("{frames} frames ({drawn} drawn), {heard} datagrams heard, {} ms the longest silence, the link down in {down} frames, {ran}", silence.as_millis());
        crate::errors::log(self.who, &said);
        *self = Watch { last_heard: self.last_heard, ticks, since: now, ..Watch::new(self.who) };
    }

    /// What the link brought this frame, and whether it is down.
    fn heard(&mut self, now: Instant, datagrams: usize, down: bool) {
        if datagrams > 0 {
            if let Some(at) = self.last_heard {
                self.silence = self.silence.max(now.saturating_duration_since(at));
            }
            self.last_heard = Some(now);
            self.heard += datagrams;
        }
        self.down += down as u32;
    }
}

/// What the app's link brought this frame, for the two-app check's count.
pub fn heard(now: Instant, datagrams: usize, down: bool) {
    with_rig(|rig| {
        if let Some(watch) = &mut rig.watch {
            watch.heard(now, datagrams, down);
        }
    });
}

/// What the app's player's lobby draws its nonces from in the pass. None outside it, and where the other
/// player is another app ([`apart`]).
pub fn entropy() -> Option<u64> {
    with_rig(|rig| (!rig.apart).then_some(ENTROPY[0])).flatten()
}

/// The builds Play lists in the pass: new builds of `game`'s versions, by the pass's names. None outside
/// the pass (`still`: the pass runs).
pub fn builds(still: bool, content: &Arc<Content>, game: &str) -> Option<Vec<(String, Side)>> {
    if !still {
        return None;
    }
    if with_rig(|rig| rig.no_builds) == Some(true) {
        return Some(Vec::new());
    }
    let made = (|| -> Result<Vec<(String, Side)>, String> {
        let builds = nettai_match::build::Builds::of(content, game)?;
        let mut out = Vec::new();
        for (i, name) in BUILDS.iter().enumerate() {
            let mut side = builds.new_build(content, &builds.groups[(i + 1) % builds.groups.len()].id)?;
            if i + 1 == BUILDS.len() {
                // (Two of its folder's entries are empty: no folder a save holds.)
                let mut folder = side.folder(content);
                (folder.chips[0], folder.chips[1]) = (None, None);
                side.set_folder(content, &folder)?;
            }
            out.push((name.to_string(), side));
        }
        Ok(out)
    })();
    Some(made.map_err(|why| crate::errors::log("making the screenshot pass's builds", &why)).unwrap_or_default())
}

/// The builds shown over the match (Tab held) made long: each a new build of the game's first version whose
/// NaviCust holds many of the game's programs, a dozen on the command line, more off it and plus parts on it, in
/// every color (nothing checks where they are: the rules say what is wrong, and the build's abilities and bugs are
/// what they come to), under the two players' names. A match shows them again as it starts.
fn crowd(app: &mut App) {
    use crate::builds::grid::{self, Placed};
    let shell = &app.shell;
    let Some(kit) = crate::builds::kit(shell) else { return };
    let content = kit.content();
    let Some(data) = grid::read(content, crate::builds::module_data(content, &kit.game, grid::MODULE).as_ref()) else { return };
    let Some(group) = kit.builds.groups.first() else { return };
    let Ok(mut side) = kit.builds.new_build(content, &group.id) else { return };
    let (plus, rest): (Vec<u16>, Vec<u16>) = content.defs.entries_of(&data.collection).iter().map(|h| h.0).partition(|p| data.plus.contains(p));
    let at = |(i, piece): (usize, u16), y: i32| Placed {
        piece,
        x: 1 + (i % 3) as i32,
        y,
        rotation: 0,
        color: data.colors.get(&piece).and_then(|c| c.get(i % c.len().max(1)).cloned()),
        compressed: false,
        record: nettai_match::facts::Stated::Record(Vec::new()),
    };
    let placed: Vec<Placed> = (rest.iter().copied().enumerate().take(12).map(|p| at(p, 3)))
        .chain(rest.iter().copied().enumerate().skip(12).take(6).map(|p| at(p, 1)))
        .chain(plus.iter().copied().enumerate().take(6).map(|p| at(p, 3)))
        .collect();
    if !grid::set_placed(content, &mut side, &placed) {
        return;
    }
    let ui = shell.ui();
    let view = ui.global::<PlayView>();
    view.set_mine(crate::builds::overview::sheet(shell, &kit, &view.get_you(), BUILDS[0], &side));
    view.set_theirs(crate::builds::overview::sheet(shell, &kit, &view.get_opponent(), PEER, &side));
}

/// The connection's figures as the pass shows them in the lobby: what two players of one process measure
/// changes from shot to shot.
pub fn steady_ping(still: bool) -> Option<(i32, i32)> {
    still.then_some((3, 42))
}

/// A frame of the pass: the second player's, and the app's player's buttons where the rig presses them.
pub fn frame(shell: &mut Shell, now: Instant) {
    let held = with_rig(|rig| {
        if rig.peer.as_mut().is_some_and(|peer| !peer.frame(now)) {
            rig.peer = None;
        }
        if let Some(watch) = &mut rig.watch {
            watch.frame(shell, now);
        }
        let was = rig.held;
        rig.held = match &shell.battle {
            Some(b) if rig.shoots && b.kind == BattleKind::Online => {
                let battle = b.stage.player.battle();
                let tick = b.stage.player.ticks() as u32;
                if battle.round.mode == mode::CUSTOM {
                    bot_buttons(battle, b.viewer as usize, tick)
                } else if tick % 8 < 2 {
                    keys::B
                } else {
                    0
                }
            }
            _ => 0,
        };
        (was, rig.held)
    });
    if let Some((was, held)) = held {
        // (As keys held: the battle reads them with the pad's and the on-screen pad's.)
        shell.keys.held = (shell.keys.held & !was) | held;
    }
}

/// What the second player is asked to do.
#[derive(Default)]
struct Wishes {
    ready: bool,
    /// Its loading stops here (0 to 1) until it is let go.
    hold_loading: Option<f32>,
    /// Leave the match under way, for the lobby.
    leave_match: bool,
    /// Leave for good.
    leave: bool,
}

/// The second player: a link, a lobby over it once it is open, and a match's player while one is up.
struct Peer {
    link: Link,
    content: Arc<Content>,
    game: String,
    graphics: Graphics,
    side: Side,
    lobby: Option<Lobby>,
    preload: Option<Preload>,
    player: Option<Player>,
    last: Option<Instant>,
    wishes: Wishes,
}

impl Peer {
    /// One frame, as a host runs its link and its lobby (docs/design/rollback.md §4.7). False once it is
    /// over: its link gave up, or its lobby ended.
    fn frame(&mut self, now: Instant) -> bool {
        // What the link brought, to the lobby.
        let fed = match &mut self.lobby {
            None => self.link.poll(),
            Some(lobby) => (|| -> Result<(), nettai_rtc::Error> {
                while let Some(datagram) = self.link.recv()? {
                    lobby.receive(now, datagram);
                }
                lobby.set_link_down(self.link.down_for().map(|elapsed| Reconnecting { elapsed, gives_up_in: self.link.gives_up_in() }));
                Ok(())
            })(),
        };
        if let Err(error) = fed {
            crate::errors::log("the screenshot pass's second player's link", &error.detail);
            return false;
        }
        if self.lobby.is_none() && self.link.is_open() {
            let role = match self.link.role() {
                Some(nettai_rtc::Role::Host) => Role::Host,
                _ => Role::Join,
            };
            let Ok(rulesets) = nettai_match::ruleset::Rulesets::of(&self.content, &self.game) else { return false };
            let ruleset = rulesets.first().clone();
            let mut lobby = Lobby::new(role, &self.content, std::sync::Arc::new(rulesets), super::settings(&self.game, 1, ruleset), self.side.clone(), ENTROPY[1], Duration::from_secs(45));
            lobby.set_name(PEER);
            self.lobby = Some(lobby);
        }
        let Some(lobby) = &mut self.lobby else { return true };
        // The match's player, at the battle's rate: its custom screen pressed through, nothing in the fight.
        if let Some(player) = &mut self.player {
            let elapsed = self.last.map_or(Duration::ZERO, |last| now.saturating_duration_since(last));
            let side = player.battle().setup.local_side as usize;
            let buttons = bot_buttons(player.battle(), side, player.ticks() as u32);
            player.advance(elapsed, buttons);
            let mut samples = Vec::new();
            player.take_samples(&mut samples);
        }
        self.last = Some(now);
        if self.wishes.leave {
            lobby.leave();
        }
        enum Polled {
            Lobby,
            Loading(Option<(Set, usize)>),
            Start(Set, usize),
            Playing,
            Failed,
        }
        let preloading = self.preload.is_some();
        let polled = match lobby.poll(now) {
            Status::Pending => Polled::Lobby,
            Status::Loading(agreed) => Polled::Loading((!preloading).then(|| (agreed.set.clone(), agreed.side))),
            Status::Start(agreed) => Polled::Start(agreed.set.clone(), agreed.side),
            Status::Playing => Polled::Playing,
            Status::Failed(_) => Polled::Failed,
        };
        let mut alive = true;
        match polled {
            Polled::Lobby => {
                self.preload = None;
                if lobby.ready() != self.wishes.ready {
                    lobby.set_ready(self.wishes.ready);
                }
            }
            Polled::Loading(agreed) => {
                if let Some((set, side)) = agreed {
                    self.preload = Some(Preload::with(self.graphics.renderer(GraphicsMode::Original, None), None, None, &set, side as u8, (240, 160)));
                }
                if let Some(preload) = &mut self.preload {
                    match self.wishes.hold_loading {
                        Some(held) => {
                            while preload.fraction() < held && preload.step() {}
                            lobby.set_loading(held);
                        }
                        None => {
                            while preload.step() {}
                            lobby.set_loaded();
                        }
                    }
                }
            }
            Polled::Start(set, side) => {
                if let Some(channel) = lobby.channel() {
                    let driver = Box::new(NetPlayer::new(channel, side, set, NetOptions::default()));
                    self.player = Some(match self.preload.take() {
                        Some(preload) => preload.player(driver),
                        None => Player::with(self.graphics.renderer(GraphicsMode::Original, None), None, None, driver),
                    });
                    // (Its ready mark was for this match.)
                    self.wishes.ready = false;
                }
            }
            Polled::Playing => {
                // Back to the lobby when it leaves the match, when its match stopped (the app's player left
                // it), or when its set is over and the app's player is back in the lobby.
                let over = self.player.as_ref().is_some_and(|p| p.stop_reason().is_some() || (p.result().is_some() && lobby.their_reopened()));
                if std::mem::take(&mut self.wishes.leave_match) || over || (self.player.is_some() && lobby.their_reopened()) {
                    self.player = None;
                    lobby.reopen();
                }
            }
            Polled::Failed => alive = false,
        }
        for datagram in lobby.outgoing() {
            if self.link.send(&datagram).is_err() {
                return false;
            }
        }
        alive
    }
}

/// How the second player comes.
#[derive(Clone, Copy)]
enum Comes {
    /// By what the app's player gives it: the invite's code, or a direct host's address.
    Joining(Brings),
    /// With an invite of its own, for the app's player to join by its code.
    Inviting,
}

/// The build the second player brings: a new build of the game's first version.
#[derive(Clone, Copy, PartialEq)]
enum Brings {
    /// As it is.
    New,
    /// With 1 HP: the app's player's first shot decides a round.
    Weak,
    /// With two of its folder's entries empty: no build the rules let into a match (its lobby won't make it
    /// ready).
    Broken,
}

/// The code of the second player's own invite, once the server has given it.
fn peer_code() -> Option<String> {
    with_rig(|rig| rig.peer.as_ref().and_then(|p| p.link.code().map(str::to_string))).flatten()
}

/// The second player comes.
fn come(app: &App, comes: Comes) {
    let Some((game, ready)) = app.shell.ready() else { return };
    let content = ready.content();
    // What the app's player would give the other: the invite's code, or its own address.
    let invite = app.shell.ui().global::<PlayView>().get_invite().to_string();
    let made = (|| -> Result<Peer, String> {
        let builds = nettai_match::build::Builds::of(content, &game)?;
        let mut side = builds.new_build(content, &builds.groups[0].id)?;
        match comes {
            Comes::Joining(Brings::Weak) => side.set_fact(content, PlayerFact::BaseHp.name(), &[Fact::Value(Value::Int(1))])?,
            Comes::Joining(Brings::Broken) => {
                let mut folder = side.folder(content);
                (folder.chips[0], folder.chips[1]) = (None, None);
                side.set_folder(content, &folder)?;
            }
            _ => {}
        }
        let server = with_rig(|rig| rig.server.url()).ok_or("the pass has no server")?;
        let link = if matches!(comes, Comes::Inviting) {
            Link::create_room(&server, config())
        } else if app.shell.ui().global::<PlayView>().get_invite_direct() {
            // (On this machine: the port the app's player listens on.)
            let port = invite.rsplit_once(':').map_or("", |(_, port)| port);
            Link::join(&format!("127.0.0.1:{port}"), config()).map_err(|e| e.detail)?
        } else {
            Link::join_room(&server, &invite, config()).map_err(|e| e.detail)?
        };
        Ok(Peer {
            link,
            content: content.clone(),
            game: game.clone(),
            graphics: ready.graphics(app.shell.lang),
            side,
            lobby: None,
            preload: None,
            player: None,
            last: None,
            wishes: Wishes::default(),
        })
    })();
    match made {
        Ok(peer) => {
            with_rig(|rig| rig.peer = Some(peer));
        }
        Err(why) => crate::errors::log("the screenshot pass's second player", &why),
    }
}

/// The time a step is given before it.
const SETTLE: u64 = 260;

fn step(f: impl Fn(&mut App) + 'static) -> Step {
    Box::new(move |_, app, _| f(&mut app.borrow_mut()))
}

fn act(action: Action) -> Step {
    step(move |app| app.act(action))
}

fn focus(id: &'static str, index: i32) -> Step {
    step(move |app| app.shell.nav.focus(id, index))
}

/// The pointer moved onto the middle of the control `id` (none: the window's top left corner, on nothing).
fn pointer_on(id: &'static str) -> Step {
    Box::new(move |ui, app, _| {
        let registry = app.borrow().shell.nav.registry();
        let at = registry.borrow().rect(&crate::nav::Key::new(id, 0));
        let position = at.filter(|_| !id.is_empty()).map_or(slint::LogicalPosition::new(1.0, 1.0), |r| slint::LogicalPosition::new(r.x + r.w / 2.0, r.y + r.h / 2.0));
        ui.window().dispatch_event(slint::platform::WindowEvent::PointerMoved { position });
    })
}

/// The field being edited takes `text`, as typing gives it.
fn typed(text: &'static str) -> Step {
    step(move |app| app.input(crate::shell::NavInput::Edited(text.into())))
}

/// A control of Play pressed, as a click presses it.
fn press(id: &'static str) -> Step {
    press_at(id, 0)
}

/// One of several controls of a name, pressed.
fn press_at(id: &'static str, index: i32) -> Step {
    step(move |app| {
        app.shell.nav.focus(id, index);
        app.input(crate::shell::NavInput::Activate(id.into(), index));
    })
}

/// The wait's practice session is being played.
fn practicing(app: &App) -> bool {
    app.shell.battle.as_ref().is_some_and(|b| b.kind == BattleKind::Practice && b.playing())
}

/// The step after waits for `come` (or a minute).
fn until(come: impl Fn(&App, &AppWindow) -> bool + 'static) -> Step {
    let come = Rc::new(come);
    Box::new(move |_, _, tour: &Rc<Tour>| {
        let come = come.clone();
        tour.wait_until(move |app| come(app, &app.shell.ui()));
    })
}

fn part(ui: &AppWindow) -> PlayPart {
    ui.global::<PlayView>().get_part()
}

fn peer(f: impl Fn(&mut Peer) + 'static) -> Step {
    Box::new(move |_, _, _| with_peer(&f))
}

/// The walk goes on once `come` has: a wait, and a step after it that has the pass say so if its minute passed
/// without it (what comes next would be pictures of something else).
fn reach(add: &mut dyn FnMut(u64, Step), come: impl Fn(&App, &AppWindow) -> bool + 'static) {
    let come = Rc::new(come);
    let wait = come.clone();
    add(
        SETTLE,
        Box::new(move |_, _, tour: &Rc<Tour>| {
            let come = wait.clone();
            tour.wait_until(move |app| come(app, &app.shell.ui()));
        }),
    );
    add(
        0,
        Box::new(move |ui, app, tour: &Rc<Tour>| {
            if !app.try_borrow().is_ok_and(|app| come(&app, ui)) {
                tour.never_came();
            }
        }),
    );
}

/// The match's result is on the picture: the fight is won or lost, and its banner has had a quarter of a second
/// of the wait the game holds it for.
fn result_shown(app: &App) -> bool {
    app.shell.battle.as_ref().is_some_and(|b| {
        let fight = &b.stage.player.battle().fight;
        matches!(fight.state, fight::WIN | fight::LOSE) && fight.init != 0 && fight.timer <= 85
    })
}

/// The lobby, the host waiting for the other player.
fn waiting(ui: &AppWindow) -> bool {
    part(ui) == PlayPart::Lobby && ui.global::<PlayView>().get_waiting()
}

/// The app's player and the other are together in the lobby.
fn together(ui: &AppWindow) -> bool {
    part(ui) == PlayPart::Lobby && ui.global::<PlayView>().get_their_present()
}

/// An online match is up.
fn in_match(app: &App) -> bool {
    app.shell.battle.as_ref().is_some_and(|b| b.kind == BattleKind::Online)
}

/// Play's steps of the pass, in a language at a size (`phone`: an upright phone's): `add` takes a step and
/// the time it is given before it; `shot` adds a shot's steps (its name; whether it is the first or the
/// second of a pair).
pub fn steps(add: &mut dyn FnMut(u64, Step), shot: &dyn Fn(&mut dyn FnMut(u64, Step), &'static str, Pair), phone: bool) {
    // The rig: the pass's own server. With no build of the game, one button.
    add(
        SETTLE,
        step(|app| {
            RIG.with(|rig| *rig.borrow_mut() = Some(Rig { no_builds: true, ..Rig::new() }));
            // (What a pass before this one left remembered.)
            app.shell.settings.remembered.retain(|key, _| !key.starts_with("play."));
            app.shell.settings.input_delay = nettai_frontend::netplay::DEFAULT_INPUT_DELAY;
            app.go_now(ScreenId::Play);
            app.shell.nav.show_cursor(true, false);
        }),
    );
    shot(add, "play-no-build", Pair::None);
    add(
        SETTLE,
        step(|app| {
            with_rig(|rig| rig.no_builds = false);
            app.with_screen(ScreenId::Play, |s, shell| s.show(shell));
        }),
    );

    // The builds, and the row: the focus a step along the row; the pointer on HOST (a desktop's: a touch has no
    // hover), with the focus elsewhere.
    add(SETTLE, focus("invite", 0));
    shot(add, "play", Pair::First);
    add(SETTLE, act(Action::Right));
    shot(add, "play-focus", Pair::Second);
    if !phone {
        add(SETTLE, focus("builds", 0));
        add(SETTLE, pointer_on("invite"));
        shot(add, "play-host-hover", Pair::None);
        add(SETTLE, pointer_on(""));
        add(SETTLE, step(|app| app.shell.nav.show_cursor(true, false)));
    }
    add(SETTLE, focus("code", 0));
    // A code typed, in small letters with a space, as the field keeps it; a code no match has.
    add(SETTLE, act(Action::Confirm));
    add(SETTLE, typed("zz9 zz"));
    shot(add, "play-code", Pair::None);
    add(SETTLE, typed("zz9 zz9"));
    add(SETTLE, act(Action::Confirm));
    add(SETTLE, press("join"));
    reach(add, |_, ui| !ui.global::<PlayView>().get_message_quiet());
    shot(add, "play-no-match", Pair::None);
    // The code's six boxes, as a pad's confirm on the field opens them: the focus a box along.
    add(SETTLE, focus("code", 0));
    add(SETTLE, step(|app| app.with_screen(ScreenId::Play, |s, shell| {
        let confirm = shell.bindings.confirm_button();
        s.button(shell, confirm);
    })));
    shot(add, "play-code-boxes", Pair::First);
    add(SETTLE, act(Action::Right));
    shot(add, "play-code-boxes-focus", Pair::Second);
    add(SETTLE, act(Action::Down));
    shot(add, "play-code-boxes-changed", Pair::None);
    add(SETTLE, act(Action::Back));
    // A build that can't be used says so.
    // (The list's row under the cursor; on a phone, the box's sheet's.)
    fn choose_build(index: usize) -> Step {
        step(move |app| {
            if app.shell.scale.phone {
                app.with_screen(ScreenId::Play, |s, shell| s.choose(shell, "build", 0, index as i32));
            } else {
                app.shell.nav.focus("builds", index as i32);
            }
        })
    }
    add(SETTLE, choose_build(BUILDS.len() - 1));
    add(SETTLE, press("invite"));
    shot(add, "play-build-unusable", Pair::None);
    add(SETTLE, choose_build(0));

    // A direct connection: the focus a step down; an address that isn't one.
    add(SETTLE, press("direct"));
    // (The dialog opens editing the address; Back ends the edit and leaves the dialog open.)
    shot(add, "play-direct", Pair::None);
    add(SETTLE, act(Action::Back));
    shot(add, "play-direct-field", Pair::First);
    add(SETTLE, act(Action::Right));
    shot(add, "play-direct-focus", Pair::Second);
    add(SETTLE, focus("direct.address", 0));
    add(SETTLE, act(Action::Confirm));
    add(SETTLE, typed("192.168.1.20"));
    add(SETTLE, act(Action::Confirm));
    add(SETTLE, press("direct.connect"));
    shot(add, "play-direct-bad-address", Pair::None);
    add(SETTLE, press("direct.cancel"));

    // An invite: the lobby with its code in the other player's place, and a practice session in this player's (on
    // a phone, asked for under it, and filling the window); the practice menu over that, the usual one with
    // CANCEL INVITE in EXIT's place (on a phone with Stop practice at its top, the way back to the invite).
    add(SETTLE, press("invite"));
    reach(add, |_, ui| waiting(ui));
    if phone {
        shot(add, "play-waiting", Pair::None);
        add(SETTLE, press("wait.train"));
    }
    reach(add, |app, _| practicing(app));
    add(2500, step(|_| {}));
    shot(add, if phone { "play-waiting-practice" } else { "play-waiting" }, Pair::None);
    add(SETTLE, step(|app| app.shell.open_menu()));
    shot(add, "play-waiting-menu", Pair::None);
    if phone {
        add(SETTLE, press_at("menu.rows.added", 0));
        reach(add, |app, ui| app.shell.battle.is_none() && waiting(ui));
    } else {
        add(SETTLE, step(|app| app.shell.close_menu()));
        reach(add, |app, _| practicing(app));
    }
    // The host sets the match's settings before anyone has come: Open setups on (the match the walk plays has
    // them: both builds show in it). (The pointer's way: on a desktop the keys are the practice battle's.)
    add(SETTLE, step(|app| app.with_screen(ScreenId::Play, |s, shell| s.step(shell, "lobby.open", 0, 1))));
    reach(add, |_, ui| waiting(ui) && ui.global::<PlayView>().get_open());
    shot(add, "play-waiting-open-setups", Pair::None);

    // The second player joins by the code: the lobby, on the settings the host set while it waited. The focus a
    // step along; a setting changed under it (the match's, then the player's own).
    add(SETTLE, step(|app| come(app, Comes::Joining(Brings::Weak))));
    reach(add, |_, ui| part(ui) == PlayPart::Lobby && ui.global::<PlayView>().get_their_present());
    reach(add, |_, ui| ui.global::<PlayView>().get_open() && with_rig(|rig| rig.peer.as_ref().and_then(|p| p.lobby.as_ref()).is_some_and(|l| l.settings().open_setups)) == Some(true));
    add(SETTLE, focus("lobby.ready", 0));
    shot(add, "play-lobby", Pair::First);
    add(SETTLE, act(Action::Right));
    shot(add, "play-lobby-focus", Pair::Second);
    add(SETTLE, focus("lobby.first-to", 0));
    // (A setting is taken to change: Enter, then the arrows. Taken before the pair's first, so the footer's hints are
    // the same in both.)
    add(SETTLE, act(Action::Confirm));
    shot(add, "play-lobby-first-to", Pair::First);
    add(SETTLE, act(Action::Right));
    reach(add, |_, ui| ui.global::<PlayView>().get_first_to() == 3);
    shot(add, "play-lobby-first-to-3", Pair::Second);
    // (The match the walk plays is a single battle: first to 1.)
    add(SETTLE, act(Action::Left));
    add(SETTLE, act(Action::Left));
    reach(add, |_, ui| ui.global::<PlayView>().get_first_to() == 1);
    add(SETTLE, focus("lobby.delay", 0));
    add(SETTLE, act(Action::Confirm));
    shot(add, "play-lobby-delay", Pair::First);
    add(SETTLE, act(Action::Right));
    shot(add, "play-lobby-delay-5", Pair::Second);
    add(SETTLE, act(Action::Left));
    // Another build.
    add(SETTLE, press("lobby.change"));
    shot(add, "play-lobby-change", Pair::None);
    add(SETTLE, act(Action::Back));
    // Esc in the lobby asks before the other player is left, Cancel focused; Cancel stays.
    add(SETTLE, act(Action::Back));
    reach(add, |app, _| app.shell.dialog.is_some());
    shot(add, "play-lobby-leave-question", Pair::None);
    add(SETTLE, act(Action::Confirm));
    reach(add, |app, ui| app.shell.dialog.is_none() && together(ui));
    // The link drops (the other player's network is gone for a while) and comes back: the lobby's line.
    add(SETTLE, peer(|p| p.link.outage(OUTAGE)));
    reach(add, |_, ui| ui.global::<PlayView>().get_note() == LobbyNote::Reconnecting);
    shot(add, "play-lobby-reconnecting", Pair::None);
    reach(add, |_, ui| ui.global::<PlayView>().get_note() != LobbyNote::Reconnecting && ui.global::<PlayView>().get_ping() >= 0);
    // The other player ready; then this one, the other's loading held: the two cards, a bar each.
    add(SETTLE, peer(|p| p.wishes.ready = true));
    reach(add, |_, ui| ui.global::<PlayView>().get_their_ready());
    add(SETTLE, focus("lobby.ready", 0));
    shot(add, "play-lobby-their-ready", Pair::None);
    add(
        SETTLE,
        step(|app| {
            app.shell.hold_cards = true;
            with_peer(|p| p.wishes.hold_loading = Some(0.74));
        }),
    );
    add(SETTLE, press("lobby.ready"));
    reach(add, |app, _| app.shell.loading_battle());
    add(1200, step(|_| {}));
    shot(add, "play-loading", Pair::None);

    // The match: the connection's corner; both builds while Tab is held; the menu and its pages.
    add(
        SETTLE,
        step(|app| {
            app.shell.hold_cards = false;
            with_peer(|p| p.wishes.hold_loading = None);
        }),
    );
    reach(add, |app, _| app.shell.battle.as_ref().is_some_and(|b| b.kind == BattleKind::Online));
    add(5000, step(|_| {}));
    shot(add, "play-match", Pair::None);
    // The link drops in the match and comes back: the corner counts down what it has left.
    add(SETTLE, peer(|p| p.link.outage(OUTAGE)));
    reach(add, |_, ui| ui.global::<nettai_ui::Shell>().get_connection_down() >= 0);
    shot(add, "play-match-reconnecting", Pair::None);
    reach(add, |_, ui| ui.global::<nettai_ui::Shell>().get_connection_down() < 0);
    add(SETTLE, step(|app| app.shell.keys.tab = true));
    shot(add, "play-match-builds", Pair::None);
    // Builds whose abilities and bugs run long: each sheet as tall as they make it.
    add(SETTLE, step(crowd));
    shot(add, "play-match-builds-crowded", Pair::None);
    add(
        SETTLE,
        step(|app| {
            app.shell.keys.tab = false;
            app.shell.open_menu();
        }),
    );
    shot(add, "play-match-menu", Pair::None);
    for (item, name) in [("menu.controls", "play-match-controls"), ("menu.settings", "play-match-settings")] {
        add(SETTLE, press(item));
        shot(add, name, Pair::None);
        add(SETTLE, press("menu.back"));
    }
    // Tab over the menu shows the builds over it, and they scroll by the keys and the pad: Down and Page Down,
    // then to their end and back.
    fn peek_y(ui: &AppWindow) -> f32 {
        ui.global::<PlayView>().get_peek_y()
    }
    add(SETTLE, step(|app| app.shell.keys.tab = true));
    reach(add, |_, ui| ui.global::<PlayView>().get_peek());
    shot(add, "play-match-menu-builds", Pair::None);
    add(SETTLE, act(Action::Down));
    add(SETTLE, act(Action::PageDown));
    reach(add, |_, ui| peek_y(ui) < -0.5);
    shot(add, "play-match-menu-builds-scrolled", Pair::None);
    add(SETTLE, act(Action::End));
    reach(add, |_, ui| (peek_y(ui) + ui.global::<PlayView>().get_peek_span()).abs() < 0.5);
    shot(add, "play-match-menu-builds-end", Pair::None);
    add(SETTLE, act(Action::Home));
    reach(add, |_, ui| peek_y(ui) == 0.0);
    add(SETTLE, step(|app| app.shell.keys.tab = false));
    // The set played to its end (the app's player shoots; the other has 1 HP): the result stays on the
    // picture, then both are back in the lobby, neither ready.
    add(
        SETTLE,
        step(|app| {
            app.shell.close_menu();
            with_rig(|rig| rig.shoots = true);
        }),
    );
    // (Up to three minutes, where the arena has something between the two. The shot is taken by the game's own
    // clock, with its banner up: the result's wait is under two seconds.)
    for _ in 0..3 {
        add(0, until(|app, _| result_shown(app)));
    }
    add(
        0,
        Box::new(|_, app, tour: &Rc<Tour>| {
            if !result_shown(&app.borrow()) {
                tour.never_came();
            }
        }),
    );
    shot(add, "play-match-result", Pair::None);
    reach(add, |app, ui| app.shell.battle.is_none() && part(ui) == PlayPart::Lobby);
    add(SETTLE, step(|_| {
        with_rig(|rig| rig.shoots = false);
    }));
    reach(add, |_, ui| !ui.global::<PlayView>().get_ready() && !ui.global::<PlayView>().get_their_ready());
    shot(add, "play-lobby-again", Pair::None);

    // The host's Ruleset stepped on from the game's first preset (Masters class) to the next (Senior class), the
    // focus on it: the other player's lobby takes it. Then a build the settings can't play, chosen in the lobby: READY
    // is refused, and the line says why; the first build again.
    let senior = || nettai_match::ruleset::Name::Preset("senior".into());
    add(SETTLE, focus("lobby.ruleset", 0));
    shot(add, "play-lobby-ruleset", Pair::First);
    add(SETTLE, act(Action::Right));
    reach(add, move |_, _| with_rig(|rig| rig.peer.as_ref().and_then(|p| p.lobby.as_ref()).is_some_and(|l| l.settings().ruleset.name == senior())) == Some(true));
    shot(add, "play-lobby-ruleset-senior", Pair::Second);
    add(SETTLE, press("lobby.change"));
    add(SETTLE, press_at("change.builds", BUILDS.len() as i32 - 1));
    reach(add, |_, ui| !ui.global::<PlayView>().get_changing());
    add(SETTLE, press("lobby.ready"));
    reach(add, |_, ui| ui.global::<PlayView>().get_note() == LobbyNote::Unusable && !ui.global::<PlayView>().get_ready());
    shot(add, "play-lobby-build-unusable", Pair::None);
    add(SETTLE, press("lobby.change"));
    add(SETTLE, press_at("change.builds", 0));
    reach(add, |_, ui| !ui.global::<PlayView>().get_changing() && ui.global::<PlayView>().get_note() != LobbyNote::Unusable);

    // A second match on the same link, under Mixed and with the setups not open; the other player leaves it:
    // both are back in the lobby, which says why.
    add(SETTLE, focus("lobby.open", 0));
    add(SETTLE, act(Action::Confirm));
    add(SETTLE, act(Action::Left));
    reach(add, |_, ui| !ui.global::<PlayView>().get_open());
    add(SETTLE, peer(|p| p.wishes.ready = true));
    add(SETTLE, press("lobby.ready"));
    reach(add, |app, _| app.shell.battle.as_ref().is_some_and(|b| b.kind == BattleKind::Online));
    add(3000, step(|app| app.shell.keys.tab = true));
    shot(add, "play-match-setups-hidden", Pair::None);
    add(
        SETTLE,
        step(|app| {
            app.shell.keys.tab = false;
            app.shell.open_menu();
        }),
    );
    shot(add, "play-match-setups-hidden-menu", Pair::None);
    add(
        SETTLE,
        step(|app| {
            app.shell.close_menu();
            with_peer(|p| p.wishes.leave_match = true);
        }),
    );
    reach(add, |app, ui| app.shell.battle.is_none() && part(ui) == PlayPart::Lobby);
    shot(add, "play-lobby-opponent-left-match", Pair::None);

    // The other player leaves for good: a failure that ends what the player was doing.
    add(SETTLE, peer(|p| p.wishes.leave = true));
    reach(add, |app, _| app.shell.dialog.is_some());
    shot(add, "play-opponent-left", Pair::None);
    add(SETTLE, act(Action::Confirm));

    // The other ways in, run end to end with no shot of their own: the app's player hosts directly and the
    // other joins its address; then the lobby's own Leave.
    add(SETTLE, press("direct"));
    add(SETTLE, press("direct.host"));
    reach(add, |_, ui| waiting(ui));
    shot(add, "play-waiting-direct", Pair::None);
    add(SETTLE, step(|app| come(app, Comes::Joining(Brings::New))));
    reach(add, |_, ui| part(ui) == PlayPart::Lobby && ui.global::<PlayView>().get_their_present());
    add(SETTLE, press("lobby.leave"));
    reach(add, |_, ui| part(ui) == PlayPart::Builds);

    // An invite canceled from the lobby while its practice runs (on a phone, its practice stopped from the menu
    // first, since it covers the lobby): the builds again.
    add(SETTLE, press("invite"));
    reach(add, |_, ui| waiting(ui));
    if phone {
        add(SETTLE, press("wait.train"));
        reach(add, |app, _| practicing(app));
        add(SETTLE, step(|app| app.shell.open_menu()));
        add(SETTLE, press_at("menu.rows.added", 0));
        reach(add, |app, ui| app.shell.battle.is_none() && waiting(ui));
    } else {
        reach(add, |app, _| practicing(app));
    }
    add(SETTLE, press("wait.cancel"));
    reach(add, |app, ui| app.shell.battle.is_none() && part(ui) == PlayPart::Builds);

    // The other player comes with a build that breaks a rule: its own lobby won't make it ready, and this
    // player, ready, waits for it. Then the lobby's Leave.
    add(SETTLE, press("invite"));
    reach(add, |_, ui| waiting(ui));
    add(SETTLE, step(|app| come(app, Comes::Joining(Brings::Broken))));
    reach(add, |_, ui| together(ui));
    add(SETTLE, peer(|p| p.wishes.ready = true));
    add(SETTLE, press("lobby.ready"));
    add(2000, step(|_| {}));
    reach(add, |_, ui| ui.global::<PlayView>().get_ready() && !ui.global::<PlayView>().get_their_ready() && ui.global::<PlayView>().get_note() == LobbyNote::Them);
    add(SETTLE, press("lobby.leave"));
    reach(add, |_, ui| part(ui) == PlayPart::Builds);

    // The other player invites, and the app's player joins by the code, typed: the lobby as the joiner (the
    // host's settings taken), a match from the right navi's console, and LEAVE MATCH, which puts both back
    // in the lobby.
    add(SETTLE, step(|app| come(app, Comes::Inviting)));
    reach(add, |_, _| peer_code().is_some());
    add(SETTLE, focus("code", 0));
    add(SETTLE, act(Action::Confirm));
    add(SETTLE, step(|app| app.input(crate::shell::NavInput::Edited(peer_code().unwrap_or_default().to_lowercase()))));
    add(SETTLE, act(Action::Confirm));
    add(SETTLE, press("join"));
    reach(add, |_, ui| part(ui) == PlayPart::Lobby && ui.global::<PlayView>().get_their_present());
    add(SETTLE, focus("lobby.ready", 0));
    shot(add, "play-lobby-joined", Pair::None);
    add(SETTLE, peer(|p| p.wishes.ready = true));
    add(SETTLE, press("lobby.ready"));
    reach(add, |app, _| app.shell.battle.as_ref().is_some_and(|b| b.kind == BattleKind::Online));
    add(4000, step(|_| {}));
    shot(add, "play-match-joined", Pair::None);
    add(SETTLE, step(|app| app.shell.open_menu()));
    add(SETTLE, press("menu.leave"));
    reach(add, |app, ui| app.shell.battle.is_none() && part(ui) == PlayPart::Lobby);
    shot(add, "play-lobby-left-match", Pair::None);
    // Esc, and LEAVE in the question: the builds again.
    add(SETTLE, act(Action::Back));
    reach(add, |app, _| app.shell.dialog.is_some());
    add(SETTLE, focus("dialog.button", 1));
    add(SETTLE, act(Action::Confirm));
    reach(add, |_, ui| part(ui) == PlayPart::Builds);

    add(
        SETTLE,
        step(|app| {
            RIG.with(|rig| *rig.borrow_mut() = None);
            app.go_now(ScreenId::Menu);
        }),
    );
}

/// How long the window is taken away for, each time ([`apart`]).
const AWAY: u64 = 8000;

/// The window taken away (`what`: minimized, or hidden) or brought back, as the foundation's
/// `NETTAI_WINDOW_PROBE` does it, and said in the log.
fn away(what: &'static str, gone: bool) -> Step {
    Box::new(move |ui, _, _| {
        crate::errors::log("play apart", &if gone { format!("the window away ({what})") } else { "the window back".to_string() });
        slint::winit_030::WinitWindowAccessor::with_winit_window(ui.window(), |w| match what {
            "minimize" => w.set_minimized(gone),
            _ => w.set_visible(!gone),
        });
    })
}

/// A link while its window isn't drawn (docs/app.md §1, "The clock when the window isn't drawn"), between two
/// apps on one machine: `NETTAI_TOUR_ONLY=play-away-host` in one and `play-away-join` in the other. The host
/// hosts directly on [`APART_PORT`] and the other joins it there, each through Play's own controls with a
/// build of the pass's. In the lobby, and again in the match they then play, the host's window is minimized
/// for eight seconds and then hidden for eight. Each app writes what its link saw to its log a second at a
/// time ([`Watch`]): the other's says whether it saw the host stall. No shot is taken.
pub fn apart(add: &mut dyn FnMut(u64, Step), hosts: bool) {
    let who = if hosts { "play watch (host)" } else { "play watch (join)" };
    add(
        SETTLE,
        step(|app| {
            RIG.with(|rig| *rig.borrow_mut() = Some(Rig { apart: true, ..Rig::new() }));
            app.shell.settings.remembered.retain(|key, _| !key.starts_with("play."));
            app.shell.settings.input_delay = nettai_frontend::netplay::DEFAULT_INPUT_DELAY;
            app.go_now(ScreenId::Play);
        }),
    );
    add(SETTLE, press("direct"));
    if hosts {
        add(SETTLE, press("direct.host"));
        add(SETTLE, until(|_, ui| waiting(ui)));
        add(SETTLE, step(|_| crate::errors::log("play apart", &"hosting")));
    } else {
        add(SETTLE, focus("direct.address", 0));
        add(SETTLE, act(Action::Confirm));
        add(SETTLE, step(|app| app.input(crate::shell::NavInput::Edited(format!("127.0.0.1:{APART_PORT}").into()))));
        add(SETTLE, act(Action::Confirm));
        add(SETTLE, press("direct.connect"));
    }
    // (The other app may still be loading its game: each wait is a minute at most.)
    for _ in 0..5 {
        add(SETTLE, until(|_, ui| together(ui)));
    }
    add(
        SETTLE,
        step(move |_| {
            with_rig(|rig| (rig.shoots, rig.watch) = (true, Some(Watch::new(who))));
        }),
    );
    if hosts {
        // In the lobby.
        add(4000, away("minimize", true));
        add(AWAY, away("minimize", false));
        add(4000, away("hide", true));
        add(AWAY, away("hide", false));
        add(4000, press("lobby.ready"));
        add(SETTLE, until(|app, _| in_match(app)));
        // In the match (the two are through their first custom screens by then).
        add(12000, away("minimize", true));
        add(AWAY, away("minimize", false));
        add(4000, away("hide", true));
        add(AWAY, away("hide", false));
        add(4000, step(|app| app.shell.open_menu()));
        add(SETTLE, press("menu.leave"));
        add(SETTLE, until(|app, ui| app.shell.battle.is_none() && part(ui) == PlayPart::Lobby));
        add(3000, press("lobby.leave"));
        add(SETTLE, until(|_, ui| part(ui) == PlayPart::Builds));
    } else {
        // Ready at once; then for as long as the host keeps the two together.
        add(1000, press("lobby.ready"));
        for _ in 0..3 {
            add(SETTLE, until(|app, _| in_match(app)));
        }
        for _ in 0..3 {
            add(SETTLE, until(|app, ui| app.shell.battle.is_none() && part(ui) == PlayPart::Lobby));
        }
        add(SETTLE, until(|app, ui| app.shell.dialog.is_some() || part(ui) == PlayPart::Builds));
    }
    add(
        1000,
        step(|app| {
            RIG.with(|rig| *rig.borrow_mut() = None);
            app.go_now(ScreenId::Menu);
        }),
    );
}

/// Play's bursts of the motion probe: the lobby's two set pieces, the other player's card sliding in as they
/// join and a ready mark stamping. `burst` adds a burst's steps (its name, the milliseconds its shots are
/// taken at, what starts the animation).
pub fn motion(add: &mut dyn FnMut(u64, Step), burst: &dyn Fn(&mut dyn FnMut(u64, Step), &'static str, &[u64], Step)) {
    add(
        SETTLE,
        step(|app| {
            RIG.with(|rig| *rig.borrow_mut() = Some(Rig::new()));
            app.go_now(ScreenId::Play);
        }),
    );
    // (The screen arrives first: a press is for a control that is there.)
    add(800, press("invite"));
    reach(add, |_, ui| waiting(ui));
    add(SETTLE, step(|app| come(app, Comes::Joining(Brings::New))));
    reach(add, |_, ui| together(ui));
    // (The other player has come by now: their card is taken away, and comes as it came.)
    add(800, Box::new(|ui, _, _| ui.global::<PlayView>().set_their_present(false)));
    burst(add, "lobby-opponent-joins", &[0, 40, 80, 130, 200, 320], Box::new(|ui, _, _| ui.global::<PlayView>().set_their_present(true)));
    burst(add, "lobby-ready-stamps", &[0, 30, 60, 100, 160, 280], press("lobby.ready"));
    add(SETTLE, press("lobby.leave"));
    add(
        SETTLE,
        step(|app| {
            RIG.with(|rig| *rig.borrow_mut() = None);
            app.go_now(ScreenId::Menu);
        }),
    );
}

/// Play in the showreel (`crate::tour::reel`): the builds; an invite, a practice session beside it while it waits;
/// the second player joining by its code, their card coming into the lobby; their ready mark and this player's;
/// the cards, and a few seconds of the match, the app's player pressing by itself. Then, off the air (`air`), the
/// match and the lobby left. Each wait for the other player goes on from when they have done what is waited for.
pub fn reel(add: &mut dyn FnMut(u64, Step), air: &dyn Fn(bool) -> Step) {
    let reach = |add: &mut dyn FnMut(u64, Step), come: fn(&App, &AppWindow) -> bool| {
        add(0, until(come));
        add(0, step(|_| {}));
    };
    add(
        0,
        step(|app| {
            RIG.with(|rig| *rig.borrow_mut() = Some(Rig::new()));
            app.shell.settings.remembered.retain(|key, _| !key.starts_with("play."));
            app.go_now(ScreenId::Play);
            app.shell.nav.focus("invite", 0);
            app.shell.nav.show_cursor(true, false);
        }),
    );
    add(1800, press("invite"));
    reach(add, |app, ui| waiting(ui) && practicing(app));
    add(2200, step(|app| come(app, Comes::Joining(Brings::New))));
    reach(add, |_, ui| together(ui));
    add(1100, focus("lobby.ready", 0));
    add(600, peer(|p| p.wishes.ready = true));
    reach(add, |_, ui| ui.global::<PlayView>().get_their_ready());
    add(900, press("lobby.ready"));
    reach(add, |app, _| in_match(app));
    add(0, step(|_| {
        with_rig(|rig| rig.shoots = true);
    }));
    add(5500, step(|app| app.shell.open_menu()));
    add(900, air(false));
    add(0, press("menu.leave"));
    reach(add, |app, ui| app.shell.battle.is_none() && part(ui) == PlayPart::Lobby);
    add(SETTLE, step(|_| {
        with_rig(|rig| rig.shoots = false);
    }));
    add(SETTLE, press("lobby.leave"));
    reach(add, |_, ui| part(ui) == PlayPart::Builds);
    add(
        SETTLE,
        step(|app| {
            RIG.with(|rig| *rig.borrow_mut() = None);
            app.go_now(ScreenId::Menu);
        }),
    );
}
