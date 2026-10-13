//! Opponents for training: AIs that play a navi on buttons alone, as a
//! player does, for a host to plug into live play
//! (`nettai_frontend::driver::LivePlayer::set_ai`). The library plays a set;
//! what plays its opponent is here.

pub mod fighter;

pub use fighter::{Fighter, Level, Spent};
