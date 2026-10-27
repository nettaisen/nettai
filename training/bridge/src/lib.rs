//! `nettai_gym`: the Python trainer's view of nettai-training's pool
//! (docs/design/training-ai.md §6.6).
//!
//! ```python
//! import nettai_gym
//! pool = nettai_gym.Pool(games=["exe6", "exe5", "exe4"], envs=256, threads=8, seed=1)
//! spec = pool.spec()        # sets in order, their counts, widths and features; the spec's hash
//! buf = pool.buffers()      # numpy views of the pool's own memory, refilled by every step
//! pool.step(actions)        # int32 [E, 2], read where buf["phase"] != 0
//! ```
//!
//! The arrays `buffers()` gives are views of memory the pool owns (each
//! keeps the pool alive): no copy is made, and a step changes what they
//! show. `step` lets go of the GIL while the workers run.

use nettai_training::builds::{Holdout, HoldoutMode};
use nettai_training::env::{Config, Controller};
use nettai_training::pool::{Pool, PoolConfig};
use nettai_training::{names, spec};
use numpy::ndarray::{ArrayView, IxDyn};
use numpy::{Element, PyArrayDyn, PyReadonlyArrayDyn};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::sync::Mutex;

fn err(e: String) -> PyErr {
    PyRuntimeError::new_err(e)
}

/// The pool, as Python holds it.
#[pyclass(name = "Pool", module = "nettai_gym")]
struct PyPool {
    pool: Mutex<Pool>,
}

impl PyPool {
    fn pool(&self) -> std::sync::MutexGuard<'_, Pool> {
        self.pool.lock().unwrap_or_else(|p| p.into_inner())
    }
}

/// A numpy view of `len` elements at `ptr`, shaped `shape`, kept alive by
/// `owner`.
///
/// SAFETY: `ptr` is `len` elements of the pool's buffers, which `owner`
/// (the pool) keeps where they are for its life.
unsafe fn view<'py, T: Element>(owner: &Bound<'py, PyPool>, ptr: *const T, len: usize, shape: &[usize]) -> Bound<'py, PyArrayDyn<T>> {
    assert_eq!(shape.iter().product::<usize>(), len, "a buffer's shape");
    unsafe {
        let a = ArrayView::from_shape_ptr(IxDyn(shape), ptr);
        PyArrayDyn::borrow_from_array(&a, owner.clone().into_any())
    }
}

#[pymethods]
impl PyPool {
    #[new]
    #[pyo3(signature = (games, envs=256, threads=8, seed=1, content=None, k=4, delay=(6, 30), max_fight_ticks=18000, hp=None, chip_holdout=(0.0, 0, "exclude".to_string()), practice=0.25, randomize=0.75, full_lists=0.5, combos=0.3, cache=None, builders=0, mirror=false, fixed_builds=0))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        py: Python<'_>,
        games: Vec<String>,
        envs: usize,
        threads: usize,
        seed: u64,
        content: Option<std::path::PathBuf>,
        k: u32,
        delay: (u16, u16),
        max_fight_ticks: u32,
        hp: Option<(u16, u16)>,
        chip_holdout: (f64, u64, String),
        practice: f64,
        randomize: f64,
        full_lists: f64,
        combos: f64,
        cache: Option<std::path::PathBuf>,
        builders: usize,
        mirror: bool,
        fixed_builds: u32,
    ) -> PyResult<PyPool> {
        if k == 0 || delay.0 > delay.1 || hp.is_some_and(|(lo, hi)| lo == 0 || lo > hi) {
            return Err(PyValueError::new_err(format!("k {k}, delay {delay:?}, hp {hp:?}: k is at least 1, each range low to high, HP above 0")));
        }
        let env = Config { k, delay, max_fight_ticks, hp, holdout: holdout(chip_holdout)?, practice, randomize, full_lists, combos, mirror, fixed_builds, ..Config::default() };
        let cfg = PoolConfig { games, envs, threads, seed, content, env, cache, builders };
        let pool = py.detach(|| Pool::new(cfg)).map_err(err)?;
        Ok(PyPool { pool: Mutex::new(pool) })
    }

    /// The observation's layout (§6.6): the spec's hash, the names'
    /// buckets, the actions, and each set's count, width, ids and features,
    /// in the sets' order.
    fn spec<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let d = PyDict::new(py);
        d.set_item("hash", spec::hash())?;
        d.set_item("buckets", names::BUCKETS)?;
        d.set_item("fight_actions", spec::FIGHT_ACTIONS)?;
        d.set_item("screen_actions", spec::SCREEN_ACTIONS)?;
        d.set_item("form_entries", spec::FORM_ENTRIES)?;
        d.set_item("probed_keys", spec::PROBED_KEYS.to_vec())?;
        let chips = PyDict::new(py);
        chips.set_item("hand", (0, spec::HAND_TOKENS))?;
        chips.set_item("used", (spec::USED_TOKEN, 1))?;
        d.set_item("chip_tokens", chips)?;
        d.set_item("fingerprint", spec::FINGERPRINT)?;
        d.set_item("fingerprint_features", spec::fingerprint::FIELDS.to_vec())?;
        d.set_item("games", self.pool().games().to_vec())?;
        // (What the envs use, which the weights record: the ticks a fight
        // decision holds, the decisions on a custom screen before only OK.)
        let cfg = self.pool().config().clone();
        d.set_item("k", cfg.k)?;
        d.set_item("screen_decisions", cfg.screen_decisions)?;
        let sets = PyDict::new(py);
        for s in spec::SETS {
            let set = PyDict::new(py);
            set.set_item("count", s.count)?;
            set.set_item("width", s.width)?;
            set.set_item("ids", s.ids)?;
            set.set_item("id_names", s.id_names.to_vec())?;
            set.set_item("features", s.fields.to_vec())?;
            sets.set_item(s.name, set)?;
        }
        d.set_item("sets", sets)?;
        Ok(d)
    }

    /// The buffers (§6.6), as numpy views of the pool's memory.
    fn buffers<'py>(this: &Bound<'py, Self>) -> PyResult<Bound<'py, PyDict>> {
        let py = this.py();
        let d = PyDict::new(py);
        let me = this.borrow();
        let mut pool = me.pool();
        let b = pool.buffers_mut();
        let e = b.envs;
        let (x, ids, mask) = (PyDict::new(py), PyDict::new(py), PyDict::new(py));
        // SAFETY: each view is of a buffer of the pool's, whole, which the
        // pool keeps where it is while it lives (the view keeps it alive).
        unsafe {
            for (k, s) in spec::SETS.iter().enumerate() {
                x.set_item(s.name, view(this, b.x[k].as_ptr(), b.x[k].len(), &[e, 2, s.count, s.width]))?;
                ids.set_item(s.name, view(this, b.ids[k].as_ptr(), b.ids[k].len(), &[e, 2, s.count, s.ids]))?;
                mask.set_item(s.name, view(this, b.mask[k].as_ptr(), b.mask[k].len(), &[e, 2, s.count]))?;
            }
            d.set_item("x", x)?;
            d.set_item("ids", ids)?;
            d.set_item("mask", mask)?;
            d.set_item("phase", view(this, b.phase.as_ptr(), b.phase.len(), &[e, 2]))?;
            d.set_item("legal_fight", view(this, b.legal_fight.as_ptr(), b.legal_fight.len(), &[e, 2, spec::FIGHT_ACTIONS]))?;
            d.set_item("legal_screen", view(this, b.legal_screen.as_ptr(), b.legal_screen.len(), &[e, 2, spec::SCREEN_ACTIONS]))?;
            d.set_item("hp_gain", view(this, b.hp_gain.as_ptr(), b.hp_gain.len(), &[e, 2]))?;
            d.set_item("done", view(this, b.done.as_ptr(), b.done.len(), &[e]))?;
            d.set_item("truncated", view(this, b.truncated.as_ptr(), b.truncated.len(), &[e]))?;
            d.set_item("ending", view(this, b.ending.as_ptr(), b.ending.len(), &[e]))?;
            d.set_item("result", view(this, b.result.as_ptr(), b.result.len(), &[e, 2]))?;
            d.set_item("game", view(this, b.game.as_ptr(), b.game.len(), &[e]))?;
            d.set_item("delay", view(this, b.delay.as_ptr(), b.delay.len(), &[e, 2]))?;
        }
        Ok(d)
    }

    /// Apply `actions` (int32 `[E, 2]`, read where the last step asked a
    /// seat) and step every env to its next decision, the GIL let go.
    fn step(&self, py: Python<'_>, actions: PyReadonlyArrayDyn<'_, i32>) -> PyResult<()> {
        let actions: Vec<i32> = actions.as_array().iter().copied().collect();
        py.detach(|| self.pool().step(&actions)).map_err(err)
    }

    /// Who plays seat `seat` of envs `envs` from their next round:
    /// "policy", "still", "mover", "masher", "shooter" or "dodger".
    fn set_seat(&self, envs: Vec<usize>, seat: usize, controller: &str) -> PyResult<()> {
        let c: Controller = nettai_training::pool::controller(controller).map_err(|e| PyValueError::new_err(e))?;
        self.pool().set_seat(&envs, seat, c).map_err(err)
    }

    /// The reaction delay seat `seat` of envs `envs` plays at from their
    /// next round, in ticks (None: drawn again).
    #[pyo3(signature = (envs, seat, ticks=None))]
    fn set_delay(&self, envs: Vec<usize>, seat: usize, ticks: Option<u16>) -> PyResult<()> {
        self.pool().set_delay(&envs, seat, ticks).map_err(err)
    }

    /// Envs `envs` dealt the same rounds from `key` from their next round
    /// (None: each from its own draws): evaluation's duplicate matches.
    #[pyo3(signature = (envs, key=None))]
    fn set_deal(&self, envs: Vec<usize>, key: Option<u64>) -> PyResult<()> {
        self.pool().set_deal(&envs, key).map_err(err)
    }

    /// The ticks seat `seat` of envs `envs` holds a fight decision from
    /// their next round (None: the env's k): a network meets another
    /// trained at another k each at its own.
    #[pyo3(signature = (envs, seat, k=None))]
    fn set_k(&self, envs: Vec<usize>, seat: usize, k: Option<u32>) -> PyResult<()> {
        self.pool().set_k(&envs, seat, k).map_err(err)
    }

    /// Both sides' base HP drawn from `lo` to `hi` each round (where the
    /// navi takes one), from the next rounds; `set_hp(None)`: each side's as
    /// its build has it (the creator's, the default).
    #[pyo3(signature = (lo, hi=None))]
    fn set_hp(&self, lo: Option<u16>, hi: Option<u16>) -> PyResult<()> {
        let hp = lo.map(|lo| (lo, hi.unwrap_or(lo)));
        self.pool().set_hp(hp).map_err(err)
    }

    /// The chips held out of training from the next rounds: those a folder
    /// may hold whose key's number with `seed` is below `fraction`
    /// (`nettai_gym.holdout_unit`); `mode` "exclude" (random builds never
    /// use them) or "favor" (folders lean toward them: about half a
    /// folder's draws).
    fn set_chip_holdout(&self, fraction: f64, seed: u64, mode: String) -> PyResult<()> {
        let h = holdout((fraction, seed, mode))?;
        self.pool().set_chip_holdout(h).map_err(err)
    }

    /// The rounds ended since the last call (§9's measures): each
    /// `{"env", "ticks", "seats": [dict, dict], ...}`, a seat's dict its
    /// "result" (+1, -1, 0) and its measures.
    fn round_stats<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        let stats = self.pool().round_stats().map_err(err)?;
        let out = PyList::empty(py);
        for r in stats {
            let d = PyDict::new(py);
            d.set_item("env", r.env)?;
            d.set_item("ticks", r.ticks)?;
            d.set_item("game", &r.game)?;
            d.set_item("fight_ticks", r.fight_ticks)?;
            d.set_item("screens", r.screens)?;
            d.set_item("truncated", r.truncated)?;
            d.set_item("ending", r.ending)?;
            d.set_item("error", r.error.clone())?;
            d.set_item("full_gauge", r.full_gauge)?;
            d.set_item("folder_order", r.folder_order)?;
            let seats = PyList::empty(py);
            for s in 0..2 {
                let seat = PyDict::new(py);
                seat.set_item("result", r.result[s])?;
                seat.set_item("controller", &r.controllers[s])?;
                seat.set_item("navi", &r.navis[s])?;
                seat.set_item("delay", r.delay[s])?;
                seat.set_item("max_hp", r.max_hp[s])?;
                seat.set_item("hp_lost", r.hp_lost[s])?;
                seat.set_item("hits_taken", r.hits_taken[s])?;
                seat.set_item("chips_used", r.chips_used[s])?;
                seat.set_item("attack_chips", r.attack_chips[s])?;
                seat.set_item("attacks_landed", r.attacks_landed[s])?;
                seat.set_item("chips_picked", r.chips_picked[s])?;
                seat.set_item("steps", r.steps[s])?;
                seat.set_item("steps_back", r.steps_back[s])?;
                seat.set_item("forms_taken", r.forms_taken[s])?;
                seat.set_item("forms_entered", r.forms_entered[s])?;
                seat.set_item("form_ticks", r.form_ticks[s])?;
                // (A folder seeded with a pair: the pair, and what came of it.)
                seat.set_item("pair", r.pair[s].clone())?;
                seat.set_item("pair_picked", r.pair_picked[s])?;
                seat.set_item("pair_ordered", r.pair_ordered[s])?;
                seat.set_item("pair_used", r.pair_used[s])?;
                seat.set_item("payoff_damage", r.payoff_damage[s])?;
                seats.append(seat)?;
            }
            d.set_item("seats", seats)?;
            out.append(d)?;
        }
        Ok(out)
    }

    /// Where the envs' time went (nanoseconds) and what they did, summed.
    fn profile<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let p = self.pool().profile();
        let d = PyDict::new(py);
        d.set_item("encode_ns", p.encode)?;
        d.set_item("reset_ns", p.reset)?;
        d.set_item("probe_ns", p.probe)?;
        d.set_item("prepare_ns", p.prepare)?;
        d.set_item("ticks", p.ticks)?;
        d.set_item("decisions", p.decisions)?;
        d.set_item("rounds", p.rounds)?;
        Ok(d)
    }

    /// The pairs of chips each game's fingerprints found (a setup, a
    /// payoff, the condition, the ratio), by game.
    fn pairs<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let pool = self.pool();
        let d = PyDict::new(py);
        for (name, pairs) in pool.pairs() {
            let list = PyList::empty(py);
            for (setup, payoff, condition, ratio) in pairs {
                list.append((setup, payoff, condition, ratio))?;
            }
            d.set_item(name, list)?;
        }
        Ok(d)
    }
}

/// A chip holdout from Python's `(fraction, seed, mode)`.
fn holdout((fraction, seed, mode): (f64, u64, String)) -> PyResult<Holdout> {
    let mode = HoldoutMode::named(&mode).ok_or_else(|| PyValueError::new_err(format!("chip holdout mode {mode:?}: \"exclude\" or \"favor\"")))?;
    if !(0.0..=1.0).contains(&fraction) {
        return Err(PyValueError::new_err(format!("a fraction held out of {fraction}: 0 to 1")));
    }
    Ok(Holdout { fraction, seed, mode })
}

/// A chip key's number in [0, 1) with `seed`: the chip is held out where it
/// is below the fraction (§6.6).
#[pyfunction]
fn holdout_unit(seed: u64, key: &str) -> f64 {
    names::unit(seed, key)
}

/// A name's two ids (§3), as the observation carries them: for checking the
/// trainer's own hash against the pool's.
#[pyfunction]
fn name_ids(name: &str) -> (i32, i32) {
    let [a, b] = names::ids(name);
    (a, b)
}

#[pymodule]
fn nettai_gym(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyPool>()?;
    m.add_function(wrap_pyfunction!(name_ids, m)?)?;
    m.add_function(wrap_pyfunction!(holdout_unit, m)?)?;
    Ok(())
}
