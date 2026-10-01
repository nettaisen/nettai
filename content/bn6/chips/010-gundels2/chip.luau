--!strict
-- Registration by number (docs/design/content-model-v2.md §12): a record
-- that names the original's action 0x37 runs this module, which runs the
-- GunDelSol's own action (chips/gundels) picked by the record's subtype
-- (0 to 3: GunDelS1, S2, S3, EX). The four are definitions and run their
-- actions without it; records still reach it where something names them by
-- number: the Cross special's GunDelS2 (the ruleset's table) and the test
-- content's numbered GunDelSols. It goes when the Cross special's list
-- names chips by handle (phase C, step 10).

local action = require("../gundels/action")
local gundels = require("../gundels/chips")

local by_number = {
    state = action.STATE,
}

function by_number.update(me: Object, s: any)
    local subtype = me.variant
    local chip = gundels[subtype + 1]
    if not chip then
        error(string.format("GunDelSol subtype %d has no chip", subtype))
    end
    chip.action.update(me, s)
end

return by_number
