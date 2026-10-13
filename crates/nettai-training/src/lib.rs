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
//! three minutes, or stopped by the engine), and the HP it took away and
//! lost a minute.
//!
//! | Game | Random presses | Easy | Normal | Hard |
//! |---|---|---|---|---|
//! | EXE6 | 11–13, 298 / 334 | 11–13, 320 / 319 | 15–9, 305 / 277 | 18–6, 344 / 223 |
//! | EXE5 | 9–14, 144 / 202 | 12–8, 150 / 161 | 15–8, 179 / 133 | 15–4, 179 / 98 |
//! | EXE4 | 13–6, 155 / 129 | 8–10, 160 / 159 | 9–10, 160 / 156 | 15–7, 186 / 129 |
//!
//! Each level against the one below it (`FIGHTER_PLAYER`), EXE6, 12
//! rounds: Hard beat Normal 8–4 (333 / 229), Normal beat Easy 8–4 (322 /
//! 221).
//!
//! A decision averages 18 ms (EXE6) to 55 ms (EXE4, EXE5) at Hard, on three
//! threads, four rounds at once on a loaded machine. Run at the battle's own
//! speed (`fighter::tests::real_time`, 60 ticks a second, 20 seconds of each
//! game at each level), every decision was ready on its tick: 8 to 44 ms on
//! average, 82 ms at most (Hard's reaction is 100 ms).
//!
//! In EXE4 about one round in twelve of these was stopped by the engine:
//! a player's chip request outlives a dimming that took the chip, and is
//! taken after it with the hand empty ("the empty hand's chip reads past
//! the chip table"; seed 16 at Normal, tick 514, the player's side).
//!
//! The measurements: `cargo test -p nettai-training --lib -- --ignored
//! --nocapture` with the test's name (`against_random_presses`,
//! `real_time`, `tick_cost`); their variables are in their docs.

pub mod fighter;

pub use fighter::{Fighter, Level, Spent};
