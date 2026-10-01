--!strict
-- Registration by number (docs/design/content-model-v2.md §5.5, "As
-- built"): Darkness keeps the pack's record, because the Program Advance
-- table names its result by number. Its record names navi chip subtype 24,
-- which runs this module's `navi_chip`: Dark MegaMan, who brings Bass. When
-- the chip is a definition it is `navi = darkness.summon` and this goes.

local darkness = require("../darkness/navi")

local by_number = {}

-- `off_802CD5C[24]`.
by_number.navi_chip = darkness.summon

return by_number
