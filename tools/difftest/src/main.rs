//! Differential test of the recompiled game code against mGBA.
//!
//! Replays a Tango match on a linked pair of mGBA cores. On core 0, traps at
//! the entry of a function and at its return site; at entry the whole
//! machine state is captured, at return the recompiled function is run from
//! that capture and the result is compared with mGBA's state.
//!
//! Usage: difftest <replay> <falzar-rom> [--frames N] [--func ADDR --ret ADDR]

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use tango_backend_mgba::GameSupport as _;
use tango_gamesupport_bn6::pvp;

/// The machine state the recompiled code can observe.
#[derive(Clone)]
struct Snap {
    ewram: Vec<u8>,
    iwram: Vec<u8>,
    io: Vec<u8>,
    palette: Vec<u8>,
    vram: Vec<u8>,
    oam: Vec<u8>,
    gprs: [u32; 16],
    cpsr: u32,
    spsr: u32,
    banked: [[u32; 7]; 6],
    banked_spsr: [u32; 6],
    dma: [(u32, u32, u32); 4],
    bios_prefetch: u32,
}

fn snap(core: &mut mgba::core::Core) -> Snap {
    unsafe {
        let g = core.gba_mut().as_raw();
        let m = &(*g).memory;
        let v = &(*g).video;
        let cpu = &*(*g).cpu;
        let regs = &cpu.__bindgen_anon_1.__bindgen_anon_1;
        let bytes = |p: *const u8, n: usize| std::slice::from_raw_parts(p, n).to_vec();
        Snap {
            ewram: bytes(m.wram as *const u8, 0x40000),
            iwram: bytes(m.iwram as *const u8, 0x8000),
            io: bytes(m.io.as_ptr() as *const u8, 0x400),
            palette: bytes(v.palette.as_ptr() as *const u8, 0x400),
            vram: bytes(v.vram as *const u8, 0x18000),
            oam: bytes(&v.oam as *const _ as *const u8, 0x400),
            gprs: regs.gprs.map(|r| r as u32),
            cpsr: regs.cpsr.packed as u32,
            spsr: regs.spsr.packed as u32,
            banked: cpu.bankedRegisters.map(|b| b.map(|r| r as u32)),
            banked_spsr: cpu.bankedSPSRs.map(|r| r as u32),
            bios_prefetch: m.biosPrefetch,
            dma: std::array::from_fn(|i| {
                let d = &m.dma[i];
                (d.nextSource, d.nextDest, d.nextCount as u32)
            }),
        }
    }
}

/// Store through mGBA's bus, so IO side effects happen.
fn io_store16(core: &mut mgba::core::Core, addr: u32, v: u16) {
    unsafe {
        let g = core.gba_mut().as_raw();
        let cpu = (*g).cpu;
        (*cpu).memory.store16.unwrap()(cpu, addr, v as i16, std::ptr::null_mut());
    }
}

/// A recompiled machine in the captured state. The trap fires before the
/// instruction at the function's entry, so gprs[15] is the prefetch PC; the
/// return address is in lr.
fn port_from(s: &Snap, rom: &Arc<[u8]>) -> gba_rt::Cpu {
    let mut c = gba_rt::Cpu::new(rom.clone(), bn6_gen::lookup);
    c.mem.ewram.copy_from_slice(&s.ewram);
    c.mem.iwram.copy_from_slice(&s.iwram);
    c.mem.io.copy_from_slice(&s.io);
    c.mem.palette.copy_from_slice(&s.palette);
    c.mem.vram.copy_from_slice(&s.vram);
    c.mem.oam.copy_from_slice(&s.oam);
    c.r = s.gprs;
    c.set_cpsr(s.cpsr, 0x9);
    c.cpsr_ctl = s.cpsr & 0xDF; // mode + I/F (T is implied by the code)
    c.spsr = s.spsr;
    c.bios_prefetch = s.bios_prefetch;
    // mGBA banks: [bank][0] = sp, [1] = lr. BANK_NONE=0, IRQ=2, SVC=3.
    c.bank_usr = [s.banked[0][0], s.banked[0][1]];
    c.bank_irq = [s.banked[2][0], s.banked[2][1], s.banked_spsr[2]];
    c.bank_svc = [s.banked[3][0], s.banked[3][1], s.banked_spsr[3]];
    for ch in 0..4 {
        let control = u16::from_le_bytes([s.io[0xBA + 12 * ch], s.io[0xBB + 12 * ch]]);
        let (src, dst, count) = s.dma[ch];
        c.io.dma[ch] = gba_rt::Dma { enabled: control & 0x8000 != 0, src, dst, count, control };
    }
    c
}

/// Named regions for reporting and exclusion.
fn region_name(addr: u32) -> &'static str {
    match addr {
        0x0201_0490..=0x0201_0C9F => "ewram:m4a-players",
        0x0200_0000..=0x0203_FFFF => "ewram",
        0x0300_45C0..=0x0300_56FF => "iwram:m4a",
        0x0300_5700..=0x0300_79D3 => "iwram:code",
        0x0300_79D4..=0x0300_7DFF => "iwram:user-stack",
        0x0300_7E00..=0x0300_7F5F => "iwram:irq-stack",
        0x0300_7F60..=0x0300_7FFF => "iwram:svc-stack+vectors",
        0x0300_0000..=0x0300_7FFF => "iwram",
        0x0400_0000..=0x0400_03FF => "io",
        0x0500_0000..=0x0500_03FF => "palette",
        0x0600_0000..=0x0601_7FFF => "vram",
        0x0700_0000..=0x0700_03FF => "oam",
        _ => "?",
    }
}

struct Diff {
    region: &'static str,
    addr: u32,
    ours: u8,
    theirs: u8,
}

fn compare(c: &gba_rt::Cpu, s: &Snap, sp_floor: u32) -> Vec<Diff> {
    let mut out = Vec::new();
    let mut cmp = |base: u32, ours: &[u8], theirs: &[u8]| {
        for (i, (a, b)) in ours.iter().zip(theirs).enumerate() {
            if a != b {
                let addr = base + i as u32;
                let region = region_name(addr);
                // Below the stack pointer is scratch space that mGBA's
                // interrupt handlers also use.
                if region == "iwram:user-stack" && addr < sp_floor {
                    continue;
                }
                out.push(Diff { region, addr, ours: *a, theirs: *b });
            }
        }
    };
    cmp(0x0200_0000, &c.mem.ewram[..], &s.ewram);
    cmp(0x0300_0000, &c.mem.iwram[..], &s.iwram);
    cmp(0x0500_0000, &c.mem.palette[..], &s.palette);
    cmp(0x0600_0000, &c.mem.vram[..], &s.vram);
    cmp(0x0700_0000, &c.mem.oam[..], &s.oam);
    cmp(0x0400_0000, &c.mem.io[..], &s.io);
    out
}

/// Regions whose differences are expected: sound state (the VCount IRQ runs
/// the mixer asynchronously in mGBA), the interrupt stacks, and IO registers
/// that hardware updates behind the program's back.
fn is_expected(d: &Diff) -> bool {
    match d.region {
        "ewram:m4a-players" | "iwram:m4a" | "iwram:irq-stack" | "iwram:svc-stack+vectors" => true,
        "io" => {
            let reg = (d.addr - 0x0400_0000) & !1;
            // DISPSTAT, VCOUNT, sound, DMA1/2 (sound), timers, SIO, IF.
            matches!(reg, 0x04 | 0x06 | 0x60..=0xAF | 0xBC..=0xD3 | 0x100..=0x10F | 0x120..=0x15F | 0x202)
        }
        _ => false,
    }
}

/// The button schedule both sides get in boot mode: taps through the logo,
/// title and menus.
fn boot_keys(frame: u32) -> u16 {
    match frame % 60 {
        30 => bn6::keys::A,
        45 => bn6::keys::START,
        _ => 0,
    }
}

/// Boot from power-on in both mGBA and the port and compare state at the
/// start of every main-loop pass.
fn boot_mode(args: &[String]) {
    let rom = std::fs::read(&args[0]).unwrap();
    let save = std::fs::read(&args[1]).unwrap();
    let frames: u32 = args.get(2).map(|s| s.parse().unwrap()).unwrap_or(600);
    let rom_arc: Arc<[u8]> = rom.clone().into();

    let mut core = mgba::core::OwnedCore::new_gba("difftest", &Default::default()).unwrap();
    core.load_rom(mgba::vfile::VFile::from_vec(rom)).unwrap();
    core.load_save(mgba::vfile::VFile::from_vec(save.clone())).unwrap();
    core.reset();

    struct State {
        port: bn6::Gba,
        frame: u32,
        failures: u32,
        shown: u32,
        region_counts: std::collections::BTreeMap<&'static str, u64>,
    }
    let state = Rc::new(RefCell::new(State {
        port: bn6::Gba::new(rom_arc, Some(&save)),
        frame: 0,
        failures: 0,
        shown: 0,
        region_counts: Default::default(),
    }));
    let st = state.clone();
    core.set_traps(vec![(
        bn6::addr::MAIN_LOOP_BODY,
        Box::new(move |core: &mut mgba::core::Core| {
            let mut s = st.borrow_mut();
            let frame = s.frame;
            let keys = boot_keys(frame);
            // KEYINPUT is read later in this pass.
            core.set_keys(keys as u32);
            let theirs = snap(core);
            if frame > 0 {
                s.port.end_frame();
            }
            s.port.begin_frame(keys);
            let sp = theirs.gprs[13];
            let diffs = compare(&s.port.cpu, &theirs, sp);
            let unexpected: Vec<&Diff> = diffs.iter().filter(|d| !is_expected(d)).collect();
            for d in &diffs {
                *s.region_counts.entry(d.region).or_default() += 1;
            }
            if !unexpected.is_empty() {
                s.failures += 1;
                if s.shown < 5 {
                    s.shown += 1;
                    println!("frame {frame}: {} unexpected diffs", unexpected.len());
                    for d in unexpected.iter().take(16) {
                        println!("    {:#010x} [{}] ours {:02x} theirs {:02x}", d.addr, d.region, d.ours, d.theirs);
                    }
                }
            }
            s.port.run_body();
            s.frame += 1;
        }),
    )]);
    while state.borrow().frame < frames {
        core.run_frame();
    }
    let s = state.borrow();
    println!("{} passes compared: {} with unexpected diffs; diff bytes by region {:?}", s.frame, s.failures, s.region_counts);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(|s| s.as_str()) == Some("boot") {
        boot_mode(&args[1..]);
        return;
    }
    let replay_path = &args[0];
    let rom_path = &args[1];
    let mut max_frames = u32::MAX;
    let mut func = 0x0800_7800u32; // battle_8007800
    let mut ret_site = 0x0812_B6ACu32; // after `bl battle_8007800` in sub_812B698
    let mut verbose = false;
    let mut mask_irq = false;
    let mut peeks: Vec<(u32, u32, u32)> = Vec::new(); // (frame, addr, len)
    let mut watch: Option<(u32, u32, u32)> = None; // (frame, addr, len)
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--frames" => {
                max_frames = args[i + 1].parse().unwrap();
                i += 1;
            }
            "--func" => {
                func = u32::from_str_radix(args[i + 1].trim_start_matches("0x"), 16).unwrap();
                i += 1;
            }
            "--ret" => {
                ret_site = u32::from_str_radix(args[i + 1].trim_start_matches("0x"), 16).unwrap();
                i += 1;
            }
            "-v" => verbose = true,
            "--mask-irq" => mask_irq = true,
            "--peek" => {
                let h = |s: &str| u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap();
                peeks.push((args[i + 1].parse().unwrap(), h(&args[i + 2]), h(&args[i + 3])));
                i += 3;
            }
            "--watch" => {
                // --watch FRAME ADDR LEN
                let h = |s: &str| u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap();
                watch = Some((args[i + 1].parse().unwrap(), h(&args[i + 2]), h(&args[i + 3])));
                i += 3;
            }
            a => panic!("unknown argument {a}"),
        }
        i += 1;
    }

    std::panic::set_hook(Box::new(|_| {}));
    let replay = tango_replay::Replay::decode(std::fs::File::open(replay_path).unwrap()).unwrap();
    let rom = std::fs::read(rom_path).unwrap();
    assert_eq!(&rom[0xAC..0xB0], b"BR6E", "difftest expects US Falzar");
    for p in 0..2 {
        let gi = replay.metadata.side(p).unwrap().game_info.as_ref().unwrap();
        assert!(gi.patch.is_none() && gi.rom_family == "bn6" && gi.rom_variant == 1, "replay must be vanilla Falzar vs Falzar");
    }
    let rom_arc: Arc<[u8]> = rom.clone().into();

    let mut pair = mgba_rollback::Link::with_options(mgba_rollback::LinkOptions {
        sides: vec![
            mgba_rollback::SideOptions { rom: rom.clone(), save: Some(replay.srams[0].clone()) },
            mgba_rollback::SideOptions { rom: rom.clone(), save: Some(replay.srams[1].clone()) },
        ],
        rtc: Some(replay.rtc_time()),
        peripheral: mgba_rollback::Peripheral::Cable,
    })
    .unwrap();
    pair.set_frameskip(0, i32::MAX);
    pair.set_frameskip(1, i32::MAX);
    let prime = tango_backend_mgba::PrimeConfig {
        match_type: (replay.metadata.match_type as u8, replay.metadata.match_subtype as u8),
        rng_seed: replay.rng_seed,
        disable_bgm: false,
    };
    let events = tango_match::telemetry::EventSink::new();
    let primed = [tango_backend_mgba::PrimedLatch::new(), tango_backend_mgba::PrimedLatch::new()];

    #[derive(Default)]
    struct Stats {
        frames: u32,
        clean: u32,
        expected_only: u32,
        failed: u32,
        first_failures: Vec<String>,
        region_counts: std::collections::BTreeMap<&'static str, u64>,
        io_counts: std::collections::BTreeMap<u32, u64>,
        port_time: std::time::Duration,
    }
    let pre: Rc<RefCell<Option<Snap>>> = Default::default();
    let stats: Rc<RefCell<Stats>> = Default::default();

    for core_index in 0..2 {
        let mut traps = pvp::PVP_BR6E_00.primer_traps(&prime, core_index, &events, &primed[core_index]);
        if core_index == 0 {
            let pre_entry = pre.clone();
            let entry_trap: Box<dyn Fn(&mut mgba::core::Core)> = Box::new(move |core: &mut mgba::core::Core| {
                if mask_irq {
                    // No interrupts inside the tested window: IME off until
                    // the return trap restores it.
                    io_store16(core, 0x0400_0208, 0);
                }
                *pre_entry.borrow_mut() = Some(snap(core));
            });
            let pre_ret = pre.clone();
            let stats_ret = stats.clone();
            let rom_ret = rom_arc.clone();
            let peeks = peeks.clone();
            let ret_trap: Box<dyn Fn(&mut mgba::core::Core)> = Box::new(move |core: &mut mgba::core::Core| {
                let Some(before) = pre_ret.borrow_mut().take() else { return };
                let after = snap(core);
                if mask_irq {
                    io_store16(core, 0x0400_0208, 1);
                }
                let mut st = stats_ret.borrow_mut();
                if st.frames >= max_frames {
                    return;
                }
                st.frames += 1;
                let frame = st.frames;
                let mut c = port_from(&before, &rom_ret);
                for &(pf, pa, pl) in &peeks {
                    if pf == frame {
                        let bytes: Vec<u8> = (0..pl).map(|k| c.mem.peek(pa + k)).collect();
                        eprintln!("frame {frame} pre [{pa:#010x}; {pl:#x}] = {bytes:02x?}");
                        let after_bytes: Vec<u8> = (0..pl)
                            .map(|k| {
                                let a = pa + k;
                                match a >> 24 {
                                    2 => after.ewram[(a & 0x3FFFF) as usize],
                                    3 => after.iwram[(a & 0x7FFF) as usize],
                                    4 => after.io[(a & 0x3FF) as usize],
                                    _ => 0,
                                }
                            })
                            .collect();
                        eprintln!("frame {frame} post(mgba) [{pa:#010x}; {pl:#x}] = {after_bytes:02x?}");
                    }
                }
                if let Some((wf, lo, len)) = watch {
                    if wf == frame {
                        c.watch_lo = lo;
                        c.watch_len = len;
                        c.on_watch = Some(|c, a, v, size| {
                            let bt = std::backtrace::Backtrace::force_capture().to_string();
                            let chain: Vec<&str> = bt
                                .lines()
                                .filter_map(|l| l.trim().split_once(": ").map(|x| x.1))
                                .filter(|l| l.starts_with("bn6_gen::"))
                                .map(|l| l.trim_start_matches("bn6_gen::").split("::").last().unwrap())
                                .collect();
                            eprintln!(
                                "write{} [{a:#010x}] = {v:#x}  r0-r3={:x?} via {}",
                                size * 8,
                                &c.r[..4],
                                chain.join(" <- ")
                            );
                        });
                    }
                }
                let t0 = std::time::Instant::now();
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    // Enter with the real return address so the stack matches.
                    let ret = before.gprs[14] & !1;
                    c.ret = ret;
                    let f = c.find(func).expect("function not translated");
                    f(&mut c);
                    assert_eq!(c.pc, ret, "returned to {:#x}", c.pc);
                }));
                st.port_time += t0.elapsed();
                if let Err(e) = result {
                    let msg = e
                        .downcast_ref::<String>()
                        .cloned()
                        .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                        .unwrap_or_default();
                    st.failed += 1;
                    if st.first_failures.len() < 10 {
                        st.first_failures.push(format!("frame {frame}: port panicked: {msg}"));
                    }
                    return;
                }
                let sp_floor = before.gprs[13].min(after.gprs[13]);
                let diffs = compare(&c, &after, sp_floor);
                let mut reg_diffs = Vec::new();
                for r in 0..13 {
                    if c.r[r] != after.gprs[r] {
                        reg_diffs.push(format!("r{r}: ours {:#x} theirs {:#x}", c.r[r], after.gprs[r]));
                    }
                }
                if c.r[13] != after.gprs[13] {
                    reg_diffs.push(format!("sp: ours {:#x} theirs {:#x}", c.r[13], after.gprs[13]));
                }
                let their_z = after.cpsr & (1 << 30) != 0;
                if c.z != their_z {
                    reg_diffs.push(format!("Z: ours {} theirs {}", c.z, their_z));
                }
                let unexpected: Vec<&Diff> = diffs.iter().filter(|d| !is_expected(d)).collect();
                for d in &diffs {
                    *st.region_counts.entry(d.region).or_default() += 1;
                    if d.region == "io" {
                        *st.io_counts.entry(d.addr & !1).or_default() += 1;
                    }
                }
                if unexpected.is_empty() && reg_diffs.is_empty() {
                    if diffs.is_empty() {
                        st.clean += 1;
                    } else {
                        st.expected_only += 1;
                    }
                } else {
                    st.failed += 1;
                    if st.first_failures.len() < 10 || verbose {
                        let mut msg = format!(
                            "frame {frame}: {} unexpected byte diffs, regs {:?}; battle state {:02x?}",
                            unexpected.len(),
                            reg_diffs,
                            &before.ewram[0x34880..0x34884]
                        );
                        for d in unexpected.iter().take(24) {
                            msg.push_str(&format!(
                                "\n    {:#010x} [{}] ours {:02x} theirs {:02x}",
                                d.addr, d.region, d.ours, d.theirs
                            ));
                        }
                        st.first_failures.push(msg);
                    }
                }
            });
            traps.push((func, entry_trap));
            traps.push((ret_site, ret_trap));
        }
        pair.set_traps(core_index, traps);
    }

    let mut prime_ticks = 0;
    while !(primed[0].is_set() && primed[1].is_set()) {
        pair.tick(&[0, 0]);
        prime_ticks += 1;
        assert!(prime_ticks < 3600, "prime timeout");
    }
    let t0 = std::time::Instant::now();
    for row in replay.inputs.iter() {
        pair.tick(&[row[0].keys as u32 & 0x3ff, row[1].keys as u32 & 0x3ff]);
        if stats.borrow().frames >= max_frames {
            break;
        }
    }
    let st = stats.borrow();
    println!(
        "{} frames tested in {:?} (port {:?}): {} identical, {} differ only in expected regions, {} FAILED",
        st.frames,
        t0.elapsed(),
        st.port_time,
        st.clean,
        st.expected_only,
        st.failed
    );
    println!("diff bytes by region: {:?}", st.region_counts);
    println!("io diffs by register: {:x?}", st.io_counts);
    for f in &st.first_failures {
        println!("{f}");
    }
}
