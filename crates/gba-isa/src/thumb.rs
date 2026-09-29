//! Thumb (16-bit) instruction decoding.

use crate::{Cond, Shift, reg_list, reg_name};
use std::fmt;

/// Format 4 ALU operations, in encoding order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AluOp {
    And,
    Eor,
    Lsl,
    Lsr,
    Asr,
    Adc,
    Sbc,
    Ror,
    Tst,
    Neg,
    Cmp,
    Cmn,
    Orr,
    Mul,
    Bic,
    Mvn,
}

impl AluOp {
    fn from_bits(b: u16) -> AluOp {
        use AluOp::*;
        [And, Eor, Lsl, Lsr, Asr, Adc, Sbc, Ror, Tst, Neg, Cmp, Cmn, Orr, Mul, Bic, Mvn][(b & 15) as usize]
    }

    pub fn mnemonic(self) -> &'static str {
        use AluOp::*;
        match self {
            And => "and",
            Eor => "eor",
            Lsl => "lsl",
            Lsr => "lsr",
            Asr => "asr",
            Adc => "adc",
            Sbc => "sbc",
            Ror => "ror",
            Tst => "tst",
            Neg => "neg",
            Cmp => "cmp",
            Cmn => "cmn",
            Orr => "orr",
            Mul => "mul",
            Bic => "bic",
            Mvn => "mvn",
        }
    }
}

/// Load/store width and signedness.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemOp {
    Str,
    Strb,
    Strh,
    Ldr,
    Ldrb,
    Ldrh,
    Ldsb,
    Ldsh,
}

impl MemOp {
    pub fn is_load(self) -> bool {
        matches!(self, MemOp::Ldr | MemOp::Ldrb | MemOp::Ldrh | MemOp::Ldsb | MemOp::Ldsh)
    }

    pub fn mnemonic(self) -> &'static str {
        match self {
            MemOp::Str => "str",
            MemOp::Strb => "strb",
            MemOp::Strh => "strh",
            MemOp::Ldr => "ldr",
            MemOp::Ldrb => "ldrb",
            MemOp::Ldrh => "ldrh",
            MemOp::Ldsb => "ldsb",
            MemOp::Ldsh => "ldsh",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Thumb {
    /// Format 1: lsl/lsr/asr rd, rs, #imm. `imm` is the raw 5-bit field
    /// (0 means 32 for lsr/asr).
    ShiftImm { op: Shift, rd: u8, rs: u8, imm: u8 },
    /// Format 2: add/sub rd, rs, rn.
    AddSubReg { sub: bool, rd: u8, rs: u8, rn: u8 },
    /// Format 2: add/sub rd, rs, #imm3.
    AddSubImm3 { sub: bool, rd: u8, rs: u8, imm: u8 },
    /// Format 3.
    MovImm { rd: u8, imm: u8 },
    CmpImm { rd: u8, imm: u8 },
    AddImm { rd: u8, imm: u8 },
    SubImm { rd: u8, imm: u8 },
    /// Format 4.
    Alu { op: AluOp, rd: u8, rs: u8 },
    /// Format 5 (registers are full 0-15 numbers).
    HiAdd { rd: u8, rs: u8 },
    HiCmp { rd: u8, rs: u8 },
    HiMov { rd: u8, rs: u8 },
    Bx { rs: u8 },
    /// Format 6: ldr rd, [pc, #imm] (imm in bytes).
    LdrPc { rd: u8, imm: u16 },
    /// Formats 7/8: register offset.
    MemReg { op: MemOp, rd: u8, rb: u8, ro: u8 },
    /// Formats 9/10: immediate offset (imm in bytes).
    MemImm { op: MemOp, rd: u8, rb: u8, imm: u8 },
    /// Format 11: sp-relative (imm in bytes).
    MemSp { load: bool, rd: u8, imm: u16 },
    /// Format 12: add rd, pc/sp, #imm (imm in bytes).
    AddPc { rd: u8, imm: u16 },
    AddSp { rd: u8, imm: u16 },
    /// Format 13: add sp, #imm (signed, bytes).
    AdjustSp { imm: i16 },
    /// Format 14.
    Push { rlist: u8, lr: bool },
    Pop { rlist: u8, pc: bool },
    /// Format 15.
    Stmia { rb: u8, rlist: u8 },
    Ldmia { rb: u8, rlist: u8 },
    /// Format 16 (offset relative to instruction address + 4).
    BCond { cond: Cond, offset: i32 },
    /// Format 17.
    Swi { imm: u8 },
    /// Format 18 (offset relative to instruction address + 4).
    B { offset: i32 },
    /// Format 19, first half: lr = pc + 4 + offset.
    BlHi { offset: i32 },
    /// Format 19, second half: pc = lr + offset; lr = next | 1.
    BlLo { offset: u16 },
    Undefined(u16),
}

fn sext(v: u32, bits: u32) -> i32 {
    ((v << (32 - bits)) as i32) >> (32 - bits)
}

pub fn decode(op: u16) -> Thumb {
    let r = |shift: u16| ((op >> shift) & 7) as u8;
    match op >> 13 {
        0 => {
            let sub = (op >> 11) & 3;
            if sub == 3 {
                let imm_form = op & (1 << 10) != 0;
                let is_sub = op & (1 << 9) != 0;
                if imm_form {
                    Thumb::AddSubImm3 { sub: is_sub, rd: r(0), rs: r(3), imm: r(6) }
                } else {
                    Thumb::AddSubReg { sub: is_sub, rd: r(0), rs: r(3), rn: r(6) }
                }
            } else {
                Thumb::ShiftImm {
                    op: Shift::from_bits(sub as u32),
                    rd: r(0),
                    rs: r(3),
                    imm: ((op >> 6) & 31) as u8,
                }
            }
        }
        1 => {
            let rd = r(8);
            let imm = op as u8;
            match (op >> 11) & 3 {
                0 => Thumb::MovImm { rd, imm },
                1 => Thumb::CmpImm { rd, imm },
                2 => Thumb::AddImm { rd, imm },
                _ => Thumb::SubImm { rd, imm },
            }
        }
        2 => {
            if op >> 10 == 0b010000 {
                Thumb::Alu { op: AluOp::from_bits(op >> 6), rd: r(0), rs: r(3) }
            } else if op >> 10 == 0b010001 {
                let rd = (((op >> 7) & 1) << 3) as u8 | r(0);
                let rs = (((op >> 6) & 1) << 3) as u8 | r(3);
                match (op >> 8) & 3 {
                    0 => Thumb::HiAdd { rd, rs },
                    1 => Thumb::HiCmp { rd, rs },
                    2 => Thumb::HiMov { rd, rs },
                    _ => {
                        if op & (1 << 7) != 0 {
                            Thumb::Undefined(op) // blx (ARMv5)
                        } else {
                            Thumb::Bx { rs }
                        }
                    }
                }
            } else if op >> 11 == 0b01001 {
                Thumb::LdrPc { rd: r(8), imm: (op & 0xff) * 4 }
            } else {
                // 0101 xxx: register offset.
                let (rd, rb, ro) = (r(0), r(3), r(6));
                let bits = (op >> 9) & 7;
                let mop = match bits {
                    0b000 => MemOp::Str,
                    0b001 => MemOp::Strh,
                    0b010 => MemOp::Strb,
                    0b011 => MemOp::Ldsb,
                    0b100 => MemOp::Ldr,
                    0b101 => MemOp::Ldrh,
                    0b110 => MemOp::Ldrb,
                    _ => MemOp::Ldsh,
                };
                Thumb::MemReg { op: mop, rd, rb, ro }
            }
        }
        3 => {
            let byte = op & (1 << 12) != 0;
            let load = op & (1 << 11) != 0;
            let off = ((op >> 6) & 31) as u8;
            let mop = match (byte, load) {
                (false, false) => MemOp::Str,
                (false, true) => MemOp::Ldr,
                (true, false) => MemOp::Strb,
                (true, true) => MemOp::Ldrb,
            };
            let imm = if byte { off } else { off * 4 };
            Thumb::MemImm { op: mop, rd: r(0), rb: r(3), imm }
        }
        4 => {
            if op & (1 << 12) == 0 {
                let load = op & (1 << 11) != 0;
                let off = ((op >> 6) & 31) as u8;
                Thumb::MemImm {
                    op: if load { MemOp::Ldrh } else { MemOp::Strh },
                    rd: r(0),
                    rb: r(3),
                    imm: off * 2,
                }
            } else {
                Thumb::MemSp { load: op & (1 << 11) != 0, rd: r(8), imm: (op & 0xff) * 4 }
            }
        }
        5 => {
            if op & (1 << 12) == 0 {
                let imm = (op & 0xff) * 4;
                if op & (1 << 11) != 0 {
                    Thumb::AddSp { rd: r(8), imm }
                } else {
                    Thumb::AddPc { rd: r(8), imm }
                }
            } else if (op >> 8) & 0xf == 0 {
                let imm = ((op & 0x7f) * 4) as i16;
                Thumb::AdjustSp { imm: if op & 0x80 != 0 { -imm } else { imm } }
            } else if (op >> 9) & 3 == 2 {
                let load = op & (1 << 11) != 0;
                let extra = op & (1 << 8) != 0;
                let rlist = op as u8;
                if load {
                    Thumb::Pop { rlist, pc: extra }
                } else {
                    Thumb::Push { rlist, lr: extra }
                }
            } else {
                Thumb::Undefined(op)
            }
        }
        6 => {
            if op & (1 << 12) == 0 {
                let rb = r(8);
                let rlist = op as u8;
                if op & (1 << 11) != 0 {
                    Thumb::Ldmia { rb, rlist }
                } else {
                    Thumb::Stmia { rb, rlist }
                }
            } else {
                let cond = (op >> 8) & 15;
                match cond {
                    15 => Thumb::Swi { imm: op as u8 },
                    14 => Thumb::Undefined(op),
                    _ => Thumb::BCond {
                        cond: Cond::from_bits(cond as u32),
                        offset: sext((op & 0xff) as u32, 8) * 2,
                    },
                }
            }
        }
        _ => match (op >> 11) & 3 {
            0 => Thumb::B { offset: sext((op & 0x7ff) as u32, 11) * 2 },
            2 => Thumb::BlHi { offset: sext((op & 0x7ff) as u32, 11) << 12 },
            3 => Thumb::BlLo { offset: (op & 0x7ff) * 2 },
            _ => Thumb::Undefined(op), // blx suffix (ARMv5)
        },
    }
}

/// A Thumb instruction together with its address, for formatting.
pub struct Disasm {
    pub addr: u32,
    pub instr: Thumb,
    /// For a paired bl, the resolved target.
    pub bl_target: Option<u32>,
}

impl fmt::Display for Disasm {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Thumb::*;
        let r = reg_name;
        let pc4 = self.addr.wrapping_add(4);
        match self.instr {
            ShiftImm { op, rd, rs, imm } => write!(f, "{} {}, {}, #{:#x}", op.mnemonic(), r(rd), r(rs), imm),
            AddSubReg { sub, rd, rs, rn } => {
                write!(f, "{} {}, {}, {}", if sub { "sub" } else { "add" }, r(rd), r(rs), r(rn))
            }
            AddSubImm3 { sub, rd, rs, imm } => {
                write!(f, "{} {}, {}, #{}", if sub { "sub" } else { "add" }, r(rd), r(rs), imm)
            }
            MovImm { rd, imm } => write!(f, "mov {}, #{:#x}", r(rd), imm),
            CmpImm { rd, imm } => write!(f, "cmp {}, #{:#x}", r(rd), imm),
            AddImm { rd, imm } => write!(f, "add {}, #{:#x}", r(rd), imm),
            SubImm { rd, imm } => write!(f, "sub {}, #{:#x}", r(rd), imm),
            Alu { op, rd, rs } => write!(f, "{} {}, {}", op.mnemonic(), r(rd), r(rs)),
            HiAdd { rd, rs } => write!(f, "add {}, {}", r(rd), r(rs)),
            HiCmp { rd, rs } => write!(f, "cmp {}, {}", r(rd), r(rs)),
            HiMov { rd, rs } => write!(f, "mov {}, {}", r(rd), r(rs)),
            Bx { rs } => write!(f, "bx {}", r(rs)),
            LdrPc { rd, imm } => write!(f, "ldr {}, [pc, #{:#x}] // {:#010x}", r(rd), imm, (pc4 & !3) + imm as u32),
            MemReg { op, rd, rb, ro } => write!(f, "{} {}, [{}, {}]", op.mnemonic(), r(rd), r(rb), r(ro)),
            MemImm { op, rd, rb, imm } => write!(f, "{} {}, [{}, #{:#x}]", op.mnemonic(), r(rd), r(rb), imm),
            MemSp { load, rd, imm } => {
                write!(f, "{} {}, [sp, #{:#x}]", if load { "ldr" } else { "str" }, r(rd), imm)
            }
            AddPc { rd, imm } => write!(f, "add {}, pc, #{:#x}", r(rd), imm),
            AddSp { rd, imm } => write!(f, "add {}, sp, #{:#x}", r(rd), imm),
            AdjustSp { imm } => {
                if imm < 0 {
                    write!(f, "sub sp, #{:#x}", -imm)
                } else {
                    write!(f, "add sp, #{:#x}", imm)
                }
            }
            Push { rlist, lr } => write!(f, "push {}", reg_list(rlist as u16 | if lr { 1 << 14 } else { 0 })),
            Pop { rlist, pc } => write!(f, "pop {}", reg_list(rlist as u16 | if pc { 1 << 15 } else { 0 })),
            Stmia { rb, rlist } => write!(f, "stmia {}!, {}", r(rb), reg_list(rlist as u16)),
            Ldmia { rb, rlist } => write!(f, "ldmia {}!, {}", r(rb), reg_list(rlist as u16)),
            BCond { cond, offset } => write!(f, "b{} {:#010x}", cond.suffix(), pc4.wrapping_add(offset as u32)),
            Swi { imm } => write!(f, "swi {:#x}", imm),
            B { offset } => write!(f, "b {:#010x}", pc4.wrapping_add(offset as u32)),
            BlHi { .. } | BlLo { .. } => match self.bl_target {
                Some(t) => write!(f, "bl {:#010x}", t),
                None => write!(f, "bl.half {:?}", self.instr),
            },
            Undefined(op) => write!(f, ".hword {:#06x}", op),
        }
    }
}
