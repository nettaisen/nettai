--!strict
-- Registration by number (docs/design/content-model-v2.md §5.5, "As
-- built"): CrcusMan, CrcusMan[EX] and CrcusMan[SP] keep the pack's records,
-- because the ruleset turns navi chips back by their numbers (AntiNavi,
-- 0xDD to 0x118) and the SP chip's damage is a formula. Their records name
-- navi chip subtype 14, which runs this module's `navi_chip`: CircusMan.
-- When the chips are definitions each is `navi = crcusman.summon` and this
-- goes.

local crcusman = require("../crcusman/navi")

local by_number = {}

-- `off_802CD5C[14]`.
by_number.navi_chip = crcusman.summon

return by_number
