--!strict
-- Registration by number (docs/design/content-model-v2.md §5.5, "As
-- built"): CrosOver keeps the pack's record, because the Program Advance
-- table names its result by number. Its record names navi chip subtype 21,
-- which runs this module's `navi_chip`: MegaMan, who brings Django. When
-- the chip is a definition it is `navi = crosover.summon` and this goes.

local crosover = require("../crosover/navi")

local by_number = {}

-- `off_802CD5C[21]`.
by_number.navi_chip = crosover.summon

return by_number
