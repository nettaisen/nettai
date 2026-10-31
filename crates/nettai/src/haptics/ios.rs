//! iOS: a light impact (`UIImpactFeedbackGenerator`), from the main thread, the window's, where the pad's touches
//! come. The generator is made once and kept prepared, so the next tap is felt as the finger lands. UIKit plays
//! nothing where System Haptics is off.

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_ui_kit::{UIImpactFeedbackGenerator, UIImpactFeedbackStyle};
use std::cell::OnceCell;

thread_local! {
    static GENERATOR: OnceCell<Retained<UIImpactFeedbackGenerator>> = const { OnceCell::new() };
}

pub fn tap() {
    let Some(main) = MainThreadMarker::new() else {
        return log::warn!("a haptic tap: not on the main thread");
    };
    GENERATOR.with(|generator| {
        // (`initWithStyle:` is deprecated since iOS 17.5 for the one attached to a view, which the app hasn't: winit
        // holds the window's. It still plays.)
        #[allow(deprecated)]
        let generator = generator.get_or_init(|| UIImpactFeedbackGenerator::initWithStyle(main.alloc(), UIImpactFeedbackStyle::Light));
        generator.impactOccurred();
        generator.prepare();
    });
}
