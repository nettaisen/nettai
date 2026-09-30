//! What loading a pack's scripts costs (docs/design/content-model-v2.md
//! §8): the define phase (`Content::define`: every module run once, the
//! registries built) and a runtime's load (`Behaviors::load`: the define
//! phase again in the VM that keeps the functions), each the best of
//! several runs, with the parts they are made of.
//!
//! cargo run --release -p bn6-content --example define_cost -- <pack> [runs]

use std::time::{Duration, Instant};

use bn6_battle::behavior::{Behaviors, Options};

fn best(runs: usize, mut f: impl FnMut()) -> Duration {
    (0..runs)
        .map(|_| {
            let t = Instant::now();
            f();
            t.elapsed()
        })
        .min()
        .expect("at least one run")
}

fn main() {
    let mut args = std::env::args().skip(1);
    let pack = std::path::PathBuf::from(args.next().expect("usage: define_cost <pack> [runs]"));
    let runs: usize = args.next().map_or(10, |n| n.parse().expect("a number of runs"));
    let mut report = bn6_content::report::Report::default();
    let content = bn6_content::battle::load(&pack, &mut report).unwrap_or_else(|| panic!("{report}"));
    let mut defined = content.clone();
    defined.define().expect("the pack defines");
    println!("{} modules, {} definitions, best of {runs} runs:", content.scripts.modules.len(), defined.defs.definitions.defs.len());
    let data = best(runs, || {
        std::hint::black_box(bn6_battle::behavior::script_data(&content));
    });
    println!("  the scripts' data (script_data):        {data:>10.2?}");
    let define = best(runs, || {
        let mut c = content.clone();
        c.define().expect("the pack defines");
        std::hint::black_box(c);
    });
    println!("  the define phase (Content::define):     {define:>10.2?}");
    let hash = best(runs, || {
        std::hint::black_box(defined.hash());
    });
    println!("  the content hash:                       {hash:>10.2?}");
    let runtime = best(runs, || {
        std::hint::black_box(Behaviors::load(&defined, Options::default()).expect("the scripts load"));
    });
    println!("  a runtime's load (Behaviors::load):     {runtime:>10.2?}");
}
