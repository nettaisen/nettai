//! Names as ids (docs/design/training-ai.md §3): every name the observation
//! carries (a panel type's, an object kind's, a chip's, a form's, a rules
//! field's) becomes two ids, each a bucket of a fixed hash, so no table of
//! names is kept anywhere and a new game's names need no list. The network
//! sums the two ids' embeddings: two names that share one bucket almost
//! never share the other.
//!
//! The hash, which the Rust player and the Python trainer must agree on for
//! as long as a network is played:
//!
//! ```text
//! id_k(name) = 1 + (fnv1a64(SEEDS[k], utf8(local(name))) mod 8191),  k = 0, 1
//! fnv1a64(h, bytes): for each byte b: h = (h xor b) * 0x100000001b3 (mod 2^64)
//! SEEDS = [0xcbf29ce484222325, 0x84222325cbf29ce4]
//! ```
//!
//! `local(name)` is the name without its root (`exe6:cannon` is `cannon`):
//! a name is hashed without its game, so a chip of one name is one token in
//! every game that has it. Id 0 is no name; ids run 1 to 8191, so an
//! embedding table has [`BUCKETS`] rows.

/// Rows of an embedding table of names: id 0 (none) and the 8191 buckets.
pub const BUCKETS: u32 = 8192;

/// The hash's seeds, one an id: FNV-1a's own offset basis, and it with its
/// halves swapped.
pub const SEEDS: [u64; 2] = [0xcbf2_9ce4_8422_2325, 0x8422_2325_cbf2_9ce4];

/// FNV-1a's 64-bit prime.
const PRIME: u64 = 0x0000_0100_0000_01b3;

/// The hash's description, which the observation spec's hash covers.
pub const DESCRIPTION: &str = "fnv1a64 seeds cbf29ce484222325 84222325cbf29ce4, 1 + h mod 8191, root dropped";

/// FNV-1a over bytes, from state `h`.
fn fnv1a(mut h: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        h = (h ^ b as u64).wrapping_mul(PRIME);
    }
    h
}

/// An id from a hash.
fn bucket(h: u64) -> i32 {
    (1 + h % (BUCKETS as u64 - 1)) as i32
}

/// A name's two ids (none for an empty name).
pub fn ids(name: &str) -> [i32; 2] {
    let local = nettai_content_api::keys::local(name);
    if local.is_empty() {
        return [0, 0];
    }
    SEEDS.map(|seed| bucket(fnv1a(seed, local.as_bytes())))
}

/// A name's two ids as it is written by `Debug` (an engine's own name, as
/// an enum's variant), with no string made.
pub fn debug_ids(value: &impl std::fmt::Debug) -> [i32; 2] {
    let mut h = Hashing(SEEDS);
    let _ = std::fmt::write(&mut h, format_args!("{value:?}"));
    h.0.map(bucket)
}

/// Both seeds' hashes, written to as text.
struct Hashing([u64; 2]);

impl std::fmt::Write for Hashing {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        for h in &mut self.0 {
            *h = fnv1a(*h, s.as_bytes());
        }
        Ok(())
    }
}

/// A name as a number in [0, 1) with `seed` (a chip held out of training is
/// one whose number is below the fraction held out, §6.6): FNV-1a 64 from the
/// first seed over the seed's eight bytes, little-endian, then the name's
/// (its root dropped), its top 53 bits over 2^53.
pub fn unit(seed: u64, name: &str) -> f64 {
    let h = fnv1a(fnv1a(SEEDS[0], &seed.to_le_bytes()), nettai_content_api::keys::local(name).as_bytes());
    (h >> 11) as f64 / (1u64 << 53) as f64
}

/// FNV-1a 64 of text with the first seed, written as 16 hex digits: the
/// observation spec's hash.
pub fn text_hash(text: &str) -> String {
    format!("{:016x}", fnv1a(SEEDS[0], text.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The hash is FNV-1a 64 (its published test vectors, with the
    /// standard seed), and a name's ids drop its root.
    #[test]
    fn the_hash_is_fnv1a_and_ids_drop_the_root() {
        assert_eq!(fnv1a(SEEDS[0], b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a(SEEDS[0], b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a(SEEDS[0], b"foobar"), 0x8594_4171_f739_67e8);
        assert_eq!(ids("exe6:cannon"), ids("cannon"));
        assert_eq!(ids(""), [0, 0]);
        let [a, b] = ids("cannon");
        assert!((1..BUCKETS as i32).contains(&a) && (1..BUCKETS as i32).contains(&b) && a != b);
        assert_eq!(debug_ids(&"x"), {
            // (Debug of a str is quoted.)
            let h = SEEDS.map(|s| fnv1a(s, b"\"x\""));
            h.map(bucket)
        });
        assert_eq!(unit(7, "exe6:cannon"), unit(7, "cannon"));
        assert!((0.0..1.0).contains(&unit(1, "cannon")));
        // Pinned: computed apart (Python), as the trainer checks them.
        assert_eq!(ids("cannon"), [834, 2843]);
        assert_eq!(ids("exe5:megaman"), [118, 1293]);
    }
}
