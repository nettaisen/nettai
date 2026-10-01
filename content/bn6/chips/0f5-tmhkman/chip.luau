--!strict
-- Registration by number (docs/design/content-model-v2.md §5.5, "As
-- built"): TmhkMan, TmhkMan[EX] and TmhkMan[SP] keep the pack's records,
-- because the ruleset turns navi chips back by their numbers (AntiNavi,
-- 0xDD to 0x118) and the SP chip's damage is a formula. Their records name
-- navi chip subtype 8, which runs this module's `navi_chip`: TomahawkMan.
-- When the chips are definitions each is `navi = tmhkman.summon` and this
-- goes.

local tmhkman = require("../tmhkman/navi")

local by_number = {}

-- `off_802CD5C[8]`.
by_number.navi_chip = tmhkman.summon

return by_number
