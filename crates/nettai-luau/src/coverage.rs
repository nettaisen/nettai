//! What content ran on a thread, recorded on request: the modules whose
//! code ran and the functions the engine called. A tool that replays
//! recordings uses it to learn which content each one exercises, so that
//! after a change to some modules it can replay only the recordings that
//! ran them (the verification workspace's impact selection).
//!
//! Recording is per thread and off unless [`start`]ed; off, it costs a
//! thread-local read per interrupt check and per call. It doesn't change
//! what content does.

use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;

use mlua::Lua;
use nettai_content_api::FnId;

/// What ran while recording.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ran {
    /// The modules a function of which ran (returned, or reached an
    /// interrupt check: a call, a return or a loop's back edge), by name
    /// (`bn6:chips/elemtrap/strike`). The define phase isn't counted
    /// unless it ran while recording (a runtime loaded then).
    pub modules: BTreeSet<String>,
    /// The functions the engine called (a kind's or an action's update, a
    /// hook), by the plan's index.
    pub functions: BTreeSet<FnId>,
}

thread_local! {
    static RECORDING: Cell<bool> = const { Cell::new(false) };
    static RAN: RefCell<Ran> = RefCell::new(Ran::default());
}

/// Start recording what runs on this thread (from nothing).
pub fn start() {
    RAN.with(|r| *r.borrow_mut() = Ran::default());
    RECORDING.with(|r| r.set(true));
}

/// Stop recording on this thread, and take what ran since [`start`].
pub fn take() -> Ran {
    RECORDING.with(|r| r.set(false));
    RAN.with(|r| std::mem::take(&mut *r.borrow_mut()))
}

pub(crate) fn recording() -> bool {
    RECORDING.with(Cell::get)
}

/// The engine calls function `f`.
pub(crate) fn called(f: FnId) {
    RAN.with(|r| r.borrow_mut().functions.insert(f));
}

/// An interrupt check: the running function's module ran.
pub(crate) fn interrupt(lua: &Lua) {
    lua.inspect_stack(0, |d| {
        let source = d.source().source;
        let Some(source) = source.as_deref() else { return };
        // The chunk's name: `@<module>.luau`.
        let module = source.strip_prefix('@').unwrap_or(source);
        let module = module.strip_suffix(".luau").unwrap_or(module);
        RAN.with(|r| {
            let mut r = r.borrow_mut();
            if !r.modules.contains(module) {
                r.modules.insert(module.to_string());
            }
        });
    });
}
