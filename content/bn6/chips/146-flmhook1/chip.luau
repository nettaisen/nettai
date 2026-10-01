--!strict
-- Registration by number (docs/design/content-model-v2.md §5.5, "As
-- built"): FlmHook1, FlmHook2 and FlmHook3 keep the pack's records,
-- because they are Program Advance results, which the ruleset's table
-- names by number. Their records name instant chip effect 14, which runs
-- this module's `instant_chip`: FlmHook's hook (chips/flmhook/hook.luau),
-- its flames small, their hit spark fire's (the records' parameters 0 and
-- 1). When the chips are definitions each is
-- `instant = hook.instant { big = false, spark = sparks.fire }` and this
-- goes.

local hook = require("../flmhook/hook")
local sparks = require("../../lib/sparks")

local by_number = {}

-- `off_80EC3F0[14]`.
by_number.instant_chip = hook.instant { big = false, spark = sparks.fire }

return by_number
