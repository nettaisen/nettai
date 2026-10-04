--!strict
-- VarSwrd (the original's action 0x53, `sub_80EF62E`): the navi holds A
-- and enters a direction sequence (lib/swords/vari): a quarter circle
-- forward (down, down-forward, forward) gives LongSwrd, a half circle from
-- back FtrSword, up-forward-down WideSwrd, down-back-up-forward-down
-- LifeSrd, and back, A+B, forward, A+B SonicBom. Letting go of A, or 48
-- ticks, gives a Sword.

local ftrsword = require("./ftrsword")
local lifesrd = require("./lifesrd")
local longswrd = require("./longswrd")
local sonicbom = require("./sonicbom")
local sword = require("./sword")
local vari = require("../lib/swords/vari")
local wideswrd = require("./wideswrd")

return define.chip {
    id = "varswrd",
    codes = { "K", "V", "W" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 2,
    mb = 28,
    damage = 160,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    library = { number = 81, index = 81, sort = 449 },
    slot_in_limit = 3,
    traits = { "no_chain" },
    action = vari.action {
        id = "varswrd/action",
        -- The sequences (`off_80EF670`), facing right (flip 0) and
        -- mirrored, as joypad words with A held: 0x81 A+down, 0x91
        -- A+down+right, 0x11 A+right, 0xA1 A+down+left, 0x21 A+left, 0x41
        -- A+up, 0x03 A+B.
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
        -- LongSwrd, FtrSword, WideSwrd, LifeSrd, SonicBom (`loc_80EF7D8`).
        picks = { longswrd, ftrsword, wideswrd, lifesrd, sonicbom },
        sword = sword,
        -- SlashCross's A-charge runs it charged: its pick is what becomes
        -- the charged slash.
        charged = "own",
        random = 6,
        no_charged = sonicbom,
    },
}
