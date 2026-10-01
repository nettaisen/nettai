--!strict
-- Registration by number (docs/design/content-model-v2.md §12): a record
-- that names the original's action 0x39 runs this module, which runs
-- RskyHny's own action (chips/rskyhny) picked by the bees' level, the
-- record's first parameter. RskyHny1 and RskyHny2 are definitions and run
-- their actions without it; RskyHny3's record still reaches it (MstrCros's
-- recipe names it by number). It goes when that record is a definition
-- (step 5, phase C).

local action = require("../rskyhny/action")
local rskyhny = require("../rskyhny/chips")

local by_number = {
    state = action.STATE,
}

-- The actions by level.
local BY_LEVEL: { [number]: Action } = {
    [0] = rskyhny[1].action,
    [1] = rskyhny[2].action,
    [2] = rskyhny[3].action,
}

function by_number.update(me: Object, s: any)
    local level = me:attack_param(1)
    local a = BY_LEVEL[level]
    if not a then
        error(string.format("honey bee level %d reads past its speed table", level))
    end
    a.update(me, s)
end

return by_number
