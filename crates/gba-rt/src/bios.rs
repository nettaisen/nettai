//! BIOS calls, emulated the way mGBA's HLE BIOS does it (the configuration
//! the reference emulator runs in): results, clobbered registers, and the
//! stack traffic of its SWI dispatcher all match, so memory stays
//! byte-identical.
//!
//! Ported from mGBA's src/gba/bios.c and src/gba/hle-bios.s,
//! Copyright (c) 2013-2015 Jeffrey Pfau, under the Mozilla Public License 2.0
//! (http://mozilla.org/MPL/2.0/). This file remains under the MPL 2.0.

use crate::Cpu;

const SWI_SOFT_RESET: u32 = 0x00;
const SWI_REGISTER_RAM_RESET: u32 = 0x01;
const SWI_HALT: u32 = 0x02;
const SWI_STOP: u32 = 0x03;
const SWI_INTR_WAIT: u32 = 0x04;
const SWI_VBLANK_INTR_WAIT: u32 = 0x05;
const SWI_DIV: u32 = 0x06;
const SWI_DIV_ARM: u32 = 0x07;
const SWI_SQRT: u32 = 0x08;
const SWI_ARCTAN: u32 = 0x09;
const SWI_ARCTAN2: u32 = 0x0A;
const SWI_CPU_SET: u32 = 0x0B;
const SWI_CPU_FAST_SET: u32 = 0x0C;
const SWI_GET_BIOS_CHECKSUM: u32 = 0x0D;
const SWI_BG_AFFINE_SET: u32 = 0x0E;
const SWI_OBJ_AFFINE_SET: u32 = 0x0F;
const SWI_BIT_UNPACK: u32 = 0x10;
const SWI_LZ77_WRAM: u32 = 0x11;
const SWI_LZ77_VRAM: u32 = 0x12;
const SWI_HUFFMAN: u32 = 0x13;
const SWI_RL_WRAM: u32 = 0x14;
const SWI_RL_VRAM: u32 = 0x15;
const SWI_DIFF8_WRAM: u32 = 0x16;
const SWI_DIFF8_VRAM: u32 = 0x17;
const SWI_DIFF16: u32 = 0x18;
const SWI_SOUND_BIAS: u32 = 0x19;
const SWI_MIDI_KEY_2_FREQ: u32 = 0x1F;

const BIOS_CHECKSUM: u32 = 0xBAAE187F;

impl Cpu {
    /// A Thumb `swi` at `pc`.
    pub fn swi_t(&mut self, n: u32, pc: u32) {
        self.swi(n, pc + 2, true);
    }

    /// An ARM `swi` at `pc` (the BIOS function number is bits 16-23).
    pub fn swi_a(&mut self, n: u32, pc: u32) {
        self.swi(n, pc + 4, false);
    }

    /// The stack traffic of hle-bios.s `swiBase`, for calls that go through
    /// it: save r11/r12/lr and SPSR on the supervisor stack, then r2 and lr
    /// on the caller's stack.
    fn swi_base_frame(&mut self, ret: u32, thumb: bool) {
        let spsr = self.cpsr() | if thumb { 0x20 } else { 0 };
        let svc_sp = self.bank_svc[0];
        let (r11, r12) = (self.r[11], self.r[12]);
        self.st32(svc_sp.wrapping_sub(12), r11);
        self.st32(svc_sp.wrapping_sub(8), r12);
        self.st32(svc_sp.wrapping_sub(4), ret);
        self.st32(svc_sp.wrapping_sub(16), spsr);
        let sp = self.r[13];
        let (r2, lr) = (self.r[2], self.r[14]);
        self.st32(sp.wrapping_sub(8), r2);
        self.st32(sp.wrapping_sub(4), lr);
    }

    fn swi(&mut self, n: u32, ret: u32, thumb: bool) {
        match n {
            SWI_SOFT_RESET | SWI_STOP => panic!("unsupported BIOS call {n:#x}"),
            SWI_REGISTER_RAM_RESET => self.register_ram_reset(),
            SWI_HALT | SWI_INTR_WAIT | SWI_VBLANK_INTR_WAIT => {
                self.swi_base_frame(ret, thumb);
                if n == SWI_VBLANK_INTR_WAIT {
                    self.r[0] = 1;
                    self.r[1] = 1;
                }
                self.with_host(|h, c| h.wait_for_interrupt(c, n));
                if n != SWI_HALT {
                    self.r[0] = 0;
                }
            }
            SWI_DIV => {
                self.swi_base_frame(ret, thumb);
                self.div(self.r[0] as i32, self.r[1] as i32);
            }
            SWI_DIV_ARM => {
                self.swi_base_frame(ret, thumb);
                self.div(self.r[1] as i32, self.r[0] as i32);
            }
            SWI_SQRT => {
                self.swi_base_frame(ret, thumb);
                self.r[0] = sqrt(self.r[0]);
            }
            SWI_ARCTAN => {
                self.swi_base_frame(ret, thumb);
                let (r0, r1, r3) = arctan(self.r[0] as i32);
                self.r[0] = r0 as i32 as u32;
                self.r[1] = r1 as u32;
                self.r[3] = r3 as u32;
            }
            SWI_ARCTAN2 => {
                let (x, y) = (self.r[0] as i32, self.r[1] as i32);
                let (result, r1) = arctan2(x, y);
                if r1.is_some() {
                    // Only the full computation stalls long enough to go
                    // through the dispatcher.
                    self.swi_base_frame(ret, thumb);
                }
                self.r[0] = result as u16 as u32;
                if let Some(r1) = r1 {
                    self.r[1] = r1 as u32;
                }
                self.r[3] = 0x170;
            }
            SWI_CPU_SET => {
                self.swi_base_frame(ret, thumb);
                self.cpu_set();
            }
            SWI_CPU_FAST_SET => {
                self.swi_base_frame(ret, thumb);
                self.cpu_fast_set();
            }
            SWI_GET_BIOS_CHECKSUM => {
                self.r[0] = BIOS_CHECKSUM;
                self.r[1] = 1;
                self.r[3] = 0x4000;
            }
            SWI_BG_AFFINE_SET => self.bg_affine_set(),
            SWI_OBJ_AFFINE_SET => self.obj_affine_set(),
            SWI_BIT_UNPACK => {
                if self.r[0] >= 0x0200_0000 {
                    self.bit_unpack();
                }
            }
            SWI_LZ77_WRAM | SWI_LZ77_VRAM => {
                if self.r[0] & 0x0E00_0000 != 0 {
                    self.swi_base_frame(ret, thumb);
                    self.lz77(if n == SWI_LZ77_WRAM { 1 } else { 2 });
                }
            }
            SWI_HUFFMAN => {
                if self.r[0] & 0x0E00_0000 != 0 {
                    self.huffman();
                }
            }
            SWI_RL_WRAM | SWI_RL_VRAM => {
                if self.r[0] & 0x0E00_0000 != 0 {
                    self.rl(if n == SWI_RL_WRAM { 1 } else { 2 });
                }
            }
            SWI_DIFF8_WRAM | SWI_DIFF8_VRAM | SWI_DIFF16 => {
                if self.r[0] & 0x0E00_0000 != 0 {
                    let inw = if n == SWI_DIFF16 { 2 } else { 1 };
                    let outw = if n == SWI_DIFF8_WRAM { 1 } else { 2 };
                    self.unfilter(inw, outw);
                }
            }
            SWI_SOUND_BIAS => {}
            SWI_MIDI_KEY_2_FREQ => {
                let key = self.ld32(self.r[0].wrapping_add(4));
                let e = (180.0f32 - self.r[1] as f32 - self.r[2] as f32 / 256.0) / 12.0;
                self.r[0] = (key as f32 / e.exp2()) as u32;
            }
            _ => {}
        }
    }

    fn div(&mut self, num: i32, denom: i32) {
        if denom == 0 {
            self.r[0] = if num < 0 { -1i32 as u32 } else { 1 };
            self.r[1] = num as u32;
            self.r[3] = 1;
        } else if denom == -1 && num == i32::MIN {
            self.r[0] = i32::MIN as u32;
            self.r[1] = 0;
            self.r[3] = i32::MIN as u32;
        } else {
            let q = num.wrapping_div(denom);
            let r = num.wrapping_rem(denom);
            self.r[0] = q as u32;
            self.r[1] = r as u32;
            self.r[3] = q.unsigned_abs();
        }
    }

    /// hle-bios.s CpuSet.
    fn cpu_set(&mut self) {
        let (mut r0, mut r1, r2) = (self.r[0], self.r[1], self.r[2]);
        let r4 = r2 << 12;
        let mut r12 = r0;
        let mut r5 = r1;
        let lt = |a: u32, b: u32| (a as i32) < (b as i32);
        if r2 & 0x0100_0000 != 0 {
            if r2 & 0x0400_0000 != 0 {
                let end = r5.wrapping_add(r4 >> 10);
                let v = self.ld32(r0 & !3);
                r0 = r0.wrapping_add(4);
                while lt(r1, end) {
                    self.st32(r1, v);
                    r1 = r1.wrapping_add(4);
                }
            } else {
                r12 &= !1;
                r5 &= !1;
                let end = r5.wrapping_add(r4 >> 11);
                let v = self.ld16(r12);
                while lt(r5, end) {
                    self.st16(r5, v);
                    r5 = r5.wrapping_add(2);
                }
            }
        } else if r2 & 0x0400_0000 != 0 {
            let end = r5.wrapping_add(r4 >> 10);
            while lt(r1, end) {
                let v = self.ld32(r0 & !3);
                r0 = r0.wrapping_add(4);
                self.st32(r1, v);
                r1 = r1.wrapping_add(4);
            }
        } else {
            let end = r5.wrapping_add(r4 >> 11);
            while lt(r5, end) {
                let v = self.ld16(r12);
                r12 = r12.wrapping_add(2);
                self.st16(r5, v);
                r5 = r5.wrapping_add(2);
            }
        }
        self.r[0] = r0;
        self.r[1] = r1;
        self.r[3] = 0x170;
    }

    /// hle-bios.s CpuFastSet.
    fn cpu_fast_set(&mut self) {
        let (mut r0, mut r1, r2) = (self.r[0], self.r[1], self.r[2]);
        let end = r1.wrapping_add((r2 << 12) >> 10);
        let lt = |a: u32, b: u32| (a as i32) < (b as i32);
        let r3;
        if r2 & 0x0100_0000 != 0 {
            let v = self.ld32(r0);
            r3 = v;
            while lt(r1, end) {
                for i in 0..8 {
                    self.st32((r1 & !3).wrapping_add(4 * i), v);
                }
                r1 = r1.wrapping_add(32);
            }
        } else {
            let mut last = self.r[3];
            while lt(r1, end) {
                let mut block = [0u32; 8];
                for (i, w) in block.iter_mut().enumerate() {
                    *w = self.ld32((r0 & !3).wrapping_add(4 * i as u32));
                }
                r0 = r0.wrapping_add(32);
                for (i, w) in block.iter().enumerate() {
                    self.st32((r1 & !3).wrapping_add(4 * i as u32), *w);
                }
                r1 = r1.wrapping_add(32);
                last = block[0];
            }
            r3 = last;
        }
        self.r[0] = r0;
        self.r[1] = r1;
        self.r[3] = r3;
    }

    fn register_ram_reset(&mut self) {
        let regs = self.r[0];
        self.io_write16(0x0400_0000, 0x0080);
        if regs & 0x01 != 0 {
            self.mem.ewram.fill(0);
        }
        if regs & 0x02 != 0 {
            self.mem.iwram[..0x8000 - 0x200].fill(0);
        }
        if regs & 0x04 != 0 {
            self.mem.palette.fill(0);
        }
        if regs & 0x08 != 0 {
            self.mem.vram.fill(0);
        }
        if regs & 0x10 != 0 {
            self.mem.oam.fill(0);
        }
        if regs & 0x20 != 0 {
            for (a, v) in [(0x128, 0), (0x134, 0x8000), (0x12A, 0), (0x140, 0)] {
                self.io_write16(0x0400_0000 | a, v);
            }
            for a in [0x150, 0x152, 0x154, 0x156] {
                self.io_write16(0x0400_0000 | a, 0);
            }
        }
        if regs & 0x40 != 0 {
            for a in (0x60..=0x84).step_by(2) {
                self.io_write16(0x0400_0000 | a, 0);
            }
            self.io_write16(0x0400_0088, 0x200);
        }
        if regs & 0x80 != 0 {
            for a in (0x04..0x56).step_by(2) {
                let v = if matches!(a, 0x20 | 0x26 | 0x30 | 0x36) { 0x100 } else { 0 };
                self.io_write16(0x0400_0000 | a, v);
            }
            for a in (0xB0..0x110).step_by(2) {
                self.io_write16(0x0400_0000 | a, 0);
            }
            self.io_write16(0x0400_0200, 0);
            self.io_write16(0x0400_0202, 0xFFFF);
            self.io_write16(0x0400_0204, 0);
            self.io_write16(0x0400_0208, 0);
        }
    }

    fn bg_affine_set(&mut self) {
        let mut i = self.r[2] as i32;
        let mut offset = self.r[0];
        let mut dest = self.r[1];
        while i > 0 {
            i -= 1;
            let ox = self.ld32(offset) as i32 as f32 / 256.0;
            let oy = self.ld32(offset + 4) as i32 as f32 / 256.0;
            let cx = self.ld16(offset + 8) as i16 as f32;
            let cy = self.ld16(offset + 10) as i16 as f32;
            let sx = self.ld16(offset + 12) as i16 as f32 / 256.0;
            let sy = self.ld16(offset + 14) as i16 as f32 / 256.0;
            let theta = (self.ld16(offset + 16) >> 8) as f32 / 128.0 * std::f32::consts::PI;
            offset += 20;
            let (s, co) = theta.sin_cos();
            let a = co * sx;
            let b = s * -sx;
            let c = s * sy;
            let d = co * sy;
            let rx = ox - (a * cx + b * cy);
            let ry = oy - (c * cx + d * cy);
            self.st16(dest, (a * 256.0) as i32 as u32);
            self.st16(dest + 2, (b * 256.0) as i32 as u32);
            self.st16(dest + 4, (c * 256.0) as i32 as u32);
            self.st16(dest + 6, (d * 256.0) as i32 as u32);
            self.st32(dest + 8, (rx * 256.0) as i32 as u32);
            self.st32(dest + 12, (ry * 256.0) as i32 as u32);
            dest += 16;
        }
    }

    fn obj_affine_set(&mut self) {
        let mut i = self.r[2] as i32;
        let mut offset = self.r[0];
        let mut dest = self.r[1];
        let diff = self.r[3];
        while i > 0 {
            i -= 1;
            let sx = self.ld16(offset) as i16 as f32 / 256.0;
            let sy = self.ld16(offset + 2) as i16 as f32 / 256.0;
            let theta = (self.ld16(offset + 4) >> 8) as f32 / 128.0 * std::f32::consts::PI;
            offset += 8;
            let (s, co) = theta.sin_cos();
            let a = co * sx;
            let b = s * -sx;
            let c = s * sy;
            let d = co * sy;
            self.st16(dest, (a * 256.0) as i32 as u32);
            self.st16(dest.wrapping_add(diff), (b * 256.0) as i32 as u32);
            self.st16(dest.wrapping_add(diff * 2), (c * 256.0) as i32 as u32);
            self.st16(dest.wrapping_add(diff * 3), (d * 256.0) as i32 as u32);
            dest = dest.wrapping_add(diff * 4);
        }
    }

    fn lz77(&mut self, width: u32) {
        let mut source = self.r[0];
        let mut dest = self.r[1];
        let mut remaining = ((self.ld32(source) & 0xFFFF_FF00) >> 8) as i32;
        source += 4;
        let mut blockheader = 0u32;
        let mut blocks_remaining = 0;
        let mut halfword: i32 = 0;
        while remaining > 0 {
            if blocks_remaining > 0 {
                if blockheader & 0x80 != 0 {
                    let block = self.ld8(source + 1) | (self.ld8(source) << 8);
                    source += 2;
                    let mut disp = dest.wrapping_sub(block & 0x0FFF).wrapping_sub(1);
                    let mut bytes = (block >> 12) + 3;
                    while bytes > 0 {
                        bytes -= 1;
                        if remaining > 0 {
                            remaining -= 1;
                        }
                        if width == 2 {
                            let mut byte = self.ld16(disp & !1) as u16 as i16 as i32;
                            byte >>= (disp & 1) * 8;
                            if dest & 1 != 0 {
                                halfword |= byte << 8;
                                self.st16(dest ^ 1, halfword as u32);
                            } else {
                                halfword = byte & 0xFF;
                            }
                        } else {
                            let byte = self.ld8(disp);
                            self.st8(dest, byte);
                        }
                        disp = disp.wrapping_add(1);
                        dest = dest.wrapping_add(1);
                    }
                } else {
                    let byte = self.ld8(source);
                    source += 1;
                    if width == 2 {
                        if dest & 1 != 0 {
                            halfword |= (byte as i32) << 8;
                            self.st16(dest ^ 1, halfword as u32);
                        } else {
                            halfword = byte as i32;
                        }
                    } else {
                        self.st8(dest, byte);
                    }
                    dest = dest.wrapping_add(1);
                    remaining -= 1;
                }
                blockheader <<= 1;
                blocks_remaining -= 1;
            } else {
                blockheader = self.ld8(source);
                source += 1;
                blocks_remaining = 8;
            }
        }
        self.r[0] = source;
        self.r[1] = dest;
        self.r[3] = 0;
    }

    fn huffman(&mut self) {
        let mut source = self.r[0] & !3;
        let mut dest = self.r[1];
        let header = self.ld32(source);
        let mut remaining = (header >> 8) as i32;
        let mut bits = header & 0xF;
        if bits == 0 {
            bits = 8;
        }
        if 32 % bits != 0 || bits == 1 {
            return;
        }
        let treesize = (self.ld8(source + 4) << 1) + 1;
        let mut block: u32 = 0;
        let tree_base = source + 5;
        source += 5 + treesize;
        let mut n_pointer = tree_base;
        let mut node = self.ld8(n_pointer);
        let mut bits_seen = 0;
        while remaining > 0 {
            let mut bitstream = self.ld32(source);
            source += 4;
            let mut bits_remaining = 32;
            while bits_remaining > 0 && remaining > 0 {
                let next = (n_pointer & !1) + (node & 0x3F) * 2 + 2;
                let read_bits;
                let mut descend = None;
                if bitstream & 0x8000_0000 != 0 {
                    if node & 0x40 != 0 {
                        read_bits = self.ld8(next + 1);
                    } else {
                        descend = Some(next + 1);
                        read_bits = 0;
                    }
                } else if node & 0x80 != 0 {
                    read_bits = self.ld8(next);
                } else {
                    descend = Some(next);
                    read_bits = 0;
                }
                if let Some(p) = descend {
                    n_pointer = p;
                    node = self.ld8(n_pointer);
                } else {
                    block |= (read_bits & ((1 << bits) - 1)) << bits_seen;
                    bits_seen += bits;
                    n_pointer = tree_base;
                    node = self.ld8(n_pointer);
                    if bits_seen == 32 {
                        bits_seen = 0;
                        self.st32(dest, block);
                        dest += 4;
                        remaining -= 4;
                        block = 0;
                    }
                }
                bits_remaining -= 1;
                bitstream <<= 1;
            }
        }
        self.r[0] = source;
        self.r[1] = dest;
    }

    fn rl(&mut self, width: u32) {
        let mut source = self.r[0];
        let mut remaining = ((self.ld32(source & !3) & 0xFFFF_FF00) >> 8) as i32;
        let mut padding = (4 - remaining) & 3;
        source += 4;
        let mut dest = self.r[1];
        let mut halfword: u32 = 0;
        let mut put = |c: &mut Cpu, dest: &mut u32, byte: u32| {
            if width == 2 {
                if *dest & 1 != 0 {
                    halfword |= byte << 8;
                    c.st16(*dest ^ 1, halfword);
                } else {
                    halfword = byte;
                }
            } else {
                c.st8(*dest, byte);
            }
            *dest = dest.wrapping_add(1);
        };
        while remaining > 0 {
            let mut blockheader = self.ld8(source) as i32;
            source += 1;
            if blockheader & 0x80 != 0 {
                blockheader = (blockheader & 0x7F) + 3;
                let block = self.ld8(source);
                source += 1;
                while blockheader > 0 && remaining > 0 {
                    blockheader -= 1;
                    remaining -= 1;
                    put(self, &mut dest, block);
                }
            } else {
                blockheader += 1;
                while blockheader > 0 && remaining > 0 {
                    blockheader -= 1;
                    remaining -= 1;
                    let byte = self.ld8(source);
                    source += 1;
                    put(self, &mut dest, byte);
                }
            }
        }
        if width == 2 {
            if dest & 1 != 0 {
                padding -= 1;
                dest += 1;
            }
            while padding > 0 {
                self.st16(dest, 0);
                padding -= 2;
                dest += 2;
            }
        } else {
            while padding > 0 {
                padding -= 1;
                self.st8(dest, 0);
                dest += 1;
            }
        }
        self.r[0] = source;
        self.r[1] = dest;
    }

    fn unfilter(&mut self, inwidth: u32, outwidth: u32) {
        let mut source = self.r[0] & !3;
        let mut dest = self.r[1];
        let header = self.ld32(source);
        let mut remaining = (header >> 8) as i32;
        let mut halfword: u16 = 0;
        let mut old: u16 = 0;
        source += 4;
        while remaining > 0 {
            let mut new = if inwidth == 1 { self.ld8(source) as u16 } else { self.ld16(source) as u16 };
            new = new.wrapping_add(old);
            if outwidth > inwidth {
                halfword >>= 8;
                halfword |= new << 8;
                if source & 1 != 0 {
                    self.st16(dest, halfword as u32);
                    dest += outwidth;
                    remaining -= outwidth as i32;
                }
            } else if outwidth == 1 {
                self.st8(dest, new as u32);
                dest += outwidth;
                remaining -= outwidth as i32;
            } else {
                self.st16(dest, new as u32);
                dest += outwidth;
                remaining -= outwidth as i32;
            }
            old = new;
            source += inwidth;
        }
        self.r[0] = source;
        self.r[1] = dest;
    }

    fn bit_unpack(&mut self) {
        let mut source = self.r[0];
        let mut dest = self.r[1];
        let info = self.r[2];
        let mut source_len = self.ld16(info);
        let source_width = self.ld8(info + 2);
        let dest_width = self.ld8(info + 3);
        if ![1, 2, 4, 8].contains(&source_width) || ![1, 2, 4, 8, 16, 32].contains(&dest_width) {
            return;
        }
        let bias = self.ld32(info + 4);
        let mut input: u8 = 0;
        let mut out: u32 = 0;
        let mut bits_remaining = 0i32;
        let mut bits_eaten = 0u32;
        while source_len > 0 || bits_remaining != 0 {
            if bits_remaining == 0 {
                input = self.ld8(source) as u8;
                bits_remaining = 8;
                source += 1;
                source_len -= 1;
            }
            let mut scaled = (input as u32) & ((1 << source_width) - 1);
            input = if source_width >= 8 { 0 } else { input >> source_width };
            if scaled != 0 || bias & 0x8000_0000 != 0 {
                scaled = scaled.wrapping_add(bias & 0x7FFF_FFFF);
            }
            bits_remaining -= source_width as i32;
            out |= scaled.checked_shl(bits_eaten).unwrap_or(0);
            bits_eaten += dest_width;
            if bits_eaten == 32 {
                self.st32(dest, out);
                bits_eaten = 0;
                out = 0;
                dest += 4;
            }
        }
        self.r[0] = source;
        self.r[1] = dest;
    }
}

/// mGBA's _Sqrt.
fn sqrt(x: u32) -> u32 {
    if x == 0 {
        return 0;
    }
    let mut upper = x;
    let mut bound: u32 = 1;
    while bound < upper {
        upper >>= 1;
        bound <<= 1;
    }
    loop {
        upper = x;
        let mut accum: u32 = 0;
        let mut lower = bound;
        loop {
            let old_lower = lower;
            if lower <= upper >> 1 {
                lower <<= 1;
            }
            if old_lower >= upper >> 1 {
                break;
            }
        }
        loop {
            accum <<= 1;
            if upper >= lower {
                accum += 1;
                upper -= lower;
            }
            if lower == bound {
                break;
            }
            lower >>= 1;
        }
        let old_bound = bound;
        bound = bound.wrapping_add(accum);
        bound >>= 1;
        if bound >= old_bound {
            bound = old_bound;
            break;
        }
    }
    bound
}

/// mGBA's _ArcTan: (r0 as int16, r1, r3).
fn arctan(i: i32) -> (i16, i32, i32) {
    let a = -(i.wrapping_mul(i) >> 14);
    let mut b = (0xA9i32.wrapping_mul(a) >> 14) + 0x390;
    b = (b.wrapping_mul(a) >> 14) + 0x91C;
    b = (b.wrapping_mul(a) >> 14) + 0xFB6;
    b = (b.wrapping_mul(a) >> 14) + 0x16AA;
    b = (b.wrapping_mul(a) >> 14) + 0x2081;
    b = (b.wrapping_mul(a) >> 14) + 0x3651;
    b = (b.wrapping_mul(a) >> 14) + 0xA2F9;
    (((i.wrapping_mul(b)) >> 16) as i16, a, b)
}

/// mGBA's _ArcTan2: (result, r1 if the full computation ran).
fn arctan2(x: i32, y: i32) -> (i32, Option<i32>) {
    if y == 0 {
        return (if x >= 0 { 0 } else { 0x8000 }, None);
    }
    if x == 0 {
        return (if y >= 0 { 0x4000 } else { 0xC000 }, None);
    }
    let at = |i: i32| {
        let (r0, r1, _) = arctan(i);
        (r0 as i32, r1)
    };
    let (v, r1) = if y >= 0 {
        if x >= 0 {
            if x >= y {
                at((y << 14) / x)
            } else {
                let (v, r1) = at((x << 14) / y);
                (0x4000 - v, r1)
            }
        } else if -x >= y {
            let (v, r1) = at((y << 14) / x);
            (v + 0x8000, r1)
        } else {
            let (v, r1) = at((x << 14) / y);
            (0x4000 - v, r1)
        }
    } else if x <= 0 {
        if -x > -y {
            let (v, r1) = at((y << 14) / x);
            (v + 0x8000, r1)
        } else {
            let (v, r1) = at((x << 14) / y);
            (0xC000 - v, r1)
        }
    } else if x >= -y {
        let (v, r1) = at((y << 14) / x);
        (v + 0x10000, r1)
    } else {
        let (v, r1) = at((x << 14) / y);
        (0xC000 - v, r1)
    };
    (v, Some(r1))
}
