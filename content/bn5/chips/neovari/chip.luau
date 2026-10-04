--!strict
-- NeoVari (BN5 chip 0x0d9): the navi holds A and enters a direction sequence: down, forward, up gives CrosSwrd; back,
-- forward, back, A+B SprSonic; up, A+B, down, A+B, up, A+B DblDream; letting go of A, or 48 ticks, gives a Sword: BN5's
-- action 0x31 (0x080EDEE2), VarSwrd's (../varswrd/action) with its own sequences and picks.
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local action = require("../varswrd/action")
local crosswrd = require("../crosswrd/chip")
local dbldream = require("../dbldream/chip")
local sprsonic = require("../sprsonic/chip")
local sword = require("../sword/chip")

return define.chip {
    id = "neovari",
    codes = { "V" },
    element = "null",
    family = "sword",
    class = "mega",
    rarity = 4,
    mb = 74,
    damage = 240,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    action = action.action {
        id = "neovari/action",
        -- The sequences (0x080EDF24), facing right (flip 0) and mirrored, as joypad words with A held: 0x81 A+down,
        -- 0x11 A+right, 0x21 A+left, 0x41 A+up, 0x03 A+B.
        sequences = {
            [0] = {
                { 0x81, 0x11, 0x41, 0 },
                { 0x21, 0x11, 0x21, 0x03, 0 },
                { 0x41, 0x03, 0x81, 0x03, 0x41, 0x03, 0 },
            },
            [1] = {
                { 0x81, 0x21, 0x41, 0 },
                { 0x11, 0x21, 0x11, 0x03, 0 },
                { 0x41, 0x03, 0x81, 0x03, 0x41, 0x03, 0 },
            },
        },
        -- CrosSwrd, SprSonic, DblDream (0x080EE024: chips 0x176 to 0x178); a Sword (chip 0x30).
        picks = { crosswrd, sprsonic, dbldream },
        sword = sword,
    },
}
