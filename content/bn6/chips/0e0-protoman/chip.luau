--!strict
-- Registration by number (docs/design/content-model-v2.md §5.5, "As
-- built"): ProtoMan, ProtoMn[EX] and ProtoMn[SP] keep the pack's records,
-- because the ruleset turns navi chips back by their numbers (AntiNavi,
-- 0xDD to 0x118) and the SP chip's damage is a formula. Their records name
-- navi chip subtype 1, which runs this module's `navi_chip`: ProtoMan. When
-- the chips are definitions each is `navi = protoman.summon` and this goes.

local protoman = require("../protoman/navi")

local by_number = {}

-- `off_802CD5C[1]`.
by_number.navi_chip = protoman.summon

return by_number
