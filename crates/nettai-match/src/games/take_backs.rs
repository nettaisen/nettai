//! The custom screen's take-backs in each game's content (docs/design/custom-screen-actions.md §2): a pick taken
//! back from anywhere (`Unpick`) with what it carries (a soul given for a chip, a capsule, Beast Out, the BeastOut
//! chip, an offered dark chip's flag), a held chip (Arm Change) kept through it and released, and a Cross
//! un-chosen with the picks kept (`Unchoose`); `Unpick` of the last pick is B, tick for tick.

use super::*;
use nettai_battle::custom::FolderChip;
use nettai_battle::custom::screen::{Phase, SPECIAL_SLOT, Screen, SlotKind, SlotState};
use nettai_battle::content::ChipCode;
use nettai_battle::input::PlayerTick;
use nettai_battle::{Battle, ScreenChoice};
use nettai_content_api::{ChipHandle, FieldValue};

/// Codes by their letters' numbers.
const A: u8 = 0;
const S: u8 = 18;
const STAR: u8 = 26;

/// A tick with side 0's choice, side 1 idle.
fn tick(b: &mut Battle, choice: Option<ScreenChoice>) {
    b.tick(&[PlayerTick::default().with_choice(choice), Default::default()], Default::default());
}

fn choosing(b: &Battle) -> bool {
    b.custom.sides[0].screen.is_some_and(|s| s.phase == Phase::Choosing)
}

fn screen(b: &Battle) -> &Screen {
    b.custom.sides[0].screen.as_ref().expect("side 0's screen")
}

/// The choice, then ticks until side 0's screen chooses again (a window it
/// opened played).
fn choose(b: &mut Battle, choice: ScreenChoice) {
    tick(b, Some(choice));
    for _ in 0..200 {
        if choosing(b) {
            return;
        }
        tick(b, None);
    }
    panic!("side 0's screen stays out of choosing after {choice:?}");
}

/// A battle of match `m` whose side 0's folder is 30 of `chip` in code
/// `code`, ticked until its first screen chooses.
fn battle(content: &Arc<Content>, m: Match, game: &str, chip: &str, code: u8) -> Battle {
    battle_in(content, m, game, chip, code, None)
}

/// The same, side 0's navi in form `form` from the round's start (a soul:
/// its screen's buttons).
fn battle_in(content: &Arc<Content>, mut m: Match, game: &str, chip: &str, code: u8, form: Option<FormHandle>) -> Battle {
    let c = crate::ids::chip(content, game, chip).unwrap();
    let mut f = m.sides[0].folder(content);
    f.chips = [Some(FolderChip::new(c, ChipCode(code))); 30];
    m.sides[0].set_folder(content, &f).unwrap();
    let mut b = Battle::new(m.round(content, 0x5EED), content.clone());
    if let Some(form) = form {
        b.stats[0].starting_form = form;
    }
    for _ in 0..600 {
        if choosing(&b) {
            return b;
        }
        tick(&mut b, None);
    }
    panic!("no custom screen");
}

/// The chip dealt in side 0's slot `slot` made `chip` in code `code`, as if
/// dealt so (its folder entry).
fn deal(b: &mut Battle, content: &Content, game: &str, slot: u8, chip: &str, code: u8) {
    let SlotKind::Chip { index, .. } = screen(b).slots[slot as usize].kind else { panic!("slot {slot} holds no dealt chip") };
    let c = crate::ids::chip(content, game, chip).unwrap();
    b.custom.sides[0].folder.chips[index as usize] = Some(FolderChip::new(c, ChipCode(code)));
}

/// OK on side 0's screen: the hand's chips and the turn's form.
fn ok(b: &mut Battle) -> (Vec<ChipHandle>, Option<FormHandle>) {
    tick(b, Some(ScreenChoice::Ok));
    let (hand, transform) = b.custom.sides[0].built.clone().expect("OK built the hand");
    (hand.map(|h| h.ids.iter().flatten().copied().collect()).unwrap_or_default(), transform.form)
}

/// A field of side 0's rules' state, by its name.
fn field(b: &Battle, name: &str) -> FieldValue {
    let (schema, state) = b.rules_state(0).expect("the game's rules");
    let path = schema.find(name).unwrap().unwrap_or_else(|| panic!("the rules keep no `{name}`"));
    state.get_at(schema.place_of(path))
}

/// The same setup played twice, then B on one and `Unpick` of the last pick
/// on the other: the same screen from that tick, and the same battle on
/// every tick after (the tick's own choices, which the battle keeps for the
/// tick, aside).
fn unpick_is_b(label: &str, new: impl Fn() -> Battle, setup: impl Fn(&mut Battle)) {
    let (mut b, mut unpick) = (new(), new());
    setup(&mut b);
    setup(&mut unpick);
    let last = screen(&b).selected - 1;
    tick(&mut b, Some(ScreenChoice::TakeBack));
    tick(&mut unpick, Some(ScreenChoice::Unpick(last)));
    for t in 0..60 {
        assert_eq!(unpick.custom.sides[0].screen, b.custom.sides[0].screen, "{label}: tick {t}");
        assert_eq!(unpick.sound_cues(), b.sound_cues(), "{label}: tick {t}");
        if t > 0 {
            assert_eq!(unpick.digest(), b.digest(), "{label}: tick {t}");
        }
        tick(&mut b, None);
        tick(&mut unpick, None);
    }
}

/// EXE5's soul given for a Sword, taken back before later picks: the Sword
/// back in its place where it goes with them (more Swords), else back in the
/// hand with the soul (a Cannon A picked since); OK builds the hand of what
/// is left, with no soul.
#[test]
fn exe5_a_soul_taken_back_before_later_picks() {
    let content = exe5_content();
    let m = || parse(&content, &exe5(&TANGO_EXE5, ""), &exe5(&TANGO_EXE5, "")).unwrap();
    let sword = crate::ids::chip(&content, "exe5", "sword").unwrap();
    let cannon = crate::ids::chip(&content, "exe5", "cannon").unwrap();
    // Fits: Swords S after it.
    let mut b = battle(&content, m(), "exe5", "sword", S);
    choose(&mut b, ScreenChoice::Pick(1));
    choose(&mut b, ScreenChoice::Pick(0));
    choose(&mut b, ScreenChoice::Press(SPECIAL_SLOT));
    assert_eq!(screen(&b).selection(), [SPECIAL_SLOT, 1]);
    choose(&mut b, ScreenChoice::Pick(2));
    choose(&mut b, ScreenChoice::Unpick(0));
    let s = screen(&b);
    assert_eq!((s.selection(), s.trade), (&[0u8, 1, 2][..], None));
    assert_eq!(s.slots[SPECIAL_SLOT as usize].state, SlotState::Selectable, "the soul on offer again (the last pick a Sword)");
    assert_eq!(ok(&mut b), (vec![sword; 3], None));
    // Doesn't fit: a Cannon A after it.
    let mut b = battle(&content, m(), "exe5", "sword", S);
    deal(&mut b, &content, "exe5", 3, "cannon", A);
    choose(&mut b, ScreenChoice::Pick(0));
    choose(&mut b, ScreenChoice::Press(SPECIAL_SLOT));
    choose(&mut b, ScreenChoice::Pick(3));
    assert_eq!(screen(&b).selection(), [SPECIAL_SLOT, 3]);
    choose(&mut b, ScreenChoice::Unpick(0));
    let s = screen(&b);
    assert_eq!((s.selection(), s.trade), (&[3u8][..], None));
    assert_eq!(s.slots[0].state, SlotState::Unavailable, "the Sword back in the hand, not with the Cannon A");
    assert_eq!(s.slots[SPECIAL_SLOT as usize].state, SlotState::Unavailable, "a Cannon offers no soul");
    assert_eq!(ok(&mut b), (vec![cannon], None));
    assert_eq!(b.custom.sides[0].folder.count(), 29, "the Sword stayed in the folder");
}

/// EXE5's `Unpick` of the last pick is B: a chip, a soul given for one, a
/// chip with a capsule (MeddySoul).
#[test]
fn exe5_unpicking_the_last_pick_is_b() {
    let content = exe5_content();
    let meddy = crate::ids::form(&content, "exe5", "meddysoul").unwrap();
    let new = |form: Option<FormHandle>| {
        let m = parse(&content, &exe5(&TANGO_EXE5, ""), &exe5(&TANGO_EXE5, "")).unwrap();
        battle_in(&content, m, "exe5", "sword", S, form)
    };
    unpick_is_b("a chip", || new(None), |b| {
        choose(b, ScreenChoice::Pick(0));
        choose(b, ScreenChoice::Pick(1));
    });
    unpick_is_b("a soul", || new(None), |b| {
        choose(b, ScreenChoice::Pick(0));
        choose(b, ScreenChoice::Press(SPECIAL_SLOT));
    });
    // (In MeddySoul from the round's start: its screen deals capsules.)
    unpick_is_b("a chip with a capsule", || new(Some(meddy)), |b| {
        choose(b, ScreenChoice::Pick(0));
        choose(b, ScreenChoice::Press(8));
        assert!(screen(b).slots[0].attached.is_some());
    });
}

/// EXE5's MeddySoul: a capsule mixed into a pick leaves with it from the
/// middle of the column; the capsule in a later pick stays, and OK's hand
/// carries its marks.
#[test]
fn exe5_a_capsule_leaves_with_its_pick() {
    let content = exe5_content();
    let meddy = crate::ids::form(&content, "exe5", "meddysoul").unwrap();
    let sword = crate::ids::chip(&content, "exe5", "sword").unwrap();
    let m = parse(&content, &exe5(&TANGO_EXE5, ""), &exe5(&TANGO_EXE5, "")).unwrap();
    let mut b = battle_in(&content, m, "exe5", "sword", S, Some(meddy));
    choose(&mut b, ScreenChoice::Pick(0));
    choose(&mut b, ScreenChoice::Press(8));
    choose(&mut b, ScreenChoice::Pick(1));
    choose(&mut b, ScreenChoice::Press(9));
    let s = screen(&b);
    assert_eq!((s.slots[0].attached, s.slots[1].attached), (Some(8), Some(9)));
    let marks = s.slots[1].marks;
    assert!(!marks.is_empty());
    choose(&mut b, ScreenChoice::Unpick(0));
    let s = screen(&b);
    assert_eq!(s.selection(), [1]);
    assert_eq!((s.slots[0].attached, s.slots[0].marks.is_empty()), (None, true), "the first capsule left with its Sword");
    assert_eq!((s.slots[1].attached, s.slots[1].marks), (Some(9), marks), "the second stays in its Sword");
    tick(&mut b, Some(ScreenChoice::Ok));
    let (hand, _) = b.custom.sides[0].built.clone().expect("OK built the hand");
    let hand = hand.expect("a hand");
    assert_eq!((hand.ids[0], hand.ids[1], hand.modifiers[0]), (Some(sword), None, marks));
}

/// EXE5's ColonelSoul: the chip Arm Change holds is no pick, so `Unpick`
/// leaves it held (its place moving down past a pick taken back before it);
/// `Release` puts it back in its place; held at OK, it is the turn's arm
/// chip.
#[test]
fn exe5_arm_change_holds_through_unpicks() {
    let content = exe5_content();
    let colonel = crate::ids::form(&content, "exe5", "colonelsoul").unwrap();
    let cannon = crate::ids::chip(&content, "exe5", "cannon").unwrap();
    let m = parse(&content, &exe5(&TANGO_EXE5, ""), &exe5(&TANGO_EXE5, "")).unwrap();
    let mut b = battle_in(&content, m, "exe5", "cannon", A, Some(colonel));
    assert!(matches!(screen(&b).slots[8].kind, SlotKind::Button { button, .. } if content.defs.button(button).name == "arm_change"));
    choose(&mut b, ScreenChoice::Pick(1));
    choose(&mut b, ScreenChoice::Pick(0));
    choose(&mut b, ScreenChoice::Press(8));
    let s = screen(&b);
    assert_eq!((s.selection(), s.hold.map(|h| (h.chip, h.at))), (&[1u8][..], Some((0, 1))));
    choose(&mut b, ScreenChoice::Pick(2));
    choose(&mut b, ScreenChoice::Unpick(0));
    let s = screen(&b);
    assert_eq!((s.selection(), s.hold.map(|h| (h.chip, h.at))), (&[2u8][..], Some((0, 0))), "held, its place moved down");
    // Released: a Cannon A goes with the Cannon A picked, back in its place.
    choose(&mut b, ScreenChoice::Release(9));
    let s = screen(&b);
    assert_eq!((s.selection(), s.hold, s.slots[0].state), (&[0u8, 2][..], None, SlotState::Selected));
    // Held again (the last pick), and the pick before it taken back: OK
    // sends no chip, and the held one is the turn's arm chip.
    choose(&mut b, ScreenChoice::Press(8));
    choose(&mut b, ScreenChoice::Unpick(0));
    assert_eq!((screen(&b).selection(), screen(&b).hold.map(|h| (h.chip, h.at))), (&[][..], Some((2, 0))));
    tick(&mut b, Some(ScreenChoice::Ok));
    assert_eq!(field(&b, "arm_chip"), FieldValue::Ref(Some((nettai_content_api::Registry::Chip, cannon.0))));
}

/// An EXE6 match whose side 0 has Beast Out (Falzar's) and these Crosses.
fn exe6_match(content: &Arc<Content>, crosses: &[&str]) -> Match {
    let falzar = "cybeast = \"falzar-beast\"";
    let mut m = parse_in(content, "exe6", &side("megaman", &EXE6, falzar), &side("megaman", &EXE6, falzar)).unwrap();
    m.sides[0].set_fact(content, "beast_out", &flag(true)).unwrap();
    m.sides[0].set_fact(content, "crosses", &forms(content, "exe6", crosses)).unwrap();
    m
}

/// EXE6's Beast Out and the BeastOut chip, each first in the column, taken
/// back before later picks: their face and form go, the button is on offer
/// again, and OK's hand is the picks left.
#[test]
fn exe6_beast_out_taken_back_before_later_picks() {
    let content = exe6_content();
    let cannon = crate::ids::chip(&content, "exe6", "cannon").unwrap();
    let mut b = battle(&content, exe6_match(&content, &[]), "exe6", "cannon", A);
    choose(&mut b, ScreenChoice::Pick(0));
    choose(&mut b, ScreenChoice::Press(SPECIAL_SLOT));
    let s = screen(&b);
    assert_eq!(s.selection(), [SPECIAL_SLOT, 0], "Beast Out first");
    assert!(s.form.is_some() && s.look.face.is_some());
    choose(&mut b, ScreenChoice::Pick(1));
    choose(&mut b, ScreenChoice::Unpick(0));
    let s = screen(&b);
    assert_eq!(s.selection(), [0, 1]);
    assert_eq!((s.form, s.look.face, s.slots[SPECIAL_SLOT as usize].state), (None, None, SlotState::Selectable));
    assert_eq!(ok(&mut b), (vec![cannon; 2], None));
    // The BeastOut chip: its window moves it first and puts on its face.
    let mut b = battle(&content, exe6_match(&content, &[]), "exe6", "cannon", A);
    deal(&mut b, &content, "exe6", 1, "beastout", STAR);
    choose(&mut b, ScreenChoice::Pick(0));
    choose(&mut b, ScreenChoice::Pick(1));
    assert_eq!(screen(&b).selection(), [1, 0]);
    assert!(screen(&b).look.face.is_some());
    choose(&mut b, ScreenChoice::Pick(2));
    choose(&mut b, ScreenChoice::Unpick(0));
    let s = screen(&b);
    assert_eq!((s.selection(), s.look.face), (&[0u8, 2][..], None));
    assert_eq!(ok(&mut b), (vec![cannon; 2], None));
    // `Unpick` of the last is B: Beast Out alone.
    unpick_is_b("Beast Out", || battle(&content, exe6_match(&content, &[]), "exe6", "cannon", A), |b| {
        choose(b, ScreenChoice::Press(SPECIAL_SLOT));
    });
}

/// EXE6's Cross un-chosen with picks: they stay, the form and face go,
/// Beast Out is on offer again, and OK's hand is the picks with no form. A
/// BeastOut chip picked keeps its face. B with nothing picked still
/// un-chooses; `Unchoose` with nothing chosen is refused.
#[test]
fn exe6_unchoosing_a_cross_keeps_the_picks() {
    let content = exe6_content();
    let cannon = crate::ids::chip(&content, "exe6", "cannon").unwrap();
    let crosses = ["heatcross", "eleccross"];
    let mut b = battle(&content, exe6_match(&content, &crosses), "exe6", "cannon", A);
    assert_eq!(b.form_list(0).map(|l| l.count), Some(2));
    choose(&mut b, ScreenChoice::Pick(0));
    choose(&mut b, ScreenChoice::Pick(1));
    choose(&mut b, ScreenChoice::ChooseForm(1));
    let s = screen(&b);
    assert!(s.form.is_some() && s.look.face.is_some());
    assert_eq!((b.form_list(0).and_then(|l| l.chosen), s.slots[SPECIAL_SLOT as usize].state), (Some(1), SlotState::Unavailable));
    choose(&mut b, ScreenChoice::Unchoose);
    let s = screen(&b);
    assert_eq!((s.selection(), s.form, s.look.face), (&[0u8, 1][..], None, None));
    assert_eq!((b.form_list(0).and_then(|l| l.chosen), s.slots[SPECIAL_SLOT as usize].state), (None, SlotState::Selectable));
    // Nothing chosen: refused, nothing changes.
    let before = *screen(&b);
    choose(&mut b, ScreenChoice::Unchoose);
    assert_eq!((screen(&b).selection(), screen(&b).form), (before.selection(), None));
    // B with nothing picked un-chooses as the game's.
    choose(&mut b, ScreenChoice::ChooseForm(0));
    for _ in 0..3 {
        choose(&mut b, ScreenChoice::TakeBack);
    }
    assert_eq!((screen(&b).selection(), screen(&b).form), (&[][..], None));
    choose(&mut b, ScreenChoice::Pick(0));
    choose(&mut b, ScreenChoice::Pick(1));
    choose(&mut b, ScreenChoice::ChooseForm(0));
    choose(&mut b, ScreenChoice::Unchoose);
    assert_eq!(ok(&mut b), (vec![cannon; 2], None));
    // A BeastOut chip picked before the Cross: its face again.
    let mut b = battle(&content, exe6_match(&content, &crosses), "exe6", "cannon", A);
    deal(&mut b, &content, "exe6", 1, "beastout", STAR);
    choose(&mut b, ScreenChoice::Pick(1));
    let beast = screen(&b).look.face;
    assert!(beast.is_some());
    choose(&mut b, ScreenChoice::ChooseForm(0));
    assert_ne!(screen(&b).look.face, beast, "the Cross's face");
    choose(&mut b, ScreenChoice::Unchoose);
    assert_eq!((screen(&b).selection(), screen(&b).look.face), (&[1u8][..], beast));
}

/// EXE4's offered dark chips (a dark MegaMan's): the flag of an offered pick
/// moves with it when a pick before it is taken back, so OK still counts it
/// sent.
#[test]
fn exe4_an_offered_dark_chip_keeps_its_flag_through_an_unpick() {
    let content = exe4_content();
    let mut m = crate::pick::live(&content, "exe4", 3, None).unwrap();
    m.sides[0].set_fact(&content, "karma", &number(100)).unwrap();
    let mut b = battle(&content, m, "exe4", "cannon", A);
    assert!(matches!(screen(&b).slots[8].kind, SlotKind::Offered(_)), "a dark MegaMan is offered dark chips");
    choose(&mut b, ScreenChoice::Pick(8));
    choose(&mut b, ScreenChoice::Pick(9));
    assert_eq!((field(&b, "picks"), field(&b, "offered_picks")), (FieldValue::U8(2), FieldValue::U16(0b11)));
    choose(&mut b, ScreenChoice::Unpick(0));
    assert_eq!(screen(&b).selection(), [9]);
    assert_eq!((field(&b, "picks"), field(&b, "offered_picks")), (FieldValue::U8(1), FieldValue::U16(0b1)));
    tick(&mut b, Some(ScreenChoice::Ok));
    assert_eq!(field(&b, "dark_chips_sent"), FieldValue::U8(1));
}

/// EXE4's Double Soul given for a Sword, taken back before a Cannon A picked
/// since: the Sword doesn't go with it and goes back to the hand with the
/// soul; the rules hear its pick taken back (their count of picks).
#[test]
fn exe4_a_double_soul_taken_back_before_later_picks() {
    let content = exe4_content();
    let cannon = crate::ids::chip(&content, "exe4", "cannon").unwrap();
    let mut m = crate::pick::live(&content, "exe4", 3, None).unwrap();
    m.sides[0].set_fact(&content, "karma", &number(500)).unwrap();
    m.sides[0].set_fact(&content, "double_soul", &flag(true)).unwrap();
    m.sides[0].set_fact(&content, "souls", &forms(&content, "exe4", &["protosoul"])).unwrap();
    let mut b = battle(&content, m, "exe4", "sword", S);
    deal(&mut b, &content, "exe4", 3, "cannon", A);
    // (Slot 1: the match's folder may have its Regular chip in slot 0, which
    // offers no soul.)
    choose(&mut b, ScreenChoice::Pick(1));
    choose(&mut b, ScreenChoice::Press(SPECIAL_SLOT));
    assert_eq!(screen(&b).selection(), [SPECIAL_SLOT]);
    choose(&mut b, ScreenChoice::Pick(3));
    assert_eq!(field(&b, "picks"), FieldValue::U8(2));
    choose(&mut b, ScreenChoice::Unpick(0));
    let s = screen(&b);
    assert_eq!((s.selection(), s.trade, s.slots[1].state), (&[3u8][..], None, SlotState::Unavailable));
    assert_eq!(field(&b, "picks"), FieldValue::U8(1));
    assert_eq!(ok(&mut b), (vec![cannon], None));
}
