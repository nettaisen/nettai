//! Summarize MIDI files: division, tracks, and how many events of each
//! kind (to see what an editor kept or dropped).
//!
//!     cargo run -p nettai-content --example midi_summary -- a.mid b.mid

use nettai_content::midi::{Message, Smf};
use std::collections::BTreeMap;

fn main() {
    for path in std::env::args().skip(1) {
        let smf = Smf::from_bytes(&std::fs::read(&path).unwrap()).unwrap();
        let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
        let mut off_grid = 0;
        for t in &smf.tracks {
            for e in &t.events {
                if !(e.tick as u64 * 24).is_multiple_of(smf.division as u64) {
                    off_grid += 1;
                }
                let k = match &e.message {
                    Message::NoteOn { .. } => "note on".to_string(),
                    Message::NoteOff { .. } => "note off".to_string(),
                    Message::Control { controller, .. } => format!("CC {controller}"),
                    Message::Program { .. } => "program".into(),
                    Message::PitchBend { .. } => "pitch bend".into(),
                    Message::Tempo(_) => "tempo".into(),
                    Message::Marker(m) => format!("marker {m:?}"),
                    Message::TrackName(_) => "track name".into(),
                    Message::Text(_) => "text".into(),
                    Message::TimeSignature(_) => "time signature".into(),
                    Message::Other(raw) => format!("other {:02x}", raw[0]),
                };
                *kinds.entry(k).or_default() += 1;
            }
        }
        println!("{path}: format {}, {} per quarter, {} tracks, {off_grid} events off the 1/96 grid", smf.format, smf.division, smf.tracks.len());
        for (k, n) in kinds {
            println!("  {k}: {n}");
        }
    }
}
