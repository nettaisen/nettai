//! What a game defines, to compare two trees (a change that should define
//! the same must print the same): for each registry its definitions' count
//! and a digest of their keys, modules and specs, then every key.
//!
//!     cargo run --release -p nettai-content --example definitions -- <pack> [--keys]
//!
//! (The content is this repository's content/, or `$NETTAI_CONTENT`; the
//! pack gives its game and its assets. The digest is of the definitions
//! alone, not the content hash, which covers the modules' text.)

use std::collections::BTreeMap;

/// FNV-1a, 64 bits: a digest that is the same on every run and machine.
fn fnv(bytes: &[u8], mut h: u64) -> u64 {
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    h
}

fn main() {
    let mut args = std::env::args().skip(1);
    let pack = std::path::PathBuf::from(args.next().expect("usage: definitions <pack> [--keys]"));
    let keys = args.next().is_some_and(|a| a == "--keys");
    let (content, _) = nettai_content::pack::load_battle(&nettai_content::index::content(), &pack).unwrap_or_else(|r| panic!("{r}"));
    let defs = &content.defs.definitions.defs;
    let mut by_registry: BTreeMap<String, (usize, u64)> = BTreeMap::new();
    let mut all = 0xCBF2_9CE4_8422_2325u64;
    for d in defs {
        let text = format!("{:?}", d);
        let e = by_registry.entry(d.registry.name().to_string()).or_insert((0, 0xCBF2_9CE4_8422_2325));
        e.0 += 1;
        e.1 = fnv(text.as_bytes(), e.1);
        all = fnv(text.as_bytes(), all);
    }
    println!("{}: {} definitions, digest {all:016x}", content.game(), defs.len());
    for (registry, (n, h)) in &by_registry {
        println!("  {registry:<18} {n:>5}  {h:016x}");
    }
    if keys {
        for (i, d) in defs.iter().enumerate() {
            println!("{i}\t{}\t{}\t{}", d.registry.name(), d.key, d.module);
        }
    }
    // (What the load read, and the content's hash, which covers its text.)
    eprintln!("{} modules read; content hash {}", content.scripts.modules.len(), content.hash());
}
