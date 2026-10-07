--!strict
-- EXE4's panels: its 12 panel types (0x0800A3A8), the flag bits each adds, and the element whose hits count once
-- more on a body standing on it (the hit kernel's 0x08012CF2: fire on grass, elec on ice). Type 5 is metal, whose
-- flag 0x20000 the panel routines refuse to crack or break; 10 SandRing's pitfall, which stops a slide or a drag
-- unless the body floats (0x080102FC, 0x08010B54) and turns normal 190 ticks after a type change makes it, a
-- stage's after a grounded body stands on it (0x0800980E); 11 the Hole chip's hole. The step rules (0x0800ADF0, a
-- dash's 0x0800C41C: EXE5's values). What the panels do (the panel tick 0x08009740: a broken panel mends after 600
-- ticks; type 8, lava, turns normal after 960, blinking its last 60): lava burns a grounded body not of fire for 50
-- (0x08013128, a player's even while dimmed; 0x0801309E), poison drains and grass heals a wood body (0x08012FF2,
-- at any HP every 20 ticks), what passes over grass of fire or lava of aqua clears it (0x08013058). Every panel
-- starts visible (0x08009186) and row 3 draws the front edges (0x0800937A). EXE4 has no any-side step rule (EXE6's
-- `sub_800E680` is absent): its rows here are EXE6's, for EXE6's code in an EXE4 arena.
-- Written by tools/exe4/gen_rules.py (bn6battle-verify) from the EXE4 ROM; people own it now.

return {
    types = {
        missing = { flags = 0x00018000 },
        broken = { flags = 0x00014000 },
        normal = { flags = 0x00010010 },
        cracked = { flags = 0x00010050 },
        poison = { flags = 0x00010110, trail_sound = asset.sound("panel-poison") },
        metal = { flags = 0x00030010, unbreakable = true },
        grass = { flags = 0x00010410, doubles = "fire", cleared_by = "fire" },
        ice = { flags = 0x00010810, doubles = "elec" },
        lava = {
            flags = 0x00011010,
            expires = 960,
            burn = { damage = 50, spared_by = 0x206, mood = 20, players_while_dimmed = true },
            cleared_by = "aqua",
        },
        holy = { flags = 0x00012010 },
        pitfall = { flags = 0x00010210, stops_slides = true, traps = true, crumbles = 190 },
        hole = { flags = 0x00010010 },
    },
    -- EXE4's panel types by its own numbers.
    numbers = { "missing", "broken", "normal", "cracked", "poison", "metal", "grass", "ice", "lava", "holy", "pitfall", "hole" },
    -- The types the engine's own code needs: the missing panel, which no type change reaches; what a break and a
    -- crack make (0x08009AEC and its kin: 1 and 3); the normal panel, what a mend, an expiry and a clearing leave
    -- (0x08009740, 0x08013058: 2).
    roles = { missing = "missing", broken = "broken", cracked = "cracked", normal = "normal" },
    start_visible = {
        { true, true, true, true, true, true, true, true },
        { true, true, true, true, true, true, true, true },
        { true, true, true, true, true, true, true, true },
        { true, true, true, true, true, true, true, true },
        { true, true, true, true, true, true, true, true },
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
    -- A broken panel mends after 600 ticks in every battle (0x08009132, 0x0800976C).
    mend = { normal = 600, battle_mode_1 = 600 },
    -- A reservation doesn't mark its holder (0x080143A8 sets the panel's alone), and a destroyed holder's stay
    -- (0x080D8C58 releases none).
    reservations = "unmarked",
    -- The flags' bits a type owns, which a crack, a break or poison clears (0x08009AEC and its kin), metal's
    -- 0x20000 among them.
    type_mask = 0x00023F5F,
}
