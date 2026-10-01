--!strict
-- Registration by number (docs/design/content-model-v2.md §12): the
-- records that name the original's action 0x42 (ElcPuls1-3 and DestPuls,
-- kept as records for their Program Advances) run this module, which runs
-- the chip's own action (chips/elcpuls) picked by the pulse's look, the
-- record's fourth parameter. It goes when the records are definitions
-- (step 5: the recipes then name their chips by value).

local action = require("../elcpuls/action")
local elcpuls = require("../elcpuls/chips")

local by_number = {
    state = action.STATE,
}

-- The actions by look.
local BY_LOOK: { [number]: Action } = {
    [0] = elcpuls.elcpuls1.action,
    [1] = elcpuls.elcpuls2.action,
    [2] = elcpuls.elcpuls3.action,
    [3] = elcpuls.destpuls.action,
}

function by_number.update(me: Object, s: any)
    local look = me:attack_param(4)
    local a = BY_LOOK[look]
    if not a then
        error(string.format("electric pulse look %d reads past dword_80D6B24", look))
    end
    a.update(me, s)
end

return by_number
