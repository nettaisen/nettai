--!strict
-- Registration by number (docs/design/content-model-v2.md §12): a record
-- that names the original's action 0x3E runs this module, which runs
-- DolThdr's own action (chips/dolthdr). DolThdr1-3 are definitions and run
-- theirs without it; the records still reach it where the ruleset names
-- them by number (the Cross special's DolThdr2 and DolThdr3). The three
-- strike the same thunder, so any level's action serves. It goes when the
-- Cross special's list names chips by handle (phase C, step 10).

local action = require("../dolthdr/action")
local dolthdr = require("../dolthdr/chips")

return {
    state = action.STATE,
    update = function(me: Object, s: any)
        dolthdr[3].action.update(me, s)
    end,
}
