//! Survey a content pack's songs as timelines: what a MIDI file has to
//! carry.
//!
//!     cargo run -p nettai-content --example audio_stats -- data/content/bn6

use nettai_content::timeline::{Ending, encode, linearize};
use m4a::bank::Command;
use std::collections::{BTreeMap, HashMap};

fn main() {
    let path = std::env::args().nth(1).expect("usage: audio_stats <pack>");
    let (bank, _) = nettai_content::pack::load_sound(std::path::Path::new(&path)).unwrap_or_else(|r| panic!("{r}"));
    let mut n = BTreeMap::<String, usize>::new();
    let mut bump = |k: &str| *n.entry(k.to_string()).or_default() += 1;
    let mut big = BTreeMap::<&str, BTreeMap<u8, usize>>::new();
    let mut tempos = BTreeMap::<u8, usize>::new();
    for (id, song) in bank.songs.iter().enumerate() {
        let Some(song) = song else { continue };
        bump("songs");
        let mut loops = Vec::new();
        for (ti, track) in song.tracks.iter().enumerate() {
            bump("tracks");
            let tl = match linearize(track) {
                Ok(t) => t,
                Err(e) => {
                    println!("song {id:#05x} track {ti}: {e}");
                    bump("tracks that don't linearize");
                    continue;
                }
            };
            if linearize(&encode(&tl)).as_ref() != Ok(&tl) {
                println!("song {id:#05x} track {ti}: encode round trip differs");
                bump("encode round trip differs");
            }
            if let Ending::Loop { start, start_tick, end_tick } = tl.ending {
                loops.push((start_tick, end_tick));
                if tl.events[..start].iter().any(|e| e.tick == start_tick) {
                    bump("loops with intro events on the loop's first tick");
                }
                if tl.events[start..].iter().any(|e| e.tick == end_tick) {
                    bump("loops with events on the loop's last tick");
                }
            } else {
                loops.push((u32::MAX, u32::MAX));
            }
            // Per command kind: values that don't fit 7 bits.
            let mut open_ties: HashMap<u8, Vec<u32>> = HashMap::new();
            let mut sounding: Vec<(u8, u32, u32)> = Vec::new(); // (key, start, end) of gated notes
            let mut last_note_tick = None;
            for (i, e) in tl.events.iter().enumerate() {
                let mut hi = |k: &'static str, v: u8| {
                    if v >= 128 {
                        *big.entry(k).or_default().entry(v).or_default() += 1;
                    }
                };
                match e.command {
                    Command::Tempo(v) => {
                        *tempos.entry(v).or_default() += 1;
                        if ti != 0 {
                            bump("tempo commands on tracks other than the first");
                        }
                    }
                    Command::KeyShift(v) => {
                        bump("KEYSH commands");
                        if v != 0 {
                            bump("KEYSH commands (non-zero)");
                        }
                    }
                    Command::Voice(v) => hi("voice", v),
                    Command::Volume(v) => hi("volume", v),
                    Command::Pan(v) => hi("pan", v),
                    Command::Bend(v) => hi("bend", v),
                    Command::BendRange(v) => hi("bend range", v),
                    Command::LfoSpeed(v) => hi("lfo speed", v),
                    Command::LfoDelay(v) => hi("lfo delay", v),
                    Command::Modulation(v) => hi("mod", v),
                    Command::ModulationType(v) => hi("mod type", v),
                    Command::Tune(v) => hi("tune", v),
                    Command::Priority(v) => hi("priority", v),
                    Command::EchoVolume(v) => hi("echo volume", v),
                    Command::EchoLength(v) => hi("echo length", v),
                    Command::Note { gate, key: Some(k), velocity: Some(v) } => {
                        if v == 0 {
                            bump("notes with velocity 0");
                        }
                        if gate == 0 {
                            if open_ties.get(&k).is_some_and(|v| !v.is_empty()) {
                                bump("ties on a key already tied");
                            }
                            open_ties.entry(k).or_default().push(e.tick);
                        } else {
                            if gate > 96 {
                                bump("notes with gate > 96");
                            }
                            // Same-key overlap with a sounding gated note.
                            sounding.retain(|&(_, _, end)| end > e.tick);
                            if sounding.iter().any(|&(kk, _, _)| kk == k) {
                                bump("gated notes overlapping a same-key gated note");
                                if sounding.iter().any(|&(kk, _, end)| kk == k && end > e.tick + gate as u32) {
                                    bump("  ... ending before it (FIFO mispairs)");
                                }
                            }
                            if open_ties.get(&k).is_some_and(|v| !v.is_empty()) {
                                bump("gated notes during a same-key tie");
                            }
                            sounding.push((k, e.tick, e.tick + gate as u32));
                        }
                        last_note_tick = Some((i, e.tick));
                    }
                    Command::EndTie { key: Some(k) } => match open_ties.get_mut(&k).and_then(|v| v.pop()) {
                        Some(start) => {
                            if e.tick - start <= 96 {
                                bump("ties of 96 ticks or less");
                            }
                            if e.tick == start {
                                bump("ties ended on their own tick");
                            }
                        }
                        None => bump("end-ties with no open tie of the key"),
                    },
                    _ => {}
                }
                // Latched settings after a note on the same tick.
                if let Some((_, t)) = last_note_tick
                    && t == e.tick
                    && !matches!(e.command, Command::Note { .. })
                {
                    let what = match e.command {
                        Command::Voice(_) => Some("VOICE"),
                        Command::Priority(_) => Some("PRIO"),
                        Command::EchoVolume(_) | Command::EchoLength(_) => Some("XCMD echo"),
                        Command::LfoDelay(_) => Some("LFODL"),
                        Command::EndTie { .. } => Some("EOT"),
                        Command::Tempo(_) => Some("TEMPO"),
                        _ => Some("other control"),
                    };
                    if let Some(w) = what {
                        bump(&format!("{w} after a note on the same tick"));
                    }
                }
            }
            if open_ties.values().any(|v| !v.is_empty()) {
                bump("tracks ending with an open tie");
            }
        }
        // Song-level loop: the latest per-track start, the common period.
        let froms: Vec<(u32, u32)> =
            song.tracks.iter().filter_map(|t| linearize(t).ok().and_then(|tl| tl.loop_from())).collect();
        if !froms.is_empty() {
            let l = froms.iter().map(|f| f.0).max().unwrap();
            let periods: std::collections::BTreeSet<u32> = froms.iter().map(|f| f.1).collect();
            println!("song {id:#05x}: loop from {l} (per track {:?}), periods {periods:?}", froms.iter().map(|f| f.0).collect::<Vec<_>>());
        }
        // Tempo's place among track 0's commands on its tick.
        if let Some(t0) = song.tracks.first().and_then(|t| linearize(t).ok()) {
            for (i, e) in t0.events.iter().enumerate() {
                if matches!(e.command, Command::Tempo(_)) {
                    let before: Vec<String> = t0.events[..i]
                        .iter()
                        .filter(|x| x.tick == e.tick)
                        .map(|x| format!("{:?}", x.command).split(['(', ' ', '{']).next().unwrap().to_string())
                        .collect();
                    bump(&format!("tempo after [{}] on its tick", before.join(",")));
                }
            }
        }
        for (ti, t) in song.tracks.iter().enumerate() {
            let Ok(tl) = linearize(t) else { continue };
            if let Ending::Fine { tick } = tl.ending {
                let last_off = tl
                    .events
                    .iter()
                    .filter_map(|e| match e.command {
                        Command::Note { gate, .. } if gate > 0 => Some(e.tick + gate as u32),
                        _ => None,
                    })
                    .max()
                    .unwrap_or(0);
                if last_off > tick {
                    bump("ending tracks whose notes outlast FINE");
                }
                if tl.events.last().is_some_and(|e| e.tick == tick) {
                    bump("ending tracks with events on the FINE tick");
                }
            }
            let ties =tl.events.iter().filter(|e| matches!(e.command, Command::Note { gate: 0, .. })).count();
            let eots = tl.events.iter().filter(|e| matches!(e.command, Command::EndTie { .. })).count();
            if ties != eots {
                println!("song {id:#05x} track {ti}: {ties} ties, {eots} end-ties, ending {:?}", tl.ending);
            }
        }
        let looping: Vec<_> = loops.iter().filter(|l| l.0 != u32::MAX).collect();
        if !looping.is_empty() {
            bump("looping songs");
            if looping.len() != loops.len() {
                bump("songs mixing looping and ending tracks");
            }
            if looping.windows(2).any(|w| w[0] != w[1]) {
                bump("looping songs whose tracks loop differently");
                println!("song {id:#05x}: loops {loops:?}");
            }
        }
    }
    for (k, v) in &n {
        println!("{k:>60}: {v}");
    }
    println!("values >= 128: {big:?}");
    println!("tempos: {tempos:?}");
}
