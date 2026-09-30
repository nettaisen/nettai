--!strict
-- VarSwrd: action 0x53 (`sub_80EF62E`). The navi holds A and enters a
-- direction sequence (lib/vari_sword): a quarter circle forward (down,
-- down-forward, forward) gives LongSwrd, a half circle from back
-- FtrSword, up-forward-down WideSwrd, down-back-up-forward-down LifeSrd,
-- and back, A+B, forward, A+B SonicBom (action 0x55). Letting go of A, or
-- 48 ticks, gives a Sword.

local vari_sword = require("../../lib/vari_sword")

export type State = VariSwordState

local var_swrd = {
    state = vari_sword.STATE,
}

-- The sequences (`off_80EF670`), facing right (flip 0) and mirrored, as
-- joypad words with A held: 0x81 A+down, 0x91 A+down+right, 0x11
-- A+right, 0xA1 A+down+left, 0x21 A+left, 0x41 A+up, 0x03 A+B.
local SPEC: VariSwordSpec = {
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
    choices = { 0x49, 0x172, 0x48, 0x153, 0x173 },
    random = 6,
    no_charged = 0x173,
}

-- `sub_80EF62E`: set up (`sub_80EF648`), then wait for the pick
-- (`sub_80EF6FC`).
function var_swrd.update(me: Object, s: State)
    local step = me.step
    if step == 0 then
        vari_sword.start(me, s, SPEC)
    elseif step == 4 then
        vari_sword.pick(me, s, SPEC)
    else
        error(string.format("VarSwrd phase %#x reads past its phase table", step))
    end
end

return var_swrd
