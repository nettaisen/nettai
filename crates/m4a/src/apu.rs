//! The GBA's sound hardware as the driver uses it: the four PSG channels
//! (two squares, the wave channel, noise) driven through their registers,
//! and what they put into the DAC.
//!
//! The PSG follows the Game Boy APU as the GBA has it, and as mGBA models
//! it (the emulator the port is checked against): duty steps, the 512 Hz
//! frame sequencer for lengths, square 1's sweep and the hardware
//! envelopes, the noise LFSR, the wave channel's two banks of wave RAM.
//! The driver writes the registers once a frame, during VBlank; this
//! model applies them where its frame starts. Time is in CPU cycles
//! (16.78 MHz).

/// CPU cycles in a frame (228 lines of 1232).
pub const FRAME_CYCLES: u32 = 280_896;
/// Frame sequencer period: 512 Hz.
const SEQUENCER_CYCLES: u32 = 32_768;

/// The PSG's registers (NR10..NR52, at 0x04000060..0x04000084).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reg {
    /// Square 1 sweep.
    Nr10,
    /// Square 1 duty and length.
    Nr11,
    /// Square 1 envelope.
    Nr12,
    /// Square 1 frequency, low byte.
    Nr13,
    /// Square 1 frequency high bits, length enable, restart.
    Nr14,
    Nr21,
    Nr22,
    Nr23,
    Nr24,
    /// Wave channel DAC enable, bank, size.
    Nr30,
    /// Wave length.
    Nr31,
    /// Wave output level.
    Nr32,
    Nr33,
    Nr34,
    /// Noise length.
    Nr41,
    Nr42,
    /// Noise clock shift, width and divisor.
    Nr43,
    Nr44,
    /// PSG master volume, left and right.
    Nr50,
    /// PSG channel enables, left (high nibble) and right (low nibble).
    Nr51,
    /// Sound enable and the channels' status bits.
    Nr52,
}

const SQUARE_DUTY: [[u8; 8]; 4] = [
    [0, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 1, 1, 1],
    [0, 1, 1, 1, 1, 1, 1, 0],
];

/// Whether an envelope still steps.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Ended {
    #[default]
    No,
    /// Stopped with sound (at 15 going up, or a level without a period).
    Loud,
    /// Stopped silent (at 0).
    Silent,
}

/// A volume envelope (NRx2).
#[derive(Clone, Copy, Debug, Default)]
struct Envelope {
    initial: u8,
    increase: bool,
    period: u8,
    volume: u8,
    timer: u8,
    ended: Ended,
}

impl Envelope {
    /// NRx2. Returns whether the channel's DAC is on.
    fn write(&mut self, value: u8) -> bool {
        self.period = value & 7;
        self.increase = value & 8 != 0;
        self.initial = value >> 4;
        self.update_ended();
        self.dac()
    }
    fn dac(&self) -> bool {
        self.initial != 0 || self.increase
    }
    /// A restart. Returns whether the DAC is on.
    fn restart(&mut self) -> bool {
        self.volume = self.initial;
        self.timer = self.period;
        self.update_ended();
        self.dac()
    }
    fn update_ended(&mut self) {
        if self.period == 0 {
            self.ended = if self.volume != 0 { Ended::Loud } else { Ended::Silent };
        } else if !self.increase && self.volume == 0 {
            self.ended = Ended::Silent;
        } else if self.increase && self.volume == 15 {
            self.ended = Ended::Loud;
        } else if self.ended != Ended::No {
            self.timer = self.period;
            self.ended = Ended::No;
        }
    }
    /// A 64 Hz step.
    fn step(&mut self) {
        if self.ended != Ended::No {
            return;
        }
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return;
        }
        let v = self.volume as i32 + if self.increase { 1 } else { -1 };
        if v >= 15 {
            self.volume = 15;
            self.ended = Ended::Loud;
        } else if v <= 0 {
            self.volume = 0;
            self.ended = Ended::Silent;
        } else {
            self.volume = v as u8;
            self.timer = self.period;
        }
    }
}

/// A sound length counter (NRx1, and NRx4's length enable).
#[derive(Clone, Copy, Debug, Default)]
struct Length {
    counter: u16,
    enabled: bool,
}

impl Length {
    /// NRx4's length enable. Turning it on while the sequencer's next step
    /// doesn't count lengths counts once at once. Returns whether that
    /// ran the length out.
    fn enable(&mut self, on: bool, extra_clock: bool) -> bool {
        let was = self.enabled;
        self.enabled = on;
        if !was && on && self.counter != 0 && extra_clock {
            self.counter -= 1;
            return self.counter == 0;
        }
        false
    }
    fn restart(&mut self, full: u16, extra_clock: bool) {
        if self.counter == 0 {
            self.counter = full;
            if self.enabled && extra_clock {
                self.counter -= 1;
            }
        }
    }
    /// A 256 Hz step; returns whether the channel ran out.
    fn step(&mut self) -> bool {
        if self.enabled && self.counter != 0 {
            self.counter -= 1;
            return self.counter == 0;
        }
        false
    }
}

/// Square 1's frequency sweep (NR10).
#[derive(Clone, Copy, Debug)]
struct Sweep {
    shift: u8,
    decrease: bool,
    /// 128 Hz steps between updates (8 for NR10's 0: no updates).
    time: u8,
    step: u8,
    enabled: bool,
    /// The frequency the sweep works from.
    shadow: u16,
    /// An update happened since the last NR10 write or restart.
    occurred: bool,
}

impl Default for Sweep {
    fn default() -> Sweep {
        Sweep { shift: 0, decrease: false, time: 8, step: 8, enabled: false, shadow: 0, occurred: false }
    }
}

/// A square channel. Its output is a stored level, as mGBA keeps it: it
/// follows the duty steps while the channel plays (and isn't silent), the
/// envelope's steps and restarts, and a register write to the channel
/// brings it up to date; a channel that stops holds its last level.
#[derive(Clone, Copy, Debug, Default)]
struct Square {
    on: bool,
    duty: u8,
    length: Length,
    envelope: Envelope,
    frequency: u16,
    /// Position in the duty cycle.
    index: u8,
    /// Cycles until the next duty step.
    timer: u32,
    sweep: Sweep,
    sample: u8,
}

impl Square {
    fn period(&self) -> u32 {
        16 * (2048 - self.frequency as u32)
    }
    fn update_sample(&mut self) {
        self.sample = SQUARE_DUTY[self.duty as usize][self.index as usize] * self.envelope.volume;
    }
    fn run(&mut self, mut cycles: u32) {
        if self.timer == 0 {
            self.timer = self.period();
        }
        let live = self.on && self.envelope.ended != Ended::Silent;
        while cycles >= self.timer {
            cycles -= self.timer;
            self.index = (self.index + 1) & 7;
            self.timer = self.period();
            if live {
                self.update_sample();
            }
        }
        self.timer -= cycles;
    }
    /// A sweep calculation (`initial`: only the overflow check, as on a
    /// restart and after an update). Returns false if the frequency
    /// overflows.
    fn update_sweep(&mut self, initial: bool) -> bool {
        if initial || self.sweep.time != 8 {
            let shadow = self.sweep.shadow as i32;
            let delta = shadow >> self.sweep.shift;
            if self.sweep.decrease {
                let f = shadow - delta;
                if !initial && f >= 0 {
                    self.frequency = f as u16;
                    self.sweep.shadow = f as u16;
                }
            } else {
                let f = shadow + delta;
                if f >= 2048 {
                    return false;
                }
                if !initial && self.sweep.shift != 0 {
                    self.frequency = f as u16;
                    self.sweep.shadow = f as u16;
                    if !self.update_sweep(true) {
                        return false;
                    }
                }
            }
            self.sweep.occurred = true;
        }
        self.sweep.step = self.sweep.time;
        true
    }
    /// A 128 Hz sweep step.
    fn sweep_step(&mut self) {
        if !self.sweep.enabled {
            return;
        }
        self.sweep.step = self.sweep.step.wrapping_sub(1);
        if self.sweep.step == 0 && !self.update_sweep(false) {
            self.on = false;
        }
    }
    /// A 64 Hz envelope step.
    fn envelope_step(&mut self) {
        if self.on && self.envelope.ended == Ended::No {
            self.envelope.step();
            self.update_sample();
        }
    }
    fn restart(&mut self, extra_clock: bool, with_sweep: bool) {
        self.on = self.envelope.restart();
        if with_sweep {
            let s = &mut self.sweep;
            s.shadow = self.frequency;
            s.step = s.time;
            s.enabled = s.step != 8 || s.shift != 0;
            s.occurred = false;
            if self.on && self.sweep.shift != 0 {
                self.on = self.update_sweep(true);
            }
        }
        self.length.restart(64, extra_clock);
        self.update_sample();
    }
}

/// The wave channel. Its output is a stored level too: the step it played
/// last at the output level, which an NR32 write works out again (from the
/// next step, and without the 75% level's x3); it holds while the channel
/// is stopped.
#[derive(Clone, Copy, Debug, Default)]
struct WaveChannel {
    on: bool,
    dac: bool,
    /// The bank that plays (NR30 bit 6); the CPU writes the other.
    bank: u8,
    /// Both banks play as one 64-step wave (NR30 bit 5).
    double: bool,
    length: Length,
    /// NR32's level bits (5-7).
    level: u8,
    rate: u16,
    /// The next step's position in the wave, 0..32 (0..64 with both banks).
    position: u8,
    timer: u32,
    ram: [[u8; 16]; 2],
    sample: u8,
}

impl WaveChannel {
    fn nibble_at(&self, p: u8) -> u8 {
        let (bank, p) = if self.double { ((self.bank + p / 32) & 1, p % 32) } else { (self.bank, p) };
        let byte = self.ram[bank as usize][(p / 2) as usize];
        if p & 1 == 0 { byte >> 4 } else { byte & 15 }
    }
    /// The level's shift: mute, 100%, 50%, 25% (and 75%: x3, then 25%).
    fn shift(&self) -> u8 {
        match self.level {
            0 => 4,
            1 => 0,
            2 => 1,
            _ => 2,
        }
    }
    /// NR32.
    fn set_level(&mut self, value: u8) {
        self.level = value >> 5;
        self.sample = self.nibble_at(self.position) >> self.shift();
    }
    fn run(&mut self, mut cycles: u32) {
        if !self.on {
            return;
        }
        let steps = if self.double { 64 } else { 32 };
        let mut stepped = None;
        while cycles >= self.timer {
            cycles -= self.timer;
            stepped = Some(self.nibble_at(self.position));
            self.position = (self.position + 1) % steps;
            self.timer = 8 * (2048 - self.rate as u32);
        }
        self.timer -= cycles;
        if let Some(n) = stepped {
            let n = if self.level > 3 { n * 3 } else { n };
            self.sample = n >> self.shift();
        }
    }
}

/// The noise channel. Its output is the last bit shifted out times the
/// volume, stored at each step (and each envelope step); a restart leaves
/// it until the next step; it holds while the channel is stopped.
#[derive(Clone, Copy, Debug, Default)]
struct Noise {
    on: bool,
    length: Length,
    envelope: Envelope,
    shift: u8,
    narrow: bool,
    divisor: u8,
    lfsr: u16,
    timer: u32,
    sample: u8,
}

impl Noise {
    fn period(&self) -> u32 {
        let base = if self.divisor == 0 { 1 } else { 2 * self.divisor as u32 };
        (base << self.shift) * 32
    }
    fn run(&mut self, mut cycles: u32) {
        if !self.on {
            return;
        }
        while cycles >= self.timer {
            cycles -= self.timer;
            let bit = ((self.lfsr ^ (self.lfsr >> 1) ^ 1) & 1) as u8;
            self.lfsr >>= 1;
            let tap = if self.narrow { 0x4040 } else { 0x4000 };
            if bit != 0 {
                self.lfsr |= tap;
            } else {
                self.lfsr &= !tap;
            }
            self.sample = bit * self.envelope.volume;
            self.timer = self.period();
        }
        self.timer -= cycles;
    }
    /// A 64 Hz envelope step.
    fn envelope_step(&mut self) {
        if self.on && self.envelope.ended == Ended::No {
            self.envelope.step();
            self.sample = u8::from(self.sample > 0) * self.envelope.volume;
        }
    }
}

/// One Direct Sound FIFO: 8 words the DMA fills 4 at a time from the PCM
/// buffer when fewer than 4 are left, and a word being shifted out a byte
/// per timer overflow.
#[derive(Clone, Debug, Default)]
struct Fifo {
    queue: std::collections::VecDeque<i8>,
    word: [i8; 4],
    /// Bytes of `word` still to play.
    left: u8,
    /// The byte the DAC sees.
    out: i8,
}

impl Fifo {
    /// A timer overflow: the next byte to the DAC. Returns whether the
    /// FIFO asks the DMA for more (it fills after this byte is taken).
    fn tick(&mut self) -> bool {
        let words = self.queue.len() / 4;
        let request = words < 4;
        if self.left == 0 && words > 0 {
            for b in self.word.iter_mut() {
                *b = self.queue.pop_front().unwrap();
            }
            self.left = 4;
        }
        // The word shifts right a byte at a time (an arithmetic shift: an
        // emptied word repeats its top byte's sign).
        self.out = self.word[0];
        self.word = [self.word[1], self.word[2], self.word[3], if self.word[3] < 0 { -1 } else { 0 }];
        self.left = self.left.saturating_sub(1);
        request
    }
}

/// The Direct Sound hardware: FIFO A (the driver's right channel) and B
/// (its left), fed by DMA 1 and 2 from the driver's PCM buffer, each
/// timer 0 overflow taking a byte to the DAC.
#[derive(Clone, Debug, Default)]
pub struct DirectSound {
    right: Fifo,
    left: Fifo,
    /// The DMAs' read position in the buffer (the same for both).
    dma: usize,
}

impl DirectSound {
    /// The driver re-arms the DMAs (each lap of its ring buffer): they
    /// read from the buffer's start again, after what the FIFOs hold.
    pub fn rearm(&mut self) {
        self.dma = 0;
    }

    /// Timer 0 overflows: a byte from each FIFO to the DAC, and the DMAs
    /// refill them from `right` and `left` (the PCM buffer's halves; past
    /// the right half the DMA reads on into the left one, past the left
    /// one into whatever follows it, read as 0).
    pub fn tick(&mut self, right: &[i8], left: &[i8]) {
        let refill_right = self.right.tick();
        let refill_left = self.left.tick();
        if refill_right || refill_left {
            let n = right.len();
            for k in self.dma..self.dma + 16 {
                if refill_right {
                    self.right.queue.push_back(if k < n { right[k] } else { left.get(k - n).copied().unwrap_or(0) });
                }
                if refill_left {
                    self.left.queue.push_back(left.get(k).copied().unwrap_or(0));
                }
            }
            self.dma += 16;
        }
    }

    /// The bytes the DAC sees: (right, left).
    pub fn output(&self) -> (i8, i8) {
        (self.right.out, self.left.out)
    }
}

/// The sound hardware.
#[derive(Clone, Debug)]
pub struct Apu {
    regs: [u8; 21],
    enabled: bool,
    square1: Square,
    square2: Square,
    wave: WaveChannel,
    noise: Noise,
    /// The frame sequencer's last step (0..8) and cycles until the next.
    step: u8,
    step_timer: u32,
    /// SOUNDCNT_H's PSG level (0: 25%, 1: 50%, 2: 100%).
    psg_level: u8,
}

impl Default for Apu {
    fn default() -> Apu {
        Apu {
            regs: [0; 21],
            enabled: false,
            square1: Square::default(),
            square2: Square::default(),
            wave: WaveChannel::default(),
            noise: Noise::default(),
            step: 0,
            step_timer: SEQUENCER_CYCLES,
            psg_level: 2,
        }
    }
}

impl Apu {
    /// A register's value as the CPU reads it back (NR52: the enable and
    /// the channels' status bits).
    pub fn read(&self, reg: Reg) -> u8 {
        match reg {
            Reg::Nr52 => {
                (self.enabled as u8) << 7
                    | self.square1.on as u8
                    | (self.square2.on as u8) << 1
                    | (self.wave.on as u8) << 2
                    | (self.noise.on as u8) << 3
            }
            r => self.regs[r as usize],
        }
    }

    /// SOUNDCNT_H's PSG level (bits 0-1).
    pub fn set_psg_level(&mut self, level: u8) {
        self.psg_level = level & 3;
    }

    pub fn write(&mut self, reg: Reg, value: u8) {
        if reg == Reg::Nr52 {
            let on = value & 0x80 != 0;
            if !on {
                let ram = self.wave.ram;
                *self = Apu { psg_level: self.psg_level, ..Apu::default() };
                self.wave.ram = ram;
            } else if !self.enabled {
                self.step = 7;
            }
            self.enabled = on;
            self.regs[Reg::Nr52 as usize] = value & 0x80;
            return;
        }
        if !self.enabled {
            return;
        }
        self.regs[reg as usize] = value;
        // A write to a square brings its level up to date first.
        match reg {
            Reg::Nr10 | Reg::Nr11 | Reg::Nr12 | Reg::Nr13 | Reg::Nr14 => self.square1.update_sample(),
            Reg::Nr21 | Reg::Nr22 | Reg::Nr23 | Reg::Nr24 => self.square2.update_sample(),
            _ => {}
        }
        // The sequencer's next step doesn't count lengths.
        let extra = self.step & 1 == 0;
        match reg {
            Reg::Nr10 => {
                let s = &mut self.square1.sweep;
                let was_decrease = s.decrease;
                s.shift = value & 7;
                s.decrease = value & 8 != 0;
                if s.occurred && was_decrease && !s.decrease {
                    self.square1.on = false;
                }
                s.occurred = false;
                s.time = match (value >> 4) & 7 {
                    0 => 8,
                    t => t,
                };
            }
            Reg::Nr11 | Reg::Nr21 => {
                let ch = if reg == Reg::Nr11 { &mut self.square1 } else { &mut self.square2 };
                ch.duty = value >> 6;
                ch.length.counter = 64 - (value & 63) as u16;
            }
            Reg::Nr12 | Reg::Nr22 => {
                let ch = if reg == Reg::Nr12 { &mut self.square1 } else { &mut self.square2 };
                if !ch.envelope.write(value) {
                    ch.on = false;
                }
            }
            Reg::Nr13 | Reg::Nr23 => {
                let ch = if reg == Reg::Nr13 { &mut self.square1 } else { &mut self.square2 };
                ch.frequency = (ch.frequency & 0x700) | value as u16;
            }
            Reg::Nr14 | Reg::Nr24 => {
                let ch = if reg == Reg::Nr14 { &mut self.square1 } else { &mut self.square2 };
                ch.frequency = (ch.frequency & 0xFF) | ((value as u16 & 7) << 8);
                if ch.length.enable(value & 0x40 != 0, extra) {
                    ch.on = false;
                }
                if value & 0x80 != 0 {
                    ch.restart(extra, reg == Reg::Nr14);
                }
            }
            Reg::Nr30 => {
                let w = &mut self.wave;
                w.dac = value & 0x80 != 0;
                w.bank = (value >> 6) & 1;
                w.double = value & 0x20 != 0;
                if !w.dac {
                    w.on = false;
                }
            }
            Reg::Nr31 => self.wave.length.counter = 256 - value as u16,
            Reg::Nr32 => self.wave.set_level(value),
            Reg::Nr33 => self.wave.rate = (self.wave.rate & 0x700) | value as u16,
            Reg::Nr34 => {
                let w = &mut self.wave;
                w.rate = (w.rate & 0xFF) | ((value as u16 & 7) << 8);
                if w.length.enable(value & 0x40 != 0, extra) {
                    w.on = false;
                }
                if value & 0x80 != 0 {
                    w.on = w.dac;
                    w.length.restart(256, extra);
                    w.timer = 8 * (2048 - w.rate as u32) + 24;
                }
            }
            Reg::Nr41 => self.noise.length.counter = 64 - (value & 63) as u16,
            Reg::Nr42 => {
                if !self.noise.envelope.write(value) {
                    self.noise.on = false;
                }
            }
            Reg::Nr43 => {
                let n = &mut self.noise;
                n.shift = value >> 4;
                n.narrow = value & 8 != 0;
                n.divisor = value & 7;
            }
            Reg::Nr44 => {
                let n = &mut self.noise;
                if n.length.enable(value & 0x40 != 0, extra) {
                    n.on = false;
                }
                if value & 0x80 != 0 {
                    n.on = n.envelope.restart();
                    n.lfsr = 0;
                    n.length.restart(64, extra);
                    n.timer = n.period();
                }
            }
            Reg::Nr50 | Reg::Nr51 | Reg::Nr52 => {}
        }
    }

    /// The CPU writes wave RAM: 16 bytes into the bank that isn't playing.
    /// (The hardware rotates a bank as it plays it; a bank written afresh
    /// starts from its first step.)
    pub fn write_wave_ram(&mut self, bytes: &[u8; 16]) {
        let bank = (self.wave.bank ^ 1) as usize;
        self.wave.ram[bank] = *bytes;
        self.wave.position = 0;
    }

    /// Advance `cycles` CPU cycles.
    pub fn run(&mut self, mut cycles: u32) {
        if !self.enabled {
            return;
        }
        while cycles > 0 {
            let n = cycles.min(self.step_timer);
            self.square1.run(n);
            self.square2.run(n);
            self.wave.run(n);
            self.noise.run(n);
            cycles -= n;
            self.step_timer -= n;
            if self.step_timer == 0 {
                self.step_timer = SEQUENCER_CYCLES;
                self.sequencer_step();
            }
        }
    }

    fn sequencer_step(&mut self) {
        self.step = (self.step + 1) & 7;
        match self.step {
            0 | 2 | 4 | 6 => {
                if self.step == 2 || self.step == 6 {
                    self.square1.sweep_step();
                }
                if self.square1.length.step() {
                    self.square1.on = false;
                }
                if self.square2.length.step() {
                    self.square2.on = false;
                }
                if self.wave.length.step() {
                    self.wave.on = false;
                }
                if self.noise.length.step() {
                    self.noise.on = false;
                }
            }
            7 => {
                self.square1.envelope_step();
                self.square2.envelope_step();
                self.noise.envelope_step();
            }
            _ => {}
        }
    }

    /// The PSG's contribution to the DAC now, (left, right), in DAC units
    /// (the 10-bit DAC centred on 0).
    pub fn sample(&self) -> (i32, i32) {
        let nr51 = self.regs[Reg::Nr51 as usize];
        let nr50 = self.regs[Reg::Nr50 as usize];
        let out = [
            self.square1.sample as i32,
            self.square2.sample as i32,
            self.wave.sample as i32,
            self.noise.sample as i32,
        ];
        let (mut left, mut right) = (0, 0);
        for (i, &o) in out.iter().enumerate() {
            if nr51 & (0x10 << i) != 0 {
                left += o;
            }
            if nr51 & (1 << i) != 0 {
                right += o;
            }
        }
        // Eight steps per level times the master volume, then SOUNDCNT_H's
        // share (100%: a quarter of that).
        let shift = 4 - self.psg_level.min(2) as i32;
        let left = (left * 8 * (1 + ((nr50 >> 4) & 7) as i32)) >> shift;
        let right = (right * 8 * (1 + (nr50 & 7) as i32)) >> shift;
        (left, right)
    }
}
