//! ARM (32-bit) instruction decoding, ARMv4T subset used by GBA software.

use crate::{Cond, Shift, reg_list, reg_name};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DpOp {
    And,
    Eor,
    Sub,
    Rsb,
    Add,
    Adc,
    Sbc,
    Rsc,
    Tst,
    Teq,
    Cmp,
    Cmn,
    Orr,
    Mov,
    Bic,
    Mvn,
}

impl DpOp {
    fn from_bits(b: u32) -> DpOp {
        use DpOp::*;
        [And, Eor, Sub, Rsb, Add, Adc, Sbc, Rsc, Tst, Teq, Cmp, Cmn, Orr, Mov, Bic, Mvn][(b & 15) as usize]
    }

    pub fn mnemonic(self) -> &'static str {
        use DpOp::*;
        match self {
            And => "and",
            Eor => "eor",
            Sub => "sub",
            Rsb => "rsb",
            Add => "add",
            Adc => "adc",
            Sbc => "sbc",
            Rsc => "rsc",
            Tst => "tst",
            Teq => "teq",
            Cmp => "cmp",
            Cmn => "cmn",
            Orr => "orr",
            Mov => "mov",
            Bic => "bic",
            Mvn => "mvn",
        }
    }

    /// Logical ops set C from the shifter; arithmetic ops from the ALU.
    pub fn is_logical(self) -> bool {
        use DpOp::*;
        matches!(self, And | Eor | Tst | Teq | Orr | Mov | Bic | Mvn)
    }

    /// Compare ops write no result.
    pub fn is_test(self) -> bool {
        use DpOp::*;
        matches!(self, Tst | Teq | Cmp | Cmn)
    }

    pub fn uses_rn(self) -> bool {
        !matches!(self, DpOp::Mov | DpOp::Mvn)
    }
}

/// The second operand of a data-processing instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operand2 {
    /// Rotated immediate: `value` is already rotated; `rot` is the rotate
    /// amount (nonzero rotates set C to bit 31 of the value for logical ops).
    Imm { value: u32, rot: u8 },
    /// rm shifted by an immediate. Raw 5-bit amount: for lsr/asr 0 means 32,
    /// for ror 0 means rrx.
    ShiftImm { rm: u8, shift: Shift, amount: u8 },
    /// rm shifted by the bottom byte of rs.
    ShiftReg { rm: u8, shift: Shift, rs: u8 },
}

/// Offset of a single data transfer (ldr/str).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemOffset {
    Imm(u32),
    /// rm shifted by an immediate, as in Operand2::ShiftImm.
    Reg { rm: u8, shift: Shift, amount: u8 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HalfKind {
    H,
    Sb,
    Sh,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HalfOffset {
    Imm(u8),
    Reg(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arm {
    DataProc { cond: Cond, op: DpOp, s: bool, rn: u8, rd: u8, op2: Operand2 },
    Mul { cond: Cond, acc: bool, s: bool, rd: u8, rn: u8, rs: u8, rm: u8 },
    MulLong { cond: Cond, signed: bool, acc: bool, s: bool, rdhi: u8, rdlo: u8, rs: u8, rm: u8 },
    Swp { cond: Cond, byte: bool, rd: u8, rm: u8, rn: u8 },
    Bx { cond: Cond, rm: u8 },
    Mrs { cond: Cond, spsr: bool, rd: u8 },
    /// `fields` is the 4-bit field mask (c, x, s, f).
    MsrReg { cond: Cond, spsr: bool, fields: u8, rm: u8 },
    MsrImm { cond: Cond, spsr: bool, fields: u8, value: u32 },
    Mem {
        cond: Cond,
        load: bool,
        byte: bool,
        pre: bool,
        up: bool,
        wb: bool,
        rn: u8,
        rd: u8,
        offset: MemOffset,
    },
    MemHalf {
        cond: Cond,
        load: bool,
        kind: HalfKind,
        pre: bool,
        up: bool,
        wb: bool,
        rn: u8,
        rd: u8,
        offset: HalfOffset,
    },
    Block { cond: Cond, load: bool, pre: bool, up: bool, s: bool, wb: bool, rn: u8, rlist: u16 },
    /// Offset relative to instruction address + 8.
    Branch { cond: Cond, link: bool, offset: i32 },
    Swi { cond: Cond, imm: u32 },
    Undefined(u32),
}

impl Arm {
    pub fn cond(&self) -> Cond {
        use Arm::*;
        match *self {
            DataProc { cond, .. }
            | Mul { cond, .. }
            | MulLong { cond, .. }
            | Swp { cond, .. }
            | Bx { cond, .. }
            | Mrs { cond, .. }
            | MsrReg { cond, .. }
            | MsrImm { cond, .. }
            | Mem { cond, .. }
            | MemHalf { cond, .. }
            | Block { cond, .. }
            | Branch { cond, .. }
            | Swi { cond, .. } => cond,
            Undefined(_) => Cond::Al,
        }
    }
}

pub fn decode(op: u32) -> Arm {
    let cond = Cond::from_bits(op >> 28);
    let reg = |shift: u32| ((op >> shift) & 15) as u8;
    if op & 0x0FFF_FFF0 == 0x012F_FF10 {
        return Arm::Bx { cond, rm: reg(0) };
    }
    if op & 0x0FC0_00F0 == 0x0000_0090 {
        return Arm::Mul {
            cond,
            acc: op & (1 << 21) != 0,
            s: op & (1 << 20) != 0,
            rd: reg(16),
            rn: reg(12),
            rs: reg(8),
            rm: reg(0),
        };
    }
    if op & 0x0F80_00F0 == 0x0080_0090 {
        return Arm::MulLong {
            cond,
            signed: op & (1 << 22) != 0,
            acc: op & (1 << 21) != 0,
            s: op & (1 << 20) != 0,
            rdhi: reg(16),
            rdlo: reg(12),
            rs: reg(8),
            rm: reg(0),
        };
    }
    if op & 0x0FB0_0FF0 == 0x0100_0090 {
        return Arm::Swp { cond, byte: op & (1 << 22) != 0, rd: reg(12), rm: reg(0), rn: reg(16) };
    }
    if op & 0x0E00_0090 == 0x0000_0090 && (op >> 5) & 3 != 0 {
        let kind = match (op >> 5) & 3 {
            1 => HalfKind::H,
            2 => HalfKind::Sb,
            _ => HalfKind::Sh,
        };
        let offset = if op & (1 << 22) != 0 {
            HalfOffset::Imm((((op >> 4) & 0xf0) | (op & 0xf)) as u8)
        } else {
            HalfOffset::Reg(reg(0))
        };
        return Arm::MemHalf {
            cond,
            load: op & (1 << 20) != 0,
            kind,
            pre: op & (1 << 24) != 0,
            up: op & (1 << 23) != 0,
            wb: op & (1 << 21) != 0,
            rn: reg(16),
            rd: reg(12),
            offset,
        };
    }
    if op & 0x0FBF_0FFF == 0x010F_0000 {
        return Arm::Mrs { cond, spsr: op & (1 << 22) != 0, rd: reg(12) };
    }
    if op & 0x0FB0_FFF0 == 0x0120_F000 {
        return Arm::MsrReg { cond, spsr: op & (1 << 22) != 0, fields: ((op >> 16) & 15) as u8, rm: reg(0) };
    }
    if op & 0x0FB0_F000 == 0x0320_F000 {
        let rot = ((op >> 8) & 15) * 2;
        return Arm::MsrImm {
            cond,
            spsr: op & (1 << 22) != 0,
            fields: ((op >> 16) & 15) as u8,
            value: (op & 0xff).rotate_right(rot),
        };
    }
    if op & 0x0C00_0000 == 0 {
        let op2 = if op & (1 << 25) != 0 {
            let rot = ((op >> 8) & 15) * 2;
            Operand2::Imm { value: (op & 0xff).rotate_right(rot), rot: rot as u8 }
        } else if op & (1 << 4) != 0 {
            Operand2::ShiftReg { rm: reg(0), shift: Shift::from_bits(op >> 5), rs: reg(8) }
        } else {
            Operand2::ShiftImm { rm: reg(0), shift: Shift::from_bits(op >> 5), amount: ((op >> 7) & 31) as u8 }
        };
        let dop = DpOp::from_bits(op >> 21);
        let s = op & (1 << 20) != 0;
        if dop.is_test() && !s {
            return Arm::Undefined(op);
        }
        return Arm::DataProc { cond, op: dop, s, rn: reg(16), rd: reg(12), op2 };
    }
    if op & 0x0E00_0010 == 0x0600_0010 {
        return Arm::Undefined(op);
    }
    if op & 0x0C00_0000 == 0x0400_0000 {
        let offset = if op & (1 << 25) != 0 {
            MemOffset::Reg { rm: reg(0), shift: Shift::from_bits(op >> 5), amount: ((op >> 7) & 31) as u8 }
        } else {
            MemOffset::Imm(op & 0xfff)
        };
        return Arm::Mem {
            cond,
            load: op & (1 << 20) != 0,
            byte: op & (1 << 22) != 0,
            pre: op & (1 << 24) != 0,
            up: op & (1 << 23) != 0,
            wb: op & (1 << 21) != 0,
            rn: reg(16),
            rd: reg(12),
            offset,
        };
    }
    if op & 0x0E00_0000 == 0x0800_0000 {
        return Arm::Block {
            cond,
            load: op & (1 << 20) != 0,
            pre: op & (1 << 24) != 0,
            up: op & (1 << 23) != 0,
            s: op & (1 << 22) != 0,
            wb: op & (1 << 21) != 0,
            rn: reg(16),
            rlist: op as u16,
        };
    }
    if op & 0x0E00_0000 == 0x0A00_0000 {
        let offset = (((op & 0x00FF_FFFF) << 8) as i32) >> 6;
        return Arm::Branch { cond, link: op & (1 << 24) != 0, offset };
    }
    if op & 0x0F00_0000 == 0x0F00_0000 {
        return Arm::Swi { cond, imm: op & 0x00FF_FFFF };
    }
    Arm::Undefined(op)
}

pub struct Disasm {
    pub addr: u32,
    pub instr: Arm,
}

fn fmt_shift(shift: Shift, amount: u8) -> String {
    match (shift, amount) {
        (Shift::Lsl, 0) => String::new(),
        (Shift::Ror, 0) => ", rrx".into(),
        (Shift::Lsr | Shift::Asr, 0) => format!(", {} #32", shift.mnemonic()),
        _ => format!(", {} #{}", shift.mnemonic(), amount),
    }
}

impl fmt::Display for Disasm {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Arm::*;
        let r = reg_name;
        match self.instr {
            DataProc { cond, op, s, rn, rd, op2 } => {
                let o2 = match op2 {
                    Operand2::Imm { value, .. } => format!("#{:#x}", value),
                    Operand2::ShiftImm { rm, shift, amount } => format!("{}{}", r(rm), fmt_shift(shift, amount)),
                    Operand2::ShiftReg { rm, shift, rs } => format!("{}, {} {}", r(rm), shift.mnemonic(), r(rs)),
                };
                let sfx = if s && !op.is_test() { "s" } else { "" };
                if op.is_test() {
                    write!(f, "{}{} {}, {}", op.mnemonic(), cond.suffix(), r(rn), o2)
                } else if !op.uses_rn() {
                    write!(f, "{}{}{} {}, {}", op.mnemonic(), cond.suffix(), sfx, r(rd), o2)
                } else {
                    write!(f, "{}{}{} {}, {}, {}", op.mnemonic(), cond.suffix(), sfx, r(rd), r(rn), o2)
                }
            }
            Mul { cond, acc, s, rd, rn, rs, rm } => {
                if acc {
                    write!(f, "mla{}{} {}, {}, {}, {}", cond.suffix(), if s { "s" } else { "" }, r(rd), r(rm), r(rs), r(rn))
                } else {
                    write!(f, "mul{}{} {}, {}, {}", cond.suffix(), if s { "s" } else { "" }, r(rd), r(rm), r(rs))
                }
            }
            MulLong { cond, signed, acc, s, rdhi, rdlo, rs, rm } => write!(
                f,
                "{}{}{}{} {}, {}, {}, {}",
                if signed { "s" } else { "u" },
                if acc { "mlal" } else { "mull" },
                cond.suffix(),
                if s { "s" } else { "" },
                r(rdlo),
                r(rdhi),
                r(rm),
                r(rs)
            ),
            Swp { cond, byte, rd, rm, rn } => {
                write!(f, "swp{}{} {}, {}, [{}]", cond.suffix(), if byte { "b" } else { "" }, r(rd), r(rm), r(rn))
            }
            Bx { cond, rm } => write!(f, "bx{} {}", cond.suffix(), r(rm)),
            Mrs { cond, spsr, rd } => write!(f, "mrs{} {}, {}", cond.suffix(), r(rd), if spsr { "spsr" } else { "cpsr" }),
            MsrReg { cond, spsr, fields, rm } => {
                write!(f, "msr{} {}_{:x}, {}", cond.suffix(), if spsr { "spsr" } else { "cpsr" }, fields, r(rm))
            }
            MsrImm { cond, spsr, fields, value } => {
                write!(f, "msr{} {}_{:x}, #{:#x}", cond.suffix(), if spsr { "spsr" } else { "cpsr" }, fields, value)
            }
            Mem { cond, load, byte, pre, up, wb, rn, rd, offset } => {
                let sign = if up { "" } else { "-" };
                let off = match offset {
                    MemOffset::Imm(0) => String::new(),
                    MemOffset::Imm(i) => format!(", #{}{:#x}", sign, i),
                    MemOffset::Reg { rm, shift, amount } => format!(", {}{}{}", sign, r(rm), fmt_shift(shift, amount)),
                };
                let addr = if pre {
                    format!("[{}{}]{}", r(rn), off, if wb { "!" } else { "" })
                } else {
                    format!("[{}]{}", r(rn), off)
                };
                write!(
                    f,
                    "{}{}{}{} {}, {}",
                    if load { "ldr" } else { "str" },
                    cond.suffix(),
                    if byte { "b" } else { "" },
                    if !pre && wb { "t" } else { "" },
                    r(rd),
                    addr
                )
            }
            MemHalf { cond, load, kind, pre, up, wb, rn, rd, offset } => {
                let sign = if up { "" } else { "-" };
                let off = match offset {
                    HalfOffset::Imm(0) => String::new(),
                    HalfOffset::Imm(i) => format!(", #{}{:#x}", sign, i),
                    HalfOffset::Reg(rm) => format!(", {}{}", sign, r(rm)),
                };
                let addr = if pre {
                    format!("[{}{}]{}", r(rn), off, if wb { "!" } else { "" })
                } else {
                    format!("[{}]{}", r(rn), off)
                };
                let k = match kind {
                    HalfKind::H => "h",
                    HalfKind::Sb => "sb",
                    HalfKind::Sh => "sh",
                };
                write!(f, "{}{}{} {}, {}", if load { "ldr" } else { "str" }, cond.suffix(), k, r(rd), addr)
            }
            Block { cond, load, pre, up, s, wb, rn, rlist } => {
                let mode = match (pre, up) {
                    (false, true) => "ia",
                    (true, true) => "ib",
                    (false, false) => "da",
                    (true, false) => "db",
                };
                write!(
                    f,
                    "{}{}{} {}{}, {}{}",
                    if load { "ldm" } else { "stm" },
                    cond.suffix(),
                    mode,
                    r(rn),
                    if wb { "!" } else { "" },
                    reg_list(rlist),
                    if s { "^" } else { "" }
                )
            }
            Branch { cond, link, offset } => write!(
                f,
                "b{}{} {:#010x}",
                if link { "l" } else { "" },
                cond.suffix(),
                self.addr.wrapping_add(8).wrapping_add(offset as u32)
            ),
            Swi { cond, imm } => write!(f, "swi{} {:#x}", cond.suffix(), imm),
            Undefined(op) => write!(f, ".word {:#010x}", op),
        }
    }
}
