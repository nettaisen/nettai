--!strict
-- Registration by number (docs/design/content-model-v2.md §5.1, "As
-- built"): a record that names the original's action 0x24 runs this
-- module, which runs the chips' own action (chips/tankcan; the three are
-- alike). Records reach it where something names a chip by number: the
-- Cross special's TankCans (the ruleset's table of numbers). It goes when
-- that is a role (step 5, phase C).

local tankcan = require("../tankcan/action")
local tankcans = require("../tankcan/chips")

local legacy = {
    state = tankcan.STATE,
}

function legacy.update(me: Object, s: any)
    (tankcans[1].action :: Action).update(me, s)
end

return legacy
