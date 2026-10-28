//! Picks nettai's battles into frames.
//!
//! Everything on screen is drawn from engine state (panels, objects with
//! their sprite, animation frame and look, HP, the custom gauge) and the
//! game's graphics, extracted from its ROMs (nettai-extract); nothing emulates the
//! original's hardware. The frame is the original's 240x160, composed with
//! its layer and sprite ordering rules ([`Renderer::render`]), with the
//! enhanced mode's text items to draw over it at the output's resolution
//! ([`present`], which also writes a frame as a PNG). A chip's pictures
//! on their own are [`pictures`]. A frame shows one console's screen, or
//! both players' on one ([`Consoles`], [`seat`]). docs/frontend.md §3.
//!
//! Drawing only: the window, the input, the sound and the sessions that
//! run a battle are nettai-frontend's and its host's (nettai, nettai-tools).

#![forbid(unsafe_code)]

pub mod audit;
pub mod bundled;
pub mod chatbox;
pub mod compose;
pub mod custom;
pub mod fonts;
pub mod hud;
pub mod lettering;
pub mod lookups;
pub mod objects;
pub mod graphics;
pub mod pictures;
pub mod pixeltext;
pub mod rasters;
pub mod present;
pub mod render;
pub mod sdf;
pub mod seat;
pub mod shapes;
pub mod smooth;
pub mod stage;
pub mod still;
pub mod strings;
pub mod textlayer;
pub mod numbers;
pub mod vfont;

pub use render::{Frame, Glide, GraphicsMode, Renderer};
pub use seat::Consoles;
