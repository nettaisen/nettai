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

fn hex_range(s: &Snap, addr: u32, len: u32) -> String {
    (0..len)
        .map(|k| {
            let a = addr + k;
            let b = match a >> 24 {
                2 => s.ewram[(a & 0x3FFFF) as usize],
                3 => s.iwram[(a & 0x7FFF) as usize],
                _ => 0,
            };
            format!("{b:02x}")
        })
        .collect()
}

/// A setup record: the battle's static inputs, captured when a round's
/// fighting state machine starts.
fn setup_line(frame: u32, s: &Snap, rom: &[u8]) -> String {
    let ew32 = |a: u32| u32::from_le_bytes(s.ewram[(a & 0x3FFFF) as usize..][..4].try_into().unwrap());
    let settings_ptr = ew32(0x0203_4880 + 0x3C);
    let settings: String = if settings_ptr >> 24 == 8 {
        rom[(settings_ptr & 0x01FF_FFFF) as usize..][..0x10].iter().map(|b| format!("{b:02x}")).collect()
    } else {
        hex_range(s, settings_ptr, 0x10)
    };
    format!(
        "{{\"setup\":{{\"frame\":{frame},\"settings_ptr\":{settings_ptr},\"settings\":\"{settings}\",\"navi_stats\":[\"{}\",\"{}\"],\"folder\":\"{}\",\"battle_state\":\"{}\",\"rng1\":{},\"rng2\":{}}}}}",
        hex_range(s, 0x0203_CE00, 0x64),
        hex_range(s, 0x0203_CE64, 0x64),
        hex_range(s, 0x0203_CDB0, 0x50),
        hex_range(s, 0x0203_4880, 0xF0),
        ew32(0x0200_1120),
        ew32(0x0200_13F0)
    )
}

/// One JSON line of observable battle state (see crates/bn6-battle/src/trace.rs).
fn trace_line(frame: u32, s: &Snap) -> String {
    let ew = |a: u32| s.ewram[(a & 0x3FFFF) as usize];
    let ew16 = |a: u32| u16::from_le_bytes([ew(a), ew(a + 1)]);
    let ew32 = |a: u32| u32::from_le_bytes([ew(a), ew(a + 1), ew(a + 2), ew(a + 3)]);
    const BS: u32 = 0x0203_4880;
    let mut o = format!(
        "{{\"frame\":{frame},\"state\":[{},{},{},{}],\"frames\":{},\"ticks\":{},\"link\":{},\"rng1\":{},\"rng2\":{}",
        ew(BS),
        ew(BS + 1),
        ew(BS + 2),
        ew(BS + 3),
        ew32(BS + 0x60),
        ew32(BS + 0x64),
        ew(0x0203_F7D9),
        ew32(0x0200_1120),
        ew32(0x0200_13F0)
    );
    // Flow state: BattleState, the fighting machine, custom gauge, pause,
    // HUD task mask, banner state.
    o.push_str(&format!(
        ",\"bs\":\"{}\",\"fight\":\"{}\",\"gauge\":{},\"gauge_rate\":{},\"paused\":{},\"hud_tasks\":{},\"banner\":\"{}\"",
        hex_range(s, BS, 0xF0),
        hex_range(s, 0x0203_CA70, 0xC),
        ew16(0x0203_52A0),
        ew16(0x0203_52A2),
        ew(0x0200_1B8A),
        ew32(0x0203_52C0),
        hex_range(s, 0x0203_6840, 0x10)
    ));
    // Per-player input records: held, pressed, released.
    o.push_str(",\"input\":[");
    for p in 0..2u32 {
        let r = 0x0203_6820 + 8 * p;
        o.push_str(&format!("{}[{},{},{}]", if p > 0 { "," } else { "" }, ew16(r + 2), ew16(r + 4), ew16(r + 6)));
    }
    o.push(']');
    // Objects in update order.
    o.push_str(",\"objects\":[");
    let mut node = ew32(0x0200_9380 + 4);
    let mut first = true;
    let mut guard = 0;
    while node != 0x0200_9AB0 && node >> 24 == 2 && guard < 200 {
        let b = node + 0x10;
        let coll = ew32(b + 0x54);
        let flags1 = if coll >> 24 == 2 { ew32(coll + 0x3C) } else { 0 };
        o.push_str(&format!(
            "{}{{\"type\":{},\"index\":{},\"flags\":{},\"params\":{},\"state\":[{},{},{},{}],\"panel\":[{},{}],\"alliance\":{},\"flip\":{},\"hp\":{},\"max_hp\":{},\"pos\":[{},{},{}],\"timer\":{},\"anim\":{},\"status\":{}}}",
            if first { "" } else { "," },
            ew(b + 2) & 0xF,
            ew(b + 1),
            ew(b),
            ew32(b + 4),
            ew(b + 8),
            ew(b + 9),
            ew(b + 10),
            ew(b + 11),
            ew(b + 0x12),
            ew(b + 0x13),
            ew(b + 0x16),
            ew(b + 0x17),
            ew16(b + 0x24),
            ew16(b + 0x26),
            ew32(b + 0x34) as i32,
            ew32(b + 0x38) as i32,
            ew32(b + 0x3C) as i32,
            ew16(b + 0x20),
            ew(b + 0x10),
            flags1
        ));
        first = false;
        node = ew32(node + 4);
        guard += 1;
    }
    o.push(']');
    // The 6x3 field: (type, alliance) per panel, row-major.
    o.push_str(",\"panels\":[");
    for y in 1..=3u32 {
        for x in 1..=6u32 {
            let p = 0x0203_9AE0 + ((y * 8 + x) << 5);
            o.push_str(&format!("{}[{},{}]", if y == 1 && x == 1 { "" } else { "," }, ew(p + 2), ew(p + 3)));
        }
    }
    o.push(']');
    // Chip blocks (the hands chosen at the custom screen), as hex.
    o.push_str(",\"chip_blocks\":[");
    for (i, base) in [0x0203_49C0u32, 0x0203_4A10].iter().enumerate() {
        let hex: String = (0..0x50).map(|k| format!("{:02x}", ew(base + k))).collect();
        o.push_str(&format!("{}\"{hex}\"", if i > 0 { "," } else { "" }));
    }
    o.push_str("]}");
    o
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

/// Write a copy of a save whose equipped folder holds the given chips:
/// `folder IN.sav OUT.sav ID:CODE [ID:CODE ...]` (ids in hex, code a letter
/// or `*`; the list repeats to fill 30 slots).
fn folder_mode(args: &[String]) {
    use tango_gamesupport_common_dataview::save::{Chip, ChipCode, Save as _};
    let buf = std::fs::read(&args[0]).unwrap();
    let mut save = tango_gamesupport_bn6_dataview::save::Save::new(&buf).unwrap();
    let chips: Vec<Chip> = args[2..]
        .iter()
        .map(|s| {
            let (id, code) = s.split_once(':').expect("ID:CODE");
            Chip {
                id: usize::from_str_radix(id.trim_start_matches("0x"), 16).unwrap(),
                code: ChipCode::from_char(code.chars().next().unwrap()).expect("chip code"),
            }
        })
        .collect();
    {
        let mut view = save.view_chips_mut().expect("chips view");
        let folder = view.equipped_folder_index();
        for i in 0..30 {
            assert!(view.set_chip(folder, i, chips[i % chips.len()].clone()));
        }
        view.rebuild_anticheat();
    }
    save.rebuild_checksum();
    std::fs::write(&args[1], save.to_sram_dump()).unwrap();
    println!("wrote {}", args[1]);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(|s| s.as_str()) == Some("folder") {
        folder_mode(&args[1..]);
        return;
    }
    if args.first().map(|s| s.as_str()) == Some("boot") {
        boot_mode(&args[1..]);
        return;
    }
    let replay_path = &args[0];
    let rom_path = &args[1];
    // Gregar ROM for Gregar sides (the port itself is Falzar-only; tracing
    // and testing happen on a Falzar core).
    let gregar_path = std::path::Path::new(rom_path).with_file_name("exe6_rom_e.srl");
    let mut max_frames = u32::MAX;
    let mut func = 0x0800_7800u32; // battle_8007800
    let mut ret_site = 0x0812_B6ACu32; // after `bl battle_8007800` in sub_812B698
    let mut verbose = false;
    let mut mask_irq = false;
    let mut peeks: Vec<(u32, u32, u32)> = Vec::new(); // (frame, addr, len)
    let mut coverage_out: Option<String> = None;
    let mut trace_out: Option<String> = None;
    let mut script: Option<String> = None;
    let mut allow_patch = false;
    let mut saves: Option<[Vec<u8>; 2]> = None;
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
            "--allow-patch" => allow_patch = true,
            "--saves" => {
                saves = Some([std::fs::read(&args[i + 1]).unwrap(), std::fs::read(&args[i + 2]).unwrap()]);
                i += 2;
            }
            "--script" => {
                script = Some(args[i + 1].clone());
                i += 1;
            }
            "--trace" => {
                trace_out = Some(args[i + 1].clone());
                i += 1;
            }
            "--coverage" => {
                coverage_out = Some(args[i + 1].clone());
                i += 1;
            }
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
        println!(
            "side {p}: {} variant {} patch {:?} inputs {} match_type {}/{}",
            gi.rom_family,
            gi.rom_variant,
            gi.patch.as_ref().map(|x| format!("{x:?}")),
            replay.inputs.len(),
            replay.metadata.match_type,
            replay.metadata.match_subtype
        );
        if gi.rom_family != "bn6" || gi.rom_variant != 1 { println!("  (not Falzar)"); }
        assert!(gi.patch.is_none() || allow_patch, "patched replay (pass --allow-patch for sound-only patches)");
    }
    let rom_arc: Arc<[u8]> = rom.clone().into();
    let is_falzar = |p: u8| replay.metadata.side(p).unwrap().game_info.as_ref().unwrap().rom_variant == 1;
    let side_rom = |p: u8| if is_falzar(p) { rom.clone() } else { std::fs::read(&gregar_path).unwrap() };
    // The core whose engine is traced and tested.
    let test_core = if is_falzar(0) { 0 } else { 1 };
    assert!(is_falzar(test_core as u8), "no Falzar side to test");
    println!("testing core {test_core}");

    let mut pair = mgba_rollback::Link::with_options(mgba_rollback::LinkOptions {
        sides: vec![
            mgba_rollback::SideOptions {
                rom: side_rom(0),
                save: Some(saves.as_ref().map(|s| s[0].clone()).unwrap_or_else(|| replay.srams[0].clone())),
            },
            mgba_rollback::SideOptions {
                rom: side_rom(1),
                save: Some(saves.as_ref().map(|s| s[1].clone()).unwrap_or_else(|| replay.srams[1].clone())),
            },
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
        coverage: Vec<u64>,
        objects: std::collections::BTreeMap<(u8, u8), u64>,
        trace: String,
        last_top_state: u8,
    }
    let pre: Rc<RefCell<Option<Snap>>> = Default::default();
    let stats: Rc<RefCell<Stats>> = Default::default();

    for core_index in 0..2 {
        let support = if is_falzar(core_index as u8) { &pvp::PVP_BR6E_00 } else { &pvp::PVP_BR5E_00 };
        let mut traps = support.primer_traps(&prime, core_index, &events, &primed[core_index]);
        if core_index == test_core {
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
            let rom_ret: Arc<[u8]> = rom_ret;
            let peeks = peeks.clone();
            let tracing = trace_out.is_some();
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
                if st.coverage.is_empty() {
                    st.coverage = vec![0; bn6_gen::FUNCTIONS.len()];
                }
                c.coverage = Some(vec![0; bn6_gen::FUNCTIONS.len()]);
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
                if let Some(cov) = c.coverage.take() {
                    for (t, n) in st.coverage.iter_mut().zip(cov) {
                        *t += n;
                    }
                }
                // Live battle objects: the update list from eBattleObjectsLinkedListStart.
                let rd32 = |a: u32| u32::from_le_bytes(after.ewram[(a & 0x3FFFF) as usize..][..4].try_into().unwrap());
                let mut node = rd32(0x0200_9380 + 4);
                let mut guard = 0;
                while node != 0x0200_9AB0 && node >> 24 == 2 && guard < 200 {
                    let obj = ((node + 0x10) & 0x3FFFF) as usize;
                    let key = (after.ewram[obj + 2] & 0xF, after.ewram[obj + 1]);
                    *st.objects.entry(key).or_default() += 1;
                    node = rd32(node + 4);
                    guard += 1;
                }
                if tracing {
                    let top = before.ewram[0x34880];
                    if top == 4 && st.last_top_state != 4 {
                        let line = setup_line(frame, &before, &rom_ret);
                        st.trace.push_str(&line);
                        st.trace.push('\n');
                    }
                    st.last_top_state = top;
                    let line = trace_line(frame, &after);
                    st.trace.push_str(&line);
                    st.trace.push('\n');
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
    // Inputs: the recording, or a script of "TICK P0KEYS P1KEYS" lines (keys
    // in hex, held from TICK until the next line; "end TICK" stops).
    let inputs: Vec<[u32; 2]> = match &script {
        None => replay.inputs.iter().map(|r| [r[0].keys as u32 & 0x3ff, r[1].keys as u32 & 0x3ff]).collect(),
        Some(path) => {
            let mut rows = Vec::new();
            let mut cur = [0u32; 2];
            let mut at = 0usize;
            for line in std::fs::read_to_string(path).unwrap().lines() {
                let line = line.split('#').next().unwrap().trim();
                if line.is_empty() {
                    continue;
                }
                let f: Vec<&str> = line.split_whitespace().collect();
                let tick: usize = f[if f[0] == "end" { 1 } else { 0 }].parse().unwrap();
                while at < tick {
                    rows.push(cur);
                    at += 1;
                }
                if f[0] == "end" {
                    break;
                }
                cur = [u32::from_str_radix(f[1], 16).unwrap(), u32::from_str_radix(f[2], 16).unwrap()];
            }
            rows
        }
    };
    for row in inputs.iter() {
        pair.tick(row);
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
    if let Some(path) = &trace_out {
        std::fs::write(path, &st.trace).unwrap();
        println!("trace: {} frames -> {path}", st.trace.lines().count());
    }
    if let Some(path) = coverage_out {
        let mut out = String::new();
        let mut rows: Vec<(u64, u32)> = st
            .coverage
            .iter()
            .enumerate()
            .filter(|(_, n)| **n > 0)
            .map(|(i, n)| (*n, bn6_gen::FUNCTIONS[i].0))
            .collect();
        rows.sort_by(|a, b| b.cmp(a));
        for (n, a) in &rows {
            out.push_str(&format!("{a:08x}\t{n}\t{}\n", bn6_gen::name(*a).unwrap_or("?")));
        }
        std::fs::write(&path, out).unwrap();
        let mut objs = String::new();
        for ((ty, idx), n) in &st.objects {
            objs.push_str(&format!("T{ty}\t{idx:#04x}\t{n}\n"));
        }
        std::fs::write(format!("{path}.objects"), objs).unwrap();
        println!("coverage: {} functions executed; objects: {} kinds", rows.len(), st.objects.len());
    }
}
