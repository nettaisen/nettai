--!strict
-- AuraHed1-3 and StreamHd: an aura head fired along the row (./action,
-- ./head); StreamHd fires five.
--
-- Their records stay the pack's for now (the ruleset knows them by number
-- as the aura chips, and AuraHed1-3 are StreamHd's Program Advance
-- recipes): this module gives their actions, which registration by number
-- runs (chips/05f-aurahed1; docs/design/content-model-v2.md §12).

local action = require("./action")

-- Every head charges at 12 pixels a tick (`byte_80D74D4`).
local SPEED = 0xC_0000

return {
    aurahed1 = {
        action = action.action {
            id = "aurahed1/action",
            head = { palette = 0, speed = SPEED, strong = true, far = false },
        },
    },
    aurahed2 = {
        action = action.action {
            id = "aurahed2/action",
            head = { palette = 1, speed = SPEED, strong = true, far = false },
        },
    },
    aurahed3 = {
        action = action.action {
            id = "aurahed3/action",
            head = { palette = 2, speed = SPEED, strong = true, far = false },
        },
    },
    -- Five heads to the far edge, the last one strong.
    streamhd = {
        action = action.action {
            id = "streamhd/action",
            head = { palette = 0, speed = SPEED, strong = false, far = true },
            stream = true,
        },
    },
}
