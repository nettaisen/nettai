//! Changing the language mid-battle, checked on a game's real graphics:
//! the pictures a player gives after `Player::set_language` are exactly the
//! ones a player shown in that language from the start gives, and before it
//! the first language's.
//!
//! It plays one seeded random match with scripted buttons four times at
//! once, a tick each in turn: in the first language throughout, in the other
//! throughout, and twice changing from the first to the other, once in the
//! custom screen with a chip's description open and once in the fight with
//! the hand's chip named on screen. Every tick it presents all four and
//! compares the pixels.
//!
//! ```text
//! cargo run -p nettai-frontend --example language -- --game exe6 --seed 7 [--from en] [--to ja] [--text original]
//! ```
//!
//! The packs are found as the program finds them (`$NETTAI_PACKS`).

use nettai_battle::battle::mode;
use nettai_battle::custom::Phase;
use nettai_battle::input::keys;
use nettai_frontend::Player;
use nettai_frontend::driver::LivePlayer;
use nettai_frontend::game::{self, Found, Game, Graphics, TextMode};
use nettai_render::vfont::TextRenderer;

/// The buttons held, by tick: R on the first custom screen opens the chip's
/// description, A closes it, then a pick and OK.
const SCRIPT: &[(u32, u32, u16)] = &[(250, 251, keys::R), (290, 291, keys::A), (320, 340, keys::A), (420, 420, keys::START), (430, 460, keys::A)];

/// The ticks the two changing players change at: in the description, and in
/// the fight.
const IN_DESCRIPTION: u32 = 265;
const IN_FIGHT: u32 = 640;
const TICKS: u32 = 760;

fn held(tick: u32) -> u16 {
    SCRIPT.iter().filter(|(from, to, _)| (*from..=*to).contains(&tick)).fold(0, |all, (.., button)| all | button)
}

fn run() -> Result<String, String> {
    let (mut name, mut seed, mut from, mut to, mut text) = (None, None, "en".to_string(), "ja".to_string(), TextMode::Font);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut value = || args.next().ok_or(format!("{a} needs a value"));
        match a.as_str() {
            "--game" => name = Some(value()?),
            "--seed" => seed = Some(value()?.parse::<u32>().map_err(|e| format!("--seed: {e}"))?),
            "--from" => from = value()?,
            "--to" => to = value()?,
            "--text" => {
                text = match value()?.as_str() {
                    "original" => TextMode::Original,
                    "font" => TextMode::Font,
                    other => return Err(format!("--text {other}: original or font")),
                }
            }
            other => return Err(format!("no option {other:?} (--game GAME --seed N [--from LANG] [--to LANG] [--text original|font])")),
        }
    }
    let name = name.ok_or("say which game: --game exe6 or --game exe5")?;
    let seed = seed.ok_or("say the match's seed: --seed N")?;

    // The game, and its graphics in each language: the host loads what it offers.
    let said = |e: game::LoadError| format!("{}\n{e}", e.report);
    let found = Found::find(&nettai_content::pack::packs_dir(), &[]).map_err(said)?;
    let game = Game::load(&found, None, &name).map_err(said)?;
    let (first, other) = (game.graphics(&from).map_err(said)?, game.graphics(&to).map_err(said)?);
    let font = game::font(text, None).map_err(said)?;
    let m = nettai_match::pick::live(&game.content, &name, seed, None)?;
    let player = |graphics: &Graphics| {
        let driver = LivePlayer::new(nettai_match::Set::of(&game.content, &m, seed));
        Player::with(graphics.renderer(text, font.clone()), font.clone().map(TextRenderer::new), None, Box::new(driver))
    };
    // In the first language, in the other, and the two that change.
    let mut players = [player(&first), player(&other), player(&first), player(&first)];
    let changes = [IN_DESCRIPTION, IN_FIGHT];

    let (w, h) = (480, 320);
    let mut pictures = vec![vec![0u32; w * h]; 4];
    let mut differ = 0;
    for tick in 1..=TICKS {
        for (k, &at) in changes.iter().enumerate() {
            if tick == at {
                players[2 + k].set_language(&other);
            }
        }
        for (p, picture) in players.iter_mut().zip(&mut pictures) {
            if !p.tick(held(tick)) {
                return Err(format!("the battle stopped at tick {tick}: {}", p.stopped().unwrap_or("?")));
            }
            p.present(picture, w, h);
        }
        differ += (pictures[0] != pictures[1]) as u32;
        // What is on screen at each change is what the check is for.
        if tick == IN_DESCRIPTION {
            let phase = players[0].battle().custom.sides[0].screen.as_ref().map(|s| &s.phase);
            if !matches!(phase, Some(Phase::Description { .. })) {
                return Err(format!("no chip description is open at tick {tick} ({phase:?})"));
            }
        }
        if tick == IN_FIGHT {
            let b = players[0].battle();
            if b.round.mode != mode::FIGHTING || b.hands[0].remaining() == 0 {
                return Err(format!("no chip is in hand in the fight at tick {tick}"));
            }
        }
        if changes.contains(&tick) && pictures[0] == pictures[1] {
            return Err(format!("{from} and {to} draw tick {tick} alike: nothing to tell a change by"));
        }
        for (k, &at) in changes.iter().enumerate() {
            let same = if tick < at { 0 } else { 1 };
            if pictures[2 + k] != pictures[same] {
                let which = if tick < at { format!("{from}'s, before its change at {at}") } else { format!("{to}'s, after its change at {at}") };
                return Err(format!("tick {tick}: the picture isn't {which}"));
            }
        }
        // (One battle, whatever it is shown in.)
        if players.iter().any(|p| p.battle().digest() != players[0].battle().digest()) {
            return Err(format!("tick {tick}: the battles differ"));
        }
    }
    Ok(format!(
        "{name} seed {seed}, {text:?} text: {from} to {to} at tick {IN_DESCRIPTION} (a chip's description open) and at tick {IN_FIGHT} (a chip in hand, named); \
         {TICKS} ticks each: before a change {from}'s pictures, from it on {to}'s, every one identical; the two languages differ on {differ} of them"
    ))
}

fn main() {
    match run() {
        Ok(said) => println!("{said}"),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
