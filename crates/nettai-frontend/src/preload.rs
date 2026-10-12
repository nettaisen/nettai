//! What a match still waits for once its game is loaded, made ready before
//! its first tick, a step at a time: for a loading screen whose bars
//! advance and whose window stays live.
//!
//! **What there is to wait for.** A game loads whole ([`crate::game`]): its
//! content, every sprite, picture and sound of its pack. A match reads no
//! file ([`Late::reads`] counts any that were read; the example
//! `preload` plays one with the pack and the content deleted). What a
//! match's first tick did wait for, before this, was made on the way:
//!
//! - **the content's runtime on the thread that runs the battle**: every
//!   thread that makes or steps a battle keeps a Luau runtime of the
//!   content (`nettai_battle::behavior`), and loading one runs all of the
//!   game's scripts: about a second (EXE4 0.4 s, EXE5 0.8 s, EXE6 1.2 to
//!   1.4 s, measured in a release build on a loaded machine:
//!   docs/frontend.md §9), paid by the first battle made on the thread, or
//!   the first build checked there. [`runtime`] loads it ahead; it is the
//!   long step of a [`Preload`], and none at all once the thread has it (a
//!   host may load it behind its start-up bar, as soon as the game is
//!   loaded);
//! - **the round's first battle and its first picture**: about 15 ms;
//! - **the text**: in the font mode, a string is laid out and its glyphs
//!   drawn from the font's outlines the first time it shows (a few
//!   milliseconds on the frame a custom screen opens, at most 3.5 ms
//!   measured at three times the frame's size; [`Late::layouts`] and
//!   [`Late::glyphs`] count them). The first picture's are made here; the
//!   strings a fight goes on to show (a chip's name, a number) are made as
//!   they show, well inside a frame.
//!
//! **Each side's part** is its things looked up in the pack that is in
//! memory: its navi's face, emblem and name, its forms' faces and names,
//! each chip of its folder's icon, picture, name and description, by the
//! same lookups a frame draws through. They take microseconds; what they
//! give is [`Preload::missing`], said before the match starts, and a bar a
//! side.
//!
//! ```ignore
//! let mut preload = Preload::new(&game, &set, 0, (width, height));
//! while preload.step() {
//!     show(preload.fraction_of(Part::Side(0)), preload.fraction_of(Part::Side(1)), preload.fraction());
//! }
//! let mut player = preload.player(Box::new(LivePlayer::new(set)));
//! // ... and after the match: player.late() says what it still made on the way.
//! ```

use crate::driver::Driver;
use crate::game::Loaded;
use crate::player::Player;
use nettai_audio::BattleAudio;
use nettai_battle::Battle;
use nettai_battle::behavior::Behaviors;
use nettai_battle::content::{ChipCode, ChipRole, Content, Emotion};
use nettai_content_api::{ChipHandle, FormHandle, NaviHandle};
use nettai_match::Set;
use nettai_render::Renderer;
use nettai_render::audit::Lookup;
use nettai_render::lookups;
use nettai_render::strings::DisplayText;
use nettai_render::vfont::TextRenderer;
use std::sync::Arc;

/// Load `content`'s runtime on this thread, if the thread hasn't it: the
/// thread that will make and step the battles (and check builds). True
/// when it was loaded now (the long wait: `preload`'s module docs), false
/// when the thread had it. A content error is the same on every machine.
pub fn runtime(content: &Content) -> Result<bool, String> {
    let before = Behaviors::loads();
    Behaviors::for_content(content).map_err(|e| e.to_string())?;
    Ok(Behaviors::loads() != before)
}

/// What a player's ticks and pictures made on the way, because nothing had
/// made it ready ([`Player::late`]): the work a [`Preload`] does ahead. A
/// match played by a preloaded player on its thread has no runtime loaded
/// and nothing read.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Late {
    /// Content runtimes this thread loaded since the player was made
    /// (`Behaviors::loads`): each a wait of about a second.
    pub runtimes: u64,
    /// Parts of a pack or of the content this thread read from disk since
    /// (`nettai_content::pack::reads`).
    pub reads: u64,
    /// Strings its text was laid out for, and glyph images drawn from the
    /// font, each for the first time, since (the font mode's).
    pub layouts: u64,
    pub glyphs: u64,
}

/// Whose a step of a match's preloading is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Part {
    /// The match's own: the content's runtime on this thread, the round's
    /// first battle, its first picture (the arena, the background, both
    /// navis as they come in, the HUD) and the sound.
    Match,
    /// One side's: its navi, its navi's forms and each chip of its folder.
    Side(u8),
}

/// What a step makes ready.
#[derive(Clone, Copy)]
enum Work {
    Runtime,
    Battle,
    Picture,
    Sound,
    Navi(NaviHandle),
    Form(NaviHandle, FormHandle),
    Chip(ChipHandle, ChipCode),
}

struct Step {
    part: Part,
    /// Its share of the whole, for the fractions: about its time.
    share: u32,
    work: Work,
    done: bool,
}

/// The sound a player is made with: the packs' banks and the content's
/// songs, as `BattleAudio::with_banks` takes them.
pub type SoundParts = (Vec<Arc<m4a::SoundBank>>, nettai_audio::Songs);

/// A match being made ready: see the module's docs. It must be stepped on
/// the thread that will run the player it becomes ([`Preload::player`]):
/// the runtime it loads is that thread's.
pub struct Preload {
    content: Arc<Content>,
    set: Set,
    /// The side whose console is shown.
    viewer: u8,
    /// The host's picture, in its pixels.
    size: (usize, usize),
    renderer: Renderer,
    text: Option<TextRenderer>,
    sound: Option<SoundParts>,
    audio: Option<BattleAudio>,
    /// The round's first battle, once made, as the viewer's console shows
    /// it.
    battle: Option<Battle>,
    steps: Vec<Step>,
    stopped: Option<String>,
}

impl Preload {
    /// What `set`'s first round waits for, shown and heard from `game` (its
    /// sound, if it was loaded) on `viewer`'s console (the local player's
    /// side: 0 in live play and a replay watched from the left, a netplay
    /// peer's own) in a picture of `size` (the host's pixels: the text is
    /// drawn at that size).
    pub fn new(game: &Loaded, set: &Set, viewer: u8, size: (usize, usize)) -> Preload {
        let sound = game.sound.as_ref().map(|s| (s.banks.clone(), s.songs.clone()));
        Preload::with(game.renderer(), game.font.clone().map(TextRenderer::new), sound, set, viewer, size)
    }

    /// The same from a player's parts ([`Player::with`]): the renderer, the
    /// font mode's text renderer and the sound (none: silent).
    pub fn with(renderer: Renderer, text: Option<TextRenderer>, sound: Option<SoundParts>, set: &Set, viewer: u8, size: (usize, usize)) -> Preload {
        let content = set.content().clone();
        let mut steps = Vec::new();
        // The runtime is the wait (about a second against the rest's
        // hundredths), unless the thread has it.
        let runtime = if Behaviors::is_loaded(set.first().content) { 1 } else { 600 };
        steps.push(Step { part: Part::Match, share: runtime, work: Work::Runtime, done: false });
        steps.push(Step { part: Part::Match, share: 6, work: Work::Battle, done: false });
        let first = set.first();
        for side in 0..2u8 {
            let mut add = |work| steps.push(Step { part: Part::Side(side), share: 1, work, done: false });
            let navi = first.navi_stats[side as usize].navi;
            add(Work::Navi(navi));
            // The form it starts in and every form its navi lists, each
            // once (a form two lists hold is one form).
            let mut forms: Vec<FormHandle> = content.navi(navi).forms.iter().flat_map(|f| f.lists.iter().flat_map(|(_, forms)| forms.iter().copied())).collect();
            forms.push(first.navi_stats[side as usize].form);
            forms.sort();
            forms.dedup();
            for form in forms {
                add(Work::Form(navi, form));
            }
            // Each chip of its folder, once in each code it is there in.
            let mut chips: Vec<(ChipHandle, ChipCode)> = first.players[side as usize].folder.chips.iter().flatten().map(|c| (c.id, c.code)).collect();
            chips.sort_by_key(|&(chip, code)| (chip, code.0));
            chips.dedup();
            for (chip, code) in chips {
                add(Work::Chip(chip, code));
            }
        }
        steps.push(Step { part: Part::Match, share: 6, work: Work::Picture, done: false });
        if sound.is_some() {
            steps.push(Step { part: Part::Match, share: 2, work: Work::Sound, done: false });
        }
        Preload { content, set: set.clone(), viewer, size, renderer, text, sound, audio: None, battle: None, steps, stopped: None }
    }

    /// Run the next step. True while a step is left after it.
    pub fn step(&mut self) -> bool {
        if let Some(i) = self.steps.iter().position(|s| !s.done) {
            self.run(i);
        }
        !self.done()
    }

    /// Run the next step of `part` (one side's alone, or the match's own).
    /// True while that part has a step left after it.
    pub fn step_of(&mut self, part: Part) -> bool {
        if let Some(i) = self.steps.iter().position(|s| !s.done && s.part == part) {
            self.run(i);
        }
        self.steps.iter().any(|s| !s.done && s.part == part)
    }

    /// Every step ran.
    pub fn done(&self) -> bool {
        self.steps.iter().all(|s| s.done)
    }

    /// How far it is, from 0 to 1: the shares of the steps that ran over
    /// all of them. The match's own steps are nearly all of it while the
    /// thread has no runtime of the content (the step that loads one is
    /// about a second, the others hundredths together).
    pub fn fraction(&self) -> f32 {
        fraction(self.steps.iter())
    }

    /// How far `part` is, from 0 to 1 (1 for a part with no step: a silent
    /// match's sound).
    pub fn fraction_of(&self, part: Part) -> f32 {
        fraction(self.steps.iter().filter(|s| s.part == part))
    }

    /// What the steps run so far looked up that the pack doesn't have (a
    /// chip without a picture, a navi without a face), one line each: the
    /// match plays without it (a missing picture isn't drawn), and the host
    /// may say so before it starts.
    pub fn missing(&self) -> Vec<String> {
        self.renderer.problems.iter().map(|(what, _)| what.to_string()).collect()
    }

    /// Why the round can't start, if a step found it can't: the content's
    /// runtime doesn't load, or the engine stopped making the first battle
    /// (the same on every machine). The steps still run to their end.
    pub fn stopped(&self) -> Option<&str> {
        self.stopped.as_deref()
    }

    /// The player of what `driver` plays (the set this was made for), with
    /// what was made ready: its renderer and text as the steps left them,
    /// its sound. Steps not run are made up on the way, as a player made
    /// without a preload makes everything ([`Player::late`] counts it).
    pub fn player(mut self, driver: Box<dyn Driver>) -> Player {
        let audio = self.audio.take().or_else(|| self.sound.take().map(|(banks, songs)| BattleAudio::with_banks(banks, songs)));
        Player::with(self.renderer, self.text, audio, driver)
    }

    fn run(&mut self, i: usize) {
        self.steps[i].done = true;
        match self.steps[i].work {
            Work::Runtime => {
                if let Err(why) = runtime(&self.content) {
                    self.stopped.get_or_insert(why);
                }
            }
            Work::Battle => self.make_battle(),
            Work::Picture => {
                if self.battle.is_none() && self.stopped.is_none() {
                    self.make_battle();
                }
                let Some(battle) = &self.battle else { return };
                // The first picture, as the player will draw it: every
                // lookup it makes, and its text at the host's size.
                let frame = self.renderer.render(battle);
                let (w, h) = self.size;
                let mut pixels = vec![0u32; w * h];
                nettai_render::present::present(&frame, self.text.as_mut(), &mut pixels, w, h);
            }
            Work::Sound => {
                if let Some((banks, songs)) = self.sound.take() {
                    self.audio = Some(BattleAudio::with_banks(banks, songs));
                }
            }
            Work::Navi(navi) => self.look_up_navi(navi),
            Work::Form(navi, form) => self.look_up_form(navi, form),
            Work::Chip(chip, code) => self.look_up_chip(chip, code),
        }
    }

    /// The round's first battle, as the set starts it, shown from the
    /// viewer's console (a tick's panic is the engine stopping: here, at
    /// the setup).
    fn make_battle(&mut self) {
        let set = &self.set;
        match crate::session::in_engine(|| set.start()) {
            Ok(mut battle) => {
                // (The console drawn is the setup's; the simulation's own
                // perspective stays: `netplay`.)
                battle.setup.local_side = self.viewer;
                self.battle = Some(battle);
            }
            Err(e) => {
                let why = e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()));
                self.stopped.get_or_insert(format!("the round can't start: {}", why.unwrap_or_else(|| "the engine stopped".into())));
            }
        }
    }

    /// A navi's own in the pack: its face (a navi that changes form shows
    /// its forms'), its emblem, its name and variant name.
    fn look_up_navi(&mut self, navi: NaviHandle) {
        let (c, graphics) = (&*self.content, self.renderer.graphics().clone());
        let packs = graphics.packs();
        let strings = self.renderer.strings().cloned();
        let text = DisplayText::new(strings.as_deref());
        let p = &mut self.renderer.problems;
        let (hud, a) = (&packs.game(c).hud, &packs.game(c).custom);
        if !c.navi(navi).changes_form() {
            lookups::navi_face(&packs, c, navi, p);
        }
        if lookups::custom_graphics(a, p) {
            lookups::emblem(a, c, navi, p);
        }
        lookups::navi_name(hud, navi, false, text.navi_name(c, navi), p);
        if let Some(name) = text.navi_variant_name(c, navi) {
            lookups::navi_name(hud, navi, true, name, p);
        }
    }

    /// A form's in the pack: its face for each of the game's emotions, and
    /// where the game's custom screen lists forms in a window, its name
    /// there and its description.
    fn look_up_form(&mut self, navi: NaviHandle, form: FormHandle) {
        let (c, graphics) = (&*self.content, self.renderer.graphics().clone());
        let packs = graphics.packs();
        let strings = self.renderer.strings().cloned();
        let text = DisplayText::new(strings.as_deref());
        let p = &mut self.renderer.problems;
        let (hud, a) = (&packs.game(c).hud, &packs.game(c).custom);
        for e in 0..c.rules().emotion.names.len() {
            for variant in [false, true] {
                lookups::form_face(&packs, c, form, Emotion(e as u8), variant, p);
            }
        }
        let windowed = c.defs.rules().is_some_and(|r| r.views.form_list.is_some());
        if windowed && lookups::custom_graphics(a, p) && nettai_render::custom::form_name_picture(c, a, navi, form).is_some() {
            lookups::form_name(a, c, navi, form, p);
            if let Some(said) = text.form_description(c, form).filter(|_| lookups::chatbox_graphics(hud, p)) {
                lookups::dialogue(&hud.dialogue_font, Lookup::FormDescription(form), said.text, p);
            }
        }
    }

    /// A chip's in the pack: its icon, its picture (the button chip's is
    /// its button's), its name, the chip window's colors in its code, and
    /// its description.
    fn look_up_chip(&mut self, chip: ChipHandle, code: ChipCode) {
        let (c, graphics) = (&*self.content, self.renderer.graphics().clone());
        let packs = graphics.packs();
        let strings = self.renderer.strings().cloned();
        let text = DisplayText::new(strings.as_deref());
        let p = &mut self.renderer.problems;
        let (hud, a) = (&packs.game(c).hud, &packs.game(c).custom);
        lookups::chip_icon(&packs, c, chip, p);
        if c.defs.roles().try_chip(ChipRole::ButtonChip) != Some(chip) {
            lookups::chip_art(&packs, c, chip, p);
        }
        lookups::chip_name(hud, c, chip, text.chip_name(c, chip), p);
        if lookups::custom_graphics(a, p) {
            lookups::chip_window(a, c, chip, code.0, p);
        }
        if let Some(said) = text.chip_description(c, chip).filter(|_| lookups::chatbox_graphics(hud, p)) {
            lookups::dialogue(&hud.dialogue_font, Lookup::ChipDescription(chip), said.text, p);
        }
    }
}

/// The shares of the steps that ran over all of `steps`'.
fn fraction<'a>(steps: impl Iterator<Item = &'a Step>) -> f32 {
    let (mut all, mut done) = (0u32, 0u32);
    for s in steps {
        all += s.share;
        done += if s.done { s.share } else { 0 };
    }
    if all == 0 { 1.0 } else { done as f32 / all as f32 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::{InFight, LivePlayer, Opponent, bot_buttons};
    use nettai_battle::battle::mode;

    /// A set of EXE6's content, picked from a seed (1000 HP each, so the
    /// round lasts): made on the test's thread, which so has the content's
    /// runtime; what a test holds the preload to it runs on a thread that
    /// hasn't ([`fresh`]).
    fn set() -> Set {
        let content = nettai_match::testing::exe6_content();
        let mut m = nettai_match::pick::live(&content, "exe6", 5, None).unwrap();
        for s in &mut m.sides {
            s.set_fact(&content, "hp", &[nettai_battle::rules::Fact::Value(nettai_content_api::Value::Int(1000))]).unwrap();
        }
        Set::of(&content, &m, 5)
    }

    /// Run `f` on a thread of its own: one that has loaded no runtime.
    fn fresh<R: Send + 'static>(f: impl FnOnce() -> R + Send + 'static) -> R {
        std::thread::spawn(f).join().unwrap()
    }

    /// A renderer of a pack with nothing in it.
    fn renderer() -> Renderer {
        Renderer::new(Arc::new(nettai_assets::Bundle::default()))
    }

    /// `player` through `ticks` ticks of its set, a picture drawn of each:
    /// the local player presses through its custom screens and mashes in
    /// the fight.
    fn play(player: &mut Player, ticks: u32) {
        let mut masher = nettai_netplay::standin::Masher::new(3);
        masher.buster = true;
        for tick in 0..ticks {
            let b = player.battle();
            let keys = if b.round.mode == mode::CUSTOM { bot_buttons(b, 0, tick) } else { masher.buttons() };
            assert!(player.tick(keys), "tick {tick}: {:?}", player.stopped());
            player.frame();
        }
        assert!(player.battle().round.turn >= 1, "the fight ran");
    }

    /// The thread's runtime is loaded once, ahead: `runtime` says whether
    /// it loaded one, and a battle made after it loads none.
    #[test]
    fn the_runtime_is_loaded_ahead() {
        let set = set();
        fresh(move || {
            let (content, hash) = (set.content(), set.first().content);
            assert!(!Behaviors::is_loaded(hash));
            assert_eq!((runtime(content), Behaviors::loads()), (Ok(true), 1));
            assert_eq!((runtime(content), Behaviors::loads()), (Ok(false), 1));
            assert!(Behaviors::is_loaded(hash));
            set.start();
            assert_eq!(Behaviors::loads(), 1);
        });
    }

    /// A match is made ready a step at a time, by part: a side's steps run
    /// alone and need no runtime; the match's own load the thread's, which
    /// is most of the whole; the fractions go from 0 to 1. The player it
    /// becomes plays its set with nothing made late: no runtime loaded on
    /// its thread and nothing read from disk, through custom screens and
    /// fights. What the pack hasn't is said before the match starts.
    #[test]
    fn a_preloaded_match_loads_nothing_late() {
        let set = set();
        fresh(move || {
            let mut preload = Preload::with(renderer(), None, None, &set, 0, (480, 320));
            let parts = |p: &Preload| [Part::Match, Part::Side(0), Part::Side(1)].map(|part| p.fraction_of(part));
            assert_eq!((preload.fraction(), parts(&preload), preload.done()), (0.0, [0.0; 3], false));
            // One side's alone: its bar fills, nothing else moves.
            let mut steps = 0;
            while preload.step_of(Part::Side(1)) {
                steps += 1;
                let f = preload.fraction_of(Part::Side(1));
                assert!(f > 0.0 && f < 1.0, "{f}");
            }
            assert!(steps >= 10, "a navi, its forms and its folder's chips: {steps} steps");
            assert_eq!(parts(&preload), [0.0, 0.0, 1.0]);
            assert_eq!(Behaviors::loads(), 0, "a side's things are looked up without the runtime");
            // The rest, in order: the thread's runtime first, most of the
            // whole.
            let before = preload.fraction();
            assert!(preload.step());
            assert_eq!(Behaviors::loads(), 1);
            assert!(preload.fraction() - before > 0.5, "the runtime's share: {} to {}", before, preload.fraction());
            let mut last = preload.fraction();
            while preload.step() {
                assert!(preload.fraction() >= last);
                last = preload.fraction();
            }
            assert!(preload.done() && !preload.step() && !preload.step_of(Part::Match));
            assert_eq!((preload.fraction(), parts(&preload), preload.stopped()), (1.0, [1.0; 3], None));
            // (A pack with nothing in it: every chip's icon is missing.)
            let missing = preload.missing();
            assert!(missing.iter().any(|m| m.contains("has no icon in the pack")), "{missing:?}");
            let opponent = Opponent { in_fight: InFight::AttacksAtRandom, ..Opponent::default() };
            let mut player = preload.player(Box::new(LivePlayer::against(set, opponent, 9)));
            assert_eq!(player.late(), Late::default());
            play(&mut player, 1500);
            let late = player.late();
            assert_eq!((late.runtimes, late.reads, Behaviors::loads()), (0, 0, 1), "{late:?}");
        });
    }

    /// Without a preload the same match loads the thread's runtime as its
    /// player is made: the wait the preload takes ahead, which `late`
    /// counts.
    #[test]
    fn a_match_that_was_not_preloaded_loads_its_runtime_late() {
        let set = set();
        fresh(move || {
            let mut player = Player::with(renderer(), None, None, Box::new(LivePlayer::new(set)));
            assert_eq!(player.late(), Late { runtimes: 1, ..Late::default() });
            play(&mut player, 400);
            assert_eq!(player.late(), Late { runtimes: 1, ..Late::default() });
        });
    }

    /// Steps left out are made up by the player: a preload nobody stepped
    /// still gives one, which then loads the runtime late.
    #[test]
    fn a_preload_not_stepped_still_gives_a_player() {
        let set = set();
        fresh(move || {
            let preload = Preload::with(renderer(), None, None, &set, 1, (240, 160));
            assert_eq!(preload.fraction_of(Part::Match), 0.0);
            let mut player = preload.player(Box::new(LivePlayer::new(set)));
            assert_eq!(player.late().runtimes, 1);
            assert!(player.tick(0));
        });
    }
}
