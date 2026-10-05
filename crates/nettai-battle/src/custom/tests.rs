//! One player's custom screen driven by scripted buttons, with made-up
//! chips.

use super::library::testing::{EVERY_CODE, TestLibrary, chip};
use super::screen::{OK_SLOT, SPECIAL_SLOT};
use super::*;
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
/// A chip with the dark flag (no BN6 chip has it), in every code.
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
    NaviStats { custom_level: 5, mega_level: 1, giga_level: 1, mood: 0x80, beast_out_counter: 3, ..NaviStats::default() }
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

/// BN6's scrap and re-deal buttons (its cross and navicust systems', in
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

    fn button_taken_back(&mut self, _: &mut Screen, _: ButtonHandle) {}

    fn dealing(&mut self, _: &mut Screen, _: &mut BattleFolder, _: &mut crate::console::Console) {}
    fn opened(&mut self, _: &mut Screen) {}

    fn confirmed(&mut self, _: &mut Screen, _: &mut BattleFolder) {}

    fn chip_picked(&mut self, _: &mut Screen, _: &mut BattleFolder, _: ChipHandle) {}

    fn chip_taken_back(&mut self, _: &mut Screen, _: ChipHandle) {}

    fn window_update(&mut self, _: &mut Screen, _: &mut BattleFolder, _: &mut Console, _: &Joypad, _: crate::content::WindowHandle) -> bool {
        false
    }

    fn keys(&mut self, _: &mut Screen, _: &mut BattleFolder, _: &Joypad) -> bool {
        false
    }

    fn take_back(&mut self, _: &mut Screen, _: &BattleFolder) -> bool {
        false
    }

    fn button_pressed(&mut self, screen: &mut Screen, _: &mut BattleFolder, b: ButtonHandle) {
        let slot = screen.cursor_button_slot().expect("a button under the cursor");
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

/// A player's side with this folder, and what their screen reads.
struct Player {
    side: Side,
    console: Console,
    lib: TestLibrary,
    stats: NaviStats,
    tick: u32,
}

impl Player {
    fn new(chips: &[(ChipId, u8)]) -> Player {
        let setup = PlayerSetup { folder: Some(folder(chips)), ..PlayerSetup::default() };
        Player { side: Side::new(&setup), console: Console::new(&setup.console), lib: library(), stats: stats(), tick: 0 }
    }

    fn context(&self) -> Context<'_> {
        Context {
            library: &self.lib,
            stats: self.stats,
            emotion: Emotion::Normal,
            turn: 1,
            own_gauges: false,
            random_battle: false,
            late_turns: false,
            now: self.tick,
            link_delay: 4,
        }
    }

    /// BN6's buttons as its systems would show them.
    fn buttons(&self) -> TestButtons {
        let megaman = self.lib.changes_form(self.stats.navi);
        TestButtons {
            // (DustCross and DustCross Beast: BN6's cross system's
            // `scrap_button`.)
            scrap: megaman && matches!(self.stats.form.0, 0x0A | 0x16),
            redeal: megaman && self.stats.chip_shuffle,
        }
    }

    fn open(&mut self) {
        let ctx = self.context();
        let mut side = self.side.clone();
        let mut console = self.console;
        side.open_with(&ctx, &mut console, &mut self.buttons());
        self.side = side;
        self.console = console;
    }

    /// One tick with these buttons held.
    fn step(&mut self, held: u16) -> Option<Request> {
        self.tick += 1;
        self.side.joypad.update(held);
        let ctx = self.context();
        let mut side = self.side.clone();
        let mut console = self.console;
        let r = side.tick_with(&ctx, &mut console, |id| self.lib.chip(id).damage, &mut self.buttons());
        self.side = side;
        self.console = console;
        r
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

#[test]
fn five_chips_are_dealt_into_the_top_row() {
    let mut p = Player::new(&[]);
    p.open();
    let s = p.screen();
    assert_eq!(s.hand_size, 5);
    assert_eq!(s.cursor, 0);
    for i in 0..5 {
        assert_eq!(s.slots[i].kind, SlotKind::Chip { index: i as u8, regular: false });
        assert_eq!(s.slots[i].vertical, None);
    }
    assert!(s.slots[5..10].iter().all(|x| matches!(x.kind, SlotKind::Empty | SlotKind::Hidden)));
    // Left of the first chip wraps to OK; OK's right to the first chip.
    assert_eq!((s.slots[0].left, s.slots[4].right), (Some(OK_SLOT), Some(OK_SLOT)));
    // (OK's up goes to the special slot when a system's button is there:
    // BN6's Beast Out, which these tests have none of.)
    let ok = s.slots[OK_SLOT as usize];
    assert_eq!((ok.vertical, ok.left, ok.right), (None, Some(4), Some(0)));
    // (The special slot holds a system's button, BN6's Beast Out, which
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
    assert_eq!(p.phase(), Phase::Choosing);
    p.step(0);
    p.press(keys::RIGHT);
    assert_eq!(p.screen().cursor, 1);
    p.press(keys::A);
    assert_eq!(p.screen().selection(), [1]);
    p.press(keys::START);
    assert_eq!(p.screen().cursor, OK_SLOT);
    // OK: the status bit clears on the next tick, the hand goes out 11
    // ticks after OK and arrives 50 + 4 ticks after that.
    let ok_tick = p.tick + 1;
    assert_eq!(p.step(keys::A), Some(Request::Confirm));
    assert!(p.side.in_custom);
    p.step(0);
    assert!(!p.side.in_custom);
    let waited = p.wait(20);
    assert_eq!(p.tick, ok_tick + 11, "sent after {waited}");
    let sent = p.side.sent.as_ref().unwrap();
    assert_eq!(sent.arrives, ok_tick + 11 + 54);
    let hand = sent.result.hand.as_ref().unwrap();
    assert_eq!(hand.ids[..2], [Some(ChipHandle(SHOT)), None]);
    assert_eq!(hand.selection[0], Some(FolderChip::new(ChipHandle(SHOT), ChipCode(1))));
    assert_eq!(hand.damage[0], 40);
    // The picked chip left the folder.
    assert_eq!(p.side.folder.unwrap().chips[1], None);
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
    assert_eq!(p.screen().cursor, 8);
    let a = p.tick + 1;
    p.step(keys::A);
    while p.phase() != Phase::Choosing && p.tick < 1000 {
        p.step(0);
    }
    // 25 ticks a chip, and 3 more.
    assert_eq!(p.tick, a + 2 * 25 + 3);
    assert_eq!(p.screen().selected, 0);
    assert_eq!(p.screen().slots[8].state, SlotState::Selected);
    // The scrapped chips went to the end of the folder, in pick order, and
    // the hand is dealt again from the front.
    let f = p.side.folder.unwrap();
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
    p.side.folder = Some(f);
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
    assert_eq!(p.screen().cursor, 8);
    p.step(keys::A);
    while p.phase() != Phase::Choosing && p.tick < 1000 {
        p.step(0);
    }
    assert!(!p.side.folder.unwrap().regular_pending);
    assert!(matches!(p.screen().slots[0].kind, SlotKind::Chip { regular: true, .. }));
}

#[test]
fn hand_size() {
    let size = |custom_level: u8, shrink: u8, turn: u8, number_open: bool| {
        let mut p = Player::new(&[]);
        p.stats.custom_level = custom_level;
        p.stats.bugs.hand_shrink_turn = shrink;
        p.stats.number_open = number_open;
        let ctx = Context { turn, ..p.context() };
        let mut side = p.side.clone();
        side.open(&ctx, &mut p.console.clone());
        side.screen.unwrap().hand_size
    };
    assert_eq!(size(5, 0, 1, false), 5);
    assert_eq!(size(10, 0, 1, false), 8);
    assert_eq!(size(6, 2, 2, false), 5);
    assert_eq!(size(6, 2, 5, false), 2);
    assert_eq!(size(5, 0, 1, true), 10);
}

#[test]
fn select_hides_the_window_until_a_key() {
    let mut p = Player::new(&[]);
    p.open();
    p.wait(10);
    p.step(0);
    p.press(keys::SELECT);
    assert!(matches!(p.phase(), Phase::Hidden { .. }));
    p.step(0);
    // The key that brings the window back does nothing else, even as it
    // starts to repeat.
    p.step(keys::RIGHT);
    p.step(keys::RIGHT);
    assert_eq!(p.phase(), Phase::Choosing);
    p.step(keys::RIGHT);
    assert_eq!(p.screen().cursor, 0);
}

#[test]
fn chip_shuffle_redeals_what_is_not_picked() {
    // Thirty different chips (ids and codes), the first one the pick.
    let chips: Vec<(ChipId, u8)> = (0..30).map(|i| ([SHOT, WAVE][i % 2], (i / 2) as u8 % 3)).collect();
    let mut p = Player::new(&chips);
    p.stats.chip_shuffle = true;
    p.console = Console::new(&ConsoleSetup { rng: 0x1234_5678, ..ConsoleSetup::default() });
    p.open();
    assert!(matches!(p.screen().slots[8].kind, SlotKind::Button { button: ButtonHandle(1), cell: ButtonCell::Left }));
    p.wait(10);
    p.step(0);
    p.press(keys::A);
    let before = p.side.folder.unwrap();
    // Down from the fourth chip to the re-deal button.
    p.press(keys::RIGHT);
    p.press(keys::RIGHT);
    p.press(keys::RIGHT);
    p.press(keys::DOWN);
    assert_eq!(p.screen().cursor, 8);
    let rng = p.console.rng;
    let a = p.tick + 1;
    p.step(keys::A);
    assert!(matches!(p.phase(), Phase::Redealing { .. }));
    while p.phase() != Phase::Choosing && p.tick < 1000 {
        p.step(0);
    }
    // The first tick, then 32.
    assert_eq!(p.tick, a + 33);
    // The pick stays; the other 29 are shuffled once from the RNG as it
    // was (29 swaps, two draws each), which the 7 shows in between (the
    // same again each) don't change.
    let after = p.side.folder.unwrap();
    assert_eq!(after.chips[0], before.chips[0]);
    let mut expected = before.chips[1..].to_vec();
    let mut r = rng;
    super::folder::shuffle(&mut expected, 29, &mut r);
    assert_eq!(after.chips[1..], expected[..]);
    for _ in 0..7 {
        super::folder::shuffle(&mut expected, 29, &mut r);
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
    p.stats.chip_shuffle = true;
    let mut f = folder(&chips);
    f.regular_pending = true;
    p.side.folder = Some(f);
    p.console = Console::new(&ConsoleSetup { rng: 0x0BAD_F00D, tag_pair: Some(12), ..ConsoleSetup::default() });
    p.open();
    assert!(matches!(p.screen().slots[0].kind, SlotKind::Chip { regular: true, .. }));
    p.wait(10);
    p.step(0);
    let before = p.side.folder.unwrap();
    p.press(keys::RIGHT);
    p.press(keys::RIGHT);
    p.press(keys::RIGHT);
    p.press(keys::DOWN);
    p.step(keys::A);
    while p.phase() != Phase::Choosing && p.tick < 1000 {
        p.step(0);
    }
    let after = p.side.folder.unwrap();
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
    p.stats.chip_shuffle = true;
    p.console = Console::new(&ConsoleSetup { rng: 0x0BAD_F00D, tag_pair: Some(12), ..ConsoleSetup::default() });
    p.open();
    p.wait(10);
    p.step(0);
    let pair = p.side.folder.unwrap().chips[12..14].to_vec();
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
    let before = p.side.folder.unwrap();
    assert_eq!(before.chips[10..12], pair[..]);
    p.press(keys::RIGHT);
    p.press(keys::RIGHT);
    p.press(keys::RIGHT);
    p.press(keys::DOWN);
    assert_eq!(p.screen().cursor, 8);
    p.step(keys::A);
    while p.phase() != Phase::Choosing && p.tick < 1000 {
        p.step(0);
    }
    let after = p.side.folder.unwrap();
    assert_eq!(after.chips[10..12], pair[..]);
    assert_ne!(after.chips, before.chips);
}

/// A screen whose first chip (SHOT A) has a description of `lines` lines,
/// taking keys.
fn describing(lines: usize) -> Player {
    let mut p = Player::new(&[(SHOT, 0), (SHOT, 1)]);
    p.lib.chips[0].1.description_lines = lines as u8;
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
        assert!(matches!(p.phase(), Phase::Description { .. }));
        for _ in 1..first - 1 {
            p.step(0);
        }
        p.step(keys::A);
        for _ in 0..30 {
            p.step(0);
        }
        assert!(matches!(p.phase(), Phase::Description { .. }), "{lines} lines: A on tick {} closed it", first - 1);
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
            assert!(matches!(p.phase(), Phase::Description { .. }));
        }
        p.step(keys::A);
        assert_eq!(p.phase(), Phase::Choosing, "{lines} lines");
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
    assert!(matches!(p.phase(), Phase::Description { .. }));
    let mut p = describing(3);
    p.step(keys::R);
    for _ in 1..=18 {
        p.step(keys::B);
    }
    for _ in 0..4 {
        assert!(matches!(p.phase(), Phase::Description { .. }));
        p.step(0);
    }
    p.step(0);
    assert_eq!(p.phase(), Phase::Choosing);
}

#[test]
fn the_no_running_message_starts_a_tick_after_l() {
    let mut p = describing(3);
    p.step(keys::L);
    assert_eq!(p.phase(), Phase::RunMessage { chatbox: None });
    // Its box takes A or B from its 80th tick (the 80th after L): a press
    // a tick before is lost.
    for _ in 1..79 {
        p.step(0);
    }
    p.step(keys::A);
    for _ in 0..20 {
        p.step(0);
    }
    assert!(matches!(p.phase(), Phase::RunMessage { .. }));
    p.step(keys::A);
    // Closed seven ticks later; the screen sees it the tick after, and
    // takes keys the tick after that.
    for _ in 0..8 {
        assert!(matches!(p.phase(), Phase::RunMessage { .. }));
        p.step(0);
    }
    assert_eq!(p.phase(), Phase::Choosing);
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
    assert_eq!(p.screen().cursor, 1);
    // The hover starts on the tick the window is in (the chips' state).
    for _ in 0..10 {
        p.step(0);
    }
    assert_eq!(p.phase(), Phase::Choosing);
    let look = p.screen().look;
    assert_eq!(look.dark, DarkHover::Darkening { step: 0 });
    assert_eq!((look.fade.mode, look.window_fade.mode), (FadeMode::DarkChip, FadeMode::DarkChipWindow));
    // The music turns down and the screen's player up, a step a tick, until
    // the window's fade is done.
    let volumes = |p: &mut Player, n: usize| {
        (0..n)
            .map(|_| {
                p.step(0);
                p.screen().look.drawn.volume
            })
            .collect::<Vec<_>>()
    };
    let down = volumes(&mut p, 6);
    assert_eq!(down, [Some((0x100, 0x80)), Some((0xE0, 0x80)), Some((0xC0, 0xA0)), Some((0xA0, 0xC0)), Some((0x80, 0xE0)), None]);
    let look = p.screen().look;
    assert_eq!(look.dark, DarkHover::Dark);
    assert_eq!(look.window_fade.level, 0x30);
    // (The screen's own fade, eight steps to 0x50, has one to go.)
    assert_eq!((look.fade.level, look.fade.active()), (0x46, true));
    // The cursor leaves it (on the second tick of the hold): back the other
    // way, a step longer.
    p.step(keys::RIGHT);
    p.step(keys::RIGHT);
    assert_eq!(p.screen().cursor, 2);
    assert_eq!(p.screen().look.dark, DarkHover::Clearing { step: 0 });
    let up = volumes(&mut p, 7);
    assert_eq!(
        up,
        [Some((0x80, 0x100)), Some((0x80, 0xE0)), Some((0xA0, 0xC0)), Some((0xC0, 0xA0)), Some((0xE0, 0x80)), Some((0x100, 0x80)), None]
    );
    let look = p.screen().look;
    assert_eq!((look.dark, look.window_fade.level), (DarkHover::Clear, 0));
    // The screen's own fade, back from 0x50, clears a tick later.
    assert_eq!(p.screen().look.fade.level, 0xA);
    p.step(0);
    assert_eq!((p.screen().look.fade.level, p.screen().look.fade.active()), (0, false));
}

#[test]
fn the_cursor_stays_put_without_a_dark_chip() {
    let mut p = Player::new(&[(SHOT, 0), (MEGA, 0)]);
    p.open();
    assert_eq!(p.screen().cursor, 0);
    for _ in 0..20 {
        p.step(0);
        assert_eq!((p.screen().look.dark, p.screen().look.drawn.volume), (DarkHover::Clear, None));
    }
}

/// Run one of the screen's routines a system's content calls (`custom.*`),
/// with the side's folder and view.
fn on_screen<R>(p: &mut Player, f: impl FnOnce(&mut Screen, &BattleFolder, &super::screen::PlayerView) -> R) -> R {
    let side = p.side.clone();
    let folder = side.folder.expect("a folder");
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

/// `custom.attach_to_last_pick` (BN5's capsules): the last pick carries
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

/// `custom.hold_last_pick` (BN5's Arm Change): the last pick leaves the
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
    assert!(p.screen().look.drawn.held);
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
    assert!(!s.look.drawn.held);
    // Held again, and OK: the first pick alone is the hand; both chips
    // left the folder.
    assert!(on_screen(&mut p, |s, f, v| s.hold_last_pick(8, f, v)));
    let hand = confirm(&mut p);
    assert_eq!(hand.ids[..2], [Some(ChipHandle(SHOT)), None]);
    let f = p.side.folder.unwrap();
    assert_eq!((f.chips[0], f.chips[1]), (None, None));
    assert_eq!(f.chips[2], Some(FolderChip::new(ChipHandle(SHOT), ChipCode(0))));
}

/// A button that shows a chip (`ButtonPlace::chip`, BN5's capsules): the
/// chip window keeps the frame of the last chip slot it showed (OK and a
/// button's picture set the standard one), and R describes the chip.
#[test]
fn a_buttons_chip_keeps_the_chip_windows_frame_and_is_described() {
    let mut p = Player::new(&[(MEGA, 0), (SHOT, 0)]);
    p.open();
    p.wait(10);
    p.step(0);
    let framed = |p: &Player| p.screen().look.chip_window.framed.map(|c| c.id);
    assert_eq!(framed(&p), Some(ChipHandle(MEGA)));
    // The second slot a button showing a chip, the third a plain button.
    {
        let s = p.side.screen.as_mut().unwrap();
        s.slots[1] = Slot { kind: SlotKind::Button { button: ButtonHandle(1), cell: ButtonCell::Only }, face: Some(ChipHandle(WAVE)), ..s.slots[1] };
        s.slots[2] = Slot { kind: SlotKind::Button { button: ButtonHandle(1), cell: ButtonCell::Only }, face: None, ..s.slots[2] };
    }
    p.press(keys::RIGHT);
    assert_eq!((p.screen().cursor, p.screen().look.chip_window.slot), (1, 1));
    assert_eq!(framed(&p), Some(ChipHandle(MEGA)), "the button's chip sets no frame");
    // R: its chip's description.
    p.press(keys::R);
    assert!(matches!(p.phase(), Phase::Description { window: None, .. }));
    while p.phase() != Phase::Choosing && p.tick < 1000 {
        p.press(keys::A);
    }
    assert_eq!(p.phase(), Phase::Choosing);
    // A button's picture: the standard frame; R on it describes nothing.
    p.press(keys::RIGHT);
    assert_eq!((p.screen().cursor, framed(&p)), (2, None));
    p.press(keys::R);
    assert_eq!(p.phase(), Phase::Choosing);
    // Back on the chip: its own again; OK: the standard one.
    p.press(keys::LEFT);
    p.press(keys::LEFT);
    assert_eq!((p.screen().cursor, framed(&p)), (0, Some(ChipHandle(MEGA))));
    p.press(keys::START);
    assert_eq!((p.screen().cursor, framed(&p)), (OK_SLOT, None));
}
