//! A game's look in the app (docs/app.md §10): `look.toml` in its pack's folder, the colors the app's menus are
//! drawn in while the game is selected, and the pattern of their backdrop.
//!
//! ```toml
//! ground_top = 0x0A1330
//! ground_bottom = 0x050916
//! glow = 0x286EFF
//! glow_strength = 0.28
//! panel = 0x0C1836
//! raised = 0x13254D
//! line = 0x27508F
//! inset = 0x040813
//! text = 0xEAF3FF
//! muted = 0x8EA8D4
//! dim = 0x55698F
//! cursor = 0x66E0FF
//! action = 0xFFB62E
//! action_ink = 0x1B1100
//!
//! [pattern]
//! kind = "hexagons"
//! color = 0x62B4FF
//! strength = 0.078
//! ```
//!
//! A color is 0xRRGGBB, as a version's is (rules/builds); a strength is the opacity something is drawn at, from 0
//! to 1. The pattern is one of the kinds the app draws, by what it is ([`PatternKind`]). Every field is stated:
//! a look is all of these or none. It is read from the content alone, as the game's name is (`locale::game_name`),
//! so the app has every game's look before any is loaded; the battle reads none of it, and it is out of the
//! content's hash.

use serde::Deserialize;
use std::path::{Path, PathBuf};

/// The file in a game pack's folder.
pub const FILE: &str = "look.toml";

/// A color, 0xRRGGBB.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(try_from = "i64")]
pub struct Rgb(pub u32);

impl TryFrom<i64> for Rgb {
    type Error = String;
    fn try_from(v: i64) -> Result<Rgb, String> {
        u32::try_from(v).ok().filter(|c| *c <= 0xFF_FFFF).map(Rgb).ok_or_else(|| format!("{v:#x} is no color (0xRRGGBB)"))
    }
}

impl Rgb {
    /// Its red, green and blue.
    pub fn channels(self) -> [u8; 3] {
        [(self.0 >> 16) as u8, (self.0 >> 8) as u8, self.0 as u8]
    }
}

/// An opacity, 0 to 1.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(try_from = "f64")]
pub struct Strength(pub f32);

impl TryFrom<f64> for Strength {
    type Error = String;
    fn try_from(v: f64) -> Result<Strength, String> {
        if (0.0..=1.0).contains(&v) { Ok(Strength(v as f32)) } else { Err(format!("{v} is no strength (0 to 1)")) }
    }
}

/// What a backdrop's pattern is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternKind {
    /// A field of hexagons, stroked.
    Hexagons,
    /// Lines at 30 degrees each way across the window.
    IsometricGrid,
    /// Three layers of dots, white and the pattern's color, with a streak across them from the pattern's color
    /// into the main action's.
    Starfield,
}

/// A backdrop's pattern: its kind, its color, and how strongly its marks are drawn.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pattern {
    pub kind: PatternKind,
    pub color: Rgb,
    pub strength: Strength,
}

/// A game's look: the theme's colors and its backdrop.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Look {
    /// The backdrop's ground, top to bottom, and the glow at its top right.
    pub ground_top: Rgb,
    pub ground_bottom: Rgb,
    pub glow: Rgb,
    pub glow_strength: Strength,
    /// A panel, a control raised on it, every line, a well cut into them.
    pub panel: Rgb,
    pub raised: Rgb,
    pub line: Rgb,
    pub inset: Rgb,
    /// Text, text that says less, text that says least.
    pub text: Rgb,
    pub muted: Rgb,
    pub dim: Rgb,
    /// The cursor, the main action, the text on the main action.
    pub cursor: Rgb,
    pub action: Rgb,
    pub action_ink: Rgb,
    pub pattern: Pattern,
}

/// The file of pack folder `pack` (content/exe6).
pub fn path(pack: &Path) -> PathBuf {
    pack.join(FILE)
}

/// Parse a look (`file` names it in messages).
pub fn parse(text: &str, file: &str) -> Result<Look, String> {
    toml::from_str(text).map_err(|e| format!("{file}: {}", e.message()))
}

/// Pack folder `pack`'s look; `None` when it states none (the app draws it in its own).
pub fn load(pack: &Path) -> Result<Option<Look>, String> {
    let path = path(pack);
    if !path.is_file() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path).map_err(|e| format!("reading {}: {e}", path.display()))?;
    parse(&text, &path.display().to_string()).map(Some)
}

/// Game `game`'s look, from content folder `dir`.
pub fn of(dir: &Path, game: &str) -> Result<Option<Look>, String> {
    load(&dir.join(game))
}

/// `nettai-content check`: each game's look reads (where it states one).
pub fn check_games(dir: &Path, c: &nettai_battle::Content, r: &mut crate::report::Report) {
    for game in c.scripts.games() {
        if let Err(e) = of(dir, &game) {
            r.error(format!("{game}/{FILE}"), e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// This repository's content.
    const CONTENT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content");

    const TEXT: &str = "ground_top = 0x0A1330\nground_bottom = 0x050916\nglow = 0x286EFF\nglow_strength = 0.28\n\
                        panel = 0x0C1836\nraised = 0x13254D\nline = 0x27508F\ninset = 0x040813\n\
                        text = 0xEAF3FF\nmuted = 0x8EA8D4\ndim = 0x55698F\n\
                        cursor = 0x66E0FF\naction = 0xFFB62E\naction_ink = 0x1B1100\n\n\
                        [pattern]\nkind = \"hexagons\"\ncolor = 0x62B4FF\nstrength = 0.078\n";

    #[test]
    fn a_look_reads_its_colors_and_its_pattern() {
        let look = parse(TEXT, "look.toml").unwrap();
        assert_eq!(look.ground_top, Rgb(0x0A1330));
        assert_eq!(look.cursor.channels(), [0x66, 0xE0, 0xFF]);
        assert_eq!(look.glow_strength, Strength(0.28));
        assert_eq!(look.pattern, Pattern { kind: PatternKind::Hexagons, color: Rgb(0x62B4FF), strength: Strength(0.078) });
    }

    /// What a look can't say: a color past 0xFFFFFF, a strength past 1, a pattern the app doesn't draw, a field it
    /// doesn't know, a field left out. Each is refused by its field's name.
    #[test]
    fn a_look_that_says_something_wrong_is_refused() {
        let wrong = |from: &str, to: &str| parse(&TEXT.replace(from, to), "look.toml").unwrap_err();
        assert!(wrong("cursor = 0x66E0FF", "cursor = 0x166E0FF").contains("no color"));
        assert!(wrong("strength = 0.078", "strength = 1.5").contains("no strength"));
        assert!(wrong("\"hexagons\"", "\"waves\"").contains("waves"));
        assert!(wrong("dim = 0x55698F", "dim = 0x55698F\nshadow = 0x000000").contains("shadow"));
        assert!(wrong("dim = 0x55698F\n", "").contains("dim"));
    }

    /// Each of this repository's games states its look, and it reads.
    #[test]
    fn every_game_of_the_repository_has_a_look() {
        let dir = Path::new(CONTENT);
        let games = crate::index::games(dir).unwrap();
        assert!(games.len() >= 3, "{games:?}");
        for game in games {
            assert!(of(dir, &game).unwrap_or_else(|e| panic!("{e}")).is_some(), "content/{game} has no {FILE}");
        }
    }
}
