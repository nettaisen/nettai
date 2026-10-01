--!strict
-- Registration by number (docs/design/content-model-v2.md §5.5, "As
-- built"): BigHook keeps the pack's record, as the navi chips do until they
-- are definitions (the navi chip trait, step 10). Its record names navi
-- chip subtype 23, which runs this module's `navi_chip`: FlmHook's hook
-- (chips/flmhook/hook.luau) with big flames, their hit spark the heavy
-- impact (the record's parameters 1 and 0xA). When the chip is a definition
-- it is `navi = hook.summon { big = true, spark = sparks.impact }` and this
-- goes.

local hook = require("../flmhook/hook")
local sparks = require("../../lib/sparks")

local by_number = {}

-- `off_802CD5C[23]`.
by_number.navi_chip = hook.summon { big = true, spark = sparks.impact }

return by_number
