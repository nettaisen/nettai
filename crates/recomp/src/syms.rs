//! Function entries, labels and code references extracted from the
//! disassembly (see tools/extract_funcs.py).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Mode {
    Thumb,
    Arm,
}

pub struct Syms {
    pub funcs: BTreeMap<u32, (Mode, String)>,
    pub labels: BTreeMap<u32, Vec<String>>,
    /// Addresses referenced from data words (bit 0 cleared).
    pub code_refs: BTreeSet<u32>,
    /// Thumb function pointers (`.word label+1`) found in data.
    pub thumb_ptrs: BTreeSet<u32>,
    /// Functions the host may override at run time.
    pub hooks: BTreeSet<u32>,
}

fn parse_hex(s: &str) -> u32 {
    u32::from_str_radix(s, 16).unwrap_or_else(|_| panic!("bad hex {s:?}"))
}

impl Syms {
    pub fn load(dir: &Path) -> Syms {
        let read = |name: &str| {
            std::fs::read_to_string(dir.join(name)).unwrap_or_else(|e| panic!("reading {name}: {e}"))
        };
        let mut funcs = BTreeMap::new();
        let extra = read("extra_entries.tsv");
        for line in read("functions.tsv").lines().chain(extra.lines().filter(|l| !l.starts_with('#'))) {
            let mut it = line.split('\t');
            let (Some(a), Some(m), Some(n)) = (it.next(), it.next(), it.next()) else { continue };
            let mode = if m == "arm" { Mode::Arm } else { Mode::Thumb };
            funcs.insert(parse_hex(a), (mode, n.to_string()));
        }
        let mut labels: BTreeMap<u32, Vec<String>> = BTreeMap::new();
        for line in read("labels.tsv").lines() {
            let mut it = line.split('\t');
            let (Some(a), Some(n)) = (it.next(), it.next()) else { continue };
            labels.entry(parse_hex(a)).or_default().push(n.to_string());
        }
        let mut code_refs = BTreeSet::new();
        let mut thumb_ptrs = BTreeSet::new();
        for line in read("code_refs.tsv").lines() {
            let mut it = line.split('\t');
            if let (Some(a), Some(t)) = (it.next(), it.next()) {
                let a = parse_hex(a) & !1;
                code_refs.insert(a);
                if t == "1" {
                    thumb_ptrs.insert(a);
                }
            }
        }
        let hooks = read("hooks.tsv")
            .lines()
            .filter(|l| !l.starts_with('#') && !l.is_empty())
            .map(|l| parse_hex(l.split('\t').next().unwrap()))
            .collect();
        Syms { funcs, labels, code_refs, thumb_ptrs, hooks }
    }

    /// A readable label for an address, preferring global names over
    /// function-local (dotted) ones.
    pub fn label(&self, addr: u32) -> Option<&str> {
        let names = self.labels.get(&addr)?;
        names.iter().find(|n| !n.contains('.')).or(names.first()).map(|s| s.as_str())
    }
}
