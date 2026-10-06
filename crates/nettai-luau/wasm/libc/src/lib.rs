//! Keep upstream libc's API, adding only the file operations needed to compile
//! mlua's heap dumper on bare Wasm. No files can be opened on this target.
#![no_std]

pub use libc_upstream::*;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub use unavailable_files::*;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
mod unavailable_files {
    use core::ffi::{c_int, c_long, c_void};

    pub enum FILE {}
    pub const SEEK_END: c_int = 2;

    // These are Rust crate APIs, not exported C symbols. They do not replace
    // the SDK's stdio implementation. mlua checks this null and returns its
    // normal "unable to dump heap" error before calling any other file API.
    pub unsafe extern "C" fn tmpfile() -> *mut FILE {
        core::ptr::null_mut()
    }

    pub unsafe extern "C" fn fseek(_: *mut FILE, _: c_long, _: c_int) -> c_int {
        -1
    }

    pub unsafe extern "C" fn ftell(_: *mut FILE) -> c_long {
        -1
    }

    pub unsafe extern "C" fn rewind(_: *mut FILE) {}

    pub unsafe extern "C" fn fread(_: *mut c_void, _: usize, _: usize, _: *mut FILE) -> usize {
        0
    }

    pub unsafe extern "C" fn fclose(_: *mut FILE) -> c_int {
        -1
    }
}
