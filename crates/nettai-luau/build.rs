use std::{env, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if env::var("TARGET").as_deref() != Ok("wasm32-unknown-unknown") {
        return;
    }
    assert!(env::var_os("CARGO_FEATURE_JIT").is_none(), "Luau's native JIT is not supported on wasm32-unknown-unknown; disable the jit feature");
    assert_eq!(env::var("CARGO_CFG_PANIC").as_deref(), Ok("unwind"), "Luau errors unwind across Rust callbacks; use panic=unwind and -Zbuild-std=std,panic_unwind (see crates/nettai-luau/README.md)");
    println!("cargo:rerun-if-env-changed=WASI_SDK_PATH");
    let sdk = PathBuf::from(env::var_os("WASI_SDK_PATH").expect("set WASI_SDK_PATH to wasi-sdk 34 (see crates/nettai-luau/README.md)"))
        .canonicalize().expect("WASI_SDK_PATH must name an existing SDK directory");
    let cxx = sdk.join(format!("bin/clang++{}", env::consts::EXE_SUFFIX));
    let ar = sdk.join(format!("bin/llvm-ar{}", env::consts::EXE_SUFFIX));
    // Build scripts run in their own single-threaded process. Only the C++
    // build uses the SDK's target/sysroot; Rust and the final module are
    // wasm32-unknown-unknown.
    unsafe {
        env::set_var("CXX_wasm32_unknown_unknown", &cxx);
        env::set_var("AR_wasm32_unknown_unknown", &ar);
        // Luau's parser relies on transitive <cstdlib> includes that
        // newer libc++ headers no longer provide.
        env::set_var("CXXFLAGS_wasm32_unknown_unknown", "--target=wasm32-wasip1 -fwasm-exceptions -mllvm -wasm-use-legacy-eh=false -include cstdlib");
    }
    let artifacts = luau0_src::Build::new().target("wasm32-wasip1").build();
    println!("cargo:rustc-link-search=native={}", artifacts.lib_dir().display());
    for lib in artifacts.libs() {
        println!("cargo:rustc-link-lib=static={lib}");
    }
    // Rust defines the exception tag on this target. Build the LLVM Wasm
    // unwinder without its second definition of that tag.
    println!("cargo:rerun-if-changed=wasm/Unwind-wasm.c");
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let object = out.join("Unwind-wasm.o");
    let status = std::process::Command::new(sdk.join(format!("bin/clang{}", env::consts::EXE_SUFFIX)))
        .arg("-I").arg(sdk.join("share/wasi-sysroot/include/wasm32-wasip1/eh"))
        .args(["--target=wasm32-wasip1", "-std=c11", "-fwasm-exceptions", "-mllvm", "-wasm-use-legacy-eh=false", "-O2", "-c", "wasm/Unwind-wasm.c", "-o"])
        .arg(&object).status().expect("compile the Wasm unwinder");
    assert!(status.success(), "compile the Wasm unwinder");
    let status = std::process::Command::new(&ar).arg("crs").arg(out.join("libnettai_wasm_unwind.a"))
        .arg(&object).status().expect("archive the Wasm unwinder");
    assert!(status.success(), "archive the Wasm unwinder");
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=nettai_wasm_unwind");
    for lib in ["c++", "c++abi", "c"] {
        let output = std::process::Command::new(&cxx)
            .args(["--target=wasm32-wasip1", "-fwasm-exceptions", &format!("-print-file-name=lib{lib}.a")])
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
