--!strict
-- Registration by number (docs/design/content-model-v2.md §12): H-Burst's
-- record, which names the original's action 0x34, runs this module, which
-- runs the chip's own action (chips/h-burst). It goes when the record is a
-- definition (step 5: its Program Advance recipes then name the chips
-- that make it by value).

local action = require("../h-burst/action")
local h_burst = require("../h-burst/chip")

return {
    state = action.STATE,
    update = function(me: Object, s: any)
        h_burst.action.update(me, s)
    end,
}
