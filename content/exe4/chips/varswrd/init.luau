--!strict
-- VarSwrd (EXE4 chip 0x037; its record at 0x080197EC + 0x37 * 0x2C): the navi holds A and enters a direction sequence:
-- a quarter circle forward (down, down-forward, forward) gives LongSwrd, a half circle from back FtrSword, up-forward-
-- down WideSwrd, down-back-up-forward-down LifeSrd, and back, A+B, forward, A+B SonicBom; letting go of A, or 48 ticks,
-- gives a Sword: action 0x36 (0x080ED6BE, ./action). EXE4 has no flip: its one set of sequences, forward the right.
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local action = require("@self/action")
local ftrsword = require("./ftrsword").ftrsword
local lifesrd = require("./lifesrd").lifesrd
local sonicbom = require("./sonicbom").sonicbom
local sword_family = require("./sword")

local varswrd: Chip = {
    action = action.action {
        id = "varswrd/action",
        -- The sequences (0x080ED6FC), as joypad words with A held: 0x81 A+down, 0x91 A+down+right, 0x11 A+right, 0xA1
        -- A+down+left, 0x21 A+left, 0x41 A+up, 0x03 A+B.
        sequences = {
            [0] = {
                { 0x81, 0x91, 0x11, 0 },
                { 0x21, 0xA1, 0x81, 0x91, 0x11, 0 },
                { 0x41, 0x11, 0x81, 0 },
                { 0x81, 0x21, 0x41, 0x11, 0x81, 0 },
                { 0x21, 0x03, 0x11, 0x03, 0 },
            },
        },
        -- LongSwrd, FtrSword, WideSwrd, LifeSrd, SonicBom (0x080ED7DC: chips 0x32, 0x172, 0x31, 0x158, 0x173); a
        -- Sword (chip 0x30).
        picks = { sword_family.longswrd, ftrsword, sword_family.wideswrd, lifesrd, sonicbom },
        sword = sword_family.sword,
    },
    codes = { "C", "J", "V" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 3,
    mb = 60,
    damage = 150,
    hit_param = 40,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

return { varswrd = varswrd }
