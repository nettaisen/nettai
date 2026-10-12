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
use nettai_frontend::game::{Graphics, TextMode};
use nettai_frontend::lobby::{Lobby, Role, Settings, Status, entropy};
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
use std::time::{Duration, Instant};

/// The pass's builds, by name: a new build each, of the game's versions in turn. The last breaks a rule.
const BUILDS: [&str; 5] = ["Ladder 10-18", "Swords", "Rush", "Tournament", "ジャッジ対策 v3"];

/// The second player's name.
const PEER: &str = "weenie";

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

/// How two players of one process connect: no server asked for an address, the loopback interface offered.
fn config() -> Config {
    Config { ice_servers: Vec::new(), loopback: true, ..Config::default() }
}

/// Where Play makes its links in the pass: the rig's server. None outside the pass.
pub fn net() -> Option<Net> {
    with_rig(|rig| Net { server: Some(rig.server.url()), config: config(), port: 0 })
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
            let mut side = builds.new_build(content, &builds.versions[(i + 1) % builds.versions.len()].id)?;
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
        shell.touch = (shell.touch & !was) | held;
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
            let mut lobby = Lobby::new(role, &self.content, Settings::new(&self.game, 1), self.side.clone(), entropy() ^ 1, Duration::from_secs(45));
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
                    self.preload = Some(Preload::with(self.graphics.renderer(TextMode::Original, None), None, None, &set, side as u8, (240, 160)));
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
                        None => Player::with(self.graphics.renderer(TextMode::Original, None), None, None, driver),
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

/// The second player comes: by the app's invite's code, or (no code) as a direct host's joiner.
fn join(app: &App, weak: bool) {
    let Some((game, ready)) = app.shell.ready() else { return };
    let content = ready.content();
    // What the app's player would give the other: the invite's code, or its own address.
    let invite = app.shell.ui().global::<PlayView>().get_invite().to_string();
    let made = (|| -> Result<Peer, String> {
        let builds = nettai_match::build::Builds::of(content, &game)?;
        let mut side = builds.new_build(content, &builds.versions[0].id)?;
        if weak {
            // (One HP: the app's player's first shot decides a round.)
            side.set_fact(content, PlayerFact::BaseHp.name(), &[Fact::Value(Value::Int(1))])?;
        }
        let server = with_rig(|rig| rig.server.url()).ok_or("the pass has no server")?;
        let link = if app.shell.ui().global::<PlayView>().get_invite_direct() {
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

/// Play's steps of the pass, in `lang` at `size`: `add` takes a step and the time it is given before it;
/// `shot` adds a shot's steps (its name; whether it is the first or the second of a pair).
pub fn steps(add: &mut dyn FnMut(u64, Step), shot: &dyn Fn(&mut dyn FnMut(u64, Step), &'static str, Pair)) {
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
    // A control of Play pressed, as a click presses it.
    fn press(id: &'static str) -> Step {
        step(move |app| {
            app.shell.nav.focus(id, 0);
            app.input(crate::shell::NavInput::Activate(id.into(), 0));
        })
    }
    // The step after waits for `come` (or a minute).
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

    // The rig: the pass's own server. With no build of the game, one button.
    add(
        SETTLE,
        step(|app| {
            RIG.with(|rig| *rig.borrow_mut() = Some(Rig { server: nettai_rtc::testing::Server::start(), peer: None, shoots: false, held: 0, no_builds: true }));
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

    // The builds, and the row: the focus a step along the row.
    add(SETTLE, focus("invite", 0));
    shot(add, "play", Pair::First);
    add(SETTLE, act(Action::Right));
    shot(add, "play-focus", Pair::Second);
    // A code typed, in small letters with a space, as the field keeps it; a code no match has.
    add(SETTLE, act(Action::Confirm));
    add(SETTLE, step(|app| app.with_screen(ScreenId::Play, |s, shell| s.edit(shell, "code", 0, "zz9 zz", false))));
    shot(add, "play-code", Pair::None);
    add(SETTLE, step(|app| app.with_screen(ScreenId::Play, |s, shell| s.edit(shell, "code", 0, "zz9 zz9", false))));
    add(SETTLE, act(Action::Confirm));
    add(SETTLE, press("join"));
    add(SETTLE, until(|_, ui| !ui.global::<PlayView>().get_message_quiet()));
    shot(add, "play-no-match", Pair::None);
    // A build that can't be used says so.
    add(SETTLE, focus("builds", BUILDS.len() as i32 - 1));
    add(SETTLE, press("invite"));
    shot(add, "play-build-unusable", Pair::None);
    add(SETTLE, focus("builds", 0));

    // A direct connection: the focus a step down; an address that isn't one.
    add(SETTLE, press("direct"));
    shot(add, "play-direct", Pair::First);
    add(SETTLE, act(Action::Right));
    shot(add, "play-direct-focus", Pair::Second);
    add(SETTLE, step(|app| app.with_screen(ScreenId::Play, |s, shell| s.edit(shell, "direct.address", 0, "192.168.1.20", true))));
    add(SETTLE, press("direct.connect"));
    shot(add, "play-direct-bad-address", Pair::None);
    add(SETTLE, press("direct.cancel"));

    // An invite: its code from the server, and a training battle beside it; the training menu over that.
    add(SETTLE, press("invite"));
    add(SETTLE, until(|_, ui| part(ui) == PlayPart::Waiting));
    add(2500, step(|_| {}));
    shot(add, "play-waiting", Pair::None);
    add(SETTLE, step(|app| app.shell.open_menu()));
    shot(add, "play-waiting-menu", Pair::None);
    add(SETTLE, step(|app| app.shell.close_menu()));

    // The second player joins by the code: the lobby. The focus a step along; a setting changed under it
    // (the match's, then the player's own).
    add(SETTLE, step(|app| join(app, true)));
    add(SETTLE, until(|_, ui| part(ui) == PlayPart::Lobby && ui.global::<PlayView>().get_their_present()));
    add(SETTLE, focus("lobby.ready", 0));
    shot(add, "play-lobby", Pair::First);
    add(SETTLE, act(Action::Right));
    shot(add, "play-lobby-focus", Pair::Second);
    add(SETTLE, focus("lobby.rounds", 0));
    shot(add, "play-lobby-rounds", Pair::First);
    add(SETTLE, act(Action::Right));
    add(SETTLE, until(|_, ui| ui.global::<PlayView>().get_rounds() == 1));
    shot(add, "play-lobby-rounds-single", Pair::Second);
    add(SETTLE, focus("lobby.delay", 0));
    shot(add, "play-lobby-delay", Pair::First);
    add(SETTLE, act(Action::Right));
    shot(add, "play-lobby-delay-5", Pair::Second);
    add(SETTLE, act(Action::Left));
    // Another build.
    add(SETTLE, press("lobby.change"));
    shot(add, "play-lobby-change", Pair::None);
    add(SETTLE, act(Action::Back));
    // The link drops (the other player's network is gone for a while) and comes back: the lobby's line.
    add(SETTLE, peer(|p| p.link.outage(OUTAGE)));
    add(SETTLE, until(|_, ui| ui.global::<PlayView>().get_note() == LobbyNote::Reconnecting));
    shot(add, "play-lobby-reconnecting", Pair::None);
    add(SETTLE, until(|_, ui| ui.global::<PlayView>().get_note() != LobbyNote::Reconnecting && ui.global::<PlayView>().get_ping() >= 0));
    // The other player ready; then this one, the other's loading held: the two cards, a bar each.
    add(SETTLE, peer(|p| p.wishes.ready = true));
    add(SETTLE, until(|_, ui| ui.global::<PlayView>().get_their_ready()));
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
    add(SETTLE, until(|app, _| app.shell.loading_battle()));
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
    add(SETTLE, until(|app, _| app.shell.battle.as_ref().is_some_and(|b| b.kind == BattleKind::Online)));
    add(5000, step(|_| {}));
    shot(add, "play-match", Pair::None);
    // The link drops in the match and comes back: the corner counts down what it has left.
    add(SETTLE, peer(|p| p.link.outage(OUTAGE)));
    add(SETTLE, until(|_, ui| ui.global::<nettai_ui::Shell>().get_connection_down() >= 0));
    shot(add, "play-match-reconnecting", Pair::None);
    add(SETTLE, until(|_, ui| ui.global::<nettai_ui::Shell>().get_connection_down() < 0));
    add(SETTLE, step(|app| app.shell.keys.tab = true));
    shot(add, "play-match-builds", Pair::None);
    add(
        SETTLE,
        step(|app| {
            app.shell.keys.tab = false;
            app.shell.open_menu();
        }),
    );
    shot(add, "play-match-menu", Pair::None);
    for (item, name) in [("menu.yours", "play-match-your-build"), ("menu.theirs", "play-match-their-build"), ("menu.controls", "play-match-controls"), ("menu.settings", "play-match-settings")] {
        add(SETTLE, press(item));
        shot(add, name, Pair::None);
        add(SETTLE, press("menu.back"));
    }
    // The set played to its end (the app's player shoots; the other has 1 HP): the result stays on the
    // picture, then both are back in the lobby, neither ready.
    add(
        SETTLE,
        step(|app| {
            app.shell.close_menu();
            with_rig(|rig| rig.shoots = true);
        }),
    );
    add(SETTLE, until(|app, _| app.shell.battle.as_ref().is_some_and(|b| matches!(b.stage.player.battle().fight.state, fight::WIN | fight::LOSE))));
    add(900, step(|_| {}));
    shot(add, "play-match-result", Pair::None);
    add(SETTLE, until(|app, ui| app.shell.battle.is_none() && part(ui) == PlayPart::Lobby));
    add(SETTLE, step(|_| {
        with_rig(|rig| rig.shoots = false);
    }));
    add(SETTLE, until(|_, ui| !ui.global::<PlayView>().get_ready() && !ui.global::<PlayView>().get_their_ready()));
    shot(add, "play-lobby-again", Pair::None);

    // A second match on the same link, with closed builds; the other player leaves it: both are back in the
    // lobby, which says why.
    add(SETTLE, focus("lobby.closed", 0));
    add(SETTLE, act(Action::Right));
    add(SETTLE, until(|_, ui| ui.global::<PlayView>().get_closed()));
    add(SETTLE, peer(|p| p.wishes.ready = true));
    add(SETTLE, press("lobby.ready"));
    add(SETTLE, until(|app, _| app.shell.battle.as_ref().is_some_and(|b| b.kind == BattleKind::Online)));
    add(3000, step(|app| app.shell.keys.tab = true));
    shot(add, "play-match-closed-builds", Pair::None);
    add(
        SETTLE,
        step(|app| {
            app.shell.keys.tab = false;
            app.shell.open_menu();
        }),
    );
    shot(add, "play-match-closed-menu", Pair::None);
    add(
        SETTLE,
        step(|app| {
            app.shell.close_menu();
            with_peer(|p| p.wishes.leave_match = true);
        }),
    );
    add(SETTLE, until(|app, ui| app.shell.battle.is_none() && part(ui) == PlayPart::Lobby));
    shot(add, "play-lobby-opponent-left-match", Pair::None);

    // The other player leaves for good: a failure that ends what the player was doing.
    add(SETTLE, peer(|p| p.wishes.leave = true));
    add(SETTLE, until(|app, _| app.shell.dialog.is_some()));
    shot(add, "play-opponent-left", Pair::None);
    add(SETTLE, act(Action::Confirm));

    // The other ways in, run end to end with no shot of their own: the app's player hosts directly and the
    // other joins its address; then the lobby's own Leave.
    add(SETTLE, press("direct"));
    add(SETTLE, press("direct.host"));
    add(SETTLE, until(|_, ui| part(ui) == PlayPart::Waiting));
    shot(add, "play-waiting-direct", Pair::None);
    add(SETTLE, step(|app| join(app, false)));
    add(SETTLE, until(|_, ui| part(ui) == PlayPart::Lobby && ui.global::<PlayView>().get_their_present()));
    add(SETTLE, press("lobby.leave"));
    add(SETTLE, until(|_, ui| part(ui) == PlayPart::Builds));

    add(
        SETTLE,
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
    const SETTLE: u64 = 260;
    fn step(f: impl Fn(&mut App) + 'static) -> Step {
        Box::new(move |_, app, _| f(&mut app.borrow_mut()))
    }
    fn press(app: &mut App, id: &str) {
        app.shell.nav.focus(id, 0);
        app.input(crate::shell::NavInput::Activate(id.into(), 0));
    }
    fn until(come: impl Fn(&AppWindow) -> bool + 'static) -> Step {
        let come = Rc::new(come);
        Box::new(move |_, _, tour: &Rc<Tour>| {
            let come = come.clone();
            tour.wait_until(move |app| come(&app.shell.ui()));
        })
    }
    add(
        SETTLE,
        step(|app| {
            RIG.with(|rig| *rig.borrow_mut() = Some(Rig { server: nettai_rtc::testing::Server::start(), peer: None, shoots: false, held: 0, no_builds: false }));
            app.go_now(ScreenId::Play);
            press(app, "invite");
        }),
    );
    add(SETTLE, until(|ui| ui.global::<PlayView>().get_part() == PlayPart::Waiting));
    add(SETTLE, step(|app| join(app, false)));
    add(SETTLE, until(|ui| ui.global::<PlayView>().get_part() == PlayPart::Lobby && ui.global::<PlayView>().get_their_present()));
    // (The other player has come by now: their card is taken away, and comes as it came.)
    add(SETTLE, Box::new(|ui, _, _| ui.global::<PlayView>().set_their_present(false)));
    burst(add, "lobby-opponent-joins", &[0, 40, 80, 130, 200, 320], Box::new(|ui, _, _| ui.global::<PlayView>().set_their_present(true)));
    burst(add, "lobby-ready-stamps", &[0, 30, 60, 100, 160, 280], step(|app| press(app, "lobby.ready")));
    add(
        SETTLE,
        step(|app| {
            press(app, "lobby.leave");
            RIG.with(|rig| *rig.borrow_mut() = None);
            app.go_now(ScreenId::Menu);
        }),
    );
}
