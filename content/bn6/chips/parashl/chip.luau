--!strict
-- ParaShl (IronShl's Program Advance): IronShl's throw
-- (chips/ironshl/throw.luau) of three shells down the navi's own back
-- column, which slide four panels, bump and slide back.
--
-- Its record stays the pack's for now (a Program Advance): this is its
-- action, which registration by number runs (chips/07b-ironshl1,
-- docs/design/content-model-v2.md §12).

local throw = require("../ironshl/throw")

-- Its shells' speed (`byte_80CA7A0`'s third), 16.16 a tick.
local SPEED = 0x8_0000

return {
    action = throw.action { id = "parashl/action", shell = { palette = 0, speed = SPEED, bumps = 1, para = true } },
}
