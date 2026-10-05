//! A bare Wasm module, with no wasm-bindgen or WASI imports. Run with
//! `node crates/nettai-luau/tests/wasm-smoke.mjs <wasm_smoke.wasm>`.

use nettai_content_api::{AssetNames, Registry};
use nettai_luau::{Options, Pack, define, sandbox};

#[unsafe(no_mangle)]
pub extern "C" fn smoke_test() -> u32 {
    let pack = Pack::root("smoke", [(
        "effect".into(),
        "return define.effect { anim = 0 }".into(),
    )]);
    let (defs, compiled) = define(&pack, &AssetNames::default(), Options::default()).unwrap();
    assert_eq!(defs.of(Registry::Effect).len(), 1);
    assert_eq!(compiled.len(), 1);
    let lua = sandbox::new_vm(false).unwrap();
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    assert!(lua.heap_dump().is_err());
    let fail = lua.create_function(|_, ()| -> mlua::Result<()> { Err(mlua::Error::runtime("callback error")) }).unwrap();
    lua.globals().set("fail", fail).unwrap();
    assert!(lua.load("fail()").exec().unwrap_err().to_string().contains("callback error"));
    assert!(lua.load("error('script error')").exec().unwrap_err().to_string().contains("script error"));
    let result = lua.load("return 6 * 7").eval().unwrap();
    lua.gc_collect().unwrap();
    result
}
