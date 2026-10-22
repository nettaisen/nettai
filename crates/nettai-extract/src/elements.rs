//! The element icons taken apart: the frame they share, and each icon's
//! symbol (the user: "make the extract element icons transparent and
//! borderless").

use nettai_assets::{CustomScreen, Tiles};

/// An icon's pixels (2x2 tiles).
const ICON: usize = 4 * Tiles::TILE;

/// `c`'s element icons as their symbols over the frame they share
/// (`CustomScreen::element_frame`): the frame each pixel's index most of
/// the icons with any share, the rest of it its ground (the frame's most
/// common index); an icon its pixels that aren't the frame's (its symbol,
/// and any pixel of the frame it draws its own way), clear elsewhere. What
/// was found is said: the icons that keep pixels of the frame, and any the
/// frame shows through (a pixel the icon leaves clear where the frame has
/// one: drawn over the frame, it isn't the game's).
pub fn split(c: &mut CustomScreen) -> Vec<String> {
    let mut said = Vec::new();
    let icons = c.elements.pixels.as_chunks::<ICON>().0;
    let live: Vec<&[u8; ICON]> = icons.iter().filter(|i| i.iter().any(|&p| p != 0)).collect();
    if live.is_empty() || !c.element_frame.is_empty() {
        return said;
    }
    let most = |values: &mut dyn Iterator<Item = u8>| -> (u8, usize) {
        let mut n = [0usize; 16];
        for v in values {
            n[v as usize & 15] += 1;
        }
        let k = (0..16).max_by_key(|&k| (n[k], std::cmp::Reverse(k))).unwrap_or(0);
        (k as u8, n[k])
    };
    let shared: Vec<(u8, usize)> = (0..ICON).map(|i| most(&mut live.iter().map(|icon| icon[i]))).collect();
    let ground = most(&mut shared.iter().map(|s| s.0)).0;
    let frame: Vec<u8> = shared.iter().map(|&(v, n)| if 2 * n > live.len() { v } else { ground }).collect();
    c.element_frame = Tiles { pixels: frame.clone() };
    let inside = c.element_inside().unwrap_or_default();
    let (mut own, mut through) = (Vec::new(), Vec::new());
    for (k, icon) in icons.iter().enumerate().filter(|(_, i)| i.iter().any(|&p| p != 0)) {
        let kept = (0..ICON).filter(|&i| !inside[i] && icon[i] != frame[i] && icon[i] != 0).count();
        let clear = (0..ICON).filter(|&i| icon[i] == 0 && frame[i] != 0).count();
        if kept > 0 {
            own.push(format!("{k} ({kept})"));
        }
        if clear > 0 {
            through.push(format!("{k} ({clear})"));
        }
    }
    let rest = c.elements.pixels.len() % ICON;
    let mut pixels: Vec<u8> = icons.iter().flat_map(|icon| icon.iter().zip(&frame).map(|(&p, &f)| if p != f { p } else { 0 })).collect();
    pixels.extend(std::iter::repeat_n(0, rest));
    c.elements = Tiles { pixels };
    if !own.is_empty() {
        said.push(format!("note: the element icons {} keep their own pixels of the frame, which aren't the frame's (how many)", own.join(", ")));
    }
    if !through.is_empty() {
        said.push(format!("the element icons {} are clear where the frame isn't (how many pixels): drawn over the frame, they aren't the game's", through.join(", ")));
    }
    said
}
