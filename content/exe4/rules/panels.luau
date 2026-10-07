--!strict
-- EXE4's panels: its 12 panel types (0x0800A3A8), the flag bits each adds; the step rules (0x0800ADF0, a dash's
-- 0x0800C41C: EXE5's values); what the panels do as far as the engine can say it (the panel tick 0x08009740: a
-- broken panel mends after 600 ticks; type 8, lava, turns normal after 960, blinking its last 60). Nearest the
-- engine can say until each is ported (exe4-map.md §18): what each type does to what stands on it and what
-- passes over it, the panel trails' sounds, types 5 and 10 (unread: each under the engine's type with its flags,
-- EXE5's sea's and metal's) and 11 (none: its stages wait), which panels start visible and the front edges (EXE5's:
-- EXE4's tables are to find), the any-side step (EXE5's rows), a broken panel's mend in battle mode 1, and the
-- reservations (EXE5's).
-- Written by tools/exe4/gen_rules.py (bn6battle-verify) from the EXE4 ROM; people own it now.

return {
    types = {
        missing = { flags = 0x00018000 },
        broken = { flags = 0x00014000 },
        normal = { flags = 0x00010010 },
        cracked = { flags = 0x00010050 },
        poison = { flags = 0x00010110 },
        sea = { flags = 0x00030010 },
        grass = { flags = 0x00010410 },
        ice = { flags = 0x00010810 },
        lava = { flags = 0x00011010, expires = 960 },
        holy = { flags = 0x00012010 },
        metal = { flags = 0x00010210 },
    },
    -- EXE4's panel types by its own numbers.
    numbers = { "missing", "broken", "normal", "cracked", "poison", "sea", "grass", "ice", "lava", "holy", "metal" },
    start_visible = {
        { false, false, false, false, false, false, false, false },
        { false, true, true, true, true, true, true, false },
        { false, true, true, true, true, true, true, false },
        { false, true, true, true, true, true, true, false },
        { false, false, false, false, false, false, false, false },
    },
    front_edges = {
        { false, false, false, false, false, false, false, false },
        { false, false, false, false, false, false, false, false },
        { false, false, false, false, false, false, false, false },
        { false, true, true, true, true, true, true, false },
        { false, false, false, false, false, false, false, false },
    },
    step = {
        grounded = {
            { require = 0x00000010, forbid = 0x0b8800a0 },
            { require = 0x00000030, forbid = 0x07880080 },
        },
        floor_free = {
            { require = 0x00000000, forbid = 0x0b8800a0 },
            { require = 0x00000020, forbid = 0x07880080 },
        },
    },
    dash_step = {
        grounded = {
            { require = 0x00000010, forbid = 0x0f8800a0 },
            { require = 0x00000030, forbid = 0x0f880080 },
        },
        floor_free = {
            { require = 0x00000000, forbid = 0x0f8800a0 },
            { require = 0x00000020, forbid = 0x0f880080 },
        },
    },
    any_side_step = {
        grounded = {
            { require = 0x00000010, forbid = 0x07800000 },
            { require = 0x00000010, forbid = 0x0B800000 },
        },
        floor_free = {
            { require = 0x00000000, forbid = 0x07800000 },
            { require = 0x00000000, forbid = 0x0B800000 },
        },
    },
    mend = { normal = 600, battle_mode_1 = 600 },
    reservations = "unmarked",
}
