--!strict
-- Registration by number (docs/design/content-model-v2.md §5.5, "As
-- built"): GrndMan, GrndMan[EX] and GrndMan[SP] keep the pack's records,
-- because the ruleset turns navi chips back by their numbers (AntiNavi,
-- 0xDD to 0x118) and the SP chip's damage is a formula. Their records name
-- navi chip subtype 10, which runs this module's `navi_chip`: GroundMan.
-- When the chips are definitions each is `navi = grndman.summon` and this
-- goes.

local grndman = require("../grndman/navi")

local by_number = {}

-- `off_802CD5C[10]`.
by_number.navi_chip = grndman.summon

return by_number
