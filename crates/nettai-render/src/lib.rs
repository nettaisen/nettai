//! Picks nettai's battles into frames.
//!
//! Everything on screen is drawn from engine state (panels, objects with
//! their sprite, animation frame and look, HP, the custom gauge) and the
//! content packs' graphics (`exe6-extract content`); nothing emulates the
//! original's hardware. The frame is the original's 240x160, composed with
//! its layer and sprite ordering rules ([`Renderer::render`]), with the
//! font mode's text items to draw over it at the output's resolution
//! ([`present`], which also writes a frame as a PNG). A chip's pictures
//! on their own are [`pictures`]. docs/frontend.md §3.
//!
//! Drawing only: the window, the input, the sound and the sessions that
//! run a battle are nettai-frontend's and its host's (nettai-demo).

pub mod audit;
pub mod chatbox;
pub mod compose;
pub mod custom;
pub mod fonts;
pub mod hud;
pub mod lookups;
pub mod objects;
pub mod packs;
pub mod pictures;
pub mod present;
pub mod render;
pub mod stage;
pub mod strings;
pub mod textlayer;
pub mod vfont;

pub use render::{Frame, Region, Renderer};
