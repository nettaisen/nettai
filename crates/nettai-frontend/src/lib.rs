//! A native renderer for nettai's battles.
//!
//! Everything on screen is drawn from engine state (panels, objects with
//! their sprite, animation frame and look, HP, the custom gauge) and a
//! content pack's graphics (`bn6-extract content`); nothing emulates the
//! original's hardware. The frame is the original's 240x160, composed with
//! its layer and sprite ordering rules.

pub mod app;
pub mod content_audit;
pub mod driver;
pub mod headless;
pub mod netplay;
pub mod session;
pub mod sound_lookups;
pub mod text;

pub use nettai_render::{audit, chatbox, compose, custom, fonts, hud, lookups, objects, packs, present, render, stage, strings, textlayer, vfont};

pub use render::{Frame, Renderer};
pub use session::{Session, TickHook};
