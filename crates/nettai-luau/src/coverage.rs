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
use std::collections::{BTreeSet, HashSet};
use std::ffi::CStr;

use mlua::{Lua, ffi};
use nettai_content_api::FnId;

/// What ran while recording.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ran {
    /// The modules a function of which ran (returned, or reached an
    /// interrupt check: a call, a return or a loop's back edge), by name
    /// (`exe6:chips/elemtrap/strike`). The define phase isn't counted
    /// unless it ran while recording (a runtime loaded then).
    pub modules: BTreeSet<String>,
    /// The functions the engine called (a kind's or an action's update, a
    /// hook), by the plan's index.
    pub functions: BTreeSet<FnId>,
}

thread_local! {
    static RECORDING: Cell<bool> = const { Cell::new(false) };
    static RAN: RefCell<Ran> = RefCell::new(Ran::default());
    /// The chunk names (interned strings, one per module) already counted
    /// since [`start`], by address: an interrupt check whose function's
    /// module is counted does no more than look it up.
    static SEEN: RefCell<HashSet<usize>> = RefCell::new(HashSet::new());
}

/// Start recording what runs on this thread (from nothing).
pub fn start() {
    RAN.with(|r| *r.borrow_mut() = Ran::default());
    SEEN.with(|s| s.borrow_mut().clear());
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
    let mut ar: ffi::lua_Debug = unsafe { std::mem::zeroed() };
    // SAFETY: the running state, level 0 (the function the check is in),
    // what `s` fills in: its chunk's name, which lives as long as the
    // function's prototype.
    if unsafe { ffi::lua_getinfo(lua.state(), 0, c"s".as_ptr(), &mut ar) } == 0 || ar.source.is_null() {
        return;
    }
    let key = ar.source as usize;
    if SEEN.with(|s| !s.borrow_mut().insert(key)) {
        return;
    }
    // SAFETY: as above; a NUL-terminated string.
    let source = unsafe { CStr::from_ptr(ar.source) }.to_string_lossy();
    // The chunk's name: `@<module>.luau`.
    let module = source.strip_prefix('@').unwrap_or(&source);
    let module = module.strip_suffix(".luau").unwrap_or(module);
    RAN.with(|r| r.borrow_mut().modules.insert(module.to_string()));
}
