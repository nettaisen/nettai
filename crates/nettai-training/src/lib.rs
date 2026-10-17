//! Opponents for training: AIs that play a navi on buttons alone, as a
//! player does, for a host to plug into live play
//! (`nettai_frontend::driver::LivePlayer::set_ai`). The library plays a set;
//! what plays its opponent is here.
//!
//! # How well the fighter plays
//!
//! Measured 2026-10-16 (release build, the repository's content on the
//! tests' made-up asset index, every sprite timed alike; `FIGHTER_PACKS`
//! plays the packs' own timings), the fighter that plans by panels
//! ([`fighter`]'s docs). Only the player opens the custom screen (live play
//! clears the dummy's L and R), so the tests' players open it.
//!
//! Against a player pressing at random (live play's "attacks at random",
//! picking every chip it can), the first round of 24 matches of each game,
//! both sides the same build at 400 HP: rounds won and lost (the rest
//! undecided in three minutes), and the HP it took away and lost a minute
//! (`fighter::tests::against_random_presses`).
//!
//! | Game | Random presses | Easy | Normal | Hard |
//! |---|---|---|---|---|
//! | EXE6 | 9–14, 245 / 307 | 18–5, 332 / 219 | 23–0, 403 / 126 | 21–3, 459 / 149 |
//! | EXE5 | 11–10, 135 / 155 | 13–5, 157 / 103 | 20–2, 288 / 81 | 21–2, 299 / 72 |
//! | EXE4 | 10–12, 179 / 190 | 15–6, 199 / 161 | 23–1, 302 / 102 | 23–1, 353 / 89 |
//!
//! Head to head (`FIGHTER_PLAYER`), 16 rounds: Hard beat Normal 7–5 in
//! EXE6 (4 undecided), Normal beat Easy 14–0 (2 undecided).
//!
//! Game sense against scripted players (`fighter::tests::game_sense`), 12
//! rounds a game, a minute of the fight: a *shooter* that walks to the
//! dummy's row and shoots there every 12 ticks and uses its chip there at
//! once; a *dodger* that steps up or down every 40 ticks and never attacks.
//! Hits taken from the shooter; its attack chips a minute and the share
//! that landed (the player lost HP within two seconds); its steps back to
//! the panel it just left within a third of a second a minute (pacing), and
//! against the dodger; Crosses, Beast Outs and souls taken (12 rounds):
//!
//! | Game | Dummy | Hits taken | Chips, landed (shooter) | Chips, landed (dodger) | Steps back (shooter / dodger) | Forms |
//! |---|---|---|---|---|---|---|
//! | EXE6 | Easy | 80 | 10.1, 46% | 10.4, 39% | 1.3 / 9.1 | 0 |
//! | EXE6 | Normal | 54 | 10.0, 42% | 10.3, 42% | 0.6 / 9.0 | 24 |
//! | EXE6 | Hard | 48 | 10.7, 45% | 10.4, 43% | 2.0 / 1.8 | 24 |
//! | EXE5 | Easy | 91 | 7.6, 51% | 7.9, 44% | 7.7 / 7.7 | 0 |
//! | EXE5 | Normal | 46 | 6.8, 64% | 7.5, 65% | 1.0 / 1.8 | 33 |
//! | EXE5 | Hard | 48 | 7.0, 65% | 8.0, 59% | 3.6 / 6.8 | 31 |
//! | EXE4 | Easy | 70 | 10.1, 30% | 11.3, 33% | 1.0 / 13.6 | 0 |
//! | EXE4 | Normal | 64 | 15.0, 48% | 13.5, 49% | 1.8 / 14.3 | 25 |
//! | EXE4 | Hard | 56 | 17.2, 42% | 13.7, 52% | 3.4 / 9.2 | 31 |
//!
//! Against the shooter Hard won 12–0, 10–0 and 10–0, taking 385, 113 and
//! 181 HP a minute. Against the shooter its chips that don't attack
//! (Barrier, Recover and the like, 0.6 to 2 a minute) were all used within a
//! second of a hit taken or the player pressing A or B; against the dodger,
//! which never threatens, it used them after holding them three seconds (a
//! chip held long is used, so it bars none behind it). The levels paced 0
//! to 4% of the fight.
//!
//! Run at the battle's own speed (`fighter::tests::real_time`, 60 ticks a
//! second, 20 seconds of each game at each level, four threads), every
//! decision was ready on its tick: 14 to 61 ms on average, 126 ms at most
//! (Hard's reaction is 150 ms, Normal's 233). On the packs' timings, Hard:
//! one decision late in EXE5's 20 seconds (7 ms waited), 150 ms at most.
//!
//! Since then a custom screen's hand is valued over a turn (the tables
//! above are from before), its work the level's budget
//! ([`fighter::Level::budget`]) rather than a time, so the hand doesn't
//! depend on the machine. Fifteen screens (five a game, Hard's budget,
//! release build, four threads, the tests' content) took 0.2 to 0.8 s in
//! EXE6 and 0.4 to 2 s in EXE5 and EXE4 (a tick of theirs costs more), its
//! look time 0.75 s at Hard and 1 s at Normal: past it the battle goes on
//! and the dummy looks longer.
//!
//! The measurements: `cargo test -p nettai-training --release --lib --
//! --ignored --nocapture` with the test's name (`against_random_presses`,
//! `game_sense`, `real_time`, `tick_cost`); their variables are in their
//! docs.

pub mod fighter;

pub use fighter::{Fighter, Level, Spent};
