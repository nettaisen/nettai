--!strict
-- NeoVari: action 0x54 (`sub_80EF7E2`). VarSwrd's scheme (lib/swords/vari)
-- with three sequences: down, forward, up gives CrosSwrd; back, forward,
-- back, A+B SprSonic (action 0x55); up, A+B, down, A+B, up, A+B DblDream.
-- Letting go of A, or 48 ticks, gives a Sword.

local vari_sword = require("../../lib/swords/vari")

export type State = VariSwordState

local neo_vari = {
    state = vari_sword.STATE,
}

-- The sequences (`off_80EF824`), facing right (flip 0) and mirrored (the
-- up-down one is the same both ways), as joypad words with A held.
local SPEC: VariSwordSpec = {
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
    choices = { 0x176, 0x177, 0x178 },
    random = 4,
    no_charged = 0x177,
}

-- `sub_80EF7E2`: set up (`sub_80EF7FC`), then wait for the pick
-- (`sub_80EF87C`).
function neo_vari.update(me: Object, s: State)
    local step = me.step
    if step == 0 then
        vari_sword.start(me, s, SPEC)
    elseif step == 4 then
        vari_sword.pick(me, s, SPEC)
    else
        error(string.format("NeoVari phase %#x reads past its phase table", step))
    end
end

return neo_vari
