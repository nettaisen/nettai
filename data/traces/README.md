# Golden traces

Per-frame observable battle state recorded from mGBA (the original game),
used to verify the native engine. Regenerate with tools/difftest:

    cd tools/difftest && cargo build
    # Vanilla Falzar vs Falzar, 2 rounds, KO:
    ./target/debug/difftest ~/Documents/Tango/replays/20260901132611-chilly-machgun-6-bn6-vs-weenie-p1.tangoreplay \
        ~/Documents/Tango/roms/exe6f_rom_f_e.srl --trace ../../data/traces/machgun.jsonl
    # Gregar vs Falzar(+soundmod, run as vanilla), 3 rounds, lots of chips; traced on the Falzar core:
    ./target/debug/difftest ~/Downloads/20260806025708-awhisperof-hiboomer-bot-bn6_soundmod-vs-Raichubudd-p2.tangoreplay \
        ~/Documents/Tango/roms/exe6f_rom_f_e.srl --allow-patch --trace ../../data/traces/soundmod.jsonl

Lines are JSON: `{"setup": {...}}` at the start of each round (battle settings,
navi stats, folder, BattleState, RNG), then one object per battle frame.
