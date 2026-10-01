--!strict
-- Registration by number (docs/design/content-model-v2.md §5.5, "As
-- built"): TwinLdrs keeps the pack's record, because the Program Advance
-- table names its result by number. Its record names navi chip subtype 20,
-- which runs this module's `navi_chip`: ProtoMan, who brings Colonel. When
-- the chip is a definition it is `navi = twinldrs.summon` and this goes.

local twinldrs = require("../twinldrs/navi")

local by_number = {}

-- `off_802CD5C[20]`.
by_number.navi_chip = twinldrs.summon

return by_number
