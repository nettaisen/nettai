--!strict
-- Registration by number (docs/design/content-model-v2.md §5.5, "As
-- built"): BassAnly keep the pack's records, because the ruleset turns navi
-- chips back by their numbers (AntiNavi) and the pack's Giga chips are
-- records until the chips' data is content (step 5). Their records name
-- navi chip subtype 27, which runs this module's `navi_chip`: BassAnly.
-- When the chip is a definition it is `navi = bassanly.summon` and this
-- goes.

local bassanly = require("../bassanly/navi")

local by_number = {}

-- `off_802CD5C[27]`.
by_number.navi_chip = bassanly.summon

return by_number
