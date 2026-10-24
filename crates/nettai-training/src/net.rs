//! The trained network's forward pass (docs/design/training-ai.md §7, §10):
//! what the trainer's model (training/nettai_ai/model.py) computes for one
//! decision of one seat, step by step as training/MODEL.md has it, from the
//! weights file its export writes.
//!
//! - **The file** is safetensors ([`safetensors`]: a small reader of its
//!   own), every tensor under its PyTorch name. Its metadata says the format
//!   and its version, the observation spec's hash (weights of another
//!   encoder are refused), the architecture (every width; the sets in their
//!   order, the ones the trainer cut from one of the encoder's, `split`) and
//!   the levels' knobs. Nothing of the network's shape is written here: a
//!   field of the architecture this doesn't know, a tensor it doesn't read
//!   or one missing is an error.
//! - **The pass** ([`Net::forward`]): each present token normalized, through
//!   its set's MLP, plus its set's vector and its names' embeddings; the
//!   transformer layers over the present tokens of the sets that aren't
//!   pool-only; a final LayerNorm; the attention pool and the one-token
//!   sets read directly; the GRU (gates r, z, n) over the summary and the
//!   previous decision's action ([`Memory::took`]: the network decides from
//!   a view its reaction delay old, so it is told what it pressed since);
//!   the heads (the fight's actions, the custom screen's pointer over the
//!   slots and its form list's entries, the value). Absent tokens are never read.
//! - **Choosing** ([`choose`]): the masked logits of the decision's head,
//!   and a level's knobs (greedy, or a sample at a temperature, with a
//!   chance of a random allowed action).
//!
//! Plain f32 loops, no threads, no unsafe: dot products over rows kept in
//! eight independent sums, so LLVM vectorizes them (NEON on the desktop,
//! the browser's WebAssembly as it is built).

use crate::observe::Obs;
use crate::{names, spec};
use nettai_netplay::rng::SplitMix64;
use serde::Deserialize;
use std::collections::HashMap;

/// The weights file's format and its version: anything else is refused.
pub const FORMAT: &str = "nettai-ai";
pub const FORMAT_VERSION: u32 = 1;

/// The GRU's input after the summary: the previous decision's action,
/// one-hot over the fight's actions, the custom screen's, and none (a
/// round's first decision).
pub const PREVIOUS_ACTIONS: usize = spec::FIGHT_ACTIONS + spec::SCREEN_ACTIONS + 1;

/// A decision's action, as the next decision's GRU reads it back: a fight
/// action or a custom screen's (the previous decision may be of the other
/// phase).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Fight(usize),
    Screen(usize),
}

/// The previous action's place in the one-hot (training/MODEL.md §4): a
/// fight action as itself, a screen action after the fight's, none last.
pub fn previous_index(last: Option<Action>) -> usize {
    match last {
        Some(Action::Fight(a)) => a,
        Some(Action::Screen(a)) => spec::FIGHT_ACTIONS + a,
        None => PREVIOUS_ACTIONS - 1,
    }
}

/// A reader of safetensors files: an 8-byte little-endian header length,
/// the header (JSON: each tensor's type, shape and byte range in the data
/// after the header, and `__metadata__`, text by name), then the data.
pub mod safetensors {
    use std::collections::HashMap;

    /// A tensor's element type, as the file names it.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Dtype {
        F32,
        I64,
        I32,
        I16,
        I8,
        U8,
        Bool,
    }

    impl Dtype {
        fn named(name: &str) -> Option<Dtype> {
            Some(match name {
                "F32" => Dtype::F32,
                "I64" => Dtype::I64,
                "I32" => Dtype::I32,
                "I16" => Dtype::I16,
                "I8" => Dtype::I8,
                "U8" => Dtype::U8,
                "BOOL" => Dtype::Bool,
                _ => return None,
            })
        }

        pub fn size(self) -> usize {
            match self {
                Dtype::F32 | Dtype::I32 => 4,
                Dtype::I64 => 8,
                Dtype::I16 => 2,
                Dtype::I8 | Dtype::U8 | Dtype::Bool => 1,
            }
        }
    }

    /// A tensor of a file: its type, shape and bytes.
    #[derive(Clone, Debug)]
    pub struct Tensor<'a> {
        pub dtype: Dtype,
        pub shape: Vec<usize>,
        pub bytes: &'a [u8],
    }

    impl Tensor<'_> {
        /// Its elements.
        pub fn len(&self) -> usize {
            self.shape.iter().product()
        }

        pub fn is_empty(&self) -> bool {
            self.len() == 0
        }

        /// Its values, of a float32 tensor.
        pub fn f32s(&self) -> Result<Vec<f32>, String> {
            if self.dtype != Dtype::F32 {
                return Err(format!("a {:?} tensor where float32 is wanted", self.dtype));
            }
            Ok(self.bytes.as_chunks::<4>().0.iter().map(|b| f32::from_le_bytes(*b)).collect())
        }

        /// Its values as integers, of an integer or bool tensor.
        pub fn ints(&self) -> Result<Vec<i64>, String> {
            let b = self.bytes;
            Ok(match self.dtype {
                Dtype::F32 => return Err("a float32 tensor where integers are wanted".into()),
                Dtype::I64 => b.as_chunks::<8>().0.iter().map(|c| i64::from_le_bytes(*c)).collect(),
                Dtype::I32 => b.as_chunks::<4>().0.iter().map(|c| i32::from_le_bytes(*c) as i64).collect(),
                Dtype::I16 => b.as_chunks::<2>().0.iter().map(|c| i16::from_le_bytes(*c) as i64).collect(),
                Dtype::I8 => b.iter().map(|&v| v as i8 as i64).collect(),
                Dtype::U8 | Dtype::Bool => b.iter().map(|&v| v as i64).collect(),
            })
        }
    }

    /// A file's tensors, by name, and its metadata.
    #[derive(Clone, Debug)]
    pub struct File<'a> {
        pub metadata: HashMap<String, String>,
        pub tensors: HashMap<String, Tensor<'a>>,
    }

    /// Read a file's header and find its tensors in `bytes`.
    pub fn parse(bytes: &[u8]) -> Result<File<'_>, String> {
        let len = bytes.get(..8).ok_or("shorter than a safetensors header")?;
        let len = u64::from_le_bytes(len.try_into().unwrap());
        let end = usize::try_from(len).ok().and_then(|l| l.checked_add(8)).filter(|&e| e <= bytes.len()).ok_or("the header runs past the file's end")?;
        let header: serde_json::Map<String, serde_json::Value> = serde_json::from_slice(&bytes[8..end]).map_err(|e| format!("the header: {e}"))?;
        let data = &bytes[end..];
        let mut out = File { metadata: HashMap::new(), tensors: HashMap::new() };
        for (name, v) in header {
            if name == "__metadata__" {
                let m = v.as_object().ok_or("__metadata__ isn't a table")?;
                for (k, v) in m {
                    let v = v.as_str().ok_or_else(|| format!("metadata {k} isn't text"))?;
                    out.metadata.insert(k.clone(), v.to_string());
                }
                continue;
            }
            let bad = |why: &str| format!("tensor {name}: {why}");
            let t = v.as_object().ok_or_else(|| bad("not a table"))?;
            let dtype = t.get("dtype").and_then(|d| d.as_str()).ok_or_else(|| bad("no dtype"))?;
            let dtype = Dtype::named(dtype).ok_or_else(|| bad(&format!("a type this reader doesn't read, {dtype}")))?;
            let shape: Vec<usize> = t
                .get("shape")
                .and_then(|s| s.as_array())
                .ok_or_else(|| bad("no shape"))?
                .iter()
                .map(|n| n.as_u64().map(|n| n as usize))
                .collect::<Option<_>>()
                .ok_or_else(|| bad("a shape not of counts"))?;
            let range: Vec<usize> = t
                .get("data_offsets")
                .and_then(|s| s.as_array())
                .ok_or_else(|| bad("no data_offsets"))?
                .iter()
                .map(|n| n.as_u64().map(|n| n as usize))
                .collect::<Option<_>>()
                .ok_or_else(|| bad("data_offsets not of counts"))?;
            let &[from, to] = range.as_slice() else { return Err(bad("data_offsets aren't two")) };
            if from > to || to > data.len() {
                return Err(bad("its data runs past the file's end"));
            }
            let t = Tensor { dtype, shape, bytes: &data[from..to] };
            if t.len() * dtype.size() != to - from {
                return Err(bad("its data isn't its shape's size"));
            }
            out.tensors.insert(name, t);
        }
        Ok(out)
    }
}

/// A level's knobs (§10), from the weights' metadata: the reaction delay in
/// ticks, the temperature logits are divided by before sampling, the chance
/// of a random allowed action, and whether it takes its best action.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Knobs {
    pub name: String,
    pub delay: u16,
    pub temperature: f32,
    pub epsilon: f32,
    pub greedy: bool,
}

/// The architecture as the weights' metadata states it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Architecture {
    format: String,
    format_version: u32,
    model: Widths,
    activation: String,
    layer_norm_eps: f64,
    slots: usize,
    /// The screen's actions after the slots: the form list's entries.
    entries: usize,
    /// The previous action's one-hot width ([`PREVIOUS_ACTIONS`]).
    previous_action: usize,
    spec: ArchSpec,
}

/// The network's widths (the trainer's `ModelConfig`).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Widths {
    d: usize,
    heads: usize,
    layers: usize,
    ffn: usize,
    token_hidden: usize,
    pool_heads: usize,
    summary: usize,
    hidden: usize,
    head_hidden: usize,
    pointer_hidden: usize,
    pointer_set: String,
    pool_only: Vec<String>,
    norm_clip: f32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ArchSpec {
    hash: String,
    buckets: usize,
    fight_actions: usize,
    screen_actions: usize,
    sets: Vec<ArchSet>,
    /// Sets the trainer cut from one of the encoder's: by the encoder's
    /// set, each part's name, first token and count.
    split: HashMap<String, Vec<(String, usize, usize)>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ArchSet {
    name: String,
    count: usize,
    width: usize,
    ids: usize,
}

/// `W·x + b`, `W` of `rows` rows of `cols`, row-major (PyTorch's `[out, in]`).
struct Linear {
    w: Vec<f32>,
    b: Vec<f32>,
    rows: usize,
    cols: usize,
}

impl Linear {
    /// `y = W·x + b`.
    fn apply(&self, x: &[f32], y: &mut [f32]) {
        let x = &x[..self.cols];
        for (r, y) in y[..self.rows].iter_mut().enumerate() {
            *y = dot(&self.w[r * self.cols..(r + 1) * self.cols], x) + self.b[r];
        }
    }

    /// `y = relu(W·x + b)`.
    fn relu(&self, x: &[f32], y: &mut [f32]) {
        self.apply(x, y);
        for v in &mut y[..self.rows] {
            *v = v.max(0.0);
        }
    }

    /// `y += W·x + b`.
    fn add(&self, x: &[f32], y: &mut [f32]) {
        let x = &x[..self.cols];
        for (r, y) in y[..self.rows].iter_mut().enumerate() {
            *y += dot(&self.w[r * self.cols..(r + 1) * self.cols], x) + self.b[r];
        }
    }
}

/// A dot product, in eight independent sums (which LLVM turns into vector
/// lanes; one sum would be a chain it may not reorder).
#[inline]
fn dot(a: &[f32], b: &[f32]) -> f32 {
    let n = a.len().min(b.len());
    let (a, b) = (&a[..n], &b[..n]);
    let mut acc = [0.0f32; 8];
    let ((ca, ra), (cb, rb)) = (a.as_chunks::<8>(), b.as_chunks::<8>());
    for (x, y) in ca.iter().zip(cb) {
        for j in 0..8 {
            acc[j] += x[j] * y[j];
        }
    }
    let mut tail = 0.0;
    for (x, y) in ra.iter().zip(rb) {
        tail += x * y;
    }
    ((acc[0] + acc[4]) + (acc[1] + acc[5])) + ((acc[2] + acc[6]) + (acc[3] + acc[7])) + tail
}

/// `y += a·x`.
#[inline]
fn axpy(a: f32, x: &[f32], y: &mut [f32]) {
    for (y, x) in y.iter_mut().zip(x) {
        *y += a * x;
    }
}

/// A LayerNorm over `d` features: `(v − mean) / sqrt(var + eps) ⊙ w + b`,
/// the variance biased.
struct Norm {
    w: Vec<f32>,
    b: Vec<f32>,
    eps: f32,
}

impl Norm {
    fn apply(&self, x: &[f32], y: &mut [f32]) {
        let d = self.w.len();
        let x = &x[..d];
        let mean = x.iter().sum::<f32>() / d as f32;
        let var = x.iter().map(|v| (v - mean) * (v - mean)).sum::<f32>() / d as f32;
        let inv = 1.0 / (var + self.eps).sqrt();
        for i in 0..d {
            y[i] = (x[i] - mean) * inv * self.w[i] + self.b[i];
        }
    }
}

/// A token set of the network: where its tokens are in the encoder's
/// observation, and its encoder.
struct SetNet {
    name: String,
    /// The encoder's set (`spec::SETS`' place) and its first token there.
    source: usize,
    first: usize,
    count: usize,
    width: usize,
    ids: usize,
    pool_only: bool,
    mean: Vec<f32>,
    inv_std: Vec<f32>,
    fc1: Linear,
    fc2: Linear,
    type_emb: Vec<f32>,
}

/// A pre-LayerNorm transformer layer.
struct Block {
    ln1: Norm,
    q: Linear,
    k: Linear,
    v: Linear,
    o: Linear,
    ln2: Norm,
    ff1: Linear,
    ff2: Linear,
}

/// The trained network, read from its weights file.
pub struct Net {
    d: usize,
    heads: usize,
    clip: f32,
    pool_heads: usize,
    summary: usize,
    hidden: usize,
    sets: Vec<SetNet>,
    /// The name table, `[buckets, d]`.
    names: Vec<f32>,
    blocks: Vec<Block>,
    ln_f: Norm,
    pool_score: Linear,
    pool_out: Linear,
    /// The one-token sets, in order (read directly), and the pointer's set.
    singles: Vec<usize>,
    pointer: usize,
    /// The GRU's input weights of the summary, and of the previous action
    /// by its one-hot's place (a column each, `[PREVIOUS_ACTIONS, 3H]`: the
    /// one-hot's product is its column).
    gru_ih: Linear,
    gru_previous: Vec<f32>,
    gru_hh: Linear,
    fight1: Linear,
    fight2: Linear,
    entry1: Linear,
    entry2: Linear,
    ptr1: Linear,
    ptr2: Linear,
    value1: Linear,
    value2: Linear,
    levels: Vec<Knobs>,
    /// Tokens at most: all, and those the transformer reads.
    tokens: usize,
    transformer_tokens: usize,
    metadata: HashMap<String, String>,
}

/// The weights file's tensors, taken one by one by name and shape; what is
/// left once the network is made is an error (a strict load).
struct Weights<'a>(HashMap<String, safetensors::Tensor<'a>>);

impl Weights<'_> {
    fn take(&mut self, name: &str, shape: &[usize]) -> Result<Vec<f32>, String> {
        let t = self.0.remove(name).ok_or_else(|| format!("the weights have no {name}"))?;
        if t.shape != shape {
            return Err(format!("{name} is {:?}, the architecture says {shape:?}", t.shape));
        }
        t.f32s().map_err(|e| format!("{name}: {e}"))
    }

    fn linear(&mut self, prefix: &str, rows: usize, cols: usize) -> Result<Linear, String> {
        Ok(Linear { w: self.take(&format!("{prefix}.weight"), &[rows, cols])?, b: self.take(&format!("{prefix}.bias"), &[rows])?, rows, cols })
    }

    fn norm(&mut self, prefix: &str, d: usize, eps: f32) -> Result<Norm, String> {
        Ok(Norm { w: self.take(&format!("{prefix}.weight"), &[d])?, b: self.take(&format!("{prefix}.bias"), &[d])?, eps })
    }
}

/// Which heads a decision computes: the fight's, the custom screen's (its
/// pointer and the form list's entries), or all of them and the value (the
/// parity test).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Heads {
    Fight,
    Screen,
    All,
}

/// A decision's outputs, of the heads asked: the logits before the legal
/// masks (an absent slot's −∞), and the value.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Output {
    pub fight: Option<[f32; spec::FIGHT_ACTIONS]>,
    pub screen: Option<[f32; spec::SCREEN_ACTIONS]>,
    pub value: Option<f32>,
}

/// A player's state between decisions: the GRU's, the last decision's
/// action, and room for a pass (so a decision allocates nothing).
pub struct Memory {
    pub h: Vec<f32>,
    /// What the last decision took (none: a round's first decision is next).
    pub last: Option<Action>,
    s: Scratch,
}

#[derive(Default)]
struct Scratch {
    /// The present tokens' vectors (the transformer's first), `[n, d]`, and
    /// each one's set and place in it.
    t: Vec<f32>,
    at: Vec<(usize, usize)>,
    /// Where each slot of the pointer set is among them, and each one-token
    /// set's token.
    slot_at: Vec<Option<usize>>,
    single_at: Vec<Option<usize>>,
    xn: Vec<f32>,
    th: Vec<f32>,
    a: Vec<f32>,
    q: Vec<f32>,
    k: Vec<f32>,
    v: Vec<f32>,
    o: Vec<f32>,
    w: Vec<f32>,
    ff: Vec<f32>,
    row: Vec<f32>,
    scores: Vec<f32>,
    pool_in: Vec<f32>,
    z: Vec<f32>,
    gi: Vec<f32>,
    gh: Vec<f32>,
    core: Vec<f32>,
    hh: Vec<f32>,
    pc: Vec<f32>,
    pu: Vec<f32>,
}

impl Memory {
    /// A round's start: the GRU's state zeros, and no previous action.
    pub fn reset(&mut self) {
        self.h.fill(0.0);
        self.last = None;
    }

    /// The decision just made took `action`: the next one reads it.
    pub fn took(&mut self, action: Action) {
        self.last = Some(action);
    }
}

fn need(ok: bool, why: impl FnOnce() -> String) -> Result<(), String> {
    if ok { Ok(()) } else { Err(why()) }
}

impl Net {
    /// The network a weights file holds (§8's export): refused where its
    /// format, its version or its spec's hash isn't this encoder's, or its
    /// architecture or tensors aren't what this pass computes.
    pub fn from_safetensors(bytes: &[u8]) -> Result<Net, String> {
        let file = safetensors::parse(bytes).map_err(|e| format!("the weights: {e}"))?;
        let meta = file.metadata;
        let get = |k: &str| meta.get(k).map(String::as_str).ok_or_else(|| format!("the weights' metadata has no {k}"));
        need(get("format")? == FORMAT, || format!("the weights aren't {FORMAT}'s ({})", meta["format"]))?;
        need(get("format_version")? == FORMAT_VERSION.to_string(), || format!("the weights are of format version {}, this reads {FORMAT_VERSION}", meta["format_version"]))?;
        let ours = spec::hash();
        need(get("spec_hash")? == ours, || format!("the weights were trained on observation spec {}, this encoder's is {ours}", meta["spec_hash"]))?;
        let arch: Architecture = serde_json::from_str(get("architecture")?).map_err(|e| format!("the weights' architecture: {e}"))?;
        need(arch.format == FORMAT && arch.format_version == FORMAT_VERSION, || "the architecture isn't of this format".into())?;
        need(arch.spec.hash == ours, || "the architecture's spec hash isn't the weights'".into())?;
        need(arch.activation == "relu", || format!("an activation this pass hasn't, {}", arch.activation))?;
        need(arch.spec.buckets == names::BUCKETS as usize, || format!("{} name buckets, the encoder's are {}", arch.spec.buckets, names::BUCKETS))?;
        need(arch.spec.fight_actions == spec::FIGHT_ACTIONS && arch.spec.screen_actions == spec::SCREEN_ACTIONS, || "the actions aren't the encoder's".into())?;
        let levels: HashMap<String, Knobs> = serde_json::from_str(get("levels")?).map_err(|e| format!("the weights' levels: {e}"))?;
        let mut levels: Vec<Knobs> = levels.into_values().collect();
        levels.sort_by(|a, b| a.name.cmp(&b.name));
        for l in &levels {
            need(l.temperature > 0.0 && (0.0..=1.0).contains(&l.epsilon), || format!("level {}'s knobs are out of range", l.name))?;
        }
        need(arch.previous_action == PREVIOUS_ACTIONS, || format!("a previous action of {} values, this pass's has {PREVIOUS_ACTIONS}", arch.previous_action))?;
        let m = &arch.model;
        let (d, eps) = (m.d, arch.layer_norm_eps as f32);
        need(d > 0 && m.heads > 0 && d % m.heads == 0, || format!("d {d} isn't a multiple of {} heads", m.heads))?;

        // The sets, each found in the encoder's observation: one of its own,
        // or a part of one the trainer cut (`split`), which together cover
        // every token of every set of the encoder's once.
        let mut covered: Vec<Vec<bool>> = spec::SETS.iter().map(|s| vec![false; s.count]).collect();
        let mut w = Weights(file.tensors);
        let mut sets = Vec::new();
        for s in &arch.spec.sets {
            let part = arch.spec.split.iter().find_map(|(source, parts)| parts.iter().find(|p| p.0 == s.name).map(|p| (source.as_str(), p.1, p.2)));
            let (source, first, count) = part.unwrap_or((s.name.as_str(), 0, s.count));
            let src = spec::SETS.iter().position(|e| e.name == source).ok_or_else(|| format!("the network's set {} is of {source}, which the encoder hasn't", s.name))?;
            let e = spec::SETS[src];
            need(count == s.count && first + count <= e.count && s.width == e.width && s.ids == e.ids, || {
                format!("the network's set {} ({} × {}, {} ids) isn't the encoder's {source} from token {first}", s.name, s.count, s.width, s.ids)
            })?;
            for c in &mut covered[src][first..first + count] {
                need(!*c, || format!("the network reads a token of {source} twice"))?;
                *c = true;
            }
            let p = format!("enc.{}", s.name);
            sets.push(SetNet {
                name: s.name.clone(),
                source: src,
                first,
                count,
                width: s.width,
                ids: s.ids,
                pool_only: m.pool_only.contains(&s.name),
                mean: w.take(&format!("{p}.norm_mean"), &[s.width])?,
                inv_std: w.take(&format!("{p}.norm_inv_std"), &[s.width])?,
                fc1: w.linear(&format!("{p}.fc1"), m.token_hidden, s.width)?,
                fc2: w.linear(&format!("{p}.fc2"), d, m.token_hidden)?,
                type_emb: w.take(&format!("{p}.type_emb"), &[d])?,
            });
        }
        for (i, c) in covered.iter().enumerate() {
            need(c.iter().all(|&c| c), || format!("the network doesn't read all of the encoder's {}", spec::SETS[i].name))?;
        }
        for name in &m.pool_only {
            need(sets.iter().any(|s| &s.name == name), || format!("a pool-only set the network hasn't, {name}"))?;
        }
        let pointer = sets.iter().position(|s| s.name == m.pointer_set).ok_or_else(|| format!("no pointer set {}", m.pointer_set))?;
        need(!sets[pointer].pool_only, || "the pointer set is pool-only".into())?;
        need(sets[pointer].count == spec::SLOT_TOKENS && arch.slots == spec::SLOT_TOKENS && arch.entries == spec::FORM_ENTRIES, || {
            format!("the pointer set's {} slots and {} entries aren't the screen's", arch.slots, arch.entries)
        })?;
        let singles: Vec<usize> = (0..sets.len()).filter(|&i| sets[i].count == 1).collect();
        let names = w.take("names.weight", &[arch.spec.buckets, d])?;
        let mut blocks = Vec::new();
        for k in 0..m.layers {
            let p = format!("blocks.{k}");
            blocks.push(Block {
                ln1: w.norm(&format!("{p}.ln1"), d, eps)?,
                q: w.linear(&format!("{p}.q"), d, d)?,
                k: w.linear(&format!("{p}.k"), d, d)?,
                v: w.linear(&format!("{p}.v"), d, d)?,
                o: w.linear(&format!("{p}.o"), d, d)?,
                ln2: w.norm(&format!("{p}.ln2"), d, eps)?,
                ff1: w.linear(&format!("{p}.ff1"), m.ffn, d)?,
                ff2: w.linear(&format!("{p}.ff2"), d, m.ffn)?,
            });
        }
        let core = m.hidden + m.summary;
        // (The GRU's input weights, `[3H, summary + PREVIOUS_ACTIONS]`: the
        // summary's columns a Linear, the previous action's a column each.)
        let cols = m.summary + PREVIOUS_ACTIONS;
        let ih = w.take("gru.weight_ih_l0", &[3 * m.hidden, cols])?;
        let mut gru_previous = vec![0.0; PREVIOUS_ACTIONS * 3 * m.hidden];
        for (r, row) in ih.chunks(cols).enumerate() {
            for (a, &v) in row[m.summary..].iter().enumerate() {
                gru_previous[a * 3 * m.hidden + r] = v;
            }
        }
        let net = Net {
            ln_f: w.norm("ln_f", d, eps)?,
            pool_score: w.linear("pool_score", m.pool_heads, d)?,
            pool_out: w.linear("pool_out", m.summary, (m.pool_heads + singles.len()) * d)?,
            gru_ih: Linear {
                w: ih.chunks(cols).flat_map(|row| row[..m.summary].iter().copied()).collect(),
                b: w.take("gru.bias_ih_l0", &[3 * m.hidden])?,
                rows: 3 * m.hidden,
                cols: m.summary,
            },
            gru_previous,
            gru_hh: Linear {
                w: w.take("gru.weight_hh_l0", &[3 * m.hidden, m.hidden])?,
                b: w.take("gru.bias_hh_l0", &[3 * m.hidden])?,
                rows: 3 * m.hidden,
                cols: m.hidden,
            },
            fight1: w.linear("fight1", m.head_hidden, core)?,
            fight2: w.linear("fight2", spec::FIGHT_ACTIONS, m.head_hidden)?,
            entry1: w.linear("entry1", m.head_hidden, core)?,
            entry2: w.linear("entry2", arch.entries, m.head_hidden)?,
            ptr1: w.linear("ptr1", m.pointer_hidden, d + core)?,
            ptr2: w.linear("ptr2", 1, m.pointer_hidden)?,
            value1: w.linear("value1", m.head_hidden, core)?,
            value2: w.linear("value2", 1, m.head_hidden)?,
            tokens: sets.iter().map(|s| s.count).sum(),
            transformer_tokens: sets.iter().filter(|s| !s.pool_only).map(|s| s.count).sum(),
            d,
            heads: m.heads,
            clip: m.norm_clip,
            pool_heads: m.pool_heads,
            summary: m.summary,
            hidden: m.hidden,
            sets,
            names,
            blocks,
            singles,
            pointer,
            levels,
            metadata: meta,
        };
        let mut left: Vec<&String> = w.0.keys().collect();
        left.sort();
        need(left.is_empty(), || format!("the weights have tensors this pass doesn't read: {left:?}"))?;
        Ok(net)
    }

    /// The levels' knobs, by name.
    pub fn level(&self, name: &str) -> Option<&Knobs> {
        self.levels.iter().find(|l| l.name == name)
    }

    pub fn levels(&self) -> &[Knobs] {
        &self.levels
    }

    /// The weights' metadata (their provenance among it).
    pub fn metadata(&self) -> &HashMap<String, String> {
        &self.metadata
    }

    /// The GRU state's width.
    pub fn hidden(&self) -> usize {
        self.hidden
    }

    /// The network's token sets, in order: each one's name, and where its
    /// tokens are in the encoder's observation (the set, by `spec::SETS`'
    /// place, its first token, how many).
    pub fn sets(&self) -> impl Iterator<Item = (&str, usize, usize, usize)> {
        self.sets.iter().map(|s| (s.name.as_str(), s.source, s.first, s.count))
    }

    /// A player's memory: the GRU state at a round's start, and room for a
    /// pass.
    pub fn memory(&self) -> Memory {
        let d = self.d;
        let max_w = self.sets.iter().map(|s| s.width).max().unwrap_or(0);
        let th = self.sets.first().map_or(0, |s| s.fc1.rows);
        let nt = self.transformer_tokens;
        let ffn = self.blocks.first().map_or(0, |b| b.ff1.rows);
        let core = self.hidden + self.summary;
        let hh = self.fight1.rows;
        Memory {
            h: vec![0.0; self.hidden],
            last: None,
            s: Scratch {
                t: vec![0.0; self.tokens * d],
                at: Vec::with_capacity(self.tokens),
                slot_at: vec![None; spec::SLOT_TOKENS],
                single_at: vec![None; self.singles.len()],
                xn: vec![0.0; max_w],
                th: vec![0.0; th],
                a: vec![0.0; nt * d],
                q: vec![0.0; nt * d],
                k: vec![0.0; nt * d],
                v: vec![0.0; nt * d],
                o: vec![0.0; nt * d],
                w: vec![0.0; nt.max(self.tokens) * self.pool_heads.max(1)],
                ff: vec![0.0; ffn],
                row: vec![0.0; d],
                scores: vec![0.0; nt.max(self.tokens)],
                pool_in: vec![0.0; self.pool_out.cols],
                z: vec![0.0; self.summary],
                gi: vec![0.0; 3 * self.hidden],
                gh: vec![0.0; 3 * self.hidden],
                core: vec![0.0; core],
                hh: vec![0.0; hh],
                pc: vec![0.0; self.ptr1.rows],
                pu: vec![0.0; self.ptr1.rows],
            },
        }
    }

    /// One decision from observation `obs` (the encoder's, `spec::SETS`'
    /// layout): the GRU state `mem.h` carried forward, the last decision's
    /// action `mem.last` read (the caller says what this one took, with
    /// [`Memory::took`]), and the heads asked.
    pub fn forward(&self, obs: &Obs, mem: &mut Memory, heads: Heads) -> Output {
        let d = self.d;
        let Memory { h, last, s } = mem;
        // 1. The present tokens, the transformer's first.
        s.at.clear();
        s.slot_at.fill(None);
        s.single_at.fill(None);
        for pass in [false, true] {
            for (si, set) in self.sets.iter().enumerate().filter(|(_, set)| set.pool_only == pass) {
                let buf = &obs.sets[set.source];
                for k in 0..set.count {
                    let tok = set.first + k;
                    if !buf.mask[tok] {
                        continue;
                    }
                    let n = s.at.len();
                    let x = &buf.x[tok * set.width..(tok + 1) * set.width];
                    for (j, xn) in s.xn[..set.width].iter_mut().enumerate() {
                        *xn = ((x[j] - set.mean[j]) * set.inv_std[j]).clamp(-self.clip, self.clip);
                    }
                    set.fc1.relu(&s.xn[..set.width], &mut s.th);
                    let e = &mut s.t[n * d..(n + 1) * d];
                    set.fc2.apply(&s.th, e);
                    for (e, t) in e.iter_mut().zip(&set.type_emb) {
                        *e += t;
                    }
                    for &id in &buf.ids[tok * set.ids..(tok + 1) * set.ids] {
                        if id != 0 {
                            let id = id as usize;
                            axpy(1.0, &self.names[id * d..(id + 1) * d], e);
                        }
                    }
                    if si == self.pointer {
                        s.slot_at[k] = Some(n);
                    }
                    if let Some(i) = self.singles.iter().position(|&x| x == si) {
                        s.single_at[i] = Some(n);
                    }
                    s.at.push((si, k));
                }
            }
        }
        let n = s.at.len();
        let nt = s.at.iter().take_while(|(si, _)| !self.sets[*si].pool_only).count();
        // 2. The transformer layers over the present tokens of the sets that
        // aren't pool-only.
        if nt > 0 {
            let nh = self.heads;
            let dh = d / nh;
            let scale = 1.0 / (dh as f32).sqrt();
            for b in &self.blocks {
                for i in 0..nt {
                    b.ln1.apply(&s.t[i * d..(i + 1) * d], &mut s.a[i * d..(i + 1) * d]);
                }
                for i in 0..nt {
                    let a = &s.a[i * d..(i + 1) * d];
                    b.q.apply(a, &mut s.q[i * d..(i + 1) * d]);
                    b.k.apply(a, &mut s.k[i * d..(i + 1) * d]);
                    b.v.apply(a, &mut s.v[i * d..(i + 1) * d]);
                }
                s.o[..nt * d].fill(0.0);
                for hd in 0..nh {
                    let f = hd * dh..(hd + 1) * dh;
                    for i in 0..nt {
                        let q = &s.q[i * d + f.start..i * d + f.end];
                        let sc = &mut s.scores[..nt];
                        for (j, sc) in sc.iter_mut().enumerate() {
                            *sc = dot(q, &s.k[j * d + f.start..j * d + f.end]) * scale;
                        }
                        softmax(sc);
                        let o = &mut s.o[i * d + f.start..i * d + f.end];
                        for (j, &w) in s.scores[..nt].iter().enumerate() {
                            axpy(w, &s.v[j * d + f.start..j * d + f.end], o);
                        }
                    }
                }
                for i in 0..nt {
                    b.o.add(&s.o[i * d..(i + 1) * d], &mut s.t[i * d..(i + 1) * d]);
                    b.ln2.apply(&s.t[i * d..(i + 1) * d], &mut s.row);
                    b.ff1.relu(&s.row, &mut s.ff);
                    b.ff2.add(&s.ff, &mut s.t[i * d..(i + 1) * d]);
                }
            }
        }
        // Then the final LayerNorm on every present token.
        for i in 0..n {
            s.row.copy_from_slice(&s.t[i * d..(i + 1) * d]);
            self.ln_f.apply(&s.row, &mut s.t[i * d..(i + 1) * d]);
        }
        // 3. The summary: the attention pool's heads (each a softmax over the
        // present tokens of a learned score), the one-token sets' tokens
        // (zeros where absent), through a Linear and a ReLU.
        let p = self.pool_heads;
        s.pool_in.fill(0.0);
        if n > 0 {
            for i in 0..n {
                self.pool_score.apply(&s.t[i * d..(i + 1) * d], &mut s.w[i * p..(i + 1) * p]);
            }
            for hd in 0..p {
                let sc = &mut s.scores[..n];
                for (i, sc) in sc.iter_mut().enumerate() {
                    *sc = s.w[i * p + hd];
                }
                softmax(sc);
                let pooled = &mut s.pool_in[hd * d..(hd + 1) * d];
                for (i, &w) in s.scores[..n].iter().enumerate() {
                    axpy(w, &s.t[i * d..(i + 1) * d], pooled);
                }
            }
        }
        for (k, at) in s.single_at.iter().enumerate() {
            if let Some(i) = at {
                s.pool_in[(p + k) * d..(p + k + 1) * d].copy_from_slice(&s.t[i * d..(i + 1) * d]);
            }
        }
        self.pool_out.relu(&s.pool_in, &mut s.z);
        // 4. The GRU: gates r, z, n, its input the summary and the previous
        // action (one-hot: its column).
        let hd = self.hidden;
        self.gru_ih.apply(&s.z, &mut s.gi);
        let column = &self.gru_previous[previous_index(*last) * 3 * hd..][..3 * hd];
        for (g, w) in s.gi.iter_mut().zip(column) {
            *g += w;
        }
        self.gru_hh.apply(h, &mut s.gh);
        for i in 0..hd {
            let r = sigmoid(s.gi[i] + s.gh[i]);
            let u = sigmoid(s.gi[hd + i] + s.gh[hd + i]);
            let nn = (s.gi[2 * hd + i] + r * s.gh[2 * hd + i]).tanh();
            h[i] = (1.0 - u) * nn + u * h[i];
        }
        // 5. The heads, of core = [h', z].
        s.core[..hd].copy_from_slice(h);
        s.core[hd..].copy_from_slice(&s.z);
        let mut out = Output::default();
        if heads != Heads::Screen {
            let mut fight = [0.0; spec::FIGHT_ACTIONS];
            self.fight1.relu(&s.core, &mut s.hh);
            self.fight2.apply(&s.hh, &mut fight);
            out.fight = Some(fight);
        }
        if heads != Heads::Fight {
            let mut screen = [f32::NEG_INFINITY; spec::SCREEN_ACTIONS];
            self.entry1.relu(&s.core, &mut s.hh);
            self.entry2.apply(&s.hh, &mut screen[spec::SLOT_TOKENS..]);
            // (ptr1 over [the slot's token, core]: the core's part once.)
            let cols = self.ptr1.cols;
            for r in 0..self.ptr1.rows {
                s.pc[r] = dot(&self.ptr1.w[r * cols + d..(r + 1) * cols], &s.core) + self.ptr1.b[r];
            }
            for (slot, at) in s.slot_at.iter().enumerate() {
                let Some(i) = at else { continue };
                let t = &s.t[i * d..(i + 1) * d];
                for r in 0..self.ptr1.rows {
                    s.pu[r] = (dot(&self.ptr1.w[r * cols..r * cols + d], t) + s.pc[r]).max(0.0);
                }
                screen[slot] = dot(&self.ptr2.w, &s.pu) + self.ptr2.b[0];
            }
            out.screen = Some(screen);
        }
        if heads == Heads::All {
            let mut v = [0.0];
            self.value1.relu(&s.core, &mut s.hh);
            self.value2.apply(&s.hh, &mut v);
            out.value = Some(v[0]);
        }
        out
    }
}

fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

/// Softmax in place.
fn softmax(v: &mut [f32]) {
    let max = v.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let mut sum = 0.0;
    for x in v.iter_mut() {
        *x = (*x - max).exp();
        sum += *x;
    }
    let inv = 1.0 / sum;
    for x in v.iter_mut() {
        *x *= inv;
    }
}

/// `logits` masked: −∞ where an action isn't allowed.
pub fn mask(logits: &mut [f32], legal: &[bool]) {
    for (l, &ok) in logits.iter_mut().zip(legal) {
        if !ok {
            *l = f32::NEG_INFINITY;
        }
    }
}

/// A uniform number in [0, 1).
fn unit(rng: &mut SplitMix64) -> f32 {
    ((rng.next_u64() >> 40) as f32) / (1u64 << 24) as f32
}

/// The action a level takes from a decision's logits (§10, MODEL.md §6):
/// the allowed ones are those `legal` allows whose logit is finite; with
/// `greedy`, the first of the largest; else with chance `epsilon` one of
/// them at random, and otherwise a sample of softmax(logits / temperature)
/// over them. None: nothing is allowed.
pub fn choose(logits: &[f32], legal: &[bool], knobs: &Knobs, rng: &mut SplitMix64) -> Option<usize> {
    let allowed = |a: usize| legal.get(a).copied().unwrap_or(false) && logits[a].is_finite();
    let count = (0..logits.len()).filter(|&a| allowed(a)).count();
    if count == 0 {
        return None;
    }
    if knobs.greedy {
        let mut best: Option<usize> = None;
        for a in (0..logits.len()).filter(|&a| allowed(a)) {
            if best.is_none_or(|b| logits[a] > logits[b]) {
                best = Some(a);
            }
        }
        return best;
    }
    if knobs.epsilon > 0.0 && unit(rng) < knobs.epsilon {
        let k = rng.below(count as u64) as usize;
        return (0..logits.len()).filter(|&a| allowed(a)).nth(k);
    }
    let max = (0..logits.len()).filter(|&a| allowed(a)).map(|a| logits[a]).fold(f32::NEG_INFINITY, f32::max);
    let weight = |a: usize| if allowed(a) { ((logits[a] - max) / knobs.temperature).exp() } else { 0.0 };
    let total: f32 = (0..logits.len()).map(weight).sum();
    let mut at = unit(rng) * total;
    let mut last = None;
    for a in 0..logits.len() {
        let w = weight(a);
        if w > 0.0 {
            last = Some(a);
            if at < w {
                return Some(a);
            }
            at -= w;
        }
    }
    last
}

#[cfg(test)]
mod tests;
