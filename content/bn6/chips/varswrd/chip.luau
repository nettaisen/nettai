--!strict
-- VarSwrd (the original's action 0x53, `sub_80EF62E`): the navi holds A
-- and enters a direction sequence (lib/swords/vari): a quarter circle
-- forward (down, down-forward, forward) gives LongSwrd, a half circle from
-- back FtrSword, up-forward-down WideSwrd, down-back-up-forward-down
-- LifeSrd, and back, A+B, forward, A+B SonicBom. Letting go of A, or 48
-- ticks, gives a Sword.
--
-- Its record stays the pack's for now (SlashCross's A-charge, weapon 0x11,
-- asks for its action by number, and the Beast rush's chain leaves the
-- variable swords out by theirs): this is its action, which registration
-- by number runs (chips/052-varswrd; docs/design/content-model-v2.md §12).

local ftrsword = require("../ftrsword/chip")
local lifesrd = require("../lifesrd/chip")
local longswrd = require("../longswrd/chip")
local sonicbom = require("../sonicbom/chips")
local sword = require("../sword/chip")
local vari = require("../../lib/swords/vari")
local wideswrd = require("../wideswrd/chip")

return {
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
        picks = {
            longswrd.action,
            ftrsword.action,
            wideswrd.action,
            lifesrd.action,
            sonicbom.actions.sonicbom,
        },
        sword = sword.action,
        -- SlashCross's A-charge runs it charged: its pick is what becomes
        -- the charged slash.
        charged = "own",
        random = 6,
        no_charged = sonicbom.actions.sonicbom,
        -- The picks' chips, by number until they are definitions (§12).
        legacy = { chips = { 0x49, 0x172, 0x48, 0x153, 0x173 }, sword = 0x47 },
    },
}
