--!strict
-- Registration by number (docs/design/content-model-v2.md §5.5, "As
-- built"): BlastMan, BlastMn[EX] and BlastMn[SP] keep the pack's records,
-- because the ruleset turns navi chips back by their numbers (AntiNavi,
-- 0xDD to 0x118) and the SP chip's damage is a formula. Their records name
-- navi chip subtype 12, which runs this module's `navi_chip`: BlastMan.
-- When the chips are definitions each is `navi = blastman.summon` and this
-- goes.

local blastman = require("../blastman/navi")

local by_number = {}

-- `off_802CD5C[12]`.
by_number.navi_chip = blastman.summon

return by_number
