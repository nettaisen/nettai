--!strict
-- Registration by number (docs/design/content-model-v2.md §12): FlmHook1-3
-- keep the pack's records, because they are Program Advances (FireHit's
-- records make them, by number). Their records name instant chip effect
-- 14, which runs this module's `instant_chip`: FlmHook's hook
-- (chips/flmhook/hook.luau), fast with the record's first parameter set,
-- its flames' hit spark the record's second (hit effects 0 to 5 are the
-- element sparks, `sub_801A146`'s). When Program Advances are definitions
-- (step 10) each is `instant = hook.instant { spark = sparks.fire }` and
-- this goes.

local hook = require("../flmhook/hook")
local sparks = require("../../lib/sparks")

local by_number = {}

-- The spark a record's hit effect byte names.
local function spark(n: number): Spark
    if n > 5 then
        error(string.format("FlmHook's record names hit effect %d, which no flame of BN6's shows", n))
    end
    return sparks.of_element(n)
end

-- `off_80EC3F0[14]`.
function by_number.instant_chip(user: Object, spec: InstantChipSpec)
    local p = spec.params
    if p[1] > 1 then
        error(string.format("FlmHook's record names flame kind %d, which reads past off_80DE88C", p[1]))
    end
    hook.instant({ fast = p[1] ~= 0, spark = spark(p[2]) })(user, spec)
end

return by_number
