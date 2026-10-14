//! A match in binary: what a replay keeps of the match it plays, and what
//! a netplay peer reveals of its side (the handshake names the rounds the
//! players agree by name: nettai-frontend's `lobby`). It is read against the content it was written
//! on, which whoever carries the bytes names beside them (a replay's head,
//! the netplay handshake: the game and the content's hash), and nothing in
//! it is a name but a round's background: a definition is its handle in the
//! content (the pack's index, the same on every load of the content the
//! hash names), and a background, which may be any game's (the background
//! pool's, docs/app.md §1, "The battle's background"), is its game and its
//! name in its game's pack, which name the same background whatever the
//! pool beside the content (no player's words: a match holds no other
//! text).
//!
//! ```text
//! place  := stage (u16: its handle), background (string: `<game>:<name>`, its game and its
//!           name in its game's pack)
//! side   := the side's setup of the game's rules in its compact form (`Block::write_compact`:
//!           each fact in the setup's order, a list as its count and its entries)
//! match  := seed (u32), the round wins its set is first to (u8: `n`, 1 to `MAX_FIRST_TO`),
//!           every one of its `2n − 1` rounds' places (as the seed picks those the match
//!           leaves: `Match::places`), side 0, side 1
//! string := length (LEB128), UTF-8
//! ```
//!
//! Numbers are little-endian. A place's layout is the engine's (a round is
//! fought on a stage and a background, whatever the game); a side's
//! is its game's rules' setup, so a game's facts change no other game's
//! bytes. A reader refuses bytes that end early or run on, a handle past
//! the content's definitions, a background neither the game's pack nor the
//! pool has, and a value no fact may hold, each with where; whether what it
//! read is a match the game's rules allow is the checks' (`check`), as for
//! a match file.

use crate::facts::Facts;
use crate::{Match, RoundSettings, Side, ids};
use nettai_battle::content::{BackgroundId, Content};
use nettai_content_api::{Block, FieldType, FieldValue, StageHandle};


/// A side, appended to `out`: its facts' block, compact.
pub fn put_side(content: &Content, side: &Side, out: &mut Vec<u8>) {
    let rules = content.defs.rules().expect("a match's content has rules");
    let block = side.facts.block().expect("a side of a game with rules has its facts");
    block.write_compact(content.defs.schema(rules.setup), out);
}

/// A side from the start of `bytes`, and how many bytes it took.
pub fn take_side(content: &Content, bytes: &[u8]) -> Result<(Side, usize), String> {
    let rules = content.defs.rules().ok_or_else(|| format!("{} has no rules", content.game()))?;
    let schema = content.defs.schema(rules.setup);
    let (block, used) = Block::read_compact(rules.setup, schema, bytes)?;
    for i in 0..schema.fields().len() {
        handles_held(content, &block, schema.place(i), &schema.field(i).name)?;
    }
    Ok((Side { facts: Facts::of_block(block) }, used))
}

/// That every definition the block holds at `place` (at any depth) is one
/// of the content's.
fn handles_held(content: &Content, block: &Block, place: nettai_content_api::Place, path: &str) -> Result<(), String> {
    match place.ty() {
        FieldType::Record(fields) => {
            for i in 0..fields.fields().len() {
                handles_held(content, block, place.field_at(i), &format!("{path}.{}", fields.field(i).name))?;
            }
        }
        FieldType::List(..) | FieldType::Array(..) => {
            for k in 0..block.len_at(place).expect("a list or an array") {
                handles_held(content, block, place.elem(k).expect("an element it holds"), &format!("{path}[{}]", k + 1))?;
            }
        }
        FieldType::Ref(registry, _) => {
            if let FieldValue::Ref(Some((r, h))) = block.get_at(place)
                && ids::key_of(content, r, h).is_none()
            {
                return Err(format!("{path}: no {registry} {h} in the content"));
            }
        }
        FieldType::Asset(kind) => {
            if let FieldValue::Asset(_, Some(h)) = block.get_at(place)
                && h as usize >= content.assets.names(*kind).len()
            {
                return Err(format!("{path}: no {kind} {h} in the pack"));
            }
        }
        _ => {}
    }
    Ok(())
}

/// What the set of `rounds` rounds is first to, appended to `out`: refused
/// for a count no set has.
fn put_first_to(rounds: usize, out: &mut Vec<u8>) -> Result<(), String> {
    out.push(crate::first_to(rounds).ok_or_else(|| crate::check::no_set(rounds))?);
    Ok(())
}

/// The rounds of the set, from what it is first to at the start of `bytes`.
fn take_rounds(bytes: &[u8]) -> Result<usize, String> {
    match bytes.first() {
        None => Err("first to: the bytes end inside it".to_string()),
        Some(&n) if n == 0 || n > crate::MAX_FIRST_TO => Err(format!("first to {n}: a set is first to 1 to {}", crate::MAX_FIRST_TO)),
        Some(&n) => Ok(crate::rounds_for(n)),
    }
}

/// A stage's handle from the start of `bytes`, one of the content's.
fn take_stage(content: &Content, bytes: &[u8], at: &str) -> Result<StageHandle, String> {
    let b = bytes.get(..2).ok_or_else(|| format!("{at}: the bytes end inside it"))?;
    let stage = u16::from_le_bytes([b[0], b[1]]);
    if stage as usize >= content.defs.stages.len() {
        return Err(format!("{at}: no stage {stage} in the content"));
    }
    Ok(StageHandle(stage))
}

/// A string, appended to `out`: its length (LEB128), then its UTF-8.
fn put_string(s: &str, out: &mut Vec<u8>) {
    let mut n = s.len();
    loop {
        let byte = (n & 0x7F) as u8;
        n >>= 7;
        if n == 0 {
            out.push(byte);
            break;
        }
        out.push(byte | 0x80);
    }
    out.extend_from_slice(s.as_bytes());
}

/// A string from the start of `bytes`, and how many bytes it took.
fn take_string<'b>(bytes: &'b [u8], at: &str) -> Result<(&'b str, usize), String> {
    let ended = || format!("{at}: the bytes end inside it");
    let (mut n, mut used) = (0usize, 0usize);
    loop {
        let b = *bytes.get(used).ok_or_else(ended)?;
        if used >= 4 {
            return Err(format!("{at}: a string's length past any"));
        }
        n |= ((b & 0x7F) as usize) << (7 * used);
        used += 1;
        if b & 0x80 == 0 {
            break;
        }
    }
    let text = bytes.get(used..used + n).ok_or_else(ended)?;
    let text = std::str::from_utf8(text).map_err(|_| format!("{at}: a string that isn't UTF-8"))?;
    Ok((text, used + n))
}

/// Background `id` as a place holds it: its game and its name in its
/// game's pack (`<game>:<name>`).
fn background_key(content: &Content, id: BackgroundId) -> Result<String, String> {
    content.background_of(id).map(|(game, name)| nettai_battle::content::pool_key(game, name)).ok_or_else(|| format!("no background {} in the content", id.0))
}

/// A place's background from the start of `bytes`: the name a match
/// writes for it (`ids::background_name`), and how many bytes it took.
fn take_background(content: &Content, bytes: &[u8], at: &str) -> Result<(String, usize), String> {
    let (key, used) = take_string(bytes, at)?;
    let id = key.split_once(':').and_then(|(game, name)| content.background_handle(game, name));
    let name = id.and_then(|id| ids::background_name(content, id)).ok_or_else(|| format!("{at}: no background {key:?} in the background pool"))?;
    Ok((name.to_string(), used))
}

/// A match in binary: its seed (refused: a match that states none), what
/// its set is first to, every round's place (each part the match leaves
/// picked from the seed), its sides.
pub fn match_bytes(content: &Content, m: &Match) -> Result<Vec<u8>, String> {
    let seed = m.seed.ok_or("a match kept in binary states its seed")?;
    let mut out = seed.to_le_bytes().to_vec();
    let places = m.places(content, seed)?;
    put_first_to(places.len(), &mut out)?;
    for (i, p) in places.iter().enumerate() {
        out.extend_from_slice(&p.stage.0.to_le_bytes());
        put_string(&background_key(content, p.background).map_err(|e| format!("round {}: {e}", i + 1))?, &mut out);
    }
    for side in &m.sides {
        put_side(content, side, &mut out);
    }
    Ok(out)
}

/// A match from all of `bytes`: every round's place stated.
pub fn read_match(content: &Content, bytes: &[u8]) -> Result<Match, String> {
    let seed = bytes.get(..4).ok_or("seed: the bytes end inside it")?;
    let seed = u32::from_le_bytes(seed.try_into().expect("four bytes"));
    let mut at = 4;
    let count = take_rounds(&bytes[at..])?;
    at += 1;
    let mut rounds = Vec::with_capacity(count);
    for i in 0..count {
        let round = format!("round {}", i + 1);
        let stage = take_stage(content, &bytes[at..], &round)?;
        let (background, used) = take_background(content, &bytes[at + 2..], &round)?;
        at += 2 + used;
        rounds.push(RoundSettings { stage: Some(stage), background: Some(background) });
    }
    let mut side = |name: &str| -> Result<Side, String> {
        let (s, used) = take_side(content, &bytes[at..]).map_err(|e| format!("{name}: {e}"))?;
        at += used;
        Ok(s)
    };
    let sides = [side("left")?, side("right")?];
    finished(bytes, at)?;
    Ok(Match { game: content.game().to_string(), seed: Some(seed), rounds, sides, backgrounds: crate::BackgroundPick::Game })
}


/// That `at` is the end of `bytes`.
fn finished(bytes: &[u8], at: usize) -> Result<(), String> {
    match bytes.len() - at {
        0 => Ok(()),
        n => Err(format!("{n} bytes after its end")),
    }
}

/// A side alone in binary (what a netplay peer reveals).
pub fn side_bytes(content: &Content, side: &Side) -> Vec<u8> {
    let mut out = Vec::new();
    put_side(content, side, &mut out);
    out
}

/// A side from all of `bytes`.
pub fn read_side(content: &Content, bytes: &[u8]) -> Result<Side, String> {
    let (side, used) = take_side(content, bytes)?;
    finished(bytes, used)?;
    Ok(side)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{exe5_content, exe6_content};

    /// A side's facts are numbers, choices and definitions, never text: a
    /// fact's type is one of these, and none of them holds a string. So a
    /// match in binary (in a replay, in a netplay offer) has nowhere a
    /// build's name could be. (A new type that holds text doesn't compile
    /// here until this is looked at.)
    #[test]
    fn no_fact_holds_text() {
        fn holds_text(ty: &FieldType) -> bool {
            match ty {
                FieldType::Bool | FieldType::U8 | FieldType::U16 | FieldType::U32 | FieldType::I8 | FieldType::I16 | FieldType::I32 => false,
                FieldType::Object | FieldType::Vec3 | FieldType::OptionalU8 | FieldType::Code => false,
                // (Stored as its variant's index; a definition and an asset
                // as their handles.)
                FieldType::Enum(_) | FieldType::Ref(..) | FieldType::Asset(_) => false,
                FieldType::Array(element, _) | FieldType::List(element, _) => holds_text(element),
                FieldType::Record(fields) => fields.fields().iter().any(|f| holds_text(&f.ty)),
            }
        }
        for (content, game) in [(exe6_content(), "exe6"), (exe5_content(), "exe5"), (crate::testing::exe4_content(), "exe4")] {
            let facts = crate::facts::fields(&content);
            assert!(!facts.is_empty(), "{game}");
            for fact in facts {
                assert!(!holds_text(fact.ty), "{game}: the fact {} holds text", fact.name);
            }
        }
    }

    /// Live play's random matches of both games written and read back: the
    /// same match, the same bytes again, a match the checks pass; and on
    /// another load of the content, the same bytes (a handle is the pack's
    /// index, the same on every load of content of one hash).
    #[test]
    fn a_match_reads_back_from_its_bytes() {
        for (content, game) in [(exe6_content(), "exe6"), (exe5_content(), "exe5")] {
            for seed in [1, 77, 4242] {
                let m = crate::pick::live(&content, game, seed, None).unwrap();
                let bytes = match_bytes(&content, &m).unwrap();
                let back = read_match(&content, &bytes).unwrap_or_else(|e| panic!("{game} {seed}: {e}"));
                assert_eq!(back, m, "{game} {seed}");
                assert_eq!(match_bytes(&content, &back).unwrap(), bytes, "{game} {seed}");
                assert_eq!(crate::check_match(&content, &back), Vec::<String>::new(), "{game} {seed}");
            }
        }
        let again = std::sync::Arc::new(crate::testing::defined(&["exe6"]).unwrap());
        assert_eq!(again.hash(), exe6_content().hash());
        let of = |c: &std::sync::Arc<Content>| match_bytes(c, &crate::pick::live(c, "exe6", 77, None).unwrap()).unwrap();
        assert_eq!(of(&again), of(&exe6_content()));
    }

    /// A random side of each game reads back alone, and fits one datagram
    /// with room to spare, EXE5's auto battle data and all.
    #[test]
    fn a_side_fits_a_datagram() {
        for (content, game) in [(exe6_content(), "exe6"), (exe5_content(), "exe5")] {
            let m = crate::pick::live(&content, game, 9, None).unwrap();
            let bytes = side_bytes(&content, &m.sides[0]);
            assert!(bytes.len() < 1200, "{game}: {} bytes", bytes.len());
            assert_eq!(read_side(&content, &bytes).unwrap(), m.sides[0], "{game}");
            let mut more = bytes.clone();
            more.push(0);
            assert_eq!(read_side(&content, &more).unwrap_err(), "1 bytes after its end");
        }
    }

    /// A match of five rounds, some parts stated and the rest left to the
    /// seed, is kept with every round's place: it reads back as the match
    /// with them all stated, which starts the same round and keeps the same
    /// bytes.
    #[test]
    fn a_match_keeps_every_rounds_place() {
        let content = exe6_content();
        let mut m = crate::pick::live(&content, "exe6", 21, None).unwrap();
        m.rounds = vec![RoundSettings::default(), m.rounds[1].clone(), RoundSettings { stage: m.rounds[0].stage, background: None }, RoundSettings::default(), RoundSettings::default()];
        let bytes = match_bytes(&content, &m).unwrap();
        let back = read_match(&content, &bytes).unwrap();
        assert_eq!(back, m.stated(&content, 21).unwrap());
        assert!(back.rounds.iter().all(|r| r.stage.is_some() && r.background.is_some()));
        assert_eq!(back.places(&content, 0).unwrap(), m.places(&content, 21).unwrap(), "stated: no seed picks them");
        assert_eq!(format!("{:?}", back.round(&content, 21)), format!("{:?}", m.round(&content, 21)));
        assert_eq!(match_bytes(&content, &back).unwrap(), bytes);
        assert_eq!(bytes[4], 3, "first to 3");
    }

    /// A match whose rounds are drawn on other games' backgrounds (the pool's: one stated, the rest picked from
    /// the pool) keeps each by its game and its name, and reads back as the match with them stated, which starts
    /// the same round: on another load of the content too.
    #[test]
    fn a_match_keeps_another_games_background() {
        let content = crate::testing::exe5_content();
        let mut m = crate::pick::live(&content, "exe5", 9, None).unwrap();
        m.backgrounds = crate::BackgroundPick::Pool;
        m.rounds = vec![RoundSettings { stage: None, background: Some("exe4:acdc-area".into()) }, RoundSettings::default(), RoundSettings::default()];
        let bytes = match_bytes(&content, &m).unwrap();
        let back = read_match(&content, &bytes).unwrap();
        assert_eq!(back.rounds[0].background.as_deref(), Some("exe4:acdc-area"));
        assert_eq!(back, Match { backgrounds: crate::BackgroundPick::Game, ..m.stated(&content, 9).unwrap() });
        assert_eq!(format!("{:?}", back.round(&content, 9)), format!("{:?}", m.round(&content, 9)));
        let key = |at: usize| String::from_utf8(bytes[at + 3..at + 3 + bytes[at + 2] as usize].to_vec()).unwrap();
        assert_eq!(key(5), "exe4:acdc-area", "by its game and its name");
        let again = std::sync::Arc::new(crate::testing::defined(&["exe5"]).unwrap());
        assert_eq!(read_match(&again, &bytes).unwrap(), back, "another load");
        // (A pool that hasn't it refuses it, saying which.)
        let mut alone = (*content).clone();
        alone.background_pool = nettai_battle::content::BackgroundPool::default();
        assert_eq!(read_match(&alone, &bytes).unwrap_err(), "round 1: no background \"exe4:acdc-area\" in the background pool");
    }

    /// What isn't a match is refused, saying where.
    #[test]
    fn bytes_that_arent_a_match_are_refused() {
        let content = exe6_content();
        let m = crate::pick::live(&content, "exe6", 5, None).unwrap();
        let bytes = match_bytes(&content, &m).unwrap();
        // (Each place: its stage, then its background's key's length and the key.)
        let places: Vec<usize> = m.places(&content, 5).unwrap().iter().map(|p| 2 + 1 + background_key(&content, p.background).unwrap().len()).collect();
        let place_at = |i: usize| 5 + places[..i].iter().sum::<usize>();
        let read = |b: &[u8]| read_match(&content, b).unwrap_err();
        assert_eq!(read(&bytes[..3]), "seed: the bytes end inside it");
        assert_eq!(read(&bytes[..4]), "first to: the bytes end inside it");
        for n in [0, crate::MAX_FIRST_TO + 1] {
            let mut first_to = bytes.clone();
            first_to[4] = n;
            assert_eq!(read(&first_to), format!("first to {n}: a set is first to 1 to 10"));
        }
        assert_eq!(read(&bytes[..place_at(1) + 1]), "round 2: the bytes end inside it");
        assert_eq!(read(&bytes[..place_at(1) + 3]), "round 2: the bytes end inside it", "inside its background");
        let cut = read(&bytes[..bytes.len() - 1]);
        assert!(cut.starts_with("right: ") && cut.ends_with("the bytes end inside it"), "{cut}");
        let mut more = bytes.clone();
        more.push(0);
        assert_eq!(read(&more), "1 bytes after its end");
        let mut stage = bytes.clone();
        stage[5..7].copy_from_slice(&u16::MAX.to_le_bytes());
        assert_eq!(read(&stage), "round 1: no stage 65535 in the content");
        let mut background = bytes.clone();
        background[place_at(2) + 3..place_at(2) + 7].copy_from_slice(b"exe9");
        let named = std::str::from_utf8(&background[place_at(2) + 3..place_at(3)]).unwrap().to_string();
        assert_eq!(read(&background), format!("round 3: no background {named:?} in the background pool"));
        // The left side's navi, a handle past the content's navis.
        let at = place_at(3) + before(&content, &m.sides[0], "navi");
        let mut navi = bytes.clone();
        navi[at..at + 2].copy_from_slice(&0x7FFFu16.to_le_bytes());
        assert_eq!(read(&navi), "left: navi: no navi 32766 in the content");
    }

    /// Where fact `name` of a side starts in its compact form: the bytes of
    /// the facts before it.
    fn before(content: &Content, side: &Side, name: &str) -> usize {
        let schema = content.defs.schema(content.defs.rules().unwrap().setup);
        let block = side.facts.block().unwrap();
        (0..schema.index_of(name).unwrap()).map(|i| compact_len(block, schema.place(i))).sum()
    }

    fn compact_len(block: &Block, place: nettai_content_api::Place) -> usize {
        match place.ty() {
            FieldType::Record(fields) => (0..fields.fields().len()).map(|i| compact_len(block, place.field_at(i))).sum(),
            FieldType::List(_, max) => {
                (if *max > 255 { 2 } else { 1 }) + (0..block.len_at(place).unwrap()).map(|k| compact_len(block, place.elem(k).unwrap())).sum::<usize>()
            }
            ty => ty.size(),
        }
    }
}
