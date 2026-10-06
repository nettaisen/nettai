import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const path = process.argv[2];
assert(path, "usage: node wasm-smoke.mjs <wasm_smoke.wasm>");
const module = await WebAssembly.compile(await readFile(path));
assert.deepEqual(WebAssembly.Module.imports(module), [], "nettai-luau must need no host imports");
const instance = await WebAssembly.instantiate(module, {});
for (let i = 0; i < 32; i++) {
    assert.equal(instance.exports.smoke_test(), 42);
}
console.log("nettai-luau: 32 bare-Wasm runs passed, with zero host imports");
