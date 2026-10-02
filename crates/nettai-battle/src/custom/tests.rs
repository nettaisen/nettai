//! One player's custom screen driven by scripted buttons, with made-up
//! chips.

use super::library::testing::{EVERY_CODE, TestLibrary, chip};
use super::screen::{OK_SLOT, SPECIAL_SLOT};
use super::*;
use crate::content::{ChipClass, ChipCode, ChipFlags};
use crate::custom::library::testing::ChipId;
use crate::input::keys;
use nettai_content_api::{ChipHandle, FormHandle};

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
            late_turns: false,
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
    let (invalid, code) = (library::testing::INVALID, screen::INVALID_CODE);
    assert_eq!(hand.ids[0], Some(ChipHandle(invalid)));
    assert_eq!(hand.selection[0], Some(FolderChip::new(ChipHandle(invalid), code)));
}

#[test]
fn beast_out() {
    for (version, beast) in [(GameVersion::Falzar, library::testing::FALZAR_BEAST), (GameVersion::Gregar, library::testing::GREGAR_BEAST)] {
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
        assert_eq!(sent.result.transform.form, Some(beast));
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
    assert_eq!(p.side.sent.as_ref().unwrap().result.transform.form, Some(FormHandle(7)));
    assert!(p.side.round.crosses_used[1]);
}

/// Open the screen, open the Cross window, move DOWN `down` times and
/// choose that Cross; the screen is back to choosing chips after.
fn choose_cross(p: &mut Player, down: usize) {
    p.open();
    p.wait(10);
    p.step(0);
    p.press(keys::UP);
    while p.phase() != (Phase::CrossWindow { entered: true }) && p.tick < 1000 {
        p.step(0);
    }
    for _ in 0..down {
        p.press(keys::DOWN);
    }
    p.step(keys::A);
    while p.phase() != Phase::Choosing && p.tick < 1000 {
        p.step(0);
    }
}

/// OK, and what goes out.
fn confirm(p: &mut Player) -> CustomResult {
    p.press(keys::START);
    p.press(keys::A);
    p.wait(20);
    p.side.sent.as_ref().unwrap().result.clone()
}

/// A setup's Cross list (nettai's extension): a Falzar player offered
/// Gregar's first Cross (form 1) and Falzar's fourth (9) gets those two,
/// in that order, and the one chosen is what goes out.
#[test]
fn a_setups_cross_list_offers_crosses_of_either_game() {
    let mut p = Player::new(&[], GameVersion::Falzar);
    p.side.unlocks.cross_list = Some(CrossList::new(&[FormHandle(1), FormHandle(9)]));
    choose_cross(&mut p, 0);
    let w = p.screen().crosses;
    assert_eq!((w.count, &w.offered[..2], w.chosen), (2, &[0, 1][..], Some(0)));
    // The emotion window shows the Cross's face.
    assert_eq!(p.screen().look.face, Some(FormHandle(1)));
    assert_eq!(confirm(&mut p).transform.form, Some(FormHandle(1)));
    assert_eq!(p.side.round.crosses_used, [true, false, false, false, false]);
    // On the round's next screen the Cross used isn't offered again.
    p.side.screen = None;
    let ctx = p.context();
    let (mut side, mut console) = (p.side.clone(), p.console);
    side.open(&Context { turn: 2, ..ctx }, &mut console);
    let w = side.screen.unwrap().crosses;
    assert_eq!((w.count, w.offered[0]), (1, 1));
}

/// Beast Out from a Cross of the other game is that Cross's form in Beast
/// Out, of that game's Beast (a Falzar player in Gregar's first Cross goes
/// to its Beast form, 0x0D), with that game's roar; tired, that game's
/// Beast Over. Without a Cross list the Beast's game is the version's.
#[test]
fn beast_out_from_the_other_games_cross_is_its_beast_form() {
    use super::look::ScreenSound;
    for list in [true, false] {
        let mut p = Player::new(&[], GameVersion::Falzar);
        if list {
            p.side.unlocks.cross_list = Some(CrossList::new(&[FormHandle(1), FormHandle(9)]));
        }
        // In Gregar's first Cross.
        p.stats.form = FormHandle(1);
        p.open();
        p.wait(10);
        p.step(0);
        p.press(keys::START);
        p.press(keys::DOWN);
        p.step(keys::A);
        let mut roars = Vec::new();
        while p.phase() != Phase::Choosing && p.tick < 1000 {
            p.step(0);
            roars.extend(p.screen().look.drawn.sounds().filter(|s| matches!(s, ScreenSound::BeastOut(_))));
        }
        let game = if list { GameVersion::Gregar } else { GameVersion::Falzar };
        assert_eq!(roars, [ScreenSound::BeastOut(game)], "list {list}");
        assert_eq!(p.screen().look.face, Some(FormHandle(0x0D)), "list {list}");
        p.press(keys::UP);
        p.press(keys::A);
        p.wait(20);
        assert_eq!(p.side.sent.as_ref().unwrap().result.transform.form, Some(FormHandle(0x0D)), "list {list}");
        // Tired: Beast Over of the Beast's game (Gregar's 0x17, Falzar's 0x18).
        let over = p.side.unlocks.beast_form(&p.lib, p.stats.navi, FormHandle(1), true);
        assert_eq!(over, Some(FormHandle(if list { 0x17 } else { 0x18 })), "list {list}");
        // From the base form Beast Out is the version's.
        let base = p.side.unlocks.beast_form(&p.lib, p.stats.navi, FormHandle(0), false);
        assert_eq!(base, Some(library::testing::FALZAR_BEAST));
        assert_eq!(p.side.unlocks.beast_game(&p.lib, FormHandle(0)), GameVersion::Falzar);
    }
}

/// In a Beast form a Cross list offers the Crosses whose Beast it is: in
/// Falzar's Beast Falzar's, in a Gregar Cross's Beast form Gregar's; each
/// takes the navi to its form in Beast Out.
#[test]
fn in_a_beast_form_a_cross_list_offers_that_beasts_crosses() {
    for (beast, place, form) in [(library::testing::FALZAR_BEAST, 1, 9), (FormHandle(0x0E), 0, 1)] {
        let mut p = Player::new(&[], GameVersion::Falzar);
        p.side.unlocks.cross_list = Some(CrossList::new(&[FormHandle(1), FormHandle(9)]));
        p.stats.form = beast;
        choose_cross(&mut p, 0);
        let w = p.screen().crosses;
        assert_eq!((w.count, w.offered[0], w.chosen), (1, place, Some(place)));
        assert_eq!(confirm(&mut p).transform.form, Some(FormHandle(form + 0x0C)));
    }
}

/// A Cross list names Crosses only, and leaves out the navi's starting
/// form, as the original's window does.
#[test]
fn a_cross_list_offers_crosses_only() {
    let mut p = Player::new(&[], GameVersion::Gregar);
    let list = [FormHandle(6), library::testing::GREGAR_BEAST, FormHandle(2), FormHandle(7)];
    p.side.unlocks.cross_list = Some(CrossList::new(&list));
    p.stats.starting_form = FormHandle(2);
    p.open();
    let w = p.screen().crosses;
    assert_eq!((w.count, &w.offered[..2]), (2, &[0, 3][..]));
}

#[test]
fn dust_cross_scraps_the_picks() {
    let mut p = Player::new(&[(SHOT, 0), (SHOT, 1), (WAVE, 0), (WAVE, 1), (SHOT, 2), (MEGA, 5), (MEGA, 6)], GameVersion::Falzar);
    p.stats.form = library::testing::DUST_CROSS;
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

#[test]
fn the_tag_pair_index_follows_the_folder_as_picks_leave_it() {
    let chips: Vec<(ChipId, u8)> = (0..30).map(|i| ([SHOT, WAVE][i % 2], (i / 2) as u8 % 3)).collect();
    let mut p = Player::new(&chips, GameVersion::Falzar);
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
    let mut p = Player::new(&[(SHOT, 0), (SHOT, 1)], GameVersion::Falzar);
    p.lib.chips[0].1.description = Some(["a", "b", "c"][..lines].join("\n"));
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
    let mut p = Player::new(&[(SHOT, 0), (DARK, 0), (SHOT, 1)], GameVersion::Falzar);
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
    let mut p = Player::new(&[(SHOT, 0), (MEGA, 0)], GameVersion::Falzar);
    p.open();
    assert_eq!(p.screen().cursor, 0);
    for _ in 0..20 {
        p.step(0);
        assert_eq!((p.screen().look.dark, p.screen().look.drawn.volume), (DarkHover::Clear, None));
    }
}
