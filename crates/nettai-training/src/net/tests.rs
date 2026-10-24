use super::*;
use crate::observe::Obs;
use std::path::PathBuf;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// The fixture's decision `i` as the encoder's observation: each of the
/// network's sets' tokens put where the encoder keeps them (a split set's
/// in its part of the encoder's set).
fn observation(net: &Net, fx: &safetensors::File, i: usize) -> Obs {
    let mut obs = Obs::new();
    for (name, source, first, count) in net.sets() {
        let e = spec::SETS[source];
        let x = fx.tensors[&format!("x.{name}")].f32s().unwrap();
        let ids = fx.tensors[&format!("ids.{name}")].ints().unwrap();
        let mask = fx.tensors[&format!("mask.{name}")].ints().unwrap();
        let buf = &mut obs.sets[source];
        for k in 0..count {
            let tok = first + k;
            let row = (i * count + k) * e.width;
            buf.x[tok * e.width..(tok + 1) * e.width].copy_from_slice(&x[row..row + e.width]);
            for j in 0..e.ids {
                buf.ids[tok * e.ids + j] = ids[(i * count + k) * e.ids + j] as i32;
            }
            buf.mask[tok] = mask[i * count + k] != 0;
        }
    }
    obs
}

/// The fixture's previous action, by its place in the one-hot.
fn action_at(i: usize) -> Option<Action> {
    if i < spec::FIGHT_ACTIONS {
        Some(Action::Fight(i))
    } else if i < PREVIOUS_ACTIONS - 1 {
        Some(Action::Screen(i - spec::FIGHT_ACTIONS))
    } else {
        None
    }
}

/// A finite value within 1e-4 (relative past 1) and every −∞ exactly.
fn close(what: &str, got: &[f32], want: &[f32]) {
    assert_eq!(got.len(), want.len(), "{what}");
    for (k, (&g, &w)) in got.iter().zip(want).enumerate() {
        if w.is_finite() {
            assert!((g - w).abs() <= 1e-4 * w.abs().max(1.0), "{what}[{k}]: {g} against {w}");
        } else {
            assert_eq!(g, w, "{what}[{k}]");
        }
    }
}

/// Every output of the trainer's network on a fixture of its own export
/// (training/MODEL.md's "Checking the port"), decision by decision from
/// its h0, the GRU state carried and reset where the fixture says, each
/// decision's previous action the fixture's (none, fight and screen
/// actions among them).
fn parity(name: &str) {
    let weights = std::fs::read(fixtures().join(format!("{name}.safetensors"))).unwrap();
    let net = Net::from_safetensors(&weights).unwrap_or_else(|e| panic!("{e}"));
    let bytes = std::fs::read(fixtures().join(format!("{name}.fixture.safetensors"))).unwrap();
    let fx = safetensors::parse(&bytes).unwrap();
    assert_eq!(fx.metadata["spec_hash"], spec::hash());
    let n = fx.tensors["reset"].shape[0];
    let reset = fx.tensors["reset"].ints().unwrap();
    let f32s = |k: &str| fx.tensors[k].f32s().unwrap();
    let ints = |k: &str| fx.tensors[k].ints().unwrap();
    let (fight_raw, screen_raw, fight, screen, value, hidden) = (f32s("fight_raw"), f32s("screen_raw"), f32s("fight"), f32s("screen"), f32s("value"), f32s("hidden"));
    let (legal_fight, legal_screen, phase, previous) = (ints("legal_fight"), ints("legal_screen"), ints("phase"), ints("previous"));
    let h = net.hidden();
    let mut mem = net.memory();
    mem.h.copy_from_slice(&f32s("h0"));
    let (nf, ns) = (spec::FIGHT_ACTIONS, spec::SCREEN_ACTIONS);
    let mut phases = String::new();
    for i in 0..n {
        if reset[i] != 0 {
            mem.reset();
        }
        let last = action_at(previous[i] as usize);
        assert_eq!(previous_index(last), previous[i] as usize);
        mem.last = last;
        let obs = observation(&net, &fx, i);
        let before = mem.h.clone();
        let out = net.forward(&obs, &mut mem, Heads::All);
        let (mut f, mut s) = (out.fight.unwrap(), out.screen.unwrap());
        close(&format!("{name} {i}: fight_raw"), &f, &fight_raw[i * nf..(i + 1) * nf]);
        close(&format!("{name} {i}: screen_raw"), &s, &screen_raw[i * ns..(i + 1) * ns]);
        close(&format!("{name} {i}: value"), &[out.value.unwrap()], &value[i..i + 1]);
        close(&format!("{name} {i}: hidden"), &mem.h, &hidden[i * h..(i + 1) * h]);
        let lf: Vec<bool> = legal_fight[i * nf..(i + 1) * nf].iter().map(|&v| v != 0).collect();
        let ls: Vec<bool> = legal_screen[i * ns..(i + 1) * ns].iter().map(|&v| v != 0).collect();
        mask(&mut f, &lf);
        mask(&mut s, &ls);
        close(&format!("{name} {i}: fight"), &f, &fight[i * nf..(i + 1) * nf]);
        close(&format!("{name} {i}: screen"), &s, &screen[i * ns..(i + 1) * ns]);
        phases.push(if phase[i] == 1 { 'f' } else { 's' });
        // (The heads a decision asks alone are the same as all of them.)
        let mut again = net.memory();
        again.h.copy_from_slice(&before);
        again.last = last;
        let alone = net.forward(&obs, &mut again, if phase[i] == 1 { Heads::Fight } else { Heads::Screen });
        assert_eq!(alone.value, None);
        if phase[i] == 1 {
            assert_eq!((alone.fight, alone.screen), (out.fight, None));
        } else {
            assert_eq!((alone.fight, alone.screen), (None, out.screen));
        }
    }
    assert!(phases.contains('f') && phases.contains('s'), "{name}: the fixture has both kinds of decisions ({phases})");
    assert!(reset.iter().any(|&r| r != 0) && reset[0] == 0, "{name}: the GRU state carries from h0 and is reset in the sequence");
    let kinds = previous.iter().map(|&p| match action_at(p as usize) {
        Some(Action::Fight(_)) => 'f',
        Some(Action::Screen(_)) => 's',
        None => 'n',
    });
    let kinds: String = kinds.collect();
    assert!(kinds.contains('f') && kinds.contains('s') && kinds.contains('n'), "{name}: previous actions of every kind ({kinds})");
}

#[test]
fn the_network_computes_what_the_trainers_does() {
    parity("net");
}

/// The fixture's levels are the trainer's, and a weights file whose spec
/// hash, format or tensors aren't this pass's is refused.
#[test]
fn weights_of_another_encoder_or_shape_are_refused() {
    let weights = std::fs::read(fixtures().join("net.safetensors")).unwrap();
    let net = Net::from_safetensors(&weights).unwrap();
    assert_eq!(net.levels().iter().map(|l| l.name.as_str()).collect::<Vec<_>>(), ["easy", "hard", "normal"]);
    assert!(net.level("hard").unwrap().greedy);
    // The header rewritten: (what to change in the metadata, what the refusal says).
    let file = safetensors::parse(&weights).unwrap();
    let rewrite = |change: &Change| -> String {
        let mut meta = file.metadata.clone();
        let mut drop = Vec::new();
        change(&mut meta, &mut drop);
        let bytes = write(&file, &meta, &drop);
        Net::from_safetensors(&bytes).err().unwrap_or_default()
    };
    assert!(rewrite(&|m, _| {
        m.insert("spec_hash".into(), "0000000000000000".into());
    })
    .contains("observation spec"));
    assert!(rewrite(&|m, _| {
        m.insert("format_version".into(), "2".into());
    })
    .contains("format version"));
    assert!(rewrite(&|m, _| {
        let a = m["architecture"].replace("\"pool_heads\"", "\"pool_heads_new\": 1, \"pool_heads\"");
        m.insert("architecture".into(), a);
    })
    .contains("pool_heads_new"));
    assert!(rewrite(&|m, _| {
        let a = m["architecture"].replace("\"previous_action\": 38", "\"previous_action\": 39");
        m.insert("architecture".into(), a);
    })
    .contains("previous action of 39"));
    assert!(rewrite(&|m, _| {
        let a = m["architecture"].replace("\"entries\": 5", "\"entries\": 7");
        m.insert("architecture".into(), a);
    })
    .contains("7 entries"));
    assert!(rewrite(&|_, drop| drop.push("ln_f.bias".into())).contains("no ln_f.bias"));
    assert_eq!(rewrite(&|_, _| {}), "", "the file as it was is taken");
}

/// A change to a weights file's metadata and the tensors it drops.
type Change = dyn Fn(&mut HashMap<String, String>, &mut Vec<String>);

/// The file `file` written again with `meta` and without the tensors
/// `drop`.
fn write(file: &safetensors::File, meta: &HashMap<String, String>, drop: &[String]) -> Vec<u8> {
    let mut header = serde_json::Map::new();
    header.insert("__metadata__".into(), serde_json::to_value(meta).unwrap());
    let mut data = Vec::new();
    let mut names: Vec<&String> = file.tensors.keys().filter(|k| !drop.contains(k)).collect();
    names.sort();
    for name in names {
        let t = &file.tensors[name];
        let dtype = match t.dtype {
            safetensors::Dtype::F32 => "F32",
            safetensors::Dtype::I64 => "I64",
            safetensors::Dtype::I32 => "I32",
            safetensors::Dtype::I16 => "I16",
            safetensors::Dtype::I8 => "I8",
            safetensors::Dtype::U8 => "U8",
            safetensors::Dtype::Bool => "BOOL",
        };
        header.insert(name.clone(), serde_json::json!({"dtype": dtype, "shape": t.shape, "data_offsets": [data.len(), data.len() + t.bytes.len()]}));
        data.extend_from_slice(t.bytes);
    }
    let header = serde_json::to_vec(&header).unwrap();
    let mut out = (header.len() as u64).to_le_bytes().to_vec();
    out.extend(header);
    out.extend(data);
    out
}

/// A level chooses among the allowed actions alone: greedy the first of
/// the largest; sampling at its temperature, now and then at random.
#[test]
fn levels_choose_allowed_actions() {
    let knobs = |greedy, epsilon| Knobs { name: "t".into(), delay: 10, temperature: 1.0, epsilon, greedy };
    let mut rng = SplitMix64::new(3);
    let logits = [1.0, 5.0, 5.0, f32::NEG_INFINITY, 2.0];
    let legal = [true, true, true, true, false];
    assert_eq!(choose(&logits, &legal, &knobs(true, 0.0), &mut rng), Some(1));
    assert_eq!(choose(&logits, &[false; 5], &knobs(false, 0.0), &mut rng), None);
    assert_eq!(choose(&logits, &[false, false, false, true, false], &knobs(false, 0.5), &mut rng), None, "−∞ is never allowed");
    assert_eq!(choose(&logits, &[false, false, false, true, true], &knobs(false, 0.5), &mut rng), Some(4));
    let mut counts = [0; 5];
    for _ in 0..4000 {
        counts[choose(&logits, &legal, &knobs(false, 0.0), &mut rng).unwrap()] += 1;
    }
    assert!(counts[3] == 0 && counts[4] == 0 && counts[0] > 0 && counts[0] < 100 && counts[1] > 1700 && counts[2] > 1700, "{counts:?}");
    let mut counts = [0; 5];
    for _ in 0..4000 {
        counts[choose(&logits, &legal, &knobs(false, 1.0), &mut rng).unwrap()] += 1;
    }
    assert!(counts[3] == 0 && counts[4] == 0 && counts.iter().take(3).all(|&c| c > 1200), "uniform: {counts:?}");
}
