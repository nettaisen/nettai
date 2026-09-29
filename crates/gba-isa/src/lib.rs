//! ARMv4T (ARM7TDMI) instruction decoding for the GBA: the Thumb and ARM
//! instruction sets, with a disassembly formatter for generated-code comments.

pub mod arm;
pub mod thumb;

/// Shift type for shifted-register operands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shift {
    Lsl,
    Lsr,
    Asr,
    Ror,
}

impl Shift {
    pub fn from_bits(b: u32) -> Shift {
        match b & 3 {
            0 => Shift::Lsl,
            1 => Shift::Lsr,
            2 => Shift::Asr,
            _ => Shift::Ror,
        }
    }

    pub fn mnemonic(self) -> &'static str {
        match self {
            Shift::Lsl => "lsl",
            Shift::Lsr => "lsr",
            Shift::Asr => "asr",
            Shift::Ror => "ror",
        }
    }
}

/// Condition codes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cond {
    Eq,
    Ne,
    Cs,
    Cc,
    Mi,
    Pl,
    Vs,
    Vc,
    Hi,
    Ls,
    Ge,
    Lt,
    Gt,
    Le,
    Al,
    Nv,
}

impl Cond {
    pub fn from_bits(b: u32) -> Cond {
        use Cond::*;
        [Eq, Ne, Cs, Cc, Mi, Pl, Vs, Vc, Hi, Ls, Ge, Lt, Gt, Le, Al, Nv][(b & 15) as usize]
    }

    pub fn suffix(self) -> &'static str {
        use Cond::*;
        match self {
            Eq => "eq",
            Ne => "ne",
            Cs => "cs",
            Cc => "cc",
            Mi => "mi",
            Pl => "pl",
            Vs => "vs",
            Vc => "vc",
            Hi => "hi",
            Ls => "ls",
            Ge => "ge",
            Lt => "lt",
            Gt => "gt",
            Le => "le",
            Al => "",
            Nv => "nv",
        }
    }

    /// Which of N, Z, C, V this condition reads, as a bitmask (N=8 Z=4 C=2 V=1).
    pub fn flags_read(self) -> u8 {
        use Cond::*;
        match self {
            Eq | Ne => 4,
            Cs | Cc => 2,
            Mi | Pl => 8,
            Vs | Vc => 1,
            Hi | Ls => 4 | 2,
            Ge | Lt => 8 | 1,
            Gt | Le => 8 | 4 | 1,
            Al | Nv => 0,
        }
    }
}

pub const FLAG_N: u8 = 8;
pub const FLAG_Z: u8 = 4;
pub const FLAG_C: u8 = 2;
pub const FLAG_V: u8 = 1;
pub const FLAGS_ALL: u8 = 15;

pub fn reg_name(r: u8) -> &'static str {
    [
        "r0", "r1", "r2", "r3", "r4", "r5", "r6", "r7", "r8", "r9", "r10", "r11", "r12", "sp",
        "lr", "pc",
    ][r as usize & 15]
}

pub fn reg_list(list: u16) -> String {
    let mut out = Vec::new();
    let mut i = 0;
    while i < 16 {
        if list & (1 << i) != 0 {
            let start = i;
            while i + 1 < 16 && list & (1 << (i + 1)) != 0 && i + 1 < 13 {
                i += 1;
            }
            if i > start {
                out.push(format!("{}-{}", reg_name(start as u8), reg_name(i as u8)));
            } else {
                out.push(reg_name(start as u8).to_string());
            }
        }
        i += 1;
    }
    format!("{{{}}}", out.join(","))
}
