--!strict
-- NeoVari (the original's action 0x54, `sub_80EF7E2`): VarSwrd's scheme
-- (lib/swords/vari) with three sequences: down, forward, up gives CrosSwrd;
-- back, forward, back, A+B SprSonic; up, A+B, down, A+B, up, A+B DblDream.
-- Letting go of A, or 48 ticks, gives a Sword.

local crosswrd = require("../crosswrd/chip")
local dbldream = require("../dbldream/chip")
local sprsonic = require("../sprsonic/chip")
local sword = require("../sword/chip")
local vari = require("../../lib/swords/vari")

return define.chip {
    id = "bn6:neovari",
    codes = { "N" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 3,
    mb = 52,
    damage = 220,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    library = { number = 82, index = 82, sort = 322 },
    slot_in_limit = 1,
    traits = { "no_chain" },
    action = vari.action {
        id = "bn6:neovari/action",
        -- The sequences (`off_80EF824`), facing right (flip 0) and mirrored
        -- (the up-down one is the same both ways), as joypad words with A
        -- held.
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
        -- CrosSwrd, SprSonic, DblDream (`byte_80EF958`).
        picks = { crosswrd, sprsonic, dbldream },
        sword = sword,
        -- SlashCross's A-charge runs it charged: its pick is what becomes
        -- the charged slash.
        charged = "own",
        random = 4,
        no_charged = sprsonic,
    },
}
