//! A native renderer for nettai's battles.
//!
//! Everything on screen is drawn from engine state (panels, objects with
//! their sprite, animation frame and look, HP, the custom gauge) and a
//! content pack's graphics (`bn6-extract content`); nothing emulates the
//! original's hardware. The frame is the original's 240x160, composed with
//! its layer and sprite ordering rules.

pub mod app;
pub mod audit;
pub mod chatbox;
pub mod compose;
pub mod custom;
pub mod driver;
pub mod fonts;
pub mod headless;
pub mod hud;
pub mod netplay;
pub mod objects;
pub mod present;
pub mod render;
pub mod session;
pub mod stage;
pub mod text;
pub mod textlayer;
pub mod vfont;
pub mod strings;

pub use render::{Frame, Renderer};
pub use session::{Session, TickHook};
