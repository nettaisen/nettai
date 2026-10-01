--!strict
-- Registration by number (docs/design/content-model-v2.md §5.1, "As
-- built"): a record that names the original's action 0x55 runs this
-- module, which runs the chip's own action (chips/sonicbom) by the boom's
-- hit modifier (the record's first parameter: SonicBom's 3, SprSonic's 1,
-- which goes through, Curse's and Punisher's 0). Records reach it where something names a chip
-- by number: VarSwrd's and NeoVari's picks (SonicBom, SprSonic), and the
-- records of Curse and Punisher. It goes when those are definitions (step
-- 5, phase C).

local sonicbom = require("../sonicbom/chips")

local by_number = {
    state = sonicbom.STATE,
}

-- The actions by the boom's hit modifier (Curse's and Punisher's booms
-- are alike; Curse's action runs both).
local BY_HIT_MOD: { [number]: Action } = {
    [3] = sonicbom.actions.sonicbom,
    [1] = sonicbom.actions.sprsonic,
    [0] = sonicbom.actions.curse,
}

function by_number.update(me: Object, s: any)
    local hit_mod = me:attack_param(1)
    local a = BY_HIT_MOD[hit_mod]
    if not a then
        error(string.format("sonic boom with hit modifier %#x: no chip has it", hit_mod))
    end
    a.update(me, s)
end

return by_number
