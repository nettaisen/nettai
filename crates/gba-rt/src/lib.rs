//! Runtime for recompiled GBA code.
//!
//! Translated functions are `fn(&mut Cpu)`. The [`Cpu`] holds the ARM register
//! file and flags, the GBA memory map, and high-level emulation of the
//! hardware the game touches (IO registers, DMA, BIOS calls). There is no
//! instruction fetch and no cycle timing: code runs as native Rust, and the
//! host drives frames and interrupts explicitly.

mod bios;
mod io;
mod mem;

pub use io::{Dma, DmaTiming};
pub use mem::Memory;

pub type Func = fn(&mut Cpu);

/// Return address handed to top-level calls made by the host. Translated code
/// never branches here, so a function that returns to it returned normally.
pub const HOST_RETURN: u32 = 0xFFFF_FFF0;

/// Processor modes (CPSR bits 0-4) that the runtime banks registers for.
pub const MODE_USER: u32 = 0x10;
pub const MODE_IRQ: u32 = 0x12;
pub const MODE_SUPERVISOR: u32 = 0x13;
pub const MODE_SYSTEM: u32 = 0x1F;

/// Callbacks from the runtime to the host for behavior that isn't
/// self-contained: waiting for the next frame, and hardware the host models.
pub trait Host {
    /// The game is blocked until the next interrupt (BIOS Halt, IntrWait,
    /// VBlankIntrWait). The host should advance time; by default this panics
    /// because a function-level harness has no notion of time.
    fn wait_for_interrupt(&mut self, cpu: &mut Cpu, swi: u32) {
        let _ = cpu;
        panic!("game waited for an interrupt (swi {swi:#x}) with no host frame loop");
    }

    /// A read of an IO register the runtime doesn't model. Returns the value
    /// to read; the default returns the last written value.
    fn io_read16(&mut self, cpu: &mut Cpu, addr: u32) -> Option<u16> {
        let _ = (cpu, addr);
        None
    }

    /// A write to an IO register, after the runtime has stored it.
    fn io_write16(&mut self, cpu: &mut Cpu, addr: u32, value: u16) {
        let _ = (cpu, addr, value);
    }
}

struct NoHost;
impl Host for NoHost {}

pub struct Cpu {
    /// r0-r15. r13 = sp, r14 = lr. r15 is not maintained: translated code
    /// knows every instruction's address statically.
    pub r: [u32; 16],
    pub n: bool,
    pub z: bool,
    pub c: bool,
    pub v: bool,
    /// Expected return address, handed from caller to callee on each call.
    pub ret: u32,
    /// Where the most recently finished function transferred control.
    pub pc: u32,
    /// Control bits of the CPSR (mode, I, F, T).
    pub cpsr_ctl: u32,
    pub spsr: u32,
    /// Banked r13/r14/SPSR for IRQ and supervisor modes, and the shared
    /// user/system bank while another mode is active.
    bank_usr: [u32; 2],
    bank_irq: [u32; 3],
    bank_svc: [u32; 3],
    pub mem: Box<Memory>,
    pub io: io::IoState,
    /// Translated-code lookup for indirect branches.
    pub lookup: fn(u32) -> Option<Func>,
    /// Host-installed replacements, consulted by indirect calls.
    pub overrides: std::collections::HashMap<u32, Func>,
    host: Option<Box<dyn Host>>,
    /// Count of returns that skipped frames (longjmp-style control flow).
    pub unwinds: u64,
}

impl Cpu {
    pub fn new(rom: std::sync::Arc<[u8]>, lookup: fn(u32) -> Option<Func>) -> Cpu {
        Cpu {
            r: [0; 16],
            n: false,
            z: false,
            c: false,
            v: false,
            ret: 0,
            pc: 0,
            cpsr_ctl: MODE_SYSTEM,
            spsr: 0,
            bank_usr: [0x0300_7F00, 0],
            bank_irq: [0x0300_7FA0, 0, 0],
            bank_svc: [0x0300_7FE0, 0, 0],
            mem: Box::new(Memory::new(rom)),
            io: io::IoState::default(),
            lookup,
            overrides: Default::default(),
            host: Some(Box::new(NoHost)),
            unwinds: 0,
        }
    }

    pub fn set_host(&mut self, host: Box<dyn Host>) {
        self.host = Some(host);
    }

    pub(crate) fn with_host<R>(&mut self, f: impl FnOnce(&mut dyn Host, &mut Cpu) -> R) -> R {
        let mut host = self.host.take().expect("host re-entered");
        let r = f(host.as_mut(), self);
        self.host = Some(host);
        r
    }

    // ---- Calls ----------------------------------------------------------

    /// The function entered at `addr`: a host override, else translated code.
    pub fn find(&self, addr: u32) -> Option<Func> {
        if let Some(f) = self.overrides.get(&addr) {
            return Some(*f);
        }
        (self.lookup)(addr)
    }

    /// Call the function at `addr` from the host, as `bl` would. Registers
    /// are left as the callee leaves them.
    pub fn call(&mut self, addr: u32) {
        let f = self.find(addr & !1).unwrap_or_else(|| panic!("no function at {addr:#010x}"));
        self.r[14] = HOST_RETURN;
        self.ret = HOST_RETURN;
        f(self);
        assert_eq!(self.pc, HOST_RETURN, "function {addr:#010x} returned to {:#010x}", self.pc);
    }

    /// Indirect call from translated code (`mov lr, pc; bx rN`).
    pub fn call_indirect(&mut self, target: u32) {
        match self.find(target & !1) {
            Some(f) => f(self),
            None => panic!("indirect call to {target:#010x}, which is not a known function"),
        }
    }

    /// A branch to an address that isn't a block of the current function.
    #[cold]
    #[inline(never)]
    pub fn fallback(&mut self, pc: u32, ret: u32) {
        if pc == ret {
            self.pc = pc;
            return;
        }
        if let Some(f) = self.find(pc) {
            // A tail call: the callee returns wherever we would have.
            self.ret = ret;
            f(self);
            return;
        }
        // A return to some frame further up the stack; each caller checks.
        self.unwinds += 1;
        self.pc = pc;
    }

    #[cold]
    pub fn undefined(&mut self, pc: u32) -> ! {
        panic!("executed undefined or untranslated code at {pc:#010x}")
    }

    #[cold]
    pub fn undefined_behavior(&mut self, what: &str) {
        panic!("unsupported instruction behavior: {what}")
    }

    // ---- Flags ----------------------------------------------------------

    #[inline(always)]
    pub fn nz(&mut self, v: u32) -> u32 {
        self.n = (v as i32) < 0;
        self.z = v == 0;
        v
    }

    #[inline(always)]
    pub fn adds(&mut self, a: u32, b: u32) -> u32 {
        let (r, carry) = a.overflowing_add(b);
        self.c = carry;
        self.v = ((a ^ r) & (b ^ r)) >> 31 != 0;
        self.nz(r)
    }

    #[inline(always)]
    pub fn subs(&mut self, a: u32, b: u32) -> u32 {
        let r = a.wrapping_sub(b);
        self.c = a >= b;
        self.v = ((a ^ b) & (a ^ r)) >> 31 != 0;
        self.nz(r)
    }

    #[inline(always)]
    pub fn adcs(&mut self, a: u32, b: u32) -> u32 {
        let wide = a as u64 + b as u64 + self.c as u64;
        let r = wide as u32;
        self.c = wide >> 32 != 0;
        self.v = ((a ^ r) & (b ^ r)) >> 31 != 0;
        self.nz(r)
    }

    #[inline(always)]
    pub fn sbcs(&mut self, a: u32, b: u32) -> u32 {
        let borrow = (!self.c) as u64;
        let r = a.wrapping_sub(b).wrapping_sub(borrow as u32);
        self.c = a as u64 >= b as u64 + borrow;
        self.v = ((a ^ b) & (a ^ r)) >> 31 != 0;
        self.nz(r)
    }

    #[inline(always)]
    pub fn adc(&self, a: u32, b: u32) -> u32 {
        a.wrapping_add(b).wrapping_add(self.c as u32)
    }

    #[inline(always)]
    pub fn sbc(&self, a: u32, b: u32) -> u32 {
        a.wrapping_sub(b).wrapping_sub((!self.c) as u32)
    }

    // Shifts that set C (the shifter carry-out). Immediate forms take the
    // decoded amount: lsl 1-31, lsr/asr 1-32, ror 1-31.

    #[inline(always)]
    pub fn lsl_imm_c(&mut self, v: u32, n: u32) -> u32 {
        self.c = (v >> (32 - n)) & 1 != 0;
        v << n
    }

    #[inline(always)]
    pub fn lsr_imm_c(&mut self, v: u32, n: u32) -> u32 {
        if n >= 32 {
            self.c = v >> 31 != 0;
            0
        } else {
            self.c = (v >> (n - 1)) & 1 != 0;
            v >> n
        }
    }

    #[inline(always)]
    pub fn asr_imm_c(&mut self, v: u32, n: u32) -> u32 {
        if n >= 32 {
            self.c = v >> 31 != 0;
            ((v as i32) >> 31) as u32
        } else {
            self.c = (v >> (n - 1)) & 1 != 0;
            ((v as i32) >> n) as u32
        }
    }

    #[inline(always)]
    pub fn ror_imm_c(&mut self, v: u32, n: u32) -> u32 {
        let r = v.rotate_right(n);
        self.c = r >> 31 != 0;
        r
    }

    #[inline(always)]
    pub fn rrx_c(&mut self, v: u32) -> u32 {
        let r = (v >> 1) | ((self.c as u32) << 31);
        self.c = v & 1 != 0;
        r
    }

    #[inline(always)]
    pub fn lsl_reg_c(&mut self, v: u32, s: u32) -> u32 {
        let n = s & 0xff;
        if n == 0 {
            v
        } else if n < 32 {
            self.c = (v >> (32 - n)) & 1 != 0;
            v << n
        } else {
            self.c = n == 32 && v & 1 != 0;
            0
        }
    }

    #[inline(always)]
    pub fn lsr_reg_c(&mut self, v: u32, s: u32) -> u32 {
        let n = s & 0xff;
        if n == 0 {
            v
        } else if n < 32 {
            self.c = (v >> (n - 1)) & 1 != 0;
            v >> n
        } else {
            self.c = n == 32 && v >> 31 != 0;
            0
        }
    }

    #[inline(always)]
    pub fn asr_reg_c(&mut self, v: u32, s: u32) -> u32 {
        let n = s & 0xff;
        if n == 0 {
            v
        } else if n < 32 {
            self.c = (v >> (n - 1)) & 1 != 0;
            ((v as i32) >> n) as u32
        } else {
            self.c = v >> 31 != 0;
            ((v as i32) >> 31) as u32
        }
    }

    #[inline(always)]
    pub fn ror_reg_c(&mut self, v: u32, s: u32) -> u32 {
        let n = s & 0xff;
        if n == 0 {
            return v;
        }
        let m = n & 31;
        let r = if m == 0 { v } else { v.rotate_right(m) };
        self.c = r >> 31 != 0;
        r
    }

    // ---- Program status ---------------------------------------------------

    pub fn cpsr(&self) -> u32 {
        ((self.n as u32) << 31) | ((self.z as u32) << 30) | ((self.c as u32) << 29) | ((self.v as u32) << 28) | self.cpsr_ctl
    }

    pub fn set_cpsr(&mut self, value: u32, fields: u32) {
        if fields & 8 != 0 {
            self.n = value & (1 << 31) != 0;
            self.z = value & (1 << 30) != 0;
            self.c = value & (1 << 29) != 0;
            self.v = value & (1 << 28) != 0;
        }
        if fields & 1 != 0 {
            let ctl = (self.cpsr_ctl & !0xFF) | (value & 0xFF);
            self.switch_mode(ctl & 0x1F);
            self.cpsr_ctl = ctl;
        }
    }

    pub fn spsr(&self) -> u32 {
        self.spsr
    }

    pub fn set_spsr(&mut self, value: u32, fields: u32) {
        let mut mask = 0;
        for i in 0..4 {
            if fields & (1 << i) != 0 {
                mask |= 0xFF << (8 * i);
            }
        }
        self.spsr = (self.spsr & !mask) | (value & mask);
    }

    /// Bank r13/r14/SPSR out of the current mode and into `mode`.
    pub fn switch_mode(&mut self, mode: u32) {
        let old = self.cpsr_ctl & 0x1F;
        if old == mode {
            return;
        }
        let (sp, lr, spsr) = (self.r[13], self.r[14], self.spsr);
        match old {
            MODE_IRQ => self.bank_irq = [sp, lr, spsr],
            MODE_SUPERVISOR => self.bank_svc = [sp, lr, spsr],
            _ => self.bank_usr = [sp, lr],
        }
        match mode {
            MODE_IRQ => [self.r[13], self.r[14], self.spsr] = self.bank_irq,
            MODE_SUPERVISOR => [self.r[13], self.r[14], self.spsr] = self.bank_svc,
            _ => [self.r[13], self.r[14]] = self.bank_usr,
        }
        self.cpsr_ctl = (self.cpsr_ctl & !0x1F) | mode;
    }

    /// `movs pc, ...` / `subs pc, ...`: return from an exception.
    pub fn exception_return(&mut self, target: u32) {
        let spsr = self.spsr;
        self.set_cpsr(spsr, 0xF);
        self.pc = target & !1;
        panic!("exception return to {target:#010x} inside translated code");
    }

    /// Take an IRQ as the BIOS does: switch to IRQ mode, save the scratch
    /// registers on the IRQ stack, and call the handler installed at
    /// 0x03007FFC. `interrupted` is the address the IRQ interrupted.
    pub fn irq(&mut self, interrupted: u32) {
        if self.cpsr_ctl & 0x80 != 0 {
            return;
        }
        let cpsr = self.cpsr();
        self.switch_mode(MODE_IRQ);
        self.spsr = cpsr;
        self.r[14] = interrupted.wrapping_add(4);
        self.cpsr_ctl = (self.cpsr_ctl & !0x20) | 0x80;
        // BIOS: stmfd sp!, {r0-r3, r12, lr}
        let saved = [self.r[0], self.r[1], self.r[2], self.r[3], self.r[12], self.r[14]];
        let sp = self.r[13].wrapping_sub(24);
        for (i, v) in saved.iter().enumerate() {
            self.st32(sp + 4 * i as u32, *v);
        }
        self.r[13] = sp;
        self.r[0] = 0x0400_0000;
        let handler = self.ld32(0x0300_7FFC);
        const BIOS_IRQ_RETURN: u32 = 0x0000_0138;
        self.r[14] = BIOS_IRQ_RETURN;
        self.ret = BIOS_IRQ_RETURN;
        let f = self.find(handler & !1).unwrap_or_else(|| panic!("no IRQ handler at {handler:#010x}"));
        f(self);
        assert_eq!(self.pc, BIOS_IRQ_RETURN, "IRQ handler returned to {:#010x}", self.pc);
        // BIOS: ldmfd sp!, {r0-r3, r12, lr}; subs pc, lr, #4
        let sp = self.r[13];
        let mut regs = [0u32; 6];
        for (i, v) in regs.iter_mut().enumerate() {
            *v = self.ld32(sp + 4 * i as u32);
        }
        self.r[13] = sp + 24;
        [self.r[0], self.r[1], self.r[2], self.r[3], self.r[12], self.r[14]] = regs;
        let spsr = self.spsr;
        self.set_cpsr(spsr, 0xF);
    }
}

// Shifts without flags, for operands whose carry-out is unused.

#[inline(always)]
pub fn lsl_imm(v: u32, n: u32) -> u32 {
    v << n
}

#[inline(always)]
pub fn lsr_imm(v: u32, n: u32) -> u32 {
    if n >= 32 { 0 } else { v >> n }
}

#[inline(always)]
pub fn asr_imm(v: u32, n: u32) -> u32 {
    ((v as i32) >> n.min(31)) as u32
}

#[inline(always)]
pub fn ror_imm(v: u32, n: u32) -> u32 {
    v.rotate_right(n)
}

#[inline(always)]
pub fn rrx(v: u32, c: bool) -> u32 {
    (v >> 1) | ((c as u32) << 31)
}

#[inline(always)]
pub fn lsl_reg(v: u32, s: u32) -> u32 {
    let n = s & 0xff;
    if n < 32 { v << n } else { 0 }
}

#[inline(always)]
pub fn lsr_reg(v: u32, s: u32) -> u32 {
    let n = s & 0xff;
    if n < 32 { v >> n } else { 0 }
}

#[inline(always)]
pub fn asr_reg(v: u32, s: u32) -> u32 {
    let n = (s & 0xff).min(31);
    ((v as i32) >> n) as u32
}

#[inline(always)]
pub fn ror_reg(v: u32, s: u32) -> u32 {
    v.rotate_right(s & 31)
}
