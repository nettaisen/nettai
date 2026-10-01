--!strict
-- Registration by number (docs/design/content-model-v2.md §5.5, "As
-- built"): ElecMan, ElecMan[EX] and ElecMan[SP] keep the pack's records,
-- because the ruleset turns navi chips back by their numbers (AntiNavi,
-- 0xDD to 0x118) and the SP chip's damage is a formula. Their records name
-- navi chip subtype 3, which runs this module's `navi_chip`: ElecMan. When
-- the chips are definitions each is `navi = elecman.summon` and this goes.

local elecman = require("../elecman/navi")

local by_number = {}

-- `off_802CD5C[3]`.
by_number.navi_chip = elecman.summon

return by_number
