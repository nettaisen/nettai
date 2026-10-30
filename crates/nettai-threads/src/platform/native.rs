//! Natively (the desktop, a phone): std's threads and locks.

use std::sync::{LockResult, Mutex, MutexGuard};

pub fn lock<T: ?Sized>(mutex: &Mutex<T>) -> LockResult<MutexGuard<'_, T>> {
    mutex.lock()
}

pub fn spawn(name: &str, work: impl FnOnce() + Send + 'static) -> std::io::Result<()> {
    std::thread::Builder::new().name(name.to_string()).spawn(work).map(drop)
}
