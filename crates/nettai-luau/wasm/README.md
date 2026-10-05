# Wasm C++ unwinder

`Unwind-wasm.c` is from LLVM 23.1.0:

<https://github.com/llvm/llvm-project/blob/llvmorg-23.1.0/libunwind/src/Unwind-wasm.c>

It is distributed under Apache-2.0 WITH LLVM-exception; see `LICENSE-LLVM`.
Two adaptations are kept local:

- Replace the internal libunwind `config.h` with standard includes, export
  visibility, and disabled tracing. The SDK provides its public unwind headers.
- Omit the definition of `__cpp_exception`. Rust's `unwind` crate defines that
  tag for `wasm32-unknown-unknown` with `panic=unwind`. Both languages must throw
  and catch using the same tag, and linking the SDK's unmodified libunwind would
  define it twice.

The personality implementation comes from the SDK's libc++abi. This runtime
uses standard Wasm exceptions, not setjmp/longjmp or JavaScript exceptions.
