//! Boot the game and run frames: boot <rom> [save] [frames]
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let rom: std::sync::Arc<[u8]> = std::fs::read(&args[0]).unwrap().into();
    let save = args.get(1).map(|p| std::fs::read(p).unwrap());
    let frames: u64 = args.get(2).map(|s| s.parse().unwrap()).unwrap_or(600);
    let t0 = std::time::Instant::now();
    let mut gba = bn6::Gba::new(rom, save.as_deref());
    for f in 0..frames {
        // Tap A every 30 frames to get through the title screen.
        let keys = if f % 60 == 30 { bn6::keys::A } else if f % 60 == 45 { bn6::keys::START } else { 0 };
        gba.run_frame(keys);
    }
    let c = &gba.cpu;
    let mode = c.mem.ewram[0x9A20 - 0x0000]; // placeholder peek
    println!(
        "{} frames in {:?}; unwinds={} bad={} main jumptable index={:#x} ({mode:#x})",
        gba.frames,
        t0.elapsed(),
        c.unwinds,
        c.io.bad_accesses,
        c.mem.peek(c.mem.peek(0x0200_93B0) as u32)
    );
}
