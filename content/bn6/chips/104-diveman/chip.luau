--!strict
-- Registration by number (docs/design/content-model-v2.md §5.5, "As
-- built"): DiveMan, DiveMan[EX] and DiveMan[SP] keep the pack's records,
-- because the ruleset turns navi chips back by their numbers (AntiNavi,
-- 0xDD to 0x118) and the SP chip's damage is a formula. Their records name
-- navi chip subtype 13, which runs this module's `navi_chip`: DiveMan. When
-- the chips are definitions each is `navi = diveman.summon` and this goes.

local diveman = require("../diveman/navi")

local by_number = {}

-- `off_802CD5C[13]`.
by_number.navi_chip = diveman.summon

return by_number
