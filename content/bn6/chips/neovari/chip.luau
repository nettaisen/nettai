--!strict
-- NeoVari (the original's action 0x54, `sub_80EF7E2`): VarSwrd's scheme
-- (lib/swords/vari) with three sequences: down, forward, up gives CrosSwrd;
-- back, forward, back, A+B SprSonic; up, A+B, down, A+B, up, A+B DblDream.
-- Letting go of A, or 48 ticks, gives a Sword.
--
-- Its record stays the pack's for now (SlashCross's A-charge, weapon 0x11,
-- asks for its action by number): this is its action, which registration
-- by number runs (chips/053-neovari; docs/design/content-model-v2.md §12).

local crosswrd = require("../crosswrd/chip")
local dbldream = require("../dbldream/chip")
local sonicbom = require("../sonicbom/chips")
local sword = require("../sword/chip")
local vari = require("../../lib/swords/vari")

return {
    action = vari.action {
        id = "neovari/action",
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
        picks = { crosswrd.action, sonicbom.actions.sprsonic, dbldream.action },
        sword = sword.action,
        random = 4,
        no_charged = sonicbom.actions.sprsonic,
        legacy = { chips = { 0x176, 0x177, 0x178 }, sword = 0x47 },
    },
}
