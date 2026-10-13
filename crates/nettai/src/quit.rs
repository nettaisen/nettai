//! A terminate request from outside the app (the system's SIGTERM; SIGINT, a terminal's Ctrl+C): the app closes
//! as its window's close button closes it.
//!
//! The gamepads' library starts SDL, and SDL takes both signals for itself and turns each into an event of its
//! own queue. Nothing of the app reads that event (the library hands on a pad's events alone), so a terminate
//! request did nothing at all, and only a kill ended the app. So the app takes the two signals itself, after
//! SDL has started: its handler writes a byte to a pipe, which is all a handler may safely do, and a thread
//! waiting on the pipe has the window's event loop end, on the window's own thread. `main` then returns as it
//! does when the last window closes, and what the app holds is dropped in order (a recording's file flushed, a
//! link closed). The settings are written as each changes: there is nothing left to save.
//!
//! A second request, while the app is closing, ends it as the system would have: the handlers are the
//! system's own again by then.

/// Whether a terminate request from outside ended the app's window loop (the screenshot pass says so in its
/// report, when one ends it before its walk is done).
pub fn asked() -> bool {
    ASKED.load(std::sync::atomic::Ordering::Relaxed)
}

static ASKED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Take terminate requests from here on. Called once, after the gamepads' library has started (it is SDL's
/// start that takes the signals: the last to take them has them).
#[cfg(unix)]
pub fn watch() {
    use std::sync::atomic::{AtomicI32, Ordering};

    /// The pipe's writing end, for the handler.
    static WAKE: AtomicI32 = AtomicI32::new(-1);

    extern "C" fn asked(_signal: libc::c_int) {
        let to = WAKE.load(Ordering::Relaxed);
        if to >= 0 {
            // SAFETY: `write` is one of the calls a signal's handler may make, on a descriptor that is never
            // closed, from a byte of the handler's own.
            unsafe { libc::write(to, [1u8].as_ptr().cast(), 1) };
        }
    }

    const SIGNALS: [libc::c_int; 2] = [libc::SIGTERM, libc::SIGINT];
    let mut ends = [0 as libc::c_int; 2];
    // SAFETY: `pipe` fills the two descriptors it is given room for.
    if unsafe { libc::pipe(ends.as_mut_ptr()) } != 0 {
        return crate::errors::log("taking terminate requests", &std::io::Error::last_os_error());
    }
    WAKE.store(ends[1], Ordering::Relaxed);
    for signal in SIGNALS {
        // SAFETY: the handler does nothing but the one `write` above.
        unsafe { libc::signal(signal, asked as extern "C" fn(libc::c_int) as libc::sighandler_t) };
    }
    let from = ends[0];
    std::thread::spawn(move || {
        let mut byte = [0u8; 1];
        loop {
            // SAFETY: a read of one byte into a buffer of one, from the pipe's reading end.
            let read = unsafe { libc::read(from, byte.as_mut_ptr().cast(), 1) };
            if read < 0 && std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            break;
        }
        ASKED.store(true, Ordering::Relaxed);
        // (A second request isn't waited out.)
        for signal in SIGNALS {
            // SAFETY: the system's own handling, put back.
            unsafe { libc::signal(signal, libc::SIG_DFL) };
        }
        // On the window's thread: its loop ends, and `main` goes on to its end.
        let _ = slint::invoke_from_event_loop(|| {
            let _ = slint::quit_event_loop();
        });
    });
}

/// Elsewhere nothing takes them from the app, and the system's own handling stands.
#[cfg(not(unix))]
pub fn watch() {}
