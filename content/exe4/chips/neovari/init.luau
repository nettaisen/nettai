--!strict
-- NeoVari (EXE4 chip 0x0D9; its record at 0x080197EC + 0xD9 * 0x2C): the navi holds A and enters a direction sequence:
-- down, forward, up gives CrosSwrd; back, forward, back, A+B SprSonic; up, A+B, down, A+B, up, A+B DblDream; letting go
-- of A, or 48 ticks, gives a Sword: action 0x5A (0x080EE1A2), VarSwrd's (../varswrd/action) with its own sequences and
-- picks.
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local action = require("./varswrd/action")
local crosswrd = require("./crosswrd").crosswrd
local dbldream = require("./dbldream").dbldream
local sprsonic = require("./sprsonic").sprsonic
local sword = require("./sword").sword

local neovari: Chip = {
    action = action.action {
        id = "neovari/action",
        -- The sequences (0x080EE1E0), as joypad words with A held: 0x81 A+down, 0x11 A+right, 0x21 A+left, 0x41 A+up,
        -- 0x03 A+B.
        sequences = {
            [0] = {
                { 0x81, 0x11, 0x41, 0 },
                { 0x21, 0x11, 0x21, 0x03, 0 },
                { 0x41, 0x03, 0x81, 0x03, 0x41, 0x03, 0 },
            },
        },
        -- CrosSwrd, SprSonic, DblDream (0x080EE2A8: chips 0x176 to 0x178); a Sword (chip 0x30).
        picks = { crosswrd, sprsonic, dbldream },
        sword = sword,
    },
    codes = { "N" },
    element = "null",
    family = "sword",
    class = "mega",
    rarity = 4,
    mb = 74,
    damage = 240,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "library" },
}

return { neovari = neovari }
