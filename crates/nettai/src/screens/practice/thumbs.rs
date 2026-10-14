//! The pictures of Practice's places, for the Arena and Background grids (docs/design/app-ui.md §2.56; docs/app.md
//! §15, "The places' pictures"): each arena a still of its field as its round's opening ends, each background a
//! still of its first frame, both cropped to the field's band of the battle's picture ([`WIDTH`] by [`HEIGHT`]).
//!
//! They are the renderer's, from the packs, made as Practice needs them and kept for the session: nothing is
//! written anywhere. A game's are made on a thread of their own ([`Thumbs::want`]: an arena's round needs the
//! content's runtime, which that thread loads for itself, about a second, so the window never waits), and handed
//! to the window's thread as they come ([`Thumbs::poll`]), where an open grid shows each as it comes. The
//! screenshot pass makes them on the window's thread at once, so that its pictures have them.

use super::{ARENA, Places, new_builds};
use nettai_battle::Content;
use nettai_frontend::driver::{InFight, LivePlayer, OnCustomScreen, Opponent};
use nettai_frontend::game::Graphics;
use nettai_frontend::session::Session;
use nettai_match::{BackgroundPick, Match, RoundSettings, Set};
use slint::{Image, Model, Rgba8Pixel, SharedPixelBuffer, VecModel};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};

/// A picture's pixels, which go between threads.
type Pixels = SharedPixelBuffer<Rgba8Pixel>;

/// A picture's size: the battle picture's width, and the band of its rows from [`TOP`] that holds the field, its
/// panels and what stands on them (two and a half to one; the grid shows it at half that, which a display of two
/// pixels a unit shows pixel for pixel).
const WIDTH: usize = 240;
const HEIGHT: usize = 96;
const TOP: usize = 56;

/// The seed an arena's round is made from: its pictures are the same each time.
const SEED: u32 = 7;

/// The most ticks an arena's round is played for its picture: as its opening ends (the field faded in, the
/// two navis on it), as the entry's preview is (`super::OPENING`).
const OPENING: u32 = super::OPENING;

/// A background by its game and its name in that game's pack: the pool's are the same in every game's list.
type BackgroundKey = (String, String);

/// What a game's thread made: an arena's picture by its place in the game's list (Random not counted), a
/// background's by its key, or that it is done.
enum Made {
    Arena(usize, Pixels),
    Background(BackgroundKey, Pixels),
    Done,
}

/// A cell of a grid: Random, or a place of the game's lists.
#[derive(Clone, PartialEq)]
enum Cell {
    Random,
    Arena(String, usize),
    Background(BackgroundKey),
}

/// A grid open: what each of its cells is, and the pictures it shows.
struct Open {
    cells: Vec<Cell>,
    pictures: Rc<VecModel<Image>>,
}

/// The pictures made so far, and what is making more.
#[derive(Default)]
pub struct Thumbs {
    /// Each game's arenas' pictures, by their places in its list (none: not made yet).
    arenas: HashMap<String, Vec<Option<Image>>>,
    /// The background pool's, by game and name; and each game's list's keys, by their places in it.
    backgrounds: HashMap<BackgroundKey, Image>,
    keys: HashMap<String, Vec<BackgroundKey>>,
    /// The games being made, each by a thread of its own.
    making: HashMap<String, Receiver<Made>>,
    open: Option<Open>,
}

impl Thumbs {
    /// Make `game`'s pictures, if they aren't made or being made: on a thread of their own (`now`: here, at
    /// once, the screenshot pass's).
    pub fn want(&mut self, game: &str, content: &Arc<Content>, graphics: &Graphics, now: bool) {
        if self.making.contains_key(game) || self.arenas.get(game).is_some_and(|a| a.iter().all(Option::is_some)) {
            return;
        }
        let places = Places::of(content, game);
        let keys: Vec<BackgroundKey> = places.backgrounds.iter().map(|b| background_key(content, game, b)).collect();
        let wanted: Vec<(usize, BackgroundKey)> = keys.iter().cloned().enumerate().filter(|(_, k)| !self.backgrounds.contains_key(k)).collect();
        self.keys.insert(game.to_string(), keys);
        self.arenas.entry(game.to_string()).or_insert_with(|| vec![None; places.arenas.len()]);
        let (send, receive) = std::sync::mpsc::channel();
        let (content, graphics, owned) = (content.clone(), graphics.clone(), game.to_string());
        if now {
            make(&content, &graphics, &owned, &places, &wanted, &send);
        } else {
            let spawned = std::thread::Builder::new().name(format!("{game} places")).spawn(move || {
                make(&content, &graphics, &owned, &places, &wanted, &send);
            });
            if let Err(why) = spawned {
                return crate::errors::log("drawing the places' pictures", &why);
            }
        }
        self.making.insert(game.to_string(), receive);
        if now {
            self.poll();
        }
    }

    /// Take what the games' threads have made: each picture kept, and shown in the grid open if it is one of its
    /// cells. Whether anything came (a select's own picture may be among it).
    pub fn poll(&mut self) -> bool {
        let mut came = false;
        let mut done = Vec::new();
        for (game, receive) in &self.making {
            for made in receive.try_iter() {
                came = true;
                let (cell, image) = match made {
                    Made::Arena(at, pixels) => {
                        let image = Image::from_rgba8(pixels);
                        if let Some(slot) = self.arenas.get_mut(game).and_then(|a| a.get_mut(at)) {
                            *slot = Some(image.clone());
                        }
                        (Cell::Arena(game.clone(), at), image)
                    }
                    Made::Background(key, pixels) => {
                        let image = Image::from_rgba8(pixels);
                        self.backgrounds.insert(key.clone(), image.clone());
                        (Cell::Background(key), image)
                    }
                    Made::Done => {
                        done.push(game.clone());
                        continue;
                    }
                };
                if let Some(open) = &self.open {
                    for at in (0..open.cells.len()).filter(|&at| open.cells[at] == cell) {
                        open.pictures.set_row_data(at, image.clone());
                    }
                }
            }
        }
        for game in done {
            self.making.remove(&game);
        }
        came
    }

    /// The picture of place `at` of `game`'s list `part` (0 Random, which has none; nor has one not made yet).
    pub fn picture(&self, game: &str, part: usize, at: usize) -> Image {
        let Some(at) = at.checked_sub(1) else { return Image::default() };
        let found = match part {
            ARENA => self.arenas.get(game).and_then(|a| a.get(at)).cloned().flatten(),
            _ => self.keys.get(game).and_then(|k| k.get(at)).and_then(|key| self.backgrounds.get(key)).cloned(),
        };
        found.unwrap_or_default()
    }

    /// The pictures of the grid of `game`'s list `part` opened now, Random first (a cell not made yet empty, and
    /// shown as it comes).
    pub fn open(&mut self, game: &str, part: usize, count: usize) -> Rc<VecModel<Image>> {
        let cells: Vec<Cell> = (0..count)
            .map(|at| match (at.checked_sub(1), part) {
                (None, _) => Cell::Random,
                (Some(at), ARENA) => Cell::Arena(game.to_string(), at),
                (Some(at), _) => self.keys.get(game).and_then(|k| k.get(at)).map_or(Cell::Random, |key| Cell::Background(key.clone())),
            })
            .collect();
        let pictures = Rc::new(VecModel::from((0..count).map(|at| self.picture(game, part, at)).collect::<Vec<_>>()));
        self.open = Some(Open { cells, pictures: pictures.clone() });
        pictures
    }
}

/// The key of the background a match of `game` names `written`.
fn background_key(content: &Content, game: &str, written: &str) -> BackgroundKey {
    match super::pool_background(content, game, written) {
        Some((of, name)) => (of.to_string(), name.to_string()),
        None => (game.to_string(), written.to_string()),
    }
}

/// Make `game`'s pictures, sending each as it is made: the backgrounds `wanted` (by their places in its list:
/// they need no runtime), then its arenas, each a round of its own.
fn make(content: &Arc<Content>, graphics: &Graphics, game: &str, places: &Places, wanted: &[(usize, BackgroundKey)], send: &Sender<Made>) {
    let packs = graphics.packs();
    for (at, key) in wanted {
        let id = nettai_match::ids::background(content, game, &places.backgrounds[*at]);
        let Some(rgb) = id.and_then(|id| nettai_render::still::background(&packs, content, id, WIDTH, TOP + HEIGHT)) else { continue };
        let argb: Vec<u32> = rgb.into_iter().map(|c| 0xFF00_0000 | c).collect();
        if send.send(Made::Background(key.clone(), crop(&argb, TOP))).is_err() {
            return;
        }
    }
    // (The rounds need the content's runtime on this thread: about a second, here and not the window's.)
    if let Err(why) = nettai_frontend::preload::runtime(content) {
        return crate::errors::log("drawing the arenas' pictures", &why);
    }
    // (MegaMan on both sides, as a new build of the game's first version has him: never the player's builds.)
    let megaman = match new_builds(content, game) {
        Ok(builds) if !builds.is_empty() => builds[0].side.clone(),
        Ok(_) => return,
        Err(why) => return crate::errors::log("drawing the arenas' pictures", &why),
    };
    for (at, records) in places.arenas.iter().enumerate() {
        let Some(field) = arena(content, &packs, game, &megaman, records[0]) else { continue };
        if send.send(Made::Arena(at, crop(&field, TOP))).is_err() {
            return;
        }
    }
    let _ = send.send(Made::Done);
}

/// An arena's picture (the user: "you should just render the arena and megaman only without his hp", "you should
/// show the opponent too as megaman on the other side"): a round on `stage` between two of `megaman`, as its
/// opening ends, its field's panels and the two navis standing on them, each where the round puts it, and
/// nothing else (no background, no HUD, no other object), each pixel `0xAARRGGBB`, clear where nothing is.
fn arena(content: &Arc<Content>, packs: &nettai_render::packs::PackGraphics, game: &str, megaman: &nettai_match::Side, stage: nettai_content_api::StageHandle) -> Option<Vec<u32>> {
    let mut m = Match::empty(content, game).map_err(|why| crate::errors::log("drawing an arena's picture", &why)).ok()?;
    m.sides = [megaman.clone(), megaman.clone()];
    let round = RoundSettings { stage: Some(stage), background: None };
    m.rounds.iter_mut().for_each(|r| *r = round.clone());
    m.backgrounds = BackgroundPick::Pool;
    let mut live = LivePlayer::new(Set::of(content, &m, SEED));
    live.set_opponent(Opponent { in_fight: InFight::StandsStill, on_custom_screen: OnCustomScreen::PicksNothing });
    let mut session = Session::new(Box::new(live));
    let opening = session.battle.round.mode;
    for _ in 0..OPENING {
        if session.battle.round.mode != opening || !session.step(0) {
            break;
        }
    }
    let b = &session.battle;
    let navis: Vec<_> = [0, 1].into_iter().filter_map(|side| b.player(side)).collect();
    Some(nettai_render::still::field(packs, b, &navis))
}

/// [`HEIGHT`] rows of `argb` (the screen's width, each pixel `0xAARRGGBB`) from row `top`, as a picture's pixels.
fn crop(argb: &[u32], top: usize) -> Pixels {
    let mut out = SharedPixelBuffer::new(WIDTH as u32, HEIGHT as u32);
    let pixels = out.make_mut_slice();
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let c = argb.get((top + y) * WIDTH + x).copied().unwrap_or(0);
            pixels[y * WIDTH + x] = Rgba8Pixel { r: (c >> 16) as u8, g: (c >> 8) as u8, b: c as u8, a: (c >> 24) as u8 };
        }
    }
    out
}
