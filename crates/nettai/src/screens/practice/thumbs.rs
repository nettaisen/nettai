//! The pictures of Practice's places, for the Arena and Background grids (docs/design/app-ui.md §2.56; docs/app.md
//! §15, "The places' pictures"): each arena a still of its field as its round's opening ends, each background a
//! still of its first frame, both cropped to the field's band of the battle's picture ([`WIDTH`] by [`HEIGHT`]).
//!
//! They are the renderer's, from the games' graphics, made as Practice needs them and kept for the session: nothing is
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
/// panels and what stands on them, the tallest whole (fifteen to seven: EXE4's flags on the back row reach up to
/// row 40, its field's front edge down to row 151; the grid shows it at half that, which a display of two pixels a
/// unit shows pixel for pixel).
const WIDTH: usize = 240;
const HEIGHT: usize = 112;
const TOP: usize = 40;

/// The seed an arena's round is made from: its pictures are the same each time.
const SEED: u32 = 7;

/// The most ticks an arena's round is played for its picture: it is taken as the round's opening ends (the field
/// faded in, the two navis on it, nothing yet over them), and no later than this.
const OPENING: u32 = 300;

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
            let spawned = nettai_platform::thread::spawn(&format!("{game} places"), move || {
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
    let held = graphics.game_graphics();
    let ids: Vec<_> = wanted.iter().map(|(at, _)| nettai_match::ids::background(content, game, &places.backgrounds[*at])).collect();
    // (Another game's backgrounds extracted now, here and not on the window's thread: each game's once a run.)
    held.wait_backgrounds(content, ids.iter().flatten().copied());
    for ((_, key), id) in wanted.iter().zip(ids) {
        let Some(rgb) = id.and_then(|id| nettai_render::still::background(&held, content, id, WIDTH, TOP + HEIGHT)) else { continue };
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
        let Some(field) = arena(content, &held, game, &megaman, records[0]) else { continue };
        if send.send(Made::Arena(at, crop(&field, TOP))).is_err() {
            return;
        }
    }
    let _ = send.send(Made::Done);
}

/// An arena's picture (the user: "you should just render the arena and megaman only without his hp", "you should
/// show the opponent too as megaman on the other side"; and, of the objects its arena places, "objects don't seem to
/// be rendered on stage previews"): a round on `stage` between two of `megaman`, as its opening ends, its field's
/// panels and what stands on them ([`drawn`]), each where the round puts it, and nothing else (no background, no
/// HUD, no object anything else made), each pixel `0xAARRGGBB`, clear where nothing is.
fn arena(content: &Arc<Content>, graphics: &nettai_render::graphics::GameGraphics, game: &str, megaman: &nettai_match::Side, stage: nettai_content_api::StageHandle) -> Option<Vec<u32>> {
    let session = opened(content, game, megaman, stage)?;
    Some(nettai_render::still::field(graphics, &session.battle, &drawn(&session.battle)))
}

/// A round on `stage` between two of `megaman`, played to its opening's end, as an arena's picture shows it.
fn opened(content: &Arc<Content>, game: &str, megaman: &nettai_match::Side, stage: nettai_content_api::StageHandle) -> Option<Session> {
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
    Some(session)
}

/// What an arena's picture draws of round `b`: its two navis, and the objects its arena places as the round
/// starts, those of the kinds its stage's list places beside the navis (the rocks, cubes, boulders, statues,
/// flags and gears: `Battle::spawn_actors`), as the round has them, each by its kind's own sprite. (Anything they
/// or the round make of other kinds is left out, the flags' HP with the rest of the HUD; the wind its windy arenas
/// place is unseen.)
fn drawn(b: &nettai_battle::Battle) -> Vec<nettai_battle::object::ObjectRef> {
    use nettai_battle::content::Place;
    let placed: Vec<_> = b.content.stage(b.setup.settings.stage).actors.iter().filter_map(|e| if let Place::Kind(kind) = e.place { Some(kind) } else { None }).collect();
    let navis = [0, 1].into_iter().filter_map(|side| b.player(side));
    navis.chain(b.objects.in_order().filter(|&r| placed.contains(&b.objects.get(r).kind))).collect()
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

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_battle::content::Place;

    /// An arena that places objects as its round starts has them in its picture, each on its panel (EXE6's cubes,
    /// ice and statues, EXE5's cubes and Guardians, EXE4's flags and gears, and the boulders of each), beside the
    /// two navis; one that places none has the two navis alone; and nothing else the round has is drawn.
    #[test]
    fn an_arenas_picture_has_its_objects() {
        use nettai_match::testing::{exe4_content, exe5_content, exe6_content};
        for (game, content) in [("exe6", exe6_content()), ("exe5", exe5_content()), ("exe4", exe4_content())] {
            let megaman = new_builds(&content, game).unwrap()[0].side.clone();
            let places = Places::of(&content, game);
            let mut with_objects = 0;
            for records in &places.arenas {
                let stage = records[0];
                let session = opened(&content, game, &megaman, stage).unwrap();
                let b = &session.battle;
                let drawn = drawn(b);
                let key = &content.defs.stage(stage).key;
                let navis: Vec<_> = [0, 1].into_iter().filter_map(|side| b.player(side)).collect();
                assert_eq!((navis.len(), &drawn[..2]), (2, &navis[..]), "{key}: the two navis first");
                let entries: Vec<_> = content.stage(stage).actors.iter().filter(|e| matches!(e.place, Place::Kind(_))).collect();
                for e in &entries {
                    let Place::Kind(kind) = e.place else { unreachable!() };
                    // (The wind its windy arenas place stands off the field.)
                    let on_field = (1..=6).contains(&e.x) && (1..=3).contains(&e.y);
                    let found = drawn[2..].iter().any(|&r| {
                        let o = b.objects.get(r);
                        o.kind == kind && (!on_field || (o.panel.x, o.panel.y) == (e.x, e.y))
                    });
                    assert!(found, "{key}: its {:?} on panel {},{}", content.defs.kind(kind).key, e.x, e.y);
                }
                with_objects += usize::from(!entries.is_empty());
                assert_eq!(drawn.len(), 2 + entries.len(), "{key}: its navis and its objects, nothing else");
                // (Drawn without a problem.)
                let graphics = nettai_frontend::testing::graphics(&content, 0x7FFF);
                let field = nettai_render::still::field(&graphics.game_graphics(), b, &drawn);
                assert_eq!(field.len(), 240 * 160, "{key}");
            }
            assert!(with_objects > 0, "{game}: some arenas place objects");
        }
    }
}
