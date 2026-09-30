//! One player's custom screen driven by scripted buttons, with made-up
//! chips.

use super::library::testing::{EVERY_CODE, TestLibrary, chip};
use super::screen::{OK_SLOT, SPECIAL_SLOT};
use super::*;
use crate::content::{ChipClass, ChipCode, ChipFlags, ChipId};
use crate::input::keys;
use bn6_content_api::{ChipHandle, FormHandle};

/// The test library's handles are the numbers.
fn form(f: Form) -> FormHandle {
    FormHandle(f.0 as u16)
}

const STAR: u8 = 26;
/// Made-up chips: a damaging chip in codes A-C and *, a second one in A, B
/// and *, a Mega chip in every code.
const SHOT: ChipId = 1;
const WAVE: ChipId = 2;
const MEGA: ChipId = 3;

fn library() -> TestLibrary {
    const SHOT_CODES: &[ChipCode] = &[ChipCode(0), ChipCode(1), ChipCode(2), ChipCode(26)];
    const WAVE_CODES: &[ChipCode] = &[ChipCode(0), ChipCode(1), ChipCode(26)];
    TestLibrary::new(
        vec![
            (SHOT, chip(ChipClass::Standard, SHOT_CODES, ChipFlags::HAS_DAMAGE, 40)),
            (WAVE, chip(ChipClass::Standard, WAVE_CODES, ChipFlags::HAS_DAMAGE, 60)),
            (MEGA, chip(ChipClass::Mega, EVERY_CODE, ChipFlags::HAS_DAMAGE, 150)),
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

/// A player's side with this folder, and what their screen reads.
struct Player {
    side: Side,
    console: Console,
    lib: TestLibrary,
    stats: NaviStats,
    tick: u32,
}

impl Player {
    fn new(chips: &[(ChipId, u8)], version: GameVersion) -> Player {
        let setup = PlayerSetup { folder: Some(folder(chips)), unlocks: Unlocks::everything(version), ..PlayerSetup::default() };
        Player { side: Side::new(&setup), console: Console::new(&setup.console), lib: library(), stats: stats(), tick: 0 }
    }

    fn context(&self) -> Context<'_> {
        Context {
            library: &self.lib,
            stats: self.stats,
            emotion: Emotion::Normal,
            turn: 1,
            per_player_gauges: false,
            random_battle: false,
            now: self.tick,
            link_delay: 4,
        }
    }

    fn open(&mut self) {
        let ctx = self.context();
        let mut side = self.side.clone();
        let mut console = self.console;
        side.open(&ctx, &mut console);
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
        let r = side.tick(&ctx, &mut console, |id| self.lib.chip(id).damage);
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
    let mut p = Player::new(&[], GameVersion::Falzar);
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
    let ok = s.slots[OK_SLOT as usize];
    assert_eq!((ok.vertical, ok.left, ok.right), (Some(SPECIAL_SLOT), Some(4), Some(0)));
    let beast = s.slots[SPECIAL_SLOT as usize];
    assert_eq!((beast.kind, beast.vertical, beast.left, beast.right), (SlotKind::BeastOut, Some(OK_SLOT), None, None));
    assert_eq!(s.crosses.count, 5);
}

#[test]
fn the_timeline_from_opening_to_sending() {
    let mut p = Player::new(&[(SHOT, 0), (SHOT, 1)], GameVersion::Falzar);
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
    let mut p = Player::new(&[], GameVersion::Falzar);
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
    let mut p = Player::new(&[(SHOT, 0), (SHOT, 1), (WAVE, 0), (WAVE, STAR), (MEGA, 2)], GameVersion::Falzar);
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
    // A on a greyed chip does nothing; B takes back the last pick.
    p.press(keys::RIGHT);
    p.press(keys::A);
    assert_eq!(p.screen().selection(), [0, 3]);
    p.press(keys::B);
    assert_eq!(p.screen().selection(), [0]);
    assert_eq!(states(&p), [Selected, Selectable, Selectable, Selectable, Unavailable]);
}

#[test]
fn mega_chips_past_the_limit_turn_invalid() {
    let mut p = Player::new(&[(MEGA, 0)], GameVersion::Falzar);
    p.side.class_uses = builder::ClassCounts { mega: 2, ..Default::default() };
    p.open();
    p.wait(10);
    p.step(0);
    p.press(keys::A);
    p.press(keys::START);
    p.press(keys::A);
    p.wait(20);
    let hand = p.side.sent.as_ref().unwrap().result.hand.clone().unwrap();
    let (invalid, code) = screen::INVALID_CHIP;
    assert_eq!(hand.ids[0], Some(ChipHandle(invalid)));
    assert_eq!(hand.selection[0], Some(FolderChip::new(ChipHandle(invalid), code)));
}

#[test]
fn beast_out() {
    for (version, beast) in [(GameVersion::Falzar, Form::FALZAR_BEAST), (GameVersion::Gregar, Form::GREGAR_BEAST)] {
        let mut p = Player::new(&[], version);
        p.open();
        p.wait(10);
        p.step(0);
        p.press(keys::START);
        p.press(keys::DOWN);
        assert_eq!(p.screen().cursor, SPECIAL_SLOT);
        let a_tick = p.tick + 1;
        p.step(keys::A);
        assert!(matches!(p.phase(), Phase::BeastOutChosen { .. }));
        while p.phase() != Phase::Choosing && p.tick < 1000 {
            p.step(0);
        }
        assert_eq!(p.tick, a_tick + 70);
        assert!(p.screen().beast_out);
        assert_eq!(p.screen().slots[SPECIAL_SLOT as usize].state, SlotState::Selected);
        p.press(keys::UP);
        p.press(keys::A);
        p.wait(20);
        let sent = p.side.sent.as_ref().unwrap();
        assert_eq!(sent.result.transform.form, Some(form(beast)));
        // Only Beast Out was picked: an empty hand goes out (and replaces
        // what the navi still held).
        assert_eq!(sent.result.hand.as_ref().unwrap().ids[0], None);
        assert!(p.side.round.beast_out_used);
    }
}

#[test]
fn a_cross_from_the_window() {
    let mut p = Player::new(&[], GameVersion::Falzar);
    p.open();
    p.wait(10);
    p.step(0);
    // UP from the top row opens the window (12 ticks); DOWN moves to the
    // second Cross; A chooses it (34 ticks).
    // UP repeats (and acts) on the second tick of the hold.
    p.step(keys::UP);
    let up = p.tick + 1;
    p.step(keys::UP);
    p.step(0);
    while p.phase() != (Phase::CrossWindow { entered: true }) && p.tick < 1000 {
        p.step(0);
    }
    assert_eq!(p.tick, up + 12);
    p.press(keys::DOWN);
    let a = p.tick + 1;
    p.step(keys::A);
    while p.phase() != Phase::Choosing && p.tick < 1000 {
        p.step(0);
    }
    assert_eq!(p.tick, a + 34);
    assert_eq!(p.screen().crosses.chosen, Some(1));
    // A chosen Cross greys out Beast Out.
    assert_eq!(p.screen().slots[SPECIAL_SLOT as usize].state, SlotState::Unavailable);
    p.press(keys::START);
    p.press(keys::A);
    p.wait(20);
    // Falzar's second Cross is form 7.
    assert_eq!(p.side.sent.as_ref().unwrap().result.transform.form, Some(form(Form(7))));
    assert!(p.side.round.crosses_used[1]);
}

#[test]
fn dust_cross_scraps_the_picks() {
    let mut p = Player::new(&[(SHOT, 0), (SHOT, 1), (WAVE, 0), (WAVE, 1), (SHOT, 2), (MEGA, 5), (MEGA, 6)], GameVersion::Falzar);
    p.stats.form = form(Form::DUST_CROSS);
    p.open();
    assert!(matches!(p.screen().slots[8].kind, SlotKind::Scrap { right_half: false }));
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
fn hand_size() {
    let size = |custom_level: u8, shrink: u8, turn: u8, number_open: bool| {
        let mut p = Player::new(&[], GameVersion::Falzar);
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
    let mut p = Player::new(&[], GameVersion::Falzar);
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
    let mut p = Player::new(&chips, GameVersion::Falzar);
    p.stats.chip_shuffle = true;
    p.console = Console::new(&ConsoleSetup { rng: 0x1234_5678, ..ConsoleSetup::default() });
    p.open();
    assert!(matches!(p.screen().slots[8].kind, SlotKind::Redeal { right_half: false }));
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
    let mut p = Player::new(&chips, GameVersion::Falzar);
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
