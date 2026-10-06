//! What drives the battle each tick: a driver gives the session each
//! tick's inputs ([`Driver`]), as live play does from the buttons
//! ([`LivePlayer`]), or runs the battle itself, as netplay does
//! (`crate::netplay`). A host may bring its own (the desktop program's
//! replays the original's recordings).

use nettai_battle::battle::mode;
use nettai_battle::console::ConsoleSetup;
use nettai_battle::cues::CueAction;
use nettai_battle::content::{ChipCode, Content};
use nettai_battle::custom::{self, BattleFolder, FolderChip, Phase, PlayerSetup, SavedFolder, SlotKind, SlotState};
use nettai_battle::input::keys;
use nettai_battle::setup::{BattleSettings, RoundSetup, SetScore};
use nettai_battle::{Battle, BattleResult, PlayerTick, Rng, TickEvents, TickInput};
use nettai_match::{After, Set};
#[cfg(test)]
use std::sync::Arc;

/// One tick's inputs.
pub struct Step {
    pub input: [PlayerTick; 2],
    pub events: TickEvents,
    /// The frame number this tick shows: the trace frame it reproduces,
    /// or the tick count in live play.
    pub frame: Option<u32>,
}

pub trait Driver {
    /// A fresh battle at the start.
    fn start(&mut self) -> Battle;
    /// The next tick's inputs (`keys`: the local player's held buttons),
    /// or None when there is nothing more to play.
    fn next(&mut self, b: &Battle, keys: u16) -> Option<Step>;
    /// Compare the battle after a step with the source, if it records
    /// what should have happened.
    fn check(&self, _b: &Battle) -> Vec<String> {
        Vec::new()
    }
    /// For a driver that plays a set by giving each tick's inputs (`next`):
    /// how the set goes on after the tick that left the battle as `b` is,
    /// if that ended the round (`nettai_match::Set::after`). The session
    /// goes on with the next round's battle, or is finished with the
    /// result. None: the round goes on (all a driver of one round says).
    fn round_ended(&mut self, _b: &Battle) -> Option<After> {
        None
    }
    /// A short description of where playback is.
    fn position(&self) -> String;
    /// The game version of the console whose screen this is, as its pack
    /// names its versions' assets, for a game whose versions the engine
    /// doesn't tell apart (EXE5's "protoman" and "colonel": the other
    /// version's chips; `Renderer::console_version`). None: the
    /// version the console's player brought (the fact `version`).
    fn console_version(&self) -> Option<&'static str> {
        None
    }
    /// The trace frames this round covers (a trace's driver).
    fn frame_range(&self) -> Option<(u32, u32)> {
        None
    }
    /// For a driver that runs the battle itself (netplay's rollback
    /// session), one wall-clock frame with the local player's buttons:
    /// put the frame to show in `shown` and say what happened. None: the
    /// driver gives each tick's inputs instead (`next`).
    fn run_frame(&mut self, _keys: u16, _shown: &mut Battle) -> Option<Result<Ran, String>> {
        None
    }
    /// How the connection to the other player is doing, of a driver that
    /// has one (netplay's): figures for a host to show as it likes.
    fn net_status(&self) -> Option<NetStatus> {
        None
    }
    /// The set's result for the local player, of a driver that knows it
    /// while it still runs (netplay's: the match is over, and the players
    /// are connected until one leaves). A driver that gives each tick's
    /// inputs says it as the set ends (`round_ended`).
    fn result(&self) -> Option<BattleResult> {
        None
    }
    /// The battle runs in real time with another player: no pause, no
    /// other speed, no restart.
    fn real_time(&self) -> bool {
        false
    }
    /// Show the frame `ticks` behind the player's newest input, of a
    /// driver that presents behind it (netplay's present delay); false
    /// from one that doesn't.
    fn set_present_delay(&mut self, _ticks: u32) -> bool {
        false
    }
}

/// How a netplay match's connection and rollback are doing
/// (`Driver::net_status`, `Player::net_status`): what a host shows of them
/// is its own to compose.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NetStatus {
    /// The smoothed round trip, in milliseconds; none until one is
    /// measured.
    pub ping_ms: Option<f64>,
    /// The share of the other player's datagrams lost, 0 to 1.
    pub loss: f64,
    /// How many ticks the frame shown is behind the player's newest input
    /// (the present delay).
    pub present_delay: u32,
    /// The last rollback's depth in ticks, the deepest so far, and how many
    /// there have been.
    pub last_rollback: u32,
    pub max_rollback: u32,
    pub rollbacks: u64,
    /// Frames the battle waited on the other player (stalled or parked).
    pub waits: u64,
}

/// What a frame of a driver that runs the battle itself did
/// (`Driver::run_frame`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ran {
    /// A new frame is in `shown` (none: the frame waited, for clock sync or
    /// the stall guard, and the last one stays).
    pub advanced: bool,
    /// A new round started (the presentation starts over).
    pub new_round: bool,
    /// The sound for this frame: cue actions (a cue played on a prediction
    /// that turned out wrong is canceled).
    pub sound: Vec<CueAction>,
}

// ---- Live play -------------------------------------------------------------------
//
// What a live round is made of (the arena, each side's player) is a match
// (`nettai_match`): live play's random pick of one (`nettai_match::pick`),
// or a match file (`--match`).

/// A round to play live on `content` with these battle settings: two
/// MegaMen of `version` (one of the names the game's rules declare) at
/// their fresh stats with a NaviCust of no programs (as live play picks
/// them: their rules build the round's stats on them), each bringing their
/// folder, shuffled from the seed, and of what
/// the game's rules take besides, the version (the engine's version fact),
/// the navi's own forms of it for their form list (the engine's: EXE6's
/// version's five Crosses) and the rules' defaults for the rest (EXE6's
/// Beast Out).
pub fn live_setup(content: &Content, settings: BattleSettings, folders: [SavedFolder; 2], version: &str, seed: u32) -> RoundSetup {
    let megaman = content.form_changing_navi().expect("a navi that changes form");
    let stats = nettai_match::Side::fresh_stats(content, megaman);
    let player = |side: u32| {
        // Each console shuffles its folder with its own RNG (RNG1), which
        // goes on from there.
        let mut rng = Rng::new(seed ^ side.wrapping_mul(0x9E37_79B9));
        let (folder, tag_pair) = BattleFolder::shuffled_with_tag_pair(&folders[side as usize], 0, &mut rng, content);
        let mut player = PlayerSetup {
            folder,
            joypad_phase: 0,
            console: ConsoleSetup { rng: rng.state, tag_pair, ..ConsoleSetup::default() },
            rules: None,
            auto_battle: Default::default(),
        };
        use nettai_battle::content::PlayerFact;
        use nettai_battle::rules::Fact;
        // (The navi, MegaMan; a NaviCust of no programs, the rules' default.)
        let navi = Fact::Value(nettai_content_api::Value::Def(nettai_content_api::Registry::Navi, megaman.0));
        player.set_fact(content, PlayerFact::Navi.name(), &[navi]).expect("the rules take the navi");
        player.set_fact(content, PlayerFact::Version.name(), &[Fact::Name(version)]).expect("the rules take the version");
        let own = content.navi(megaman).forms.as_ref().map_or(&[][..], |f| f.listed(version));
        let own: Vec<Fact> = own.iter().map(|f| Fact::Value(nettai_content_api::Value::Def(nettai_content_api::Registry::Form, f.0))).collect();
        player.set_fact(content, PlayerFact::CrossList.name(), &own).expect("the rules take a form list");
        player
    };
    RoundSetup {
        content: content.hash(),
        settings,
        navi_stats: [stats, stats],
        rng: seed,
        local_side: 0,
        score: SetScore::default(),
        later_stages: Default::default(),
        low_hp_music_latched: false,
        players: [player(0), player(1)],
    }
}

/// A folder of these chips (the content's, by key, with codes) repeated to
/// 30, with no Regular or tag chips.
pub fn folder_of(content: &Content, chips: &[(&str, u8)]) -> SavedFolder {
    let chip = |key: &str| content.defs.chip_by_key(key).unwrap_or_else(|| panic!("the content defines no chip {key:?}"));
    SavedFolder {
        chips: std::array::from_fn(|i| {
            let (key, code) = chips[i % chips.len()];
            FolderChip::new(chip(key), ChipCode(code))
        }),
        regular: None,
        tags: None,
    }
}

/// Plays a set from the keyboard, round after round to its end: the local
/// player is the left navi, with their own custom screen; the right navi
/// stands still, and its custom screen picks the first chip it can and
/// presses OK.
pub struct LivePlayer {
    set: Set,
    /// Ticks played since the start, through every round (a frame's number).
    ticks: u32,
    /// The round being played, from 1.
    round: u8,
}

impl LivePlayer {
    pub fn new(set: Set) -> LivePlayer {
        LivePlayer { set, ticks: 0, round: 1 }
    }
}

/// A set's result as its player is told it.
pub fn result_text(r: BattleResult) -> &'static str {
    match r {
        BattleResult::Won => "you won",
        BattleResult::Lost => "you lost",
        BattleResult::Drawn => "a draw",
        _ => "cut short",
    }
}

/// The right navi's buttons: on its custom screen, A on the chip under the
/// cursor (the first one) if it can be picked, then START and A, a press
/// every other tick.
pub(crate) fn bot_buttons(b: &Battle, side: usize, tick: u32) -> u16 {
    let s = &b.custom.sides[side];
    let Some(screen) = s.screen.as_ref().filter(|_| b.round.mode == mode::CUSTOM && s.in_custom) else { return 0 };
    if screen.phase != Phase::Choosing || tick % 2 == 0 {
        return 0;
    }
    let here = &screen.slots[screen.cursor as usize];
    if screen.selected == 0 && matches!(here.kind, SlotKind::Chip { .. }) && here.state == SlotState::Selectable {
        keys::A
    } else if screen.cursor != custom::screen::OK_SLOT {
        keys::START
    } else {
        keys::A
    }
}

impl Driver for LivePlayer {
    fn start(&mut self) -> Battle {
        self.ticks = 0;
        self.round = 1;
        self.set.start()
    }

    fn next(&mut self, b: &Battle, keys: u16) -> Option<Step> {
        self.ticks += 1;
        let local = b.setup.local_side as usize;
        let buttons = [0, 1].map(|side| if side == local { keys } else { bot_buttons(b, side, self.ticks) });
        // (The buttons are the whole input, as a netplay peer's are.)
        let TickInput { players, events } = nettai_netplay::standin::tick_input(b, buttons);
        Some(Step { input: players, events, frame: Some(self.ticks) })
    }

    fn round_ended(&mut self, b: &Battle) -> Option<After> {
        let after = self.set.after(b, b.setup.local_side)?;
        if matches!(after, After::Round(_)) {
            self.round += 1;
        }
        Some(after)
    }

    fn position(&self) -> String {
        format!("live round {} tick {}", self.round, self.ticks)
    }
}

#[cfg(test)]
pub(crate) mod short_set {
    use super::*;

    /// A match of `game` picked from `seed` (two MegaMen, the first a
    /// version's that has one), the right navi with 1 HP: a round is over
    /// when the left one's buster hits.
    pub fn of(content: &Arc<Content>, game: &str, seed: u32) -> nettai_match::Match {
        let mut m = nettai_match::pick::live(content, game, seed, None).unwrap();
        // (A base HP of 1: what the save brings, which the NaviCust's
        // compile makes the maximum.)
        let base_hp = nettai_battle::content::PlayerFact::BaseHp.name();
        m.sides[1].set_fact(content, base_hp, &[nettai_battle::rules::Fact::Value(nettai_content_api::Value::Int(1))]).unwrap();
        assert_eq!(nettai_match::check_match(content, &m), Vec::<String>::new());
        m
    }

    /// The buttons of the player on `side` who wins it: on the custom
    /// screen a chip and OK, as the standing navi's screen presses them; in
    /// the fight the buster, again and again.
    pub fn shooter(b: &Battle, side: usize, tick: u32) -> u16 {
        if b.round.mode == mode::CUSTOM {
            bot_buttons(b, side, tick)
        } else if tick % 8 < 2 {
            keys::B
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Live play through the first custom screen: the local player picks
    /// the first chip and presses OK, the right navi's screen does the
    /// same, and the fight starts with both hands.
    #[test]
    fn live_custom_screen() {
        let content = nettai_battle::content::testing::content();
        let stage = content.stage_by_key(nettai_battle::content::testing::LINK_BATTLE);
        let settings = BattleSettings::on(&content, stage);
        // GunDelS3 N, which the test content has.
        let folder = folder_of(&content, &[("gundels3", 13)]);
        let mut live = LivePlayer::new(Set::new(content.clone(), live_setup(&content, settings, [folder, folder], "falzar", 7), [folder, folder]));
        let mut b = live.start();
        let mut shown = false;
        for tick in 0..3000u32 {
            let s = &b.custom.sides[0];
            let choosing = s.in_custom && s.screen.as_ref().is_some_and(|x| x.phase == Phase::Choosing);
            shown |= choosing && s.screen.as_ref().is_some_and(|x| x.slots[custom::screen::OK_SLOT as usize].kind == SlotKind::Ok);
            let picked = s.screen.as_ref().is_some_and(|x| x.selected > 0);
            let on_ok = s.screen.as_ref().is_some_and(|x| x.cursor == custom::screen::OK_SLOT);
            // A press every other tick: A on the first chip, START, A on OK.
            let held = match (choosing && tick % 2 == 1, picked, on_ok) {
                (false, _, _) => 0,
                (true, false, _) | (true, true, true) => keys::A,
                (true, true, false) => keys::START,
            };
            let step = live.next(&b, held).unwrap();
            b.tick(&step.input, step.events);
            if b.round.turn == 1 && b.round.mode == mode::FIGHTING {
                break;
            }
        }
        assert!(shown);
        assert_eq!((b.round.turn, b.round.mode), (1, mode::FIGHTING));
        for side in 0..2 {
            assert_eq!(b.hands[side].remaining(), 1, "side {side}");
        }
    }

    /// `--save-match` then `--match`: live play's draw for a seed, written
    /// as a match file and played from it, is the same battle as playing
    /// the draw itself, the same digest every tick (the local player
    /// mashing, the right navi the stand-in).
    #[test]
    fn a_saved_match_plays_the_same_battle() {
        let content = nettai_match::testing::exe6_content();
        for seed in [5, 77] {
            let mut drawn = nettai_match::pick::live(&content, "exe6", seed, None).unwrap();
            // (1000 HP each, so the round lasts the test.)
            for s in &mut drawn.sides {
                s.set_fact(&content, "hp", &[nettai_battle::rules::Fact::Value(nettai_content_api::Value::Int(1000))]).unwrap();
            }
            let text = nettai_match::write(&content, &drawn);
            let read = nettai_match::parse(&content, &text).unwrap();
            let mut a = LivePlayer::new(Set::of(&content, &drawn, seed));
            let mut b = LivePlayer::new(Set::of(&content, &read, read.seed.unwrap()));
            let (mut x, mut y) = (a.start(), b.start());
            let mut masher = nettai_netplay::standin::Masher::new(seed as u64);
            for tick in 0..2500 {
                // On the custom screen, a pick and OK as the stand-in does;
                // in the fight, anything.
                let mashed = masher.buttons();
                let keys = if x.round.mode == mode::CUSTOM { bot_buttons(&x, 0, tick) } else { mashed };
                let (sa, sb) = (a.next(&x, keys).unwrap(), b.next(&y, keys).unwrap());
                x.tick(&sa.input, sa.events);
                y.tick(&sb.input, sb.events);
                assert_eq!(x.digest(), y.digest(), "seed {seed}: tick {tick}");
            }
            assert!(x.round.turn > 1, "seed {seed}: the battle went on (turn {})", x.round.turn);
        }
    }

    /// Run `live` until `done`, with the local player's buttons from
    /// `keys` (tick, battle); panics past `limit` ticks.
    fn play_until(
        live: &mut LivePlayer,
        b: &mut Battle,
        limit: u32,
        mut keys: impl FnMut(u32, &Battle) -> u16,
        mut done: impl FnMut(&Battle) -> bool,
    ) {
        for tick in 0..limit {
            if done(b) {
                return;
            }
            let held = keys(tick, b);
            let step = live.next(b, held).unwrap();
            b.tick(&step.input, step.events);
        }
        panic!("not done in {limit} ticks: mode {:#x}, turn {}", b.round.mode, b.round.turn);
    }

    /// The local player's screen, while it takes keys.
    fn choosing(b: &Battle) -> Option<&custom::Screen> {
        let s = &b.custom.sides[0];
        s.screen.as_ref().filter(|x| s.in_custom && b.round.mode == mode::CUSTOM && x.phase == Phase::Choosing)
    }

    /// Crosses of the other game on EXE6's content: a Falzar player offered
    /// HeatCross, Gregar's, chooses it on the custom screen and fights in
    /// it (its form, element, buster and charged shot: HeatCross's flame);
    /// on the next screen Beast Out from it is HeatCross's Beast form, a
    /// Gregar Beast, with its weapons.
    #[test]
    fn a_falzar_player_plays_a_gregar_cross() {
        use nettai_battle::battle::battle_flags;
        use nettai_battle::content::Element;
        use nettai_battle::kinds::player::{NaviAction, navi_action};
        let content = nettai_match::testing::exe6_content();
        let heat = content.defs.form_by_key("heatcross").unwrap();
        let heat_beast = content.defs.form_by_key("heatcross-beast").unwrap();
        let stage = nettai_match::link_battle_stages(&content, "exe6")[0];
        let settings = BattleSettings { stage, background: Default::default(), effects: content.stage(stage).effects | nettai_match::MATCH_EFFECTS };
        let folder = folder_of(&content, &[("cannon", 0)]);
        let mut setup = live_setup(&content, settings, [folder, folder], "falzar", 5);
        setup.players[0].set_fact(&content, "crosses", &[form_fact(heat)]).unwrap();
        let mut live = LivePlayer::new(Set::new(content.clone(), setup, [folder, folder]));
        let mut b = live.start();
        // The first screen: UP opens the Cross window (a hold acts on its
        // second tick), A chooses HeatCross, START and A press OK.
        play_until(
            &mut live,
            &mut b,
            3000,
            |tick, b| {
                let s = &b.custom.sides[0];
                let Some(screen) = s.screen.as_ref().filter(|_| s.in_custom && b.round.mode == mode::CUSTOM) else { return 0 };
                let w = b.form_list(0).unwrap_or_default();
                match screen.phase {
                    Phase::Choosing if w.chosen.is_none() => [keys::UP, keys::UP, 0][tick as usize % 3],
                    // (The window up, past its first tick, which reads no
                    // keys.)
                    Phase::Window { window, tick: 1.. } if b.content.defs.window(window).name == "cross_window" && w.chosen.is_none() => {
                        let under_cursor = nettai_render::custom::cross_at(b, 0, w.offered[w.cursor as usize]);
                        assert_eq!((w.count, under_cursor), (1, Some(heat)));
                        if tick % 2 == 1 { keys::A } else { 0 }
                    }
                    Phase::Choosing if tick % 2 == 1 => {
                        if screen.cursor == custom::screen::OK_SLOT { keys::A } else { keys::START }
                    }
                    _ => 0,
                }
            },
            |b| b.custom.sides[0].sent.is_some(),
        );
        assert_eq!(b.custom.sides[0].sent.as_ref().unwrap().result.transform.form, Some(heat));
        // The fight resumes and the navi changes into HeatCross.
        let p0 = b.player(0).unwrap();
        play_until(&mut live, &mut b, 1000, |_, _| 0, |b| b.stats[0].form == heat && navi_action(b, p0) == NaviAction::Idle);
        let actor = b.objects.get(p0).actor.unwrap();
        let weapons = content.form(heat).weapons;
        assert_eq!((b.actors.get(actor).buster, b.actors.get(actor).charge_shot), (weapons.buster, weapons.charge_shot));
        assert_eq!(b.objects.get(p0).element & 0xF, Element::Fire as u8);
        // B held charges the buster; let go, HeatCross's flame.
        let flame = content.defs.action_by_key("heatcross/charge/action").unwrap();
        let mut charged = false;
        play_until(
            &mut live,
            &mut b,
            600,
            |tick, _| if tick < 200 { keys::B } else { 0 },
            |b| {
                charged |= navi_action(b, p0) == NaviAction::Content(flame);
                charged
            },
        );
        // The next screen, opened with L once the gauge is full: Beast Out
        // (START, DOWN, A) from HeatCross is HeatCross's Beast form, of
        // Gregar's Beast: the Beast Out button and pictures are Gregar's.
        assert_eq!(content.form(heat).version.as_deref(), Some("gregar"));
        let mut beast = false;
        play_until(
            &mut live,
            &mut b,
            6000,
            |tick, b| {
                if b.round.mode == mode::FIGHTING {
                    return if b.round.flags & battle_flags::GAUGE_FULL != 0 && tick % 2 == 1 { keys::L } else { 0 };
                }
                let Some(screen) = choosing(b) else { return 0 };
                if b.round.turn < 2 {
                    return 0;
                }
                beast |= screen.form.is_some();
                // Each key held two ticks (a direction acts on a hold's
                // second), then let go.
                let key = match (beast, screen.cursor) {
                    (false, custom::screen::OK_SLOT) => keys::DOWN,
                    (false, custom::screen::SPECIAL_SLOT) => keys::A,
                    (false, _) => keys::START,
                    (true, custom::screen::SPECIAL_SLOT) => keys::UP,
                    (true, _) => keys::A,
                };
                [key, key, 0][tick as usize % 3]
            },
            |b| b.round.turn >= 2 && b.custom.sides[0].sent.is_some(),
        );
        assert_eq!(b.custom.sides[0].sent.as_ref().unwrap().result.transform.form, Some(heat_beast));
        play_until(&mut live, &mut b, 1000, |_, _| 0, |b| b.stats[0].form == heat_beast && navi_action(b, p0) == NaviAction::Idle);
        let weapons = content.form(heat_beast).weapons;
        assert_eq!((b.actors.get(actor).buster, b.actors.get(actor).charge_shot), (weapons.buster, weapons.charge_shot));
        assert_eq!(content.form(heat_beast).version.as_deref(), Some("gregar"));
    }

    // EXE6's Cross window (rules/cross's: content/exe6/rules/cross/
    // window.luau) with Crosses of both games, which no recording covers.

    /// A link battle on EXE6's content whose side 0 is a `version` player
    /// with the Crosses `list` (form keys; none: the version's own five),
    /// its stats changed by `tweak`, run to its first screen's choosing.
    fn cross_battle(
        version: &str,
        list: Option<&[&str]>,
        tweak: impl FnOnce(&Content, &mut nettai_battle::setup::NaviStats),
    ) -> (Arc<Content>, LivePlayer, Battle) {
        let content = nettai_match::testing::exe6_content();
        let stage = nettai_match::link_battle_stages(&content, "exe6")[0];
        let settings = BattleSettings { stage, background: Default::default(), effects: content.stage(stage).effects | nettai_match::MATCH_EFFECTS };
        let folder = folder_of(&content, &[("cannon", 0)]);
        let mut setup = live_setup(&content, settings, [folder, folder], "falzar", 5);
        setup.players[0].set_fact(&content, "version", &[nettai_battle::rules::Fact::Name(version)]).unwrap();
        let megaman = content.navi(setup.navi_stats[0].navi).forms.as_ref().expect("MegaMan's forms");
        let list: Vec<_> = match list {
            Some(list) => list.iter().map(|k| form_fact(form_of(&content, k))).collect(),
            None => megaman.listed(version).iter().map(|&f| form_fact(f)).collect(),
        };
        setup.players[0].set_fact(&content, "crosses", &list).unwrap();
        tweak(&content, &mut setup.navi_stats[0]);
        let mut live = LivePlayer::new(Set::new(content.clone(), setup, [folder, folder]));
        let mut b = live.start();
        play_until(&mut live, &mut b, 3000, |_, _| 0, |b| choosing(b).is_some());
        (content, live, b)
    }

    /// A form as a setup's fact takes it (an entry of a form list).
    fn form_fact(form: nettai_content_api::FormHandle) -> nettai_battle::rules::Fact<'static> {
        nettai_battle::rules::Fact::Value(nettai_content_api::Value::Def(nettai_content_api::Registry::Form, form.0))
    }

    /// The form `key` (EXE6's, without its prefix).
    fn form_of(content: &Content, key: &str) -> nettai_content_api::FormHandle {
        content.defs.form_by_key(&format!("{key}")).unwrap_or_else(|| panic!("no form {key}"))
    }

    /// Side 0's keys, one per tick, then on to the next tick.
    fn keys_in_turn(live: &mut LivePlayer, b: &mut Battle, held: &[u16]) {
        for &h in held {
            let step = live.next(b, h).unwrap();
            b.tick(&step.input, step.events);
        }
    }

    /// From choosing, UP opens the Cross window (a direction acts on a
    /// hold's second tick); then the window is up and takes keys.
    fn open_cross_window(live: &mut LivePlayer, b: &mut Battle) {
        keys_in_turn(live, b, &[keys::UP, keys::UP, 0]);
        play_until(live, b, 100, |_, _| 0, |b| {
            let s = b.custom.sides[0].screen.as_ref().unwrap();
            matches!(s.phase, Phase::Window { window, tick: 1.. } if b.content.defs.window(window).name == "cross_window")
        });
    }

    /// In the open Cross window, DOWN `down` times and A: the Cross under
    /// the cursor is chosen, and the screen is back to choosing chips.
    fn choose_cross(live: &mut LivePlayer, b: &mut Battle, down: usize) {
        for _ in 0..down {
            keys_in_turn(live, b, &[keys::DOWN, keys::DOWN, 0]);
        }
        keys_in_turn(live, b, &[keys::A, 0]);
        play_until(live, b, 100, |_, _| 0, |b| choosing(b).is_some());
    }

    /// OK, and what side 0 sends.
    fn confirm(live: &mut LivePlayer, b: &mut Battle) -> nettai_battle::CustomResult {
        keys_in_turn(live, b, &[keys::START, 0, keys::A, 0]);
        play_until(live, b, 200, |_, _| 0, |b| b.custom.sides[0].sent.is_some());
        b.custom.sides[0].sent.as_ref().unwrap().result.clone()
    }

    /// The rules' record of the Crosses used this round (EXE6's cross
    /// part's).
    fn crosses_used(b: &Battle) -> [bool; 5] {
        let (schema, state) = b.rules_state(0).expect("EXE6's rules");
        let i = schema.index_of("crosses_used").unwrap();
        std::array::from_fn(|k| state.get_elem(schema, i, k) == Some(nettai_content_api::FieldValue::Bool(true)))
    }

    /// A setup's Cross list offers Crosses of either game in its order: a
    /// Falzar player offered HeatCross (Gregar's first) and GroundCross
    /// (Falzar's fourth) gets those two; the one chosen shows its face and
    /// goes out, is used for the round, and isn't offered on the round's
    /// next screen.
    #[test]
    fn a_cross_list_offers_crosses_of_either_game_once_a_round() {
        use nettai_battle::battle::battle_flags;
        let (content, mut live, mut b) = cross_battle("falzar", Some(&["heatcross", "groundcross"]), |_, _| {});
        let heat = form_of(&content, "heatcross");
        let w = b.form_list(0).unwrap();
        assert_eq!((w.count, &w.offered[..2]), (2, &[0, 1][..]));
        open_cross_window(&mut live, &mut b);
        choose_cross(&mut live, &mut b, 0);
        let screen = b.custom.sides[0].screen.unwrap();
        assert_eq!((screen.look.face, b.form_list(0).unwrap().chosen), (Some(heat), Some(0)));
        assert_eq!(confirm(&mut live, &mut b).transform.form, Some(heat));
        assert_eq!(crosses_used(&b), [true, false, false, false, false]);
        // The round's next screen, opened with L once the gauge is full.
        play_until(
            &mut live,
            &mut b,
            6000,
            |tick, b| if b.round.mode == mode::FIGHTING && b.round.flags & battle_flags::GAUGE_FULL != 0 && tick % 2 == 1 { keys::L } else { 0 },
            |b| b.round.turn >= 2 && choosing(b).is_some(),
        );
        let w = b.form_list(0).unwrap();
        assert_eq!((w.count, w.offered[0]), (1, 1));
    }

    /// R in the Cross window describes the Cross under the cursor by its
    /// form: with a Cross list mixing both games, a Falzar player's window
    /// shows HeatCross's own description, not that of Falzar's Cross in its
    /// place (SpoutCross); without a list, the version's Crosses in order.
    #[test]
    fn r_in_the_cross_window_describes_the_cross_under_the_cursor() {
        for (list, down, key) in [(true, 0, "heatcross"), (true, 1, "groundcross"), (false, 0, "spoutcross"), (false, 1, "tomahawkcross")] {
            let list = list.then_some(&["heatcross", "groundcross"][..]);
            let (content, mut live, mut b) = cross_battle("falzar", list, |_, _| {});
            open_cross_window(&mut live, &mut b);
            for _ in 0..down {
                keys_in_turn(&mut live, &mut b, &[keys::DOWN, keys::DOWN, 0]);
            }
            keys_in_turn(&mut live, &mut b, &[keys::R]);
            let phase = b.custom.sides[0].screen.unwrap().phase;
            let Phase::Description { window: Some(_), form, chatbox } = phase else { panic!("{key}: no description: {phase:?}") };
            let want = form_of(&content, key);
            assert_eq!(form, Some(want), "{key}");
            let breaks = content.form(want).description_lines - 1;
            assert_eq!(chatbox.script(), custom::chatbox::Script::Description { breaks }, "{key}");
        }
    }

    /// In a Beast form a Cross list offers the Crosses whose Beast it is: in
    /// Falzar's Beast Falzar's, in a Gregar Cross's Beast form Gregar's;
    /// each takes the navi to its form in Beast Out. (The navi starts the
    /// battle in the Beast form: a spawned navi takes its starting form.)
    #[test]
    fn in_a_beast_form_a_cross_list_offers_that_beasts_crosses() {
        for (beast, place, result) in [("falzar-beast", 1, "groundcross-beast"), ("heatcross-beast", 0, "heatcross-beast")] {
            let (content, mut live, mut b) = cross_battle("falzar", Some(&["heatcross", "groundcross"]), |c, stats| {
                stats.form = form_of(c, beast);
                stats.starting_form = stats.form;
            });
            assert_eq!(b.stats[0].form, form_of(&content, beast));
            let w = b.form_list(0).unwrap();
            assert_eq!((w.count, w.offered[0]), (1, place), "{beast}");
            open_cross_window(&mut live, &mut b);
            choose_cross(&mut live, &mut b, 0);
            assert_eq!(confirm(&mut live, &mut b).transform.form, Some(form_of(&content, result)), "{beast}");
        }
    }

    /// A Cross list offers Crosses only, and leaves out the navi's starting
    /// form, as the original's window does.
    #[test]
    fn a_cross_list_offers_crosses_only() {
        let list = ["spoutcross", "gregar-beast", "eleccross", "tomahawkcross"];
        let (_, _, b) = cross_battle("gregar", Some(&list), |c, stats| stats.starting_form = form_of(c, "eleccross"));
        let w = b.form_list(0).unwrap();
        assert_eq!((w.count, &w.offered[..2]), (2, &[0, 3][..]));
    }

    /// B with nothing picked takes the Cross chosen back: its face and its
    /// form go, Beast Out is on offer again, and nothing goes out.
    #[test]
    fn b_takes_the_cross_back() {
        let (_, mut live, mut b) = cross_battle("falzar", None, |_, _| {});
        open_cross_window(&mut live, &mut b);
        choose_cross(&mut live, &mut b, 1);
        let screen = b.custom.sides[0].screen.unwrap();
        assert!(screen.form.is_some() && screen.look.face.is_some());
        assert_eq!(screen.slots[custom::screen::SPECIAL_SLOT as usize].state, SlotState::Unavailable);
        keys_in_turn(&mut live, &mut b, &[keys::B, 0]);
        let screen = b.custom.sides[0].screen.unwrap();
        assert_eq!((screen.form, screen.look.face), (None, None));
        assert_eq!(screen.slots[custom::screen::SPECIAL_SLOT as usize].state, SlotState::Selectable);
        assert_eq!(b.form_list(0).unwrap().chosen, None);
        assert_eq!(confirm(&mut live, &mut b).transform.form, None);
        assert_eq!(crosses_used(&b), [false; 5]);
    }

    /// MegaMan received from a navi code (event flag 0x163, the setup's
    /// level) has no Beast Out button (`sub_8029FB4`), and his Cross window
    /// stays (`sub_8029F70`: with the flag, MegaMan's); without a code the
    /// button is there.
    #[test]
    fn a_navi_code_seals_beast_out() {
        for level in [None, Some(3)] {
            let content = nettai_match::testing::exe6_content();
            let stage = nettai_match::link_battle_stages(&content, "exe6")[0];
            let settings = BattleSettings { stage, background: Default::default(), effects: content.stage(stage).effects | nettai_match::MATCH_EFFECTS };
            let folder = folder_of(&content, &[("cannon", 0)]);
            let mut setup = live_setup(&content, settings, [folder, folder], "falzar", 5);
            let stated = level.map_or(nettai_content_api::Value::Nil, |l| nettai_content_api::Value::Int(l as i64));
            setup.players[0].set_fact(&content, "level", &[nettai_battle::rules::Fact::Value(stated)]).unwrap();
            let mut live = LivePlayer::new(Set::new(content.clone(), setup, [folder, folder]));
            let mut b = live.start();
            play_until(&mut live, &mut b, 3000, |_, _| 0, |b| choosing(b).is_some());
            let screen = b.custom.sides[0].screen.unwrap();
            let button = match screen.slots[custom::screen::SPECIAL_SLOT as usize].kind {
                SlotKind::Button { button, .. } => Some(b.content.defs.button(button).name.as_str()),
                _ => None,
            };
            assert_eq!(button, if level.is_none() { Some("beast_out") } else { None }, "level {level:?}");
            assert_eq!(b.form_list(0).unwrap().count, 5, "level {level:?}");
        }
    }
}
