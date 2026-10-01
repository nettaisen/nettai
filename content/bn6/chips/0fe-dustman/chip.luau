--!strict
-- Registration by number (docs/design/content-model-v2.md §5.5, "As
-- built"): DustMan, DustMan[EX] and DustMan[SP] keep the pack's records,
-- because the ruleset turns navi chips back by their numbers (AntiNavi,
-- 0xDD to 0x118) and the SP chip's damage is a formula. Their records name
-- navi chip subtype 11, which runs this module's `navi_chip`: DustMan. When
-- the chips are definitions each is `navi = dustman.summon` and this goes.

local dustman = require("../dustman/navi")

local by_number = {}

-- `off_802CD5C[11]`.
by_number.navi_chip = dustman.summon

return by_number
