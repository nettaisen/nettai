//! What a battle asked the presentation for and the pack didn't have: a
//! sprite, an animation or a palette an object names, a chip without a
//! name or an icon, a banner without glyphs, a sound without a song.
//!
//! Drawing skips what it can't find, so nothing here stops a frame; the
//! renderer and the audio check note each case in [`Problems`]. Every
//! lookup the drawing code makes of the packs and the content goes through
//! [`crate::lookups`] (the audio's, a cue's song, through nettai-frontend's
//! `sound_lookups`), which notes it as a [`Lookup`] and checks it once a
//! run. Two audits, nettai-frontend's, use them:
//!
//! - `--audit-content` (nettai-frontend's `content_audit`) makes every
//!   lookup the content can: every chip's icon, picture and name, every
//!   navi's and form's face, every asset the content names, in both
//!   languages;
//! - `--audit` runs traces and makes the lookups their frames make (only
//!   those: nothing is drawn, `Renderer::set_lookups_only`), the asset's
//!   animation and palette an object asks for among them, and lists each
//!   trace's lookups for the verification's trace cover (`--lookups`).
//!
//! An empty list says the frames were drawn and the cues played with
//! everything they named, not that they look or sound like the original
//! (the frame comparison outside this repository checks that).

use nettai_battle::content::{BackgroundId, BannerId, Content, MugshotId, PackId, SpriteId};
use nettai_battle::custom::GameVersion;
use nettai_content_api::{AssetKind, ChipHandle, FormHandle, NaviHandle};
use std::collections::{BTreeMap, HashSet};

/// A lookup the drawing code or the audio makes of the packs or the
/// content: what a frame or a cue named, by what it is for. Each is
/// checked once a run ([`Problems::lookup`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Lookup {
    /// A sprite's sheet, in its pack.
    Sprite(SpriteId),
    /// A sprite's animation, with its frames.
    Animation(SpriteId, u8),
    /// A palette of a sprite's palette set: (set, index).
    Palette(SpriteId, u8, u8),
    /// A background, in its pack.
    Background(BackgroundId),
    /// A pack's field's panel block for a panel type (the engine's
    /// number), owner (0 the viewer's) and row (1..=3).
    Panel(PackId, u8, u8, u8),
    /// A panel type (the engine's number), or a highlight (16 + its number),
    /// that no loaded pack's field draws in an arena of a pack's: a tinted
    /// normal panel (`stage::FieldArt`).
    PanelTint(PackId, u8),
    /// A chip's icon (over the navi, on the custom screen's slots).
    ChipIcon(ChipHandle),
    /// A chip's picture in the chip window.
    ChipArt(ChipHandle),
    /// A chip's name in the 8x16 font (the next chip, the chip window, a
    /// telop).
    ChipName(ChipHandle),
    /// The chip window's colors and pictures of a chip's class, element
    /// and code.
    ChipWindow(ChipHandle),
    /// A chip's name and code in the Program Advance animation.
    AdvanceName(ChipHandle),
    /// A chip's description in the dialogue font (R on the custom screen).
    ChipDescription(ChipHandle),
    /// A mugshot, in its pack's HUD.
    Mugshot(MugshotId),
    /// A link navi's own face in the emotion window.
    NaviFace(NaviHandle),
    /// A form's face for an emotion (`emotion_number`).
    FormFace(FormHandle, u8),
    /// A navi's name on the custom screen (the enemy names).
    NaviName(NaviHandle),
    /// A navi's variant name there (`battle.set_name_variant`: BN5's Hub
    /// Style).
    NaviVariantName(NaviHandle),
    /// A navi's number in BN6's compat (its emblem's).
    NaviNumber(NaviHandle),
    /// A navi's emblem on a console of a game's custom screen.
    Emblem(NaviHandle, GameVersion),
    /// A navi's no-running message in the dialogue font, with its portrait.
    RunMessage(NaviHandle),
    /// A Cross's name and colors in the Cross window.
    CrossName(FormHandle),
    /// A Cross's description in the dialogue font.
    CrossDescription(FormHandle),
    /// A banner's glyphs.
    Banner(BannerId),
    /// A telop's place (its banner's layout).
    Telop(BannerId),
    /// A telop of a chip the engine wasn't told.
    TelopUnknown,
    /// A line of the HUD's text (`Hud::texts`).
    TextLine(u8),
    /// A sound's song, in its pack's sound.
    Sound(u16),
    /// A part of the pack's graphics that is there or not as a whole.
    Graphics(Graphics),
}

/// The parts of a pack's graphics a frame needs as a whole.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Graphics {
    /// The custom screen's (`graphics/custom`).
    CustomScreen,
    /// The chatbox's and the dialogue font.
    Chatbox,
    /// The warning marker.
    Warning,
}

/// An emotion's number in a [`Lookup::FormFace`] (`sub_8015B54`'s code;
/// BN5's worried, its own 1, 6; 0x10 more for the form's second set).
pub fn emotion_number(e: nettai_battle::kinds::player::Emotion) -> u8 {
    use nettai_battle::kinds::player::Emotion;
    match e {
        Emotion::Normal => 0,
        Emotion::Tired => 1,
        Emotion::FullSynchro => 2,
        Emotion::Angry => 3,
        Emotion::WornOut => 5,
        Emotion::Worried => 6,
    }
}

impl Lookup {
    /// The lookup as a line of text, by the content's names (`--lookups`:
    /// stable from run to run, and between content of other packs).
    pub fn describe(&self, c: &Content) -> String {
        let asset = |kind: AssetKind, h: u16| crate::packs::name(c, kind, h).map_or(format!("#{h}"), str::to_string);
        let sprite = |id: SpriteId| asset(AssetKind::Sprite, id.0);
        let chip = |h: ChipHandle| &c.defs.chip(h).key;
        let navi = |h: NaviHandle| &c.defs.navi(h).key;
        let form = |h: FormHandle| &c.defs.form(h).key;
        let pack_name = |p: PackId| c.assets.packs.get(p.index()).map_or(format!("pack {}", p.index()), String::clone);
        match *self {
            Lookup::Sprite(id) => format!("sprite {}", sprite(id)),
            Lookup::Animation(id, anim) => format!("sprite {} animation {anim}", sprite(id)),
            Lookup::Palette(id, set, index) => format!("sprite {} palette {set}/{index}", sprite(id)),
            Lookup::Background(id) => format!("background {}", asset(AssetKind::Background, id.0)),
            Lookup::Panel(pack, kind, owner, row) => format!("panel {kind} owner {owner} row {row} in {}", pack_name(pack)),
            Lookup::PanelTint(arena, kind) => format!("panel {kind} tinted in an arena of {}", pack_name(arena)),
            Lookup::ChipIcon(h) => format!("chip {} icon", chip(h)),
            Lookup::ChipArt(h) => format!("chip {} picture", chip(h)),
            Lookup::ChipName(h) => format!("chip {} name", chip(h)),
            Lookup::ChipWindow(h) => format!("chip {} window", chip(h)),
            Lookup::AdvanceName(h) => format!("chip {} advance name", chip(h)),
            Lookup::ChipDescription(h) => format!("chip {} description", chip(h)),
            Lookup::Mugshot(id) => format!("mugshot {}", asset(AssetKind::Mugshot, id.0)),
            Lookup::NaviFace(h) => format!("navi {} face", navi(h)),
            Lookup::FormFace(h, e) => format!("form {} face {e}", form(h)),
            Lookup::NaviName(h) => format!("navi {} name", navi(h)),
            Lookup::NaviVariantName(h) => format!("navi {} variant name", navi(h)),
            Lookup::NaviNumber(h) => format!("navi {} number", navi(h)),
            Lookup::Emblem(h, v) => format!("navi {} emblem {}", navi(h), crate::custom::game_name(v)),
            Lookup::RunMessage(h) => format!("navi {} run message", navi(h)),
            Lookup::CrossName(h) => format!("form {} cross name", form(h)),
            Lookup::CrossDescription(h) => format!("form {} description", form(h)),
            Lookup::Banner(id) => format!("banner {}", asset(AssetKind::Banner, id.0)),
            Lookup::Telop(id) => format!("telop {}", asset(AssetKind::Banner, id.0)),
            Lookup::TelopUnknown => "telop of an untold chip".into(),
            Lookup::TextLine(n) => format!("text line {n}"),
            Lookup::Sound(h) => format!("sound {}", asset(AssetKind::Sound, h)),
            Lookup::Graphics(g) => format!("graphics {g:?}"),
        }
    }
}

/// How often a problem was seen, and on which frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seen {
    pub count: u32,
    /// The first and last frames it was seen on (the driver's frame
    /// numbers; none when it has none).
    pub first: Option<u32>,
    pub last: Option<u32>,
}

/// The problems seen so far, each once, in the order of their text; and
/// the lookups made so far.
#[derive(Clone, Debug, Default)]
pub struct Problems {
    frame: Option<u32>,
    seen: BTreeMap<String, Seen>,
    /// The last frame's places where the frontend draws something else
    /// than the original on purpose (`Known`).
    pub known: Vec<Known>,
    /// Every distinct lookup made so far, each checked once.
    lookups: HashSet<Lookup>,
    /// What is drawn otherwise than the packs would, by design, and said,
    /// not counted (a panel no pack draws, drawn tinted).
    said: BTreeMap<String, Seen>,
}

/// A place of a frame where the frontend differs from the original on
/// purpose: a rectangle and why. The frame comparison leaves it out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Known {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub why: &'static str,
}

impl Problems {
    /// Note a place the frontend draws differently on purpose this frame.
    pub fn known(&mut self, x: i32, y: i32, width: i32, height: i32, why: &'static str) {
        self.known.push(Known { x, y, width, height, why });
    }

    /// The frame the notes that follow belong to.
    pub fn at(&mut self, frame: Option<u32>) {
        self.frame = frame;
    }

    /// Note a problem (the text names what is missing and who asked).
    pub fn note(&mut self, what: String) {
        let frame = self.frame;
        let s = self.seen.entry(what).or_insert(Seen { count: 0, first: frame, last: frame });
        s.count += 1;
        s.last = frame;
    }

    /// Note what is drawn otherwise by design: said, not counted as a
    /// problem.
    pub fn say(&mut self, what: String) {
        let frame = self.frame;
        let s = self.said.entry(what).or_insert(Seen { count: 0, first: frame, last: frame });
        s.count += 1;
        s.last = frame;
    }

    /// What was said ([`Problems::say`]), as [`Problems::lines`] says the
    /// problems.
    pub fn said_lines(&self) -> Vec<String> {
        lines(&self.said)
    }

    /// Note a lookup: true the first time it is made this run, when the
    /// caller checks it (a lookup is checked once a run).
    pub fn lookup(&mut self, l: Lookup) -> bool {
        self.lookups.insert(l)
    }

    /// The distinct lookups made so far.
    pub fn lookups(&self) -> impl Iterator<Item = Lookup> + '_ {
        self.lookups.iter().copied()
    }

    pub fn is_empty(&self) -> bool {
        self.seen.is_empty()
    }

    pub fn len(&self) -> usize {
        self.seen.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, Seen)> {
        self.seen.iter().map(|(k, &v)| (k.as_str(), v))
    }

    /// Forget everything (a new run).
    pub fn clear(&mut self) {
        *self = Problems::default();
    }

    /// One line per problem: the text, how often, and the frames.
    pub fn lines(&self) -> Vec<String> {
        lines(&self.seen)
    }
}

/// One line per entry: the text, how often, and the frames.
fn lines(seen: &BTreeMap<String, Seen>) -> Vec<String> {
    seen.iter()
        .map(|(what, s)| match (s.first, s.last) {
            (Some(a), Some(b)) if a != b => format!("{what} ({} times, frames {a}..={b})", s.count),
            (Some(a), _) => format!("{what} (frame {a})"),
            _ if s.count == 1 => format!("{what} (once)"),
            _ => format!("{what} ({} times)", s.count),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn problems_are_counted_once_each() {
        let mut p = Problems::default();
        assert!(p.is_empty());
        p.at(Some(10));
        p.note("no sprite".into());
        p.at(Some(12));
        p.note("no sprite".into());
        p.note("no icon".into());
        assert_eq!(p.len(), 2);
        assert_eq!(p.lines(), ["no icon (frame 12)", "no sprite (2 times, frames 10..=12)"]);
    }
}
