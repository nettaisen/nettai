//! The selected game's look (docs/app.md §10): the theme's colors and the ground behind the menus, as each game's
//! content states them (its look.toml: `nettai_content::look`). Every game's look is read at start-up from the
//! content alone, as the games' names are, so the menus are in the selected game's look before it is loaded, and
//! read again when the game's content changes (a game whose content was downloaded after start-up, or a newer
//! revision of it); a game whose content states none, or one that can't be read (the log says why), has the app's
//! own look, the theme's as the UI starts.
//!
//! A change of game is two motions over the theme's `settle`. The new game's ground sweeps across the window from
//! the side its arrow was pressed on, over the one before (`Shell.grounds`, the last on top: each ground runs its
//! own sweep from its creation, at the frame after the press, and those under it are dropped once it is done), and
//! the theme's colors cross-fade from the ones shown to the new game's (`Theme.fade`, moved here a frame at a time,
//! eased as the sweep is). A
//! second change while one runs starts at once from what is shown: a third ground sweeps over the two, and the
//! colors fade on from where they were. Nothing waits on either, and neither changes a size. With motion off, the
//! new look is there at once.
//!
//! The patterns' paths ([`patterns`]) are drawn here for the window's size, on the tiles `GroundLayer` moves them
//! by: once for a size, never a frame at a time.

use nettai_ui::{AppWindow, Ground, Look, PatternKind, PatternPaths, Theme};
use slint::{Color, ComponentHandle, Model, ModelRc, VecModel};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fmt::Write as _;
use std::rc::Rc;
use std::time::Duration;
use web_time::Instant;

/// The most grounds there are at once: a change over a change over a change; one more drops the oldest.
const MOST_GROUNDS: usize = 4;

/// How long after the last ground's sweep began those under it are dropped: its sweep, and a little for the
/// frame its clock starts in.
const SWEPT: Duration = Duration::from_millis(60);

/// The looks, and the change under way.
pub struct Looks {
    /// Each game's look, by its id, with the content directory it was read from (none: the content states none, or
    /// it couldn't be read).
    by_game: HashMap<String, (PathBuf, Option<Look>)>,
    /// The app's own look: the theme's as the UI starts.
    plain: Look,
    /// The look shown, or being changed to.
    target: Look,
    /// The colors' cross-fade under way: since when.
    fading: Option<Instant>,
    /// A change's ground, to come in at the next frame.
    coming: Option<Ground>,
    /// The grounds the window shows, the last on top, and when each came.
    grounds: Rc<VecModel<Ground>>,
    came: Vec<Instant>,
}

impl Looks {
    /// Every game's look in `games`, read from each one's content (`crate::contents`); the window shows the app's
    /// own until a game is shown ([`Looks::show`]).
    pub fn new<'g>(ui: &AppWindow, games: impl IntoIterator<Item = &'g str>) -> Looks {
        let mut by_game = HashMap::new();
        for game in games {
            if let Some(dir) = crate::contents::dir(game) {
                let look = read(&dir, game);
                by_game.insert(game.to_string(), (dir, look));
            }
        }
        let plain = ui.global::<Theme>().get_look();
        let grounds = Rc::new(VecModel::from(vec![Ground { look: plain.clone(), from: 0 }]));
        ui.global::<nettai_ui::Shell>().set_grounds(ModelRc::from(grounds.clone()));
        Looks { by_game, target: plain.clone(), plain, fading: None, coming: None, grounds, came: vec![Instant::now()] }
    }

    /// Show game `game`'s look, its ground sweeping in from side `from` (-1 the left, 1 the right); `from` 0, or
    /// motion turned off, and it is there at once.
    ///
    /// The sweep and the colors' fade start at the next frame ([`Looks::frame`]), not here: what the change of game
    /// does besides (the screen shown again for the new game, the menu's reel starting its demo) holds the window
    /// for a tenth of a second or so after the press, and a clock started now would have run most of its way before
    /// a frame of it was drawn. Until that frame the colors are the ones shown.
    pub fn show(&mut self, ui: &AppWindow, game: Option<&str>, from: i32) {
        let look = game.and_then(|g| self.look_of(g)).unwrap_or_else(|| self.plain.clone());
        if look == self.target {
            return;
        }
        let theme = ui.global::<Theme>();
        let at_once = from == 0 || theme.get_settle() <= 0;
        // (From the colors as they are shown now: the last change's, part of the way.)
        let shown = blend(&theme.get_look(), &theme.get_was(), theme.get_fade());
        theme.set_was(shown);
        theme.set_look(look.clone());
        theme.set_fade(if at_once { 1.0 } else { 0.0 });
        self.fading = None;
        self.target = look.clone();
        if at_once {
            self.coming = None;
            self.grounds.set_vec(vec![Ground { look, from: 0 }]);
            self.came = vec![Instant::now()];
        } else {
            self.coming = Some(Ground { look, from: from.signum() });
        }
    }

    /// A frame at `now`, before it is drawn: a change's ground comes in (its sweep's clock starting at this frame)
    /// and its colors start to fade, the colors a step further, and the grounds the last has covered dropped.
    pub fn frame(&mut self, ui: &AppWindow, now: Instant) {
        let theme = ui.global::<Theme>();
        let settle = Duration::from_millis(theme.get_settle().max(0) as u64);
        if let Some(ground) = self.coming.take() {
            self.grounds.push(ground);
            self.came.push(now);
            while self.grounds.row_count() > MOST_GROUNDS {
                self.grounds.remove(0);
                self.came.remove(0);
            }
            self.fading = Some(now);
        }
        if let Some(at) = self.fading {
            let linear = if settle.is_zero() { 1.0 } else { (now.saturating_duration_since(at).as_secs_f32() / settle.as_secs_f32()).min(1.0) };
            // (Eased out, as the sweep's clock is: `Tween`'s `t`.)
            theme.set_fade(1.0 - (1.0 - linear).powi(3));
            if linear >= 1.0 {
                self.fading = None;
            }
        }
        if self.grounds.row_count() > 1 && self.came.last().is_some_and(|at| now.saturating_duration_since(*at) >= settle + SWEPT) {
            while self.grounds.row_count() > 1 {
                self.grounds.remove(0);
                self.came.remove(0);
            }
        }
    }

    /// How many grounds the window has (the screenshot pass's report).
    #[cfg_attr(not(feature = "tour"), allow(dead_code))]
    pub fn grounds(&self) -> usize {
        self.grounds.row_count()
    }
}

/// A look of the content's, as the theme takes it.
impl Looks {
    /// Game `game`'s look, from the content it has now: read again when that isn't what it was read from.
    fn look_of(&mut self, game: &str) -> Option<Look> {
        let dir = crate::contents::dir(game)?;
        if self.by_game.get(game).is_none_or(|(read_from, _)| *read_from != dir) {
            let look = read(&dir, game);
            self.by_game.insert(game.to_string(), (dir, look));
        }
        self.by_game.get(game)?.1.clone()
    }
}

/// Game `game`'s look in content directory `dir`: none where it states none, or can't be read (said in the log).
fn read(dir: &Path, game: &str) -> Option<Look> {
    match nettai_content::look::of(dir, game) {
        Ok(look) => look.as_ref().map(of),
        Err(why) => {
            crate::errors::log(&format!("reading {game}'s look"), &why);
            None
        }
    }
}

fn of(look: &nettai_content::look::Look) -> Look {
    use nettai_content::look::{PatternKind as Kind, Rgb, Strength};
    let rgb = |c: Rgb| {
        let [r, g, b] = c.channels();
        Color::from_rgb_u8(r, g, b)
    };
    let with = |c: Rgb, s: Strength| rgb(c).with_alpha(s.0);
    Look {
        ground_top: rgb(look.ground_top),
        ground_bottom: rgb(look.ground_bottom),
        ground_glow: with(look.glow, look.glow_strength),
        panel: rgb(look.panel),
        raised: rgb(look.raised),
        line: rgb(look.line),
        inset: rgb(look.inset),
        text: rgb(look.text),
        muted: rgb(look.muted),
        dim: rgb(look.dim),
        cursor: rgb(look.cursor),
        action: rgb(look.action),
        action_ink: rgb(look.action_ink),
        pattern: match look.pattern.kind {
            Kind::Hexagons => PatternKind::Hexagons,
            Kind::IsometricGrid => PatternKind::IsometricGrid,
            Kind::Starfield => PatternKind::Starfield,
        },
        pattern_color: rgb(look.pattern.color),
        pattern_strength: look.pattern.strength.0,
    }
}

/// `new`'s colors `fade` of the way from `old`'s, as the theme blends them (`Theme.blend`); the rest is `new`'s.
fn blend(new: &Look, old: &Look, fade: f32) -> Look {
    if fade >= 1.0 {
        return new.clone();
    }
    let mix = |a: Color, b: Color| a.mix(&b, fade);
    Look {
        ground_top: mix(new.ground_top, old.ground_top),
        ground_bottom: mix(new.ground_bottom, old.ground_bottom),
        panel: mix(new.panel, old.panel),
        raised: mix(new.raised, old.raised),
        line: mix(new.line, old.line),
        inset: mix(new.inset, old.inset),
        text: mix(new.text, old.text),
        muted: mix(new.muted, old.muted),
        dim: mix(new.dim, old.dim),
        cursor: mix(new.cursor, old.cursor),
        action: mix(new.action, old.action),
        action_ink: mix(new.action_ink, old.action_ink),
        ..new.clone()
    }
}

// ---- The patterns' paths ------------------------------------------------------------------------------------

/// The tiles the patterns repeat in, in units (as `GroundLayer` has them): a hexagon and the stem to the one
/// under it; the grid's lines, 46 apart at 30 degrees each way, which repeat every 46 / sin 30° across and
/// 46 / cos 30° down; the starfield's three lattices of dots, four, three and two to a tile each way.
const HEXAGON_TILE: (f32, f32) = (42.0, 72.75);
const GRID_TILE: (f32, f32) = (92.0, 53.116);
const STAR_TILE: (f32, f32) = (292.0, 244.0);

/// Every pattern's paths over a window of `width` by `height` units and a tile more each way.
pub fn patterns(width: i32, height: i32) -> PatternPaths {
    let [stars, far_stars, bright_stars] = starfield(width, height);
    PatternPaths {
        hexagons: hexagons(width, height).into(),
        isometric_grid: isometric_grid(width, height).into(),
        stars: stars.into(),
        far_stars: far_stars.into(),
        bright_stars: bright_stars.into(),
    }
}

/// The field of hexagons: each tile holds one hexagon and the stem to the one under it, and draws the edges its
/// neighbors don't.
fn hexagons(width: i32, height: i32) -> String {
    let (columns, rows) = ((width as f32 / HEXAGON_TILE.0).ceil() as i32 + 1, (height as f32 / HEXAGON_TILE.1).ceil() as i32 + 1);
    let mut path = String::new();
    for row in 0..rows {
        for column in 0..columns {
            let (x, y) = (column as f32 * HEXAGON_TILE.0, row as f32 * HEXAGON_TILE.1);
            // The left edge up to the top and down the right shoulder; the two lower edges; the stem.
            let _ = write!(
                path,
                "M{:.2} {:.2}V{:.2}L{:.2} {:.2}L{:.2} {:.2}M{:.2} {:.2}L{:.2} {:.2}L{:.2} {:.2}M{:.2} {:.2}V{:.2}",
                x,
                y + 36.38,
                y + 12.12,
                x + 21.0,
                y,
                x + 42.0,
                y + 12.12,
                x,
                y + 36.38,
                x + 21.0,
                y + 48.5,
                x + 42.0,
                y + 36.38,
                x + 21.0,
                y + 48.5,
                y + 72.75,
            );
        }
    }
    path
}

/// The isometric grid: lines falling and rising at 30 degrees, from the left edge to the right of the area (the
/// path clips what is past its bottom and top).
fn isometric_grid(width: i32, height: i32) -> String {
    let (w, h) = (width as f32 + GRID_TILE.0, height as f32 + GRID_TILE.1);
    // (Across the area a line at 30 degrees falls or rises this far.)
    let drop = w * 30f32.to_radians().tan();
    let lines = ((h + drop) / GRID_TILE.1).ceil() as i32;
    let mut path = String::new();
    for k in 0..=lines {
        let at = k as f32 * GRID_TILE.1;
        // Falling, from the left edge at `at - drop` to the right edge at `at`; rising, from `at` to `at - drop`.
        let _ = write!(path, "M0 {:.2}L{w:.2} {:.2}M0 {:.2}L{w:.2} {:.2}", at - drop, at, at, at - drop);
    }
    path
}

/// The starfield's three layers, each a lattice of dots: the white ones (four to a tile each way), the fainter
/// white ones (three, offset), and the brighter ones in the pattern's color, a little larger (two, offset).
fn starfield(width: i32, height: i32) -> [String; 3] {
    let (w, h) = (width as f32 + STAR_TILE.0, height as f32 + STAR_TILE.1);
    let layer = |per_tile: f32, offset: (f32, f32), radius: f32| {
        let step = (STAR_TILE.0 / per_tile, STAR_TILE.1 / per_tile);
        let mut path = String::new();
        let mut y = offset.1;
        while y < h {
            let mut x = offset.0;
            while x < w {
                // A circle: two half arcs.
                let _ = write!(path, "M{:.2} {y:.2}a{radius} {radius} 0 1 0 {d} 0a{radius} {radius} 0 1 0 -{d} 0Z", x - radius, d = radius * 2.0);
                x += step.0;
            }
            y += step.1;
        }
        path
    };
    [layer(4.0, (0.0, 0.0), 1.2), layer(3.0, (31.0, 17.0), 1.2), layer(2.0, (55.0, 44.0), 1.55)]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each pattern's path covers the window and a tile more each way, and repeats on its tile: shifted by one, a
    /// mark lands on another (so the ground can move by less than a tile and never shows its edge).
    #[test]
    fn the_patterns_cover_the_window_and_repeat_on_their_tiles() {
        // The isometric grid: each line falls (or rises) one tile's height across one tile's width.
        assert!((GRID_TILE.0 * 30f32.to_radians().tan() - GRID_TILE.1).abs() < 0.01);
        assert!((GRID_TILE.1 * 30f32.to_radians().cos() - 46.0).abs() < 0.01, "the lines are 46 apart");
        let grid = isometric_grid(1280, 800);
        let moves = grid.matches('M').count();
        assert!(moves >= 2 * ((800.0 + 53.116 + 1372.0 * 0.5774) / 53.116) as usize, "{moves} lines");
        // The starfield: each lattice's step divides the tile, and the dots reach past the window by a tile.
        let [stars, far, bright] = starfield(1280, 800);
        for (layer, per_tile) in [(&stars, 4.0), (&far, 3.0), (&bright, 2.0)] {
            let step = STAR_TILE.0 / per_tile;
            assert!((step * per_tile - STAR_TILE.0).abs() < 0.001);
            let dots = layer.matches('Z').count();
            let across = ((1280.0 + STAR_TILE.0) / step).floor() as usize;
            assert!(dots >= across * ((800.0 + STAR_TILE.1) / (STAR_TILE.1 / per_tile)).floor() as usize, "{dots} dots");
        }
        // The hexagons as they were.
        assert!(hexagons(1280, 800).starts_with("M0.00 36.38V12.12L21.00 0.00L42.00 12.12"));
    }

    /// A blend all the way is the new look; part of the way, its colors are between the two, and the rest is the
    /// new look's.
    /// Every game's content states its look, and it reads: the menus show each game's own.
    #[test]
    fn each_game_reads_its_look_from_its_content() {
        for game in ["exe4", "exe5", "exe6"] {
            let dir = crate::contents::dir(game).unwrap_or_else(|| panic!("{game} has no content"));
            assert!(read(&dir, game).is_some(), "{game}'s look");
        }
    }

    #[test]
    fn a_look_blends_from_the_one_before() {
        let mut old = Look { pattern: PatternKind::Hexagons, ..Default::default() };
        old.panel = Color::from_rgb_u8(0, 0, 0);
        let mut new = Look { pattern: PatternKind::Starfield, pattern_strength: 0.22, ..Default::default() };
        new.panel = Color::from_rgb_u8(200, 100, 0);
        assert_eq!(blend(&new, &old, 1.0), new);
        let half = blend(&new, &old, 0.5);
        assert_eq!((half.panel.red(), half.panel.green(), half.panel.blue()), (100, 50, 0));
        assert_eq!((half.pattern, half.pattern_strength), (PatternKind::Starfield, 0.22));
    }
}
