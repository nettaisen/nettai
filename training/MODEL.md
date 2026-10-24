# The network's forward pass, for the Rust port

What `nettai_ai/model.py` computes for one decision of one seat, exactly, in the order the Rust side
(`crates/nettai-training/src/net.rs`) must follow. The weights file is written by `nettai_ai.export`; its
parity fixture checks the port (§ "Checking the port").

Notation: `Linear(x)` with a weight `W` of shape `[out, in]` and bias `b` is `W·x + b`. Every tensor below is
float32. `d`, `H` (GRU), and the other widths come from the file's `architecture` metadata; the sets, their
order, counts `C`, widths `W` and id counts `I` come from its `spec`.

## The file

- safetensors, every tensor under the PyTorch name listed here. Metadata (strings):
  - `format` = `"nettai-ai"`, `format_version` = `"1"`: refuse anything else.
  - `spec_hash`: refuse weights whose hash isn't the encoder's own spec hash.
  - `architecture`: JSON with `model` (the widths, `pointer_set`, `pool_only`), `spec` (`buckets`,
    `fight_actions`, `screen_actions`, `sets` in order with `name`, `count`, `width`, `ids`, and `split`),
    `activation` (`"relu"`), `layer_norm_eps` (`1e-5`), `slots` (the pointer set's count) and `keys`
    (`screen_actions - slots`).
  - `spec.split`: sets the trainer cut from one of the encoder's, as `{source: [[set, first token, count], …]}`.
    The encoder's `chips` set (7 tokens) is read as `hand` (tokens 0–5) and `used` (6), two sets of their own,
    each with `chips`' width and ids, in `chips`' place in the order.
  - `levels`: JSON, by level name: `delay` (ticks), `temperature`, `epsilon`, `greedy` (§10).
  - `k`: ticks a fight decision is held, and `screen_decisions`: screen decisions before only OK is allowed, as
    the env asked for decisions in training.
  - `git_commit`, `git_dirty`, `update`, `decisions`: provenance only.
- The pointer set is `model.pointer_set` (`"slots"`). Its tokens, in order, are screen actions `0 .. slots-1`;
  screen actions `slots ..` are the keys.

## Inputs

For each set `s` in the architecture's order: `x[s]` `[C, W]`, `ids[s]` `[C, I]` (int32, 0 = no name),
`mask[s]` `[C]` (true = present). With spec c4d9e4a61b0f54ba the trainer's sets are, in order: `general` (1),
`panels` (18), `objects` (48), `hand` (6) and `used` (1) (the encoder's `chips` set, split as `spec.split` says:
its tokens 0–5 and 6, same width and ids), `slots` (12, the pointer set), `rules` (96, pool-only) and `forms`
(12). Widths and id counts are in the architecture. Also `legal_fight` `[20]`, `legal_screen` `[19]`, and the GRU state `h` `[H]` (zeros at a
round's start). These are the pool's buffers for one seat (§6.6).

Absent tokens are never read: skip them everywhere. (PyTorch computes them and masks them out; the result is
the same.)

## 1. Tokens

For each present token `i` of set `s` (`p = enc.<s>.`):

1. Normalize: `xn = clamp((x − p.norm_mean) ⊙ p.norm_inv_std, −c, c)` elementwise, `c = model.norm_clip` (5).
   `norm_mean`, `norm_inv_std` are `[W]`.
2. MLP: `e = p.fc2(relu(p.fc1(xn)))` (`fc1`: `[token_hidden, W]`, `fc2`: `[d, token_hidden]`).
3. `e += p.type_emb` (`[d]`).
4. Names: for each of the token's `I` ids, `e += names.weight[id]` when `id != 0` (`names.weight`: `[buckets, d]`,
   one table shared by every set). Each name comes as two ids (two seeds of the hash); both are summed.

The present tokens of all sets form one list `t` of `n` vectors of width `d`. Keep where each came from (its
set and index): the pointer set's tokens and the one-token sets are read again after step 2.

## 2. Transformer layers (`model.layers` of them, `blocks.<k>.`, pre-LayerNorm)

Only the tokens of sets not in `model.pool_only` (`["rules"]`) take part: they are the queries and the keys.
The pool-only sets' tokens skip this step unchanged and are read by step 3 only.

`LN(v) = (v − mean(v)) / sqrt(var(v) + 1e-5) ⊙ weight + bias`, mean and (biased, divide by `d`) variance over
the `d` features.

For each layer, with `nh = model.heads`, `dh = d / nh`:

1. `a_i = blocks.k.ln1(t_i)` for every present token.
2. `q_i = q(a_i)`, `k_i = k(a_i)`, `v_i = v(a_i)` (each `[d, d]`). Head `j` uses features `[j·dh, (j+1)·dh)`.
3. For each head and each token `i`: `w_ij = softmax_j(q_i·k_j / sqrt(dh))` over the present tokens `j`
   (including `i`), `o_i = Σ_j w_ij v_j` on that head's features. Concatenate the heads back in order.
4. `t_i += o(o_i)` (`[d, d]`).
5. `t_i += ff2(relu(ff1(ln2(t_i))))` (`ff1`: `[ffn, d]`, `ff2`: `[d, ffn]`).

With no present token at all, there is nothing to do here.

Then `t_i = ln_f(t_i)` for every present token, pool-only sets' included.

## 3. Summary

1. Pool, `P = model.pool_heads`: `s_i = pool_score(t_i)` (`[P, d]` → `[P]`). For each head `p`:
   `a_ip = softmax_i(s_ip)` over present tokens; `pooled_p = Σ_i a_ip t_i` (`[d]`). No present token:
   `pooled_p = 0`.
2. Direct reads: each set whose count is 1, in spec order, gives its token `t` (after `ln_f`), or zeros if absent.
3. `z = relu(pool_out([pooled_0, …, pooled_{P−1}, direct_0, …]))`, concatenated in that order
   (`pool_out`: `[summary, (P + ones)·d]`).

## 4. GRU (`gru.`, PyTorch's equations, gate order r, z, n)

`weight_ih_l0` `[3H, summary]`, `weight_hh_l0` `[3H, H]`, `bias_ih_l0`, `bias_hh_l0` `[3H]`. Rows `[0, H)` are
r, `[H, 2H)` are z, `[2H, 3H)` are n. With `gi = weight_ih_l0·z + bias_ih_l0`, `gh = weight_hh_l0·h + bias_hh_l0`:

```
r  = σ(gi[r] + gh[r])
u  = σ(gi[z] + gh[z])
nn = tanh(gi[n] + r ⊙ gh[n])
h' = (1 − u) ⊙ nn + u ⊙ h
```

`h'` is kept for the next decision. At a round's start, `h = 0`.

## 5. Heads

`core = [h', z]` (width `H + summary`).

- Fight: `fight = fight2(relu(fight1(core)))`, `[fight_actions]`.
- Keys: `keys = key2(relu(key1(core)))`, `[keys]`.
- Slot pointer: for each token `i` of the pointer set, in order: if present,
  `u_i = relu(ptr1.weight[:, :d]·t_i + ptr1.weight[:, d:]·core + ptr1.bias)` (the core's part is the same for
  every slot: compute it once) and `slot_i = ptr2(u_i)` (a scalar); if absent, `slot_i = −inf`. (`t_i` is the
  token after `ln_f`.)
- `screen = [slot_0, …, slot_{slots−1}, keys_0, …, keys_{keys−1}]`, `[screen_actions]`.
- Value: `value2(relu(value1(core)))`. Training only; the app needn't compute it.

Masks: `fight[a] = −inf` where `!legal_fight[a]`; `screen[a] = −inf` where `!legal_screen[a]`. A fight
decision reads only `fight`, a screen decision only `screen`.

## 6. Choosing (§10's levels)

From the masked logits `ℓ` of the decision's head and the level's knobs:

- `greedy`: the legal action with the largest `ℓ`.
- otherwise: with probability `epsilon` a uniformly random legal action; else sample from
  `softmax(ℓ / temperature)` over legal actions.

The reaction delay is the level's `delay` in ticks: the decision at tick t uses the observation of tick t − delay.

## Checking the port

`nettai_ai.export` also writes `<name>.fixture.safetensors`: one seat's recorded decisions (from training)
and the outputs, computed decision by decision from `h0`:

- inputs: `x.<set>`, `ids.<set>`, `mask.<set>` (`[N, C, …]`), `legal_fight` `[N, 20]`, `legal_screen` `[N, 19]`,
  `phase` `[N]` (1 fight, 2 screen), `reset` `[N]` (set `h = 0` before that decision), `h0` `[H]`;
- outputs: `fight_raw` `[N, 20]` and `screen_raw` `[N, 19]` (before the legal masks; absent slots −inf),
  `fight` and `screen` (masked), `value` `[N]`, `hidden` `[N, H]` (the GRU state after each decision).

The port should match every finite value to about 1e-4 (float32 sums in another order) and every −inf exactly.

## Cost

`nettai_ai.model.count_macs` counts the multiply-adds of one decision over present tokens only; `uv run python
-m nettai_ai.model [--size large]` prints them. The default is about 2.2 M at a typical fight position
(95 tokens, 38 of them in the transformer) and 5.8 M with every token present (§7).
