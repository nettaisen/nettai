--!strict
-- A weapon whose setup names an action definition rather than a number.

local ticker = require("../lib/ticker")

local SHOT = ticker.action { id = "test/tick-shot/shot", length = 12 }

return new.weapon {
    id = "test/tick-shot",
    charge_ticks = { 0, 0, 0, 0, 0 },
    setup = function(navi: Object): any
        return SHOT
    end,
}
