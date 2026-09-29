//! Byte offsets of the video I/O registers within the 0x400-byte I/O block
//! (register address minus `0x0400_0000`). Registers are little-endian
//! halfwords; 32-bit registers (`BGxX`, `BGxY`) are split into a low
//! halfword at the listed offset and a high halfword at offset + 2.
//!
//! Handy for `line_io` callbacks that replay per-scanline register writes.

/// LCD control: mode, frame select, OBJ mapping, forced blank, layer and
/// window enables.
pub const DISPCNT: usize = 0x00;
/// Undocumented green swap (mGBA: `STEREOCNT`); bit 0 swaps the green
/// channel of each horizontal pixel pair.
pub const GREENSWP: usize = 0x02;
pub const BG0CNT: usize = 0x08;
pub const BG1CNT: usize = 0x0A;
pub const BG2CNT: usize = 0x0C;
pub const BG3CNT: usize = 0x0E;
pub const BG0HOFS: usize = 0x10;
pub const BG0VOFS: usize = 0x12;
pub const BG1HOFS: usize = 0x14;
pub const BG1VOFS: usize = 0x16;
pub const BG2HOFS: usize = 0x18;
pub const BG2VOFS: usize = 0x1A;
pub const BG3HOFS: usize = 0x1C;
pub const BG3VOFS: usize = 0x1E;
pub const BG2PA: usize = 0x20;
pub const BG2PB: usize = 0x22;
pub const BG2PC: usize = 0x24;
pub const BG2PD: usize = 0x26;
/// BG2 reference point X (28-bit signed 20.8 fixed point, two halfwords).
pub const BG2X: usize = 0x28;
/// BG2 reference point Y (28-bit signed 20.8 fixed point, two halfwords).
pub const BG2Y: usize = 0x2C;
pub const BG3PA: usize = 0x30;
pub const BG3PB: usize = 0x32;
pub const BG3PC: usize = 0x34;
pub const BG3PD: usize = 0x36;
pub const BG3X: usize = 0x38;
pub const BG3Y: usize = 0x3C;
pub const WIN0H: usize = 0x40;
pub const WIN1H: usize = 0x42;
pub const WIN0V: usize = 0x44;
pub const WIN1V: usize = 0x46;
pub const WININ: usize = 0x48;
pub const WINOUT: usize = 0x4A;
pub const MOSAIC: usize = 0x4C;
pub const BLDCNT: usize = 0x50;
pub const BLDALPHA: usize = 0x52;
pub const BLDY: usize = 0x54;

/// Every halfword the renderer consumes, in the order mGBA replays them
/// into a freshly attached renderer (`GBAVideoAssociateRenderer`).
pub(crate) const VIDEO_REGISTERS: [usize; 40] = {
    let mut regs = [0; 40];
    regs[0] = DISPCNT;
    regs[1] = GREENSWP;
    let mut i = 2;
    let mut addr = BG0CNT;
    while addr <= BLDY {
        if addr != 0x4E {
            regs[i] = addr;
            i += 1;
        }
        addr += 2;
    }
    regs
};
