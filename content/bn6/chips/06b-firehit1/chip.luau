--!strict
-- Registration by number (docs/design/content-model-v2.md §12): FireHit1-3
-- keep the pack's records, because FlmHook1-3's Program Advances (and
-- MstrCros's) name them by number. Their records name instant chip effect
-- 8, which runs this module's `instant_chip`: FireHit's fists
-- (chips/firehit/fist.luau) with the record's second parameter as their
-- hit modifier. When the recipes name their ingredients (step 10) each is
-- `instant = fist.instant(3)` and this goes.

local fist = require("../firehit/fist")

local by_number = {}

-- `off_80EC3F0[8]`.
function by_number.instant_chip(user: Object, spec: InstantChipSpec)
    fist.instant(spec.params[2])(user, spec)
end

return by_number
