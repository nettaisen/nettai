//! BN6 (US Falzar) running natively.
//!
//! [`Gba`] is one console: the recompiled game code on a [`gba_rt::Cpu`],
//! driven one game frame at a time. The game has a single wait-for-frame
//! point (`main_awaitFrame` in the main loop), which is hooked to hand
//! control back to the host; each [`Gba::run_frame`] then raises the
//! frame's interrupts and runs one pass of the main loop.

use gba_rt::{Cpu, DmaTiming, Func, HOST_RETURN};
use std::sync::Arc;

pub mod addr {
    //! ROM addresses (US Falzar) the host relies on.
    pub const GAME_ENTRY_POINT: u32 = 0x0800_0000;
    /// `main_gameRoutine`, just after `bl main_awaitFrame`: one pass of the
    /// main loop starts here.
    pub const MAIN_LOOP_BODY: u32 = 0x0800_02D4;
    pub const MAIN_AWAIT_FRAME: u32 = 0x0800_03A0;
    pub const MAIN_POLL_LCD_STATUS: u32 = 0x0800_03D0;
    /// The per-frame link step (link library + packet distribution).
    pub const LINK_STEP: u32 = 0x0803_EAE4;
}

/// Returned in `Cpu::pc` when the game reaches its wait-for-frame point.
pub const FRAME_YIELD: u32 = 0xFFFF_FFE0;

/// GBA button bits (KEYINPUT order, 1 = pressed).
pub mod keys {
    pub const A: u16 = 1 << 0;
    pub const B: u16 = 1 << 1;
    pub const SELECT: u16 = 1 << 2;
    pub const START: u16 = 1 << 3;
    pub const RIGHT: u16 = 1 << 4;
    pub const LEFT: u16 = 1 << 5;
    pub const UP: u16 = 1 << 6;
    pub const DOWN: u16 = 1 << 7;
    pub const R: u16 = 1 << 8;
    pub const L: u16 = 1 << 9;
}

const IRQ_VBLANK: u32 = 0;
const IRQ_VCOUNT: u32 = 2;

pub struct Gba {
    pub cpu: Cpu,
    /// Main-loop passes run since boot.
    pub frames: u64,
}

fn await_frame(c: &mut Cpu) {
    c.pc = FRAME_YIELD;
}

fn return_now(c: &mut Cpu) {
    c.pc = c.ret;
}

impl Gba {
    /// Power on with the given cartridge save and run the boot code up to the
    /// first frame wait.
    pub fn new(rom: Arc<[u8]>, save: Option<&[u8]>) -> Gba {
        let mut cpu = Cpu::new(rom, bn6_gen::lookup);
        if let Some(save) = save {
            let n = save.len().min(cpu.mem.sram.len());
            cpu.mem.sram[..n].copy_from_slice(&save[..n]);
        }
        // The SRAM library runs its read/verify loops from copies on the stack.
        cpu.ram_code.push((0x0814_D8CC, 0x24));
        cpu.ram_code.push((0x0814_D994, 0x30));
        cpu.overrides.insert(addr::MAIN_AWAIT_FRAME, await_frame as Func);
        cpu.overrides.insert(addr::MAIN_POLL_LCD_STATUS, return_now as Func);
        // State after the BIOS hands over to the cartridge (as mGBA's HLE
        // BIOS leaves it): System mode, BIOS-default stacks.
        cpu.cpsr_ctl = gba_rt::MODE_SYSTEM;
        cpu.r[13] = 0x0300_7F00;
        cpu.bank_irq[0] = 0x0300_7FA0;
        cpu.bank_svc[0] = 0x0300_7FE0;
        cpu.bios_prefetch = 0xE129_F000;
        // IO state mGBA's BIOS skip leaves: POSTFLG and the internal memory
        // control register (which mGBA keeps at IO offset 0x210).
        cpu.mem.io[0x300] = 1;
        cpu.mem.io[0x210..0x214].copy_from_slice(&0x0D00_0020u32.to_le_bytes());
        cpu.io.line = 0;
        let mut gba = Gba { cpu, frames: 0 };
        gba.run_until_frame_wait(addr::GAME_ENTRY_POINT);
        gba
    }

    fn run_until_frame_wait(&mut self, entry: u32) {
        let c = &mut self.cpu;
        let f = c.find(entry).unwrap_or_else(|| panic!("no code at {entry:#x}"));
        c.r[14] = HOST_RETURN;
        c.ret = HOST_RETURN;
        f(c);
        assert_eq!(c.pc, FRAME_YIELD, "game code returned to the host at {:#010x}", c.pc);
    }

    /// Run one game frame with the given buttons held: VBlank (interrupt and
    /// DMA), one pass of the main loop, then the mid-frame VCount interrupt
    /// (the sound driver).
    pub fn run_frame(&mut self, keys: u16) {
        self.begin_frame(keys);
        self.run_body();
        self.end_frame();
    }

    /// Latch the buttons and take VBlank: the state the main loop sees when
    /// its frame wait returns.
    pub fn begin_frame(&mut self, keys: u16) {
        self.cpu.io.keys = keys;
        self.vblank();
    }

    /// One pass of the main loop, up to the next frame wait.
    pub fn run_body(&mut self) {
        self.run_until_frame_wait(addr::MAIN_LOOP_BODY);
    }

    /// The rest of the frame: the VCount interrupt (sound driver).
    pub fn end_frame(&mut self) {
        self.vcount_line(80);
        self.frames += 1;
    }

    fn vblank(&mut self) {
        let c = &mut self.cpu;
        c.io.line = 160;
        c.trigger_dma(DmaTiming::VBlank);
        if c.io_peek16(0x0400_0004) & (1 << 3) != 0 {
            c.request_irq(IRQ_VBLANK);
        }
        c.service_irqs();
        c.io.line = 0;
    }

    fn vcount_line(&mut self, line: u16) {
        let c = &mut self.cpu;
        c.io.line = line;
        let dispstat = c.io_peek16(0x0400_0004);
        if dispstat & (1 << 5) != 0 && dispstat >> 8 == line {
            c.request_irq(IRQ_VCOUNT);
        }
        c.service_irqs();
    }

    pub fn ewram(&self) -> &[u8] {
        &self.cpu.mem.ewram[..]
    }
}
