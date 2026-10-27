//! The test content of the three games (`nettai_match::testing`: content/'s
//! definitions, every sprite timed alike), for this crate's tests and the
//! bridge's.

use nettai_battle::Content;
use std::sync::Arc;

/// Game `game`'s test content (exe4, exe5 or exe6).
pub fn content(game: &str) -> Arc<Content> {
    match game {
        "exe4" => nettai_match::testing::exe4_content(),
        "exe5" => nettai_match::testing::exe5_content(),
        "exe6" => nettai_match::testing::exe6_content(),
        other => panic!("no test content of {other}"),
    }
}

/// The three games of the test content.
pub const GAMES: [&str; 3] = ["exe6", "exe5", "exe4"];

/// The test content's games `names`, as a pool plays them.
pub fn games(names: &[&str]) -> Arc<Vec<crate::env::Game>> {
    Arc::new(names.iter().map(|g| crate::env::Game::new(content(g)).unwrap_or_else(|e| panic!("{e}"))).collect())
}
