--!strict
-- Registration by number (docs/design/content-model-v2.md §5.5, "As
-- built"): MstrCros keeps the pack's record, because the Program Advance
-- table names its result by number. Its record names navi chip subtype 22,
-- which runs this module's `navi_chip`: the leader, who brings the Crosses.
-- When the chip is a definition it is `navi = mstrcros.summon` and this
-- goes.

local mstrcros = require("../mstrcros/navi")

local by_number = {}

-- `off_802CD5C[22]`.
by_number.navi_chip = mstrcros.summon

return by_number
