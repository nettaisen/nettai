--!strict
-- Registration by number (docs/design/content-model-v2.md §5.5, "As
-- built"): SunMoon keeps the pack's record, because the Program Advance
-- table names its result by number. Its record names navi chip subtype 25,
-- which runs this module's `navi_chip`: the sun-and-moon (chips/sunmoon).
-- When the chip is a definition it is `navi = sunmoon.summon` and this
-- goes.

local sunmoon = require("../sunmoon/navi")

local by_number = {}

-- `off_802CD5C[25]`.
by_number.navi_chip = sunmoon.summon

return by_number
