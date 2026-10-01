--!strict
-- Registration by number (docs/design/content-model-v2.md §12): a record
-- that names the original's action 0x32 runs this module, which runs the
-- AquaNdl's own action (chips/aquandl) picked by the needles' palette, the
-- record's first parameter. AquaNdl1 and AquaNdl2 are definitions and run
-- their actions without it; AquaNdl3's record still reaches it (MstrCros's
-- recipe names it by number). It goes when that record is a definition
-- (step 5, phase C).

local action = require("../aquandl/action")
local aquandl = require("../aquandl/chips")

local by_number = {
    state = action.STATE,
}

-- The actions by palette.
local BY_PALETTE: { [number]: Action } = {
    [0] = aquandl[1].action,
    [1] = aquandl[2].action,
    [2] = aquandl[3].action,
}

function by_number.update(me: Object, s: any)
    local palette = me:attack_param(1)
    local a = BY_PALETTE[palette]
    if not a then
        error(string.format("AquaNdl palette %d has no chip", palette))
    end
    a.update(me, s)
end

return by_number
