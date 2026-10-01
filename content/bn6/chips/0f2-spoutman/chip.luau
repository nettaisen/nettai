--!strict
-- Registration by number (docs/design/content-model-v2.md §5.5, "As
-- built"): SpoutMan, SpoutMn[EX] and SpoutMn[SP] keep the pack's records,
-- because the ruleset turns navi chips back by their numbers (AntiNavi,
-- 0xDD to 0x118) and the SP chip's damage is a formula. Their records name
-- navi chip subtype 7, which runs this module's `navi_chip`: SpoutMan. When
-- the chips are definitions each is `navi = spoutman.summon` and this goes.

local spoutman = require("../spoutman/navi")

local by_number = {}

-- `off_802CD5C[7]`.
by_number.navi_chip = spoutman.summon

return by_number
