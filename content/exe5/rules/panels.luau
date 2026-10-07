--!strict
-- EXE5's panels (docs/design/exe5-map.md §15.2): its 11 panel types (0x0800BCF0) are EXE6's but for metal (type 5,
-- EXE6's road flags, a plate's look), lava (type 8, the volcano's flags and sound but its own behavior) and sea
-- (type 10, flag 0x20000); it has no roads. Each type's flag bits; the sound a panel trail makes turning a panel
-- into it (0x080113A0); which panels start visible and the front edges (0x0800ABAC, 0x0800ABD4); the step rules
-- (0x0800CAE8, dashes 0x0800E114: EXE6's values). EXE5 has no any-side step rule (EXE6's `sub_800E680` is absent):
-- its rows here are EXE6's, for EXE6's code in an EXE5 arena.
--
-- What the panels do (0x0800A998, EXE6's `sub_800C380`): a broken panel mends after 600 ticks whatever the
-- battle; lava and sea turn normal after 960 ticks (blinking in the last 60, each its own timer, which a panel's
-- new type sets: 0x0800B2AE); no panel erupts. A grounded body not of fire on lava burns for 50 (shifted by its
-- weakness to fire) and the panel turns normal (0x08016D80, 0x08016E18); sea drains a fire body as poison drains
-- any (0x08016C7E, EXE6's `sub_801A186`), holds a body that ends a move on it for 20 ticks with a splash
-- (0x0801715E) and submerges what can dive (0x08017030); a move's end on metal starts a slide, tried in turn by
-- the direction of the move (0x08017216, the tables at 0x0800C920 and 0x0800C9C0: dx toward the front). A hitbox
-- of fire turns grass normal as it passes over, of aqua lava, of wood metal (0x08016D14).
-- Written by tools/exe5/gen_rules.py (bn6battle-verify) from the EXE5 ROM; people own it now.

-- The steps a metal slide tries: forward, back, up, down.
local F, B, U, D = { dx = 1, dy = 0 }, { dx = -1, dy = 0 }, { dx = 0, dy = -1 }, { dx = 0, dy = 1 }

return {
    types = {
        missing = { flags = 0x00018000 },
        broken = { flags = 0x00014000 },
        normal = { flags = 0x00010010 },
        cracked = { flags = 0x00010050 },
        poison = { flags = 0x00010110, trail_sound = asset.sound("panel-poison") },
        metal = {
            flags = 0x00010210,
            trail_sound = asset.sound("immobilizer"),
            cleared_by = "wood",
            -- By the direction of the move: none, up, down, back, forward, other.
            slide = { {}, { F, U, B, D }, { B, D, F, U }, { U, B, D, F }, { D, F, U, B }, { D, F, U, B } },
        },
        -- A fire hit counts once more on a body standing on grass, an elec one on the sea (0x08016AF6).
        grass = { flags = 0x00010410, trail_sound = asset.sound("grass"), cleared_by = "fire", doubles = "fire" },
        ice = { flags = 0x00010810, trail_sound = asset.sound("freeze") },
        lava = {
            flags = 0x00011010,
            trail_sound = asset.sound("panel-volcano"),
            expires = 960,
            burn = 50,
            cleared_by = "aqua",
        },
        holy = { flags = 0x00012010, trail_sound = asset.sound("panel-change") },
        sea = {
            flags = 0x00030010,
            trail_sound = asset.sound("sand-worm"),
            expires = 960,
            drains = "fire",
            holds = 20,
            submerges = true,
            doubles = "elec",
        },
    },
    -- EXE5's panel types by its own numbers (a panel trail's byte, NaviStats +0x12, names one: the cards' water a sea,
    -- magnet a metal panel).
    numbers = { "missing", "broken", "normal", "cracked", "poison", "metal", "grass", "ice", "lava", "holy", "sea" },
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
    -- A reservation doesn't mark its holder (0x0801865C sets the panel's alone), and a destroyed holder's stay
    -- (EXE5's destroys, 0x080138B6 and 0x080138F2, release none).
    reservations = "unmarked",
    -- The flags' bits a type owns, which a crack, a break or poison clears (0x0800AFF8 and its kin), the sea's
    -- 0x20000 among them.
    type_mask = 0x00023F5F,
}
