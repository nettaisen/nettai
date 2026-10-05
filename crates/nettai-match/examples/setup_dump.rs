//! Prints what a round is made from and what its battle starts with, for
//! comparing before and after a change to how a side is made: the first
//! round's setup of random matches of each game (seeds 0 to 11) and of the
//! match files given, and the stats each side's round starts with once the
//! game's rules have set it up.
//!
//! ```text
//! cargo run -p nettai-match --features testing --example setup_dump -- [MATCH.toml...] > before.txt
//! ```
//!
//! A change that must not alter a round gives the same text. (The setup's
//! content hash is left out: it changes with the content.)

use nettai_battle::Content;
use nettai_match::Match;
use nettai_match::testing::{exe5_content, exe6_content};
use std::sync::Arc;

fn dump(name: &str, content: &Arc<Content>, m: &Match, seed: u32) {
    let mut setup = m.round(content, seed);
    setup.content = Default::default();
    println!("== {name}");
    println!("setup: {setup:?}");
    match nettai_match::check::start(content, m) {
        Ok(b) => {
            for side in 0..2 {
                println!("start {side}: {:?}", b.stats[side]);
            }
        }
        Err(e) => println!("start: {e}"),
    }
}

fn main() {
    for (game, content) in [("exe6", exe6_content()), ("exe5", exe5_content())] {
        for seed in 0..12 {
            match nettai_match::pick::live(&content, game, seed, None) {
                Ok(m) => dump(&format!("{game} seed {seed}"), &content, &m, seed),
                Err(e) => println!("== {game} seed {seed}\nno match: {e}"),
            }
        }
    }
    for path in std::env::args().skip(1) {
        let name = std::path::Path::new(&path).file_name().map_or(path.clone(), |n| n.to_string_lossy().into_owned());
        let read = std::fs::read_to_string(&path).map_err(|e| e.to_string()).and_then(|text| {
            let game = nettai_match::file::game_of(&text)?;
            let content = if game == "exe5" { exe5_content() } else { exe6_content() };
            let m = nettai_match::parse(&content, &text).map_err(|problems| problems.join("; "))?;
            Ok((content, m))
        });
        match read {
            Ok((content, m)) => dump(&name, &content, &m, m.seed.unwrap_or(0)),
            Err(e) => println!("== {name}\nno match: {e}"),
        }
    }
}
