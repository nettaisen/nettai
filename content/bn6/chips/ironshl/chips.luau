--!strict
-- IronShl1, IronShl2 and IronShl3: an iron shell thrown onto the panel
-- ahead that slides to the far column, bumps there once and flies off
-- (throw.luau); each faster and in its own palette.
--
-- Their records stay the pack's for now (ParaShl's Program Advance
-- recipes name them by number): these are their actions, which
-- registration by number runs (chips/07b-ironshl1,
-- docs/design/content-model-v2.md §12).

local throw = require("./throw")

-- `byte_80CA7A0`: the shells' speeds, 16.16 a tick.
local SPEEDS = { 0x6_0000, 0x7_0000, 0x8_0000 }

return {
    { action = throw.action { id = "ironshl1/action", shell = { palette = 0, speed = SPEEDS[1], bumps = 1 } } },
    { action = throw.action { id = "ironshl2/action", shell = { palette = 1, speed = SPEEDS[2], bumps = 1 } } },
    { action = throw.action { id = "ironshl3/action", shell = { palette = 2, speed = SPEEDS[3], bumps = 1 } } },
}
