//! Backward flag liveness for Thumb functions, so instructions whose flag
//! results are never read can skip computing them.

use crate::discover::{Func, Ins};
use gba_isa::thumb::{AluOp, Thumb};
use gba_isa::{FLAG_C, FLAG_N, FLAG_V, FLAG_Z, FLAGS_ALL, Shift};
use std::collections::BTreeMap;

const NZ: u8 = FLAG_N | FLAG_Z;
const NZCV: u8 = FLAGS_ALL;

/// (flags read, flags always written) for one instruction.
fn effects(ins: &Ins, is_call: bool) -> (u8, u8) {
    if is_call {
        // Conservatively, a callee may read the caller's flags; it always
        // returns with flags of its own.
        return (FLAGS_ALL, FLAGS_ALL);
    }
    let Ins::T(t) = ins else { return (FLAGS_ALL, 0) };
    use Thumb::*;
    match *t {
        ShiftImm { op: Shift::Lsl, imm: 0, .. } => (0, NZ),
        ShiftImm { .. } => (0, NZ | FLAG_C),
        AddSubReg { .. } | AddSubImm3 { .. } | CmpImm { .. } | AddImm { .. } | SubImm { .. } => (0, NZCV),
        MovImm { .. } => (0, NZ),
        Alu { op, .. } => match op {
            AluOp::And | AluOp::Eor | AluOp::Orr | AluOp::Bic | AluOp::Mvn | AluOp::Tst | AluOp::Mul => (0, NZ),
            // A register shift by zero leaves C alone.
            AluOp::Lsl | AluOp::Lsr | AluOp::Asr | AluOp::Ror => (FLAG_C, NZ | FLAG_C),
            AluOp::Adc | AluOp::Sbc => (FLAG_C, NZCV),
            AluOp::Neg | AluOp::Cmp | AluOp::Cmn => (0, NZCV),
        },
        HiCmp { .. } => (0, NZCV),
        BCond { cond, .. } => (cond.flags_read(), 0),
        _ => (0, 0),
    }
}

/// Flags live after each instruction. `is_call(addr)` says whether the
/// instruction at `addr` is a call; `succs(addr)` gives in-function
/// successors and whether control may also leave the function.
pub fn thumb_live_out(
    f: &Func,
    is_call: impl Fn(u32) -> bool,
    succs: impl Fn(u32) -> (Vec<u32>, bool),
) -> BTreeMap<u32, u8> {
    let addrs: Vec<u32> = f.insns.keys().copied().collect();
    let mut live_in: BTreeMap<u32, u8> = addrs.iter().map(|&a| (a, 0)).collect();
    let mut live_out: BTreeMap<u32, u8> = addrs.iter().map(|&a| (a, 0)).collect();
    let info: BTreeMap<u32, ((u8, u8), (Vec<u32>, bool))> = addrs
        .iter()
        .map(|&a| (a, (effects(&f.insns[&a], is_call(a)), succs(a))))
        .collect();
    loop {
        let mut changed = false;
        for &a in addrs.iter().rev() {
            let ((reads, writes), (ss, exits)) = &info[&a];
            let mut out = if *exits { FLAGS_ALL } else { 0 };
            for s in ss {
                out |= live_in.get(s).copied().unwrap_or(FLAGS_ALL);
            }
            let inn = reads | (out & !writes);
            if out != live_out[&a] || inn != live_in[&a] {
                live_out.insert(a, out);
                live_in.insert(a, inn);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let _ = FLAG_V;
    live_out
}
