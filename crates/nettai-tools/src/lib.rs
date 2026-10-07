//! nettai's tools without a window, over the nettai-frontend library: what
//! the developer and the verification run (the command line: `main.rs`,
//! `nettai-tool`). Chosen frames of a battle written to PNGs
//! ([`headless`]); the audits: the content's every lookup
//! ([`content_audit`]), a trace's frames' and cues' ([`headless::audit_traces`],
//! with the audio's lookups, [`sound_lookups`]); the replay of the
//! original's recordings ([`trace`], with the compat crates).

pub mod content_audit;
pub mod headless;
pub mod sound_lookups;
pub mod trace;
