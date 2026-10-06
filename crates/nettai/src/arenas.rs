//! The arenas of a match, drawn: each round's field and background as the
//! round opens, for the Play screen's preview. Each is the round played
//! alone for a moment (nobody pressing anything) and its picture taken.

use crate::RoundCard;
use crate::games::Ready;
use nettai_frontend::driver::LivePlayer;
use nettai_frontend::game::{Graphics, TextMode};
use nettai_frontend::player::Player;
use nettai_render::compose::{HEIGHT, WIDTH};
use slint::{Image, Rgb8Pixel, SharedPixelBuffer};

/// The ticks a round is played before its picture is taken: the field
/// faded in, the custom screen not yet over it.
const OPENING: u32 = 24;

/// Each round's arena: its picture, its stage and its background (by
/// their names in the content; it has no display names for them).
pub fn pictures(ready: &Ready, graphics: &Graphics, m: &nettai_match::Match, seed: u32) -> Vec<RoundCard> {
    let content = ready.content();
    let Ok(places) = m.places(content, seed) else { return Vec::new() };
    places
        .into_iter()
        .map(|place| {
            let one = nettai_match::Match { rounds: vec![nettai_match::RoundSettings::at(content, place)], seed: Some(seed), ..m.clone() };
            let set = nettai_match::Set::of(content, &one, seed);
            let mut player = Player::with(graphics.renderer(TextMode::Original, None), None, None, Box::new(LivePlayer::new(set)));
            for _ in 0..OPENING {
                player.tick(0);
            }
            let frame = player.frame();
            let mut pixels = SharedPixelBuffer::<Rgb8Pixel>::new(WIDTH as u32, HEIGHT as u32);
            for (out, &c) in pixels.make_mut_slice().iter_mut().zip(&frame.pixels) {
                let px = nettai_render::compose::to_rgb(c);
                *out = Rgb8Pixel { r: (px >> 16) as u8, g: (px >> 8) as u8, b: px as u8 };
            }
            let stage = nettai_match::ids::local(&content.defs.stage(place.stage).key).to_string();
            let background = nettai_match::ids::background_name(content, place.background).unwrap_or_default().to_string();
            RoundCard { picture: Image::from_rgb8(pixels), stage: stage.into(), background: background.into() }
        })
        .collect()
}
