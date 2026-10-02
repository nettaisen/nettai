//! The chatbox, as far as the custom screen waits on it: a chip's or a
//! Cross's description (R) and the no-running message (L).
//!
//! The original runs a text script through its chatbox (`chatbox_onUpdate`,
//! once a frame after the battle's update) and the screen waits for the
//! chatbox to close. What the scripts say is presentation; when the box
//! closes isn't, because the screen reads no keys until then. This is the
//! interpreter's timing for the commands those scripts use:
//!
//! - the box: open at once (`E8 06`, the descriptions') or in three steps
//!   (`E8 00`), then closing in three;
//! - the operator's portrait (`F5`): the box's opening waits for its
//!   fade-in (seven ticks once the box is open), the close for its
//!   fade-out (three);
//! - text: at print speed 0 a whole line a tick; otherwise a character
//!   every `speed + 1` ticks, and at once from the tick B is held or A is
//!   pressed (`chatbox_8040154`), which the box only looks for once it has
//!   run four ticks without waiting on a command;
//! - a line break (`E9`), which ends the tick's printing;
//! - the wait for a key (`E7`): six ticks before it takes one, then A or B
//!   (or any key) pressed, or B held for eleven ticks;
//! - the end (`E6`), which closes the box.
//!
//! Verified against chip-lab recordings (docs/engine/custom-screen.md
//! §3.5): the tick a description takes keys from by its line breaks, a
//! held B, and the no-running message with A pressed on every other frame.
//!
//! What the box shows (its opening steps, the text printed so far, the
//! portrait's face and tint, the key-wait arrow) is read through the
//! accessors and [`ChatboxLook`], which the timing never reads and the
//! state digest leaves out (a frontend draws it: nettai-frontend
//! `chatbox`).

use crate::input::keys;

/// What a script does that takes time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Script {
    /// A chip's or a Cross's description: the box at once, the text at
    /// print speed 0 with `breaks` line breaks, then any key.
    Description { breaks: u8 },
    /// The no-running message: the operator's portrait, the box opening,
    /// text at the default speed in lines of this many characters (a line
    /// after the first with none isn't there), then A or B.
    RunMessage { lines: [u8; 3] },
}

/// One command of a script, as the timing sees it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Op {
    /// `F5`: the portrait, fading in.
    Portrait,
    /// `E8 00`: the box opens.
    Open,
    /// `E8 06`: the box is open.
    OpenAtOnce,
    /// `F1`: the print speed.
    Speed(u8),
    /// Characters to print.
    Text(u8),
    /// `E9`.
    Break,
    /// `E7`: wait for a key (`any`: any key, else A or B).
    Halt { any: bool },
    /// `E6`.
    End,
}

impl Script {
    /// The script's `i`th command.
    fn op(self, i: u8) -> Op {
        match self {
            Script::Description { breaks } => match i {
                0 => Op::OpenAtOnce,
                1 => Op::Speed(0),
                // A line, then a break and a line for each break.
                i if i <= 2 + 2 * breaks => {
                    if i % 2 == 0 {
                        Op::Text(1)
                    } else {
                        Op::Break
                    }
                }
                i if i == 3 + 2 * breaks => Op::Halt { any: true },
                _ => Op::End,
            },
            Script::RunMessage { lines } => {
                let count = 1 + lines[1..].iter().take_while(|&&n| n != 0).count() as u8;
                match i {
                    0 => Op::Portrait,
                    1 => Op::Open,
                    i if i < 2 + 2 * count - 1 => {
                        if i % 2 == 0 {
                            Op::Text(lines[(i as usize - 2) / 2])
                        } else {
                            Op::Break
                        }
                    }
                    i if i == 2 + 2 * count - 1 => Op::Halt { any: false },
                    _ => Op::End,
                }
            }
        }
    }
}

/// The portrait's tint as it fades in (`0x18C6`, one 0x421 a tick) and
/// out (0x842 a tick until a channel reaches 6).
const TINT: u16 = 0x18C6;
const FADE_IN: u16 = 0x421;
const FADE_OUT: u16 = 0x842;
/// The print speed a script starts with: a character every third tick.
const SPEED: u8 = 2;
/// Ticks the box runs before it looks for the keys that rush the text
/// (ticks spent waiting on a command don't count).
const RUSH_DELAY: u8 = 4;
/// The wait for a key: ticks before it takes one, and the ticks of B held
/// that answer it.
const HALT_DELAY: u16 = 5;
const HELD_TICKS: u16 = 10;
/// The box's opening steps.
const OPEN: u8 = 3;
/// Every button.
const ANY_KEY: u16 = 0x3FF;

/// What the chatbox shows that its timing never reads (presentation; the
/// state digest leaves it out).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChatboxLook {
    /// Which of the message's characters move the speaker's mouth, by
    /// line (bit k: the line's character k).
    talking: [u32; 3],
    face: Face,
    /// The portrait as drawn this tick, if it is.
    pub portrait: Option<PortraitLook>,
    /// The key-wait arrow's frame (0-2) this tick, if it is drawn, and
    /// its animation's step (`+0x17`).
    pub arrow: Option<u8>,
    arrow_step: u8,
    /// What the text's sprites hold: the lines done and the characters of
    /// the one printing, as the line buffer was when last copied to them
    /// (`sub_30070B4`); none, blank.
    pub text: Option<(u8, u8)>,
    /// The buffer was cleared (the end's `chatbox_8045F60`), the wait for
    /// a key has run (flag 0x400), and the sprites keep their tiles (`+0x3D`
    /// is 2: the copy that follows a wait's start sets it, a tick printing
    /// all at once sets it back).
    cleared: bool,
    halted: bool,
    kept: bool,
}

impl std::hash::Hash for ChatboxLook {
    /// Presentation: left out of the state digest.
    fn hash<H: std::hash::Hasher>(&self, _: &mut H) {}
}

/// The portrait as a tick draws it (`chatbox_8040B8C`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PortraitLook {
    /// The portrait sprite's animation (its faces: 0 still, 1 idle, 2
    /// talking) and the sprite updates since it was set (the first on
    /// the tick it is set).
    pub anim: u8,
    pub updates: u16,
    /// The colour added to its palette (BGR555 per channel, saturating):
    /// the tint before this tick's step of the fade.
    pub tint: u16,
}

/// The speaker's face (`+0x1F0`..`+0x1F3`): the animation the sprite
/// takes next, whether the last character talked, and what the sprite
/// last took.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Face {
    anim: u8,
    talking: bool,
    shown: bool,
    /// The sprite's animation and its updates.
    sprite_anim: u8,
    updates: u16,
}

impl Default for Face {
    fn default() -> Face {
        Face { anim: FACE_TALKING, talking: false, shown: false, sprite_anim: 0, updates: 0 }
    }
}

/// The portrait's animations: still (as `F5` loads it), idle (after
/// talking), talking; one the faces keep (`3`).
const FACE_IDLE: u8 = 1;
const FACE_TALKING: u8 = 2;
const FACE_KEPT: u8 = 3;

impl Face {
    /// `chatbox_8040C44`: a character printed.
    fn said(&mut self, talks: bool) {
        self.talking = talks;
        if talks {
            self.anim = FACE_TALKING;
        } else if self.anim != FACE_KEPT {
            self.anim = FACE_IDLE;
        }
    }

    /// `chatbox_8040C9C`: after a command (or a character all at once).
    fn after_command(&mut self) {
        if !matches!(self.anim, 0 | FACE_KEPT) {
            self.talking = false;
            self.anim = FACE_IDLE;
        }
    }
}

/// The arrow's frames by its step (`byte_80408A4`); past the last the
/// step goes back to 1.
const ARROW_FRAMES: [u8; 18] = [0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2];

/// A running chatbox.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Chatbox {
    script: Script,
    /// The command the script is at, and the characters of it already
    /// printed (a text command).
    at: u8,
    printed: u8,
    /// The box is up (the screen waits while it is).
    open: bool,
    /// A command holds the script (the box's flag 1).
    waiting: bool,
    /// The box isn't drawn yet, or no longer (flag 0x100).
    hidden: bool,
    /// The portrait: shown (flag 2), fading in (4), fading out (8), and
    /// its tint.
    portrait: bool,
    fading_in: bool,
    fading_out: bool,
    tint: u16,
    /// The print speed and the ticks until the next character.
    speed: u8,
    char_wait: u8,
    /// Ticks left before the rush keys are looked for.
    rush_delay: u8,
    /// The box's opening step (0 closed .. 3 open).
    steps: u8,
    /// A countdown the box's opening, its closing and the wait for a key
    /// share (the game's BoxY, with the halfword around it).
    count: u16,
    /// The wait for a key: 0 not begun, 1 its delay, 2 taking keys.
    halt: u8,
    /// What it shows (presentation).
    look: ChatboxLook,
}

impl Chatbox {
    /// `chatbox_runScript`.
    pub fn new(script: Script) -> Chatbox {
        Chatbox {
            script,
            at: 0,
            printed: 0,
            open: true,
            waiting: false,
            hidden: true,
            portrait: false,
            fading_in: false,
            fading_out: false,
            tint: 0,
            speed: SPEED,
            char_wait: 0,
            rush_delay: RUSH_DELAY,
            steps: 0,
            count: 1,
            halt: 0,
            look: ChatboxLook::default(),
        }
    }

    /// Which of the message's characters move the speaker's mouth, by line
    /// (presentation).
    pub fn talking(mut self, talking: [u32; 3]) -> Chatbox {
        self.look.talking = talking;
        self
    }

    /// Whether the box is still up.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// The script it runs.
    pub fn script(&self) -> Script {
        self.script
    }

    /// The box's opening step as drawn (`chatbox_CopyBackgroundTiles_8040344`:
    /// 0 to 3, open), or none while it isn't drawn.
    pub fn box_step(&self) -> Option<u8> {
        (self.open && !self.hidden).then_some(self.steps)
    }

    /// Whether the text and the portrait show: the box is fully open (or
    /// not drawn at all).
    pub fn shows_contents(&self) -> bool {
        self.open && (self.hidden || self.steps == OPEN)
    }

    /// The text in the line buffer: the lines done and the characters of
    /// the one printing (a description's lines print whole).
    fn printed_text(&self) -> (u8, u8) {
        let done = (0..self.at).filter(|&i| matches!(self.script.op(i), Op::Text(_))).count() as u8;
        let printing = if matches!(self.script.op(self.at), Op::Text(_)) { self.printed } else { 0 };
        (done, printing)
    }

    /// What it shows besides (presentation).
    pub fn look(&self) -> &ChatboxLook {
        &self.look
    }

    /// `chatbox_onUpdate`: one tick on the console's joypad.
    pub fn update(&mut self, held: u16, pressed: u16) {
        if !self.open {
            return;
        }
        let mut rush = false;
        if !self.waiting {
            if self.rush_delay != 0 {
                self.rush_delay -= 1;
            } else {
                // chatbox_8040154
                rush = held & keys::B != 0 || pressed & keys::A != 0 || self.speed == 0;
            }
        }
        if rush {
            self.look.kept = false;
            self.print_all(held, pressed);
        } else {
            self.print(held, pressed);
        }
        self.copy_text(rush);
        if self.open {
            self.fade_portrait();
        }
        self.draw_arrow();
    }

    /// `sub_30070B4`: while the text's sprites are drawn, the line buffer
    /// goes to their tiles, on a tick printing all at once always, else
    /// unless they keep what they have; after the wait for a key has run
    /// they keep it from then on (until a tick prints all at once).
    fn copy_text(&mut self, rush: bool) {
        if !self.shows_contents() || !(rush || !self.look.kept) {
            return;
        }
        self.look.text = (!self.look.cleared).then(|| self.printed_text());
        if self.look.halted {
            self.look.kept = true;
        }
    }

    /// `chatbox_804082C`: the key-wait arrow, while the script waits for a
    /// key (`E7` sets its flag each tick, the key clears it).
    fn draw_arrow(&mut self) {
        let look = &mut self.look;
        look.arrow = None;
        if self.open && self.halt != 0 {
            look.arrow = Some(ARROW_FRAMES[look.arrow_step as usize]);
            look.arrow_step = if look.arrow_step as usize + 1 >= ARROW_FRAMES.len() { 1 } else { look.arrow_step + 1 };
        }
    }

    /// The line a text command prints (the commands before the first
    /// text are two).
    fn line(&self) -> usize {
        (self.at as usize).saturating_sub(2) / 2
    }

    /// `chatbox_interpreteAndDrawDialogChar`: commands and characters until
    /// one takes the rest of the tick.
    fn print(&mut self, held: u16, pressed: u16) {
        loop {
            let goes_on = match self.script.op(self.at) {
                Op::Text(n) => {
                    self.waiting = false;
                    if self.char_wait == 0 {
                        self.char_wait = self.speed;
                        let talks = self.look.talking.get(self.line()).is_some_and(|t| t >> self.printed.min(31) & 1 != 0);
                        self.look.face.said(talks);
                        self.printed += 1;
                        if self.printed >= n {
                            self.next();
                        }
                        true
                    } else {
                        self.char_wait -= 1;
                        false
                    }
                }
                // A command waits out the last character too.
                _ if self.char_wait != 0 => {
                    self.char_wait -= 1;
                    false
                }
                _ => {
                    let goes_on = self.command(held, pressed);
                    self.look.face.after_command();
                    goes_on
                }
            };
            if !goes_on || !self.open {
                break;
            }
        }
    }

    /// `chatbox_interpreteAndDrawDialogChar_1`: everything at once, up to
    /// a command that holds the script.
    fn print_all(&mut self, held: u16, pressed: u16) {
        loop {
            match self.script.op(self.at) {
                Op::Text(_) => self.next(),
                _ => {
                    self.command(held, pressed);
                }
            }
            // (Characters printed at once, and the commands, leave the
            // face idle.)
            self.look.face.after_command();
            if !self.open || self.waiting {
                break;
            }
        }
    }

    fn next(&mut self) {
        self.at += 1;
        self.printed = 0;
    }

    /// One command; whether the script goes on this tick.
    fn command(&mut self, held: u16, pressed: u16) -> bool {
        match self.script.op(self.at) {
            Op::Text(_) => unreachable!("text is printed, not run"),
            // chatbox_F5_mugshot
            Op::Portrait => {
                if !self.portrait {
                    self.portrait = true;
                    self.fading_in = true;
                    self.tint = TINT;
                }
                // sub_8040B3A: the sprite starts still.
                let face = &mut self.look.face;
                (face.sprite_anim, face.updates) = (0, 0);
                self.next();
                true
            }
            // chatbox_804103E
            Op::Open => {
                self.waiting = true;
                if self.steps != OPEN {
                    self.hidden = false;
                    if self.count & 0xFF != 0 {
                        self.count -= 1;
                        return false;
                    }
                    self.steps += 1;
                    if self.steps != OPEN {
                        self.count &= 0xFF00;
                        return false;
                    }
                }
                if self.fading_in {
                    return false;
                }
                self.waiting = false;
                self.next();
                true
            }
            // chatbox_80410F8
            Op::OpenAtOnce => {
                self.hidden = false;
                self.waiting = false;
                self.steps = OPEN;
                self.next();
                true
            }
            // chatbox_F1_textspeed
            Op::Speed(speed) => {
                self.speed = speed;
                self.next();
                true
            }
            // chatbox_E9_newline
            Op::Break => {
                self.next();
                false
            }
            // chatbox_E7_buttonhalt
            Op::Halt { any } => {
                self.waiting = true;
                self.look.halted = true;
                match self.halt {
                    0 => {
                        self.count = HALT_DELAY;
                        self.halt = 1;
                        return false;
                    }
                    1 if self.count != 0 => {
                        self.count -= 1;
                        return false;
                    }
                    _ => self.halt = 2,
                }
                let mask = if any { ANY_KEY } else { keys::A | keys::B };
                let answered = if pressed & mask != 0 {
                    true
                } else if held & keys::B != 0 {
                    if self.count >= HELD_TICKS {
                        true
                    } else {
                        self.count += 1;
                        false
                    }
                } else {
                    false
                };
                if answered {
                    self.waiting = false;
                    self.halt = 0;
                    self.count = 0;
                    self.next();
                }
                false
            }
            // chatbox_E6_end
            Op::End => {
                self.waiting = true;
                if !self.hidden {
                    // chatbox_8041090: the text is cleared, the portrait
                    // fades out, then the box closes step by step.
                    self.look.cleared = true;
                    self.fading_out = true;
                    if self.portrait {
                        return false;
                    }
                    if self.count & 0xFF != 0 {
                        self.count -= 1;
                        return false;
                    }
                    if self.steps != 0 {
                        self.steps -= 1;
                        self.count &= 0xFF00;
                        return false;
                    }
                    self.count &= 0xFF00;
                    self.fading_out = false;
                    self.waiting = false;
                    self.hidden = true;
                } else if self.portrait {
                    self.fading_out = true;
                    return false;
                }
                self.open = false;
                self.halt = 0;
                false
            }
        }
    }

    /// `chatbox_8040B8C`: the portrait's fade, a step a tick while the box
    /// is fully open (or not drawn).
    fn fade_portrait(&mut self) {
        self.look.portrait = None;
        if !self.portrait || (!self.hidden && self.steps != OPEN) {
            return;
        }
        // It is drawn with the tint before this step, its face changed
        // when the talking did, its sprite updated.
        let tint = self.tint;
        let face = &mut self.look.face;
        if face.talking != face.shown {
            face.shown = face.talking;
            (face.sprite_anim, face.updates) = (face.anim, 0);
        }
        face.updates = face.updates.saturating_add(1);
        self.look.portrait = Some(PortraitLook { anim: face.sprite_anim, updates: face.updates, tint });
        if self.fading_in {
            match self.tint.checked_sub(FADE_IN) {
                Some(t) => self.tint = t,
                None => {
                    self.tint = 0;
                    self.fading_in = false;
                }
            }
        } else if self.fading_out {
            self.tint = self.tint.wrapping_add(FADE_OUT);
            if self.tint & 0x1F >= 6 {
                self.fading_out = false;
                self.portrait = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tick (0 = the script's first) the box closes on, with the keys
    /// `keys(tick)` gives as (held, pressed).
    fn closes(script: Script, keys: impl Fn(u32) -> (u16, u16)) -> Option<u32> {
        let mut c = Chatbox::new(script);
        (0..600).find(|&t| {
            let (held, pressed) = keys(t);
            c.update(held, pressed);
            !c.is_open()
        })
    }

    fn press(key: u16, at: u32) -> impl Fn(u32) -> (u16, u16) {
        move |t| if t == at { (key, key) } else { (0, 0) }
    }

    #[test]
    fn a_description_takes_keys_six_ticks_in_and_one_more_a_line_break() {
        for breaks in 0..3u8 {
            let first = 6 + breaks as u32;
            let script = Script::Description { breaks };
            // A key before that is lost; the first one taken closes the
            // box four ticks later.
            assert_eq!(closes(script, press(keys::A, first - 1)), None, "{breaks} breaks");
            assert_eq!(closes(script, press(keys::A, first)), Some(first + 4), "{breaks} breaks");
            assert_eq!(closes(script, press(keys::SELECT, first + 9)), Some(first + 13));
        }
    }

    #[test]
    fn b_held_closes_a_description_on_its_eleventh_tick() {
        let script = Script::Description { breaks: 2 };
        // Held from before the box takes keys (its press isn't seen): the
        // hold is counted from tick 8, and answers on its 11th tick.
        let held = |until: u32| move |t: u32| if (1..until).contains(&t) { (keys::B, if t == 1 { keys::B } else { 0 }) } else { (0, 0) };
        assert_eq!(closes(script, held(18)), None);
        assert_eq!(closes(script, held(19)), Some(22));
        // Held ticks count whether or not they are in a row.
        let twice = |t: u32| if (1..14).contains(&t) || (20..25).contains(&t) { (keys::B, if t == 1 || t == 20 { keys::B } else { 0 }) } else { (0, 0) };
        // (The second press is itself a key: it answers at once.)
        assert_eq!(closes(script, twice), Some(20 + 4));
    }

    #[test]
    fn the_no_running_message_prints_a_character_every_other_tick() {
        let megaman = Script::RunMessage { lines: [19, 12, 0] };
        // The box opens over ticks 0-3, the portrait fades in over 3-9,
        // the text starts on tick 10: 31 characters two ticks apart and a
        // line break, then the wait for A or B, which takes a key from
        // tick 79. The portrait fades out and the box closes in 7 ticks.
        assert_eq!(closes(megaman, press(keys::A, 78)), None);
        assert_eq!(closes(megaman, press(keys::A, 79)), Some(86));
        assert_eq!(closes(megaman, press(keys::B, 150)), Some(157));
        // Any other key doesn't answer it.
        assert_eq!(closes(megaman, press(keys::START, 150)), None);
        // One line of 20 characters.
        let one = Script::RunMessage { lines: [20, 0, 0] };
        assert_eq!(closes(one, press(keys::A, 55)), None);
        assert_eq!(closes(one, press(keys::A, 56)), Some(63));
    }

    #[test]
    fn the_message_shows_its_text_face_and_arrow_as_it_prints() {
        // "Lan,th": L, a, n talk, the comma doesn't.
        let mut c = Chatbox::new(Script::RunMessage { lines: [6, 0, 0] }).talking([0b11_0111, 0, 0]);
        let mut shown = Vec::new();
        for _ in 0..40 {
            c.update(0, 0);
            shown.push((c.box_step(), c.look().text, c.look().portrait.map(|p| (p.anim, p.updates, p.tint)), c.look().arrow));
        }
        // The box opens over ticks 0-3; the portrait shows from tick 3,
        // still and fading in by 0x421 a tick.
        assert_eq!(shown[0].0, Some(0));
        assert_eq!(shown[2].0, Some(2));
        assert_eq!(shown[3], (Some(3), Some((0, 0)), Some((0, 1, 0x18C6)), None));
        assert_eq!(shown[4].2, Some((0, 2, 0x18C6 - 0x421)));
        // The first character on tick 10 sets the talking face.
        assert_eq!(shown[10].1, Some((0, 1)));
        assert_eq!(shown[10].2.map(|p| (p.0, p.1)), Some((FACE_TALKING, 1)));
        assert_eq!(shown[11].2.map(|p| (p.0, p.1)), Some((FACE_TALKING, 2)));
        // The comma, three characters later, stops it.
        let comma = shown.iter().position(|s| s.1 == Some((0, 4))).unwrap();
        assert_eq!(shown[comma].2.map(|p| (p.0, p.1)), Some((FACE_IDLE, 1)));
        // Once printed, the arrow: frame 0 for six ticks, then 1.
        let arrow = shown.iter().position(|s| s.3.is_some()).unwrap();
        assert_eq!(shown[arrow].1, Some((1, 0)));
        assert_eq!(shown[arrow..arrow + 7].iter().map(|s| s.3.unwrap()).collect::<Vec<_>>(), [0, 0, 0, 0, 0, 0, 1]);
    }

    #[test]
    fn the_text_stays_through_the_portraits_fade_unless_printed_at_once() {
        let megaman = Script::RunMessage { lines: [19, 12, 0] };
        // A pressed: the end runs a tick later, and the sprites keep the
        // text while the portrait fades out.
        let mut c = Chatbox::new(megaman);
        let texts: Vec<_> = (0..90u32)
            .map(|t| {
                let k = if t == 79 { keys::A } else { 0 };
                c.update(k, k);
                (c.look().text, c.box_step())
            })
            .collect();
        assert_eq!(texts[80], (Some((2, 0)), Some(3)));
        assert_eq!(texts[82], (Some((2, 0)), Some(3)));
        // B held: the end runs on a tick that prints all at once, which
        // copies the cleared buffer.
        let mut c = Chatbox::new(megaman);
        let mut ended = None;
        for t in 0..90u32 {
            c.update(keys::B, if t == 0 { keys::B } else { 0 });
            if c.look().text.is_none() && t > 10 && ended.is_none() {
                ended = Some((t, c.box_step()));
            }
        }
        assert_eq!(ended.map(|e| e.1), Some(Some(3)));
    }

    #[test]
    fn a_or_held_b_rushes_the_message() {
        let megaman = Script::RunMessage { lines: [19, 12, 0] };
        // A on every even tick: the box first looks for it on tick 14 and
        // the text is all there at once; the wait takes the press on tick
        // 20, and the box closes on tick 27.
        let even = |t: u32| if t % 2 == 0 { (keys::A, keys::A) } else { (0, 0) };
        assert_eq!(closes(megaman, even), Some(27));
        // On every odd tick: tick 15 rushes it, a tick after a character
        // was printed, whose wait holds the key wait up one tick; it takes
        // keys from tick 22, the press on 23 answers it, closed on 30.
        let odd = |t: u32| if t % 2 == 1 { (keys::A, keys::A) } else { (0, 0) };
        assert_eq!(closes(megaman, odd), Some(30));
        // B held from the start rushes the text on tick 14 and answers the
        // wait on its eleventh tick of taking keys.
        let b = |t: u32| (keys::B, if t == 0 { keys::B } else { 0 });
        assert_eq!(closes(megaman, b), Some(37));
    }
}
