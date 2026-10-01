--!strict
-- H-Burst: a shot along the row that bursts ten times where it hits
-- (./action).
--
-- Its record stays the pack's for now (a Program Advance's, which its
-- recipes make): this is its action, which registration by number runs
-- (chips/152-h-burst; docs/design/content-model-v2.md §12).

local action = require("./action")

return {
    -- Its first parameter: ten bursts.
    action = action.action { id = "h-burst/action", bursts = 10 },
}
