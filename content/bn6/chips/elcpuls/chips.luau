--!strict
-- ElcPuls1-3 and DestPuls: an electric pulse held out over the panel ahead
-- and the column past it, pulling what it hits (./action, ./pulse).
--
-- Their records stay the pack's for now (ElcPuls1-3 are DestPuls's
-- Program Advance recipe, which names them by number, and ElcPuls3
-- MstrCros's; DestPuls is the Program Advance): this module gives their
-- actions, which registration by number runs (chips/022-elcpuls1;
-- docs/design/content-model-v2.md §12).

local action = require("./action")
local pulse = require("./pulse")
local statuses = require("../../rules/status")

-- Every pulse lasts 60 ticks; the navi holds it out 5 less, or 5 for the
-- original's second look (ElcPuls2's).
local TICKS = 0x3C
local HOLD, SHORT_HOLD = TICKS - 5, 5

return {
    elcpuls1 = {
        action = action.action {
            id = "elcpuls1/action",
            pulse = pulse.variant { hit_mod = 1, status = statuses.paralyze_90, ticks = TICKS, palette = 1 },
            hold = HOLD,
        },
    },
    elcpuls2 = {
        action = action.action {
            id = "elcpuls2/action",
            pulse = pulse.variant { hit_mod = 0x51, status = nil, ticks = TICKS, palette = 2 },
            hold = SHORT_HOLD,
        },
    },
    elcpuls3 = {
        action = action.action {
            id = "elcpuls3/action",
            -- Its hits carry bug 0x18 with argument 1.
            pulse = pulse.variant { hit_mod = 3, status = nil, ticks = TICKS, palette = 0, bug = 0x18, bug_arg = 1 },
            hold = HOLD,
        },
    },
    destpuls = {
        action = action.action {
            id = "destpuls/action",
            -- Its hits carry the HP bug 0xF6.
            pulse = pulse.variant { hit_mod = 0, status = statuses.paralyze_150, ticks = TICKS, palette = 3, bug = 0xF6 },
            hold = HOLD,
        },
    },
}
