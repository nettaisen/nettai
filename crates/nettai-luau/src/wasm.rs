//! C/C++ allocations share Rust's allocator. A second libc heap would
//! independently grow into Rust's Wasm linear memory.

use std::alloc::{self, Layout};
use std::ffi::{c_int, c_void};
use std::ptr;

const ALIGN: usize = 16;

/// Register Luau's C++ statics once per module instance. An explicit reference
/// also tells wasm-ld not to wrap every export with constructors/destructors
/// (its default "command" behavior would re-register Luau flags on each call).
pub(crate) fn initialize() {
    static INIT: std::sync::Once = std::sync::Once::new();
    unsafe extern "C" {
        fn __wasm_call_ctors();
    }
    INIT.call_once(|| unsafe { __wasm_call_ctors() });
}

// The sandbox exposes neither os.clock nor math.random. Luau still asks for
// a clock while seeding its private PRNG at startup; no host clock is needed.
#[unsafe(no_mangle)]
extern "C" fn clock() -> i64 {
    0
}

#[derive(Clone, Copy)]
#[repr(C)]
struct Header {
    size: usize,
    align: usize,
}

fn layout(size: usize, align: usize) -> Option<Layout> {
    Layout::from_size_align(size.max(1).checked_add(align)?, align).ok()
}

unsafe fn allocate(size: usize, align: usize, zero: bool) -> *mut c_void {
    let Some(layout) = layout(size, align) else { return ptr::null_mut() };
    unsafe {
        let base = if zero { alloc::alloc_zeroed(layout) } else { alloc::alloc(layout) };
        if base.is_null() {
            return ptr::null_mut();
        }
        let data = base.add(align);
        data.cast::<Header>().sub(1).write(Header { size: layout.size(), align });
        data.cast()
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn malloc(size: usize) -> *mut c_void {
    unsafe { allocate(size, ALIGN, false) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn calloc(count: usize, size: usize) -> *mut c_void {
    let Some(size) = count.checked_mul(size) else { return ptr::null_mut() };
    unsafe { allocate(size, ALIGN, true) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn free(data: *mut c_void) {
    if data.is_null() {
        return;
    }
    unsafe {
        let h = data.cast::<Header>().sub(1).read();
        alloc::dealloc(data.cast::<u8>().sub(h.align), Layout::from_size_align_unchecked(h.size, h.align));
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn realloc(data: *mut c_void, size: usize) -> *mut c_void {
    unsafe {
        if data.is_null() {
            return malloc(size);
        }
        if size == 0 {
            free(data);
            return ptr::null_mut();
        }
        let h = data.cast::<Header>().sub(1).read();
        let Some(new) = layout(size, h.align) else { return ptr::null_mut() };
        let base = alloc::realloc(data.cast::<u8>().sub(h.align), Layout::from_size_align_unchecked(h.size, h.align), new.size());
        if base.is_null() {
            return ptr::null_mut();
        }
        let data = base.add(h.align);
        data.cast::<Header>().sub(1).write(Header { size: new.size(), align: h.align });
        data.cast()
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn posix_memalign(out: *mut *mut c_void, align: usize, size: usize) -> c_int {
    if !align.is_power_of_two() || align < size_of::<usize>() {
        return 28; // wasi-libc EINVAL
    }
    unsafe {
        let data = allocate(size, align.max(ALIGN), false);
        if data.is_null() {
            return 48; // wasi-libc ENOMEM
        }
        out.write(data);
        0
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn aligned_alloc(align: usize, size: usize) -> *mut c_void {
    if !align.is_power_of_two() || size % align != 0 {
        return ptr::null_mut();
    }
    unsafe { allocate(size, align.max(ALIGN), false) }
}

// musl's internal aliases must use the same heap too. Defining them keeps
// its dlmalloc archive member (and its independent sbrk heap) out of the link.
#[unsafe(no_mangle)]
unsafe extern "C" fn __libc_malloc(size: usize) -> *mut c_void {
    unsafe { malloc(size) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn __libc_calloc(count: usize, size: usize) -> *mut c_void {
    unsafe { calloc(count, size) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn __libc_free(data: *mut c_void) {
    unsafe { free(data) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn malloc_usable_size(data: *mut c_void) -> usize {
    if data.is_null() {
        return 0;
    }
    let h = unsafe { data.cast::<Header>().sub(1).read() };
    h.size - h.align
}

// Hostless implementations of the small OS-facing part of wasi-libc that
// its stdio, timezone, and C++ error-reporting code retains. These symbols
// prevent libc's WASI import object from being linked. No filesystem, clock,
// environment, or process host is required (or exposed to content).
#[unsafe(no_mangle)]
unsafe extern "C" fn __wasi_environ_sizes_get(count: *mut usize, bytes: *mut usize) -> u16 {
    unsafe {
        count.write(0);
        bytes.write(0);
    }
    0
}

#[unsafe(no_mangle)]
extern "C" fn __wasi_environ_get(_env: *mut *mut u8, _buf: *mut u8) -> u16 {
    0
}

#[unsafe(no_mangle)]
extern "C" fn __wasi_clock_time_get(_id: u32, _precision: u64, _time: *mut u64) -> u16 {
    52 // ENOSYS: there is no host clock.
}

#[unsafe(no_mangle)]
extern "C" fn __wasi_fd_close(_fd: u32) -> u16 {
    8 // EBADF: there are no host file descriptors.
}

#[unsafe(no_mangle)]
extern "C" fn __wasi_fd_fdstat_get(_fd: u32, _stat: *mut c_void) -> u16 {
    8
}

#[unsafe(no_mangle)]
extern "C" fn __wasi_fd_seek(_fd: u32, _offset: i64, _whence: u8, _position: *mut u64) -> u16 {
    8
}

#[unsafe(no_mangle)]
extern "C" fn __wasi_fd_write(_fd: u32, _iov: *const c_void, _count: usize, _written: *mut usize) -> u16 {
    8
}

#[unsafe(no_mangle)]
extern "C" fn __wasi_proc_exit(_code: u32) -> ! {
    std::arch::wasm32::unreachable()
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    fn heap_dump_reports_unavailable_and_leaves_the_vm_usable() {
        let lua = crate::sandbox::new_vm(false).unwrap();
        lua.load("kept = {1, 2, 3}").exec().unwrap();
        for _ in 0..8 {
            let error = match lua.heap_dump() {
                Ok(_) => panic!("bare Wasm has no temporary files for a heap dump"),
                Err(error) => error,
            };
            assert!(error.to_string().contains("unable to dump heap"), "{error}");
            assert_eq!(lua.load("return kept[1] + kept[2] + kept[3]").eval::<i64>().unwrap(), 6);
            lua.gc_collect().unwrap();
        }
    }

    #[wasm_bindgen_test]
    fn c_and_rust_allocations_survive_growth_and_reallocation() {
        unsafe {
            let p = malloc(64).cast::<u8>();
            assert!(!p.is_null());
            assert_eq!(p as usize % ALIGN, 0);
            p.write_bytes(0x5a, 64);
            // Force memory growth while both allocators have live objects.
            let rust = vec![0xa5u8; 8 * 1024 * 1024];
            let p = realloc(p.cast(), 4 * 1024 * 1024).cast::<u8>();
            assert!(!p.is_null());
            assert_eq!(std::slice::from_raw_parts(p, 64), &[0x5a; 64]);
            p.add(64).write_bytes(0x3c, 4 * 1024 * 1024 - 64);
            assert!(rust.iter().all(|b| *b == 0xa5));
            let p = realloc(p.cast(), 32).cast::<u8>();
            assert_eq!(std::slice::from_raw_parts(p, 32), &[0x5a; 32]);
            assert!(realloc(p.cast(), usize::MAX).is_null());
            assert_eq!(std::slice::from_raw_parts(p, 32), &[0x5a; 32]);
            free(p.cast());
            free(ptr::null_mut());
        }
    }

    #[wasm_bindgen_test]
    fn c_allocations_handle_zeroing_overflow_and_alignment() {
        unsafe {
            // Exercise the actual C entry points. LLVM otherwise recognizes
            // the libc names and folds constant allocation calls in this test.
            let calloc = std::hint::black_box(calloc as unsafe extern "C" fn(usize, usize) -> *mut c_void);
            let malloc = std::hint::black_box(malloc as unsafe extern "C" fn(usize) -> *mut c_void);
            let p = calloc(32, 8).cast::<u8>();
            assert!(!p.is_null());
            assert_eq!(std::slice::from_raw_parts(p, 256), &[0; 256]);
            free(p.cast());
            assert!(calloc(usize::MAX, 2).is_null());
            assert!(malloc(usize::MAX).is_null());
            assert!(aligned_alloc(3, 12).is_null());
            let mut p = ptr::null_mut();
            assert_eq!(posix_memalign(&mut p, 3, 256), 28);
            assert!(p.is_null());
            assert_eq!(posix_memalign(&mut p, 256, 1000), 0);
            assert_eq!(p as usize % 256, 0);
            p.cast::<u8>().write_bytes(0xc3, 1000);
            let p = realloc(p, 2000).cast::<u8>();
            assert!(!p.is_null());
            assert_eq!(p as usize % 256, 0);
            assert!(std::slice::from_raw_parts(p, 1000).iter().all(|b| *b == 0xc3));
            assert!(realloc(p.cast(), 0).is_null());
        }
    }
}
