--!strict
-- Registration by number (docs/design/content-model-v2.md §5.5, "As
-- built"): TenguMan, TenguMn[EX] and TenguMn[SP] keep the pack's records,
-- because the ruleset turns navi chips back by their numbers (AntiNavi,
-- 0xDD to 0x118) and the SP chip's damage is a formula. Their records name
-- navi chip subtype 9, which runs this module's `navi_chip`: TenguMan. When
-- the chips are definitions each is `navi = tenguman.summon` and this goes.

local tenguman = require("../tenguman/navi")

local by_number = {}

-- `off_802CD5C[9]`.
by_number.navi_chip = tenguman.summon

return by_number
