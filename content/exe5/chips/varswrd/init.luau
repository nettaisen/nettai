--!strict
-- VarSwrd (EXE5 chip 0x037): the navi holds A and enters a direction sequence: a quarter circle forward (down,
-- down-forward, forward) gives LongSwrd, a half circle from back FtrSword, up-forward-down WideSwrd,
-- down-back-up-forward-down LifeSrd, and back, A+B, forward, A+B SonicBom; letting go of A, or 48 ticks, gives a
-- Sword: EXE5's action 0x2A (action.luau).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local action = require("@self/action")
local ftrsword = require("./ftrsword").ftrsword
local lifesrd = require("./lifesrd").lifesrd
local longswrd = require("./longswrd").longswrd
local sonicbom = require("./sonicbom").sonicbom
local sword = require("./sword").sword
local wideswrd = require("./wideswrd").wideswrd

local chip: Chip = {
    codes = { "B", "C", "V" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 3,
    mb = 60,
    damage = 160,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = action.action {
        -- The sequences (0x080ED4E4), facing right (flip 0) and mirrored, as joypad words with A held: 0x81 A+down, 0x91
        -- A+down+right, 0x11 A+right, 0xA1 A+down+left, 0x21 A+left, 0x41 A+up, 0x03 A+B.
        sequences = {
            [0] = {
                { 0x81, 0x91, 0x11, 0 },
                { 0x21, 0xA1, 0x81, 0x91, 0x11, 0 },
                { 0x41, 0x11, 0x81, 0 },
                { 0x81, 0x21, 0x41, 0x11, 0x81, 0 },
                { 0x21, 0x03, 0x11, 0x03, 0 },
            },
            [1] = {
                { 0x81, 0xA1, 0x21, 0 },
                { 0x11, 0x91, 0x81, 0xA1, 0x21, 0 },
                { 0x41, 0x21, 0x81, 0 },
                { 0x81, 0x11, 0x41, 0x21, 0x81, 0 },
                { 0x11, 0x03, 0x21, 0x03, 0 },
            },
        },
        -- LongSwrd, FtrSword, WideSwrd, LifeSrd, SonicBom (0x080ED618); a Sword (chip 0x30).
        picks = { longswrd, ftrsword, wideswrd, lifesrd, sonicbom },
        sword = sword,
    },
    auto_battle = { class = "sword_swing", swings = { "two_in_row", "three_in_row", "column_ahead", "two_columns_ahead", "all_ahead", "one_ahead" } },
}

return { varswrd = chip }
