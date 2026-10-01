--!strict
-- Registration by number (docs/design/content-model-v2.md §12): a record
-- that names the original's action 0x36 runs this module, which runs
-- RlngLog's own action (chips/rlnglog). RlngLog1-3 are definitions and run
-- theirs without it; the records still reach it where the ruleset names
-- them by number (the Cross special's RlngLog2 and RlngLog3). The three
-- throw the same logs, so any level's action serves. It goes when the
-- Cross special's list names chips by handle (phase C, step 10).

local action = require("../rlnglog/action")
local rlnglog = require("../rlnglog/chips")

return {
    state = action.STATE,
    update = function(me: Object, s: any)
        rlnglog[1].action.update(me, s)
    end,
}
