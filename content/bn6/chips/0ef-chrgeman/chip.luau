--!strict
-- Registration by number (docs/design/content-model-v2.md §5.5, "As
-- built"): ChrgeMan, ChrgeMn[EX] and ChrgeMn[SP] keep the pack's records,
-- because the ruleset turns navi chips back by their numbers (AntiNavi,
-- 0xDD to 0x118) and the SP chip's damage is a formula. Their records name
-- navi chip subtype 6, which runs this module's `navi_chip`: ChargeMan.
-- When the chips are definitions each is `navi = chrgeman.summon` and this
-- goes.

local chrgeman = require("../chrgeman/navi")

local by_number = {}

-- `off_802CD5C[6]`.
by_number.navi_chip = chrgeman.summon

return by_number
