# nettai-luau on wasm32-unknown-unknown

The content compiler and interpreter run in a single Wasm module. The runtime
does not require JavaScript bindings, Emscripten, or a WASI host. The rest of the
workspace is not ported by this change.

## Build

Use nightly Rust with rust-src and the `wasm32-unknown-unknown` target (the standard library is built again
with atomics), and [WASI SDK 34](https://github.com/WebAssembly/wasi-sdk/releases/tag/wasi-sdk-34).
Extract the SDK for your build machine and set `WASI_SDK_PATH` to its directory (in the environment, or in
`~/.cargo/config.toml`'s `[env]`). The target's flags (atomics, a shared memory) and its standard library built
again with them are in .cargo/config.toml, the browser app's (crates/nettai-web).
The SDK supplies Clang, libc++, and libc at build time; its WASI imports and
allocator are removed from the libc copy linked into the module.

```sh
rustup target add wasm32-unknown-unknown
export WASI_SDK_PATH=/path/to/wasi-sdk-34.0-arm64-macos

bash crates/nettai-luau/cargo-wasm.sh build
# Or: cargo-wasm.sh check / cargo-wasm.sh build --release
```

The wrapper just runs Cargo with the required target. Both `mlua` 0.12.2 and Luau are unmodified crates.io dependencies. There
are no source patches, generated dependency copies, or alternate lockfiles.

Luau is built with `LUA_USE_LONGJMP`: a script error is a `longjmp` (Wasm
exception handling's, `-mllvm -wasm-enable-sjlj`) that crosses Rust callback
frames, as mlua's errors do with every C Lua. Luau's compiler throws and catches
C++ exceptions inside C++ alone. Rust takes no part in either, so the module is
an ordinary `panic=abort` build with the prebuilt `std`; Slint's and winit's web
backends need that (under `panic=unwind` wasm-bindgen asks their closures to be
`UnwindSafe`, and winit has no `run_app` with Rust's `exception-handling`
feature). A Rust panic traps. When using this crate from another workspace,
copy the root `[patch.crates-io]` entry for the small `libc` shim described
below.

Leave the `jit` feature off. Use a Wasm engine with standard exception handling
(`try_table`); the legacy exception encoding is not used.

## Test

Install Node.js and the test runner version matching the pinned
`wasm-bindgen-test` dependency:

```sh
cargo install wasm-bindgen-cli --version 0.2.126 --locked
bash crates/nettai-luau/test-wasm.sh
bash crates/nettai-luau/test-wasm.sh release
```

The script runs the crate's existing tests in Node, including content loading,
definition binding, freezing, bytecode verification, and coverage. Additional
tests exercise script/compiler/callback errors, interrupt budgets, 32-bit integer
conversions, and interleaved C/Rust allocations across memory growth.

It also builds `examples/wasm_smoke.rs` as a `cdylib` and instantiates it directly
with the WebAssembly API. This check asserts **zero imports** and repeatedly
compiles content, catches script and callback errors, and runs the VM. This
example does not use wasm-bindgen or a WASI shim.

## Integration details

- On this target, `mlua-sys/external` delegates the C++ build to `build.rs`.
  Native targets retain mlua's normal build.
- C and C++ allocations use Rust's global allocator, including libc's internal
  allocation aliases. The VM already uses mlua's Rust allocator.
- The small retained OS-facing portion of libc has no host capabilities:
  the environment is empty, clock queries are unsupported, file descriptor
  operations report bad descriptors, and process exit traps. The content
  sandbox exposes neither OS functions nor Luau's private random generator.
- The SDK's own libunwind defines the C++ exception tag, and its libsetjmp
  the `longjmp` tag.
- The [libc shim](wasm/libc/src/lib.rs) re-exports unmodified upstream `libc`
  and adds the file APIs missing on bare Wasm. `tmpfile()` returns null, so
  stock mlua's `heap_dump()` returns its existing "unable to dump heap" error.
  These Rust functions do not override the SDK's C stdio symbols. Other targets
  simply use upstream libc's API. The upstream dependency is pinned to its
  official release in Git so Cargo does not recursively apply the crates.io
  patch to it; no libc sources are checked into this repository.
- C++ constructors run once per module instance, before the first content
  compilation or VM creation. Exported functions can be called repeatedly.
- `Pack::new` and `Pack::of` accept modules supplied by the host. This work does
  not make filesystem-backed module providers or the other crates portable.

The build intentionally fails if newly retained C code needs another OS
operation, instead of silently adding a host import.
