//! Control-flow discovery: which instructions belong to each function, where
//! its blocks start, and which other functions it calls.

use crate::image::Image;
use crate::syms::Mode;
use gba_isa::arm::{self, Arm, DpOp, Operand2};
use gba_isa::thumb::{self, Thumb};
use gba_isa::{Cond, Shift};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug)]
pub enum Ins {
    T(Thumb),
    /// A paired Thumb bl (4 bytes) with its resolved target.
    TBl { target: u32 },
    A(Arm),
}

impl Ins {
    pub fn size(&self) -> u32 {
        match self {
            Ins::T(_) => 2,
            Ins::TBl { .. } | Ins::A(_) => 4,
        }
    }
}

pub struct Func {
    pub entry: u32,
    pub mode: Mode,
    pub insns: BTreeMap<u32, Ins>,
    pub leaders: BTreeSet<u32>,
    /// Direct call targets (and the mode they are entered in).
    pub calls: BTreeSet<(u32, Mode)>,
    /// Addresses of indirect jumps that aren't recognizable returns or calls.
    pub computed_jumps: Vec<u32>,
    pub problems: Vec<String>,
}

pub struct Ctx<'a> {
    pub image: &'a Image,
    /// All known function entries.
    pub entries: &'a BTreeMap<u32, Mode>,
    pub code_refs: &'a BTreeSet<u32>,
}

impl Ctx<'_> {
    fn is_other_entry(&self, addr: u32, own: u32) -> bool {
        addr != own && self.entries.contains_key(&addr)
    }

    /// Exclusive end of the address range that nominally belongs to the
    /// function at `entry` (the next function's entry).
    fn range_end(&self, entry: u32) -> u32 {
        self.entries.range(entry + 1..).next().map(|(a, _)| *a).unwrap_or(entry + 0x1000)
    }
}

/// Whether the Thumb instruction is `mov lr, pc` (the indirect-call idiom).
fn is_thumb_mov_lr_pc(i: Option<&Ins>) -> bool {
    matches!(i, Some(Ins::T(Thumb::HiMov { rd: 14, rs: 15 })))
}

fn is_arm_mov_lr_pc(i: Option<&Ins>) -> bool {
    matches!(
        i,
        Some(Ins::A(Arm::DataProc {
            op: DpOp::Mov,
            rd: 14,
            op2: Operand2::ShiftImm { rm: 15, shift: Shift::Lsl, amount: 0 },
            ..
        }))
    )
}

pub fn discover(ctx: &Ctx, entry: u32, mode: Mode) -> Func {
    let mut f = Func {
        entry,
        mode,
        insns: BTreeMap::new(),
        leaders: BTreeSet::from([entry]),
        calls: BTreeSet::new(),
        computed_jumps: Vec::new(),
        problems: Vec::new(),
    };
    let mut queue = vec![entry];
    let mut added_refs = false;
    while let Some(start) = queue.pop() {
        let mut pc = start;
        loop {
            if f.insns.contains_key(&pc) {
                break;
            }
            if pc != start && ctx.is_other_entry(pc, entry) {
                break; // falls through into another function
            }
            let stop = match mode {
                Mode::Thumb => step_thumb(ctx, &mut f, &mut queue, &mut added_refs, pc),
                Mode::Arm => step_arm(ctx, &mut f, &mut queue, &mut added_refs, pc),
            };
            match stop {
                Some(next) => pc = next,
                None => break,
            }
        }
    }
    f
}

fn add_target(ctx: &Ctx, f: &mut Func, queue: &mut Vec<u32>, t: u32) {
    if ctx.is_other_entry(t, f.entry) {
        return; // tail call
    }
    if f.leaders.insert(t) {
        queue.push(t);
    }
}

/// Candidate targets of a computed jump: code addresses referenced from data
/// that fall inside this function's nominal range.
fn add_code_refs(ctx: &Ctx, f: &mut Func, queue: &mut Vec<u32>, added: &mut bool) {
    if *added {
        return;
    }
    *added = true;
    let end = ctx.range_end(f.entry);
    let refs: Vec<u32> = ctx.code_refs.range(f.entry + 1..end).copied().collect();
    for r in refs {
        add_target(ctx, f, queue, r);
    }
}

/// Decode one Thumb instruction at `pc`; returns the next linear address, or
/// None if control never falls through.
fn step_thumb(ctx: &Ctx, f: &mut Func, queue: &mut Vec<u32>, added: &mut bool, pc: u32) -> Option<u32> {
    let Some(op) = ctx.image.read16(pc) else {
        f.problems.push(format!("{pc:#010x}: fetch outside image"));
        return None;
    };
    let ins = thumb::decode(op);
    if let Thumb::BlHi { offset } = ins {
        if let Some(Thumb::BlLo { offset: lo }) = ctx.image.read16(pc + 2).map(thumb::decode) {
            let target = pc.wrapping_add(4).wrapping_add(offset as u32).wrapping_add(lo as u32);
            f.insns.insert(pc, Ins::TBl { target });
            f.calls.insert((target, Mode::Thumb));
            return Some(pc + 4);
        }
        f.problems.push(format!("{pc:#010x}: unpaired bl"));
        f.insns.insert(pc, Ins::T(Thumb::Undefined(op)));
        return None;
    }
    let prev_is_mov_lr_pc = is_thumb_mov_lr_pc(f.insns.get(&(pc.wrapping_sub(2))));
    f.insns.insert(pc, Ins::T(ins));
    let pc4 = pc.wrapping_add(4);
    match ins {
        Thumb::B { offset } => {
            add_target(ctx, f, queue, pc4.wrapping_add(offset as u32));
            None
        }
        Thumb::BCond { offset, .. } => {
            add_target(ctx, f, queue, pc4.wrapping_add(offset as u32));
            Some(pc + 2)
        }
        Thumb::Bx { rs: 15 } => {
            // Switch to ARM at the next word: an ARM function in its own right.
            let t = pc4 & !3;
            f.calls.insert((t, Mode::Arm));
            None
        }
        Thumb::Bx { rs } | Thumb::HiMov { rd: 15, rs } => {
            if prev_is_mov_lr_pc {
                Some(pc + 2) // indirect call; returns to the next instruction
            } else {
                if rs != 14 && !is_return_idiom(f, pc) {
                    f.computed_jumps.push(pc);
                    add_code_refs(ctx, f, queue, added);
                }
                None
            }
        }
        Thumb::HiAdd { rd: 15, .. } => {
            f.computed_jumps.push(pc);
            add_code_refs(ctx, f, queue, added);
            None
        }
        Thumb::Pop { pc: true, .. } => None,
        Thumb::Undefined(_) | Thumb::BlLo { .. } => {
            f.problems.push(format!("{pc:#010x}: undefined instruction {op:#06x}"));
            None
        }
        _ => Some(pc + 2),
    }
}

/// `pop {rN}; bx rN` is GCC's interworking return.
fn is_return_idiom(f: &Func, pc: u32) -> bool {
    let Some(Ins::T(Thumb::Bx { rs })) = f.insns.get(&pc) else { return false };
    matches!(f.insns.get(&(pc.wrapping_sub(2))), Some(Ins::T(Thumb::Pop { rlist, pc: false })) if *rlist == 1 << rs)
}

fn step_arm(ctx: &Ctx, f: &mut Func, queue: &mut Vec<u32>, added: &mut bool, pc: u32) -> Option<u32> {
    let Some(op) = ctx.image.read32(pc) else {
        f.problems.push(format!("{pc:#010x}: fetch outside image"));
        return None;
    };
    let ins = arm::decode(op);
    let prev_is_mov_lr_pc = is_arm_mov_lr_pc(f.insns.get(&(pc.wrapping_sub(4))));
    f.insns.insert(pc, Ins::A(ins));
    let always = ins.cond() == Cond::Al;
    let next = if always { None } else { Some(pc + 4) };
    let pc8 = pc.wrapping_add(8);
    match ins {
        Arm::Branch { link: false, offset, .. } => {
            add_target(ctx, f, queue, pc8.wrapping_add(offset as u32));
            next
        }
        Arm::Branch { link: true, offset, .. } => {
            f.calls.insert((pc8.wrapping_add(offset as u32), Mode::Arm));
            Some(pc + 4)
        }
        Arm::Bx { rm, .. } => {
            if prev_is_mov_lr_pc {
                Some(pc + 4)
            } else {
                if rm != 14 {
                    f.computed_jumps.push(pc);
                    add_code_refs(ctx, f, queue, added);
                }
                next
            }
        }
        Arm::DataProc { op, rd: 15, rn, op2, .. } if !op.is_test() => {
            if prev_is_mov_lr_pc {
                return Some(pc + 4);
            }
            // add pc, pc, rX, lsl #2: a table of branches follows at pc + 8.
            if op == DpOp::Add
                && rn == 15
                && matches!(op2, Operand2::ShiftImm { shift: Shift::Lsl, amount: 2, rm } if rm != 15)
            {
                let mut t = pc8;
                while let Some(Arm::Branch { cond: Cond::Al, link: false, .. }) =
                    ctx.image.read32(t).map(arm::decode)
                {
                    add_target(ctx, f, queue, t);
                    t += 4;
                    if t - pc8 > 1024 {
                        break;
                    }
                }
            } else if !matches!(op2, Operand2::ShiftImm { rm: 14, shift: Shift::Lsl, amount: 0 }) {
                f.computed_jumps.push(pc);
                add_code_refs(ctx, f, queue, added);
            }
            next
        }
        Arm::Mem { load: true, rd: 15, .. } => {
            if prev_is_mov_lr_pc {
                return Some(pc + 4);
            }
            f.computed_jumps.push(pc);
            add_code_refs(ctx, f, queue, added);
            next
        }
        Arm::Block { load: true, rlist, .. } if rlist & 0x8000 != 0 => next,
        Arm::Undefined(_) => {
            f.problems.push(format!("{pc:#010x}: undefined ARM instruction {op:#010x}"));
            None
        }
        _ => Some(pc + 4),
    }
}
