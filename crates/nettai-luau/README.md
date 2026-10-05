# nettai-luau on wasm32-unknown-unknown

The content compiler and interpreter run in a single Wasm module. The runtime
does not require JavaScript bindings, Emscripten, or a WASI host. The rest of the
workspace is not ported by this change.

## Build

Use Rust nightly with `rust-src`, and [WASI SDK 34](https://github.com/WebAssembly/wasi-sdk/releases/tag/wasi-sdk-34).
Extract the SDK for your build machine and set `WASI_SDK_PATH` to its directory.
The SDK supplies Clang, libc++, and libc at build time; its WASI imports and
allocator are removed from the libc copy linked into the module.

```sh
rustup component add rust-src --toolchain nightly
rustup target add wasm32-unknown-unknown --toolchain nightly
export WASI_SDK_PATH=/path/to/wasi-sdk-34.0-arm64-macos

bash crates/nettai-luau/cargo-wasm.sh build
# Or: cargo-wasm.sh check / cargo-wasm.sh build --release
```

The wrapper just runs Cargo with the required target and standard-library build
flags. Both `mlua` 0.12.2 and Luau are unmodified crates.io dependencies. There
are no source patches, generated dependency copies, or alternate lockfiles.

The repository's `.cargo/config.toml` enables standard Wasm exception handling
and `panic=unwind` for this target. Rebuilding `std` is required: Luau implements
script errors with C++ exceptions, which cross Rust callback frames. With
`panic=abort` those callbacks abort instead of returning an error. When using
this crate from another workspace, copy those target rustflags and the root
`[patch.crates-io]` entry for the small `libc` shim described below.

Leave the `jit` feature off. Use a Wasm engine with standard exception handling
(`try_table`); the legacy exception encoding is not used. Rust panics require
the same unwinding support, and an uncaught panic at a non-unwinding exported
function still traps normally.

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
- LLVM's Wasm unwinder is built with Rust supplying the single exception tag;
  see [wasm/README.md](wasm/README.md).
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
