//! A match in binary: what a replay keeps of the match it plays, and what
//! a netplay offer carries. It is read against the content it was written
//! on, which whoever carries the bytes names beside them (a replay's head,
//! the netplay handshake: the game and the content's hash), and nothing in
//! it is a name: a definition is its handle in the content (the pack's
//! index, the same on every load of the content the hash names), a
//! background its number in the pack.
//!
//! ```text
//! place := stage (u16: its handle), background (u16: its number in the pack)
//! round := what the round states (a byte: 1 its stage, 2 its background, 3 both, 0 neither),
//!          then its stage's handle (u16) if stated, its background's number (u16) if stated
//! side  := the side's setup of the game's rules in its compact form (`Block::write_compact`:
//!          each fact in the setup's order, a list as its count and its entries)
//! match := seed (u32), the rounds' count (u8), every round's place (as the seed picks those
//!          the match leaves: `Match::places`), side 0, side 1
//! offer := the rounds' count (u8), each round as the player states it (round), then the side
//! ```
//!
//! Numbers are little-endian. A place's layout is the engine's (a round is
//! fought on a stage and a background, whatever the game); a side's
//! is its game's rules' setup, so a game's facts change no other game's
//! bytes. A reader refuses bytes that end early or run on, a handle past
//! the content's definitions, a background the pack hasn't, and a value no
//! fact may hold, each with where; whether what it read is a match the
//! game's rules allow is the checks' (`check`), as for a match file.

use crate::facts::Facts;
use crate::{Match, RoundSettings, Side, ids};
use nettai_battle::content::{BackgroundId, Content};
use nettai_content_api::{Block, FieldType, FieldValue, StageHandle};

/// The bytes of a stage and its background.
const PLACE: usize = 4;

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

/// The rounds' count, appended to `out`: refused past a byte's.
fn put_count(rounds: usize, out: &mut Vec<u8>) -> Result<(), String> {
    out.push(u8::try_from(rounds).map_err(|_| format!("{rounds} rounds: a match in binary has at most 255"))?);
    Ok(())
}

/// The rounds' count from the start of `bytes`.
fn take_count(bytes: &[u8]) -> Result<usize, String> {
    bytes.first().map(|&n| n as usize).ok_or_else(|| "rounds: the bytes end inside it".to_string())
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

/// A background's number from the start of `bytes`: its name in the
/// content's game's pack.
fn take_background(content: &Content, bytes: &[u8], at: &str) -> Result<String, String> {
    let b = bytes.get(..2).ok_or_else(|| format!("{at}: the bytes end inside it"))?;
    let n = u16::from_le_bytes([b[0], b[1]]);
    let name = ids::background_name(content, BackgroundId(n)).ok_or_else(|| format!("{at}: no background {n} in {}'s pack", content.game()))?;
    Ok(name.to_string())
}

/// A match in binary: its seed (refused: a match that states none), every
/// round's place (each part the match leaves picked from the seed), its
/// sides.
pub fn match_bytes(content: &Content, m: &Match) -> Result<Vec<u8>, String> {
    let seed = m.seed.ok_or("a match kept in binary states its seed")?;
    let mut out = seed.to_le_bytes().to_vec();
    let places = m.places(content, seed)?;
    put_count(places.len(), &mut out)?;
    for p in &places {
        out.extend_from_slice(&p.stage.0.to_le_bytes());
        out.extend_from_slice(&p.background.0.to_le_bytes());
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
    let count = take_count(&bytes[at..])?;
    at += 1;
    let mut rounds = Vec::with_capacity(count);
    for i in 0..count {
        let round = format!("round {}", i + 1);
        let stage = take_stage(content, &bytes[at..], &round)?;
        let background = take_background(content, &bytes[at + 2..], &round)?;
        at += PLACE;
        rounds.push(RoundSettings { stage: Some(stage), background: Some(background) });
    }
    let mut side = |name: &str| -> Result<Side, String> {
        let (s, used) = take_side(content, &bytes[at..]).map_err(|e| format!("{name}: {e}"))?;
        at += used;
        Ok(s)
    };
    let sides = [side("left")?, side("right")?];
    finished(bytes, at)?;
    Ok(Match { game: content.game().to_string(), seed: Some(seed), rounds, sides })
}

/// What a round states (a round's first byte): its stage.
const STATES_STAGE: u8 = 1;
/// What a round states: its background.
const STATES_BACKGROUND: u8 = 2;

/// That `at` is the end of `bytes`.
fn finished(bytes: &[u8], at: usize) -> Result<(), String> {
    match bytes.len() - at {
        0 => Ok(()),
        n => Err(format!("{n} bytes after its end")),
    }
}

/// A netplay offer in binary: the player's rounds as they state them (a
/// background named one its game's pack has), then their side.
pub fn offer_bytes(content: &Content, rounds: &[RoundSettings], side: &Side) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    put_count(rounds.len(), &mut out)?;
    for (i, r) in rounds.iter().enumerate() {
        let background = match &r.background {
            None => None,
            Some(name) => Some(crate::background(content, content.game(), name).ok_or_else(|| crate::no_background(&format!("round {}", i + 1), content.game(), name))?),
        };
        out.push(if r.stage.is_some() { STATES_STAGE } else { 0 } | if background.is_some() { STATES_BACKGROUND } else { 0 });
        if let Some(s) = r.stage {
            out.extend_from_slice(&s.0.to_le_bytes());
        }
        if let Some(b) = background {
            out.extend_from_slice(&b.0.to_le_bytes());
        }
    }
    put_side(content, side, &mut out);
    Ok(out)
}

/// A netplay offer from all of `bytes`: the player's rounds, their side.
pub fn read_offer(content: &Content, bytes: &[u8]) -> Result<(Vec<RoundSettings>, Side), String> {
    let count = take_count(bytes)?;
    let mut at = 1;
    let mut rounds = Vec::with_capacity(count);
    for i in 0..count {
        let round = format!("round {}", i + 1);
        let &states = bytes.get(at).ok_or_else(|| format!("{round}: the bytes end inside it"))?;
        at += 1;
        if states & !(STATES_STAGE | STATES_BACKGROUND) != 0 {
            return Err(format!("{round}: states {states:#04x}: a stage (1), a background (2), both (3) or neither (0)"));
        }
        let mut r = RoundSettings::default();
        if states & STATES_STAGE != 0 {
            r.stage = Some(take_stage(content, &bytes[at..], &round)?);
            at += 2;
        }
        if states & STATES_BACKGROUND != 0 {
            r.background = Some(take_background(content, &bytes[at..], &round)?);
            at += 2;
        }
        rounds.push(r);
    }
    let (side, used) = take_side(content, &bytes[at..]).map_err(|e| format!("side: {e}"))?;
    at += used;
    finished(bytes, at)?;
    Ok((rounds, side))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{exe5_content, exe6_content};

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

    /// An offer of a random side and its player's rounds (none stated, a
    /// stage, a random match's three, five of every kind) reads back, and
    /// fits one datagram with room to spare, EXE5's auto battle data and
    /// all.
    #[test]
    fn an_offer_fits_a_datagram() {
        for (content, game) in [(exe6_content(), "exe6"), (exe5_content(), "exe5")] {
            let m = crate::pick::live(&content, game, 9, None).unwrap();
            let stage = RoundSettings { stage: m.rounds[0].stage, background: None };
            let background = RoundSettings { stage: None, background: m.rounds[1].background.clone() };
            let five = vec![RoundSettings::default(), stage.clone(), background, m.rounds[2].clone(), RoundSettings::default()];
            for rounds in [vec![RoundSettings::default(); 3], vec![stage], m.rounds.clone(), five] {
                let bytes = offer_bytes(&content, &rounds, &m.sides[0]).unwrap();
                assert!(bytes.len() < 1200, "{game}: {} bytes", bytes.len());
                assert_eq!(read_offer(&content, &bytes).unwrap(), (rounds, m.sides[0].clone()), "{game}");
            }
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
        assert_eq!(bytes[4], 5);
    }

    /// What isn't a match is refused, saying where.
    #[test]
    fn bytes_that_arent_a_match_are_refused() {
        let content = exe6_content();
        let m = crate::pick::live(&content, "exe6", 5, None).unwrap();
        let bytes = match_bytes(&content, &m).unwrap();
        let read = |b: &[u8]| read_match(&content, b).unwrap_err();
        assert_eq!(read(&bytes[..3]), "seed: the bytes end inside it");
        assert_eq!(read(&bytes[..4]), "rounds: the bytes end inside it");
        assert_eq!(read(&bytes[..10]), "round 2: the bytes end inside it");
        let cut = read(&bytes[..bytes.len() - 1]);
        assert!(cut.starts_with("right: ") && cut.ends_with("the bytes end inside it"), "{cut}");
        let mut more = bytes.clone();
        more.push(0);
        assert_eq!(read(&more), "1 bytes after its end");
        let mut stage = bytes.clone();
        stage[5..7].copy_from_slice(&u16::MAX.to_le_bytes());
        assert_eq!(read(&stage), "round 1: no stage 65535 in the content");
        let mut background = bytes.clone();
        background[15..17].copy_from_slice(&u16::MAX.to_le_bytes());
        assert_eq!(read(&background), "round 3: no background 65535 in exe6's pack");
        // The left side's navi, a handle past the content's navis.
        let at = 5 + 3 * PLACE + before(&content, &m.sides[0], "navi");
        let mut navi = bytes.clone();
        navi[at..at + 2].copy_from_slice(&0x7FFFu16.to_le_bytes());
        assert_eq!(read(&navi), "left: navi: no navi 32766 in the content");
        let mut offer = offer_bytes(&content, &[RoundSettings::default()], &m.sides[0]).unwrap();
        offer[1] = 4;
        assert_eq!(read_offer(&content, &offer).unwrap_err(), "round 1: states 0x04: a stage (1), a background (2), both (3) or neither (0)");
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
