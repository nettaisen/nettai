//! One player's custom screen driven by scripted buttons through their
//! layer (`super::layer`), with made-up chips.

use super::layer::{HiddenStage, Layer, Presented, Sub};
use super::library::testing::{EVERY_CODE, TestLibrary, chip};
use super::screen::{OK_SLOT, SPECIAL_SLOT};
use super::*;
use crate::input::{PlayerTick, ScreenChoice};
use crate::content::{ButtonHandle, ChipClass, ChipCode, ChipFlags};
use crate::custom::library::testing::ChipId;
use crate::input::keys;
use nettai_content_api::ChipHandle;

const STAR: u8 = 26;
/// Made-up chips: a damaging chip in codes A-C and *, a second one in A, B
/// and *, a Mega chip in every code.
const SHOT: ChipId = 1;
const WAVE: ChipId = 2;
const MEGA: ChipId = 3;
/// A chip with the dark flag (no EXE6 chip has it), in every code.
const DARK: ChipId = 4;

fn library() -> TestLibrary {
    const SHOT_CODES: &[ChipCode] = &[ChipCode(0), ChipCode(1), ChipCode(2), ChipCode(26)];
    const WAVE_CODES: &[ChipCode] = &[ChipCode(0), ChipCode(1), ChipCode(26)];
    TestLibrary::new(
        vec![
            (SHOT, chip(ChipClass::Standard, SHOT_CODES, ChipFlags::HAS_DAMAGE, 40)),
            (WAVE, chip(ChipClass::Standard, WAVE_CODES, ChipFlags::HAS_DAMAGE, 60)),
            (MEGA, chip(ChipClass::Mega, EVERY_CODE, ChipFlags::HAS_DAMAGE, 150)),
            (DARK, chip(ChipClass::Standard, EVERY_CODE, ChipFlags::HAS_DAMAGE | ChipFlags::DARK, 300)),
        ],
        Vec::new(),
    )
}

fn stats() -> NaviStats {
    NaviStats { custom_level: 5, mega_level: 1, giga_level: 1, mood: 0x80, ..NaviStats::default() }
}

/// A folder of these chips, then plain chips.
fn folder(chips: &[(ChipId, u8)]) -> BattleFolder {
    let mut f = BattleFolder::empty();
    for (i, slot) in f.chips.iter_mut().enumerate() {
        let (id, code) = chips.get(i).copied().unwrap_or((0x50, STAR));
        *slot = Some(FolderChip::new(ChipHandle(id), ChipCode(code)));
    }
    f
}

/// EXE6's scrap and re-deal buttons (its cross and rules/navicust, in
/// Luau), in Rust for the screen's own tests: button 0 the scrap, 1 the
/// re-deal, two wide on slots 8 and 9.
struct TestButtons {
    scrap: bool,
    redeal: bool,
}

impl Extras for TestButtons {
    fn hand_size(&mut self) -> Option<u8> {
        None
    }

    fn buttons(&mut self, _: &Screen) -> Vec<ButtonPlace> {
        let place = |b| ButtonPlace { button: ButtonHandle(b), slot: 8, cells: 2, uses: 1, right: Some(11), left: Some(7), chip: None };
        if self.scrap {
            vec![place(0)]
        } else if self.redeal {
            vec![place(1)]
        } else {
            Vec::new()
        }
    }

    fn button_state(&mut self, screen: &Screen, b: ButtonHandle) -> Option<SlotState> {
        (b.0 == 0).then(|| if screen.last_pick_is_chip() { SlotState::Selectable } else { SlotState::Unavailable })
    }

    fn button_taken_back(&mut self, _: &mut Screen, _: ButtonHandle, _: u8) {}

    fn dealing(&mut self, _: &mut Screen, _: &mut BattleFolder, _: &mut crate::console::Console) {}
    fn opened(&mut self, _: &mut Screen) {}

    fn confirmed(&mut self, _: &mut Screen, _: &mut BattleFolder) {}

    fn button_hand_chip(&mut self, _: &Screen, _: crate::content::ButtonHandle) -> Option<ChipHandle> {
        None
    }

    fn chip_picked(&mut self, _: &mut Screen, _: &mut BattleFolder, _: ChipHandle) {}

    fn chip_taken_back(&mut self, _: &mut Screen, _: ChipHandle, _: u8) {}

    fn window_update(&mut self, _: &mut Screen, _: &mut BattleFolder, _: &mut Console, _: crate::content::WindowHandle) -> bool {
        false
    }

    fn choose_form(&mut self, _: &mut Screen, _: &mut BattleFolder, _: u8) {}

    fn unchoose_form(&mut self, _: &mut Screen, _: &BattleFolder) -> bool {
        false
    }

    fn member_busy(&mut self, _: u8) -> bool {
        false
    }

    fn button_pressed(&mut self, screen: &mut Screen, _: &mut BattleFolder, _: &mut Console, b: ButtonHandle) {
        let slot = screen.pressed_button_slot().expect("a button pressed");
        if screen.slots[slot as usize].state != SlotState::Selectable {
            screen.refuse();
        } else if b.0 == 0 {
            screen.start_sacrifice(slot);
        } else {
            screen.play_named("redeal");
            screen.start_redeal(slot);
        }
    }
}

/// What the player sees: their screen's state, or what their layer has up
/// over the chips.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Up {
    Screen(Phase),
    Hidden(HiddenStage),
    Description,
    /// L's message (its chatbox started).
    RunMessage { started: bool },
}

/// A player's side with this folder, their layer, and what their screen
/// reads.
struct Player {
    side: Side,
    layer: Layer,
    /// The screen's number in the round, counted as the player opens them.
    turn: u8,
    console: Console,
    lib: TestLibrary,
    stats: NaviStats,
    /// EXE6's ChpShufl (its rules' stat): the re-deal button shows.
    chip_shuffle: bool,
    /// L is the screen's run key (a console's battle).
    run_key: bool,
    /// The round's custom screen time limit, in ticks (none: a console's).
    time_limit: Option<u16>,
    tick: u32,
}

impl Player {
    fn new(chips: &[(ChipId, u8)]) -> Player {
        let setup = PlayerSetup { folder: folder(chips), ..PlayerSetup::default() };
        Player {
            side: Side::new(&setup),
            layer: Layer::on_beat(0, setup.joypad_phase),
            turn: 0,
            console: Console::new(&setup.console),
            lib: library(),
            stats: stats(),
            chip_shuffle: false,
            run_key: true,
            time_limit: None,
            tick: 0,
        }
    }

    fn context(&self) -> Context<'_> {
        context_of(&self.lib, self.stats, self.turn, self.run_key, self.time_limit, self.tick)
    }

    /// EXE6's buttons as its systems would show them.
    fn buttons(&self) -> TestButtons {
        let megaman = self.lib.changes_form(self.stats.navi);
        TestButtons {
            // (DustCross and DustCross Beast: EXE6's rules/cross's
            // `scrap_button`.)
            scrap: megaman && matches!(self.stats.form.0, 0x0A | 0x16),
            redeal: megaman && self.chip_shuffle,
        }
    }

    fn open(&mut self) {
        self.turn += 1;
        let mut buttons = self.buttons();
        let ctx = context_of(&self.lib, self.stats, self.turn, self.run_key, self.time_limit, self.tick);
        self.side.open_with(&ctx, &mut self.console, &mut buttons);
        // (The layer sees the screen as the opening tick left it.)
        self.layer.follow_on(presented(&self.side, &ctx, self.turn, self.time_limit).as_ref());
    }

    /// One tick with these buttons held, through the layer.
    fn step(&mut self, held: u16) -> Option<Request> {
        self.step_input(PlayerTick::buttons(held)).0
    }

    /// One tick of this input through the layer: what the screen asked, and
    /// the tick's input as the layer gave it.
    fn step_input(&mut self, input: PlayerTick) -> (Option<Request>, PlayerTick) {
        self.tick += 1;
        let mut buttons = self.buttons();
        let ctx = context_of(&self.lib, self.stats, self.turn, self.run_key, self.time_limit, self.tick);
        let input = self.layer.input_on(input, presented(&self.side, &ctx, self.turn, self.time_limit).as_ref());
        let lib = &self.lib;
        let r = self.side.tick_with(&ctx, input.choice, &mut self.console, |id| lib.chip(id).damage, &mut buttons);
        self.layer.follow_on(presented(&self.side, &ctx, self.turn, self.time_limit).as_ref());
        (r, input)
    }

    /// A tick as a replay plays it: the layer given the recorded input, its
    /// choice with it, and the screen taking the recorded choice alone (what
    /// the layer's keys would choose is its view, not the battle's).
    fn step_replayed(&mut self, recorded: PlayerTick) {
        self.tick += 1;
        let mut buttons = self.buttons();
        let ctx = context_of(&self.lib, self.stats, self.turn, self.run_key, self.time_limit, self.tick);
        self.layer.input_on(recorded, presented(&self.side, &ctx, self.turn, self.time_limit).as_ref());
        let lib = &self.lib;
        self.side.tick_with(&ctx, recorded.choice, &mut self.console, |id| lib.chip(id).damage, &mut buttons);
        self.layer.follow_on(presented(&self.side, &ctx, self.turn, self.time_limit).as_ref());
    }

    /// What the player sees.
    fn up(&self) -> Up {
        match self.layer.sub {
            Sub::Hidden(stage) => Up::Hidden(stage),
            Sub::Description { .. } => Up::Description,
            Sub::RunMessage { chatbox } => Up::RunMessage { started: chatbox.is_some() },
            _ => Up::Screen(self.phase()),
        }
    }

    /// The tick's screen sounds as the player hears them: the screen's, then
    /// the layer's.
    fn sounds(&self) -> Vec<look::ScreenSound> {
        let layer = self.layer.calls().filter_map(|c| match c {
            look::ScreenCall::Sound(s) => Some(s),
            look::ScreenCall::Volume { .. } => None,
        });
        self.screen().look.drawn.sounds().chain(layer).collect()
    }

    /// The volumes the tick's hover set, if it set them.
    fn volume(&self) -> Option<(u16, u16)> {
        self.layer.calls().find_map(|c| match c {
            look::ScreenCall::Volume { music, screen } => Some((music, screen)),
            look::ScreenCall::Sound(_) => None,
        })
    }

    /// Press a button: two ticks down (a direction moves the cursor on
    /// the second tick of a hold), one up. Returns the requests.
    fn press(&mut self, key: u16) -> Vec<Request> {
        [self.step(key), self.step(key), self.step(0)].into_iter().flatten().collect()
    }

    /// Ticks with nothing held until the screen asks for something (or
    /// `limit` ticks pass); returns the ticks waited.
    fn wait(&mut self, limit: u32) -> u32 {
        (1..=limit).find(|_| self.step(0).is_some()).unwrap_or(limit)
    }

    fn screen(&self) -> &Screen {
        self.side.screen.as_ref().unwrap()
    }

    fn phase(&self) -> Phase {
        self.screen().phase
    }
}

/// What a player's screen reads of the battle, in these tests.
fn context_of(lib: &TestLibrary, stats: NaviStats, turn: u8, run_key: bool, time_limit: Option<u16>, tick: u32) -> Context<'_> {
    Context {
        library: lib,
        stats,
        emotion: Emotion::default(),
        turn: turn.max(1),
        own_gauges: false,
        random_battle: false,
        battle_mode: 0,
        party: crate::party::Party::of(stats),
        switch_request: None,
        late_turns: false,
        run_key,
        time_limit,
        now: tick,
    }
}

/// The side's screen as its layer reads it.
fn presented<'a>(side: &'a Side, ctx: &'a Context, turn: u8, time_limit: Option<u16>) -> Option<Presented<'a>> {
    Some(Presented {
        screen: side.screen.as_ref()?,
        folder: &side.folder,
        view: side.view(ctx, side.folder.regular_pending),
        turn,
        time_left: side.time_left(time_limit),
        form_list: None,
    })
}

#[test]
fn five_chips_are_dealt_into_the_top_row() {
    let mut p = Player::new(&[]);
    p.open();
    let s = p.screen();
    assert_eq!(s.hand_size, 5);
    assert_eq!(p.layer.cursor, 0);
    for i in 0..5 {
        assert_eq!(s.slots[i].kind, SlotKind::Chip { index: i as u8, regular: false });
        assert_eq!(s.slots[i].vertical, None);
    }
    assert!(s.slots[5..10].iter().all(|x| matches!(x.kind, SlotKind::Empty | SlotKind::Hidden)));
    // Left of the first chip wraps to OK; OK's right to the first chip.
    assert_eq!((s.slots[0].left, s.slots[4].right), (Some(OK_SLOT), Some(OK_SLOT)));
    // (OK's up goes to the special slot when a button of the rules is there:
    // EXE6's Beast Out, which these tests have none of.)
    let ok = s.slots[OK_SLOT as usize];
    assert_eq!((ok.vertical, ok.left, ok.right), (None, Some(4), Some(0)));
    // (The special slot holds a button of the rules, EXE6's Beast Out, which
    // these tests have none of: it is absent.)
    assert!(matches!(s.slots[SPECIAL_SLOT as usize].kind, SlotKind::Empty | SlotKind::Hidden));
}

#[test]
fn the_timeline_from_opening_to_sending() {
    let mut p = Player::new(&[(SHOT, 0), (SHOT, 1)]);
    p.open();
    assert!(p.side.in_custom);
    // The window slides in for 10 ticks; keys meanwhile are lost.
    for _ in 0..10 {
        assert!(matches!(p.phase(), Phase::Opening { .. }));
        p.step(keys::A);
    }
    assert_eq!(p.up(), Up::Screen(Phase::Choosing));
    p.step(0);
    p.press(keys::RIGHT);
    assert_eq!(p.layer.cursor, 1);
    p.press(keys::A);
    assert_eq!(p.screen().selection(), [1]);
    p.press(keys::START);
    assert_eq!(p.layer.cursor, OK_SLOT);
    // OK: the status bit clears on the next tick, the hand goes out 11
    // ticks after OK and arrives 50 ticks after that (its 50 words, one a
    // tick).
    let ok_tick = p.tick + 1;
    assert_eq!(p.step(keys::A), Some(Request::Confirm));
    assert!(p.side.in_custom);
    p.step(0);
    assert!(!p.side.in_custom);
    let waited = p.wait(20);
    assert_eq!(p.tick, ok_tick + 11, "sent after {waited}");
    let sent = p.side.sent.as_ref().unwrap();
    assert_eq!(sent.arrives, ok_tick + 11 + 50);
    let hand = sent.result.hand.as_ref().unwrap();
    assert_eq!(hand.ids[..2], [Some(ChipHandle(SHOT)), None]);
    assert_eq!(hand.selection[0], Some(FolderChip::new(ChipHandle(SHOT), ChipCode(1))));
    assert_eq!(hand.damage[0], 40);
    // The picked chip left the folder.
    assert_eq!(p.side.folder.chips[1], None);
}

#[test]
fn nothing_picked_sends_no_hand() {
    let mut p = Player::new(&[]);
    p.open();
    p.wait(10);
    p.step(0);
    p.press(keys::START);
    p.press(keys::A);
    p.wait(20);
    let sent = p.side.sent.as_ref().unwrap();
    assert_eq!(sent.result.hand, None);
    assert_eq!(sent.result.transform, crate::transform::TransformRequest::NONE);
}

#[test]
fn picks_share_a_code_or_a_chip() {
    // SHOT A, SHOT B, WAVE A, WAVE *, MEGA C.
    let mut p = Player::new(&[(SHOT, 0), (SHOT, 1), (WAVE, 0), (WAVE, STAR), (MEGA, 2)]);
    p.open();
    p.wait(10);
    p.step(0);
    p.press(keys::A);
    let states = |p: &Player| p.screen().slots[..5].iter().map(|x| x.state).collect::<Vec<_>>();
    use SlotState::*;
    // The same chip in another code, the same code, and `*` go with SHOT A.
    assert_eq!(states(&p), [Selected, Selectable, Selectable, Selectable, Unavailable]);
    // With WAVE * too, only code A (or `*`) goes.
    p.press(keys::RIGHT);
    p.press(keys::RIGHT);
    p.press(keys::RIGHT);
    p.press(keys::A);
    assert_eq!(states(&p), [Selected, Unavailable, Selectable, Selected, Unavailable]);
    // A on a grayed chip does nothing; B takes back the last pick.
    p.press(keys::RIGHT);
    p.press(keys::A);
    assert_eq!(p.screen().selection(), [0, 3]);
    p.press(keys::B);
    assert_eq!(p.screen().selection(), [0]);
    assert_eq!(states(&p), [Selected, Selectable, Selectable, Selectable, Unavailable]);
}

#[test]
fn mega_chips_past_the_limit_turn_invalid() {
    let mut p = Player::new(&[(MEGA, 0)]);
    p.side.class_uses = builder::ClassCounts { mega: 2, ..Default::default() };
    p.open();
    p.wait(10);
    p.step(0);
    p.press(keys::A);
    p.press(keys::START);
    p.press(keys::A);
    p.wait(20);
    let hand = p.side.sent.as_ref().unwrap().result.hand.clone().unwrap();
    let (invalid, code) = (library::testing::INVALID, screen::INVALID_CODE);
    assert_eq!(hand.ids[0], Some(ChipHandle(invalid)));
    assert_eq!(hand.selection[0], Some(FolderChip::new(ChipHandle(invalid), code)));
}

#[test]
fn dust_cross_scraps_the_picks() {
    let mut p = Player::new(&[(SHOT, 0), (SHOT, 1), (WAVE, 0), (WAVE, 1), (SHOT, 2), (MEGA, 5), (MEGA, 6)]);
    p.stats.form = library::testing::DUST_CROSS;
    p.open();
    assert!(matches!(p.screen().slots[8].kind, SlotKind::Button { button: ButtonHandle(0), cell: ButtonCell::Left }));
    p.wait(10);
    p.step(0);
    p.press(keys::A);
    p.press(keys::RIGHT);
    p.press(keys::A);
    // Down from the fourth chip to the scrap button.
    p.press(keys::RIGHT);
    p.press(keys::RIGHT);
    p.press(keys::DOWN);
    assert_eq!(p.layer.cursor, 8);
    let a = p.tick + 1;
    p.step(keys::A);
    while p.up() != Up::Screen(Phase::Choosing) && p.tick < 1000 {
        p.step(0);
    }
    // 25 ticks a chip, and 3 more.
    assert_eq!(p.tick, a + 2 * 25 + 3);
    assert_eq!(p.screen().selected, 0);
    assert_eq!(p.screen().slots[8].state, SlotState::Selected);
    // The scrapped chips went to the end of the folder, in pick order, and
    // the hand is dealt again from the front.
    let f = p.side.folder;
    let ids: Vec<(ChipId, u8)> = f.chips.iter().flatten().map(|c| (c.id.0, c.code.0)).collect();
    assert_eq!(ids[..5], [(WAVE, 0), (WAVE, 1), (SHOT, 2), (MEGA, 5), (MEGA, 6)]);
    assert_eq!(ids[28..], [(SHOT, 0), (SHOT, 1)]);
}

#[test]
fn dust_cross_scrapping_the_regular_chip_ends_it() {
    // sub_8027458: the Regular chip scrapped clears BattleState+0x17, so
    // the next screen deals no Regular chip. (This screen's front slot
    // keeps its Regular bit: sub_802A61A, which shows the chips dealt
    // again, only resets the slots' states.)
    let chips = [(SHOT, 0), (SHOT, 1), (WAVE, 0), (WAVE, 1), (SHOT, 2), (MEGA, 5), (MEGA, 6)];
    let mut p = Player::new(&chips);
    let mut f = folder(&chips);
    f.regular_pending = true;
    p.side.folder = f;
    p.stats.form = library::testing::DUST_CROSS;
    p.open();
    assert!(matches!(p.screen().slots[0].kind, SlotKind::Chip { regular: true, .. }));
    p.wait(10);
    p.step(0);
    p.press(keys::A);
    // Down from the fourth chip to the scrap button.
    for _ in 0..3 {
        p.press(keys::RIGHT);
    }
    p.press(keys::DOWN);
    assert_eq!(p.layer.cursor, 8);
    p.step(keys::A);
    while p.up() != Up::Screen(Phase::Choosing) && p.tick < 1000 {
        p.step(0);
    }
    assert!(!p.side.folder.regular_pending);
    assert!(matches!(p.screen().slots[0].kind, SlotKind::Chip { regular: true, .. }));
}

#[test]
fn hand_size() {
    let size = |custom_level: u8, shrink: u8, turn: u8| {
        let mut p = Player::new(&[]);
        p.stats.custom_level = custom_level;
        p.stats.bugs.hand_shrink_turn = shrink;
        let ctx = Context { turn, ..p.context() };
        let mut side = p.side.clone();
        side.open(&ctx, &mut p.console.clone());
        side.screen.unwrap().hand_size
    };
    assert_eq!(size(5, 0, 1), 5);
    assert_eq!(size(10, 0, 1), 8);
    assert_eq!(size(6, 2, 2), 5);
    assert_eq!(size(6, 2, 5), 2);
}

#[test]
fn select_hides_the_window_until_a_key() {
    let mut p = Player::new(&[]);
    p.open();
    p.wait(10);
    p.step(0);
    p.press(keys::SELECT);
    assert!(matches!(p.up(), Up::Hidden(_)));
    p.step(0);
    // The key that brings the window back does nothing else, even as it
    // starts to repeat.
    p.step(keys::RIGHT);
    p.step(keys::RIGHT);
    assert_eq!(p.up(), Up::Screen(Phase::Choosing));
    p.step(keys::RIGHT);
    assert_eq!(p.layer.cursor, 0);
}

/// The emblem as the hidden window comes back is the game's screen's: one
/// whose layout says `emblem_at_window_return` draws it on the tick of the
/// key and the next (EXE6's `sub_8026D06`), one that doesn't on neither
/// (EXE5's 0x08023022): it is back with the choosing state's own draw.
#[test]
fn the_returning_window_draws_the_emblem_where_the_screen_does() {
    for at_return in [true, false] {
        let mut p = Player::new(&[]);
        p.lib.layout.emblem_at_window_return = at_return;
        p.open();
        p.wait(10);
        p.step(0);
        assert!(p.layer.look.drawn.emblem.is_some(), "the choosing state draws it");
        p.press(keys::SELECT);
        p.step(0);
        assert!(p.up() == Up::Hidden(HiddenStage::Waiting));
        assert!(p.layer.look.drawn.emblem.is_none(), "hidden");
        p.step(keys::RIGHT);
        assert!(p.up() == Up::Hidden(HiddenStage::Restoring));
        assert_eq!(p.layer.look.drawn.emblem.is_some(), at_return, "the tick of the key");
        p.step(keys::RIGHT);
        assert_eq!(p.up(), Up::Screen(Phase::Choosing));
        assert_eq!(p.layer.look.drawn.emblem.is_some(), at_return, "the next");
        p.step(0);
        assert!(p.layer.look.drawn.emblem.is_some(), "choosing again");
    }
}

#[test]
fn chip_shuffle_redeals_what_is_not_picked() {
    // Thirty different chips (ids and codes), the first one the pick.
    let chips: Vec<(ChipId, u8)> = (0..30).map(|i| ([SHOT, WAVE][i % 2], (i / 2) as u8 % 3)).collect();
    let mut p = Player::new(&chips);
    p.chip_shuffle = true;
    p.console = Console::new(&ConsoleSetup { rng: 0x1234_5678, ..ConsoleSetup::default() });
    p.open();
    assert!(matches!(p.screen().slots[8].kind, SlotKind::Button { button: ButtonHandle(1), cell: ButtonCell::Left }));
    p.wait(10);
    p.step(0);
    p.press(keys::A);
    let before = p.side.folder;
    // Down from the fourth chip to the re-deal button.
    p.press(keys::RIGHT);
    p.press(keys::RIGHT);
    p.press(keys::RIGHT);
    p.press(keys::DOWN);
    assert_eq!(p.layer.cursor, 8);
    let rng = p.console.rng;
    let a = p.tick + 1;
    p.step(keys::A);
    assert!(matches!(p.phase(), Phase::Redealing { .. }));
    while p.up() != Up::Screen(Phase::Choosing) && p.tick < 1000 {
        p.step(0);
    }
    // The first tick, then 32.
    assert_eq!(p.tick, a + 33);
    // The pick stays; the other 29 are shuffled once from the RNG as it
    // was (29 rounds, two draws each), which the 7 shows in between (the
    // same again each) don't change.
    let after = p.side.folder;
    assert_eq!(after.chips[0], before.chips[0]);
    let mut expected = before.chips[1..].to_vec();
    let mut r = rng;
    super::folder::shuffle(&mut expected, 2 * 29, || r.next_positive().into());
    assert_eq!(after.chips[1..], expected[..]);
    for _ in 0..7 {
        super::folder::shuffle(&mut expected, 2 * 29, || r.next_positive().into());
    }
    assert_eq!(p.console.rng, r);
    // The button is used up; the pick is still the pick.
    assert_eq!(p.screen().slots[8].state, SlotState::Unavailable);
    assert_eq!(p.screen().selection(), &[0]);
}

#[test]
fn chip_shuffle_leaves_the_regular_chip_and_the_tag_pair() {
    let chips: Vec<(ChipId, u8)> = (0..30).map(|i| ([SHOT, WAVE][i % 2], (i / 2) as u8 % 3)).collect();
    let mut p = Player::new(&chips);
    p.chip_shuffle = true;
    let mut f = folder(&chips);
    f.regular_pending = true;
    p.side.folder = f;
    p.console = Console::new(&ConsoleSetup { rng: 0x0BAD_F00D, tag_pair: Some(12), ..ConsoleSetup::default() });
    p.open();
    assert!(matches!(p.screen().slots[0].kind, SlotKind::Chip { regular: true, .. }));
    p.wait(10);
    p.step(0);
    let before = p.side.folder;
    p.press(keys::RIGHT);
    p.press(keys::RIGHT);
    p.press(keys::RIGHT);
    p.press(keys::DOWN);
    p.step(keys::A);
    while p.up() != Up::Screen(Phase::Choosing) && p.tick < 1000 {
        p.step(0);
    }
    let after = p.side.folder;
    // The Regular chip (entry 0) and the tag pair (entries 12 and 13)
    // stay where they are.
    assert_eq!(after.chips[0], before.chips[0]);
    assert_eq!(after.chips[12..14], before.chips[12..14]);
    assert_ne!(after.chips, before.chips);
}

#[test]
fn the_tag_pair_index_follows_the_folder_as_picks_leave_it() {
    let chips: Vec<(ChipId, u8)> = (0..30).map(|i| ([SHOT, WAVE][i % 2], (i / 2) as u8 % 3)).collect();
    let mut p = Player::new(&chips);
    p.chip_shuffle = true;
    p.console = Console::new(&ConsoleSetup { rng: 0x0BAD_F00D, tag_pair: Some(12), ..ConsoleSetup::default() });
    p.open();
    p.wait(10);
    p.step(0);
    let pair = p.side.folder.chips[12..14].to_vec();
    // Two picks (the same chip in two codes), then OK: each chip taken out
    // moves the pair's index down with the pair.
    p.press(keys::A);
    p.press(keys::RIGHT);
    p.press(keys::RIGHT);
    p.press(keys::A);
    assert_eq!(p.screen().selection(), [0, 2]);
    p.press(keys::START);
    assert_eq!(p.step(keys::A), Some(Request::Confirm));
    assert_eq!(p.console.tag_pair, Some(10));
    p.wait(20);
    // The next screen closes the folder up, and its re-deal leaves the
    // pair where it now is.
    p.open();
    p.wait(10);
    p.step(0);
    let before = p.side.folder;
    assert_eq!(before.chips[10..12], pair[..]);
    p.press(keys::RIGHT);
    p.press(keys::RIGHT);
    p.press(keys::RIGHT);
    p.press(keys::DOWN);
    assert_eq!(p.layer.cursor, 8);
    p.step(keys::A);
    while p.up() != Up::Screen(Phase::Choosing) && p.tick < 1000 {
        p.step(0);
    }
    let after = p.side.folder;
    assert_eq!(after.chips[10..12], pair[..]);
    assert_ne!(after.chips, before.chips);
}

/// A screen whose first chip (SHOT A) has a description of `lines` lines,
/// taking keys.
fn describing(lines: usize) -> Player {
    let mut p = Player::new(&[(SHOT, 0), (SHOT, 1)]);
    p.lib.chips[0].1.description_lines = crate::content::Shown(lines as u8);
    p.open();
    p.wait(10);
    p.step(0);
    p
}

#[test]
fn a_description_takes_keys_later_for_each_line() {
    for (lines, first) in [(1usize, 6u32), (2, 7), (3, 8)] {
        // A the tick before the box takes keys is lost.
        let mut p = describing(lines);
        p.step(keys::R);
        assert!(p.up() == Up::Description);
        for _ in 1..first - 1 {
            p.step(0);
        }
        p.step(keys::A);
        for _ in 0..30 {
            p.step(0);
        }
        assert!(p.up() == Up::Description, "{lines} lines: A on tick {} closed it", first - 1);
        // On the tick it does, the box closes; the grid takes keys six
        // ticks after the key (not five).
        let mut p = describing(lines);
        p.step(keys::R);
        for _ in 1..first {
            p.step(0);
        }
        p.step(keys::A);
        for _ in 0..4 {
            p.step(0);
            assert!(p.up() == Up::Description);
        }
        p.step(keys::A);
        assert_eq!(p.up(), Up::Screen(Phase::Choosing), "{lines} lines");
        assert_eq!(p.screen().selection(), &[] as &[u8]);
        p.step(0);
        p.step(keys::A);
        assert_eq!(p.screen().selection(), [0]);
    }
}

#[test]
fn b_held_closes_a_description() {
    // B pressed before the box takes keys and held: its eleventh tick of
    // taking keys answers it (tick 18 of a three-line description), and
    // the screen is back four ticks later.
    let mut p = describing(3);
    p.step(keys::R);
    for _ in 1..=17 {
        p.step(keys::B);
    }
    for _ in 0..10 {
        p.step(0);
    }
    assert!(p.up() == Up::Description);
    let mut p = describing(3);
    p.step(keys::R);
    for _ in 1..=18 {
        p.step(keys::B);
    }
    for _ in 0..4 {
        assert!(p.up() == Up::Description);
        p.step(0);
    }
    p.step(0);
    assert_eq!(p.up(), Up::Screen(Phase::Choosing));
}

/// A navi whose content states no no-running message has no box to open:
/// L does nothing, and the grid takes the next key.
#[test]
fn l_opens_no_message_for_a_navi_without_one() {
    let mut p = describing(3);
    p.stats.navi = library::testing::SILENT_NAVI;
    p.step(keys::L);
    assert_eq!(p.up(), Up::Screen(Phase::Choosing));
    p.step(0);
    p.step(keys::A);
    assert_eq!(p.screen().selection(), [0]);
}

/// In a round whose host took the run key (`HostKeys::run`), L on the
/// choosing grid opens no message and makes no sound, for a navi with one:
/// the key is read and does nothing, and the grid takes the next key. L is
/// still any key to a description's box and to the hidden window.
#[test]
fn l_does_nothing_when_the_host_took_the_run_key() {
    let mut p = describing(3);
    p.run_key = false;
    for held in [keys::L, 0, keys::L, keys::L, 0] {
        p.step(held);
        assert_eq!(p.up(), Up::Screen(Phase::Choosing));
        assert_eq!(p.sounds().len(), 0, "L makes no sound");
    }
    p.step(keys::A);
    assert_eq!(p.screen().selection(), [0]);
    p.step(0);
    // A description closes on any key, L among them.
    p.step(keys::R);
    assert!(p.up() == Up::Description);
    for _ in 1..8 {
        p.step(0);
    }
    p.step(keys::L);
    for _ in 0..6 {
        p.step(0);
    }
    assert_eq!(p.up(), Up::Screen(Phase::Choosing));
    // The hidden window comes back on any key, L among them.
    p.press(keys::SELECT);
    p.step(0);
    assert!(p.up() == Up::Hidden(HiddenStage::Waiting));
    p.step(keys::L);
    assert!(p.up() == Up::Hidden(HiddenStage::Restoring));
    p.step(0);
    assert_eq!(p.up(), Up::Screen(Phase::Choosing));
    // And on a console it is the run key still.
    p.run_key = true;
    p.step(0);
    p.step(keys::L);
    assert_eq!(p.up(), Up::RunMessage { started: false });
}

#[test]
fn the_no_running_message_starts_a_tick_after_l() {
    let mut p = describing(3);
    p.step(keys::L);
    assert_eq!(p.up(), Up::RunMessage { started: false });
    // Its box takes A or B from its 80th tick (the 80th after L): a press
    // a tick before is lost.
    for _ in 1..79 {
        p.step(0);
    }
    p.step(keys::A);
    for _ in 0..20 {
        p.step(0);
    }
    assert!(matches!(p.up(), Up::RunMessage { .. }));
    p.step(keys::A);
    // Closed seven ticks later; the screen sees it the tick after, and
    // takes keys the tick after that.
    for _ in 0..8 {
        assert!(matches!(p.up(), Up::RunMessage { .. }));
        p.step(0);
    }
    assert_eq!(p.up(), Up::Screen(Phase::Choosing));
    assert_eq!(p.screen().selection(), &[] as &[u8]);
    p.step(keys::A);
    assert_eq!(p.screen().selection(), [0]);
}

#[test]
fn a_dark_chip_takes_the_cursor_and_darkens_the_screen() {
    use crate::battle::FadeMode;
    let mut p = Player::new(&[(SHOT, 0), (DARK, 0), (SHOT, 1)]);
    p.open();
    // sub_802806C: the cursor starts on the first dark chip dealt.
    assert_eq!(p.layer.cursor, 1);
    // The hover starts on the tick the window is in (the chips' state).
    for _ in 0..10 {
        p.step(0);
    }
    assert_eq!(p.up(), Up::Screen(Phase::Choosing));
    let look = p.layer.hover;
    assert_eq!(look.shade, ChipShade::Shading { step: 0 });
    assert_eq!((look.fade.mode, look.window_fade.mode), (FadeMode::Shade, FadeMode::ShadeWindow));
    // The music turns down and the screen's player up, a step a tick, until
    // the window's fade is done.
    let volumes = |p: &mut Player, n: usize| {
        (0..n)
            .map(|_| {
                p.step(0);
                p.volume()
            })
            .collect::<Vec<_>>()
    };
    let down = volumes(&mut p, 6);
    assert_eq!(down, [Some((0x100, 0x80)), Some((0xE0, 0x80)), Some((0xC0, 0xA0)), Some((0xA0, 0xC0)), Some((0x80, 0xE0)), None]);
    let look = p.layer.hover;
    assert_eq!(look.shade, ChipShade::Shaded);
    assert_eq!(look.window_fade.level, 0x30);
    // (The screen's own fade, eight steps to 0x50, has one to go.)
    assert_eq!((look.fade.level, look.fade.active()), (0x46, true));
    // The cursor leaves it (on the second tick of the hold): back the other
    // way, a step longer.
    p.step(keys::RIGHT);
    p.step(keys::RIGHT);
    assert_eq!(p.layer.cursor, 2);
    assert_eq!(p.layer.hover.shade, ChipShade::Clearing { step: 0 });
    let up = volumes(&mut p, 7);
    assert_eq!(
        up,
        [Some((0x80, 0x100)), Some((0x80, 0xE0)), Some((0xA0, 0xC0)), Some((0xC0, 0xA0)), Some((0xE0, 0x80)), Some((0x100, 0x80)), None]
    );
    let look = p.layer.hover;
    assert_eq!((look.shade, look.window_fade.level), (ChipShade::Clear, 0));
    // The screen's own fade, back from 0x50, clears a tick later.
    assert_eq!(p.layer.hover.fade.level, 0xA);
    p.step(0);
    assert_eq!((p.layer.hover.fade.level, p.layer.hover.fade.active()), (0, false));
}

#[test]
fn the_cursor_stays_put_without_a_dark_chip() {
    let mut p = Player::new(&[(SHOT, 0), (MEGA, 0)]);
    p.open();
    assert_eq!(p.layer.cursor, 0);
    for _ in 0..20 {
        p.step(0);
        assert_eq!((p.layer.hover.shade, p.volume()), (ChipShade::Clear, None));
    }
}

/// Run one of the screen's routines the rules' content calls (`custom.*`),
/// with the side's folder and view.
fn on_screen<R>(p: &mut Player, f: impl FnOnce(&mut Screen, &BattleFolder, &super::screen::PlayerView) -> R) -> R {
    let side = p.side.clone();
    let folder = side.folder;
    let mut screen = side.screen.expect("a screen");
    let r = {
        let ctx = p.context();
        let view = side.view(&ctx, folder.regular_pending);
        f(&mut screen, &folder, &view)
    };
    p.side.screen = Some(screen);
    r
}

/// OK, and the hand the side sends.
fn confirm(p: &mut Player) -> crate::hand::ChipHand {
    p.press(keys::START);
    assert_eq!(p.step(keys::A), Some(Request::Confirm));
    p.step(0);
    p.wait(20);
    p.side.sent.as_ref().expect("a result sent").result.hand.clone().expect("a hand")
}

/// `custom.attach_to_last_pick` (EXE5's capsules): the last pick carries
/// the button's modifier bits into the hand, one button a chip; B on the
/// chip clears them and frees the button.
#[test]
fn a_button_attached_to_a_pick_marks_it_into_the_hand() {
    let mut p = Player::new(&[(SHOT, 0), (WAVE, 0), (SHOT, 0)]);
    p.open();
    p.wait(10);
    p.step(0);
    // Nothing picked: nothing to attach to.
    assert!(!on_screen(&mut p, |s, f, v| s.attach_to_last_pick(8, 0x04, f, v)));
    p.press(keys::A);
    assert!(on_screen(&mut p, |s, f, v| s.attach_to_last_pick(8, 0x04, f, v)));
    let s = p.screen();
    assert_eq!((s.slots[0].marks, s.slots[0].attached, s.slots[8].state), (0x04, Some(8), SlotState::Selected));
    assert!(on_screen(&mut p, |s, f, v| s.last_pick(f, v)).is_some_and(|l| l.attached));
    // A second button on the same chip is refused.
    assert!(!on_screen(&mut p, |s, f, v| s.attach_to_last_pick(9, 0x10, f, v)));
    assert_eq!(p.screen().slots[0].marks, 0x04);
    // B: the chip is taken back, its marks with it, and the button is free.
    p.press(keys::B);
    let s = p.screen();
    assert_eq!((s.selected, s.slots[0].marks, s.slots[0].attached, s.slots[8].state), (0, 0, None, SlotState::Selectable));
    // Picked and attached again (the Regular chip's bit is never a
    // button's), then OK: the hand's modifiers carry the bits.
    p.press(keys::A);
    assert!(on_screen(&mut p, |s, f, v| s.attach_to_last_pick(8, 0x04 | 0x01, f, v)));
    assert_eq!(p.screen().slots[0].marks, 0x04);
    let hand = confirm(&mut p);
    assert_eq!((hand.ids[0], hand.modifiers[0]), (Some(ChipHandle(SHOT)), 0x04));
}

/// `custom.hold_last_pick` (EXE5's Arm Change): the last pick leaves the
/// picks for the button; B puts it back once the picks are as they were
/// then, after any later pick; at OK it leaves the folder without being in
/// the hand.
#[test]
fn a_button_holding_a_pick_takes_it_out_of_the_picks_and_the_folder() {
    let mut p = Player::new(&[(SHOT, 0), (WAVE, 0), (SHOT, 0)]);
    p.open();
    p.wait(10);
    p.step(0);
    assert!(!on_screen(&mut p, |s, f, v| s.hold_last_pick(8, f, v)), "nothing picked");
    p.press(keys::A);
    p.press(keys::RIGHT);
    p.press(keys::A);
    assert_eq!(p.screen().selection(), [0, 1]);
    assert!(on_screen(&mut p, |s, f, v| s.hold_last_pick(8, f, v)));
    let s = p.screen();
    assert_eq!(s.selection(), [0]);
    assert_eq!(s.hold.map(|h| (h.button, h.chip, h.at)), Some((8, 1, 1)));
    assert_eq!((s.slots[1].state, s.slots[8].state), (SlotState::Selected, SlotState::Selected));
    // (Its cell in the column stays drawn until the blink redraws it.)
    assert_eq!(s.look.column_kept, Some(1));
    assert!(!on_screen(&mut p, |s, f, v| s.hold_last_pick(8, f, v)), "one chip held at a time");
    assert_eq!(on_screen(&mut p, |s, f, v| s.held_pick(8, f, v)).map(|c| c.id), Some(ChipHandle(WAVE)));
    assert_eq!(on_screen(&mut p, |s, f, v| s.held_pick(9, f, v)), None);
    // The blink: the icon in the cell it left, hidden or shown.
    assert!(on_screen(&mut p, |s, f, v| s.set_held_icon(8, false, f, v)));
    assert_eq!((p.screen().look.column[1], p.screen().look.column_kept), (None, None));
    assert!(on_screen(&mut p, |s, f, v| s.set_held_icon(8, true, f, v)));
    assert_eq!(p.screen().look.column[1].map(|c| c.id), Some(ChipHandle(WAVE)));
    assert!(on_screen(&mut p, |s, f, v| s.set_held_icon(8, false, f, v)));
    // The held chip is drawn over the button while choosing.
    p.step(0);
    assert!(p.layer.look.drawn.held);
    // A later pick is taken back first; then, the picks as they were, the
    // held chip is the last pick again and the button is free.
    p.press(keys::RIGHT);
    p.press(keys::A);
    assert_eq!(p.screen().selection(), [0, 2]);
    p.press(keys::B);
    assert_eq!(p.screen().selection(), [0]);
    assert!(p.screen().hold.is_some());
    p.press(keys::B);
    let s = p.screen();
    assert_eq!((s.selection(), s.hold, s.slots[8].state), (&[0u8, 1][..], None, SlotState::Selectable));
    assert_eq!(s.look.column[1].map(|c| c.id), Some(ChipHandle(WAVE)));
    assert!(!p.layer.look.drawn.held);
    // Held again, and OK: the first pick alone is the hand; both chips
    // left the folder.
    assert!(on_screen(&mut p, |s, f, v| s.hold_last_pick(8, f, v)));
    let hand = confirm(&mut p);
    assert_eq!(hand.ids[..2], [Some(ChipHandle(SHOT)), None]);
    let f = p.side.folder;
    assert_eq!((f.chips[0], f.chips[1]), (None, None));
    assert_eq!(f.chips[2], Some(FolderChip::new(ChipHandle(SHOT), ChipCode(0))));
}

/// A button that shows a chip (`ButtonPlace::chip`, EXE5's capsules): the
/// chip window keeps the frame of the last chip slot it showed (OK and a
/// button's picture set the standard one), and R describes the chip.
#[test]
fn a_buttons_chip_keeps_the_chip_windows_frame_and_is_described() {
    let mut p = Player::new(&[(MEGA, 0), (SHOT, 0)]);
    p.open();
    p.wait(10);
    p.step(0);
    let framed = |p: &Player| p.layer.chip_window.framed.map(|c| c.id);
    assert_eq!(framed(&p), Some(ChipHandle(MEGA)));
    // The second slot a button showing a chip, the third a plain button.
    {
        let s = p.side.screen.as_mut().unwrap();
        s.slots[1] = Slot { kind: SlotKind::Button { button: ButtonHandle(1), cell: ButtonCell::Only }, face: Some(ChipHandle(WAVE)), ..s.slots[1] };
        s.slots[2] = Slot { kind: SlotKind::Button { button: ButtonHandle(1), cell: ButtonCell::Only }, face: None, ..s.slots[2] };
    }
    p.press(keys::RIGHT);
    assert_eq!((p.layer.cursor, p.layer.chip_window.slot), (1, 1));
    assert_eq!(framed(&p), Some(ChipHandle(MEGA)), "the button's chip sets no frame");
    // R: its chip's description.
    p.press(keys::R);
    assert!(p.up() == Up::Description);
    while p.up() != Up::Screen(Phase::Choosing) && p.tick < 1000 {
        p.press(keys::A);
    }
    assert_eq!(p.up(), Up::Screen(Phase::Choosing));
    // A button's picture: the standard frame; R on it describes nothing.
    p.press(keys::RIGHT);
    assert_eq!((p.layer.cursor, framed(&p)), (2, None));
    p.press(keys::R);
    assert_eq!(p.up(), Up::Screen(Phase::Choosing));
    // Back on the chip: its own again; OK: the standard one.
    p.press(keys::LEFT);
    p.press(keys::LEFT);
    assert_eq!((p.layer.cursor, framed(&p)), (0, Some(ChipHandle(MEGA))));
    p.press(keys::START);
    assert_eq!((p.layer.cursor, framed(&p)), (OK_SLOT, None));
}

/// The hover over a dark chip plays its sound each time the hover's
/// counter wraps, every 64 ticks from the screen's opening, while the
/// screen isn't clear (EXE5's 0x08025A8C); never without a dark chip.
#[test]
fn the_dark_hover_sounds_every_64_ticks() {
    let sounded = |chips: &[(ChipId, u8)]| -> Vec<u32> {
        let mut p = Player::new(chips);
        p.open();
        let mut ticks = Vec::new();
        for _ in 0..200 {
            p.step(0);
            if p.sounds().into_iter().any(|s| s == super::look::ScreenSound::Shade) {
                assert_eq!(p.layer.hover.count, 0);
                assert_ne!(p.layer.hover.shade, ChipShade::Clear);
                ticks.push(p.tick);
            }
        }
        ticks
    };
    let ticks = sounded(&[(DARK, 0), (SHOT, 0)]);
    assert_eq!(ticks.len(), 3, "{ticks:?}");
    assert!(ticks.windows(2).all(|w| w[1] - w[0] == 64), "{ticks:?}");
    assert_eq!(sounded(&[(SHOT, 0), (MEGA, 0)]), Vec::<u32>::new());
}

/// The hover's routine runs its state before it steps its counter (EXE5's
/// 0x08025A80), so on a tick where a volume step and the counter's wrap
/// fall together, the volumes are asked for before the sound (the chip
/// lab's custom/dark-hover has one: its frame 598).
#[test]
fn the_dark_hover_sets_its_volumes_before_it_sounds() {
    use super::look::{Hover, ScreenCall, ScreenSound};
    let mut look = Hover::default();
    let rules = crate::content::testing::rules().custom_screen.hover;
    // A tick of the hover as the layer runs it: the hover, then the fades'
    // steps; what it asked of the sound driver, in order.
    let tick = |look: &mut Hover, on_dark: bool| {
        let mut calls = Vec::new();
        look.hover(Some(on_dark), &rules, &mut calls);
        look.step_fades(crate::content::FadeClear::AtTarget);
        calls
    };
    // Off a dark chip for 60 ticks: nothing asked.
    for _ in 0..60 {
        assert_eq!(tick(&mut look, false), []);
    }
    // Onto one: the hover starts, a volume step a tick follows, and the
    // third falls on the counter's wrap.
    assert_eq!(tick(&mut look, true), []);
    assert_eq!(tick(&mut look, true), [ScreenCall::Volume { music: 0x100, screen: 0x80 }]);
    assert_eq!(tick(&mut look, true), [ScreenCall::Volume { music: 0xE0, screen: 0x80 }]);
    assert_eq!(look.count, 63);
    assert_eq!(
        tick(&mut look, true),
        [ScreenCall::Volume { music: 0xC0, screen: 0xA0 }, ScreenCall::Sound(ScreenSound::Shade)]
    );
    assert_eq!(look.count, 0);
}

/// EXE4's hover (0x0801E478): a ramp of 11 ticks with a volume call every
/// other tick, the sound on the tick after the shade settles and every 61
/// after that, counted only while it stays (none while it ramps back).
#[test]
fn a_hover_that_sounds_while_dark() {
    use super::look::{Hover, ScreenCall, ScreenSound};
    use crate::content::{HoverRules, HoverRuns, HoverSound};
    let calls = |music: u16, screen: u16| Some([music, screen]);
    let gaps = |v: [Option<[u16; 2]>; 5]| -> Vec<Option<[u16; 2]>> { v.into_iter().flat_map(|c| [None, c]).chain([None]).collect() };
    let rules = HoverRules {
        runs: HoverRuns::WhileChoosing,
        to_dark: gaps([calls(0xE0, 0xE0), calls(0xC0, 0x100), calls(0xA0, 0x100), calls(0x80, 0xE0), calls(0x60, 0xC0)]),
        to_clear: gaps([calls(0xE0, 0xE0), calls(0x100, 0xC0), calls(0x100, 0xA0), calls(0xE0, 0x80), calls(0xC0, 0x60)]),
        players: [31, 9],
        sound: HoverSound::WhileDark { every: 61 },
    };
    let mut look = Hover::default();
    let tick = |look: &mut Hover, on_dark: Option<bool>| {
        let mut calls = Vec::new();
        look.hover(on_dark, &rules, &mut calls);
        calls
    };
    assert_eq!(tick(&mut look, Some(false)), []);
    // The turn, then 11 ticks of ramp, the volumes on every other.
    assert_eq!(tick(&mut look, Some(true)), []);
    let ramp: Vec<_> = (0..11).map(|_| tick(&mut look, Some(true))).collect();
    assert_eq!(ramp[0], []);
    assert_eq!(ramp[1], [ScreenCall::Volume { music: 0xE0, screen: 0xE0 }]);
    assert_eq!(ramp[9], [ScreenCall::Volume { music: 0x60, screen: 0xC0 }]);
    assert_eq!(ramp[10], []);
    // Settled: the sound at once, then every 61 ticks.
    assert_eq!(tick(&mut look, Some(true)), [ScreenCall::Sound(ScreenSound::Shade)]);
    for _ in 0..60 {
        assert_eq!(tick(&mut look, Some(true)), []);
    }
    assert_eq!(tick(&mut look, Some(true)), [ScreenCall::Sound(ScreenSound::Shade)]);
    // A tick whose state left the chips turns nothing.
    assert_eq!(tick(&mut look, None), []);
    assert_eq!(look.shade, super::ChipShade::Shaded);
    // Off it: the sound's counter waits again; back on it after the ramp,
    // the sound on the first settled tick.
    assert_eq!(tick(&mut look, Some(false)), []);
    assert!((0..11).all(|_| { tick(&mut look, Some(false)); true }));
    assert_eq!(look.shade, super::ChipShade::Clear);
    assert_eq!(tick(&mut look, Some(true)), []);
    for _ in 0..11 {
        assert!(!tick(&mut look, Some(true)).contains(&ScreenCall::Sound(ScreenSound::Shade)));
    }
    assert_eq!(tick(&mut look, Some(true)), [ScreenCall::Sound(ScreenSound::Shade)]);
}

// The round's custom screen time limit (nettai's own, not the original's:
// `RoundSetup::custom_screen_limit`).

/// A screen of SHOT A and SHOT B whose round has a time limit of `limit`
/// ticks, open and choosing.
fn timed(limit: Option<u16>) -> Player {
    let mut p = Player::new(&[(SHOT, 0), (SHOT, 1)]);
    p.time_limit = limit;
    p.open();
    p.wait(10);
    assert_eq!(p.up(), Up::Screen(Phase::Choosing));
    p
}

/// Ticks with nothing held until the tick before the screen's time is up,
/// each asking nothing; then that tick, with `held` held, which closes the
/// screen as OK does whatever the keys.
fn run_out(p: &mut Player, held: u16) {
    let left = p.side.time_left(p.time_limit).expect("the screen counts");
    for k in 1..left {
        assert_eq!(p.step(0), None, "{k} of {left}");
    }
    assert_eq!(p.step(held), Some(Request::Confirm));
    assert_eq!(p.phase(), Phase::Closing { tick: 0 });
    assert_eq!((p.side.clock, p.side.time_left(p.time_limit)), (p.time_limit.unwrap(), None));
}

/// What the side sends once its screen has closed.
fn sent(p: &mut Player) -> Option<crate::hand::ChipHand> {
    p.wait(20);
    p.side.sent.as_ref().expect("a result sent").result.hand.clone()
}

/// Time up while choosing: the screen closes as if OK were pressed, the
/// chips picked so far the hand; the keys of its last tick are ignored (B
/// would have taken the pick back). The clock counts from the tick the
/// window is in, the limit's ticks.
#[test]
fn time_up_confirms_the_chips_picked_so_far() {
    let mut p = timed(Some(120));
    assert_eq!(p.side.time_left(Some(120)), Some(120), "the window is in: nothing counted yet");
    p.press(keys::RIGHT);
    p.press(keys::A);
    assert_eq!(p.screen().selection(), [1]);
    assert_eq!(p.side.clock, 6);
    run_out(&mut p, keys::B);
    assert_eq!(p.screen().selection(), [1]);
    let hand = sent(&mut p).expect("a hand");
    assert_eq!(hand.selection[0], Some(FolderChip::new(ChipHandle(SHOT), ChipCode(1))));
    assert_eq!(p.side.folder.chips[1], None, "the pick left the folder");
    // Nothing picked: as OK with nothing, no hand.
    let mut p = timed(Some(30));
    run_out(&mut p, 0);
    assert_eq!(sent(&mut p), None);
}

/// Time up over the chips: SELECT's hiding, R's description and L's
/// message close silently, then OK is pressed with the picks.
#[test]
fn time_up_closes_what_is_over_the_chips_then_confirms() {
    for key in [keys::SELECT, keys::R, keys::L] {
        let mut p = timed(Some(60));
        p.lib.chips[0].1.description_lines = crate::content::Shown(2);
        p.press(keys::A);
        p.press(key);
        assert!(matches!(p.up(), Up::Hidden(_) | Up::Description | Up::RunMessage { .. }), "{key:#x}");
        run_out(&mut p, 0);
        assert_eq!(sent(&mut p).expect("a hand").selection[0], Some(FolderChip::new(ChipHandle(SHOT), ChipCode(0))), "{key:#x}");
    }
}

/// No limit: the screen counts nothing and waits for OK, as the original's.
#[test]
fn without_a_limit_the_screen_waits() {
    let mut p = timed(None);
    for _ in 0..20_000 {
        assert_eq!(p.step(0), None);
    }
    assert_eq!((p.phase(), p.side.clock, p.side.time_left(None)), (Phase::Choosing, 0, None));
}

/// In a battle both screens count in the simulation: with nobody pressing
/// anything both close on their last tick and the fight resumes, and a
/// battle restored from a snapshot taken while they count goes on to the
/// same state as the battle it was taken of.
#[test]
fn the_clock_is_part_of_the_battles_state() {
    use crate::battle::mode;
    use crate::{Battle, PlayerTick, TickEvents};
    let limit = 300;
    let mut b = Battle::new(crate::RoundSetup { custom_screen_limit: Some(limit), ..crate::scenario::setup() }, crate::scenario::content());
    let idle = [PlayerTick::buttons(0); 2];
    let choosing = |b: &Battle| b.custom.sides.iter().all(|s| s.screen.is_some_and(|s| s.phase == Phase::Choosing));
    for _ in 0..2000 {
        if choosing(&b) {
            break;
        }
        b.tick(&idle, TickEvents::default());
    }
    assert!(choosing(&b), "the screens never opened");
    for _ in 0..100 {
        b.tick(&idle, TickEvents::default());
    }
    let left = b.custom_time_left(0).expect("counting");
    assert_eq!(b.custom.sides[1].clock, b.custom.sides[0].clock);
    let snapshot = b.clone();
    let mut restored = snapshot.clone();
    for _ in 0..left + 200 {
        b.tick(&idle, TickEvents::default());
        restored.tick(&idle, TickEvents::default());
    }
    assert_eq!(restored.digest(), b.digest());
    assert_eq!(b.custom.sides.each_ref().map(|s| s.clock), [limit; 2]);
    assert_eq!(b.round.mode, mode::FIGHTING, "both results are in");
    let mut later = snapshot.clone();
    later.custom.sides[0].clock += 1;
    assert_ne!(snapshot.digest(), later.digest(), "the digest covers the clock");
}

/// The pointer (docs/design/custom-screen-actions.md §5): the pointer over a
/// slot moves the cursor with no choice; a click picks; a click on a pick takes it back,
/// the later ones moving up; a describe is R on what it is on, and a click
/// closes the description.
#[test]
fn the_pointer_moves_picks_takes_back_and_describes() {
    use super::layer::{Gesture, Target};
    let mut p = Player::new(&[(SHOT, 0), (SHOT, 1), (SHOT, 2), (WAVE, 0), (MEGA, 2)]);
    p.open();
    p.wait(10);
    p.step(0);
    assert_eq!(p.phase(), Phase::Choosing);
    // Over a slot: the cursor moves (its sound), nothing is chosen.
    p.layer.point(Gesture::Over(Target::Slot(2)));
    p.step(0);
    assert_eq!(p.layer.cursor, 2);
    assert!(p.sounds().contains(&look::ScreenSound::Cursor));
    assert!(p.screen().selection().is_empty());
    // Over a slot the cursor never goes to (none dealt there): nothing.
    p.layer.point(Gesture::Over(Target::Slot(7)));
    p.step(0);
    assert_eq!(p.layer.cursor, 2);
    // Clicks pick, the cursor going to each.
    for s in [0, 1, 2] {
        p.layer.point(Gesture::Click(Some(Target::Slot(s))));
        p.step(0);
        assert_eq!(p.layer.cursor, s);
    }
    assert_eq!(p.screen().selection(), [0, 1, 2]);
    // A click on the last pick takes it back.
    p.layer.point(Gesture::Click(Some(Target::Pick(2))));
    p.step(0);
    assert_eq!(p.screen().selection(), [0, 1]);
    // On the first: it alone, the second moving up, its icon with it.
    p.layer.point(Gesture::Click(Some(Target::Pick(0))));
    p.step(0);
    assert_eq!(p.screen().selection(), [1]);
    assert_eq!(p.screen().look.column.map(|c| c.map(|c| c.code)), [Some(ChipCode(1)), None, None, None, None]);
    p.step(0);
    assert_eq!(p.screen().selection(), [1]);
    p.layer.point(Gesture::Click(Some(Target::Pick(0))));
    p.step(0);
    assert!(p.screen().selection().is_empty());
    // A click past the picks does nothing.
    p.layer.point(Gesture::Click(Some(Target::Pick(0))));
    p.step(0);
    assert!(p.screen().selection().is_empty());
    // Right-click: R on the chip under the pointer; a click closes it.
    p.layer.point(Gesture::Describe(Target::Slot(4)));
    p.step(0);
    assert_eq!((p.up(), p.layer.cursor), (Up::Description, 4));
    let opened = (1..200).find(|_| {
        p.layer.point(Gesture::Click(None));
        p.step(0);
        p.up() != Up::Description
    });
    assert!(opened.is_some(), "a click closes the description");
    p.wait(4);
    assert_eq!(p.up(), Up::Screen(Phase::Choosing));
    // A click on OK sends the hand.
    p.layer.point(Gesture::Click(Some(Target::Slot(OK_SLOT))));
    p.step(0);
    assert_ne!(p.phase(), Phase::Choosing);
}

/// A right-click on a pick describes that pick's chip, wherever the cursor
/// is (the user: the pick's description, not the cursor's chip's).
#[test]
fn a_pick_described_is_its_own_chip() {
    use super::layer::{Gesture, Target};
    let mut p = Player::new(&[(SHOT, 0), (WAVE, 0), (MEGA, 0)]);
    p.open();
    p.wait(10);
    p.step(0);
    for s in [0, 1] {
        p.layer.point(Gesture::Click(Some(Target::Slot(s))));
        p.step(0);
    }
    assert_eq!((p.screen().selection(), p.layer.cursor), (&[0, 1][..], 1));
    p.layer.point(Gesture::Describe(Target::Pick(0)));
    p.step(0);
    assert_eq!(p.layer.cursor, 1, "the cursor stays");
    assert!(matches!(p.layer.sub, Sub::Description { chip: Some(c), from_list: false, .. } if c == ChipHandle(SHOT)), "{:?}", p.layer.sub);
}

/// A layer given another's ticks as a replay gives them (their buttons and
/// choices; no gestures, which the input doesn't carry) takes the same
/// choices, and its cursor goes to what each names as the pointer's did: a
/// chip's slot, the slot a pick taken back goes back to, OK.
#[test]
fn a_layer_given_the_ticks_follows_the_pointers_choices() {
    use super::layer::{Gesture, Target};
    let chips = [(SHOT, 0), (SHOT, 1), (SHOT, 2), (WAVE, 0), (MEGA, 2)];
    let (mut live, mut again) = (Player::new(&chips), Player::new(&chips));
    for p in [&mut live, &mut again] {
        p.open();
    }
    let script: [(u32, Option<Gesture>, u16); 16] = [
        (12, Some(Gesture::Over(Target::Slot(3))), 0),
        (13, Some(Gesture::Over(Target::Slot(1))), 0),
        (14, Some(Gesture::Click(Some(Target::Slot(4)))), 0),
        (16, None, keys::RIGHT),
        (20, Some(Gesture::Click(Some(Target::Slot(1)))), 0),
        (21, Some(Gesture::Over(Target::Pick(1))), 0),
        (22, Some(Gesture::Describe(Target::Pick(0))), 0),
        (40, Some(Gesture::Click(None)), 0),
        (58, Some(Gesture::Over(Target::Pick(0))), 0),
        (59, Some(Gesture::Away), 0),
        (60, Some(Gesture::Over(Target::Slot(2))), 0),
        (61, Some(Gesture::Click(Some(Target::Pick(0)))), 0),
        (70, Some(Gesture::Click(Some(Target::Slot(0)))), 0),
        (80, Some(Gesture::Over(Target::Slot(2))), 0),
        (90, None, keys::A),
        (110, Some(Gesture::Click(Some(Target::Slot(OK_SLOT)))), 0),
    ];
    let mut chosen = Vec::new();
    for tick in 0..130 {
        let mut held = 0;
        for &(_, gesture, keys) in script.iter().filter(|s| s.0 == tick) {
            if let Some(g) = gesture {
                live.layer.point(g);
            }
            held = keys;
        }
        let (_, given) = live.step_input(PlayerTick::buttons(held));
        again.step_replayed(given);
        assert_eq!(again.screen().selection(), live.screen().selection(), "tick {tick}");
        if let Some(c) = given.choice {
            chosen.push(c);
            assert_eq!(again.layer.cursor, live.layer.cursor, "tick {tick}: {c:?}");
        }
    }
    use ScreenChoice::{Ok, Pick, Unpick};
    assert_eq!(chosen, [Pick(4), Pick(1), Unpick(0), Pick(0), Pick(2), Ok]);
    assert_ne!(again.phase(), Phase::Choosing, "OK sent the hand");
}

/// One tick with this choice given (a replay's or a bot's, past the
/// layer's keys).
fn choose(p: &mut Player, choice: ScreenChoice) {
    p.step_input(PlayerTick::buttons(0).with_choice(Some(choice)));
}

/// The screen's availability done again, as the rules' `update_availability`
/// does after a trade or a hold.
fn redo_availability(p: &mut Player) {
    on_screen(p, |s, f, v| s.update_availability(v, f, &mut NoExtras));
}

/// The icons the column shows, by their chips' codes.
fn column_codes(p: &Player) -> [Option<u8>; 5] {
    p.screen().look.column.map(|c| c.map(|c| c.code.0))
}

/// `Unpick`: a pick anywhere taken back, the picks after it moving up a
/// place with their icons and what they carry (a button attached to it goes
/// with it, one attached to a later pick stays); the availability follows,
/// and OK builds the hand from what is left.
#[test]
fn a_pick_anywhere_is_taken_back_and_the_later_move_up() {
    let mut p = Player::new(&[(SHOT, 0), (SHOT, 1), (SHOT, 2), (WAVE, 0), (MEGA, 2)]);
    p.open();
    p.wait(10);
    p.step(0);
    choose(&mut p, ScreenChoice::Pick(0));
    choose(&mut p, ScreenChoice::Pick(1));
    assert!(on_screen(&mut p, |s, f, v| s.attach_to_last_pick(8, 0x04, f, v)));
    choose(&mut p, ScreenChoice::Pick(2));
    assert!(on_screen(&mut p, |s, f, v| s.attach_to_last_pick(9, 0x10, f, v)));
    assert_eq!(p.screen().selection(), [0, 1, 2]);
    assert_eq!(p.screen().slots[3].state, SlotState::Unavailable, "WAVE A goes with SHOT A, B and C by none");
    choose(&mut p, ScreenChoice::Unpick(1));
    let s = p.screen();
    assert_eq!(s.selection(), [0, 2]);
    assert_eq!(column_codes(&p), [Some(0), Some(2), None, None, None]);
    assert_eq!((s.slots[1].state, s.slots[1].marks, s.slots[1].attached, s.slots[8].state), (SlotState::Selectable, 0, None, SlotState::Selectable));
    assert_eq!((s.slots[2].marks, s.slots[2].attached, s.slots[9].state), (0x10, Some(9), SlotState::Selected));
    assert_eq!(p.sounds(), [look::ScreenSound::Back]);
    // The first: SHOT C alone is left, which MEGA C goes with and WAVE A
    // doesn't.
    choose(&mut p, ScreenChoice::Unpick(0));
    let s = p.screen();
    assert_eq!(s.selection(), [2]);
    assert_eq!(column_codes(&p), [Some(2), None, None, None, None]);
    assert_eq!((s.slots[0].state, s.slots[3].state, s.slots[4].state), (SlotState::Selectable, SlotState::Unavailable, SlotState::Selectable));
    // Past the picks: refused.
    choose(&mut p, ScreenChoice::Unpick(1));
    assert_eq!((p.screen().selection(), p.sounds()), (&[2u8][..], vec![look::ScreenSound::Refused]));
    choose(&mut p, ScreenChoice::Pick(4));
    let hand = confirm(&mut p);
    assert_eq!(hand.ids[..3], [Some(ChipHandle(SHOT)), Some(ChipHandle(MEGA)), None]);
    assert_eq!(hand.modifiers[..2], [0x10, 0]);
}

/// `Unpick` of the last pick is B: the same screen and sounds on every
/// tick, for a chip, a chip with a button attached, and a button picked in
/// a chip's place.
#[test]
fn unpicking_the_last_pick_is_b() {
    type Setup = fn(&mut Player);
    let setups: [(&str, Setup); 3] = [
        ("a chip", |p| {
            choose(p, ScreenChoice::Pick(0));
            choose(p, ScreenChoice::Pick(1));
        }),
        ("a chip with a button", |p| {
            choose(p, ScreenChoice::Pick(0));
            choose(p, ScreenChoice::Pick(1));
            assert!(on_screen(p, |s, f, v| s.attach_to_last_pick(8, 0x04, f, v)));
        }),
        ("a button in a chip's place", |p| {
            choose(p, ScreenChoice::Pick(1));
            assert!(on_screen(p, |s, f, v| s.trade_last_pick(8, f, v)));
            on_screen(p, |s, _, _| s.slots[8].state = SlotState::Selected);
            redo_availability(p);
        }),
    ];
    for (what, setup) in setups {
        let chips = [(SHOT, 0), (SHOT, 1), (SHOT, 2), (WAVE, 0), (MEGA, 2)];
        let (mut b, mut unpick) = (Player::new(&chips), Player::new(&chips));
        for p in [&mut b, &mut unpick] {
            p.open();
            p.wait(10);
            p.step(0);
            setup(p);
        }
        let last = b.screen().selected - 1;
        choose(&mut b, ScreenChoice::TakeBack);
        choose(&mut unpick, ScreenChoice::Unpick(last));
        for tick in 0..4 {
            assert_eq!(unpick.screen(), b.screen(), "{what}, tick {tick}");
            assert_eq!(unpick.sounds(), b.sounds(), "{what}, tick {tick}");
            b.step(0);
            unpick.step(0);
        }
    }
}

/// A button picked in a chip's place (`trade_last_pick`: EXE5's and EXE4's
/// souls) taken back from before later picks: the chip given up for it
/// back in its place where it goes with the picks left; where it doesn't,
/// back to the hand, with the button.
#[test]
fn a_traded_pick_gives_its_chip_back_where_it_goes_with_the_others() {
    for (later, back) in [(2, true), (3, false)] {
        let mut p = Player::new(&[(SHOT, 0), (SHOT, 1), (SHOT, 2), (WAVE, 0), (MEGA, 2)]);
        p.open();
        p.wait(10);
        p.step(0);
        choose(&mut p, ScreenChoice::Pick(1));
        choose(&mut p, ScreenChoice::Pick(0));
        // SHOT A given up for the button: SHOT B is left, which SHOT C
        // goes with (the same chip) and WAVE B (the same code).
        assert!(on_screen(&mut p, |s, f, v| s.trade_last_pick(8, f, v)));
        on_screen(&mut p, |s, _, _| s.slots[8].state = SlotState::Selected);
        redo_availability(&mut p);
        assert_eq!(p.screen().selection(), [8, 1]);
        if !back {
            // (Slot 3's chip made WAVE B, which goes with SHOT B by its code.)
            p.side.folder.chips[3] = Some(FolderChip::new(ChipHandle(WAVE), ChipCode(1)));
            redo_availability(&mut p);
        }
        choose(&mut p, ScreenChoice::Pick(later));
        assert_eq!(p.screen().selection(), [8, 1, later]);
        choose(&mut p, ScreenChoice::Unpick(0));
        let s = p.screen();
        assert_eq!((s.trade, s.slots[8].state), (None, SlotState::Selectable), "later {later}");
        if back {
            // SHOT A goes with SHOT B and SHOT C.
            assert_eq!(s.selection(), [0, 1, later]);
            assert_eq!(s.slots[0].state, SlotState::Selected);
            assert_eq!(column_codes(&p), [Some(0), Some(1), Some(2), None, None]);
        } else {
            // SHOT A goes with SHOT B but not WAVE B: both leave.
            assert_eq!(s.selection(), [1, later]);
            assert_eq!(s.slots[0].state, SlotState::Unavailable, "back in the hand, not with SHOT B and WAVE B");
            assert_eq!(column_codes(&p), [Some(1), Some(1), None, None, None]);
        }
        assert_eq!(p.sounds(), [look::ScreenSound::Back]);
        let hand = confirm(&mut p);
        let picked = if back { 3 } else { 2 };
        assert!(hand.ids[..picked].iter().all(Option::is_some) && hand.ids[picked].is_none(), "later {later}: {:?}", hand.ids);
    }
}

/// A button's held chip (`hold_last_pick`: EXE5's Arm Change) is no pick:
/// `Unpick` never puts it back (where B, with the picks as they were, would
/// first), and a pick before its place moves the place down, so that B
/// still puts it back once the picks after it are gone. `Release` puts it
/// back in its place where it goes with the picks, else back to the hand.
#[test]
fn a_held_chip_stays_held_through_unpicks_until_released() {
    let mut p = Player::new(&[(SHOT, 0), (WAVE, 0), (SHOT, 1), (MEGA, 0), (MEGA, 1)]);
    // (A button two wide on slots 8 and 9 to hold it: DustCross's.)
    p.stats.form = library::testing::DUST_CROSS;
    p.open();
    p.wait(10);
    p.step(0);
    choose(&mut p, ScreenChoice::Pick(0));
    choose(&mut p, ScreenChoice::Pick(1));
    assert!(on_screen(&mut p, |s, f, v| s.hold_last_pick(8, f, v)));
    redo_availability(&mut p);
    assert_eq!(p.screen().hold.map(|h| h.at), Some(1));
    // The picks as they were: `Unpick` of the last takes SHOT A, not the
    // held chip; the place moves down.
    choose(&mut p, ScreenChoice::Unpick(0));
    let s = p.screen();
    assert_eq!((s.selection(), s.hold.map(|h| (h.chip, h.at))), (&[][..], Some((1, 0))));
    // B puts it back, the picks being as they were.
    choose(&mut p, ScreenChoice::TakeBack);
    assert_eq!((p.screen().selection(), p.screen().hold), (&[1u8][..], None));
    // Held again (WAVE A), then SHOT B (code B, which WAVE A doesn't go
    // with): released, it goes back to the hand.
    assert!(on_screen(&mut p, |s, f, v| s.hold_last_pick(8, f, v)));
    on_screen(&mut p, |s, _, _| s.look.column_kept = None);
    redo_availability(&mut p);
    choose(&mut p, ScreenChoice::Pick(2));
    assert_eq!(p.screen().selection(), [2]);
    choose(&mut p, ScreenChoice::Release(9));
    let s = p.screen();
    assert_eq!((s.selection(), s.hold, s.slots[1].state, s.slots[8].state), (&[2u8][..], None, SlotState::Unavailable, SlotState::Selectable));
    assert_eq!(p.sounds(), [look::ScreenSound::Back]);
    // Held again after MEGA A, then MEGA B taken back and SHOT A picked
    // (code A): released, it goes back in the place it left.
    choose(&mut p, ScreenChoice::Unpick(0));
    choose(&mut p, ScreenChoice::Pick(3));
    choose(&mut p, ScreenChoice::Pick(1));
    assert!(on_screen(&mut p, |s, f, v| s.hold_last_pick(8, f, v)));
    redo_availability(&mut p);
    choose(&mut p, ScreenChoice::Pick(0));
    assert_eq!(p.screen().selection(), [3, 0]);
    choose(&mut p, ScreenChoice::Release(8));
    let s = p.screen();
    assert_eq!((s.selection(), s.hold, s.slots[8].state), (&[3u8, 1, 0][..], None, SlotState::Selectable));
    assert_eq!(column_codes(&p), [Some(0), Some(0), Some(0), None, None]);
    // A button holding nothing: refused.
    choose(&mut p, ScreenChoice::Release(8));
    assert_eq!(p.sounds(), [look::ScreenSound::Refused]);
    let hand = confirm(&mut p);
    assert_eq!(hand.ids[..4], [Some(ChipHandle(MEGA)), Some(ChipHandle(WAVE)), Some(ChipHandle(SHOT)), None]);
}

/// `Unchoose` where the rules un-choose nothing: refused, the picks kept.
#[test]
fn unchoosing_nothing_chosen_is_refused() {
    let mut p = Player::new(&[(SHOT, 0), (SHOT, 1)]);
    p.open();
    p.wait(10);
    p.step(0);
    choose(&mut p, ScreenChoice::Pick(0));
    choose(&mut p, ScreenChoice::Unchoose);
    assert_eq!((p.screen().selection(), p.sounds()), (&[0u8][..], vec![look::ScreenSound::Refused]));
}

/// The cursor on a pick (the user: "when mousing over the stack of chips you
/// should show the cursor there too"): the pointer over a pick puts it there, its
/// slot kept, the chip window showing the pick's chip; a direction brings it
/// back to its slot and moves it no further; A takes the pick back, the
/// cursor to the slot it goes back to (the user: "when deleting a chip from
/// the tray, you should reposition the cursor to where the chip returns");
/// R describes it; B taking the pick from under it leaves it on its slot; the
/// pointer over a slot or another target, or away, brings it back.
#[test]
fn the_cursor_goes_on_a_pick_by_pointer_only() {
    use super::layer::{Gesture, Target};
    let mut p = Player::new(&[(SHOT, 0), (WAVE, 0), (SHOT, 1), (MEGA, 0), (MEGA, 1)]);
    p.open();
    p.wait(10);
    p.step(0);
    choose(&mut p, ScreenChoice::Pick(0));
    choose(&mut p, ScreenChoice::Pick(1));
    p.layer.point(Gesture::Over(Target::Slot(3)));
    p.step(0);
    assert_eq!((p.layer.cursor, p.layer.pick_cursor), (3, None));
    // On the second pick: the cursor there, the slot kept, WAVE in the
    // chip window, the cursor's sound.
    p.layer.point(Gesture::Over(Target::Pick(1)));
    p.step(0);
    assert_eq!((p.layer.cursor, p.layer.pick_cursor), (3, Some(1)));
    assert_eq!((p.layer.chip_window.slot, p.layer.chip_window.last_chip.map(|c| c.id)), (1, Some(ChipHandle(WAVE))));
    assert!(p.sounds().contains(&look::ScreenSound::Cursor));
    // A direction: back to slot 3, no further.
    p.press(keys::RIGHT);
    assert_eq!((p.layer.cursor, p.layer.pick_cursor), (3, None));
    assert_eq!(p.layer.chip_window.slot, 3);
    // Keys never go into the column: the next RIGHT moves on the slots.
    p.press(keys::RIGHT);
    assert_eq!((p.layer.cursor, p.layer.pick_cursor), (4, None));
    // R on a pick describes its chip; A takes it back.
    p.layer.point(Gesture::Over(Target::Pick(0)));
    p.step(0);
    assert_eq!(p.layer.pick_cursor, Some(0));
    p.press(keys::R);
    assert!(matches!(p.layer.sub, Sub::Description { chip: Some(c), .. } if c == ChipHandle(SHOT)), "{:?}", p.layer.sub);
    let closed = (1..200).find(|_| {
        p.step(keys::B);
        p.step(0);
        p.up() == Up::Screen(Phase::Choosing)
    });
    assert!(closed.is_some());
    p.wait(4);
    assert_eq!(p.layer.pick_cursor, Some(0));
    let (_, given) = p.step_input(PlayerTick::buttons(keys::A));
    assert_eq!(given.choice, Some(ScreenChoice::Unpick(0)));
    p.step(0);
    // WAVE moved up; the cursor on SHOT's slot, which it went back to.
    assert_eq!((p.screen().selection(), p.layer.pick_cursor, p.layer.cursor), (&[1u8][..], None, 0));
    assert_eq!((p.layer.chip_window.slot, p.layer.chip_window.last_chip.map(|c| c.id)), (0, Some(ChipHandle(SHOT))));
    // On WAVE's pick, B is the game's B: the pick gone from under it, the
    // cursor back on its slot.
    p.layer.point(Gesture::Over(Target::Pick(0)));
    p.step(0);
    assert_eq!(p.layer.pick_cursor, Some(0));
    let (_, given) = p.step_input(PlayerTick::buttons(keys::B));
    assert_eq!(given.choice, Some(ScreenChoice::TakeBack));
    p.step(0);
    assert_eq!((p.screen().selection(), p.layer.pick_cursor, p.layer.cursor), (&[][..], None, 0));
    // The pointer over another target or away: on nothing, silently, its
    // slot kept; over a slot: there.
    choose(&mut p, ScreenChoice::Pick(0));
    for (off, cursor, away) in [(Gesture::Over(Target::FormTab), 0, true), (Gesture::Away, 0, true), (Gesture::Over(Target::Slot(2)), 2, false)] {
        p.layer.point(Gesture::Over(Target::Pick(0)));
        p.step(0);
        assert_eq!((p.layer.pick_cursor, p.layer.away), (Some(0), false));
        p.layer.point(off);
        p.step(0);
        assert_eq!((p.layer.pick_cursor, p.layer.cursor, p.layer.away), (None, cursor, away), "{off:?}");
        assert_eq!(p.sounds().contains(&look::ScreenSound::Cursor), !away, "{off:?}");
    }
}

/// The pointer off the slots and picks puts the cursor on nothing: none
/// drawn, nor the element icon (EXE4's, with a chip shown), the chip window
/// empty, the slot kept, through the next screen's opening; over a slot, or
/// a direction, A or R, brings it back (the key doing no more), START to OK.
#[test]
fn the_pointer_away_leaves_the_cursor_on_nothing() {
    use super::layer::{Gesture, Target};
    let mut p = Player::new(&[(SHOT, 0), (SHOT, 1), (SHOT, 2), (WAVE, 0), (MEGA, 2)]);
    p.open();
    p.wait(10);
    p.step(0);
    p.layer.point(Gesture::Over(Target::Slot(1)));
    p.step(0);
    assert!(p.layer.look.drawn.cursor.is_some() && p.layer.look.drawn.element);
    assert_eq!((p.layer.chip_window.slot, p.layer.chip_window.empty), (1, false));
    p.layer.point(Gesture::Away);
    p.step(0);
    assert_eq!((p.layer.cursor, p.layer.away, p.layer.chip_window.empty), (1, true, true));
    assert_eq!((p.layer.look.drawn.cursor, p.layer.look.drawn.element), (None, false));
    assert!(!p.sounds().contains(&look::ScreenSound::Cursor));
    p.wait(4);
    assert_eq!(p.layer.look.drawn.cursor, None);
    // A direction, A, R: back on slot 1, nothing else.
    for key in [keys::RIGHT, keys::A, keys::R] {
        p.layer.point(Gesture::Away);
        p.step(0);
        if key == keys::RIGHT {
            // (A direction moves from its hold's second tick.)
            p.step(key);
        }
        let (_, given) = p.step_input(PlayerTick::buttons(key));
        assert_eq!((given.choice, p.layer.cursor, p.layer.away, p.layer.sub), (None, 1, false, Sub::None), "{key:#x}");
        assert!(p.sounds().contains(&look::ScreenSound::Cursor), "{key:#x}");
        assert!(p.layer.look.drawn.cursor.is_some() && p.layer.look.drawn.element, "{key:#x}");
        assert!(!p.layer.chip_window.empty, "{key:#x}");
        p.step(0);
        assert!(p.screen().selection().is_empty());
    }
    // B is the game's, away or not.
    choose(&mut p, ScreenChoice::Pick(0));
    p.layer.point(Gesture::Away);
    p.step(0);
    let (_, given) = p.step_input(PlayerTick::buttons(keys::B));
    assert_eq!((given.choice, p.layer.away), (Some(ScreenChoice::TakeBack), true));
    p.step(0);
    // START: to OK.
    p.step_input(PlayerTick::buttons(keys::START));
    assert_eq!((p.layer.cursor, p.layer.away), (OK_SLOT, false));
    p.step(0);
    // Away as the next screen opens: on nothing there too.
    p.layer.point(Gesture::Away);
    p.step(0);
    p.open();
    p.wait(10);
    p.step(0);
    assert_eq!((p.layer.cursor, p.layer.away, p.layer.look.drawn.cursor, p.layer.chip_window.empty), (0, true, None, true));
    // Over a slot: there.
    p.layer.point(Gesture::Over(Target::Slot(2)));
    p.step(0);
    assert_eq!((p.layer.cursor, p.layer.away), (2, false));
    assert!(p.layer.look.drawn.cursor.is_some());
}

/// The pointer on a button holding a chip releases it (no key does: A on it
/// is refused, as the game's); on a pick, `Unpick`.
#[test]
fn the_pointer_releases_a_held_chip_and_unpicks() {
    use super::layer::{Gesture, Target};
    let mut p = Player::new(&[(SHOT, 0), (WAVE, 0), (SHOT, 1), (MEGA, 0), (MEGA, 1)]);
    p.stats.form = library::testing::DUST_CROSS;
    p.open();
    p.wait(10);
    p.step(0);
    choose(&mut p, ScreenChoice::Pick(0));
    choose(&mut p, ScreenChoice::Pick(1));
    assert!(on_screen(&mut p, |s, f, v| s.hold_last_pick(8, f, v)));
    redo_availability(&mut p);
    p.layer.point(Gesture::Over(Target::Slot(4)));
    p.step(0);
    assert_eq!(p.layer.cursor, 4);
    p.layer.point(Gesture::Click(Some(Target::Pick(0))));
    let (_, given) = p.step_input(PlayerTick::buttons(0));
    assert_eq!(given.choice, Some(ScreenChoice::Unpick(0)));
    // (The cursor to the slot the pick goes back to.)
    assert_eq!((p.layer.cursor, p.layer.pick_cursor), (0, None));
    p.layer.point(Gesture::Click(Some(Target::Slot(9))));
    let (_, given) = p.step_input(PlayerTick::buttons(0));
    assert_eq!(given.choice, Some(ScreenChoice::Release(9)));
    assert_eq!((p.screen().selection(), p.screen().hold), (&[1u8][..], None));
    // Not holding: a click on it is A on it.
    p.layer.point(Gesture::Click(Some(Target::Slot(8))));
    let (_, given) = p.step_input(PlayerTick::buttons(0));
    assert_eq!(given.choice, Some(ScreenChoice::Press(8)));
}
