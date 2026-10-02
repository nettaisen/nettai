--!strict
-- BN5's panels, as data the `panels` section can't take yet: BN5's 11 panel types (0x0800BCF0) are BN6's
-- but for metal (type 5, BN6's road flags, a plate's look) and sea (type 10, flag 0x20000), and it has no
-- roads. The engine's section names exactly its own 13 types (docs/design/bn5-map.md §15 proposes the
-- addition); this is `define.rules("panels", ...)`'s spec once it can. Each type's flag bits; the sound a
-- panel trail makes turning a panel into it (0x080113A0); which panels start visible and the front edges
-- (0x0800ABAC, 0x0800ABD4); the step rules (0x0800CAE8, dashes 0x0800E114: BN6's values). BN5 has no
-- any-side step rule (BN6's `sub_800E680` is absent).
--
-- What the panels do differs from BN6's (0x0800A998, BN6's `sub_800C380`): a broken panel mends after 600
-- ticks whatever the battle; lava and sea panels turn normal after 960 ticks (blinking in the last 60); no
-- panel erupts. A grounded body not of fire on lava burns for 50 (the weakness shift applied), and the panel
-- turns normal (0x08016D80, 0x08016E18); sea drains a fire body as poison drains any (0x08016C7E, BN6's
-- `sub_801A186`). The type BN6 calls volcano is BN5's lava here, for its flags and sound only.
-- Written by tools/bn5/gen_rules.py (bn6battle-verify) from the BN5 ROM; people own it now.

return {
    types = {
        missing = { flags = 0x00018000 },
        broken = { flags = 0x00014000 },
        normal = { flags = 0x00010010 },
        cracked = { flags = 0x00010050 },
        poison = { flags = 0x00010110, trail_sound = asset.sound("panel-poison") },
        metal = { flags = 0x00010210, trail_sound = asset.sound("immobilizer") },
        grass = { flags = 0x00010410, trail_sound = asset.sound("grass") },
        ice = { flags = 0x00010810, trail_sound = asset.sound("freeze") },
        volcano = { flags = 0x00011010, trail_sound = asset.sound("panel-volcano") },  -- BN5's lava
        holy = { flags = 0x00012010, trail_sound = asset.sound("panel-change") },
        sea = { flags = 0x00030010, trail_sound = asset.sound("sand-worm") },
    },
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
}
