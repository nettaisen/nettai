use std::{env, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if env::var("TARGET").as_deref() != Ok("wasm32-unknown-unknown") {
        return;
    }
    assert!(env::var_os("CARGO_FEATURE_JIT").is_none(), "Luau's native JIT is not supported on wasm32-unknown-unknown; disable the jit feature");
    println!("cargo:rerun-if-env-changed=WASI_SDK_PATH");
    let sdk = PathBuf::from(env::var_os("WASI_SDK_PATH").expect("set WASI_SDK_PATH to wasi-sdk 34 (see crates/nettai-luau/README.md)"))
        .canonicalize().expect("WASI_SDK_PATH must name an existing SDK directory");
    // A module with Wasm's atomics (the app's, whose threads are Web Workers on one shared memory: packaging/web)
    // takes the SDK's threads build of Luau's C++ and its runtime; any other the single-threaded one.
    let atomics = env::var("CARGO_CFG_TARGET_FEATURE").is_ok_and(|f| f.split(',').any(|f| f == "atomics"));
    let c_target = if atomics { "wasm32-wasip1-threads" } else { "wasm32-wasip1" };
    let cxx = sdk.join(format!("bin/clang++{}", env::consts::EXE_SUFFIX));
    let ar = sdk.join(format!("bin/llvm-ar{}", env::consts::EXE_SUFFIX));
    // Build scripts run in their own single-threaded process. Only the C++
    // build uses the SDK's target/sysroot; Rust and the final module are
    // wasm32-unknown-unknown.
    unsafe {
        env::set_var("CXX_wasm32_unknown_unknown", &cxx);
        env::set_var("AR_wasm32_unknown_unknown", &ar);
        // Luau's parser relies on transitive <cstdlib> includes that
        // newer libc++ headers no longer provide. Its compiler's C++
        // exceptions stay inside C++; its script errors are setjmp/longjmp,
        // both on Wasm exception handling.
        env::set_var("CXXFLAGS_wasm32_unknown_unknown", format!("--target={c_target} -fwasm-exceptions -mllvm -wasm-enable-sjlj -mllvm -wasm-use-legacy-eh=false -include cstdlib"));
    }
    // Script errors longjmp across Rust callback frames (as mlua's do with
    // every C Lua), so Rust needs no unwinding: the module is panic=abort,
    // which Slint's and winit's web backends require.
    let artifacts = luau0_src::Build::new().target(c_target).use_longjmp(true).build();
    println!("cargo:rustc-link-search=native={}", artifacts.lib_dir().display());
    for lib in artifacts.libs() {
        println!("cargo:rustc-link-lib=static={lib}");
    }
    // The SDK's C++ runtime and unwinder (which defines the C++ exception
    // tag), its setjmp runtime (which defines the longjmp tag), and libc.
    for lib in ["c++", "c++abi", "unwind", "setjmp", "c"] {
        let output = std::process::Command::new(&cxx)
            .args([&format!("--target={c_target}"), "-fwasm-exceptions", &format!("-print-file-name=lib{lib}.a")])
            .output().expect("run wasi-sdk clang++");
        assert!(output.status.success(), "query wasi-sdk libraries");
        let path = PathBuf::from(String::from_utf8(output.stdout).unwrap().trim());
        assert!(path.is_file(), "missing {}", path.display());
        println!("cargo:rerun-if-changed={}", path.display());
        if lib == "c" {
            // Keep libc's pure functions, but never link its second heap or
            // WASI imports. Missing host functionality must fail at link time.
            let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
            let library = out.join("libnettai_wasm_c.a");
            std::fs::copy(&path, &library).expect("copy wasi-libc");
            let members = std::process::Command::new(&ar).arg("t").arg(&library).output().expect("inspect wasi-libc");
            assert!(members.status.success(), "inspect wasi-libc archive members");
            let members = String::from_utf8(members.stdout).unwrap();
            for name in ["dlmalloc.c.obj", "__wasilibc_real.c.obj"] {
                assert!(members.lines().any(|m| m == name), "unsupported wasi-libc archive: missing {name}; use wasi-sdk 34");
            }
            let status = std::process::Command::new(&ar)
                .arg("d").arg(&library).args(["dlmalloc.c.obj", "__wasilibc_real.c.obj"])
                .status().expect("remove allocator and WASI imports from libc copy");
            assert!(status.success(), "prepare hostless libc");
            println!("cargo:rustc-link-search=native={}", out.display());
            println!("cargo:rustc-link-lib=static=nettai_wasm_c");
        } else {
            println!("cargo:rustc-link-search=native={}", path.parent().unwrap().display());
            println!("cargo:rustc-link-lib=static={lib}");
        }
    }
}
