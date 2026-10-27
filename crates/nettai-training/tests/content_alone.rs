//! A headless host's battles need no asset pack (docs/design/content-model-v2.md §6.3): each game's real content
//! (content/), loaded alone as the training pool loads it, has every sprite it names defined and timed, and plays a
//! battle (a random match, both sides mashing) with no error.

use nettai_battle::RoundEnd;
use nettai_netplay::standin::Masher;

#[test]
fn each_game_plays_from_its_content_alone() {
    for game in ["exe4", "exe5", "exe6"] {
        let content = nettai_training::pool::load_game(None, game).unwrap_or_else(|e| panic!("{e}"));
        // (Every sprite the content names is one it defines, with its animations.)
        assert!(!content.assets.sprites.is_empty(), "{game}: no sprite");
        assert_eq!(content.animations.sprites.len(), content.assets.sprites.len(), "{game}");
        assert!(content.animations.sprites.iter().all(|s| !s.animations.is_empty()), "{game}: a sprite without animations");
        let m = nettai_match::pick::live(&content, game, 7, None).unwrap_or_else(|e| panic!("{game}: {e}"));
        let mut b = nettai_match::Set::of(&content, &m, 7).start();
        let mut sides = [Masher::new(1), Masher::new(2)];
        for s in &mut sides {
            s.buster = true;
        }
        let mut ticks = 0;
        while ticks < 3600 && nettai_training::env::step_battle(&mut b, [sides[0].buttons(), sides[1].buttons()]) {
            ticks += 1;
        }
        if let Some(RoundEnd::Error(why)) = b.round_end() {
            panic!("{game}: the battle stopped after {ticks} ticks: {why}");
        }
        assert!(ticks == 3600 || b.round_end().is_some(), "{game}: {ticks} ticks");
    }
}
