// The NDK's compiler-rt builtins, linked into the library: rustc links with `-nodefaultlibs`, and C the library
// compiles calls their outline atomics (aws-lc's, under str0m's DTLS: `__aarch64_cas4_acq_rel`), which nothing else
// then defines. Where the archive is, the NDK's clang says (the one cc compiles the target's C with).
fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("android") {
        return;
    }
    let clang = cc::Build::new().cargo_metadata(false).get_compiler();
    let said = std::process::Command::new(clang.path())
        .args(clang.args())
        .args(["-rtlib=compiler-rt", "-print-libgcc-file-name"])
        .output()
        .expect("the NDK's clang runs");
    let builtins = String::from_utf8(said.stdout).expect("a path");
    let builtins = builtins.trim();
    assert!(std::path::Path::new(builtins).is_file(), "no compiler-rt builtins at {builtins}");
    println!("cargo:rustc-link-arg={builtins}");
    println!("cargo:rerun-if-env-changed=CC_aarch64_linux_android");
    println!("cargo:rerun-if-env-changed=ANDROID_NDK_HOME");
}
