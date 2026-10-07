//! What the custom screen reads: its slot layout (a rule), the Program
//! Advances (each with the chip it makes) and the link navis' own chips
//! (each with its navi).

use super::ChipCode;
use nettai_content_api::ChipHandle;
use serde::{Deserialize, Serialize};

/// What a slot of the starting grid holds before the chips are dealt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateSlot {
    /// A place for a dealt chip (empty unless one is dealt).
    ChipPosition,
    /// The OK button.
    Ok,
    /// Nothing, unless a special button is put there.
    Hidden,
}

/// A slot of the starting grid, and how the cursor leaves it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SlotLayout {
    pub kind: TemplateSlot,
    pub moves: SlotMoves,
}

/// How the cursor leaves a slot, as a game's screen says.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SlotMoves {
    /// Its neighbors (slot numbers), which the screen fixes up as it opens
    /// where one is absent (`sub_8027F42`'s scans, the layout's lists); UP
    /// and DOWN both go to `vertical`. The keys are read in the screen's
    /// order (`sub_8028B74`: the directions, then A, B, START, SELECT, R and
    /// L).
    Neighbors { vertical: u8, left: u8, right: u8 },
    /// The keys the screen reads with the cursor here, in the order it reads
    /// them (the first one down is the tick's), and for each direction the
    /// slots it goes to, the first that is there as the key is read (EXE4's
    /// selection, 0x08020350, and its lists: 0x080204C0 for UP and DOWN by
    /// column, 0x08020518 and 0x08020568 for RIGHT and LEFT by slot; OK's,
    /// 0x08020620, and the button's under it, 0x08020728). A direction with
    /// none there goes nowhere.
    Candidates { keys: Vec<ScreenKey>, up: Vec<u8>, down: Vec<u8>, left: Vec<u8>, right: Vec<u8> },
}

impl SlotMoves {
    /// The neighbors model's neighbors (UP and DOWN, LEFT, RIGHT), for a
    /// slot that has them.
    pub fn neighbors(&self) -> Option<(u8, u8, u8)> {
        match *self {
            SlotMoves::Neighbors { vertical, left, right } => Some((vertical, left, right)),
            SlotMoves::Candidates { .. } => None,
        }
    }
}

/// A key the custom screen reads while choosing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreenKey {
    A,
    B,
    Start,
    Select,
    R,
    L,
    Up,
    Down,
    Left,
    Right,
}

impl ScreenKey {
    /// The neighbors model's order (`sub_8028B74`): the directions (UP and
    /// DOWN to the same slot), then A, B, START, SELECT, R and L.
    pub const NEIGHBORS_ORDER: [ScreenKey; 10] = [
        ScreenKey::Up,
        ScreenKey::Down,
        ScreenKey::Left,
        ScreenKey::Right,
        ScreenKey::A,
        ScreenKey::B,
        ScreenKey::Start,
        ScreenKey::Select,
        ScreenKey::R,
        ScreenKey::L,
    ];

    /// Whether it is a direction (read by its auto-repeat, the others by a
    /// press).
    pub fn is_direction(self) -> bool {
        matches!(self, ScreenKey::Up | ScreenKey::Down | ScreenKey::Left | ScreenKey::Right)
    }
}

/// The custom screen's slot grid (`dword_802A7CC`) and the lists its
/// neighbor fix-up scans (`sub_8027F42`): slots 0-4 are the top row,
/// 5-9 the bottom row, 10 OK and 11 the special button under it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CustomScreenLayout {
    pub slots: [SlotLayout; 12],
    /// Where the next slot to the left is looked for: from the top row
    /// and OK, and from the bottom row and slot 11.
    pub left_scan_top: Vec<u8>,
    pub left_scan_bottom: Vec<u8>,
    /// The same to the right.
    pub right_scan_top: Vec<u8>,
    pub right_scan_bottom: Vec<u8>,
    /// By slot: where its left and right scans start in their lists.
    pub left_scan_start: [u8; 12],
    pub right_scan_start: [u8; 12],
    /// The re-deal (`sub_8029788`): by how many of the hand's chips it deals
    /// again, how many of them stay in the hand (`byte_80298C8`: EXE6's are
    /// all zeros, as none listed; EXE5's 0x080254C8).
    pub redeal_kept: Vec<u8>,
    /// SELECT's hidden window, brought back by a key, draws the navi's
    /// emblem on that tick and the next (EXE6's `sub_8026D06`, its two
    /// calls of `sub_8029C08`); a screen that doesn't shows it again with
    /// the choosing state's own draw, two ticks later (EXE5's 0x08023022
    /// has neither call).
    pub emblem_at_window_return: bool,
    /// A command of a chatbox script waits out the character printed
    /// before it, as the next character would: the print delay still
    /// running holds it (EXE6's `chatbox_interpreteAndDrawDialogChar`
    /// tests the delay before a command too). A screen whose interpreter
    /// runs a command at once (EXE5's 0x0803EADC tests the delay before a
    /// character alone) starts a message's wait for a key two ticks sooner
    /// after its last character, and its speaker's mouth closes at a
    /// line's end as much sooner. A description prints with no delay:
    /// nothing of it waits either way.
    pub chatbox_commands_wait_for_text: bool,
    /// The characters of a message that move its speaker's mouth.
    /// Presentation.
    pub talking_characters: TalkingCharacters,
    /// The tick the window is in (its slide's last), the screen chooses from
    /// the next and reads its keys at once (EXE6's `sub_8026CCC`); else that
    /// tick only sets the cursor's state up, reading no key, and the keys
    /// count from the tick after (EXE4's selection state 0, 0x08020340: it
    /// puts back the state the cursor was in, and sets the custom screen's
    /// status bit 1).
    pub first_choosing_tick_reads_keys: bool,
    /// L's no-running message starts its chatbox with the key (EXE4's
    /// 0x080205C6, its state waiting from the next tick, 0x08020A28); else
    /// on the next tick, with its sound (EXE6's `sub_8026EC8`).
    pub run_message_at_key: bool,
    /// What becomes of a pick of a chip that counts as the invalid chip (a
    /// Mega or Giga chip past the navi's limit, a code the chip hasn't).
    pub invalid_picks: InvalidPicks,
    /// How a selection treats the codes outside the alphabet (the invalid
    /// chip's 0x1B, and 0x1C).
    pub special_codes: SpecialCodes,
    /// How OK makes a Program Advance of a selection.
    pub program_advances: ProgramAdvanceRules,
    /// A modifier chip folded into the chip before it leaves its Regular
    /// chip's mark on that chip (EXE4's 0x0801F176: the flags' bit 1 ORed
    /// in); else the mark goes with the modifier (EXE6's `sub_8029224`).
    pub modifier_passes_regular: bool,
    /// Until when a player's console says its custom screen is open (the
    /// status bits the other console's fight reads).
    pub status_until: StatusUntil,
    /// The shade the cursor casts resting on a dark chip (presentation).
    pub hover: HoverRules,
    /// The sound players the close sets back to full volume, in its order
    /// (EXE6's `sub_802A3CC`: 31, 22; EXE4's 0x0801E194: 9, 31).
    pub restore_players: Vec<u8>,
    /// The custom gauge empties as the screen opens (EXE6's `sub_8026840`:
    /// `sub_801DF92`), and again at the send; else it stays full until the
    /// send (EXE4's 0x0801E1B4: 0x080159B0 empties it).
    pub gauge_empties_at_open: bool,
    /// OK clears the console's fade records at once (EXE4's 0x08020656: a
    /// dark chip's shade goes with it); else they run on until the screens
    /// close (EXE6's `sub_8026A6C`: `sub_80062EC`). Presentation.
    pub fades_clear_at_ok: bool,
    /// The tick a key leaves the choosing (OK, SELECT, R, L, a window)
    /// still draws the cursor and the last turns' block (EXE6's
    /// `sub_8026CCC` draws after its keys); else the tick draws what the
    /// state its keys left it in draws: no cursor, the block taken off for
    /// OK and SELECT (EXE4's 0x0801E412). Presentation.
    pub cursor_after_leaving: bool,
    /// The choosing state draws the last turns' block and counts its frame
    /// (the control block's +0x40, which the cursor blinks by) before its
    /// keys and the rest of what it draws (EXE4's 0x0801E3DA, 0x0801E3DE:
    /// the cursor a frame further on); else after them (EXE6's
    /// `sub_8026CCC`). Presentation.
    pub frame_counts_first: bool,
    /// R's description and L's message are states of the choosing (EXE4's
    /// selection states 0x18 and 0x1C, 0x0801E430): each of their ticks
    /// draws the last turns' block and counts the frame as the choosing does
    /// (0x0801E3D8), and the tick one sees its chatbox closed goes back to
    /// the state it came from and draws it (the Regular chip's frame and the
    /// cursor: 0x08020A0A, 0x0801E412); else they are the screen's own
    /// states, which draw only the emblem (EXE6's `sub_8026E4C`).
    /// Presentation.
    pub description_in_choosing: bool,
}

/// The cursor's hover over a dark chip (EXE6's `sub_802A2B0`, EXE4's
/// 0x0801E478): the screen darkens, the music's volume and the screen's
/// player's ramp, and its sound plays now and then. Presentation.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct HoverRules {
    /// On which ticks it runs.
    pub runs: HoverRuns,
    /// The volume calls of its ramp toward the shade and back: by tick
    /// from the one after the hover turns, the music's player's and the
    /// screen's, or none that tick. It settles on the last.
    pub to_dark: Vec<Option<[u16; 2]>>,
    pub to_clear: Vec<Option<[u16; 2]>>,
    /// The players it ramps: the music's, the screen's.
    pub players: [u8; 2],
    /// When its sound plays.
    pub sound: HoverSound,
}

/// On which ticks the hover runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HoverRuns {
    /// After every tick's state (EXE6's `sub_802A2B0`): whatever is up
    /// that isn't the chips or a chip's description clears it.
    EveryTick,
    /// After the state of a tick the screen begins choosing (EXE4's call
    /// in its choosing state, 0x0801E40E): it stands still while the
    /// window slides, is hidden, the Program Advance plays or the result is
    /// sent; a tick whose state left choosing changes nothing of it but a
    /// ramp's step (0x0801E4B6).
    WhileChoosing,
}

/// When the hover's sound plays.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "counts", deny_unknown_fields)]
pub enum HoverSound {
    /// A counter from the screen's opening, every tick the hover runs,
    /// after its ramp: the sound as it wraps to 0 every `every` ticks
    /// unless the hover is clear (EXE5's 0x08025A80).
    FromOpening { every: u8 },
    /// A counter from each turn of the hover, before it: on the tick
    /// after the shade settles and every `every` ticks the shade stays
    /// (EXE4's +0x5C, 0x0801E49C).
    WhileDark { every: u8 },
}

/// What becomes of a pick of a chip that counts as the invalid chip
/// (`getChipID_802A54E`, EXE4's 0x08020306).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvalidPicks {
    /// Picked, as the invalid chip (EXE6's `sub_8028CCC`); the selection
    /// counts each chip as checked.
    Picked,
    /// Refused (EXE4's 0x0801F73C: the refusal's sound, nothing picked);
    /// the selection counts each chip as it is, its own code (0x0801F9E0).
    Refused,
}

/// How a selection treats the codes outside the alphabet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpecialCodes {
    /// 0x1B and 0x1C stand apart (EXE6's `sub_8028E4C`): a pick of one
    /// constrains no other code, and a chip of one goes with a selection
    /// that has none of them, or the same one.
    Apart,
    /// 0x1B is a code of its own that `*` doesn't stand for (EXE4's
    /// 0x0801F9E0, the dark chips' code): a chip of it goes with a selection
    /// of it alone, and a pick of it takes only chips of it (or the same
    /// chip). 0x1C is a code like the others.
    Unstarred,
}

/// How OK makes a Program Advance of a selection (EXE6's `sub_8029520`,
/// EXE4's 0x0801F390).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProgramAdvanceRules {
    /// Each forms once a round (EXE6's `sub_8029652`; EXE4 keeps no
    /// record: one forms whenever its recipe is picked).
    pub once_a_round: bool,
    /// The Program Advance is the Regular chip if one of its parts was
    /// (EXE6's `sub_80292CC`; EXE4's 0x0801F404 clears the flags).
    pub keeps_regular: bool,
    /// The entries past the selection's end, once the recipe's parts are
    /// taken out, are cleared (EXE4's 0x0801F438: no chip, no damage); else
    /// they keep what they held (EXE6's).
    pub clears_past_end: bool,
}

/// Until when a player's console says its custom screen is open.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusUntil {
    /// The window starts sliding out (EXE6's `sub_8026BF4`: status value 4);
    /// no other bit says the selection runs.
    Closing,
    /// The result is sent (EXE4: value 4 from the screen's opening,
    /// 0x08007618, to the send, 0x0801E986); and value 1 says the selection
    /// runs, from its first tick (0x08020348) to OK (0x08020652:
    /// `custom::Side::selecting`). The fight reads the two together.
    Sending,
}

/// The characters of a message that move its speaker's mouth as they
/// print, as the game's chatbox tests each (EXE6's `chatbox_8040C44`:
/// ranges of its charmap do, the rest don't; EXE5's 0x0803F7CC: a range
/// and two characters don't, the rest do). A game states one of the two;
/// with neither none does. Presentation.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TalkingCharacters {
    /// These, and no others.
    #[serde(default)]
    pub only: Option<String>,
    /// Every character but these.
    #[serde(default)]
    pub all_but: Option<String>,
}

impl TalkingCharacters {
    /// Whether `c` moves the mouth.
    pub fn talks(&self, c: char) -> bool {
        match (&self.only, &self.all_but) {
            (Some(only), _) => only.contains(c),
            (None, Some(all_but)) => !all_but.contains(c),
            (None, None) => false,
        }
    }
}

impl Default for CustomScreenLayout {
    fn default() -> CustomScreenLayout {
        let slot = SlotLayout { kind: TemplateSlot::Hidden, moves: SlotMoves::Neighbors { vertical: 0, left: 0, right: 0 } };
        CustomScreenLayout {
            slots: std::array::from_fn(|_| slot.clone()),
            left_scan_top: Vec::new(),
            left_scan_bottom: Vec::new(),
            right_scan_top: Vec::new(),
            right_scan_bottom: Vec::new(),
            left_scan_start: [0; 12],
            right_scan_start: [0; 12],
            redeal_kept: Vec::new(),
            emblem_at_window_return: false,
            chatbox_commands_wait_for_text: false,
            talking_characters: TalkingCharacters::default(),
            first_choosing_tick_reads_keys: true,
            run_message_at_key: false,
            invalid_picks: InvalidPicks::Picked,
            special_codes: SpecialCodes::Apart,
            program_advances: ProgramAdvanceRules { once_a_round: true, keeps_regular: true, clears_past_end: false },
            modifier_passes_regular: false,
            status_until: StatusUntil::Closing,
            hover: HoverRules {
                runs: HoverRuns::EveryTick,
                to_dark: Vec::new(),
                to_clear: Vec::new(),
                players: [31, 22],
                sound: HoverSound::FromOpening { every: 64 },
            },
            restore_players: vec![31, 22],
            gauge_empties_at_open: true,
            fades_clear_at_ok: false,
            cursor_after_leaving: true,
            frame_counts_first: false,
            description_in_choosing: false,
        }
    }
}

/// How a Program Advance is spelled in a selection.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaRecipe {
    /// `count` of one chip (by key) with consecutive codes (`sub_80295C8`).
    CodeRun { chip: String, count: u8 },
    /// These chips (by key) in this order, whatever their codes
    /// (`sub_802961A`).
    Sequence(Vec<String>),
}

impl PaRecipe {
    /// How many chips the recipe takes.
    pub fn len(&self) -> usize {
        match self {
            PaRecipe::CodeRun { count, .. } => *count as usize,
            PaRecipe::Sequence(ids) => ids.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A Program Advance: the chip a recipe turns into, and the recipe, by
/// chip handles.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ProgramAdvance {
    pub result: ChipHandle,
    pub recipe: Recipe,
    /// Tried only with battle flag 0x40 (see
    /// [`ProgramAdvanceRecipe::operation_battle_only`]).
    pub operation_battle_only: bool,
}

/// A recipe by chip handles (see [`PaRecipe`]).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Recipe {
    /// `count` of one chip with consecutive codes.
    CodeRun { chip: ChipHandle, count: u8 },
    /// These chips in this order, whatever their codes.
    Sequence(Vec<ChipHandle>),
}

impl Recipe {
    /// How many chips the recipe takes.
    pub fn len(&self) -> usize {
        match self {
            Recipe::CodeRun { count, .. } => *count as usize,
            Recipe::Sequence(ids) => ids.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A recipe for the chip that holds it. In a content file,
/// `{ order = N, sequence = [...] }` or `{ order = N, code_run = { chip, count } }`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProgramAdvanceRecipe {
    /// Where the recipe is tried among all the Program Advances (the
    /// original's table order): the first that matches wins.
    pub order: u8,
    /// Tried only in a battle where each side keeps its own gauge (battle
    /// flag 0x40, never set in a netbattle): EXE5's 21 recipes its full
    /// table (0x08027FC8) has before the netbattles' (0x0802801C, the rest
    /// of it; 0x080251DC picks the table by `sub_800A8F8`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub operation_battle_only: bool,
    #[serde(flatten)]
    pub recipe: PaRecipe,
}

/// What a modifier chip does to the chip picked before it (`sub_8029224`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChipModifier {
    /// Adds its damage to a damaging chip's attack bonus (Atk+10, Atk+30).
    AttackPlus,
    /// Adds its damage to a navi chip's attack bonus (Navi+20).
    NaviPlus,
    /// Makes a damaging chip paralyze (WhiCapsl).
    Paralyze,
    /// Makes a damaging chip that isn't a dimming chip uninstall (Uninstll).
    Uninstall,
}

/// A chip (by key) with its code (a link navi's own chip).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodedChip {
    pub chip: String,
    pub code: ChipCode,
}
