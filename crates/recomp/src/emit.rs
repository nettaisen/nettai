//! Rust code generation for discovered functions.
//!
//! Each function becomes `fn name(c: &mut Cpu)`: a `loop { match pc { .. } }`
//! over its basic blocks. Registers and flags live in the `Cpu`; calls are
//! Rust calls. A callee finishes by storing its branch target in `c.pc`, and
//! the caller checks it against the expected return address, so non-standard
//! returns (returning two frames up, returning into the middle of the caller)
//! still transfer control correctly.

use crate::discover::{Func, Ins};
use crate::image::Image;
use crate::syms::{Mode, Syms};
use gba_isa::arm::{self, Arm, DpOp, HalfKind, HalfOffset, MemOffset, Operand2};
use gba_isa::thumb::{self, AluOp, MemOp, Thumb};
use gba_isa::{Cond, FLAG_C, FLAG_N, FLAG_Z, FLAGS_ALL, Shift};
use std::collections::BTreeMap;
use std::fmt::Write;

pub struct Emitter<'a> {
    pub image: &'a Image,
    pub syms: &'a Syms,
    /// Function entry -> Rust identifier.
    pub idents: &'a BTreeMap<u32, String>,
}

pub fn cond_expr(c: Cond) -> &'static str {
    match c {
        Cond::Eq => "c.z",
        Cond::Ne => "!c.z",
        Cond::Cs => "c.c",
        Cond::Cc => "!c.c",
        Cond::Mi => "c.n",
        Cond::Pl => "!c.n",
        Cond::Vs => "c.v",
        Cond::Vc => "!c.v",
        Cond::Hi => "c.c && !c.z",
        Cond::Ls => "!c.c || c.z",
        Cond::Ge => "c.n == c.v",
        Cond::Lt => "c.n != c.v",
        Cond::Gt => "!c.z && c.n == c.v",
        Cond::Le => "c.z || c.n != c.v",
        Cond::Al => "true",
        Cond::Nv => "false",
    }
}

fn hex(v: u32) -> String {
    format!("{v:#010x}")
}

/// `x + imm` as Rust, folding zero.
fn add_const(x: &str, imm: u32) -> String {
    if imm == 0 { x.to_string() } else { format!("{x}.wrapping_add({imm:#x})") }
}

fn sub_const(x: &str, imm: u32) -> String {
    if imm == 0 { x.to_string() } else { format!("{x}.wrapping_sub({imm:#x})") }
}

/// Where a branch goes, from the current function's point of view.
enum Dest {
    Block(u32),
    Tail(u32),
}

impl Emitter<'_> {
    fn dest(&self, f: &Func, t: u32) -> Dest {
        if f.leaders.contains(&t) {
            Dest::Block(t)
        } else if self.idents.contains_key(&t) {
            Dest::Tail(t)
        } else {
            // Discovery added every in-function target as a leader.
            panic!("branch target {t:#x} in {:#x} is neither block nor function", f.entry)
        }
    }

    fn jump(&self, f: &Func, t: u32) -> String {
        match self.dest(f, t) {
            Dest::Block(b) => format!("pc = {}; continue;", hex(b)),
            Dest::Tail(e) => format!("c.ret = ret; {}(c); return;", self.idents[&e]),
        }
    }

    fn call(&self, target: u32, ret_addr: u32, lr: u32) -> String {
        let callee = match self.idents.get(&target) {
            Some(n) => format!("{n}(c)"),
            None => format!("c.call_indirect({})", hex(target)),
        };
        format!(
            "c.r[14] = {}; c.ret = {}; {callee}; if c.pc != {} {{ pc = c.pc; continue; }}",
            hex(lr),
            hex(ret_addr),
            hex(ret_addr)
        )
    }

    fn indirect_call(&self, target_expr: &str, ret_addr: u32) -> String {
        format!(
            "let t = {target_expr}; c.ret = {}; c.call_indirect(t); if c.pc != {} {{ pc = c.pc; continue; }}",
            hex(ret_addr),
            hex(ret_addr)
        )
    }

    /// An indirect jump: returns, switch-table jumps, tail calls.
    fn indirect_jump(target_expr: &str) -> String {
        format!("pc = {target_expr}; if pc == ret {{ c.pc = pc; return; }} continue;")
    }

    fn const_comment(&self, v: u32) -> String {
        match self.syms.label(v & !1).or_else(|| self.syms.label(v)) {
            Some(l) => format!(" ={l}{}", if v & 1 != 0 && self.syms.label(v & !1).is_some() { "+1" } else { "" }),
            None => String::new(),
        }
    }

    pub fn emit_function(&self, f: &Func, name: &str, live: &BTreeMap<u32, u8>, out: &mut String) {
        let ident = &self.idents[&f.entry];
        let mode = match f.mode {
            Mode::Thumb => "thumb",
            Mode::Arm => "arm",
        };
        writeln!(out, "/// `{name}` ({}, {mode})", hex(f.entry)).unwrap();
        for p in &f.problems {
            writeln!(out, "/// PROBLEM: {p}").unwrap();
        }
        writeln!(out, "pub fn {ident}(c: &mut Cpu) {{").unwrap();
        writeln!(out, "    let ret = c.ret;").unwrap();
        writeln!(out, "    let mut pc: u32 = {};", hex(f.entry)).unwrap();
        writeln!(out, "    loop {{").unwrap();
        writeln!(out, "        match pc {{").unwrap();
        for &leader in &f.leaders {
            if !f.insns.contains_key(&leader) {
                // A target that couldn't be decoded (reported as a problem).
                writeln!(out, "            {} => c.undefined({}),", hex(leader), hex(leader)).unwrap();
                continue;
            }
            match self.syms.label(leader) {
                Some(l) if leader != f.entry => writeln!(out, "            // {l}").unwrap(),
                _ => {}
            }
            writeln!(out, "            {} => {{", hex(leader)).unwrap();
            let mut addr = leader;
            loop {
                if addr != leader && f.leaders.contains(&addr) {
                    writeln!(out, "                pc = {};", hex(addr)).unwrap();
                    break;
                }
                let Some(ins) = f.insns.get(&addr) else {
                    if self.idents.contains_key(&addr) {
                        writeln!(out, "                c.ret = ret; {}(c); return; // falls through", self.idents[&addr])
                            .unwrap();
                    } else {
                        writeln!(out, "                c.undefined({});", hex(addr)).unwrap();
                    }
                    break;
                };
                let lf = live.get(&addr).copied().unwrap_or(FLAGS_ALL);
                let (code, text, terminal) = match *ins {
                    Ins::T(t) => {
                        let (code, term) = self.thumb(f, addr, t, lf);
                        (code, thumb::Disasm { addr, instr: t, bl_target: None }.to_string(), term)
                    }
                    Ins::TBl { target } => {
                        let code = self.call(target, addr + 4, (addr + 4) | 1);
                        let name = self.syms.label(target).map(|s| s.to_string()).unwrap_or_else(|| hex(target));
                        (code, format!("bl {name}"), false)
                    }
                    Ins::A(a) => {
                        let (code, term) = self.arm(f, addr, a);
                        (code, arm::Disasm { addr, instr: a }.to_string(), term)
                    }
                };
                writeln!(out, "                {code} // {:07x}: {text}", addr & 0x0fff_ffff).unwrap();
                if terminal {
                    break;
                }
                addr += ins.size();
            }
            writeln!(out, "            }}").unwrap();
        }
        writeln!(out, "            _ => return c.fallback(pc, ret),").unwrap();
        writeln!(out, "        }}").unwrap();
        writeln!(out, "    }}").unwrap();
        writeln!(out, "}}").unwrap();
        writeln!(out).unwrap();
    }

    /// Emit one Thumb instruction. Returns (code, is_terminal).
    fn thumb(&self, f: &Func, pc: u32, ins: Thumb, lf: u8) -> (String, bool) {
        use Thumb::*;
        let pc4 = pc.wrapping_add(4);
        let nz = lf & (FLAG_N | FLAG_Z) != 0;
        let any = lf != 0;
        let need_c = lf & FLAG_C != 0;
        let r = |n: u8| -> String { if n == 15 { format!("{}u32", hex(pc4)) } else { format!("c.r[{n}]") } };
        let set_nz = |dst: &str| if nz { format!(" c.nz({dst});") } else { String::new() };
        let code = match ins {
            ShiftImm { op, rd, rs, imm } => {
                let d = r(rd);
                let s = r(rs);
                let n = if imm == 0 && op != Shift::Lsl { 32 } else { imm as u32 };
                let expr = match op {
                    Shift::Lsl if n == 0 => s.clone(),
                    Shift::Lsl if need_c => format!("c.lsl_imm_c({s}, {n})"),
                    Shift::Lsl => format!("{s} << {n}"),
                    Shift::Lsr if need_c => format!("c.lsr_imm_c({s}, {n})"),
                    Shift::Lsr if n == 32 => "0".into(),
                    Shift::Lsr => format!("{s} >> {n}"),
                    Shift::Asr if need_c => format!("c.asr_imm_c({s}, {n})"),
                    Shift::Asr => format!("(({s} as i32) >> {}) as u32", n.min(31)),
                    Shift::Ror => unreachable!(),
                };
                format!("{d} = {expr};{}", set_nz(&d))
            }
            AddSubReg { sub, rd, rs, rn } => self.addsub(&r(rd), &r(rs), &r(rn), sub, any),
            AddSubImm3 { sub, rd, rs, imm } => self.addsub(&r(rd), &r(rs), &format!("{imm}"), sub, any),
            MovImm { rd, imm } => {
                if nz {
                    format!("{} = {imm:#x}; c.n = false; c.z = {};", r(rd), imm == 0)
                } else {
                    format!("{} = {imm:#x};", r(rd))
                }
            }
            CmpImm { rd, imm } => {
                if any { format!("c.subs({}, {imm:#x});", r(rd)) } else { "/* flags dead */".into() }
            }
            AddImm { rd, imm } => self.addsub(&r(rd), &r(rd), &format!("{imm:#x}"), false, any),
            SubImm { rd, imm } => self.addsub(&r(rd), &r(rd), &format!("{imm:#x}"), true, any),
            Alu { op, rd, rs } => {
                let d = r(rd);
                let s = r(rs);
                let logical = |e: String| {
                    if nz { format!("{d} = c.nz({e});") } else { format!("{d} = {e};") }
                };
                match op {
                    AluOp::And => logical(format!("{d} & {s}")),
                    AluOp::Eor => logical(format!("{d} ^ {s}")),
                    AluOp::Orr => logical(format!("{d} | {s}")),
                    AluOp::Bic => logical(format!("{d} & !{s}")),
                    AluOp::Mvn => logical(format!("!{s}")),
                    AluOp::Mul => logical(format!("{d}.wrapping_mul({s})")),
                    AluOp::Tst => {
                        if nz { format!("c.nz({d} & {s});") } else { "/* flags dead */".into() }
                    }
                    AluOp::Lsl | AluOp::Lsr | AluOp::Asr | AluOp::Ror => {
                        let name = match op {
                            AluOp::Lsl => "lsl",
                            AluOp::Lsr => "lsr",
                            AluOp::Asr => "asr",
                            _ => "ror",
                        };
                        if need_c {
                            format!("{d} = c.{name}_reg_c({d}, {s});{}", set_nz(&d))
                        } else {
                            format!("{d} = {name}_reg({d}, {s});{}", set_nz(&d))
                        }
                    }
                    AluOp::Adc => {
                        if any { format!("{d} = c.adcs({d}, {s});") } else { format!("{d} = c.adc({d}, {s});") }
                    }
                    AluOp::Sbc => {
                        if any { format!("{d} = c.sbcs({d}, {s});") } else { format!("{d} = c.sbc({d}, {s});") }
                    }
                    AluOp::Neg => {
                        if any { format!("{d} = c.subs(0, {s});") } else { format!("{d} = 0u32.wrapping_sub({s});") }
                    }
                    AluOp::Cmp => {
                        if any { format!("c.subs({d}, {s});") } else { "/* flags dead */".into() }
                    }
                    AluOp::Cmn => {
                        if any { format!("c.adds({d}, {s});") } else { "/* flags dead */".into() }
                    }
                }
            }
            HiAdd { rd: 15, rs } => {
                return (Self::indirect_jump(&format!("{}.wrapping_add({}) & !1", hex(pc4), r(rs))), true);
            }
            HiAdd { rd, rs } => format!("{} = {}.wrapping_add({});", r(rd), r(rd), r(rs)),
            HiCmp { rd, rs } => format!("c.subs({}, {});", r(rd), r(rs)),
            HiMov { rd: 15, rs } => {
                if self.prev_is_mov_lr_pc(f, pc) {
                    self.indirect_call(&format!("{} & !1", r(rs)), pc + 2)
                } else {
                    return (Self::indirect_jump(&format!("{} & !1", r(rs))), true);
                }
            }
            HiMov { rd, rs } if rd == rs => "/* nop */".into(),
            HiMov { rd, rs } => format!("{} = {};", r(rd), r(rs)),
            Bx { rs: 15 } => {
                let t = pc4 & !3;
                return (format!("c.ret = ret; {}(c); return;", self.ident_or_indirect(t)), true);
            }
            Bx { rs } => {
                if self.prev_is_mov_lr_pc(f, pc) {
                    self.indirect_call(&format!("{} & !1", r(rs)), pc + 2)
                } else {
                    return (Self::indirect_jump(&format!("{} & !1", r(rs))), true);
                }
            }
            LdrPc { rd, imm } => {
                let a = (pc4 & !3) + imm as u32;
                match self.image.read32(a) {
                    Some(v) => format!("{} = {};{}", r(rd), hex(v), comment_tail(&self.const_comment(v))),
                    None => format!("{} = c.ld32({});", r(rd), hex(a)),
                }
            }
            MemReg { op, rd, rb, ro } => mem_op(op, &r(rd), &format!("{}.wrapping_add({})", r(rb), r(ro))),
            MemImm { op, rd, rb, imm } => mem_op(op, &r(rd), &add_const(&r(rb), imm as u32)),
            MemSp { load, rd, imm } => {
                mem_op(if load { MemOp::Ldr } else { MemOp::Str }, &r(rd), &add_const("c.r[13]", imm as u32))
            }
            AddPc { rd, imm } => format!("{} = {};", r(rd), hex((pc4 & !3) + imm as u32)),
            AddSp { rd, imm } => format!("{} = {};", r(rd), add_const("c.r[13]", imm as u32)),
            AdjustSp { imm } => {
                if imm >= 0 {
                    format!("c.r[13] = {};", add_const("c.r[13]", imm as u32))
                } else {
                    format!("c.r[13] = {};", sub_const("c.r[13]", (-imm) as u32))
                }
            }
            Push { rlist, lr } => {
                let mut regs: Vec<u8> = (0..8).filter(|i| rlist & (1 << i) != 0).collect();
                if lr {
                    regs.push(14);
                }
                let n = regs.len() as u32;
                let mut s = format!("let a = c.r[13].wrapping_sub({:#x});", 4 * n);
                for (i, reg) in regs.iter().enumerate() {
                    write!(s, " c.st32({}, c.r[{reg}]);", add_const("a", 4 * i as u32)).unwrap();
                }
                s.push_str(" c.r[13] = a;");
                s
            }
            Pop { rlist, pc: pop_pc } => {
                let regs: Vec<u8> = (0..8).filter(|i| rlist & (1 << i) != 0).collect();
                let n = regs.len() as u32 + pop_pc as u32;
                let mut s = "let a = c.r[13];".to_string();
                for (i, reg) in regs.iter().enumerate() {
                    write!(s, " c.r[{reg}] = c.ld32({});", add_const("a", 4 * i as u32)).unwrap();
                }
                if pop_pc {
                    write!(s, " let t = c.ld32({});", add_const("a", 4 * regs.len() as u32)).unwrap();
                }
                write!(s, " c.r[13] = a.wrapping_add({:#x});", 4 * n).unwrap();
                if pop_pc {
                    s.push(' ');
                    s.push_str(&Self::indirect_jump("t & !1"));
                    return (s, true);
                }
                s
            }
            Stmia { rb, rlist } => {
                let regs: Vec<u8> = (0..8).filter(|i| rlist & (1 << i) != 0).collect();
                if regs.is_empty() {
                    return ("c.undefined_behavior(\"stmia with empty list\");".into(), false);
                }
                let n = regs.len() as u32;
                let mut s = format!("let a = c.r[{rb}]; let wb = a.wrapping_add({:#x});", 4 * n);
                for (i, &reg) in regs.iter().enumerate() {
                    // ARM7TDMI: base in the list stores the old value if it's
                    // first, otherwise the written-back value.
                    let v = if reg == rb {
                        if i == 0 { "a".to_string() } else { "wb".to_string() }
                    } else {
                        format!("c.r[{reg}]")
                    };
                    write!(s, " c.st32({}, {v});", add_const("a", 4 * i as u32)).unwrap();
                }
                write!(s, " c.r[{rb}] = wb;").unwrap();
                s
            }
            Ldmia { rb, rlist } => {
                let regs: Vec<u8> = (0..8).filter(|i| rlist & (1 << i) != 0).collect();
                if regs.is_empty() {
                    return ("c.undefined_behavior(\"ldmia with empty list\");".into(), false);
                }
                let n = regs.len() as u32;
                let mut s = format!("let a = c.r[{rb}];");
                if rlist & (1 << rb) == 0 {
                    write!(s, " c.r[{rb}] = a.wrapping_add({:#x});", 4 * n).unwrap();
                }
                for (i, reg) in regs.iter().enumerate() {
                    write!(s, " c.r[{reg}] = c.ld32({});", add_const("a", 4 * i as u32)).unwrap();
                }
                s
            }
            BCond { cond, offset } => {
                let t = pc4.wrapping_add(offset as u32);
                format!("if {} {{ {} }}", cond_expr(cond), self.jump(f, t))
            }
            Swi { imm } => format!("c.swi_t({imm:#x}, {});", hex(pc)),
            B { offset } => {
                let t = pc4.wrapping_add(offset as u32);
                return (self.jump(f, t), true);
            }
            BlHi { .. } | BlLo { .. } | Undefined(_) => {
                return (format!("c.undefined({});", hex(pc)), true);
            }
        };
        (code, false)
    }

    fn addsub(&self, d: &str, a: &str, b: &str, sub: bool, flags: bool) -> String {
        match (sub, flags) {
            (false, true) => format!("{d} = c.adds({a}, {b});"),
            (true, true) => format!("{d} = c.subs({a}, {b});"),
            (false, false) => format!("{d} = {a}.wrapping_add({b});"),
            (true, false) => format!("{d} = {a}.wrapping_sub({b});"),
        }
    }

    fn prev_is_mov_lr_pc(&self, f: &Func, pc: u32) -> bool {
        match f.mode {
            Mode::Thumb => matches!(f.insns.get(&(pc.wrapping_sub(2))), Some(Ins::T(Thumb::HiMov { rd: 14, rs: 15 }))),
            Mode::Arm => matches!(
                f.insns.get(&(pc.wrapping_sub(4))),
                Some(Ins::A(Arm::DataProc {
                    op: DpOp::Mov,
                    rd: 14,
                    op2: Operand2::ShiftImm { rm: 15, shift: Shift::Lsl, amount: 0 },
                    ..
                }))
            ),
        }
    }

    fn ident_or_indirect(&self, t: u32) -> String {
        match self.idents.get(&t) {
            Some(n) => n.clone(),
            None => format!("(|c: &mut Cpu| c.call_indirect({}))", hex(t)),
        }
    }

    /// Emit one ARM instruction. Returns (code, is_terminal).
    fn arm(&self, f: &Func, pc: u32, ins: Arm) -> (String, bool) {
        let cond = ins.cond();
        let always = cond == Cond::Al;
        let pc8 = pc.wrapping_add(8);
        let r = |n: u8| -> String { if n == 15 { format!("{}u32", hex(pc8)) } else { format!("c.r[{n}]") } };
        let wrap = |body: String, terminal: bool| -> (String, bool) {
            if always { (body, terminal) } else { (format!("if {} {{ {body} }}", cond_expr(cond)), false) }
        };
        match ins {
            Arm::DataProc { op, s, rn, rd, op2, .. } => {
                let logical_s = s && op.is_logical();
                let mut pre = String::new();
                // The shifter operand, with carry-out for logical flag-setting ops.
                let o2 = match op2 {
                    Operand2::Imm { value, rot } => {
                        if logical_s && rot != 0 {
                            write!(pre, "c.c = {}; ", value >> 31 != 0).unwrap();
                        }
                        hex(value)
                    }
                    Operand2::ShiftImm { rm, shift, amount } => {
                        let v = r(rm);
                        let n = amount as u32;
                        match (shift, n) {
                            (Shift::Lsl, 0) => v,
                            (Shift::Ror, 0) => {
                                if logical_s { format!("c.rrx_c({v})") } else { format!("rrx({v}, c.c)") }
                            }
                            (sh, n) => {
                                let n = if n == 0 { 32 } else { n };
                                let name = sh.mnemonic();
                                if logical_s {
                                    format!("c.{name}_imm_c({v}, {n})")
                                } else {
                                    format!("{name}_imm({v}, {n})")
                                }
                            }
                        }
                    }
                    Operand2::ShiftReg { rm, shift, rs } => {
                        let v = if rm == 15 { format!("{}u32", hex(pc.wrapping_add(12))) } else { r(rm) };
                        let name = shift.mnemonic();
                        if logical_s {
                            format!("c.{name}_reg_c({v}, c.r[{rs}])")
                        } else {
                            format!("{name}_reg({v}, c.r[{rs}])")
                        }
                    }
                };
                let rn_v = match op2 {
                    Operand2::ShiftReg { .. } if rn == 15 => format!("{}u32", hex(pc.wrapping_add(12))),
                    _ => r(rn),
                };
                let mut body = format!("{pre}let o2: u32 = {o2}; ");
                let result = match op {
                    DpOp::And => Some(format!("{rn_v} & o2")),
                    DpOp::Eor => Some(format!("{rn_v} ^ o2")),
                    DpOp::Orr => Some(format!("{rn_v} | o2")),
                    DpOp::Bic => Some(format!("{rn_v} & !o2")),
                    DpOp::Mov => Some("o2".to_string()),
                    DpOp::Mvn => Some("!o2".to_string()),
                    DpOp::Tst => {
                        body.push_str(&format!("c.nz({rn_v} & o2);"));
                        None
                    }
                    DpOp::Teq => {
                        body.push_str(&format!("c.nz({rn_v} ^ o2);"));
                        None
                    }
                    DpOp::Cmp => {
                        body.push_str(&format!("c.subs({rn_v}, o2);"));
                        None
                    }
                    DpOp::Cmn => {
                        body.push_str(&format!("c.adds({rn_v}, o2);"));
                        None
                    }
                    DpOp::Add => Some(if s { format!("c.adds({rn_v}, o2)") } else { format!("{rn_v}.wrapping_add(o2)") }),
                    DpOp::Sub => Some(if s { format!("c.subs({rn_v}, o2)") } else { format!("{rn_v}.wrapping_sub(o2)") }),
                    DpOp::Rsb => Some(if s { format!("c.subs(o2, {rn_v})") } else { format!("o2.wrapping_sub({rn_v})") }),
                    DpOp::Adc => Some(if s { format!("c.adcs({rn_v}, o2)") } else { format!("c.adc({rn_v}, o2)") }),
                    DpOp::Sbc => Some(if s { format!("c.sbcs({rn_v}, o2)") } else { format!("c.sbc({rn_v}, o2)") }),
                    DpOp::Rsc => Some(if s { format!("c.sbcs(o2, {rn_v})") } else { format!("c.sbc(o2, {rn_v})") }),
                };
                if let Some(res) = result {
                    let res = if op.is_logical() && s { format!("c.nz({res})") } else { res };
                    if rd == 15 {
                        if s {
                            return wrap(format!("{body}c.exception_return({res});"), true);
                        }
                        if self.prev_is_mov_lr_pc(f, pc) {
                            return wrap(format!("{body}{}", self.indirect_call(&format!("({res}) & !3"), pc + 4)), false);
                        }
                        return wrap(format!("{body}{}", Self::indirect_jump(&format!("({res}) & !3"))), true);
                    }
                    body.push_str(&format!("c.r[{rd}] = {res};"));
                }
                wrap(body, false)
            }
            Arm::Mul { acc, s, rd, rn, rs, rm, .. } => {
                let mut e = format!("c.r[{rm}].wrapping_mul(c.r[{rs}])");
                if acc {
                    e = format!("{e}.wrapping_add(c.r[{rn}])");
                }
                if s {
                    e = format!("c.nz({e})");
                }
                wrap(format!("c.r[{rd}] = {e};"), false)
            }
            Arm::MulLong { signed, acc, s, rdhi, rdlo, rs, rm, .. } => {
                let prod = if signed {
                    format!("((c.r[{rm}] as i32 as i64).wrapping_mul(c.r[{rs}] as i32 as i64)) as u64")
                } else {
                    format!("(c.r[{rm}] as u64).wrapping_mul(c.r[{rs}] as u64)")
                };
                let mut body = format!("let mut v: u64 = {prod};");
                if acc {
                    body.push_str(&format!(" v = v.wrapping_add(((c.r[{rdhi}] as u64) << 32) | c.r[{rdlo}] as u64);"));
                }
                body.push_str(&format!(" c.r[{rdlo}] = v as u32; c.r[{rdhi}] = (v >> 32) as u32;"));
                if s {
                    body.push_str(" c.n = (v >> 63) != 0; c.z = v == 0;");
                }
                wrap(body, false)
            }
            Arm::Swp { byte, rd, rm, rn, .. } => {
                let body = if byte {
                    format!("let a = c.r[{rn}]; let t = c.ld8(a); c.st8(a, c.r[{rm}]); c.r[{rd}] = t;")
                } else {
                    format!("let a = c.r[{rn}]; let t = c.ld32(a); c.st32(a, c.r[{rm}]); c.r[{rd}] = t;")
                };
                wrap(body, false)
            }
            Arm::Bx { rm, .. } => {
                if self.prev_is_mov_lr_pc(f, pc) {
                    wrap(self.indirect_call(&format!("c.r[{rm}] & !1"), pc + 4), false)
                } else {
                    wrap(Self::indirect_jump(&format!("c.r[{rm}] & !1")), true)
                }
            }
            Arm::Mrs { spsr, rd, .. } => {
                wrap(format!("c.r[{rd}] = {};", if spsr { "c.spsr()" } else { "c.cpsr()" }), false)
            }
            Arm::MsrReg { spsr, fields, rm, .. } => wrap(
                format!("c.{}(c.r[{rm}], {fields:#x});", if spsr { "set_spsr" } else { "set_cpsr" }),
                false,
            ),
            Arm::MsrImm { spsr, fields, value, .. } => wrap(
                format!("c.{}({}, {fields:#x});", if spsr { "set_spsr" } else { "set_cpsr" }, hex(value)),
                false,
            ),
            Arm::Mem { load: true, byte: false, pre: true, up, wb: false, rn: 15, rd, offset: MemOffset::Imm(i), .. }
                if rd != 15 =>
            {
                let a = if up { pc8.wrapping_add(i) } else { pc8.wrapping_sub(i) };
                match self.image.read32(a) {
                    Some(v) => wrap(format!("c.r[{rd}] = {};{}", hex(v), comment_tail(&self.const_comment(v))), false),
                    None => wrap(format!("c.r[{rd}] = c.ld32({});", hex(a)), false),
                }
            }
            Arm::Mem { load, byte, pre, up, wb, rn, rd, offset, .. } => {
                let off = match offset {
                    MemOffset::Imm(i) => hex(i),
                    MemOffset::Reg { rm, shift, amount } => {
                        let v = r(rm);
                        match (shift, amount) {
                            (Shift::Lsl, 0) => v,
                            (Shift::Ror, 0) => format!("rrx({v}, c.c)"),
                            (sh, n) => format!("{}_imm({v}, {})", sh.mnemonic(), if n == 0 { 32 } else { n as u32 }),
                        }
                    }
                };
                let base = r(rn);
                let moved = if up { format!("{base}.wrapping_add({off})") } else { format!("{base}.wrapping_sub({off})") };
                let mut body = format!("let a = {}; ", if pre { moved.clone() } else { base.clone() });
                let writeback = (!pre || wb) && rn != 15;
                if writeback {
                    body.push_str(&format!("let wb = {moved}; "));
                }
                if load {
                    body.push_str(&format!("let v = {}; ", if byte { "c.ld8(a)" } else { "c.ld32(a)" }));
                    if writeback {
                        body.push_str(&format!("c.r[{rn}] = wb; "));
                    }
                    if rd == 15 {
                        if self.prev_is_mov_lr_pc(f, pc) {
                            return wrap(format!("{body}{}", self.indirect_call("v & !1", pc + 4)), false);
                        }
                        return wrap(format!("{body}{}", Self::indirect_jump("v & !1")), true);
                    }
                    body.push_str(&format!("c.r[{rd}] = v;"));
                } else {
                    let v = if rd == 15 { hex(pc.wrapping_add(12)) } else { format!("c.r[{rd}]") };
                    body.push_str(&format!("{}(a, {v});", if byte { "c.st8" } else { "c.st32" }));
                    if writeback {
                        body.push_str(&format!(" c.r[{rn}] = wb;"));
                    }
                }
                wrap(body, false)
            }
            Arm::MemHalf { load, kind, pre, up, wb, rn, rd, offset, .. } => {
                let off = match offset {
                    HalfOffset::Imm(i) => hex(i as u32),
                    HalfOffset::Reg(rm) => r(rm),
                };
                let base = r(rn);
                let moved = if up { format!("{base}.wrapping_add({off})") } else { format!("{base}.wrapping_sub({off})") };
                let mut body = format!("let a = {}; ", if pre { moved.clone() } else { base.clone() });
                let writeback = (!pre || wb) && rn != 15;
                if writeback {
                    body.push_str(&format!("let wb = {moved}; "));
                }
                if load {
                    let ld = match kind {
                        HalfKind::H => "c.ld16(a)",
                        HalfKind::Sb => "c.ld8s(a)",
                        HalfKind::Sh => "c.ld16s(a)",
                    };
                    body.push_str(&format!("let v = {ld}; "));
                    if writeback {
                        body.push_str(&format!("c.r[{rn}] = wb; "));
                    }
                    body.push_str(&format!("c.r[{rd}] = v;"));
                } else {
                    body.push_str(&format!("c.st16(a, c.r[{rd}]);"));
                    if writeback {
                        body.push_str(&format!(" c.r[{rn}] = wb;"));
                    }
                }
                wrap(body, false)
            }
            Arm::Block { load, pre, up, s, wb, rn, rlist, .. } => {
                if s {
                    return wrap("c.undefined_behavior(\"ldm/stm with S bit\");".into(), false);
                }
                let regs: Vec<u8> = (0..16).filter(|i| rlist & (1 << i) != 0).collect();
                let n = regs.len() as u32;
                let start = match (pre, up) {
                    (false, true) => "a".to_string(),
                    (true, true) => "a.wrapping_add(4)".to_string(),
                    (false, false) => format!("a.wrapping_sub({:#x})", 4 * n - 4),
                    (true, false) => format!("a.wrapping_sub({:#x})", 4 * n),
                };
                let new_base = if up { format!("a.wrapping_add({:#x})", 4 * n) } else { format!("a.wrapping_sub({:#x})", 4 * n) };
                let mut body = format!("let a = c.r[{rn}]; let s = {start}; let nb = {new_base}; ");
                if load {
                    if wb && rlist & (1 << rn) == 0 {
                        body.push_str(&format!("c.r[{rn}] = nb; "));
                    }
                    let mut target = None;
                    for (i, &reg) in regs.iter().enumerate() {
                        let a = add_const("s", 4 * i as u32);
                        if reg == 15 {
                            body.push_str(&format!("let t = c.ld32({a}); "));
                            target = Some(());
                        } else {
                            body.push_str(&format!("c.r[{reg}] = c.ld32({a}); "));
                        }
                    }
                    if target.is_some() {
                        return wrap(format!("{body}{}", Self::indirect_jump("t & !1")), true);
                    }
                } else {
                    for (i, &reg) in regs.iter().enumerate() {
                        let a = add_const("s", 4 * i as u32);
                        let v = if reg == 15 {
                            hex(pc.wrapping_add(12))
                        } else if reg == rn && wb {
                            if i == 0 { "a".into() } else { "nb".into() }
                        } else {
                            format!("c.r[{reg}]")
                        };
                        body.push_str(&format!("c.st32({a}, {v}); "));
                    }
                    if wb {
                        body.push_str(&format!("c.r[{rn}] = nb;"));
                    }
                }
                wrap(body, false)
            }
            Arm::Branch { link, offset, .. } => {
                let t = pc8.wrapping_add(offset as u32);
                if link {
                    wrap(self.call(t, pc + 4, pc + 4), false)
                } else {
                    wrap(self.jump(f, t), true)
                }
            }
            Arm::Swi { imm, .. } => wrap(format!("c.swi_a({:#x}, {});", imm >> 16, hex(pc)), false),
            Arm::Undefined(_) => (format!("c.undefined({});", hex(pc)), true),
        }
    }
}

fn comment_tail(s: &str) -> String {
    if s.is_empty() { String::new() } else { format!(" //{s}") }
}

fn mem_op(op: MemOp, rd: &str, addr: &str) -> String {
    match op {
        MemOp::Str => format!("c.st32({addr}, {rd});"),
        MemOp::Strh => format!("c.st16({addr}, {rd});"),
        MemOp::Strb => format!("c.st8({addr}, {rd});"),
        MemOp::Ldr => format!("{rd} = c.ld32({addr});"),
        MemOp::Ldrh => format!("{rd} = c.ld16({addr});"),
        MemOp::Ldrb => format!("{rd} = c.ld8({addr});"),
        MemOp::Ldsb => format!("{rd} = c.ld8s({addr});"),
        MemOp::Ldsh => format!("{rd} = c.ld16s({addr});"),
    }
}
