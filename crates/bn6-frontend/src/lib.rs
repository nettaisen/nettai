//! A native renderer for the BN6 battle engine.
//!
//! Everything on screen is drawn from engine state (panels, objects with
//! their sprite, animation frame and look, HP, the custom gauge) and a
//! content pack's graphics (`bn6-extract content`); nothing emulates the
//! original's hardware. The frame is the original's 240x160, composed with
//! its layer and sprite ordering rules.

pub mod app;
pub mod audit;
pub mod compose;
pub mod custom;
pub mod driver;
pub mod fonts;
pub mod headless;
pub mod hud;
pub mod objects;
pub mod render;
pub mod session;
pub mod stage;
pub mod text;

pub use render::Renderer;
pub use session::{Session, TickHook};
