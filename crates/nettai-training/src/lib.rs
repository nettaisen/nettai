//! Opponents for training: AIs that play a navi on buttons alone, as a
//! player does, for a host to plug into live play
//! (`nettai_frontend::driver::LivePlayer::set_ai`). The library plays a set;
//! what plays its opponent is here.
//!
//! # How well the fighter plays
//!
//! Measured 2026-10-13 (`fighter::tests::against_random_presses`): the
//! first round of 24 matches of each game picked from seeds 0 to 23, both
//! sides the same build at 400 HP, the left navi pressing at random (live
//! play's "attacks at random", picking every chip it can), the right one
//! the fighter at each level; rounds won and lost (the rest undecided in
//! three minutes), and the HP it took away and lost a minute.
//!
//! | Game | Random presses | Easy | Normal | Hard |
//! |---|---|---|---|---|
//! | EXE6 | 11–13, 298 / 334 | 16–7, 334 / 245 | 20–4, 363 / 231 | 20–4, 362 / 164 |
//! | EXE5 | 9–14, 144 / 202 | 11–9, 164 / 169 | 12–7, 157 / 132 | 15–4, 173 / 97 |
//! | EXE4 | 13–6, 155 / 129 | 12–7, 153 / 131 | 15–4, 176 / 121 | 19–2, 181 / 93 |
//!
//! Against each other (`FIGHTER_PLAYER`), EXE6, 16 rounds: Hard beat Normal
//! 12–4 (311 / 202), Normal beat Easy 10–6 (314 / 276). The user found Hard
//! "not really that hard": against the Hard before it, which counted the
//! most it could lose in any guess, used no chip it didn't have to and
//! decided on its beat alone, today's beat it 12–3 in EXE6 (381 / 208), 9–3
//! in EXE5 and 6–5 in EXE4, and holds a chip some 9 seconds at most where
//! that one held them 25 to 56.
//!
//! Run at the battle's own speed (`fighter::tests::real_time`, 60 ticks a
//! second, 20 seconds of each game at each level, three threads), every
//! decision was ready on its tick: 5 to 35 ms on average, 91 ms at most
//! (Hard's reaction is 100 ms).
//!
//! In EXE4 a round could stop in the engine where a chip request outlived a
//! dimming that took the chip; fixed in the engine (docs/engine/chips.md
//! §2.9): none of these rounds stopped.
//!
//! The measurements: `cargo test -p nettai-training --lib -- --ignored
//! --nocapture` with the test's name (`against_random_presses`,
//! `real_time`, `tick_cost`); their variables are in their docs.

pub mod fighter;

pub use fighter::{Fighter, Level, Spent};
